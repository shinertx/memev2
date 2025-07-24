use crate::{
    register_strategy,
    strategies::{EventType, MarketEvent, OrderDetails, Strategy, StrategyAction, TradeMode},
};
use anyhow::Result;
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{json, Value};
use shared_models::Side;
use std::collections::HashSet;
use tracing::info;

#[derive(Default, Deserialize)]
struct LiquidityMigration {
    min_volume_migrate_usd: f64,
    #[serde(skip)]
    migrated_tokens: HashSet<String>, // Tracks tokens we've already traded for this migration
}

#[async_trait]
impl Strategy for LiquidityMigration {
    fn id(&self) -> &'static str {
        "liquidity_migration"
    }
    // This strategy now subscribes to the new, high-fidelity Bridge events
    fn subscriptions(&self) -> HashSet<EventType> {
        [EventType::Bridge].iter().cloned().collect()
    }

    async fn init(&mut self, params: &Value) -> Result<()> {
        #[derive(Deserialize)]
        struct P {
            min_volume_migrate_usd: f64,
        }
        let p: P = serde_json::from_value(params.clone())?;
        self.min_volume_migrate_usd = p.min_volume_migrate_usd;
        info!(
            strategy = self.id(),
            "Initialized with min_volume_migrate_usd: {}", self.min_volume_migrate_usd
        );
        Ok(())
    }

    async fn on_event(&mut self, event: &MarketEvent) -> Result<StrategyAction> {
        // The logic now reacts to the correct event type, not a noisy proxy.
        if let MarketEvent::Bridge(bridge_event) = event {
            if bridge_event.volume_usd > self.min_volume_migrate_usd
                && !self.migrated_tokens.contains(&bridge_event.token_address)
            {
                info!(
                    id = self.id(),
                    token = %bridge_event.token_address,
                    "BUY signal: Detected significant bridge inflow of {:.0} USD.",
                    bridge_event.volume_usd
                );
                self.migrated_tokens
                    .insert(bridge_event.token_address.clone());

                // Capture triggering features for analysis
                let features = json!({
                    "bridge_volume_usd": bridge_event.volume_usd,
                    "source_chain": bridge_event.source_chain,
                    "min_volume_migrate_usd": self.min_volume_migrate_usd,
                });

                return Ok(StrategyAction::Execute(
                    OrderDetails {
                        token_address: event.token().to_string(),
                        // FIXED: undefined `liquidity_proportion`. Using a fixed size.
                        suggested_size_usd: 1000.0,
                        confidence: 0.75,
                        side: Side::Long,
                        // ADDED: new fields for enhanced data collection and control
                        limit_price: None, // This strategy is a market taker
                        triggering_features: Some(features),
                    },
                    TradeMode::Paper,
                ));
            }
        }
        Ok(StrategyAction::Hold)
    }
}
register_strategy!(LiquidityMigration, "liquidity_migration");
