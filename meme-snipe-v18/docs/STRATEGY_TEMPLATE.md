# Strategy: [Your Strategy Name]
- **Family ID:** `[e.g., social_buzz_v1]` (Must be unique and static)
- **Author:** `[Your Name/Handle]`
- **Version:** `1.0.0`

## 1. Core Thesis
*A one-sentence description of the alpha signal this strategy aims to capture.*
> **Example:** "This strategy hypothesizes that a rapid increase in Twitter mentions for a low-cap token precedes a short-term price increase."

## 2. Data Requirements
*List the specific `EventType`s this strategy subscribes to.*
- `EventType::Price`
- `EventType::Social`
- `EventType::Depth`
- `EventType::Bridge`
- `EventType::Funding`
- `EventType::OnChain`
- `EventType::SolPrice`

## 3. Configurable Parameters (`spec.params`)
*Define the JSON schema for this strategy's parameters. These will be passed to `init()`.*
```json
{
  "type": "object",
  "properties": {
    "lookback_period": { "type": "integer", "default": 15, "description": "Lookback period in minutes for mention velocity." },
    "std_dev_threshold": { "type": "number", "default": 3.0, "description": "Number of standard deviations to trigger a buy signal." }
  },
  "required": ["lookback_period", "std_dev_threshold"]
}
```

## 4. Risk Assessment
*Describe the potential risks and failure modes of this strategy.*
- **Market Risk:** What market conditions could cause this strategy to fail?
- **Data Risk:** What if the data sources become unreliable?
- **Execution Risk:** What could go wrong during trade execution?
- **Parameter Risk:** How sensitive is the strategy to parameter changes?

## 5. Performance Expectations
*Define what success looks like for this strategy.*
- **Expected Win Rate:** What percentage of trades should be profitable?
- **Expected Sharpe Ratio:** What risk-adjusted return should this strategy achieve?
- **Expected Drawdown:** What maximum drawdown is acceptable?
- **Expected Holding Period:** How long should positions typically be held?

## 6. Implementation Notes
*Any technical considerations for implementing this strategy.*
- **Computational Complexity:** How resource-intensive is this strategy?
- **Data Storage:** What historical data needs to be maintained?
- **Edge Cases:** What edge cases need to be handled?
- **Testing:** How should this strategy be tested?

## 7. Backtesting Results (Optional)
*If available, include backtesting results.*
- **Period:** What time period was tested?
- **Performance:** Key metrics (PnL, Sharpe, Max Drawdown)
- **Sample Size:** How many trades were generated?
- **Market Conditions:** What market conditions were tested?

## 8. Live Trading Considerations
*What needs to be monitored when this strategy goes live.*
- **Key Metrics:** What metrics should be tracked in real-time?
- **Alerts:** What conditions should trigger alerts?
- **Adjustments:** What parameters might need adjustment based on live performance?
- **Shutdown Conditions:** When should this strategy be disabled?

---

## Implementation Checklist
- [ ] Strategy struct defined with `Default` and `Deserialize` derives
- [ ] `Strategy` trait implemented with all required methods
- [ ] `register_strategy!` macro called with correct family ID
- [ ] Strategy added to `mod.rs` declarations
- [ ] Default parameters added to `strategy_factory/factory.py`
- [ ] Unit tests written and passing
- [ ] Documentation completed using this template
- [ ] Code reviewed for safety and efficiency
