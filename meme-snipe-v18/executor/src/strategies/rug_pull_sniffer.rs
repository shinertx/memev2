use crate::{register_strategy, strategies::{Strategy, MarketEvent, StrategyAction, OrderDetails, EventType}};
use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashSet;
use tracing::info;
use shared_models::Side; // P-5: Import Side

#[derive(Default)]
struct RugPullSniffer;

#[async_trait]
impl Strategy for RugPullSniffer {
    fn id(&self) -> &'static str { "rug_pull_sniffer" }
    // This strategy would ideally subscribe to 'OnChain' events with LP lock/dev wallet info.
    // For this simulation, we'll use price/volume characteristics of a crash.
    fn subscriptions(&self) -> HashSet<EventType> { [EventType::Price].iter().cloned().collect() }

    async fn init(&mut self, _params: &Value) -> Result<()> { 
        info!(strategy = self.id(), "Initialized.");
        Ok(()) 
    }

    async fn on_event(&mut self, event: &MarketEvent) -> Result<StrategyAction> {
        if let MarketEvent::Price(tick) = event {
            // Simulate: A very sharp, high-volume price drop (e.g., price below $0.10 with high volume)
            // A real rug pull sniffer would integrate with on-chain data for LP unlocks, dev wallet activity, etc.
            if tick.price_usd < 0.1 && tick.volume_usd_1m > 100_000.0 {
                 info!(id = self.id(), token = %tick.token_address, "SHORT signal: Detected potential rug pull pattern (price crash with high volume).");
                 return Ok(StrategyAction::Execute(OrderDetails { // P-5: Use Execute
                     token_address: tick.token_address.clone(),
                     suggested_size_usd: 1000.0, // Aggressive short size
                     confidence: 0.9,
                     side: Side::Short, // P-5: Add side
                 }));
            }
        }
        Ok(StrategyAction::Hold)
    }
}
register_strategy!(RugPullSniffer, "rug_pull_sniffer");
