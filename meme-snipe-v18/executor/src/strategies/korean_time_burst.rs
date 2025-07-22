use crate::{register_strategy, strategies::{Strategy, MarketEvent, StrategyAction, OrderDetails, EventType}};
use anyhow::Result;
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashSet;
use tracing::info;
use chrono::{Timelike, Utc, NaiveTime};
use shared_models::Side; // P-5: Import Side

#[derive(Default, Deserialize)]
struct KoreanTimeBurst {
    volume_multiplier_threshold: f64,
    #[serde(skip)] active_burst_tokens: HashSet<String>, // To avoid multiple buys on the same burst
}

#[async_trait]
impl Strategy for KoreanTimeBurst {
    fn id(&self) -> &'static str { "korean_time_burst" }
    fn subscriptions(&self) -> HashSet<EventType> { [EventType::Price].iter().cloned().collect() }

    async fn init(&mut self, params: &Value) -> Result<()> {
        #[derive(Deserialize)] struct P { volume_multiplier_threshold: f64 }
        let p: P = serde_json::from_value(params.clone())?;
        self.volume_multiplier_threshold = p.volume_multiplier_threshold;
        info!(strategy = self.id(), "Initialized with volume_multiplier_threshold: {}", self.volume_multiplier_threshold);
        Ok(())
    }

    async fn on_event(&mut self, event: &MarketEvent) -> Result<StrategyAction> {
        if let MarketEvent::Price(tick) = event {
            let now = Utc::now().with_timezone(&chrono_tz::Asia::Seoul);
            let hour = now.hour();

            // KST 09:00-11:00 corresponds to UTC 00:00-02:00 if no DST difference, or 01:00-03:00 if UTC+9
            // Simplified check: if it's "Korean business hours" in UTC (for simulator)
            let is_korean_trading_hour = hour >= 0 && hour < 3; // Approx 9 AM - 12 PM KST in UTC

            if is_korean_trading_hour {
                // This would need historical average volume for the specific token.
                // For simulation, we'll use a high absolute volume threshold.
                if tick.volume_usd_1m > 50_000.0 * self.volume_multiplier_threshold && !self.active_burst_tokens.contains(&tick.token_address) {
                    info!(id = self.id(), token = %tick.token_address, "BUY signal: Detected Korean time volume burst (V: {:.0} USD).", tick.volume_usd_1m);
                    self.active_burst_tokens.insert(tick.token_address.clone());
                    return Ok(StrategyAction::Execute(OrderDetails { // P-5: Use Execute
                        token_address: tick.token_address.clone(),
                        suggested_size_usd: 650.0,
                        confidence: 0.7,
                        side: Side::Long, // P-5: Add side
                    }));
                }
            }
        }
        Ok(StrategyAction::Hold)
    }
}
register_strategy!(KoreanTimeBurst, "korean_time_burst");
