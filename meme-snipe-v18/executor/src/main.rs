// executor/src/main.rs
mod config;
mod database;
mod executor;
mod jito_client; // Corrected module name
mod jupiter;
mod portfolio_monitor;
mod signer_client;
mod strategies;

use crate::config::CONFIG;
use anyhow::Result;
use axum::{routing::get, Router};
use database::Database;
use executor::MasterExecutor;
use prometheus::{Encoder, TextEncoder};
use std::sync::Arc;
use tracing::{info, level_filters::LevelFilter};
use tracing_subscriber::EnvFilter;
use axum::Json;
use serde_json::{json, Value};

async fn metrics_handler() -> String {
    let encoder = TextEncoder::new();
    let metric_families = prometheus::gather();
    let mut buffer = Vec::new();
    encoder.encode(&metric_families, &mut buffer).unwrap();
    String::from_utf8(buffer).unwrap()
}

async fn health_handler() -> &'static str {
    "OK"
}

async fn state_handler(axum::extract::State(executor): axum::extract::State<Arc<tokio::sync::Mutex<MasterExecutor>>>) -> Json<Value> {
    let executor = executor.lock().await;
    Json(executor.get_state_snapshot())
}

#[tokio::main]
async fn main() -> Result<()> {
    let filter = EnvFilter::builder()
        .with_default_directive(LevelFilter::INFO.into())
        .from_env_lossy();
    tracing_subscriber::fmt().with_env_filter(filter).init();

    info!(version = %env!("CARGO_PKG_VERSION"), "🚀 Starting MemeSnipe Executor Orchestrator v18 - The Alpha Engine...");

    let db = Arc::new(Database::new(&CONFIG.database_path)?);
    let master_executor = MasterExecutor::new(db.clone()).await?;
    let executor_state = Arc::new(tokio::sync::Mutex::new(master_executor));

    // Start Prometheus metrics server
    let metrics_app = Router::new()
        .route("/metrics", get(metrics_handler))
        .route("/health", get(health_handler))
        .route("/api/v1/state", get(state_handler))
        .with_state(executor_state.clone());

    let metrics_listener = tokio::net::TcpListener::bind("0.0.0.0:9090").await?;
    info!("📊 Prometheus metrics server listening on http://0.0.0.0:9090/metrics");

    tokio::spawn(async move {
        if let Err(e) = axum::serve(metrics_listener, metrics_app).await {
            tracing::error!("Metrics server error: {}", e);
        }
    });

    // Start the portfolio monitor task
    tokio::spawn(portfolio_monitor::run_monitor(
        db.clone(),
        executor_state.lock().await.paused_flag(),
    ));

    let mut executor = executor_state.lock().await;
    executor.run().await?;
    Ok(())
}
