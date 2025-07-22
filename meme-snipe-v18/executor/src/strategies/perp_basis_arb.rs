use crate::{register_strategy, strategies::{Strategy, MarketEvent, StrategyAction, OrderDetails, EventType}};
use anyhow::Result;
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::Value;
use std::collections::{HashSet, HashMap};
use tracing::info;
use shared_models::Side; // P-5: Import Side

#[derive(Default, Deserialize)]
struct PerpBasisArb {
    basis_threshold_pct: f64,
    #[serde(skip)] spot_prices: HashMap<String, f64>,
    #[serde(skip)] funding_rates: HashMap<String, f64>,
}

#[async_trait]
impl Strategy for PerpBasisArb {
    fn id(&self) -> &'static str { "perp_basis_arb" }
    // This strategy now subscribes to Price (for spot) and Funding events
    fn subscriptions(&self) -> HashSet<EventType> {
        [EventType::Price, EventType::Funding].iter().cloned().collect()
    }

    async fn init(&mut self, params: &Value) -> Result<()> {
        #[derive(Deserialize)] struct P { basis_threshold_pct: f64 }
        let p: P = serde_json::from_value(params.clone())?;
        self.basis_threshold_pct = p.basis_threshold_pct;
        info!(strategy = self.id(), "Initialized with basis_threshold_pct: {}", self.basis_threshold_pct);
        Ok(())
    }

    async fn on_event(&mut self, event: &MarketEvent) -> Result<StrategyAction> {
        match event {
            MarketEvent::Price(tick) => {
                self.spot_prices.insert(tick.token_address.clone(), tick.price_usd);
            }
            MarketEvent::Funding(funding_event) => {
                self.funding_rates.insert(funding_event.token_address.clone(), funding_event.funding_rate_pct);
            }
            _ => {} // Ignore other event types
        }

        if let (Some(&spot_price), Some(&funding_rate_pct)) = (self.spot_prices.get(&event.token()), self.funding_rates.get(&event.token())) {
            // Simplified: Basis is directly the funding rate. A real basis would be (perp_price - spot_price) / spot_price
            let basis = funding_rate_pct; 

            if basis.abs() > self.basis_threshold_pct / 100.0 {
                if basis > 0.0 { // Positive basis: perp is more expensive, short perp & long spot
                    info!(id = self.id(), token = %event.token(), "SHORT PERP/LONG SPOT signal: Basis {:.4}% is above threshold. (Simulated)", basis * 100.0);
                    return Ok(StrategyAction::Execute(OrderDetails { // P-5: Use Execute
                        token_address: event.token().to_string(),
                        suggested_size_usd: 800.0,
                        confidence: 0.9,
                        side: Side::Short, // P-5: Add side (for the short leg)
                    }));
                    // A real strategy would also execute the long spot leg here
                } else { // Negative basis: perp is cheaper, long perp & short spot
                     info!(id = self.id(), token = %event.token(), "LONG PERP/SHORT SPOT signal: Basis {:.4}% is below threshold. (Simulated)", basis * 100.0);
                     return Ok(StrategyAction::Execute(OrderDetails { // P-5: Use Execute
                         token_address: event.token().to_string(),
                         suggested_size_usd: 800.0,
                         confidence: 0.9,
                         side: Side::Long, // P-5: Add side (for the long leg)
                     }));
                     // A real strategy would also execute the short spot leg here
                }
            }
        }
        Ok(StrategyAction::Hold)
    }
}
register_strategy!(PerpBasisArb, "perp_basis_arb");

// Helper to get token address from any MarketEvent
// This trait is now part of shared-models/src/lib.rs
