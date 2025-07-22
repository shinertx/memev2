use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;
use shared_models::{EventType, MarketEvent, StrategyAction, Side}; // P-5: Import Side
use std::collections::HashSet;

#[async_trait]
pub trait Strategy: Send + Sync + 'static { // Added 'static bound
    fn id(&self) -> &'static str;
    fn subscriptions(&self) -> HashSet<EventType>;
    async fn init(&mut self, params: &Value) -> Result<()>;
    async fn on_event(&mut self, event: &MarketEvent) -> Result<StrategyAction>;
}

// Strategy constructor for dynamic loading
pub struct StrategyConstructor(pub &'static str, pub Box<dyn Fn() -> Box<dyn Strategy> + Send + Sync>);
inventory::collect!(StrategyConstructor);

// Macro to simplify registration in each strategy file
#[macro_export]
macro_rules! register_strategy {
    ($strat_type:ty, $id:expr) => {
        inventory::submit! {
            $crate::strategies::StrategyConstructor(
                $id,
                Box::new(|| Box::new(<$strat_type>::default()))
            )
        }
    };
}

// Import and declare all strategy modules
pub mod airdrop_rotation;
pub mod bridge_inflow;
pub mod dev_wallet_drain;
pub mod korean_time_burst;
pub mod liquidity_migration;
pub mod mean_revert_1h;
pub mod momentum_5m;
pub mod perp_basis_arb;
pub mod rug_pull_sniffer;
pub mod social_buzz;
