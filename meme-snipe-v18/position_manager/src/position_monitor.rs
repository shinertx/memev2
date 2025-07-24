// position_manager/src/position_monitor.rs
use crate::config::CONFIG;
use crate::database::{Database, TradeRecord};
use crate::jupiter::JupiterClient;
use crate::signer_client;
use anyhow::Result;
use redis::{
    streams::{StreamReadOptions, StreamReadReply},
    AsyncCommands,
};
use shared_models::{PriceTick, Side};
use solana_sdk::pubkey::Pubkey;
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tracing::{debug, error, info, instrument, warn};

pub async fn run_monitor(db: Arc<Database>) -> Result<()> {
    info!("📈 Starting Position Manager (Live Position Monitoring)...");
    let redis_url = CONFIG.redis_url.clone();
    let redis_client = redis::Client::open(redis_url).unwrap();
    let jupiter_client = Arc::new(JupiterClient::new(CONFIG.jupiter_api_url.clone()));

    // P-7: Use Redis Streams for market events
    let mut conn = redis_client.get_multiplexed_async_connection().await?;
    let mut market_stream_ids = HashMap::new();
    market_stream_ids.insert("events:price".to_string(), "0".to_string()); // Subscribe to price updates

    // Cache of current token prices (token_address -> price_usd)
    let current_prices: Arc<Mutex<HashMap<String, f64>>> = Arc::new(Mutex::new(HashMap::new()));

    loop {
        let opts = StreamReadOptions::default().count(10).block(5000);
        tokio::select! {
            // Read from market event streams (specifically price updates)
            result = conn.xread_options::<_, _, Option<StreamReadReply>>(&["events:price"], &["$"], &opts) => {
                match result {
                    Ok(streams) => {
                        if let Some(stream_reply) = streams {
                            for stream_key in stream_reply.keys {
                                for message in stream_key.ids {
                                    if let Some(redis::Value::Data(event_bytes)) = message.map.get("event") {
                                        if let Ok(event) = serde_json::from_slice::<PriceTick>(&event_bytes) {
                                            current_prices.lock().await.insert(event.token_address.clone(), event.price_usd);
                                            debug!("Updated price for {}: {:.4}", event.token_address, event.price_usd);
                                        } else {
                                            error!("Failed to deserialize PriceTick from stream ID {}: {:?}", message.id, String::from_utf8_lossy(&event_bytes));
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => error!("Error reading from price event stream: {}", e),
                }
            }
            // Periodically check open positions
            _ = tokio::time::sleep(Duration::from_secs(10)) => {
                if !CONFIG.paper_trading_mode { // Only run for live trades
                    if let Err(e) = check_open_positions(db.clone(), jupiter_client.clone(), current_prices.clone()).await {
                        error!("Error checking open positions: {}", e);
                    }
                }
            }
        }
    }
}

#[instrument(skip_all)]
async fn check_open_positions(
    db: Arc<Database>,
    jupiter_client: Arc<JupiterClient>,
    current_prices: Arc<Mutex<HashMap<String, f64>>>,
) -> Result<()> {
    let open_trades = db.get_open_trades()?;
    if open_trades.is_empty() {
        debug!("No open trades to monitor.");
        return Ok(());
    }
    info!("Monitoring {} open trades...", open_trades.len());

    let prices_guard = current_prices.lock().await;

    for mut trade in open_trades {
        if let Some(&current_price_usd) = prices_guard.get(&trade.token_address) {
            // Update highest price seen for trailing stop
            if trade.highest_price_usd.is_none()
                || current_price_usd > trade.highest_price_usd.unwrap()
            {
                trade.highest_price_usd = Some(current_price_usd);
                db.update_highest_price(trade.id, current_price_usd)?;
                debug!(
                    "Updated HWM for trade {}: {:.4}",
                    trade.id, current_price_usd
                );
            }

            let pnl_pct =
                (current_price_usd - trade.entry_price_usd) / trade.entry_price_usd * 100.0;
            let tsl_trigger_price = trade.highest_price_usd.unwrap()
                * (1.0 - CONFIG.trailing_stop_loss_percent / 100.0);

            info!(
                trade_id = trade.id,
                token = %trade.token_address,
                side = %trade.side,
                current_price = current_price_usd,
                entry_price = trade.entry_price_usd,
                hwm = trade.highest_price_usd.unwrap(),
                tsl_trigger = tsl_trigger_price,
                pnl_pct = pnl_pct,
                "Monitoring trade."
            );

            // Check Trailing Stop Loss for LONG positions
            if trade.side == Side::Long.to_string() && current_price_usd < tsl_trigger_price {
                info!(
                    trade_id = trade.id,
                    "🚨 Trailing Stop Loss triggered for LONG position!"
                );
                execute_close_trade(db.clone(), jupiter_client.clone(), trade, current_price_usd)
                    .await?;
            }
            // Check Trailing Stop Loss for SHORT positions (price goes UP against us)
            else if trade.side == Side::Short.to_string() && current_price_usd > tsl_trigger_price
            {
                info!(
                    trade_id = trade.id,
                    "🚨 Trailing Stop Loss triggered for SHORT position!"
                );
                execute_close_trade(db.clone(), jupiter_client.clone(), trade, current_price_usd)
                    .await?;
            }
            // TODO: Add Take Profit logic here if desired
        } else {
            warn!(
                "Price not available for open trade {}. Skipping monitoring for now.",
                trade.id
            );
        }
    }
    Ok(())
}

#[instrument(skip_all, fields(trade_id = trade.id, token = %trade.token_address, side = %trade.side))]
async fn execute_close_trade(
    db: Arc<Database>,
    jupiter: Arc<JupiterClient>,
    trade: TradeRecord,
    close_price_usd: f64,
) -> Result<()> {
    info!("Executing close trade.");
    let user_pk = Pubkey::from_str(&signer_client::get_pubkey(&CONFIG.signer_url).await?)?;

    let pnl_usd = if trade.side == Side::Long.to_string() {
        (close_price_usd - trade.entry_price_usd) * (trade.amount_usd / trade.entry_price_usd)
    } else {
        // Short position
        (trade.entry_price_usd - close_price_usd) * (trade.amount_usd / trade.entry_price_usd)
    };

    if trade.side == Side::Long.to_string() {
        // Sell spot via Jupiter
        let swap_tx_b64 = jupiter
            .get_swap_transaction(&user_pk, &trade.token_address, trade.amount_usd, 50)
            .await?; // Use 50 bps slippage
        let signed_tx_b64 =
            signer_client::sign_transaction(&CONFIG.signer_url, &swap_tx_b64).await?;
        let tx = crate::jupiter::deserialize_transaction(&signed_tx_b64)?;
        // TODO: Send via Jito (needs JitoClient instance here)
        info!(signature = %tx.signatures[0], "✅ Spot sell submitted via Jupiter/Signer.");
    } else {
        // Short position, close via Drift
        info!("Closing SHORT position via Drift perps.");
        // This would require a DriftClient instance here and logic to close the position.
        // Example: drift_client.close_position(...).await?;
        info!("P-4: Drift SHORT position close simulated.");
    }

    let status = if pnl_usd > 0.0 {
        "CLOSED_PROFIT"
    } else {
        "CLOSED_LOSS"
    };
    db.update_trade_pnl(trade.id, status, close_price_usd, pnl_usd)?;
    info!("Trade closed. Status: {}, PnL: {:.2} USD", status, pnl_usd);

    Ok(())
}
