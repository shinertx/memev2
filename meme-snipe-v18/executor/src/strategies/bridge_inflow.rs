use crate::{
    register_strategy,
    strategies::{EventType, MarketEvent, OrderDetails, Strategy, StrategyAction},
};
use anyhow::Result;
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::Value;
use shared_models::Side;
use std::collections::HashSet;
use tracing::info; // P-5: Import Side

#[derive(Default, Deserialize)]
struct BridgeInflow {
    min_bridge_volume_usd: f64,
    #[serde(skip)]
    tokens_with_recent_inflow: HashSet<String>,
}

#[async_trait]
impl Strategy for BridgeInflow {
    fn id(&self) -> &'static str {
        "bridge_inflow"
    }
    // This strategy now subscribes to the new, high-fidelity Bridge events
    fn subscriptions(&self) -> HashSet<EventType> {
        [EventType::Bridge].iter().cloned().collect()
    }

    async fn init(&mut self, params: &Value) -> Result<()> {
        #[derive(Deserialize)]
        struct P {
            min_bridge_volume_usd: f64,
        }
        let p: P = serde_json::from_value(params.clone())?;
        self.min_bridge_volume_usd = p.min_bridge_volume_usd;
        info!(
            strategy = self.id(),
            "Initialized with min_bridge_volume_usd: {}", self.min_bridge_volume_usd
        );
        Ok(())
    }

    async fn on_event(&mut self, event: &MarketEvent) -> Result<StrategyAction> {
        // The logic now reacts to the correct event type, not a noisy proxy.
        if let MarketEvent::Bridge(bridge_event) = event {
            if bridge_event.volume_usd > self.min_bridge_volume_usd
                && !self
                    .tokens_with_recent_inflow
                    .contains(&bridge_event.token_address)
            {
                info!(
                    id = self.id(),
                    token = %bridge_event.token_address,
                    "BUY signal: Detected significant bridge inflow of {:.0} USD.",
                    bridge_event.volume_usd
                );
                self.tokens_with_recent_inflow
                    .insert(bridge_event.token_address.clone());
                return Ok(StrategyAction::Execute(
                    OrderDetails {
                        // P-5: Use Execute
                        token_address: tick.token_address.clone(),
                        suggested_size_usd: bridge_size_multiplier * 300.0,
                        confidence: 0.8,
                        side: Side::Long, // P-5: Add side
                    },
                    TradeMode::Paper,
                ));
            }
        }
        Ok(StrategyAction::Hold)
    }
}
register_strategy!(BridgeInflow, "bridge_inflow");
