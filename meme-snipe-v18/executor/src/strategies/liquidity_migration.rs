use crate::{register_strategy, strategies::{Strategy, MarketEvent, StrategyAction, OrderDetails, EventType}};
use anyhow::Result;
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashSet;
use tracing::info;
use shared_models::Side; // P-5: Import Side

#[derive(Default, Deserialize)]
struct LiquidityMigration {
    min_volume_migrate_usd: f64,
    #[serde(skip)] migrated_tokens: HashSet<String>, // Tracks tokens we've already traded for this migration
}

#[async_trait]
impl Strategy for LiquidityMigration {
    fn id(&self) -> &'static str { "liquidity_migration" }
    // This strategy now subscribes to the new, high-fidelity Bridge events
    fn subscriptions(&self) -> HashSet<EventType> {
        [EventType::Bridge].iter().cloned().collect()
    }

    async fn init(&mut self, params: &Value) -> Result<()> {
        #[derive(Deserialize)] struct P { min_volume_migrate_usd: f64 }
        let p: P = serde_json::from_value(params.clone())?;
        self.min_volume_migrate_usd = p.min_volume_migrate_usd;
        info!(strategy = self.id(), "Initialized with min_volume_migrate_usd: {}", self.min_volume_migrate_usd);
        Ok(())
    }

    async fn on_event(&mut self, event: &MarketEvent) -> Result<StrategyAction> {
        // The logic now reacts to the correct event type, not a noisy proxy.
        if let MarketEvent::Bridge(bridge_event) = event {
            if bridge_event.volume_usd > self.min_volume_migrate_usd && !self.migrated_tokens.contains(&bridge_event.token_address) {
                info!(
                    id = self.id(),
                    token = %bridge_event.token_address,
                    "BUY signal: Detected significant bridge inflow of {:.0} USD.",
                    bridge_event.volume_usd
                );
                self.migrated_tokens.insert(bridge_event.token_address.clone());
                return Ok(StrategyAction::Execute(OrderDetails { // P-5: Use Execute
                    token_address: bridge_event.token_address.clone(),
                    suggested_size_usd: 700.0,
                    confidence: 0.85,
                    side: Side::Long, // P-5: Add side
                }));
            }
        }
        Ok(StrategyAction::Hold)
    }
}
register_strategy!(LiquidityMigration, "liquidity_migration");
