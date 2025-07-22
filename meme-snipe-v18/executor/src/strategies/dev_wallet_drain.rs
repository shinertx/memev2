use crate::{register_strategy, strategies::{Strategy, MarketEvent, StrategyAction, OrderDetails, EventType}};
use anyhow::Result;
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashSet;
use tracing::info;
use shared_models::Side; // P-5: Import Side

#[derive(Default, Deserialize)]
struct DevWalletDrain {
    dev_balance_threshold_pct: f64,
    #[serde(skip)] monitored_dev_wallets: HashSet<String>, // In a real system, this would be populated from on-chain data
}

#[async_trait]
impl Strategy for DevWalletDrain {
    fn id(&self) -> &'static str { "dev_wallet_drain" }
    // This strategy needs 'OnChain' events detailing wallet balance changes.
    // For simulation, we'll trigger a short on high volume and sharp price drop.
    fn subscriptions(&self) -> HashSet<EventType> { [EventType::Price].iter().cloned().collect() }

    async fn init(&mut self, params: &Value) -> Result<()> {
        #[derive(Deserialize)] struct P { dev_balance_threshold_pct: f64 }
        let p: P = serde_json::from_value(params.clone())?;
        self.dev_balance_threshold_pct = p.dev_balance_threshold_pct;
        info!(strategy = self.id(), "Initialized with dev_balance_threshold_pct: {}", self.dev_balance_threshold_pct);
        Ok(())
    }

    async fn on_event(&mut self, event: &MarketEvent) -> Result<StrategyAction> {
        if let MarketEvent::Price(tick) = event {
            // Simulate: If price drops sharply with very high volume, it could be a dev dump.
            // A real strategy would monitor specific known dev wallet addresses and their outflows.
            if tick.price_usd < 0.2 && tick.volume_usd_1m > 200_000.0 {
                 info!(id = self.id(), token = %tick.token_address, "SHORT signal: Possible dev wallet dump detected (simulated price crash + high volume).");
                 return Ok(StrategyAction::Execute(OrderDetails { // P-5: Use Execute
                     token_address: tick.token_address.clone(),
                     suggested_size_usd: 1200.0,
                     confidence: 0.85,
                     side: Side::Short, // P-5: Add side
                 }));
            }
        }
        Ok(StrategyAction::Hold)
    }
}
register_strategy!(DevWalletDrain, "dev_wallet_drain");
