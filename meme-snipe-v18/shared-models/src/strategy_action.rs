// shared-models/src/strategy_action.rs
use crate::{OrderDetails, TradeMode};

/// Returned by a Strategy's on_event method.
/// This enum explicitly defines the possible outcomes of a strategy's
/// decision-making process for a given market event.
#[derive(Debug)]
pub enum StrategyAction {
    /// The strategy has decided to take no action and maintain its
    /// current state.
    Hold,

    /// The strategy has identified a trading opportunity and requests
    /// the execution of an order. The `TradeMode` is passed along
    /// so the executor can immediately know whether to route the trade
    /// to the live signer or to a paper-trading simulator.
    Execute(OrderDetails, TradeMode),
}
