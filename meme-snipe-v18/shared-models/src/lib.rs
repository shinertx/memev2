// shared-models/src/lib.rs
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct StrategySpec {
    pub id: String,
    pub family: String,
    pub params: serde_json::Value,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct StrategyAllocation {
    pub id: String,
    pub weight: f64,
    pub sharpe_ratio: f64,
}

#[derive(Serialize, Deserialize, Debug, Clone, Hash, Eq, PartialEq)]
pub enum EventType {
    Price,
    Social,
    Depth,   // From a new depth-of-book feed
    Bridge,  // From a new bridge event feed
    Funding, // From a new perp funding rate feed
    OnChain, // Placeholder for future expansion (e.g., LP locks, holder changes)
    SolPrice, // P-2: For real-time SOL/USD price
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PriceTick {
    pub token_address: String,
    pub price_usd: f64,
    pub volume_usd_1m: f64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SocialMention {
    pub token_address: String,
    pub source: String, // "twitter", "telegram", etc.
    pub sentiment: f64, // -1.0 to 1.0
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DepthEvent {
    pub token_address: String,
    pub bid_price: f64,
    pub ask_price: f64,
    pub bid_size_usd: f64,
    pub ask_size_usd: f64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BridgeEvent {
    pub token_address: String,
    pub source_chain: String, // e.g., "ethereum"
    pub destination_chain: String, // e.g., "solana"
    pub volume_usd: f64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FundingEvent {
    pub token_address: String,
    pub funding_rate_pct: f64, // e.g., 0.01 for 1%
    pub next_funding_time_sec: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SolPriceEvent { // P-2: New event type for SOL/USD price
    pub price_usd: f64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type")]
pub enum MarketEvent {
    Price(PriceTick),
    Social(SocialMention),
    Depth(DepthEvent),
    Bridge(BridgeEvent),
    Funding(FundingEvent),
    SolPrice(SolPriceEvent), // P-2: New event variant
    // OnChain events would be added here
}

impl MarketEvent {
    pub fn get_type(&self) -> EventType {
        match self {
            MarketEvent::Price(_) => EventType::Price,
            MarketEvent::Social(_) => EventType::Social,
            MarketEvent::Depth(_) => EventType::Depth,
            MarketEvent::Bridge(_) => EventType::Bridge,
            MarketEvent::Funding(_) => EventType::Funding,
            MarketEvent::SolPrice(_) => EventType::SolPrice, // P-2
        }
    }
    // Helper to get token address from any MarketEvent
    pub fn token(&self) -> &str {
        match self {
            MarketEvent::Price(e) => &e.token_address,
            MarketEvent::Social(e) => &e.token_address,
            MarketEvent::Depth(e) => &e.token_address,
            MarketEvent::Bridge(e) => &e.token_address,
            MarketEvent::Funding(e) => &e.token_address,
            MarketEvent::SolPrice(_) => "So11111111111111111111111111111111111111112", // SOL mint address
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct OrderDetails {
    pub token_address: String,
    pub suggested_size_usd: f64,
    pub confidence: f64,
    pub side: Side,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum StrategyAction {
    Execute(OrderDetails),   // single unified action
    Hold,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum Side { Long, Short }


#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SignRequest { pub transaction_b64: String, }
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SignResponse { pub signed_transaction_b64: String }
