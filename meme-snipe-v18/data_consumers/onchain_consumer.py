import redis
import json
import time
import os
import requests
import logging
import asyncio
from datetime import datetime
from prometheus_client import start_http_server, Counter
import threading

# Configure logging
logging.basicConfig(level=logging.INFO, format='%(asctime)s - %(levelname)s - %(message)s')
logger = logging.getLogger(__name__)

# Configuration
REDIS_URL = os.getenv("REDIS_URL", "redis://redis:6379")
HELIUS_RPC_URL = os.getenv("HELIUS_RPC_URL", "https://api.mainnet-beta.solana.com")
HELIUS_API_KEY = os.getenv("HELIUS_API_KEY", "")
RAYDIUM_AMM_PROGRAM = "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8"

# Prometheus metrics
EVENTS_PUBLISHED = Counter('onchain_events_published_total', 'Total number of onchain events published to Redis')
API_ERRORS = Counter('onchain_api_errors_total', 'Total number of API errors encountered by the onchain consumer')

class OnChainConsumer:
    def __init__(self):
        self.redis_client = redis.from_url(REDIS_URL)
        self.session = requests.Session()
        self.last_heartbeat = time.time()
        
    async def monitor_new_pools(self):
        """Monitor Raydium for new liquidity pools"""
        try:
            # TODO: Implement websocket subscription when Helius premium available
            # For now, poll for new pools periodically
            
            headers = {}
            if HELIUS_API_KEY:
                headers["Authorization"] = f"Bearer {HELIUS_API_KEY}"
                
            response = self.session.post(
                HELIUS_RPC_URL,
                headers=headers,
                json={
                    "jsonrpc": "2.0",
                    "id": 1,
                    "method": "getProgramAccounts",
                    "params": [
                        RAYDIUM_AMM_PROGRAM,
                        {
                            "encoding": "base64",
                            "commitment": "confirmed",
                            "filters": [
                                {"dataSize": 752},
                                {"memcmp": {"offset": 0, "bytes": "1"}}
                            ]
                        }
                    ]
                }
            )
            
            if response.status_code == 200:
                data = response.json()
                if "result" in data:
                    event = {
                        "event_type": "on_chain",
                        "timestamp": datetime.now().timestamp(),
                        "data": {
                            "type": "pool_update",
                            "program": "raydium",
                            "pool_count": len(data["result"])
                        }
                    }
                    self.redis_client.xadd("events:onchain", {"data": json.dumps(event)})
                    logger.info(f"Published pool update: {len(data['result'])} pools")
                    EVENTS_PUBLISHED.inc()
                    
        except Exception as e:
            logger.error(f"Error monitoring pools: {e}")
            API_ERRORS.inc()
            
    async def publish_heartbeat(self):
        """Publish service heartbeat"""
        heartbeat_data = {
            "source": "onchain_consumer",
            "timestamp": datetime.now().timestamp(),
            "status": "healthy"
        }
        self.redis_client.xadd(
            "events:data_source_heartbeat",
            {"data": json.dumps(heartbeat_data)}
        )
        
    async def run(self):
        """Main event loop"""
        while True:
            try:
                await self.monitor_new_pools()
                await self.publish_heartbeat()
                
                # TODO: Add more on-chain monitoring:
                # - Large wallet movements
                # - Program upgrades
                # - Liquidity migrations
                
            except Exception as e:
                logger.error(f"OnChain consumer error: {e}")
                
            # Poll every 30 seconds (increase frequency with premium API)
            await asyncio.sleep(30)

def start_metrics_server():
    """Starts a Prometheus metrics server in a background thread."""
    start_http_server(8001)
    logging.info("Prometheus metrics server started on port 8001.")

def get_helius_api_key():
    api_key = os.getenv("HELIUS_API_KEY")
    if not api_key:
        logging.error("HELIUS_API_KEY environment variable not set.")
        raise ValueError("HELIUS_API_KEY not set")
    return api_key

def get_signatures_for_address(api_key, address, limit=10):
    """
    Gets the most recent transaction signatures for a given address.
    """
    url = f"https://api.helius.xyz/v0/addresses/{address}/transactions?api-key={api_key}&limit={limit}"
    try:
        response = requests.get(url)
        response.raise_for_status()
        return [tx['signature'] for tx in response.json()]
    except requests.exceptions.RequestException as e:
        logging.error(f"Error fetching transaction signatures for {address}: {e}")
        API_ERRORS.inc()
        return []

def parse_transaction(api_key, signature):
    """
    Parses a single transaction to extract relevant events.
    """
    url = f"https://api.helius.xyz/v0/transactions/?api-key={api_key}"
    headers = {'Content-Type': 'application/json'}
    payload = {"transactions": [signature]}
    
    try:
        response = requests.post(url, headers=headers, json=payload)
        response.raise_for_status()
        tx_details = response.json()
        
        if not tx_details:
            return None

        tx_detail = tx_details[0] # We are only sending one signature
        description = tx_detail.get("description", "")
        
        # Example of parsing a specific event type: Large transfer
        # This logic can be greatly expanded to detect many other event types.
        if "transfer" in description.lower():
            for instruction in tx_detail.get("instructions", []):
                if instruction.get("programId") == "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA":
                    # This is a SPL token instruction, could be a transfer
                    # A real implementation would decode the instruction data.
                    # For now, we'll focus on native SOL transfers from the description.
                    pass

            # Look for large native SOL transfers
            for change in tx_detail.get("nativeTransfers", []):
                if abs(change['amount']) > 100 * 10**9: # More than 100 SOL
                    from_wallet = change['fromUserAccount']
                    to_wallet = change['toUserAccount']
                    amount_sol = change['amount'] / 10**9
                    
                    return {
                        "type": "OnChain",
                        "token_address": "So11111111111111111111111111111111111111112", # Native SOL
                        "event_type": "LARGE_TRANSFER",
                        "details": {
                            "from": from_wallet,
                            "to": to_wallet,
                            "amount_sol": amount_sol
                        }
                    }
        return None

    except requests.exceptions.RequestException as e:
        logging.error(f"Error parsing transaction {signature}: {e}")
        API_ERRORS.inc()
        return None
    except Exception as e:
        logging.error(f"An unexpected error occurred while parsing transaction {signature}: {e}")
        API_ERRORS.inc()
        return None


def main():
    logging.info("🚀 Starting OnChain Event Consumer (Helius RPC)...")
    
    # Start Prometheus metrics server in a background thread
    metrics_thread = threading.Thread(target=start_metrics_server, daemon=True)
    metrics_thread.start()
    
    consumer = OnChainConsumer()
    asyncio.run(consumer.run())

if __name__ == "__main__":
    logger.info("Starting OnChain Event Consumer")
    main()
    metrics_thread = threading.Thread(target=start_metrics_server, daemon=True)
    metrics_thread.start()
    
    r = redis.Redis.from_url(os.getenv("REDIS_URL", "redis://redis:6379"), decode_responses=True)
    api_key = get_helius_api_key()

    # Wallets to monitor. In a real system, this would be dynamic.
    # Using some known, active wallets for demonstration.
    # (e.g., a known large trader, a protocol treasury, etc.)
    wallets_to_monitor = [
        "4pUQS4sjw1b9D1fG5B1Lz2iLhGk1G5A1d2a3b4c5d6e7", # Placeholder, replace with real addresses
        "7iK1N1fG5B1Lz2iLhGk1G5A1d2a3b4c5d6e7f8g9h0", # Placeholder
    ]
    
    # Keep track of processed signatures to avoid duplicates
    processed_signatures = set()

    while True:
        for wallet_address in wallets_to_monitor:
            logging.info(f"Checking for new transactions for wallet: {wallet_address}")
            signatures = get_signatures_for_address(api_key, wallet_address, limit=20)
            
            for sig in signatures:
                if sig not in processed_signatures:
                    logging.info(f"Found new transaction: {sig}")
                    event_data = parse_transaction(api_key, sig)
                    
                    if event_data:
                        event_json = json.dumps(event_data)
                        r.xadd("events:onchain", {"event": event_json})
                        EVENTS_PUBLISHED.inc()  # Increment the counter for published events
                        logging.info(f"Published OnChain Event: {event_data['event_type']} for {event_data['token_address']}")
                    
                    processed_signatures.add(sig)
                    # To prevent the set from growing indefinitely
                    if len(processed_signatures) > 10000:
                        processed_signatures.pop()

            # Be respectful of API rate limits
            time.sleep(5)
        
        # Wait before polling all wallets again
        logging.info("Completed a cycle of wallet checks. Waiting before next cycle...")
        time.sleep(60)

if __name__ == "__main__":
    main()
