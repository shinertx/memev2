// position_manager/src/main.rs
mod config;
mod database;
mod jupiter;
mod position_monitor;
mod signer_client; // Main logic for monitoring

use crate::config::CONFIG;
use anyhow::Result;
use database::Database;
use std::sync::Arc;
use tracing::{info, level_filters::LevelFilter};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<()> {
    let filter = EnvFilter::builder()
        .with_default_directive(LevelFilter::INFO.into())
        .from_env_lossy();
    tracing_subscriber::fmt().with_env_filter(filter).init();

    info!(version = %env!("CARGO_PKG_VERSION"), "📈 Starting MemeSnipe Position Manager v18...");

    let db = Arc::new(Database::new(&CONFIG.database_path)?);

    // Start the position monitoring loop
    position_monitor::run_monitor(db.clone()).await?;

    Ok(())
}
