use crate::register_strategy;
use crate::strategies::{EventType, MarketEvent, OrderDetails, Strategy, StrategyAction};
use anyhow::Result;
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::Value;
use shared_models::{Side, TradeMode};
use std::collections::{HashSet, VecDeque};
use tracing::info;

#[derive(Default, Deserialize)]
struct Momentum5m {
    lookback: usize,
    vol_multiplier: f64,
    price_change_threshold: f64,
    #[serde(skip)]
    price_history: VecDeque<f64>,
    #[serde(skip)]
    volume_history: VecDeque<f64>,
    #[serde(skip)]
    current_mode: TradeMode,
}

#[async_trait]
impl Strategy for Momentum5m {
    fn id(&self) -> &'static str {
        "momentum_5m"
    }
    fn subscriptions(&self) -> HashSet<EventType> {
        [EventType::Price].iter().cloned().collect()
    }

    async fn init(&mut self, params: &Value) -> Result<()> {
        #[derive(Deserialize)]
        struct P {
            lookback: usize,
            vol_multiplier: f64,
            price_change_threshold: f64,
        }
        let p: P = serde_json::from_value(params.clone())?;
        self.lookback = p.lookback;
        self.vol_multiplier = p.vol_multiplier;
        self.price_change_threshold = p.price_change_threshold;
        self.price_history = VecDeque::with_capacity(self.lookback);
        self.volume_history = VecDeque::with_capacity(self.lookback);
        self.current_mode = TradeMode::Paper; // Start in paper mode
        info!(
            strategy = self.id(),
            "Initialized with lookback: {}, vol_multiplier: {}, price_change_threshold: {}",
            self.lookback,
            self.vol_multiplier,
            self.price_change_threshold
        );
        Ok(())
    }

    async fn on_event(&mut self, event: &MarketEvent) -> Result<StrategyAction> {
        if let MarketEvent::Price(tick) = event {
            if self.price_history.len() == self.lookback {
                self.price_history.pop_front();
            }
            if self.volume_history.len() == self.lookback {
                self.volume_history.pop_front();
            }
            self.price_history.push_back(tick.price_usd);
            self.volume_history.push_back(tick.volume_usd_1m);

            if self.price_history.len() < self.lookback {
                return Ok(StrategyAction::Hold);
            }

            let avg_volume = self.volume_history.iter().sum::<f64>() / self.lookback as f64;
            let old_price = self.price_history.front().unwrap_or(&tick.price_usd);
            let price_change = (tick.price_usd - old_price) / old_price;

            if price_change > self.price_change_threshold
                && tick.volume_usd_1m > avg_volume * self.vol_multiplier
            {
                info!(id = self.id(), token = %tick.token_address, "BUY signal: Price change {:.2}% > threshold and Volume spike > {:.1}x", price_change * 100.0, self.vol_multiplier);
                return Ok(StrategyAction::Execute(OrderDetails {
                    token_address: tick.token_address.clone(),
                    suggested_size_usd: 500.0,
                    confidence: 0.75,
                    side: Side::Long,
                }));
            }
        }
        Ok(StrategyAction::Hold)
    }
}
register_strategy!(Momentum5m, "momentum_5m");
