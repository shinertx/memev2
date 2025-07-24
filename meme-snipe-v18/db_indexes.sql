-- Performance indexes for trades database
CREATE INDEX IF NOT EXISTS idx_trades_status ON trades(status);
CREATE INDEX IF NOT EXISTS idx_trades_timestamp ON trades(timestamp);
CREATE INDEX IF NOT EXISTS idx_trades_strategy ON trades(strategy_id);
CREATE INDEX IF NOT EXISTS idx_trades_token ON trades(token_address);
CREATE INDEX IF NOT EXISTS idx_perf_strategy_time ON performance_metrics(strategy_id, timestamp);

-- Optimize database
VACUUM;
ANALYZE;
