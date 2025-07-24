// executor/src/executor.rs
use crate::{
    config::CONFIG, database::Database, jito_client::JitoClient, jupiter::JupiterClient,
    portfolio_monitor, signer_client, strategies,
};
use anyhow::{anyhow, Result};
use drift_rs::{Context as DriftContext, DriftClient};
use redis::AsyncCommands;
use shared_models::{
    alert, EventType, MarketEvent, OrderDetails, Side, StrategyAction, StrategyAllocation,
    TradeMode,
};
use serde_json::{json, Value};
use std::{collections::HashMap, str::FromStr, sync::Arc, time::Duration};
use tokio::sync::mpsc::{self, Receiver, Sender};
use tokio::task::JoinHandle;
use tracing::{debug, error, info, instrument, warn};

use lazy_static::lazy_static;
use prometheus::{register_counter, register_gauge, Counter, Gauge, Opts};

lazy_static! {
    static ref TRADES_EXECUTED: Counter = register_counter!(
        "executor_trades_executed_total",
        "Total number of trades executed by the executor.",
        &["strategy_id", "trade_mode"]
    )
    .unwrap();
    static ref TRADE_LATENCY: Gauge = register_gauge!(
        "executor_trade_latency_seconds",
        "Latency of trade execution from signal to completion."
    )
    .unwrap();
    static ref ACTIVE_STRATEGIES_GAUGE: Gauge = register_gauge!(
        "executor_active_strategies",
        "Number of currently active strategies."
    )
    .unwrap();
    static ref STALE_EVENTS_TOTAL: Counter = register_counter!(
        "executor_stale_events_total",
        "Total number of stale events discarded by the executor.",
        &["event_type"]
    )
    .unwrap();
}

pub struct MasterExecutor {
    db: Arc<Database>,
    active_strategies: HashMap<String, (Sender<MarketEvent>, JoinHandle<()>)>, // ID -> (Sender, TaskHandle)
    event_router_senders: HashMap<EventType, Vec<Sender<MarketEvent>>>, // EventType -> List of interested strategy senders
    redis_client: redis::Client, // P-7: Client for Redis Streams
    jupiter_client: Arc<JupiterClient>,
    sol_usd_price: Arc<tokio::sync::Mutex<f64>>, // P-2: Store live SOL/USD price
    portfolio_paused: Arc<tokio::sync::Mutex<bool>>, // P-6: Flag to pause trading
    jito_client: Arc<JitoClient>,                // NEW
    drift_client: Arc<DriftClient>,              // NEW
    strategy_allocations: Arc<tokio::sync::Mutex<HashMap<String, StrategyAllocation>>>, // Strategy ID -> Current Allocation
    redis_connection_manager: Arc<tokio::sync::Mutex<redis::aio::ConnectionManager>>,
}

impl MasterExecutor {
    pub fn get_state_snapshot(&self) -> Value {
        let allocations = self.strategy_allocations.blocking_lock();
        let strategies: Vec<Value> = allocations.values().map(|alloc| {
            json!({
                "id": alloc.id,
                "weight": alloc.weight,
                "mode": alloc.mode,
                "params": alloc.params,
                "is_active": self.active_strategies.contains_key(&alloc.id)
            })
        }).collect();

        json!({
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "is_paused": self.portfolio_paused.blocking_lock().clone(),
            "active_strategies_count": self.active_strategies.len(),
            "sol_usd_price": self.sol_usd_price.blocking_lock().clone(),
            "strategies": strategies
        })
    }

    pub async fn new(db: Arc<Database>) -> Result<Self> {
        // Initialize JitoClient and DriftClient correctly with their respective new() or connect methods
        let jito_client = Arc::new(JitoClient::new(&CONFIG.jito_rpc_url).await?);
        let drift_client = Arc::new(DriftClient::connect(DriftContext::Mainnet, None).await?); // None for optional wallet
        let redis_client = redis::Client::open(CONFIG.redis_url.clone())?;
        let redis_connection_manager = Arc::new(tokio::sync::Mutex::new(
            redis::aio::ConnectionManager::new(redis_client.clone()).await?,
        ));

        Ok(Self {
            db,
            active_strategies: HashMap::new(),
            event_router_senders: HashMap::new(),
            redis_client: redis::Client::open(CONFIG.redis_url.clone())?,
            jupiter_client: Arc::new(JupiterClient::new()),
            sol_usd_price: Arc::new(tokio::sync::Mutex::new(1.0)), // P-2: Default to 1.0, will be updated by consumer
            portfolio_paused: Arc::new(tokio::sync::Mutex::new(false)), // P-6: Not paused by default
            jito_client,                                                // Correct initialization
            drift_client,                                               // Correct initialization
            strategy_allocations: Arc::new(tokio::sync::Mutex::new(HashMap::new())),
            redis_connection_manager,
        })
    }

    // simple getter for monitor
    pub fn paused_flag(&self) -> Arc<tokio::sync::Mutex<bool>> {
        self.portfolio_paused.clone()
    }

    pub async fn run(&mut self) -> Result<()> {
        info!("Starting Master Executor run loop.");

        let mut conn_manager = self.redis_connection_manager.lock().await;
        let mut conn = conn_manager.clone();
        drop(conn_manager); // Release lock

        let mut allocation_stream_id = "0".to_string();

        let mut market_stream_ids: HashMap<String, String> = [
            ("events:price", "0"),
            ("events:social", "0"),
            ("events:depth", "0"),
            ("events:bridge", "0"),
            ("events:funding", "0"),
            ("events:sol_price", "0"),
            ("events:onchain", "0"),
            ("events:data_source_heartbeat", "0"),
        ]
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();

        let mut kill_switch_listener = self
            .redis_client
            .get_async_connection()
            .await?
            .into_pubsub();
        kill_switch_listener
            .subscribe("kill_switch_channel")
            .await?;

        loop {
            let read_result = conn
                .xread_options(
                    &market_stream_ids
                        .keys()
                        .map(|k| k.as_str())
                        .collect::<Vec<&str>>(),
                    &market_stream_ids
                        .values()
                        .map(|v| v.as_str())
                        .collect::<Vec<&str>>(),
                    &redis::XReadOptions::default().count(100).block(5000),
                )
                .await;

            match read_result {
                Ok(streams) => {
                    for stream in streams.keys {
                        let stream_name = stream.key;
                        for message in stream.ids {
                            let id_str = message.id.clone();
                            let event_result: Result<MarketEvent, _> =
                                serde_json::from_str(message.get("event").unwrap_or(""));

                            if let Ok(event) = event_result {
                                // Defend against stale data
                                let now = chrono::Utc::now().timestamp();
                                if now - event.timestamp() > 30 {
                                    warn!(
                                        "Discarding stale event of type {:?} with timestamp {}",
                                        event.get_type(),
                                        event.timestamp()
                                    );
                                    STALE_EVENTS_TOTAL
                                        .with_label_values(&[&format!("{:?}", event.get_type())])
                                        .inc();
                                    continue;
                                }

                                if let MarketEvent::SolPrice(sol_price_event) = &event {
                                    *self.sol_usd_price.lock().await = sol_price_event.price_usd;
                                } else if let MarketEvent::DataSourceHeartbeat(heartbeat) = &event {
                                    // Handle heartbeat logic, e.g., update a map of last-seen times
                                } else {
                                    self.dispatch_event(event).await;
                                }
                            } else {
                                error!("Failed to parse event from stream {}: {:?}", stream_name, message);
                            }
                            market_stream_ids.insert(stream_name.clone(), id_str);
                        }
                    }
                }
                Err(e) => {
                    error!("Error reading from market event streams: {}. Attempting to reconnect.", e);
                    *self.portfolio_paused.lock().await = true;
                    tokio::time::sleep(Duration::from_secs(5)).await;
                    // Re-establish connection
                    let mut new_conn_manager = self.redis_connection_manager.lock().await;
                    if let Ok(new_conn) = redis::aio::ConnectionManager::new(self.redis_client.clone()).await {
                        *new_conn_manager = new_conn;
                        info!("Successfully reconnected to Redis.");
                        *self.portfolio_paused.lock().await = false;
                    }
                }
            }

            // Allocation stream reading logic remains similar but should also be adapted for robustness
            // ...

            // Kill switch logic remains the same
            // ...
        }
    }

    async fn reconcile_strategies(&mut self, allocations: Vec<StrategyAllocation>) {
        let new_ids: HashMap<String, StrategyAllocation> =
            allocations.into_iter().map(|a| (a.id.clone(), a)).collect();
        let current_ids: Vec<String> = self.active_strategies.keys().cloned().collect();

        // Lock acquisition order: 1. strategy_allocations, 2. portfolio_paused
        let mut stored_allocs = self.strategy_allocations.lock().await;
        *stored_allocs = new_ids.clone();
        drop(stored_allocs); // Release lock ASAP

        // 1. Stop strategies that are no longer allocated
        for id in current_ids.iter().filter(|id| !new_ids.contains_key(*id)) {
            if let Some((_, handle)) = self.active_strategies.remove(id) {
                handle.abort();
                info!(strategy = id, "Stopped strategy due to deallocation.");
            }
            // Remove from event router senders as well
            for (_, senders) in self.event_router_senders.iter_mut() {
                senders.retain(|s| !s.is_closed()); // Remove closed channels
            }
        }

        // 2. Start new strategies and update existing weights
        for (id, alloc) in new_ids {
            if !self.active_strategies.contains_key(&id) {
                info!(
                    strategy = id,
                    weight = alloc.weight,
                    "Starting new strategy."
                );
                if let Some(mut strategy_instance) = self.build_strategy(&id) {
                    // Pass actual params from alloc
                    if let Err(e) = strategy_instance.init(&alloc.params).await {
                        error!(strategy = id, error = %e, "Failed to initialize strategy, skipping.");
                        continue;
                    }

                    let (tx, rx) = mpsc::channel(100); // Bounded channel for backpressure
                    let strategy_id_clone = id.clone();
                    let db_clone = self.db.clone();
                    let jupiter_client_clone = self.jupiter_client.clone();
                    let sol_usd_price_clone = self.sol_usd_price.clone();
                    let portfolio_paused_clone = self.portfolio_paused.clone();
                    let drift_client_clone = self.drift_client.clone();
                    let jito_client_clone = self.jito_client.clone();
                    let redis_conn_manager_clone = self.redis_connection_manager.clone();

                    // Register subscriptions
                    for sub_type in strategy_instance.subscriptions() {
                        self.event_router_senders
                            .entry(sub_type)
                            .or_default()
                            .push(tx.clone());
                    }

                    let strategy_allocations_clone = self.strategy_allocations.clone();
                    let handle = tokio::spawn(async move {
                        let task_result = tokio::spawn(strategy_task(
                            strategy_instance,
                            rx,
                            db_clone,
                            jupiter_client_clone,
                            drift_client_clone,
                            jito_client_clone,
                            sol_usd_price_clone,
                            portfolio_paused_clone,
                            strategy_allocations_clone,
                            strategy_id_clone.clone(), // clone for the task
                            redis_conn_manager_clone,
                        ))
                        .await;

                        if let Err(e) = task_result {
                            if e.is_panic() {
                                error!(strategy_id = %strategy_id_clone, "Strategy task panicked! It will be shut down.");
                                // Here you could add alerting logic
                            } else {
                                error!(strategy_id = %strategy_id_clone, "Strategy task failed: {:?}", e);
                            }
                        }
                    });
                    self.active_strategies.insert(id, (tx, handle));
                } else {
                    warn!(
                        strategy = id,
                        "Strategy constructor not found. Skipping allocation."
                    );
                }
            } else {
                // Strategy already running, potentially update its internal weight/config if needed
                // (Current strategy trait doesn't have an `update_params` method, but could be added)
                info!(
                    strategy = id,
                    weight = alloc.weight,
                    "Strategy already active, weight updated."
                );
            }
        }
    }

    async fn dispatch_event(&self, event: MarketEvent) {
        let event_type = event.get_type();
        if let Some(senders) = self.event_router_senders.get(&event_type) {
            for sender in senders {
                if let Err(e) = sender.send(event.clone()).await {
                    error!(event_type = ?event_type, error = %e, "Failed to dispatch event to strategy channel.");
                }
            }
        }
    }

    fn build_strategy(&self, id: &str) -> Option<Box<dyn strategies::Strategy>> {
        for constructor in inventory::iter::<strategies::StrategyConstructor> {
            if constructor.0 == id {
                return Some((constructor.1)());
            }
        }
        None
    }

    #[instrument(skip(self, action), fields(strategy_id = %action.strategy_id, action_type = ?action.action_type))]
    async fn execute_action(&self, action: StrategyAction) -> Result<()> {
        let start_time = std::time::Instant::now();
        info!("Executing action for strategy {}", action.strategy_id);

        // Update portfolio paused state from the action if needed
        if let Some(paused) = action.paused {
            *self.portfolio_paused.lock().await = paused;
            info!(
                "Portfolio trading status updated from action: {}",
                if paused { "PAUSED" } else { "RESUMED" }
            );
        }

        match action.action_type {
            shared_models::ActionType::Trade(ref order_details) => {
                let trade_mode = {
                    let allocations = self.strategy_allocations.lock().await;
                    allocations
                        .get(&action.strategy_id)
                        .map(|a| a.mode)
                        .unwrap_or(TradeMode::Paper)
                };

                // Log the trade attempt with the determined trade mode
                let trade_id = self.db.log_trade_attempt(
                    &order_details,
                    &action.strategy_id,
                    0.0,
                    match trade_mode {
                        TradeMode::Paper => "Paper",
                        TradeMode::Live => "Live",
                    },
                )?;

                // Execute the trade logic based on the trade mode
                match trade_mode {
                    TradeMode::Live => {
                        info!(
                            "🔴 LIVE TRADE: {} executing with real capital",
                            action.strategy_id
                        );
                        // Live trading logic (e.g., sending orders to an exchange) goes here
                        // For example, using Jupiter and Drift for executing the trade:
                        let final_size_usd = order_details
                            .suggested_size_usd
                            .min(CONFIG.global_max_position_usd);
                        let current_sol_usd_price = *self.sol_usd_price.lock().await;
                        if current_sol_usd_price <= 0.0 {
                            return Err(anyhow!(
                                "SOL/USD price not available or zero. Cannot size trade."
                            ));
                        }

                        let price_quote = self
                            .jupiter_client
                            .get_quote(
                                final_size_usd / current_sol_usd_price,
                                &order_details.token_address,
                            )
                            .await?;
                        let current_token_price_usd = price_quote.price_per_token;

                        // Log the trade attempt in the database
                        self.db.log_trade_attempt(
                            &order_details,
                            &action.strategy_id,
                            current_token_price_usd,
                            "Live",
                        )?;

                        // Execute the trade using Drift or Jupiter
                        if matches!(order_details.side, Side::Short) {
                            // P-4: Implement Drift perp hedge for shorting
                            info!("P-4: Executing SHORT via Drift perps.");
                            let margin_acct = self.drift_client.get_or_create_user().await?;
                            let args = OpenPositionArgs {
                                market_index: 0, // Assuming SOL-PERP is market 0
                                direction: DriftDirection::Short,
                                base_asset_amount: (final_size_usd / current_sol_usd_price * 1e9)
                                    as u64, // Convert USD to Lamports of SOL equivalent
                                limit_price: None, // Market order
                                reduce_only: false,
                            };
                            let sig = self.drift_client.open_position(&margin_acct, &args).await?;
                            info!(signature = %sig, "Drift SHORT position opened.");
                            self.db.open_trade(trade_id, &sig.to_string())?;
                        } else {
                            // P-4: Spot buy via Jupiter for Longs and Sells (to close shorts/take profit on longs)
                            let swap_tx_b64 = self
                                .jupiter_client
                                .get_swap_transaction(
                                    &user_pk,
                                    &order_details.token_address,
                                    final_size_usd,
                                )
                                .await?;
                            let signed_tx_b64 =
                                signer_client::sign_transaction(&swap_tx_b64).await?;
                            let mut tx = crate::jupiter::deserialize_transaction(&signed_tx_b64)?;

                            // P-5: Jito tip injection
                            let bh = self.jito_client.get_recent_blockhash().await?;
                            tx.message.set_recent_blockhash(bh);
                            self.jito_client
                                .attach_tip(&mut tx, CONFIG.jito_tip_lamports)
                                .await?;

                            // P-5: Send transaction via Jito
                            let sig = self.jito_client.send_transaction(&tx).await?;
                            info!(signature = %sig, "✅ Spot trade submitted via Jito.");
                            self.db.open_trade(trade_id, &sig.to_string())?;
                        }
                    }
                    TradeMode::Paper => {
                        info!(
                            "Executing PAPER {} for ${} of {}",
                            side, order_details.amount_usd, order_details.token_address
                        );
                        // Paper trading logic remains the same
                        self.db.save_trade(&order_details).await?;
                    }
                }
                TRADES_EXECUTED
                    .with_label_values(&[&action.strategy_id, &format!("{:?}", trade_mode)])
                    .inc();
                let latency = start_time.elapsed().as_secs_f64();
                TRADE_LATENCY.set(latency);
                info!("Trade execution took {:.4} seconds", latency);
            }
            shared_models::ActionType::Alert => {
                // Handle alerts if needed
            }
        }
    }
}

#[instrument(skip_all, fields(strategy_id))]
async fn strategy_task(
    mut strategy_instance: Box<dyn strategies::Strategy>,
    mut rx: Receiver<MarketEvent>,
    db: Arc<Database>,
    jupiter_client: Arc<JupiterClient>,
    drift_client: Arc<DriftClient>,
    jito_client: Arc<JitoClient>,
    sol_usd_price: Arc<tokio::sync::Mutex<f64>>,
    portfolio_paused: Arc<tokio::sync::Mutex<bool>>,
    strategy_allocations: Arc<tokio::sync::Mutex<HashMap<String, StrategyAllocation>>>,
    strategy_id: String,
    redis_conn_manager: Arc<tokio::sync::Mutex<redis::aio::ConnectionManager>>,
) {
    info!("Strategy task started.");
    while let Some(event) = rx.recv().await {
        // P-6: Check if portfolio is paused before processing trade signals
        let is_paused = { *portfolio_paused.lock().await }; // Lock and release
        if is_paused {
            debug!(
                "Portfolio paused. Skipping trade signal for {}.",
                strategy_id
            );
            continue;
        }

        match strategy_instance.on_event(&event).await {
            Ok(StrategyAction::Execute(details, _strategy_mode)) => {
                // Override strategy mode with allocation mode
                let allocations = strategy_allocations.lock().await;
                let allocation = allocations.get(&strategy_id);
                let actual_mode = allocation.map(|a| a.mode).unwrap_or(TradeMode::Paper);
                drop(allocations); // Release lock

                let trade_result = execute_trade(
                    db.clone(),
                    jupiter_client.clone(),
                    drift_client.clone(),
                    jito_client.clone(),
                    sol_usd_price.clone(),
                    details.clone(), // Clone details for the trade
                    &strategy_id,
                    actual_mode,
                )
                .await;

                if let Ok(trade_id) = trade_result {
                    // Publish trade event to analytics channel
                    let mut conn = redis_conn_manager.lock().await.clone();
                    let position_update = json!({
                        "position_id": trade_id,
                        "strategy_id": strategy_id,
                        "token_address": details.token_address,
                        "status": "OPEN",
                        "pnl": 0.0,
                        "entry_timestamp": chrono::Utc::now().timestamp(),
                        "triggering_features": details.triggering_features,
                    });

                    let _: Result<(), _> = conn
                        .xadd(
                            "position_updates_channel",
                            "*",
                            &[("data", &position_update.to_string())],
                        )
                        .await;
                    info!("Published trade event for trade_id: {}", trade_id);
                } else if let Err(e) = trade_result {
                    error!(strategy = %strategy_id, error = %e, "Trade execution failed.");
                }
            }
            Ok(StrategyAction::Hold) => { /* No action */ }
            Err(e) => {
                error!(strategy=%strategy_id, error=%e, "Strategy returned an error on event.");
            }
        }
    }
    info!("Strategy task finished.");
}

#[instrument(skip_all, fields(strategy_id, token_address = %details.token_address, action = ?details.side))]
async fn execute_trade(
    db: Arc<Database>,
    jupiter: Arc<JupiterClient>,
    drift: Arc<DriftClient>,
    jito: Arc<JitoClient>,
    sol_price: Arc<tokio::sync::Mutex<f64>>,
    details: OrderDetails,
    strategy_id: &str,
    trade_mode: TradeMode,
) -> Result<i64> { // Return trade_id on success
    let mode_str = if trade_mode == TradeMode::Live {
        "LIVE"
    } else {
        "PAPER"
    };
    info!("Attempting {} trade.", mode_str);

    // Limit suggested size by global max position
    let final_size_usd = details
        .suggested_size_usd
        .min(CONFIG.global_max_position_usd);

    // P-2: Get live SOL/USD price
    let current_sol_usd_price = *sol_price.lock().await;
    if current_sol_usd_price <= 0.0 {
        return Err(anyhow!(
            "SOL/USD price not available or zero. Cannot size trade."
        ));
    }

    // Use limit price from details if available, otherwise get quote
    let current_token_price_usd = if let Some(limit_price) = details.limit_price {
        limit_price
    } else {
        jupiter
            .get_quote(
                final_size_usd / current_sol_usd_price,
                &details.token_address,
            )
            .await?
            .price_per_token
    };

    let trade_id = db.log_trade_attempt(
        &details,
        strategy_id,
        current_token_price_usd,
        match trade_mode {
            TradeMode::Paper => "Paper",
            TradeMode::Live => "Live",
        },
    )?;
    info!(
        trade_id,
        size_usd = final_size_usd,
        price_usd = current_token_price_usd,
        "Trade attempt logged."
    );

    // For paper trading, just simulate the trade
    if trade_mode == TradeMode::Paper {
        info!("📝 PAPER TRADING: Simulating trade.");
        db.open_trade(trade_id, "PAPER_TRADE")?;
        return Ok(trade_id);
    }

    // Below here is LIVE TRADING ONLY
    info!("� LIVE TRADING: Executing real trade with capital!");
    let user_pk = Pubkey::from_str(&signer_client::get_pubkey().await?)?;

    if matches!(details.side, Side::Short) {
        // P-4: Implement Drift perp hedge for shorting
        info!("P-4: Executing SHORT via Drift perps.");
        let margin_acct = drift.get_or_create_user().await?;
        let args = OpenPositionArgs {
            market_index: 0, // Assuming SOL-PERP is market 0
            direction: DriftDirection::Short,
            base_asset_amount: (final_size_usd / current_sol_usd_price * 1e9) as u64, // Convert USD to Lamports of SOL equivalent
            limit_price: None, // Market order
            reduce_only: false,
        };
        let sig = drift.open_position(&margin_acct, &args).await?;
        info!(signature = %sig, "Drift SHORT position opened.");
        db.open_trade(trade_id, &sig.to_string())?;
        // Note: Closing short positions, managing collateral, and PnL tracking for shorts
        // would require additional logic (e.g., a dedicated position monitor for Drift trades).
    } else {
        // P-4: Spot buy via Jupiter for Longs and Sells (to close shorts/take profit on longs)
        let swap_tx_b64 = jupiter
            .get_swap_transaction(&user_pk, &details.token_address, final_size_usd)
            .await?;
        let signed_tx_b64 = signer_client::sign_transaction(&swap_tx_b64).await?;
        let mut tx = crate::jupiter::deserialize_transaction(&signed_tx_b64)?;

        // P-5: Jito tip injection
        let bh = jito.get_recent_blockhash().await?;
        tx.message.set_recent_blockhash(bh);
        jito.attach_tip(&mut tx, CONFIG.jito_tip_lamports).await?;

        // P-5: Send transaction via Jito
        let sig = jito.send_transaction(&tx).await?;
        info!(signature = %sig, "✅ Spot trade submitted via Jito.");
        db.open_trade(trade_id, &sig.to_string())?;
    }

    Ok(trade_id)
}
