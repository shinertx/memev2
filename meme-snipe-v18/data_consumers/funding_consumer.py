import redis
import json
import time
import random
import os
import requests # For Drift API calls

def main():
    print("🚀 Starting Funding Event Consumer (Live/Simulator)...")
    r = redis.Redis.from_url(os.getenv("REDIS_URL", "redis://redis:6379"), decode_responses=True)
    drift_api_url = os.getenv("DRIFT_API_URL")

    # For live data, you'd poll or subscribe to a Drift API endpoint for funding rates.
    # Example: requests.get(f"{drift_api_url}/markets/funding_rates")
    # This script continues to simulate for out-of-box functionality.

    tokens = ["SOL_MEME1", "SOL_MEME2", "SOL_MEME3", "SOL_MEME4", "SOL_MEME5"]

    while True:
        for token in tokens:
            funding_rate_pct = random.uniform(-0.005, 0.005)
            next_funding_time_sec = int(time.time()) + random.randint(1, 8) * 3600

            event = {
                "type": "Funding",
                "token_address": token,
                "funding_rate_pct": funding_rate_pct,
                "next_funding_time_sec": next_funding_time_sec,
            }
            r.xadd("events:funding", {"event": json.dumps(event)})
        
        time.sleep(30)

if __name__ == "__main__":
    main()
