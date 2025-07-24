import os
import json
import sqlite3
import sys
from datetime import datetime, timedelta
from flask import Flask, jsonify, render_template
import redis
import humanize
from dotenv import load_dotenv
import pandas as pd
import numpy as np

load_dotenv()

# Add shared path for API configuration
sys.path.append(os.path.join(os.path.dirname(__file__), '..', 'shared'))

try:
    from api_config import api_manager
    API_MANAGER_AVAILABLE = True
except ImportError:
    API_MANAGER_AVAILABLE = False
    print("⚠️ API Manager not available, some monitoring features disabled")

app = Flask(__name__)
redis_client = redis.Redis(host='redis', port=6379, decode_responses=True)
DB_PATH = os.getenv("DATABASE_PATH", "/app/shared/trades_v18.db")

def get_db_connection():
    try:
        conn = sqlite3.connect(f"file:{DB_PATH}?mode=ro", uri=True)
        conn.row_factory = sqlite3.Row
        return conn
    except sqlite3.OperationalError as e:
        print(f"❌ Database connection failed: {e}")
        return None

@app.template_filter('format_time')
def format_timestamp(ts):
    if not ts: return "N/A"
    try:
        return datetime.fromtimestamp(float(ts)).strftime('%Y-%m-%d %H:%M:%S')
    except (ValueError, TypeError):
        return ts

@app.template_filter('humanize_time')
def humanize_time_filter(dt):
    if not dt: return "N/A"
    if isinstance(dt, (int, float)):
        dt = datetime.fromtimestamp(dt)
    elif isinstance(dt, str):
        try:
            dt = datetime.fromisoformat(dt)
        except ValueError:
            return dt
    return humanize.naturaltime(dt)

@app.template_filter('format_pnl')
def format_pnl(pnl):
    if pnl is None: return "$0.00"
    return f"${pnl:,.2f}"

@app.template_filter('pnl_color')
def pnl_color(pnl):
    if pnl is None or pnl == 0: return "text-gray-500"
    return "text-green-500" if pnl > 0 else "text-red-500"

@app.route('/health')
def health_check():
    """Health check endpoint for Docker health checks"""
    try:
        # Check Redis
        redis_client.ping()
        # Check DB
        conn = get_db_connection()
        if conn:
            conn.close()
            return jsonify({"status": "healthy", "service": "dashboard"}), 200
        else:
            return jsonify({"status": "unhealthy", "error": "DB connection failed"}), 503
    except Exception as e:
        return jsonify({"status": "unhealthy", "error": str(e)}), 503

@app.route('/api/system/status')
def system_status():
    """Enhanced system status with API monitoring"""
    status = {
        "redis": "unknown",
        "database": "unknown",
        "api_manager": "not available",
        "timestamp": datetime.utcnow().isoformat()
    }
    
    # Check Redis
    try:
        redis_client.ping()
        status["redis"] = "healthy"
    except:
        status["redis"] = "unhealthy"
    
    # Check Database
    conn = get_db_connection()
    if conn:
        try:
            conn.execute("SELECT 1")
            status["database"] = "healthy"
        except:
            status["database"] = "unhealthy"
        finally:
            conn.close()
    else:
        status["database"] = "connection failed"
    
    # Check API Manager
    if API_MANAGER_AVAILABLE:
        try:
            status["api_manager"] = "available"
            status["api_endpoints"] = api_manager.get_all_endpoints()
        except:
            status["api_manager"] = "error"
    
    return jsonify(status)

@app.route('/api/urls/health')
def api_urls_health():
    """Check health of all external API URLs"""
    if not API_MANAGER_AVAILABLE:
        return jsonify({"error": "API Manager not available"}), 503
    
    try:
        health_results = api_manager.check_all_endpoints()
        recommendations = generate_api_recommendations(health_results)
        
        return jsonify({
            "timestamp": datetime.utcnow().isoformat(),
            "results": health_results,
            "recommendations": recommendations
        })
    except Exception as e:
        return jsonify({"error": str(e)}), 500

def generate_api_recommendations(health_results):
    """Generate recommendations based on API health"""
    recommendations = []
    
    for endpoint, result in health_results.items():
        if not result.get("healthy", False):
            if "timeout" in result.get("error", "").lower():
                recommendations.append(f"Consider increasing timeout for {endpoint}")
            elif "rate limit" in result.get("error", "").lower():
                recommendations.append(f"Implement rate limiting backoff for {endpoint}")
            else:
                recommendations.append(f"Check API credentials for {endpoint}")
    
    return recommendations

def calculate_drawdown(pnl_series):
    """Calculates the maximum drawdown from a PnL series."""
    cumulative_pnl = pnl_series.cumsum()
    peak = cumulative_pnl.expanding().max()
    drawdown = (cumulative_pnl - peak) / peak.replace(0, 1)
    max_drawdown = drawdown.min()
    return max_drawdown if np.isfinite(max_drawdown) else 0.0

@app.route('/')
def dashboard():
    conn = get_db_connection()
    if not conn:
        return "Database connection failed", 503

    try:
        # Fetch all trades and create a DataFrame
        trades_df = pd.read_sql_query("SELECT * FROM trades WHERE status LIKE 'CLOSED_%'", conn)
        if not trades_df.empty:
            trades_df['timestamp'] = pd.to_datetime(trades_df['entry_time'])
            trades_df.set_index('timestamp', inplace=True)
    except Exception as e:
        print(f"Error fetching trades: {e}")
        trades_df = pd.DataFrame()

    # --- Global KPIs ---
    global_kpis = {
        'pnl': 0, 'trades': 0, 'win_rate': 0, 'sharpe': 0, 'max_drawdown': 0
    }
    if not trades_df.empty:
        pnl_series = trades_df['pnl_usd'].dropna()
        global_kpis['pnl'] = pnl_series.sum()
        global_kpis['trades'] = len(pnl_series)
        if global_kpis['trades'] > 0:
            wins = (pnl_series > 0).sum()
            global_kpis['win_rate'] = (wins / global_kpis['trades']) * 100
            # Simplified Sharpe
            if pnl_series.std() > 0:
                global_kpis['sharpe'] = pnl_series.mean() / pnl_series.std() * np.sqrt(252) # Annualized
            global_kpis['max_drawdown'] = calculate_drawdown(pnl_series)

    # --- Per-Strategy Performance ---
    strategy_performance = {}
    if not trades_df.empty:
        for strat_id, group in trades_df.groupby('strategy_id'):
            pnl_series = group['pnl_usd'].dropna()
            trade_count = len(pnl_series)
            if trade_count > 0:
                total_pnl = pnl_series.sum()
                win_rate = (pnl_series > 0).sum() / trade_count * 100
                sharpe = 0
                if pnl_series.std() > 0:
                    sharpe = pnl_series.mean() / pnl_series.std() * np.sqrt(252) # Annualized
                max_drawdown = calculate_drawdown(pnl_series)
                
                strategy_performance[strat_id] = {
                    'total_pnl': total_pnl,
                    'trade_count': trade_count,
                    'win_rate': win_rate,
                    'sharpe_ratio': sharpe,
                    'max_drawdown': max_drawdown
                }

    # --- Time-Series Data for Charts ---
    charts_data = {}
    if not trades_df.empty:
        # Global PnL over time
        daily_pnl = trades_df['pnl_usd'].resample('D').sum().cumsum()
        charts_data['global_pnl'] = {
            'labels': daily_pnl.index.strftime('%Y-%m-%d').tolist(),
            'data': daily_pnl.values.tolist()
        }
        # Strategy PnL over time
        strategy_pnl_chart = {}
        for strat_id, group in trades_df.groupby('strategy_id'):
            daily_pnl_strat = group['pnl_usd'].resample('D').sum().cumsum()
            strategy_pnl_chart[strat_id] = {
                'labels': daily_pnl_strat.index.strftime('%Y-%m-%d').tolist(),
                'data': daily_pnl_strat.values.tolist()
            }
        charts_data['strategy_pnl'] = strategy_pnl_chart

    # --- Allocator and Strategy Info from Redis ---
    active_allocations = json.loads(redis_client.get("active_allocations") or '{}')
    strategy_specs_raw = redis_client.xrange("strategy_registry_stream", "-", "+")
    strategy_specs = [json.loads(spec_data[b'spec']) for _, spec_data in strategy_specs_raw if b'spec' in spec_data]

    # --- Recent Trades and Risk Events ---
    recent_trades = pd.read_sql_query("SELECT * FROM trades ORDER BY entry_time DESC LIMIT 10", conn).to_dict('records')
    risk_events = pd.read_sql_query("SELECT * FROM risk_events ORDER BY timestamp DESC LIMIT 10", conn).to_dict('records')

    conn.close()

    return render_template('index.html', 
                           global_kpis=global_kpis,
                           allocations=active_allocations, 
                           specs=strategy_specs,
                           num_strategies=len(strategy_specs),
                           strategy_performance=strategy_performance,
                           charts_data=json.dumps(charts_data),
                           recent_trades=recent_trades,
                           risk_events=risk_events)

@app.route('/api/trades')
def api_trades():
    """Return recent trades as JSON for API consumption."""
    try:
        conn = get_db_connection()
        if not conn:
            return jsonify({'error': 'Database connection failed'}), 503
        
        trades = pd.read_sql_query(
            "SELECT * FROM trades ORDER BY entry_time DESC LIMIT 100", 
            conn
        ).to_dict('records')
        conn.close()
        
        return jsonify([{
            'timestamp': trade.get('entry_time', ''),
            'symbol': trade.get('symbol', ''),
            'side': trade.get('side', ''),
            'amount': trade.get('amount', 0),
            'price': trade.get('entry_price', 0),
            'pnl': trade.get('pnl_usd', 0),
            'strategy': trade.get('strategy_id', '')
        } for trade in trades])
    except Exception as e:
        print(f"Error fetching trades: {e}")
        return jsonify({'error': 'Failed to fetch trades'}), 500

if __name__ == '__main__':
    app.run(host='0.0.0.0', port=5000)
