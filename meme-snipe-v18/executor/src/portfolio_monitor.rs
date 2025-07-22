// executor/src/portfolio_monitor.rs
use crate::config::CONFIG;
use crate::database::Database;
use anyhow::Result;
use std::{sync::Arc, time::Duration};
use tracing::{error, info, warn};
use redis::AsyncCommands; // P-7: For Redis Streams

pub async fn run_monitor(db: Arc<Database>, portfolio_paused_flag: Arc<tokio::sync::Mutex<bool>>) {
    info!("📈 Starting Portfolio Monitor (P-6)...");
    let redis_url = CONFIG.redis_url.clone();
    let client = match redis::Client::open(redis_url) {
        Ok(client) => client,
        Err(e) => {
            error!("Failed to create Redis client: {}", e);
            return;
        }
    };

    let mut highest_water_mark_pnl = 0.0; // Track highest PnL achieved
    let mut current_pnl = 0.0;

    loop {
        tokio::time::sleep(Duration::from_secs(30)).await; // Check every 30 seconds

        let mut conn = match client.get_async_connection().await {
            Ok(c) => c,
            Err(e) => {
                warn!("Portfolio Monitor: Failed to connect to Redis: {}. Retrying in 5s.", e);
                tokio::time::sleep(Duration::from_secs(5)).await;
                continue;
            }
        };

        match db.get_total_pnl().await {
            Ok(total_pnl) => {
                current_pnl = total_pnl;
                highest_water_mark_pnl = highest_water_mark_pnl.max(current_pnl);

                let drawdown_from_peak = if highest_water_mark_pnl > 0.0 {
                    (highest_water_mark_pnl - current_pnl) / highest_water_mark_pnl * 100.0
                } else {
                    0.0 // No drawdown if no profit yet
                };

                info!(
                    "Portfolio PnL: {:.2} USD (Peak: {:.2} USD, Drawdown: {:.2}%)",
                    current_pnl, highest_water_mark_pnl, drawdown_from_peak
                );

                if drawdown_from_peak > CONFIG.portfolio_stop_loss_percent {
                    if !*portfolio_paused_flag.lock().await { // P-6: Check internal flag
                        error!(
                            "🚨 PORTFOLIO STOP LOSS TRIGGERED! Drawdown {:.2}% > Threshold {:.2}%. Pausing trading.",
                            drawdown_from_peak, CONFIG.portfolio_stop_loss_percent
                        );
                        // P-6: Publish to kill switch channel (Pub/Sub)
                        if let Err(e) = conn.publish("kill_switch_channel", "PAUSE").await {
                            error!("Failed to publish PAUSE to kill_switch_channel: {}", e);
                        }
                        *portfolio_paused_flag.lock().await = true; // P-6: Update internal flag
                    }
                } else if *portfolio_paused_flag.lock().await { // P-6: Check internal flag
                    // If currently paused but drawdown is recovered, resume
                    if drawdown_from_peak < CONFIG.portfolio_stop_loss_percent * 0.8 { // Resume if recovered significantly
                        info!("✅ Portfolio recovered. Drawdown {:.2}% < Threshold {:.2}%. Resuming trading.",
                            drawdown_from_peak, CONFIG.portfolio_stop_loss_percent * 0.8);
                        // P-6: Publish to kill switch channel (Pub/Sub)
                        if let Err(e) = conn.publish("kill_switch_channel", "RESUME").await {
                            error!("Failed to publish RESUME to kill_switch_channel: {}", e);
                        }
                        *portfolio_paused_flag.lock().await = false; // P-6: Update internal flag
                    }
                }
            }
            Err(e) => {
                error!("Portfolio Monitor: Failed to get total PnL from DB: {}", e);
            }
        }
    }
}
