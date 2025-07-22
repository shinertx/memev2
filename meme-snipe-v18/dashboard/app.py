import os
import json
import sqlite3
from datetime import datetime
from flask import Flask, jsonify, render_template
import redis
import humanize
from dotenv import load_dotenv

load_dotenv()

app = Flask(__name__)
redis_client = redis.Redis(host='redis', port=6379, decode_responses=True)
DB_PATH = os.getenv("DATABASE_PATH", "/app/shared/trades_v18.db") # Updated DB path

def get_db_connection():
    try:
        conn = sqlite3.connect(f"file:{DB_PATH}?mode=ro", uri=True)
        conn.row_factory = sqlite3.Row
        return conn
    except sqlite3.OperationalError:
        return None

@app.template_filter('format_time')
def format_timestamp(ts):
    if not ts: return "N/A"
    return datetime.fromtimestamp(ts).strftime('%Y-%m-%d %H:%M:%S')

@app.template_filter('humanize_time')
def humanize_time_filter(dt):
    if not dt: return "N/A"
    if isinstance(dt, (int, float)):
        dt = datetime.fromtimestamp(dt)
    return humanize.naturaltime(dt)

@app.route('/health')
def health_check():
    """Health check endpoint for Docker health checks"""
    try:
        # Check Redis connection
        redis_client.ping()
        redis_healthy = True
    except redis.RedisError:
        redis_healthy = False
    
    # Check database connection
    conn = get_db_connection()
    db_healthy = conn is not None
    if conn:
        conn.close()
    
    status = "healthy" if redis_healthy and db_healthy else "unhealthy"
    return jsonify({
        "status": status,
        "timestamp": datetime.now().isoformat(),
        "services": {
            "redis": "healthy" if redis_healthy else "unhealthy",
            "database": "healthy" if db_healthy else "unhealthy"
        }
    }), 200 if status == "healthy" else 503

@app.route('/')
def dashboard():
    # Get active allocations from Redis
    active_allocations = []
    try:
        allocations_raw = redis_client.get("active_allocations")
        if allocations_raw:
            active_allocations = json.loads(allocations_raw)
    except (json.JSONDecodeError, redis.RedisError):
        pass

    # Get strategy specs from Redis
    strategy_specs = []
    try:
        # P-7: Read from strategy_registry_stream for dashboard display
        # This is a simplified read for dashboard, not a full stream consumer
        specs_raw_list = redis_client.xrange("strategy_registry_stream", "-", "+")
        for item_id, item_data in specs_raw_list:
            if b'spec' in item_data:
                try:
                    spec = json.loads(item_data[b'spec'].decode('utf-8'))
                    strategy_specs.append(spec)
                except json.JSONDecodeError:
                    pass
    except redis.RedisError:
        pass

    # Get per-strategy performance metrics from DB
    strategy_performance = {}
    conn = get_db_connection()
    if conn:
        try:
            # Fetch all closed trades to calculate PnL and Sharpe per strategy
            closed_trades = conn.execute("SELECT strategy_id, pnl_usd FROM trades WHERE status LIKE 'CLOSED_%'").fetchall()
            
            # Organize PnL per strategy
            pnl_by_strategy = {}
            for trade in closed_trades:
                pnl_by_strategy.setdefault(trade['strategy_id'], []).append(trade['pnl_usd'] if trade['pnl_usd'] is not None else 0.0)

            for strat_id, pnl_values in pnl_by_strategy.items():
                total_pnl = sum(pnl_values)
                trade_count = len(pnl_values)
                wins = sum(1 for p in pnl_values if p > 0)
                win_rate = (wins / trade_count) * 100 if trade_count > 0 else 0
                
                # Simple Sharpe (for display, not rigorous)
                if len(pnl_values) > 1:
                    mean_pnl_per_trade = total_pnl / trade_count
                    variance = sum([(x - mean_pnl_per_trade)**2 for x in pnl_values]) / (trade_count - 1) if trade_count > 1 else 0
                    std_dev = variance**0.5
                    sharpe_ratio = mean_pnl_per_trade / std_dev if std_dev > 0 else 0.0
                else:
                    sharpe_ratio = 0.0 # Not enough data for Sharpe

                strategy_performance[strat_id] = {
                    'total_pnl': total_pnl,
                    'trade_count': trade_count,
                    'win_rate': win_rate,
                    'sharpe_ratio': sharpe_ratio,
                }
        except sqlite3.Error as e:
            print(f"Error fetching strategy performance: {e}")
        finally:
            conn.close()

    # Calculate global KPIs
    global_total_pnl = sum(metrics['total_pnl'] for metrics in strategy_performance.values())
    global_total_trades = sum(metrics['trade_count'] for metrics in strategy_performance.values())
    global_total_wins = sum(metrics['win_rate'] * metrics['trade_count'] / 100 for metrics in strategy_performance.values()) # Approx
    global_win_rate = (global_total_wins / global_total_trades) * 100 if global_total_trades > 0 else 0

    return render_template('index.html', 
                           allocations=active_allocations, 
                           specs=strategy_specs,
                           num_strategies=len(strategy_specs),
                           strategy_performance=strategy_performance,
                           global_total_pnl=global_total_pnl,
                           global_total_trades=global_total_trades,
                           global_win_rate=global_win_rate)

@app.route('/api/trades')
def api_trades():
    conn = get_db_connection()
    if not conn:
        return jsonify({"error": "Database not available."}), 503
    
    try:
        trades = conn.execute('SELECT * FROM trades ORDER BY entry_time DESC LIMIT 100').fetchall()
        return jsonify([dict(row) for row in trades])
    except sqlite3.Error as e:
        return jsonify({"error": f"Database query failed: {e}"}), 500
    finally:
        conn.close()

if __name__ == '__main__':
    app.run(host='0.0.0.0', port=5000)
