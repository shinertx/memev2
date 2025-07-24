// risk_guardian/src/main.rs
use anyhow::*;
use axum::{routing::get, Router, Json};
use redis::AsyncCommands;
use shared_models::{alert, StrategyAllocation};
use std::collections::HashMap;
use std::env;
use tracing::{info, warn, error};
use chrono::{DateTime, Utc, Duration};

#[derive(serde::Deserialize, serde::Serialize, Clone)]
struct RiskMetrics {
    total_exposure_usd: f64,
    daily_var_95: f64, // Value at Risk at 95% confidence
    max_drawdown_pct: f64,
    position_count: u32,
    last_updated: DateTime<Utc>,
}

#[derive(Clone)]
struct App {
    redis_url: String,
    max_portfolio_var: f64,
    max_daily_loss_usd: f64,
    max_position_count: u32,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    
    let redis_url = env::var("REDIS_URL")
        .unwrap_or_else(|_| "redis://redis:6379".to_string());
    let max_portfolio_var = env::var("MAX_PORTFOLIO_VAR")
        .unwrap_or_else(|_| "10000.0".to_string())
        .parse::<f64>()
        .unwrap_or(10000.0); // $10k max VaR
    let max_daily_loss_usd = env::var("MAX_DAILY_LOSS_USD")
        .unwrap_or_else(|_| "5000.0".to_string())
        .parse::<f64>()
        .unwrap_or(5000.0); // $5k max daily loss
    let max_position_count = env::var("MAX_POSITION_COUNT")
        .unwrap_or_else(|_| "50".to_string())
        .parse::<u32>()
        .unwrap_or(50); // Max 50 positions
    
    let app = App {
        redis_url: redis_url.clone(),
        max_portfolio_var,
        max_daily_loss_usd,
        max_position_count,
    };
    
    info!("🛡️  Starting Risk Guardian on :7200...");
    info!("📊 Max Portfolio VaR: ${:.0}", max_portfolio_var);
    info!("📉 Max Daily Loss: ${:.0}", max_daily_loss_usd);
    info!("📈 Max Position Count: {}", max_position_count);
    
    // Start background risk monitor
    let monitor_app = app.clone();
    tokio::spawn(async move {
        monitor_portfolio_risk(monitor_app).await;
    });
    
    // Start HTTP server
    let api = Router::new()
        .route("/risk", get(get_risk_metrics))
        .route("/health", get(health_check))
        .with_state(app);
    
    let listener = tokio::net::TcpListener::bind("0.0.0.0:7200").await?;
    axum::serve(listener, api).await?;
    
    Ok(())
}

async fn get_risk_metrics(
    axum::extract::State(app): axum::extract::State<App>
) -> Json<serde_json::Value> {
    match calculate_portfolio_risk(&app).await {
        Ok(metrics) => {
            Json(serde_json::json!({
                "totalExposureUsd": metrics.total_exposure_usd,
                "dailyVar95": metrics.daily_var_95,
                "maxDrawdownPct": metrics.max_drawdown_pct,
                "positionCount": metrics.position_count,
                "lastUpdated": metrics.last_updated,
                "limits": {
                    "maxPortfolioVar": app.max_portfolio_var,
                    "maxDailyLossUsd": app.max_daily_loss_usd,
                    "maxPositionCount": app.max_position_count
                },
                "status": if metrics.daily_var_95 > app.max_portfolio_var { "OVER_LIMIT" } else { "OK" }
            }))
        }
        Err(e) => {
            error!("Failed to calculate risk metrics: {}", e);
            Json(serde_json::json!({
                "error": format!("Failed to calculate risk: {}", e),
                "status": "ERROR"
            }))
        }
    }
}

async fn health_check() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "service": "risk_guardian",
        "status": "healthy"
    }))
}

async fn calculate_portfolio_risk(app: &App) -> Result<RiskMetrics> {
    let client = redis::Client::open(&app.redis_url)?;
    let mut conn = client.get_async_connection().await?;
    
    // Get current allocations
    let allocations_json: Option<String> = conn.get("active_allocations").await?;
    let allocations: Vec<StrategyAllocation> = if let Some(json) = allocations_json {
        serde_json::from_str(&json)?
    } else {
        Vec::new()
    };
    
    // Calculate total exposure (simplified)
    let total_exposure_usd = allocations.iter()
        .filter(|a| a.is_live()) // Only count live allocations
        .map(|a| a.weight * 10000.0) // Assume $10k base allocation per strategy
        .sum::<f64>();
    
    // Simplified VaR calculation (in practice, would use historical returns)
    let daily_var_95 = total_exposure_usd * 0.05; // 5% of total exposure as VaR estimate
    
    // Get position count from active trades - check multiple sources
    let position_count: u32 = {
        // Try getting from positions hash
        let positions_count: u32 = conn.hlen("positions").await.unwrap_or(0) as u32;
        if positions_count > 0 {
            positions_count
        } else {
            // Fallback to active_position_count key
            conn.get("active_position_count").await.unwrap_or(0)
        }
    };
    
    // Get real portfolio value from Redis if available
    let portfolio_value: f64 = conn.hget("portfolio_metrics", "total_value_usd").await.unwrap_or(total_exposure_usd);
    
    // Get real daily PnL if available
    let daily_pnl: f64 = conn.hget("portfolio_metrics", "daily_pnl_usd").await.unwrap_or(0.0);
    
    // Calculate VaR based on portfolio value and position volatility
    let daily_var_95 = if portfolio_value > 0.0 {
        portfolio_value * 0.05 // 5% of portfolio value
    } else {
        total_exposure_usd * 0.05 // Fallback to exposure-based calculation
    };
    
    // Calculate max drawdown from daily PnL if negative
    let max_drawdown_pct = if daily_pnl < 0.0 && portfolio_value > 0.0 {
        (daily_pnl.abs() / portfolio_value) * 100.0
    } else {
        0.0
    };
    
    Ok(RiskMetrics {
        total_exposure_usd,
        daily_var_95,
        max_drawdown_pct,
        position_count,
        last_updated: Utc::now(),
    })
}

async fn monitor_portfolio_risk(app: App) {
    info!("🔍 Starting portfolio risk monitor...");
    
    loop {
        match calculate_portfolio_risk(&app).await {
            Ok(metrics) => {
                let client = redis::Client::open(&app.redis_url).unwrap();
                let mut conn = client.get_async_connection().await.unwrap();
                
                // Check VaR limit
                if metrics.daily_var_95 > app.max_portfolio_var {
                    let msg = format!("🚨 PORTFOLIO VAR BREACH: ${:.0} exceeds limit of ${:.0}", 
                                     metrics.daily_var_95, app.max_portfolio_var);
                    warn!("{}", msg);
                    
                    // Send kill switch
                    if let Err(e) = send_kill_switch(&app.redis_url, "PAUSE_VAR_BREACH").await {
                        error!("Failed to send VaR kill switch: {}", e);
                    }
                    
                    // Send alert
                    alert!(conn, "{}", msg).await;
                }
                
                // Check position count limit
                if metrics.position_count > app.max_position_count {
                    let msg = format!("⚠️  POSITION COUNT HIGH: {} exceeds limit of {}", 
                                     metrics.position_count, app.max_position_count);
                    warn!("{}", msg);
                    alert!(conn, "{}", msg).await;
                }
                
                // Store risk metrics
                let metrics_json = serde_json::to_string(&metrics).unwrap_or_default();
                if let Err(e) = conn.set::<&str, &str, ()>("portfolio_risk_metrics", &metrics_json).await {
                    error!("Failed to store risk metrics: {}", e);
                }
                
                info!("💰 Portfolio VaR: ${:.0} | Positions: {} | Exposure: ${:.0}", 
                      metrics.daily_var_95, metrics.position_count, metrics.total_exposure_usd);
            }
            Err(e) => {
                error!("Failed to calculate portfolio risk: {}", e);
            }
        }
        
        tokio::time::sleep(std::time::Duration::from_secs(60)).await; // Check every minute
    }
}

async fn send_kill_switch(redis_url: &str, message: &str) -> Result<()> {
    let client = redis::Client::open(redis_url)?;
    let mut conn = client.get_async_connection().await?;
    
    redis::cmd("PUBLISH")
        .arg("kill_switch_channel")
        .arg(message)
        .query_async(&mut conn)
        .await?;
    
    Ok(())
}
