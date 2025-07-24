import redis
import json
import sqlite3
import pandas as pd
import numpy as np
import time
import os
from statsmodels.formula.api import ols

# --- Configuration ---
REDIS_URL = os.getenv("REDIS_URL", "redis://localhost:6379")
POSITION_UPDATES_STREAM = "position_updates_channel"
DB_PATH = os.getenv("DB_PATH", "/app/shared/trades_v18.db")
ALPHA_DECAY_LOG = "/app/logs/alpha_decay.log"
ANALYTICS_REDIS_KEY_PREFIX = "analytics:strategy:"

# --- Main Service Logic ---

def get_redis_connection():
    """Establishes a connection to Redis."""
    return redis.from_url(REDIS_URL, decode_responses=True)

def setup_database():
    """Ensure the necessary analytics table exists."""
    with sqlite3.connect(DB_PATH) as conn:
        cursor = conn.cursor()
        cursor.execute("""
        CREATE TABLE IF NOT EXISTS trade_features (
            trade_id TEXT PRIMARY KEY,
            strategy_id TEXT,
            token_address TEXT,
            entry_timestamp REAL,
            pnl_usd REAL,
            feature_json TEXT
        )
        """)
        conn.commit()

def log_trade_features(data):
    """Logs the features that triggered a trade to the database."""
    trade_id = data.get('position_id')
    if not trade_id or data.get('status') != 'CLOSED':
        return

    with sqlite3.connect(DB_PATH) as conn:
        cursor = conn.cursor()
        cursor.execute("""
        INSERT INTO trade_features (trade_id, strategy_id, token_address, entry_timestamp, pnl_usd, feature_json)
        VALUES (?, ?, ?, ?, ?, ?)
        ON CONFLICT(trade_id) DO UPDATE SET
            pnl_usd=excluded.pnl_usd
        """, (
            trade_id,
            data.get('strategy_id'),
            data.get('token_address'),
            data.get('entry_timestamp'),
            data.get('pnl'),
            json.dumps(data.get('triggering_features', {}))
        ))
        conn.commit()
    print(f"Logged features for closed trade: {trade_id}")

def calculate_tail_risk_metrics(pnl_series: pd.Series, risk_free_rate: float = 0.0):
    """Calculates Sortino Ratio and CVaR (Expected Shortfall)."""
    if pnl_series.empty:
        return {"sortino_ratio": 0, "cvar_95": 0}

    # Sortino Ratio
    daily_returns = pnl_series.pct_change().dropna()
    target_return = risk_free_rate / 252 # Daily risk-free rate
    downside_returns = daily_returns[daily_returns < target_return]
    expected_return = daily_returns.mean()
    downside_std = downside_returns.std()
    sortino_ratio = (expected_return - target_return) / downside_std if downside_std != 0 else 0
    sortino_ratio *= np.sqrt(252) # Annualize

    # CVaR (95%)
    pnl_values = pnl_series.values
    var_95 = np.percentile(pnl_values, 5)
    cvar_95 = pnl_values[pnl_values <= var_95].mean()

    return {"sortino_ratio": sortino_ratio, "cvar_95": cvar_95}

def run_periodic_analysis(r: redis.Redis):
    """Runs alpha decay and tail risk analysis."""
    print("Running periodic analysis...")
    with sqlite3.connect(DB_PATH) as conn:
        df = pd.read_sql_query("SELECT * FROM trade_features WHERE pnl_usd IS NOT NULL", conn)

    if len(df) < 20:
        print(f"Not enough data for analysis (found {len(df)} trades).")
        return

    strategies = df['strategy_id'].unique()
    for strategy_id in strategies:
        strategy_df = df[df['strategy_id'] == strategy_id].copy()
        if strategy_df.empty:
            continue

        # --- Tail Risk Analysis ---
        pnl_series = strategy_df.sort_values('entry_timestamp')['pnl_usd'].cumsum()
        tail_metrics = calculate_tail_risk_metrics(pnl_series)
        
        # Store metrics in Redis
        redis_key = f"{ANALYTICS_REDIS_KEY_PREFIX}{strategy_id}"
        r.hset(redis_key, mapping={
            "sortino_ratio_annualized": tail_metrics["sortino_ratio"],
            "cvar_95_usd": tail_metrics["cvar_95"],
            "last_updated_ts": int(time.time())
        })
        print(f"Updated tail risk metrics for {strategy_id}: Sortino={tail_metrics['sortino_ratio']:.2f}, CVaR={tail_metrics['cvar_95']:.2f}")

        # --- Alpha Decay Analysis ---
        # (This part remains the same as before)
        try:
            features_df = pd.json_normalize(strategy_df['feature_json'].apply(json.loads))
            numeric_features = features_df.select_dtypes(include=np.number).columns
            strategy_df = strategy_df.join(features_df)

            for feature in numeric_features:
                if strategy_df[feature].nunique() > 1:
                    # Simplified correlation trend analysis
                    correlation = strategy_df['pnl_usd'].corr(strategy_df[feature])
                    # A more robust implementation would track this correlation over time.
                    # For now, we log the current correlation.
                    r.hset(redis_key, f"feature_corr_{feature}", f"{correlation:.4f}")
        except Exception as e:
            print(f"Could not run feature analysis for {strategy_id}: {e}")


def main():
    """Main loop to listen for position updates and run analysis."""
    print("Starting Analytics & Alpha Decay Monitoring Service...")
    setup_database()
    r = get_redis_connection()
    
    stream_name = POSITION_UPDATES_STREAM
    group_name = "analytics_monitor_group"
    consumer_name = f"consumer-{os.getpid()}"

    try:
        r.xgroup_create(stream_name, group_name, id='0', mkstream=True)
    except redis.exceptions.ResponseError as e:
        if "BUSYGROUP" not in str(e):
            print(f"Consumer group '{group_name}' already exists.")
        else:
            raise

    last_analysis_time = time.time()

    while True:
        try:
            messages = r.xreadgroup(group_name, consumer_name, {stream_name: '>'}, count=100, block=5000)
            
            if messages:
                for _, msg_list in messages:
                    for msg_id, msg_data in msg_list:
                        try:
                            data = json.loads(msg_data['data'])
                            log_trade_features(data)
                            r.xack(stream_name, group_name, msg_id)
                        except Exception as e:
                            print(f"Error processing message {msg_id}: {e}")

            # Run analysis periodically
            if time.time() - last_analysis_time > 3600: # Every hour
                run_periodic_analysis(r)
                last_analysis_time = time.time()

        except redis.exceptions.ConnectionError as e:
            print(f"Redis connection error: {e}. Retrying in 10s...")
            time.sleep(10)
        except Exception as e:
            print(f"An error occurred in the main loop: {e}")
            time.sleep(10)

if __name__ == "__main__":
    main()
