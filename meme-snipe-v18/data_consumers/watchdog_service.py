#!/usr/bin/env python3
"""
MemeSnipe v18 - System Watchdog Service
Monitor system health, service availability, and trigger alerts
"""

import asyncio
import redis.asyncio as redis
import logging
import json
import time
from datetime import datetime, timedelta
from dataclasses import dataclass, asdict
from typing import Dict, List, Optional
import os
import aiohttp
import sys

# Setup logging
logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(name)s - %(levelname)s - %(message)s'
)
logger = logging.getLogger('watchdog')

@dataclass
class HealthStatus:
    service: str
    status: str
    last_seen: datetime
    response_time_ms: Optional[float] = None
    error_message: Optional[str] = None

class SystemWatchdog:
    def __init__(self):
        self.redis_url = os.getenv("REDIS_URL", "redis://redis:6379")
        self.redis_client = None
        self.services_to_monitor = {
            "dashboard": "http://dashboard:5000/health",
            "signer": "http://signer:8989/pubkey"
        }
        self.alert_thresholds = {
            "service_down_minutes": 5,
            "response_time_ms": 5000,
            "consecutive_failures": 3
        }
        self.service_failures: Dict[str, int] = {}
        
    async def connect_redis(self):
        """Connect to Redis with retry logic"""
        max_retries = 5
        for attempt in range(max_retries):
            try:
                self.redis_client = redis.from_url(self.redis_url)
                await self.redis_client.ping()
                logger.info("Connected to Redis successfully")
                return
            except Exception as e:
                logger.warning(f"Redis connection attempt {attempt + 1} failed: {e}")
                if attempt < max_retries - 1:
                    await asyncio.sleep(2 ** attempt)
                else:
                    raise

    async def check_service_health(self, service_name: str, health_url: str) -> HealthStatus:
        """Check health of a single service"""
        start_time = time.time()
        
        try:
            timeout = aiohttp.ClientTimeout(total=10)
            async with aiohttp.ClientSession(timeout=timeout) as session:
                async with session.get(health_url) as response:
                    response_time = (time.time() - start_time) * 1000
                    
                    if response.status == 200:
                        self.service_failures[service_name] = 0
                        return HealthStatus(
                            service=service_name,
                            status="healthy",
                            last_seen=datetime.utcnow(),
                            response_time_ms=response_time
                        )
                    else:
                        self.service_failures[service_name] = self.service_failures.get(service_name, 0) + 1
                        return HealthStatus(
                            service=service_name,
                            status="unhealthy",
                            last_seen=datetime.utcnow(),
                            response_time_ms=response_time,
                            error_message=f"HTTP {response.status}"
                        )
                        
        except Exception as e:
            self.service_failures[service_name] = self.service_failures.get(service_name, 0) + 1
            return HealthStatus(
                service=service_name,
                status="down",
                last_seen=datetime.utcnow(),
                error_message=str(e)
            )

    async def check_redis_health(self) -> HealthStatus:
        """Check Redis connectivity and performance"""
        start_time = time.time()
        
        try:
            # Test basic connectivity
            await self.redis_client.ping()
            
            # Test read/write performance
            test_key = "watchdog:health_test"
            await self.redis_client.set(test_key, "test", ex=60)
            result = await self.redis_client.get(test_key)
            
            response_time = (time.time() - start_time) * 1000
            
            if result == b"test":
                return HealthStatus(
                    service="redis",
                    status="healthy",
                    last_seen=datetime.utcnow(),
                    response_time_ms=response_time
                )
            else:
                return HealthStatus(
                    service="redis",
                    status="degraded",
                    last_seen=datetime.utcnow(),
                    response_time_ms=response_time,
                    error_message="Read/write test failed"
                )
                
        except Exception as e:
            return HealthStatus(
                service="redis",
                status="down",
                last_seen=datetime.utcnow(),
                error_message=str(e)
            )

    async def publish_health_status(self, health_statuses: List[HealthStatus]):
        """Publish health status to Redis for other services"""
        try:
            health_data = {
                "timestamp": datetime.utcnow().isoformat(),
                "services": [asdict(status) for status in health_statuses],
                "overall_status": self.calculate_overall_status(health_statuses)
            }
            
            # Convert datetime objects to strings for JSON serialization
            for service in health_data["services"]:
                if "last_seen" in service and isinstance(service["last_seen"], datetime):
                    service["last_seen"] = service["last_seen"].isoformat()
            
            await self.redis_client.publish(
                "system:health_status",
                json.dumps(health_data)
            )
            
            # Store in hash for quick lookup
            await self.redis_client.hset(
                "system:current_health",
                mapping={service["service"]: json.dumps(service) for service in health_data["services"]}
            )
            
        except Exception as e:
            logger.error(f"Failed to publish health status: {e}")

    def calculate_overall_status(self, health_statuses: List[HealthStatus]) -> str:
        """Calculate overall system status"""
        if not health_statuses:
            return "unknown"
            
        statuses = [status.status for status in health_statuses]
        
        if all(status == "healthy" for status in statuses):
            return "healthy"
        elif any(status == "down" for status in statuses):
            return "critical"
        elif any(status in ["unhealthy", "degraded"] for status in statuses):
            return "degraded"
        else:
            return "unknown"

    async def check_alert_conditions(self, health_statuses: List[HealthStatus]):
        """Check if any alert conditions are met"""
        for status in health_statuses:
            service_name = status.service
            
            # Check consecutive failures
            failures = self.service_failures.get(service_name, 0)
            if failures >= self.alert_thresholds["consecutive_failures"]:
                await self.trigger_alert(
                    f"Service {service_name} has failed {failures} consecutive health checks",
                    "critical",
                    {"service": service_name, "failures": failures, "status": status.status}
                )
            
            # Check response time
            if (status.response_time_ms and 
                status.response_time_ms > self.alert_thresholds["response_time_ms"]):
                await self.trigger_alert(
                    f"Service {service_name} response time is {status.response_time_ms:.1f}ms",
                    "warning",
                    {"service": service_name, "response_time": status.response_time_ms}
                )

    async def trigger_alert(self, message: str, severity: str, metadata: Dict):
        """Trigger an alert via Redis"""
        try:
            alert_data = {
                "timestamp": datetime.utcnow().isoformat(),
                "message": message,
                "severity": severity,
                "source": "watchdog",
                "metadata": metadata
            }
            
            await self.redis_client.publish("alerts:system", json.dumps(alert_data))
            logger.warning(f"Alert triggered: {message}")
            
        except Exception as e:
            logger.error(f"Failed to trigger alert: {e}")

    async def monitor_loop(self):
        """Main monitoring loop"""
        logger.info("Starting system watchdog monitoring...")
        
        while True:
            try:
                health_statuses = []
                
                # Check Redis health first
                redis_health = await self.check_redis_health()
                health_statuses.append(redis_health)
                
                # Check all configured services
                for service_name, health_url in self.services_to_monitor.items():
                    service_health = await self.check_service_health(service_name, health_url)
                    health_statuses.append(service_health)
                
                # Publish health status
                await self.publish_health_status(health_statuses)
                
                # Check alert conditions
                await self.check_alert_conditions(health_statuses)
                
                # Log summary
                healthy_count = sum(1 for s in health_statuses if s.status == "healthy")
                total_count = len(health_statuses)
                logger.info(f"Health check complete: {healthy_count}/{total_count} services healthy")
                
                # Wait before next check
                await asyncio.sleep(30)  # Check every 30 seconds
                
            except Exception as e:
                logger.error(f"Error in monitoring loop: {e}")
                await asyncio.sleep(10)  # Shorter wait on error

    async def run(self):
        """Run the watchdog service"""
        try:
            await self.connect_redis()
            await self.monitor_loop()
        except KeyboardInterrupt:
            logger.info("Watchdog service stopped by user")
        except Exception as e:
            logger.error(f"Watchdog service failed: {e}")
            sys.exit(1)
        finally:
            if self.redis_client:
                await self.redis_client.close()

async def main():
    """Main entry point"""
    watchdog = SystemWatchdog()
    await watchdog.run()

if __name__ == "__main__":
    asyncio.run(main())
