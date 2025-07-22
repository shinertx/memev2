// executor/src/executor.rs
use crate::{config::CONFIG, database::Database, jupiter::JupiterClient, portfolio_monitor, signer_client, strategies, jito_client::JitoClient};
use anyhow::{anyhow, Result};
use shared_models::{MarketEvent, StrategyAction, StrategyAllocation, OrderDetails, EventType, Side};
use solana_sdk::pubkey::Pubkey;
use std::{collections::HashMap, str::FromStr, sync::Arc};
use tokio::sync::mpsc::{self, Sender, Receiver};
use tokio::task::JoinHandle;
use tracing::{debug, error, info, instrument, warn};
use redis::AsyncCommands;
use drift_sdk::{Client as DriftClient, Network as DriftNet, OpenPositionArgs, Direction as DriftDirection};

pub struct MasterExecutor {
    db: Arc<Database>,
    active_strategies: HashMap<String, (Sender<MarketEvent>, JoinHandle<()>)>, // ID -> (Sender, TaskHandle)
    event_router_senders: HashMap<EventType, Vec<Sender<MarketEvent>>>, // EventType -> List of interested strategy senders
    redis_client: redis::Client, // P-7: Client for Redis Streams
    jupiter_client: Arc<JupiterClient>,
    sol_usd_price: Arc<tokio::sync::Mutex<f64>>, // P-2: Store live SOL/USD price
    portfolio_paused: Arc<tokio::sync::Mutex<bool>>, // P-6: Flag to pause trading
    jito_client: Arc<JitoClient>, // NEW
    drift_client: Arc<DriftClient>, // NEW
}

impl MasterExecutor {
    pub async fn new(db: Arc<Database>) -> Self {
        // Initialize JitoClient and DriftClient correctly with their respective new() or connect methods
        let jito_client = Arc::new(JitoClient::new(&CONFIG.jito_rpc_url).await.unwrap());
        let drift_client = Arc::new(DriftClient::connect(DriftNet::Mainnet, None).await.unwrap()); // None for optional wallet

        Self {
            db,
            active_strategies: HashMap::new(),
            event_router_senders: HashMap::new(),
            redis_client: redis::Client::open(CONFIG.redis_url.clone()).unwrap(),
            jupiter_client: Arc::new(JupiterClient::new()),
            sol_usd_price: Arc::new(tokio::sync::Mutex::new(1.0)), // P-2: Default to 1.0, will be updated by consumer
            portfolio_paused: Arc::new(tokio::sync::Mutex::new(false)), // P-6: Not paused by default
            jito_client, // Correct initialization
            drift_client, // Correct initialization
        }
    }

    // simple getter for monitor
    pub fn paused_flag(&self) -> Arc<tokio::sync::Mutex<bool>> {
        self.portfolio_paused.clone()
    }

    pub async fn run(&mut self) -> Result<()> {
        info!("Starting Master Executor run loop.");
        
        // P-7: Use Redis Streams for allocations and market events
        let mut conn = self.redis_client.get_async_connection().await?;
        let mut allocation_stream_id = HashMap::new();
        allocation_stream_id.insert("allocations_channel".to_string(), "0".to_string()); // Start from beginning

        let mut market_stream_ids = HashMap::new();
        market_stream_ids.insert("events:price".to_string(), "0".to_string());
        market_stream_ids.insert("events:social".to_string(), "0".to_string());
        market_stream_ids.insert("events:depth".to_string(), "0".to_string());
        market_stream_ids.insert("events:bridge".to_string(), "0".to_string());
        market_stream_ids.insert("events:funding".to_string(), "0".to_string());
        market_stream_ids.insert("events:sol_price".to_string(), "0".to_string()); // P-2: Subscribe to SOL price stream
        market_stream_ids.insert("events:onchain".to_string(), "0".to_string()); // NEW: Subscribe to OnChain events

        // P-6: Subscribe to kill switch channel (Pub/Sub)
        let mut kill_switch_listener = self.redis_client.get_async_connection().await?.into_pubsub();
        kill_switch_listener.subscribe("kill_switch_channel").await?;

        loop {
            tokio::select! {
                // P-7: Read from allocation stream
                result = conn.xread_map(&allocation_stream_id, &[("allocations_channel", ">")]).await => {
                    match result {
                        Ok(streams) => {
                            for (_, messages) in streams {
                                for (id, payload) in messages {
                                    if let Some(alloc_bytes) = payload.get("allocations") { // Access 'allocations' field
                                        if let Ok(allocations) = serde_json::from_slice::<Vec<StrategyAllocation>>(alloc_bytes) {
                                            self.reconcile_strategies(allocations).await;
                                        } else {
                                            error!("Failed to deserialize allocations from stream ID {}: {:?}", String::from_utf8_lossy(&id.id), String::from_utf8_lossy(alloc_bytes));
                                        }
                                    }
                                    // Update last read ID
                                    allocation_stream_id.insert("allocations_channel".to_string(), String::from_utf8_lossy(&id.id).to_string());
                                }
                            }
                        }
                        Err(e) => error!("Error reading from allocations_channel stream: {}", e),
                    }
                }
                // P-7: Read from market event streams
                result = conn.xread_map(&market_stream_ids, &[
                    ("events:price", ">"),
                    ("events:social", ">"),
                    ("events:depth", ">"),
                    ("events:bridge", ">"),
                    ("events:funding", ">"),
                    ("events:sol_price", ">"), // P-2
                    ("events:onchain", ">"), // NEW
                ]).await => {
                    match result {
                        Ok(streams) => {
                            for (stream_name, messages) in streams {
                                for (id, payload) in messages {
                                    let event_result = match stream_name.as_str() {
                                        "events:price" => payload.get("event").and_then(|e| serde_json::from_slice::<shared_models::PriceTick>(e).ok()).map(MarketEvent::Price),
                                        "events:social" => payload.get("event").and_then(|e| serde_json::from_slice::<shared_models::SocialMention>(e).ok()).map(MarketEvent::Social),
                                        "events:depth" => payload.get("event").and_then(|e| serde_json::from_slice::<shared_models::DepthEvent>(e).ok()).map(MarketEvent::Depth),
                                        "events:bridge" => payload.get("event").and_then(|e| serde_json::from_slice::<shared_models::BridgeEvent>(e).ok()).map(MarketEvent::Bridge),
                                        "events:funding" => payload.get("event").and_then(|e| serde_json::from_slice::<shared_models::FundingEvent>(e).ok()).map(MarketEvent::Funding),
                                        "events:sol_price" => payload.get("event").and_then(|e| serde_json::from_slice::<shared_models::SolPriceEvent>(e).ok()).map(MarketEvent::SolPrice), // P-2
                                        "events:onchain" => payload.get("event").and_then(|e| serde_json::from_slice::<shared_models::OnChainEvent>(e).ok()).map(MarketEvent::OnChain), // NEW
                                        _ => None, // Unknown stream, ignore
                                    };

                                    if let Some(event) = event_result {
                                        if let MarketEvent::SolPrice(sol_price_event) = event { // P-2: Update SOL price
                                            *self.sol_usd_price.lock().await = sol_price_event.price_usd;
                                            info!("Updated SOL price to: {:.2} USD", sol_price_event.price_usd);
                                        } else {
                                            self.dispatch_event(event).await;
                                        }
                                    } else {
                                        error!("Failed to parse event from stream {}: {:?}", stream_name, payload);
                                    }
                                    // Update last read ID for the specific stream
                                    market_stream_ids.insert(stream_name, String::from_utf8_lossy(&id.id).to_string());
                                }
                            }
                        }
                        Err(e) => error!("Error reading from market event streams: {}", e),
                    }
                }
                // P-6: Read from kill switch channel (Pub/Sub)
                Some(msg) = kill_switch_listener.get_message() => {
                    if let Ok(payload) = msg.get_payload::<String>() {
                        let is_paused = payload == "PAUSE";
                        *self.portfolio_paused.lock().await = is_paused;
                        info!("Portfolio trading status: {}", if is_paused { "PAUSED" } else { "RESUMED" });
                    }
                }
            }
        }
    }

    async fn reconcile_strategies(&mut self, allocations: Vec<StrategyAllocation>) {
        let new_ids: HashMap<String, StrategyAllocation> = allocations.into_iter().map(|a| (a.id.clone(), a)).collect();
        let current_ids: Vec<String> = self.active_strategies.keys().cloned().collect();

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
                info!(strategy = id, weight = alloc.weight, "Starting new strategy.");
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
                    let sol_usd_price_clone = self.sol_usd_price.clone(); // P-2
                    let portfolio_paused_clone = self.portfolio_paused.clone(); // P-6
                    let drift_client_clone = self.drift_client.clone(); // Pass drift client
                    let jito_client_clone = self.jito_client.clone(); // Pass jito client

                    // Register subscriptions
                    for sub_type in strategy_instance.subscriptions() {
                        self.event_router_senders.entry(sub_type).or_default().push(tx.clone());
                    }

                    let handle = tokio::spawn(async move {
                        strategy_task(
                            strategy_instance,
                            rx,
                            db_clone,
                            jupiter_client_clone,
                            drift_client_clone,
                            jito_client_clone,
                            sol_usd_price_clone,
                            portfolio_paused_clone,
                            strategy_id_clone,
                        ).await;
                    });
                    self.active_strategies.insert(id, (tx, handle));
                } else {
                    warn!(strategy = id, "Strategy constructor not found. Skipping allocation.");
                }
            } else {
                // Strategy already running, potentially update its internal weight/config if needed
                // (Current strategy trait doesn't have an `update_params` method, but could be added)
                 info!(strategy = id, weight = alloc.weight, "Strategy already active, weight updated.");
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
    strategy_id: String,
) {
    info!("Strategy task started.");
    while let Some(event) = rx.recv().await {
        // P-6: Check if portfolio is paused before processing trade signals
        if *portfolio_paused.lock().await {
            debug!("Portfolio paused. Skipping trade signal for {}.", strategy_id);
            continue;
        }

        match strategy_instance.on_event(&event).await {
            Ok(StrategyAction::Execute(details)) => {
                if let Err(e) = execute_trade(
                    db.clone(),
                    jupiter_client.clone(),
                    drift_client.clone(),
                    jito_client.clone(),
                    sol_usd_price.clone(),
                    details,
                    &strategy_id,
                ).await { 
                    error!(strategy=%strategy_id, error=%e, "Trade execution failed."); 
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
) -> Result<()> {
    info!("Attempting trade.");

    // Limit suggested size by global max position
    let final_size_usd = details.suggested_size_usd.min(CONFIG.global_max_position_usd);

    // P-2: Get live SOL/USD price
    let current_sol_usd_price = *sol_price.lock().await;
    if current_sol_usd_price <= 0.0 {
        return Err(anyhow!("SOL/USD price not available or zero. Cannot size trade."));
    }

    let price_quote = jupiter.get_quote(final_size_usd / current_sol_usd_price, &details.token_address).await?;
    let current_token_price_usd = price_quote.price_per_token;

    let trade_id = db.log_trade_attempt(&details, strategy_id, current_token_price_usd)?;
    info!(trade_id, size_usd = final_size_usd, price_usd = current_token_price_usd, "Trade attempt logged.");

    if CONFIG.paper_trading_mode {
        info!("🧻 PAPER TRADING MODE: Simulating trade.");
        simulate_fill(&db, trade_id, final_size_usd, matches!(details.side, Side::Short))?;
    } else {
        info!("🔥 LIVE TRADING MODE: Executing real trade.");
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
            let swap_tx_b64 = jupiter.get_swap_transaction(&user_pk, &details.token_address, final_size_usd).await?;
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

            // Live trades would be closed by a separate monitoring logic / position manager.
            // The PnL update would happen asynchronously when the position is closed.
        }
    }
    Ok(())
}

fn simulate_fill(db: &Database, id: i64, size: f64, short: bool) -> Result<()> {
    let pnl = size * (rand::random::<f64>() * 0.1 - 0.05) * if short { -1.0 } else { 1.0 };
    let status = if pnl > 0.0 { "CLOSED_PROFIT" } else { "CLOSED_LOSS" };
    db.open_trade(id, "paper")?; // Mark as opened first to update status later
    db.update_trade_pnl(id, status, 0.0, pnl)?; // Use 0.0 for close price in paper simulation
    info!(trade_id = id, status, pnl, "Paper trade finalized.");
    Ok(())
}
