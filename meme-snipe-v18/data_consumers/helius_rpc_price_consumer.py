import redis
import json
import time
import os
import requests
import random

def get_sol_price_from_helius(api_key: str) -> float | None:
    """Fetches the current SOL/USD price from Helius RPC (or simulates)."""
    url = f"https://rpc.helius.xyz/?api-key={api_key}"
    headers = {"Content-Type": "application/json"}
    payload = {
        "jsonrpc": "2.0",
        "id": 1,
        "method": "getAsset",
        "params": {
            "id": "So11111111111111111111111111111111111111112" # SOL mint address
        }
    }
    try:
        response = requests.post(url, headers=headers, json=payload, timeout=5)
        response.raise_for_status()
        data = response.json()
        
        # This is a placeholder for actual price extraction from Helius getAsset or a dedicated oracle.
        # Helius getAsset might not directly provide USD price for native SOL.
        # For production, integrate with Pyth Network (via Pyth client library) for robust SOL/USD price.
        
        # Simulating a price for now, even if API key is present, as direct extraction is complex.
        simulated_price = 150.0 + (time.time() % 1000) / 100.0 
        return simulated_price
    except Exception as e:
        print(f"Error fetching SOL price from Helius: {e}. Simulating price.")
        simulated_price = 150.0 + (time.time() % 1000) / 100.0 
        return simulated_price

def main():
    print("🚀 Starting Helius RPC SOL Price Consumer (Live/Simulator)...")
    redis_url = os.getenv("REDIS_URL", "redis://redis:6379")
    helius_api_key = os.getenv("HELIUS_API_KEY")
    r = redis.Redis.from_url(redis_url, decode_responses=True)

    if not helius_api_key:
        print("WARNING: HELIUS_API_KEY not set. SOL price will be purely simulated.")

    while True:
        sol_price = get_sol_price_from_helius(helius_api_key)
        if sol_price is None:
            sol_price = 150.0 + random.uniform(-5.0, 5.0) # Fallback simulation

        event = {
            "type": "SolPrice",
            "price_usd": sol_price
        }
        r.xadd("events:sol_price", {"event": json.dumps(event)})
        print(f"Published SOL price: ${sol_price:.2f}")
        time.sleep(5)

if __name__ == "__main__":
    main()
