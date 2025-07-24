import redis
import json
import time
import os
import requests
import logging
import asyncio
from datetime import datetime
from typing import Dict, List
from prometheus_client import start_http_server, Counter
import threading

# Configure logging
logging.basicConfig(level=logging.INFO, format='%(asctime)s - %(levelname)s - %(message)s')
logger = logging.getLogger(__name__)

# Configuration
REDIS_URL = os.getenv("REDIS_URL", "redis://redis:6379")
TWITTER_BEARER_TOKEN = os.getenv("TWITTER_BEARER_TOKEN", "")
SOCIAL_CHECK_INTERVAL = int(os.getenv("SOCIAL_CHECK_INTERVAL", "60"))

# Prometheus metrics
EVENTS_PUBLISHED = Counter('social_events_published_total', 'Total number of social events published to Redis', ['source'])
API_ERRORS = Counter('social_api_errors_total', 'Total number of API errors encountered by the social consumer', ['source'])

class SocialConsumer:
    def __init__(self):
        self.redis_client = redis.from_url(REDIS_URL)
        self.last_heartbeat = time.time()
        
    async def fetch_twitter_mentions(self, query: str) -> List[Dict]:
        """
        TODO: Implement Twitter API v2 integration
        Requires TWITTER_BEARER_TOKEN in .env
        """
        if not TWITTER_BEARER_TOKEN:
            logger.warning("Twitter Bearer Token not configured - social signals disabled")
            return []
            
        # TODO: Implement actual Twitter API calls
        # headers = {"Authorization": f"Bearer {TWITTER_BEARER_TOKEN}"}
        # url = "https://api.twitter.com/2/tweets/search/recent"
        # params = {"query": query, "max_results": 100}
        return []
    
    async def process_social_signals(self):
        """Main processing loop for social data"""
        while True:
            try:
                # Publish heartbeat
                heartbeat_data = {
                    "source": "social_consumer",
                    "timestamp": datetime.now().timestamp(),
                    "status": "healthy" if TWITTER_BEARER_TOKEN else "degraded"
                }
                self.redis_client.xadd(
                    "events:data_source_heartbeat",
                    {"data": json.dumps(heartbeat_data)}
                )
                
                # TODO: Implement actual social signal processing
                # Example queries: "$SOL", "solana memecoin", etc.
                
                if not TWITTER_BEARER_TOKEN:
                    logger.info("Waiting for Twitter API configuration...")
                else:
                    # TODO: Process mentions and publish to events:social
                    pass
                    
            except Exception as e:
                logger.error(f"Error in social consumer: {e}")
                
            await asyncio.sleep(SOCIAL_CHECK_INTERVAL)

def start_metrics_server():
    """Starts a Prometheus metrics server in a background thread."""
    start_http_server(8005)
    logging.info("Prometheus metrics server started on port 8005.")

def get_twitter_bearer_token():
    """Get Twitter Bearer Token from environment."""
    token = os.getenv("TWITTER_BEARER_TOKEN")
    if not token:
        logging.error("TWITTER_BEARER_TOKEN environment variable not set.")
        raise ValueError("TWITTER_BEARER_TOKEN not set")
    return token

def search_crypto_tweets(bearer_token, query="SOL OR solana OR meme coin", max_results=10):
    """Search for crypto-related tweets using Twitter API v2."""
    url = "https://api.twitter.com/2/tweets/search/recent"
    headers = {
        "Authorization": f"Bearer {bearer_token}",
        "Content-Type": "application/json"
    }
    
    params = {
        "query": query,
        "max_results": max_results,
        "tweet.fields": "created_at,public_metrics,context_annotations,author_id",
        "expansions": "author_id",
        "user.fields": "public_metrics,verified"
    }
    
    try:
        response = requests.get(url, headers=headers, params=params, timeout=10)
        response.raise_for_status()
        return response.json()
    except requests.exceptions.RequestException as e:
        logging.error(f"Error fetching tweets: {e}")
        API_ERRORS.labels(source='twitter').inc()
        return None

def analyze_tweet_sentiment(tweet_text):
    """Simple sentiment analysis based on keywords."""
    bullish_keywords = ["moon", "pump", "bullish", "buy", "hodl", "🚀", "📈", "💎", "🔥"]
    bearish_keywords = ["dump", "crash", "bearish", "sell", "rekt", "📉", "💩", "😭"]
    
    text_lower = tweet_text.lower()
    
    bullish_score = sum(1 for keyword in bullish_keywords if keyword in text_lower)
    bearish_score = sum(1 for keyword in bearish_keywords if keyword in text_lower)
    
    if bullish_score > bearish_score:
        return "bullish", bullish_score - bearish_score
    elif bearish_score > bullish_score:
        return "bearish", bearish_score - bullish_score
    else:
        return "neutral", 0

def process_tweets(tweet_data):
    """Process tweet data and extract relevant social signals."""
    if not tweet_data or 'data' not in tweet_data:
        return []
    
    social_events = []
    
    for tweet in tweet_data['data']:
        try:
            # Analyze sentiment
            sentiment, sentiment_score = analyze_tweet_sentiment(tweet['text'])
            
            # Extract metrics
            metrics = tweet.get('public_metrics', {})
            engagement_score = (
                metrics.get('retweet_count', 0) * 3 +
                metrics.get('like_count', 0) * 1 +
                metrics.get('reply_count', 0) * 2
            )
            
            # Create social event
            event = {
                "type": "Social",
                "platform": "twitter",
                "sentiment": sentiment,
                "sentiment_score": sentiment_score,
                "engagement_score": engagement_score,
                "text": tweet['text'][:200],  # Truncate for storage
                "created_at": tweet.get('created_at'),
                "author_id": tweet.get('author_id'),
                "metrics": metrics,
                "timestamp": time.time()
            }
            
            # Only include tweets with significant engagement or strong sentiment
            if engagement_score > 10 or abs(sentiment_score) > 2:
                social_events.append(event)
                EVENTS_PUBLISHED.labels(source='twitter').inc()
            
        except Exception as e:
            logging.warning(f"Error processing tweet: {e}")
            continue
            
    return social_events
def get_reddit_crypto_posts():
    """Fetch crypto-related posts from Reddit."""
    try:
        # Use Reddit's public JSON API (no auth required for public posts)
        subreddits = ["solana", "SolanaTrading", "memecoins", "CryptoCurrency"]
        all_posts = []
        
        for subreddit in subreddits:
            url = f"https://www.reddit.com/r/{subreddit}/hot.json?limit=10"
            headers = {"User-Agent": "MemeSnipe/1.0 (crypto trading bot)"}
            
            response = requests.get(url, headers=headers, timeout=10)
            response.raise_for_status()
            
            data = response.json()
            if 'data' in data and 'children' in data['data']:
                for post in data['data']['children']:
                    post_data = post['data']
                    
                    # Simple sentiment analysis
                    title_text = post_data.get('title', '') + ' ' + post_data.get('selftext', '')
                    sentiment, sentiment_score = analyze_tweet_sentiment(title_text)
                    
                    event = {
                        "type": "Social",
                        "platform": "reddit",
                        "subreddit": subreddit,
                        "sentiment": sentiment,
                        "sentiment_score": sentiment_score,
                        "score": post_data.get('score', 0),
                        "num_comments": post_data.get('num_comments', 0),
                        "title": post_data.get('title', '')[:200],
                        "created_utc": post_data.get('created_utc'),
                        "timestamp": time.time()
                    }
                    
                    # Only include posts with significant engagement or strong sentiment
                    if post_data.get('score', 0) > 50 or abs(sentiment_score) > 2:
                        all_posts.append(event)
                        EVENTS_PUBLISHED.labels(source='reddit').inc()
        
        return all_posts
        
    except requests.exceptions.RequestException as e:
        logging.error(f"Error fetching Reddit posts: {e}")
        API_ERRORS.labels(source='reddit').inc()
        return []

def main():
    logging.info("🚀 Starting Social Media Consumer (Twitter + Reddit)...")
    
    # Start Prometheus metrics server in a background thread
    metrics_thread = threading.Thread(target=start_metrics_server, daemon=True)
    metrics_thread.start()
    
    r = redis.Redis.from_url(os.getenv("REDIS_URL", "redis://redis:6379"), decode_responses=True)
    
    # Check if Twitter API is available
    try:
        bearer_token = get_twitter_bearer_token()
        twitter_enabled = True
        logging.info("✅ Twitter API configured")
    except ValueError:
        twitter_enabled = False
        logging.warning("⚠️ Twitter API not configured, will only use Reddit")
    
    while True:
        try:
            all_social_events = []
            
            # Fetch Twitter data if available
            if twitter_enabled:
                tweet_data = search_crypto_tweets(bearer_token)
                if tweet_data:
                    twitter_events = process_tweets(tweet_data)
                    all_social_events.extend(twitter_events)
                    logging.info(f"📱 Processed {len(twitter_events)} Twitter events")
            
            # Fetch Reddit data
            reddit_events = get_reddit_crypto_posts()
            all_social_events.extend(reddit_events)
            logging.info(f"📝 Processed {len(reddit_events)} Reddit events")
            
            # Publish all social events
            for event in all_social_events:
                r.xadd("events:social", {"event": json.dumps(event)})
                # EVENTS_PUBLISHED is now incremented in the processing functions
            
            if all_social_events:
                logging.info(f"📊 Published {len(all_social_events)} social media events")
            
            # Wait before next fetch (respecting API rate limits)
            time.sleep(300)  # 5 minutes between fetches
            
        except Exception as e:
            logging.error(f"Error in social media consumer: {e}")
            time.sleep(60)  # Wait before retrying

async def main():
    consumer = SocialConsumer()
    await consumer.process_social_signals()

if __name__ == "__main__":
    logger.info("Starting Social Data Consumer (skeleton mode)")
    asyncio.run(main())
