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
struct SocialBuzz {
    lookback_minutes: usize,
    std_dev_threshold: f64,
    #[serde(skip)]
    mention_counts_per_minute: VecDeque<u32>, // Each entry is mention count for one minute
}

#[async_trait]
impl Strategy for SocialBuzz {
    fn id(&self) -> &'static str {
        "social_buzz"
    }
    fn subscriptions(&self) -> HashSet<EventType> {
        [EventType::Social].iter().cloned().collect()
    }

    async fn init(&mut self, params: &Value) -> Result<()> {
        #[derive(Deserialize)]
        struct P {
            lookback_minutes: usize,
            std_dev_threshold: f64,
        }
        let p: P = serde_json::from_value(params.clone())?;
        self.lookback_minutes = p.lookback_minutes;
        self.std_dev_threshold = p.std_dev_threshold;
        self.mention_counts_per_minute = VecDeque::with_capacity(self.lookback_minutes);
        // Initialize with zeros or previous data to avoid false positives on startup
        for _ in 0..self.lookback_minutes {
            self.mention_counts_per_minute.push_back(0);
        }
        info!(
            strategy = self.id(),
            "Initialized with lookback: {}, std_dev_threshold: {}",
            self.lookback_minutes,
            self.std_dev_threshold
        );
        Ok(())
    }

    async fn on_event(&mut self, event: &MarketEvent) -> Result<StrategyAction> {
        if let MarketEvent::Social(mention) = event {
            // Simulate incrementing the current minute's count.
            // In a real system, `on_event` would be called with aggregated data
            // or this would be driven by a time-based tick.
            if let Some(last_count) = self.mention_counts_per_minute.back_mut() {
                *last_count += 1;
            } else {
                // Should not happen if initialized correctly
                self.mention_counts_per_minute.push_back(1);
            }

            if self.mention_counts_per_minute.len() < self.lookback_minutes {
                return Ok(StrategyAction::Hold);
            }

            let sum: u32 = self.mention_counts_per_minute.iter().sum();
            let mean = sum as f64 / self.lookback_minutes as f64;

            let variance: f64 = self
                .mention_counts_per_minute
                .iter()
                .map(|&count| (count as f64 - mean).powi(2))
                .sum::<f64>()
                / self.lookback_minutes as f64;
            let std_dev = variance.sqrt().max(0.1); // Avoid division by zero

            let current_minute_mentions =
                *self.mention_counts_per_minute.back().unwrap_or(&0) as f64;

            if current_minute_mentions > mean + self.std_dev_threshold * std_dev {
                info!(id = self.id(), token = %mention.token_address, "BUY signal: Social mention rate spike detected (current: {:.0}, mean: {:.1}, std_dev: {:.1}).", current_minute_mentions, mean, std_dev);
                return Ok(StrategyAction::Execute(
                    OrderDetails {
                        // P-5: Use Execute
                        token_address: mention.token_address.clone(),
                        suggested_size_usd: buzz_score * 10.0, // Scale position size with buzz score
                        confidence: 0.7,
                        side: Side::Long, // P-5: Add side
                    },
                    TradeMode::Paper,
                ));
            }
        }
        Ok(StrategyAction::Hold)
    }
}
register_strategy!(SocialBuzz, "social_buzz");
