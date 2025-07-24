import redis
import json
import time
import random
import os

# Ensure all strategies in executor/src/strategies/ are registered
STRATEGY_FAMILIES = [
    "momentum_5m", 
    "mean_revert_1h", 
    "social_buzz", 
    "liquidity_migration",
    "perp_basis_arb", 
    "dev_wallet_drain", 
    "airdrop_rotation",
    "korean_time_burst", 
    "bridge_inflow", 
    "rug_pull_sniffer"
    # TODO: Add any additional strategies found in executor/src/strategies/
    # Exclude: template.rs.example
]

def get_default_params(family):
    """Gets realistic default parameters for a given strategy family."""
    if family == "momentum_5m":
        return {"lookback": 5, "vol_multiplier": 2.0, "price_change_threshold": 0.05}
    elif family == "mean_revert_1h":
        return {"period_hours": 1, "z_score_threshold": 2.0}
    elif family == "social_buzz":
        return {"lookback_minutes": 10, "std_dev_threshold": 2.5}
    elif family == "liquidity_migration":
        return {"min_volume_migrate_usd": 50000.0}
    elif family == "perp_basis_arb":
        return {"basis_threshold_pct": 0.5}
    elif family == "dev_wallet_drain":
        return {"dev_balance_threshold_pct": 2.0}
    elif family == "airdrop_rotation":
        return {"min_new_holders": 100}
    elif family == "korean_time_burst":
        return {"volume_multiplier_threshold": 1.5}
    elif family == "bridge_inflow":
        return {"min_bridge_volume_usd": 100000.0}
    elif family == "rug_pull_sniffer":
        return {"price_drop_pct": 0.8, "volume_multiplier": 5.0} # Example params for a simulated sniffer
    return {}

def main():
    print("🚀 Starting Strategy Factory & Data Simulator v18...")
    redis_url = os.getenv("REDIS_URL", "redis://redis:6379")
    r = redis.Redis.from_url(redis_url, decode_responses=True)

    # --- Publish Strategy Specs on Startup ---
    for family in STRATEGY_FAMILIES:
        spec = {
            "id": f"{family}_default",
            "family": family,
            "params": get_default_params(family)
        }
        r.xadd("strategy_registry_stream", {"spec": json.dumps(spec)})
    print(f"Published {len(STRATEGY_FAMILIES)} default strategy specs to registry stream.")

    # --- Data Simulation Loop (Comment out for live data) ---
    print("Starting data simulation loop...")
    tokens = ["SOL_MEME1", "SOL_MEME2", "SOL_MEME3", "SOL_MEME4", "SOL_MEME5"] # Example tokens
    price_data = {t: 1.0 for t in tokens} # Start all at $1.0

    while True:
        # Simulate Price Ticks for a few tokens
        for token in tokens:
            change_pct = random.uniform(-0.02, 0.02) # +/- 2% price change
            price_data[token] *= (1 + change_pct)
            if price_data[token] < 0.01: price_data[token] = 0.01 
            
            price_tick = {
                "type": "Price",
                "token_address": token,
                "price_usd": price_data[token],
                "volume_usd_1m": random.uniform(5000, 100000) # Realistic volume range
            }
            r.xadd("events:price", {"event": json.dumps(price_tick)})

        # Simulate Social Mentions (less frequent)
        if random.random() < 0.3: # 30% chance per loop iteration
            social_mention = {
                "type": "Social",
                "token_address": random.choice(tokens),
                "source": "twitter",
                "sentiment": random.uniform(-0.8, 0.8)
            }
            r.xadd("events:social", {"event": json.dumps(social_mention)})
        
        time.sleep(1) # Simulate events every second for faster testing

if __name__ == "__main__":
    main()
