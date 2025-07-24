import redis
import json
import time
import os
import requests
import logging
from prometheus_client import start_http_server, Counter
import threading
from datetime import datetime

# Configure logging
logging.basicConfig(level=logging.INFO, format='%(asctime)s - %(levelname)s - %(message)s')
logger = logging.getLogger(__name__)

# Configuration
DRIFT_API_KEY = os.getenv("DRIFT_API_KEY", "")
DRIFT_API_URL = os.getenv("DRIFT_API_URL", "https://mainnet-beta.api.drift.trade/v2")
JUPITER_API_KEY = os.getenv("JUPITER_API_KEY", "")
BYBIT_API_ENABLED = os.getenv('BYBIT_API_ENABLED', 'false').lower() == 'true'

# Prometheus metrics
EVENTS_PUBLISHED = Counter('funding_events_published_total', 'Total number of funding events published to Redis')
API_ERRORS = Counter('funding_api_errors_total', 'Total number of API errors encountered by the funding consumer')

class FundingRateAggregator:
    def __init__(self, redis_client):
        self.redis_client = redis_client
        self.last_heartbeat = time.time()
        self.sources_active = set()

    def publish_heartbeat(self):
        """Publish heartbeat for service monitoring"""
        now = time.time()
        if now - self.last_heartbeat > 60:  # Every 60 seconds
            heartbeat = {
                'service': 'funding_consumer',
                'timestamp': datetime.utcnow().isoformat(),
                'status': 'active' if self.sources_active else 'degraded',
                'sources_count': len(self.sources_active),
                'sources_active': json.dumps(list(self.sources_active))
            }
            
            try:
                self.redis_client.xadd('events:data_source_heartbeat', heartbeat)
                logger.info(f"Published heartbeat: {len(self.sources_active)} sources active")
                self.last_heartbeat = now
            except Exception as e:
                logger.error(f"Failed to publish heartbeat: {e}")

def start_metrics_server():
    """Starts a Prometheus metrics server in a background thread."""
    start_http_server(8002)
    logging.info("Prometheus metrics server started on port 8002.")

def get_drift_funding_rates():
    """Fetches real funding rates from Drift Protocol API."""
    if not DRIFT_API_KEY:
        logger.warning("Drift API key not configured - using simulated funding data")
        # Provide simulated data for paper trading
        return [{
            "marketSymbol": "SOL-PERP",
            "fundingRate": 0.0001,
            "nextFundingTime": time.time() + 28800,
            "marketIndex": 0
        }, {
            "marketSymbol": "BTC-PERP", 
            "fundingRate": -0.0005,
            "nextFundingTime": time.time() + 28800,
            "marketIndex": 1
        }]
        
    try:
        headers = {"X-API-KEY": DRIFT_API_KEY}
        response = requests.get(f"{DRIFT_API_URL}/funding-rates", headers=headers, timeout=10)
        response.raise_for_status()
        return response.json().get('data', [])
        
    except Exception as e:
        logger.error(f"Error fetching funding rates: {e}")
        API_ERRORS.inc()
        return None

def get_drift_funding_rates_legacy():
    """Legacy function - TODO: Remove when new implementation is ready"""
    drift_api_url = os.getenv("DRIFT_API_URL", "https://api.drift.trade")
    
    try:
        # Drift API endpoint for funding rates
        response = requests.get(f"{drift_api_url}/v2/funding/rates", timeout=10)
        response.raise_for_status()
        
        funding_data = response.json()
        return funding_data.get('data', [])
        
    except requests.exceptions.RequestException as e:
        logging.error(f"Error fetching funding rates from Drift API: {e}")
        API_ERRORS.inc()
        return []
    except Exception as e:
        logging.error(f"Unexpected error fetching funding rates: {e}")
        API_ERRORS.inc()
        return []

def fetch_bybit_funding():
    """Fetch Bybit funding rates for comparison"""
    if not BYBIT_API_ENABLED:
        return []
        
    try:
        url = "https://api.bybit.com/v5/market/funding/history"
        params = {
            'category': 'linear',
            'symbol': 'SOLUSDT',
            'limit': 10
        }
        
        response = requests.get(url, params=params, timeout=10)
        if response.status_code == 200:
            data = response.json()
            return data.get('result', {}).get('list', [])
    except Exception as e:
        logger.error(f"Bybit funding fetch error: {e}")
    
    return []

def main():
    logging.info("🚀 Starting Enhanced Funding Event Consumer...")
    logging.info(f"Drift API configured: {bool(DRIFT_API_KEY)}")
    logging.info(f"Jupiter API configured: {bool(JUPITER_API_KEY)}")
    logging.info(f"Bybit monitoring enabled: {BYBIT_API_ENABLED}")
    
    # Start Prometheus metrics server in a background thread
    metrics_thread = threading.Thread(target=start_metrics_server, daemon=True)
    metrics_thread.start()
    
    r = redis.Redis.from_url(os.getenv("REDIS_URL", "redis://redis:6379"), decode_responses=True)
    aggregator = FundingRateAggregator(r)

    while True:
        try:
            # Fetch from Drift (primary source)
            funding_rates = get_drift_funding_rates()
            
            if funding_rates:
                aggregator.sources_active.add('drift')
                for rate_info in funding_rates:
                    try:
                        event = {
                            "type": "Funding",
                            "token_address": rate_info.get("marketSymbol", "UNKNOWN"),
                            "funding_rate_pct": float(rate_info.get("fundingRate", 0)),
                            "next_funding_time_sec": int(rate_info.get("nextFundingTime", time.time() + 3600)),
                            "market_index": rate_info.get("marketIndex"),
                            "source": "drift",
                            "timestamp": datetime.utcnow().isoformat()
                        }
                        
                        r.xadd("events:funding", {"event": json.dumps(event)})
                        EVENTS_PUBLISHED.inc()
                        logging.info(f"Published funding rate for {event['token_address']}: {event['funding_rate_pct']:.4f}%")
                        
                    except (ValueError, TypeError) as e:
                        logging.warning(f"Error processing funding rate data: {e}")
                        continue
            else:
                aggregator.sources_active.discard('drift')
                logging.warning("No funding rate data received from Drift API")
            
            # Fetch additional comparison data from Bybit
            if BYBIT_API_ENABLED:
                bybit_rates = fetch_bybit_funding()
                if bybit_rates:
                    aggregator.sources_active.add('bybit')
                    # Publish comparative funding event
                    comparison_event = {
                        "type": "FundingComparison",
                        "source": "bybit",
                        "data": bybit_rates[:3],  # Latest 3 rates
                        "timestamp": datetime.utcnow().isoformat()
                    }
                    r.xadd("events:funding", {"event": json.dumps(comparison_event)})
                else:
                    aggregator.sources_active.discard('bybit')
            
            # Publish heartbeat
            aggregator.publish_heartbeat()
            
            # Sleep for 60 seconds before next poll (funding rates don't change frequently)
            time.sleep(60)
            
        except Exception as e:
            logger.error(f"Error in funding consumer main loop: {e}")
            time.sleep(120)  # Wait longer on error

if __name__ == "__main__":
    main()

def main():
    logging.info("🚀 Starting Funding Event Consumer (Drift API)...")
    
    # Start Prometheus metrics server in a background thread
    metrics_thread = threading.Thread(target=start_metrics_server, daemon=True)
    metrics_thread.start()
    
    r = redis.Redis.from_url(os.getenv("REDIS_URL", "redis://redis:6379"), decode_responses=True)

    while True:
        funding_rates = get_drift_funding_rates()
        
        if funding_rates:
            for rate_info in funding_rates:
                try:
                    event = {
                        "type": "Funding",
                        "token_address": rate_info.get("marketSymbol", "UNKNOWN"),
                        "funding_rate_pct": float(rate_info.get("fundingRate", 0)),
                        "next_funding_time_sec": int(rate_info.get("nextFundingTime", time.time() + 3600)),
                        "market_index": rate_info.get("marketIndex"),
                    }
                    
                    r.xadd("events:funding", {"event": json.dumps(event)})
                    EVENTS_PUBLISHED.inc()
                    logging.info(f"Published funding rate for {event['token_address']}: {event['funding_rate_pct']:.4f}%")
                    
                except (ValueError, TypeError) as e:
                    logging.warning(f"Error processing funding rate data: {e}")
                    continue
        else:
            logging.warning("No funding rate data received from Drift API")
        
        # Sleep for 60 seconds before next poll (funding rates don't change frequently)
        time.sleep(60)

if __name__ == "__main__":
    main()
