use anyhow::Result;
use redis::AsyncCommands;
use shared_models::{StrategyAllocation, StrategySpec};
use std::collections::HashMap;
use std::time::Duration;
use tracing::{info, warn, level_filters::LevelFilter};
use tracing_subscriber::EnvFilter;

// Simple statistical functions to avoid heavy dependencies
fn mean(values: &[f64]) -> f64 {
    if values.is_empty() { 0.0 } else { values.iter().sum::<f64>() / values.len() as f64 }
}

fn std_dev(values: &[f64]) -> f64 {
    if values.len() < 2 { 0.0 } else {
        let m = mean(values);
        let variance = values.iter().map(|x| (x - m).powi(2)).sum::<f64>() / (values.len() - 1) as f64;
        variance.sqrt()
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let filter = EnvFilter::builder()
        .with_default_directive(LevelFilter::INFO.into())
        .from_env_lossy();
    tracing_subscriber::fmt().with_env_filter(filter).init();
    
    info!("🚀 Starting Meta-Allocator v18...");

    let redis_url = std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://redis:6379".to_string());
    let client = redis::Client::open(redis_url)?;

    // P-7: For Redis Streams
    let mut strategy_registry_stream_id = HashMap::new();
    strategy_registry_stream_id.insert("strategy_registry_stream".to_string(), "0".to_string()); // Start from beginning

    loop {
        info!("Allocator loop starting...");
        let mut conn = match client.get_async_connection().await {
            Ok(c) => c,
            Err(e) => {
                warn!("Failed to connect to Redis: {}. Retrying in 10s.", e);
                tokio::time::sleep(Duration::from_secs(10)).await;
                continue;
            }
        };

        info!("Checking strategy registry for new specs...");

        // P-7: Read from strategy registry stream
        let mut specs: Vec<StrategySpec> = Vec::new();
        match conn.xread_map(&strategy_registry_stream_id, &[("strategy_registry_stream", ">")]).await {
            Ok(streams) => {
                for (_, messages) in streams {
                    for (id, payload) in messages {
                        if let Some(spec_json) = payload.get("spec") {
                            if let Ok(spec) = serde_json::from_slice::<StrategySpec>(spec_json) {
                                specs.push(spec);
                            } else {
                                warn!("Failed to deserialize strategy spec from stream ID {}: {:?}", id, String::from_utf8_lossy(spec_json));
                            }
                        }
                        strategy_registry_stream_id.insert("strategy_registry_stream".to_string(), String::from_utf8_lossy(&id.id).to_string()); // Update last read ID
                    }
                }
            }
            Err(e) => warn!("Error reading from strategy_registry_stream: {}", e),
        }

        if specs.is_empty() {
            warn!("No valid strategy specs found in registry. Waiting...");
            tokio::time::sleep(Duration::from_secs(30)).await;
            continue;
        }

        // 1. Get performance data for each strategy
        let mut strategy_metrics = HashMap::new();
        for spec in &specs {
            let pnl_history_key = format!("perf:{}:pnl_history", spec.id);
            // P-7: Read from PnL history stream
            let pnl_history_stream_data: Vec<HashMap<String, Vec<u8>>> = conn.xrange_map(&pnl_history_key, "-", "+").await.unwrap_or_default();
            
            let pnl_values: Vec<f64> = pnl_history_stream_data.into_iter()
                .filter_map(|mut entry| {
                    entry.remove("pnl").and_then(|pnl_bytes| {
                        String::from_utf8(pnl_bytes).ok().and_then(|s| s.parse::<f64>().ok())
                    })
                })
                .collect();

            if pnl_values.len() > 1 {
                let mean_pnl = mean(&pnl_values);
                let std_dev_pnl = std_dev(&pnl_values);
                
                // Calculate Sharpe Ratio (simplified: uses mean PnL as excess return, std dev as risk)
                // A true Sharpe would use daily returns and risk-free rate
                let sharpe_ratio = if std_dev_pnl > 0.0 { mean_pnl / std_dev_pnl } else { 0.0 };
                strategy_metrics.insert(spec.id.clone(), (mean_pnl, sharpe_ratio));
            } else {
                strategy_metrics.insert(spec.id.clone(), (0.0, 0.0)); // No data yet
            }
        }

        // 2. Calculate weights based on Sharpe Ratio (and PnL for tie-breaking)
        let mut sorted_strategies: Vec<&StrategySpec> = specs.iter().collect();
        sorted_strategies.sort_by(|a, b| {
            let (pnl_a, sharpe_a) = strategy_metrics.get(&a.id).unwrap_or(&(0.0, 0.0));
            let (pnl_b, sharpe_b) = strategy_metrics.get(&b.id).unwrap_or(&(0.0, 0.0));
            
            sharpe_b.partial_cmp(sharpe_a) // Higher Sharpe first
                .unwrap_or_else(|| pnl_b.partial_cmp(pnl_a).unwrap_or(std::cmp::Ordering::Equal)) // Then higher PnL
        });

        let mut allocations: Vec<StrategyAllocation> = Vec::new();
        let mut total_sharpe_for_weighting = 0.0;
        for spec in sorted_strategies.iter() { // Iterate over sorted_strategies directly
            let (_, sharpe) = strategy_metrics.get(&spec.id).unwrap_or(&(0.0, 0.0));
            // Only consider positive Sharpe ratios for weighting, or a small base weight for new strategies
            let weight_factor = sharpe.max(0.1); // Give a floor to new/low-sharpe strategies
            total_sharpe_for_weighting += weight_factor;
        }

        for spec in sorted_strategies {
            let (_, sharpe) = strategy_metrics.get(&spec.id).unwrap_or(&(0.0, 0.0));
            let weight = if total_sharpe_for_weighting > 0.0 {
                (sharpe.max(0.1)) / total_sharpe_for_weighting
            } else {
                1.0 / specs.len() as f64 // Fallback if no positive sharpe sum
            };
            
            allocations.push(StrategyAllocation { id: spec.id.clone(), weight, sharpe_ratio: *sharpe });
        }

        info!("Publishing {} allocations with dynamic Sharpe-based weights.", allocations.len());
        let payload = serde_json::to_string(&allocations)?;
        
        // Store current allocations for dashboard
        conn.set("active_allocations", &payload).await?; 
        // P-7: Publish to allocations_channel stream
        if let Err(e) = conn.xadd("allocations_channel", "*", &[("allocations", payload.as_bytes())]).await {
            warn!("Failed to publish allocations to stream: {}.", e);
        }

        tokio::time::sleep(Duration::from_secs(60)).await;
    }
}
