// wallet_guard/src/main.rs
use anyhow::*;
use axum::{routing::get, Router, Json};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use std::{env, str::FromStr, time::Duration};
use tracing::{info, warn, error};

#[derive(Clone)]
struct App {
    rpc: RpcClient,
    wallet_pubkey: Pubkey,
    threshold_lamports: u64,
    redis_url: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    
    let solana_rpc_url = env::var("SOLANA_RPC_URL")
        .unwrap_or_else(|_| "https://api.mainnet-beta.solana.com".to_string());
    let wallet_address = env::var("WALLET_ADDRESS")
        .expect("WALLET_ADDRESS env var required");
    let redis_url = env::var("REDIS_URL")
        .unwrap_or_else(|_| "redis://redis:6379".to_string());
    
    let rpc = RpcClient::new(solana_rpc_url);
    let wallet_pubkey = Pubkey::from_str(&wallet_address)?;
    let threshold_lamports = 20_000_000; // 0.02 SOL
    
    let app = App {
        rpc,
        wallet_pubkey,
        threshold_lamports,
        redis_url: redis_url.clone(),
    };
    
    info!("🔒 Starting Wallet Guard on :7070...");
    info!("👛 Monitoring wallet: {}", wallet_address);
    info!("⚠️  Low balance threshold: {} SOL", threshold_lamports as f64 / 1e9);
    
    // Start background monitor
    let monitor_app = app.clone();
    tokio::spawn(async move {
        monitor_wallet(monitor_app).await;
    });
    
    // Start HTTP server
    let api = Router::new()
        .route("/balance", get(get_balance))
        .route("/health", get(health_check))
        .with_state(app);
    
    let listener = tokio::net::TcpListener::bind("0.0.0.0:7070").await?;
    axum::serve(listener, api).await?;
    
    Ok(())
}

async fn get_balance(
    axum::extract::State(app): axum::extract::State<App>
) -> Json<serde_json::Value> {
    match get_wallet_balance(&app).await {
        Ok(lamports) => {
            let sol = lamports as f64 / 1e9;
            Json(serde_json::json!({
                "sol": sol,
                "lamports": lamports,
                "wallet": app.wallet_pubkey.to_string(),
                "threshold_sol": app.threshold_lamports as f64 / 1e9,
                "status": if lamports >= app.threshold_lamports { "OK" } else { "LOW" }
            }))
        }
        Err(e) => {
            error!("Failed to get wallet balance: {}", e);
            Json(serde_json::json!({
                "error": format!("Failed to get balance: {}", e),
                "status": "ERROR"
            }))
        }
    }
}

async fn health_check() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "service": "wallet_guard",
        "status": "healthy"
    }))
}

async fn get_wallet_balance(app: &App) -> Result<u64> {
    let balance = app.rpc.get_balance(&app.wallet_pubkey).await?;
    Ok(balance)
}

async fn monitor_wallet(app: App) {
    info!("🔍 Starting wallet balance monitor...");
    
    loop {
        match get_wallet_balance(&app).await {
            Ok(balance) => {
                let sol_balance = balance as f64 / 1e9;
                
                if balance < app.threshold_lamports {
                    let msg = format!("🚨 WALLET LOW: {:.4} SOL (below {:.4} SOL threshold)", 
                                     sol_balance, app.threshold_lamports as f64 / 1e9);
                    warn!("{}", msg);
                    
                    // Send kill switch signal
                    if let Err(e) = send_kill_switch(&app.redis_url, "PAUSE_WALLET_LOW").await {
                        error!("Failed to send kill switch: {}", e);
                    }
                    
                    // Send alert
                    if let Err(e) = send_alert(&app.redis_url, &msg).await {
                        error!("Failed to send alert: {}", e);
                    }
                } else {
                    info!("💰 Wallet balance OK: {:.4} SOL", sol_balance);
                }
            }
            Err(e) => {
                error!("Failed to check wallet balance: {}", e);
            }
        }
        
        tokio::time::sleep(Duration::from_secs(30)).await;
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

async fn send_alert(redis_url: &str, message: &str) -> Result<()> {
    let client = redis::Client::open(redis_url)?;
    let mut conn = client.get_async_connection().await?;
    
    redis::cmd("PUBLISH")
        .arg("alerts")
        .arg(message)
        .query_async(&mut conn)
        .await?;
    
    Ok(())
}
