import os
import redis

class BaseConsumer:
    def __init__(self):
        # Use REDIS_URL instead of separate host/port
        self.redis_url = os.getenv('REDIS_URL', 'redis://redis:6379')
        self.redis_client = self._connect_redis()
        
    def _connect_redis(self):
        """Connect to Redis using the URL format"""
        return redis.from_url(self.redis_url, decode_responses=True)