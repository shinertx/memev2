#!/usr/bin/env python3
"""
Real Market Data Consumer - Fetches actual market data from multiple sources
Replaces simulated depth, volume, and order book data with real feeds
"""

import redis
import json
import time
import os
import requests
import asyncio
import websockets
from typing import Dict, Optional
import logging

# Configure logging
logging.basicConfig(level=logging.INFO)
logger = logging.getLogger('real_market_data')

class RealMarketDataConsumer:
    def __init__(self):
        self.redis_client = redis.Redis.from_url(
            os.getenv("REDIS_URL", "redis://redis:6379"), 
            decode_responses=True
        )
        self.jupiter_api = "https://quote-api.jup.ag/v6"
        
    def get_real_sol_volume_birdeye(self) -> Optional[Dict]:
        """Get real SOL trading volume from Birdeye API (free tier)"""
        try:
            # Birdeye API for SOL trading data
            url = "https://public-api.birdeye.so/defi/price"
            params = {
                "address": "So11111111111111111111111111111111111111112"  # SOL mint
            }
            headers = {
                "X-API-KEY": os.getenv("BIRDEYE_API_KEY", "")  # Optional, works without key
            }
            
            response = requests.get(url, params=params, headers=headers, timeout=10)
            if response.status_code == 200:
                data = response.json()
                if data.get('success') and 'data' in data:
                    price_data = data['data']
                    return {
                        "price": price_data.get('value', 0),
                        "volume24h": price_data.get('volume24hUsd', 0),
                        "liquidity": price_data.get('liquidity', 0),
                        "price_change_24h": price_data.get('priceChange24hPercent', 0)
                    }
                    
        except Exception as e:
            logger.warning(f"Birdeye API error: {e}")
            return None
    
    def get_real_dex_liquidity(self) -> Optional[Dict]:
        """Get real DEX liquidity data from Jupiter"""
        try:
            # Get SOL/USDC liquidity from Jupiter
            url = f"{self.jupiter_api}/quote"
            params = {
                "inputMint": "So11111111111111111111111111111111111111112",  # SOL
                "outputMint": "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",  # USDC
                "amount": "1000000000",  # 1 SOL in lamports
                "slippageBps": "50"
            }
            
            response = requests.get(url, params=params, timeout=10)
            if response.status_code == 200:
                quote_data = response.json()
                return {
                    "input_amount": quote_data.get('inAmount', 0),
                    "output_amount": quote_data.get('outAmount', 0),
                    "price_impact": quote_data.get('priceImpactPct', 0),
                    "route_plan": len(quote_data.get('routePlan', []))
                }
                
        except Exception as e:
            logger.warning(f"Jupiter API error: {e}")
            return None
            
    def get_coingecko_market_data(self) -> Optional[Dict]:
        """Get comprehensive market data from CoinGecko (free API)"""
        try:
            url = "https://api.coingecko.com/api/v3/simple/price"
            params = {
                "ids": "solana",
                "vs_currencies": "usd",
                "include_market_cap": "true",
                "include_24hr_vol": "true",
                "include_24hr_change": "true"
            }
            
            response = requests.get(url, params=params, timeout=10)
            if response.status_code == 200:
                data = response.json()
                if 'solana' in data:
                    sol_data = data['solana']
                    return {
                        "price_usd": sol_data.get('usd', 0),
                        "market_cap": sol_data.get('usd_market_cap', 0),
                        "volume_24h": sol_data.get('usd_24h_vol', 0),
                        "price_change_24h": sol_data.get('usd_24h_change', 0)
                    }
                    
        except Exception as e:
            logger.warning(f"CoinGecko API error: {e}")
            return None
    
    def publish_market_event(self, event_type: str, data: Dict):
        """Publish real market data to Redis streams"""
        try:
            event = {
                "type": event_type,
                "timestamp": time.time(),
                "source": "real_market_data",
                **data
            }
            
            stream_name = f"events:{event_type.lower()}"
            self.redis_client.xadd(stream_name, {"event": json.dumps(event)})
            logger.info(f"📊 Published REAL {event_type}: {data}")
            
        except Exception as e:
            logger.error(f"Failed to publish {event_type}: {e}")
    
    def run_market_data_collection(self):
        """Main loop to collect and publish real market data"""
        logger.info("🚀 Starting Real Market Data Consumer...")
        logger.info("📊 Data sources: Birdeye, Jupiter, CoinGecko")
        
        while True:
            try:
                # Collect real volume and liquidity data
                birdeye_data = self.get_real_sol_volume_birdeye()
                if birdeye_data:
                    self.publish_market_event("Volume", {
                        "token_address": "SOL",
                        "volume_24h_usd": birdeye_data["volume24h"],
                        "liquidity_usd": birdeye_data["liquidity"],
                        "price_change_24h": birdeye_data["price_change_24h"]
                    })
                
                # Collect real DEX depth data
                jupiter_data = self.get_real_dex_liquidity()
                if jupiter_data:
                    self.publish_market_event("Depth", {
                        "token_address": "SOL",
                        "price_impact_pct": jupiter_data["price_impact"],
                        "liquidity_depth": jupiter_data["output_amount"],
                        "route_complexity": jupiter_data["route_plan"]
                    })
                
                # Collect comprehensive market data
                coingecko_data = self.get_coingecko_market_data()
                if coingecko_data:
                    self.publish_market_event("Market", {
                        "token_address": "SOL",
                        "market_cap_usd": coingecko_data["market_cap"],
                        "volume_24h_usd": coingecko_data["volume_24h"],
                        "price_usd": coingecko_data["price_usd"]
                    })
                    
            except Exception as e:
                logger.error(f"Market data collection error: {e}")
            
            # Update every 30 seconds (reasonable for free APIs)
            time.sleep(30)

def main():
    consumer = RealMarketDataConsumer()
    consumer.run_market_data_collection()

if __name__ == "__main__":
    main()
