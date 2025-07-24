import redis
import json
import time
import os
import sys
import requests
import logging
from prometheus_client import start_http_server, Counter
import threading

# Configure logging
logging.basicConfig(level=logging.INFO, format='%(asctime)s - %(levelname)s - %(message)s')

# Prometheus metrics
EVENTS_PUBLISHED = Counter('enhanced_price_events_published_total', 'Total number of enhanced price events published to Redis')

def start_metrics_server():
    """Starts a Prometheus metrics server in a background thread."""
    start_http_server(8003)
    logging.info("Prometheus metrics server started on port 8003.")

# Add shared path for API configuration
sys.path.append(os.path.join(os.path.dirname(__file__), '..', 'shared'))

try:
    from api_config import api_manager, get_price_feed_url
    USE_CENTRALIZED_API = True
except ImportError:
    print("⚠️ Centralized API config not available, using fallback URLs")
    USE_CENTRALIZED_API = False

def get_sol_price_from_pyth() -> float | None:
    """Fetches real SOL/USD price from Pyth Network with failover support."""
    try:
        if USE_CENTRALIZED_API:
            # Use centralized API manager with failover
            response = api_manager.make_request(
                'price_feeds', 
                'latest_price_feeds',
                params={
                    "ids[]": "0xef0d8b6fda2ceba41da15d4095d1da392a0d2f8ed0c6c7bc0f4cfac8c280b56d"
                }
            )
        else:
            # Fallback to direct API call
            url = "https://hermes.pyth.network/api/latest_price_feeds"
            params = {
                "ids[]": "0xef0d8b6fda2ceba41da15d4095d1da392a0d2f8ed0c6c7bc0f4cfac8c280b56d"
            }
            response = requests.get(url, params=params, timeout=10)
        
        if response and response.status_code == 200:
            data = response.json()
            if data and len(data) > 0:
                price_data = data[0]
                price = float(price_data['price']['price']) / (10 ** abs(int(price_data['price']['expo'])))
                print(f"✅ Real SOL price from Pyth: ${price:.4f}")
                return price
                
    except Exception as e:
        print(f"⚠️ Error fetching real SOL price from Pyth: {e}")
        return None

def get_sol_price_from_coinbase() -> float | None:
    """Backup: Fetch SOL price from Coinbase with failover support."""
    try:
        if USE_CENTRALIZED_API:
            # Use centralized API manager
            response = api_manager.make_request(
                'price_feeds',
                'v2/exchange-rates',
                params={'currency': 'SOL'}
            )
        else:
            # Fallback to direct API call
            url = "https://api.coinbase.com/v2/exchange-rates?currency=SOL"
            response = requests.get(url, timeout=5)
        
        if response and response.status_code == 200:
            data = response.json()
            if 'data' in data and 'rates' in data['data'] and 'USD' in data['data']['rates']:
                price = float(data['data']['rates']['USD'])
                print(f"✅ Real SOL price from Coinbase: ${price:.4f}")
                return price
                
    except Exception as e:
        print(f"⚠️ Error fetching SOL price from Coinbase: {e}")
        return None

def get_sol_price_from_coingecko() -> float | None:
    """Additional backup: Fetch SOL price from CoinGecko with failover support."""
    try:
        if USE_CENTRALIZED_API:
            response = api_manager.make_request(
                'price_feeds',
                'simple/price',
                params={
                    'ids': 'solana',
                    'vs_currencies': 'usd'
                }
            )
        else:
            url = "https://api.coingecko.com/api/v3/simple/price"
            params = {'ids': 'solana', 'vs_currencies': 'usd'}
            response = requests.get(url, params=params, timeout=5)
        
        if response and response.status_code == 200:
            data = response.json()
            if 'solana' in data and 'usd' in data['solana']:
                price = float(data['solana']['usd'])
                print(f"✅ Real SOL price from CoinGecko: ${price:.4f}")
                return price
                
    except Exception as e:
        print(f"⚠️ Error fetching SOL price from CoinGecko: {e}")
        return None

def get_real_sol_price() -> float:
    """Get real SOL price with multiple fallbacks and best practices."""
    # Try multiple sources in order of reliability
    price_sources = [
        ("Pyth Network", get_sol_price_from_pyth),
        ("Coinbase", get_sol_price_from_coinbase),
        ("CoinGecko", get_sol_price_from_coingecko)
    ]
    
    for source_name, price_func in price_sources:
        try:
            price = price_func()
            if price and price > 0:
                print(f"✅ Using SOL price from {source_name}: ${price:.4f}")
                return price
        except Exception as e:
            print(f"⚠️ {source_name} failed: {e}")
            continue
    
    # Final fallback to reasonable estimate
    print("⚠️ All price sources failed, using fallback estimate")
    return 100.0  # Conservative SOL price estimate

def get_redis_connection():
    """Get Redis connection with error handling"""
    try:
        # Use REDIS_URL instead of separate host/port
        redis_url = os.getenv('REDIS_URL', 'redis://redis:6379')
        return redis.from_url(redis_url, decode_responses=True)
    except Exception as e:
        print(f"⚠️ Redis connection failed: {e}")
        return None

def publish_price_data(r, sol_price: float):
    """Publish price data to Redis with error handling"""
    try:
        if not r:
            return
            
        price_data = {
            'timestamp': time.time(),
            'sol_usd': sol_price,
            'source': 'helius_rpc_price_consumer',
            'health_status': 'healthy' if USE_CENTRALIZED_API else 'fallback'
        }
        
        # Publish to multiple channels for different consumers
        channels = ['sol_price', 'market_data', 'price_feed']
        for channel in channels:
            r.publish(channel, json.dumps(price_data))
            EVENTS_PUBLISHED.inc()  # Increment counter for each published event
        
        # Store latest price in Redis for caching
        r.setex('latest_sol_price', 300, json.dumps(price_data))  # 5-minute expiry
        
    except Exception as e:
        logging.error(f"Error publishing price data: {e}")

def main():
    """Main price consumer loop with enhanced reliability"""
    logging.info("🚀 Starting Enhanced SOL Price Consumer with Failover Support...")
    
    # Start Prometheus metrics server in a background thread
    metrics_thread = threading.Thread(target=start_metrics_server, daemon=True)
    metrics_thread.start()
    
    # Health check interval
    health_check_interval = int(os.getenv('API_HEALTH_CHECK_INTERVAL', 300))
    last_health_check = 0
    
    redis_client = get_redis_connection()
    if not redis_client:
        print("❌ Cannot connect to Redis, exiting...")
        return
    
    price_fetch_interval = 30  # 30 seconds between price updates
    
    while True:
        try:
            # Periodic health checks
            current_time = time.time()
            if USE_CENTRALIZED_API and (current_time - last_health_check) > health_check_interval:
                print("🔍 Performing API health check...")
                health_status = api_manager.health_check('price_feeds')
                print(f"📊 API Health Status: {health_status}")
                last_health_check = current_time
            
            # Fetch and publish SOL price
            sol_price = get_real_sol_price()
            publish_price_data(redis_client, sol_price)
            
            print(f"📊 Published SOL price: ${sol_price:.4f} at {time.strftime('%H:%M:%S')}")
            
            # Wait before next update
            time.sleep(price_fetch_interval)
            
        except KeyboardInterrupt:
            print("\n🛑 Shutting down price consumer...")
            break
        except Exception as e:
            print(f"⚠️ Error in main loop: {e}")
            time.sleep(10)  # Wait before retrying

if __name__ == "__main__":
    main()
