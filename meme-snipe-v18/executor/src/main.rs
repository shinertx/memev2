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
use database::Database;
use executor::MasterExecutor;
use std::sync::Arc;
use tracing::{info, level_filters::LevelFilter};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<()> {
    let filter = EnvFilter::builder()
        .with_default_directive(LevelFilter::INFO.into())
        .from_env_lossy();
    tracing_subscriber::fmt().with_env_filter(filter).init();

    info!(version = %env!("CARGO_PKG_VERSION"), "🚀 Starting MemeSnipe Executor Orchestrator v18 - The Alpha Engine...");

    let db = Arc::new(Database::new(&CONFIG.database_path)?);
    let mut master_executor = MasterExecutor::new(db.clone()).await?; // Await new() as it's async

    // Start the portfolio monitor task
    tokio::spawn(portfolio_monitor::run_monitor(db.clone(), master_executor.paused_flag()));

    master_executor.run().await?;
    Ok(())
}
