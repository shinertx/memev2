import redis
import json
import time
import random
import os
import websocket # For live WebSocket feeds

def main():
    print("🚀 Starting Depth Event Consumer (Live/Simulator)...")
    r = redis.Redis.from_url(os.getenv("REDIS_URL", "redis://redis:6379"), decode_responses=True)
    helius_api_key = os.getenv("HELIUS_API_KEY")

    # For live data, you'd connect to a WebSocket feed (e.g., Helius LaserStream, or a DEX's WebSocket)
    # Example: ws = websocket.WebSocketApp("wss://api.helius.xyz/v0/ws?api-key=...", on_message=...)
    # This script continues to simulate for out-of-box functionality.

    tokens = ["SOL_MEME1", "SOL_MEME2", "SOL_MEME3", "SOL_MEME4", "SOL_MEME5"]
    current_prices = {t: 1.0 for t in tokens}

    while True:
        for token in tokens:
            spread_pct = random.uniform(0.001, 0.01)
            bid_price = current_prices[token] * (1 - spread_pct / 2)
            ask_price = current_prices[token] * (1 + spread_pct / 2)
            bid_size = random.uniform(1000, 10000)
            ask_size = random.uniform(1000, 10000)

            event = {
                "type": "Depth",
                "token_address": token,
                "bid_price": bid_price,
                "ask_price": ask_price,
                "bid_size_usd": bid_size,
                "ask_size_usd": ask_size,
            }
            r.xadd("events:depth", {"event": json.dumps(event)})
            current_prices[token] += random.uniform(-0.005, 0.005)
            if current_prices[token] < 0.01: current_prices[token] = 0.01

        time.sleep(1)

if __name__ == "__main__":
    main()
