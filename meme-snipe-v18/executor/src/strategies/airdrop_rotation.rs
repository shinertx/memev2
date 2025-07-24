use crate::{
    register_strategy,
    strategies::{EventType, MarketEvent, OrderDetails, Strategy, StrategyAction},
};
use anyhow::Result;
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::Value;
use shared_models::Side;
use std::collections::{HashMap, HashSet};
use tracing::info; // P-5: Import Side

#[derive(Default, Deserialize)]
struct AirdropRotation {
    min_new_holders: u32,
    #[serde(skip)]
    token_holder_counts: HashMap<String, u32>, // Simulated holder counts
}

#[async_trait]
impl Strategy for AirdropRotation {
    fn id(&self) -> &'static str {
        "airdrop_rotation"
    }
    // This strategy needs 'OnChain' events for holder delta.
    // For simulation, we'll react to a high social buzz combined with a price tick.
    fn subscriptions(&self) -> HashSet<EventType> {
        [EventType::Social, EventType::Price]
            .iter()
            .cloned()
            .collect()
    }

    async fn init(&mut self, params: &Value) -> Result<()> {
        #[derive(Deserialize)]
        struct P {
            min_new_holders: u32,
        }
        let p: P = serde_json::from_value(params.clone())?;
        self.min_new_holders = p.min_new_holders;
        info!(
            strategy = self.id(),
            "Initialized with min_new_holders: {}", self.min_new_holders
        );
        Ok(())
    }

    async fn on_event(&mut self, event: &MarketEvent) -> Result<StrategyAction> {
        if let MarketEvent::Social(mention) = event {
            // Simulate: A high social buzz might indicate new holder growth (like an airdrop causing buzz).
            if mention.sentiment > 0.5 {
                let current_holders = self
                    .token_holder_counts
                    .entry(mention.token_address.clone())
                    .or_insert(100);
                let new_holders_simulated = rand::random::<u32>() % 200 + 50; // Simulate 50-250 new holders
                *current_holders += new_holders_simulated;

                if new_holders_simulated > self.min_new_holders {
                    info!(id = self.id(), token = %mention.token_address, "BUY signal: Simulated airdrop detected with {} new holders.", new_holders_simulated);
                    return Ok(StrategyAction::Execute(
                        OrderDetails {
                            // P-5: Use Execute
                            token_address: mention.token_address.clone(),
                            suggested_size_usd: 600.0,
                            confidence: 0.7,
                            side: Side::Long, // P-5: Add side
                        },
                        TradeMode::Paper,
                    ));
                }
            }
        }
        Ok(StrategyAction::Hold)
    }
}
register_strategy!(AirdropRotation, "airdrop_rotation");
