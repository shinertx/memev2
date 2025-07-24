import redis
import json
import time
import os
import requests
import logging
from prometheus_client import start_http_server, Counter
import threading

# Configure logging
logging.basicConfig(level=logging.INFO, format='%(asctime)s - %(levelname)s - %(message)s')

# Prometheus metrics
EVENTS_PUBLISHED = Counter('depth_events_published_total', 'Total number of depth events published to Redis')
API_ERRORS = Counter('depth_api_errors_total', 'Total number of API errors encountered by the depth consumer')

def start_metrics_server():
    """Starts a Prometheus metrics server in a background thread."""
    start_http_server(8000)
    logging.info("Prometheus metrics server started on port 8000.")

def get_jupiter_depth(token_address):
    """
    Fetches order book depth from Jupiter's v6 API for a given token.
    Note: Jupiter's API provides quotes, which we can use to infer depth.
    For more granular depth, a direct connection to a DEX (like Serum or Openbook) would be needed,
    but Jupiter provides a good aggregated view.
    """
    # URL for Jupiter's v6 Quote API
    # We'll query for a large size to see the price impact, which indicates depth.
    # We'll check for both sides of the book by swapping SOL for the token and vice-versa.
    base_url = "https://quote-api.jup.ag/v6/quote"
    
    # Let's assume we are trading against USDC for this example.
    # A more robust implementation would handle multiple quote currencies.
    usdc_mint = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"
    
    # Query for buying the token (selling USDC)
    buy_params = {
        "inputMint": usdc_mint,
        "outputMint": token_address,
        "amount": 10000 * 10**6,  # 10,000 USDC (assuming 6 decimals for USDC)
        "slippageBps": 100 # 1% slippage
    }
    
    # Query for selling the token (buying USDC)
    sell_params = {
        "inputMint": token_address,
        "outputMint": usdc_mint,
        "amount": 10000 * 10**6, # A large amount of the token, assuming 6 decimals
        "slippageBps": 100
    }

    try:
        # Get price for buying the token
        buy_response = requests.get(base_url, params=buy_params)
        buy_quote = buy_response.json()
        
        # Get price for selling the token
        # To get a sell quote, we need to know how much of the token to sell.
        # Let's first get a price for 1 token, then use that to sell a larger amount.
        price_check_params = {
            "inputMint": token_address,
            "outputMint": usdc_mint,
            "amount": 1 * 10**6, # 1 token (assuming 6 decimals)
        }
        price_check_response = requests.get(base_url, params=price_check_params)
        price_check_quote = price_check_response.json()
        
        if 'outAmount' not in price_check_quote:
             logging.error(f"Could not get price for {token_address}")
             return None

        # Now sell a larger amount based on the price
        amount_to_sell = int(10000 / (int(price_check_quote['outAmount']) / 10**6)) * 10**6 # $10k worth
        sell_params['amount'] = amount_to_sell
        
        sell_response = requests.get(base_url, params=sell_params)
        sell_quote = sell_response.json()

        if 'inAmount' not in buy_quote or 'outAmount' not in sell_quote:
            logging.warning(f"Could not get full depth for {token_address}")
            return None

        # Infer bid/ask from the quotes
        # Buy quote: how much of outputMint you get for inputMint
        # Sell quote: how much of outputMint you get for inputMint
        
        ask_price = int(buy_quote['inAmount']) / int(buy_quote['outAmount']) # Price in input per output
        bid_price = int(sell_quote['outAmount']) / int(sell_quote['inAmount']) # Price in output per input

        # This is a simplification. The size is what we queried.
        ask_size_usd = 10000
        bid_size_usd = 10000

        return {
            "bid_price": bid_price,
            "ask_price": ask_price,
            "bid_size_usd": bid_size_usd,
            "ask_size_usd": ask_size_usd,
        }
        
    except requests.exceptions.RequestException as e:
        logging.error(f"Error fetching depth from Jupiter API: {e}")
        API_ERRORS.inc()
        return None
    except Exception as e:
        logging.error(f"An unexpected error occurred: {e}")
        API_ERRORS.inc()
        return None


def main():
    logging.info("🚀 Starting Depth Event Consumer (Jupiter v6)...")
    # Start Prometheus metrics server in a background thread
    metrics_thread = threading.Thread(target=start_metrics_server, daemon=True)
    metrics_thread.start()
    r = redis.Redis.from_url(os.getenv("REDIS_URL", "redis://redis:6379"), decode_responses=True)
    
    # List of tokens to monitor - should be managed elsewhere in a real system (e.g., config file, database)
    tokens_to_monitor = [
        "JUP", # Jupiter
        "WIF", # dogwifhat
        "BONK", # Bonk
        # Add other meme tokens or tokens of interest here
    ]
    
    # For this example, we'll use their symbols. For the API, we need mint addresses.
    # A real implementation needs a mapping from symbol to mint address.
    # Let's use a few known ones for now.
    token_map = {
        "JUP": "JUPyiwrYJFskUPiHa7hkeR8VUtAeFoSYbKedZNsDvCN",
        "WIF": "EKpQGSJtjMFqKZ9KQanSqYXRcF8fBopzLHYxdM65zcjm",
        "BONK": "DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263",
        "SOL": "So11111111111111111111111111111111111111112", # For reference
        "USDC": "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v" # For reference
    }

    # Start the Prometheus metrics server in a background thread
    metrics_thread = threading.Thread(target=start_metrics_server, daemon=True)
    metrics_thread.start()

    while True:
        for token_symbol in tokens_to_monitor:
            token_address = token_map.get(token_symbol)
            if not token_address:
                logging.warning(f"Mint address for token {token_symbol} not found.")
                continue

            logging.info(f"Fetching depth for {token_symbol} ({token_address})")
            depth_data = get_jupiter_depth(token_address)

            if depth_data:
                event = {
                    "type": "Depth",
                    "token_address": token_address, # Using mint address as the canonical ID
                    "token_symbol": token_symbol,
                    "bid_price": depth_data["bid_price"],
                    "ask_price": depth_data["ask_price"],
                    "bid_size_usd": depth_data["bid_size_usd"],
                    "ask_size_usd": depth_data["ask_size_usd"],
                }
                event_json = json.dumps(event)
                r.xadd("events:depth", {"event": event_json})
                EVENTS_PUBLISHED.inc()
                logging.info(f"Published depth event for {token_symbol} to Redis stream 'events:depth'")
                EVENTS_PUBLISHED.inc()  # Increment the counter for published events
            else:
                logging.warning(f"Failed to fetch or process depth data for {token_symbol}")

        # Sleep for a reasonable interval. Jupiter's API has rate limits.
        # Polling every 10-15 seconds per token is reasonable.
        time.sleep(15)

if __name__ == "__main__":
    main()
