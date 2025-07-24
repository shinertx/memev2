use crate::{
    register_strategy,
    strategies::{EventType, MarketEvent, OrderDetails, Strategy, StrategyAction},
};
use anyhow::Result;
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::Value;
use shared_models::Side;
use std::collections::{HashSet, VecDeque};
use tracing::info; // P-5: Import Side

#[derive(Default, Deserialize)]
struct MeanRevert1h {
    period_hours: usize,
    z_score_threshold: f64,
    #[serde(skip)]
    price_history: VecDeque<f64>, // Stores prices for Z-score calculation
}

#[async_trait]
impl Strategy for MeanRevert1h {
    fn id(&self) -> &'static str {
        "mean_revert_1h"
    }
    fn subscriptions(&self) -> HashSet<EventType> {
        [EventType::Price].iter().cloned().collect()
    }

    async fn init(&mut self, params: &Value) -> Result<()> {
        #[derive(Deserialize)]
        struct P {
            period_hours: usize,
            z_score_threshold: f64,
        }
        let p: P = serde_json::from_value(params.clone())?;
        self.period_hours = p.period_hours;
        self.z_score_threshold = p.z_score_threshold;
        self.price_history = VecDeque::with_capacity(self.period_hours * 60); // Assuming 1-minute ticks
        info!(
            strategy = self.id(),
            "Initialized with period_hours: {}, z_score_threshold: {}",
            self.period_hours,
            self.z_score_threshold
        );
        Ok(())
    }

    async fn on_event(&mut self, event: &MarketEvent) -> Result<StrategyAction> {
        if let MarketEvent::Price(tick) = event {
            // Simplified: Add each tick. A real 1h strategy would aggregate to 1h candles.
            if self.price_history.len() == self.period_hours * 60 {
                self.price_history.pop_front();
            }
            self.price_history.push_back(tick.price_usd);

            if self.price_history.len() < self.period_hours * 60 {
                return Ok(StrategyAction::Hold);
            }

            let mean: f64 =
                self.price_history.iter().sum::<f64>() / (self.period_hours * 60) as f64;
            let std_dev = (self
                .price_history
                .iter()
                .map(|&p| (p - mean).powi(2))
                .sum::<f64>()
                / (self.period_hours * 60) as f64)
                .sqrt();

            if std_dev > 0.0 {
                let z_score = (tick.price_usd - mean) / std_dev;
                if z_score < -self.z_score_threshold {
                    // Buy when significantly oversold
                    info!(id = self.id(), token = %tick.token_address, "BUY signal: Price z-score {:.2} is below threshold -{:.2}", z_score, self.z_score_threshold);
                    return Ok(StrategyAction::Execute(
                        OrderDetails {
                            // P-5: Use Execute
                            token_address: tick.token_address.clone(),
                            suggested_size_usd: 300.0,
                            confidence: 0.6,
                            side: Side::Short, // P-5: Add side
                        },
                        TradeMode::Paper,
                    ));
                } else if z_score > self.z_score_threshold {
                    // Sell when significantly overbought
                    info!(id = self.id(), token = %tick.token_address, "SELL signal: Price z-score {:.2} is above threshold {:.2}", z_score, self.z_score_threshold);
                    return Ok(StrategyAction::Execute(OrderDetails {
                        // P-5: Use Execute
                        token_address: tick.token_address.clone(),
                        suggested_size_usd: 400.0, // Amount to sell
                        confidence: 0.7,
                        side: Side::Long, // P-5: Add side (for closing a long or opening a short)
                    }));
                }
            }
        }
        Ok(StrategyAction::Hold)
    }
}
register_strategy!(MeanRevert1h, "mean_revert_1h");
