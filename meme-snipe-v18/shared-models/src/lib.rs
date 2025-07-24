// shared-models/src/lib.rs
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Trading mode for an allocation – determines whether orders are routed
/// to the signer (Live) or only simulated in-process (Paper).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TradeMode {
    Paper,
    Live,
}

pub fn default_trade_mode() -> TradeMode {
    TradeMode::Paper
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct StrategySpec {
    pub id: String,
    pub family: String,
    pub params: serde_json::Value,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct StrategyAllocation {
    pub id: String,
    pub weight: f64,
    pub sharpe_ratio: f64,
    /// NEW – defaults to `Paper` until the allocator upgrades it.
    #[serde(default = "default_trade_mode")]
    pub mode: TradeMode,
}

impl StrategyAllocation {
    pub fn is_live(&self) -> bool {
        self.mode == TradeMode::Live
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Hash, Eq, PartialEq)]
pub enum EventType {
    Price,
    Social,
    Depth,    // From a new depth-of-book feed
    Bridge,   // From a new bridge event feed
    Funding,  // From a new perp funding rate feed
    OnChain,  // Placeholder for future expansion (e.g., LP locks, holder changes)
    SolPrice, // P-2: For real-time SOL/USD price
    DataSourceHeartbeat, // For monitoring data consumer health
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PriceTick {
    pub timestamp: i64,
    pub token_address: String,
    pub price_usd: f64,
    pub volume_usd_1m: f64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SocialMention {
    pub timestamp: i64,
    pub token_address: String,
    pub source: String, // "twitter", "telegram", etc.
    pub sentiment: f64, // -1.0 to 1.0
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DepthEvent {
    pub timestamp: i64,
    pub token_address: String,
    pub bid_price: f64,
    pub ask_price: f64,
    pub bid_size_usd: f64,
    pub ask_size_usd: f64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BridgeEvent {
    pub timestamp: i64,
    pub token_address: String,
    pub source_chain: String,      // e.g., "ethereum"
    pub destination_chain: String, // e.g., "solana"
    pub volume_usd: f64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FundingEvent {
    pub timestamp: i64,
    pub token_address: String,
    pub funding_rate_pct: f64, // e.g., 0.01 for 1%
    pub next_funding_time_sec: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SolPriceEvent {
    pub timestamp: i64,
    // P-2: New event type for SOL/USD price
    pub price_usd: f64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct OnChainEvent {
    pub timestamp: i64,
    pub token_address: String,
    pub event_type: String, // e.g., "LiquidityAdd", "RugPull"
    pub data: Value,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DataSourceHeartbeat {
    pub source_name: String,
    pub last_processed_timestamp: i64,
    pub timestamp: i64,
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
    OnChain(OnChainEvent),
    DataSourceHeartbeat(DataSourceHeartbeat),
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
            MarketEvent::OnChain(_) => EventType::OnChain,
            MarketEvent::DataSourceHeartbeat(_) => EventType::DataSourceHeartbeat,
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
            MarketEvent::OnChain(e) => &e.token_address,
            MarketEvent::SolPrice(_) => "So11111111111111111111111111111111111111112", // SOL mint address
            MarketEvent::DataSourceHeartbeat(_) => "N/A",
        }
    }

    pub fn timestamp(&self) -> i64 {
        match self {
            MarketEvent::Price(e) => e.timestamp,
            MarketEvent::Social(e) => e.timestamp,
            MarketEvent::Depth(e) => e.timestamp,
            MarketEvent::Bridge(e) => e.timestamp,
            MarketEvent::Funding(e) => e.timestamp,
            MarketEvent::SolPrice(e) => e.timestamp,
            MarketEvent::OnChain(e) => e.timestamp,
            MarketEvent::DataSourceHeartbeat(e) => e.timestamp,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct OrderDetails {
    pub token_address: String,
    pub suggested_size_usd: f64,
    pub confidence: f64,
    pub side: Side,
    pub limit_price: Option<f64>,
    pub triggering_features: Option<Value>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(tag = "type", content = "payload")]
pub enum StrategyAction {
    /// Execute with explicit trade‑mode so the executor doesn't have to
    /// do a lookup.
    Execute(OrderDetails, TradeMode),
    Hold,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum Side {
    Long,
    Short,
}

impl std::fmt::Display for Side {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Side::Long => write!(f, "Long"),
            Side::Short => write!(f, "Short"),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SignRequest {
    pub transaction_b64: String,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SignResponse {
    pub signed_transaction_b64: String,
}

/// Alert macro that takes a Redis connection and message
#[macro_export]
macro_rules! alert {
    ($conn:expr, $($arg:tt)*) => {{
        let msg = format!($($arg)*);
        tracing::warn!("📢 ALERT: {}", msg);
        let _ = redis::cmd("PUBLISH")
            .arg("alerts")
            .arg(&msg)
            .query_async::<_, ()>(&mut $conn)
            .await;
    }};
}
