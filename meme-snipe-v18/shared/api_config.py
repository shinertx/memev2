"""
Centralized API Configuration with Failover Support
Best practices for external API management and resilience
"""
import os
import random
import time
from typing import List, Dict, Optional
import requests
from dataclasses import dataclass

@dataclass
class APIEndpoint:
    """Configuration for an API endpoint with health monitoring"""
    url: str
    timeout: int = 10
    max_retries: int = 3
    weight: int = 1  # For load balancing
    last_success: float = 0
    last_failure: float = 0
    consecutive_failures: int = 0
    is_healthy: bool = True

class APIManager:
    """Centralized API management with failover and health monitoring"""
    
    def __init__(self):
        self.endpoints = {
            'solana_rpc': [
                APIEndpoint("https://mainnet.helius-rpc.com/?api-key=" + os.getenv('HELIUS_API_KEY', ''), weight=3),
                APIEndpoint("https://api.mainnet-beta.solana.com", weight=2),
                APIEndpoint("https://solana-api.projectserum.com", weight=1),
                APIEndpoint("https://rpc.ankr.com/solana", weight=1),
            ],
            'price_feeds': [
                APIEndpoint("https://hermes.pyth.network/api/latest_price_feeds", weight=3),
                APIEndpoint("https://api.coinbase.com/v2/exchange-rates", weight=2),
                APIEndpoint("https://api.coingecko.com/api/v3/simple/price", weight=2),
                APIEndpoint("https://public-api.birdeye.so/defi/price", weight=1),
            ],
            'jupiter_api': [
                APIEndpoint("https://quote-api.jup.ag/v6", weight=3),
                APIEndpoint("https://quote-api.jup.ag/v4", weight=1),  # Fallback
            ],
            'jito_rpc': [
                APIEndpoint("https://mainnet.block-engine.jito.wtf/api", weight=3),
                APIEndpoint("https://ny.mainnet.block-engine.jito.wtf/api", weight=2),
                APIEndpoint("https://amsterdam.mainnet.block-engine.jito.wtf/api", weight=1),
            ],
            'drift_api': [
                APIEndpoint("https://api.drift.trade", weight=3),
                APIEndpoint("https://dlob.drift.trade", weight=1),  # Alternative endpoint
            ]
        }
        
        # Health check intervals (seconds)
        self.health_check_interval = 300  # 5 minutes
        self.failure_threshold = 3
        self.recovery_time = 600  # 10 minutes
    
    def get_healthy_endpoint(self, service: str) -> Optional[APIEndpoint]:
        """Get a healthy endpoint for the specified service using weighted selection"""
        if service not in self.endpoints:
            return None
        
        # Filter healthy endpoints
        healthy_endpoints = [
            ep for ep in self.endpoints[service] 
            if ep.is_healthy and (time.time() - ep.last_failure) > self.recovery_time
        ]
        
        if not healthy_endpoints:
            # If no healthy endpoints, try the least recently failed
            healthy_endpoints = sorted(
                self.endpoints[service], 
                key=lambda x: x.last_failure
            )[:1]
        
        if not healthy_endpoints:
            return None
        
        # Weighted random selection
        weights = [ep.weight for ep in healthy_endpoints]
        selected = random.choices(healthy_endpoints, weights=weights)[0]
        
        return selected
    
    def make_request(self, service: str, path: str = "", method: str = "GET", 
                    params: Dict = None, data: Dict = None, headers: Dict = None) -> Optional[requests.Response]:
        """Make a request with automatic failover"""
        max_attempts = 3
        
        for attempt in range(max_attempts):
            endpoint = self.get_healthy_endpoint(service)
            if not endpoint:
                break
            
            try:
                url = endpoint.url.rstrip('/') + '/' + path.lstrip('/')
                
                response = requests.request(
                    method=method,
                    url=url,
                    params=params,
                    json=data,
                    headers=headers,
                    timeout=endpoint.timeout
                )
                
                if response.status_code < 400:
                    # Success - update health status
                    endpoint.last_success = time.time()
                    endpoint.consecutive_failures = 0
                    endpoint.is_healthy = True
                    return response
                else:
                    raise requests.HTTPError(f"HTTP {response.status_code}")
                    
            except Exception as e:
                # Mark endpoint as potentially unhealthy
                endpoint.last_failure = time.time()
                endpoint.consecutive_failures += 1
                
                if endpoint.consecutive_failures >= self.failure_threshold:
                    endpoint.is_healthy = False
                
                print(f"API request failed for {service} (attempt {attempt + 1}): {e}")
                
                if attempt == max_attempts - 1:
                    break
                
                # Short delay before retry
                time.sleep(0.5 * (attempt + 1))
        
        return None
    
    def health_check(self, service: str = None) -> Dict[str, bool]:
        """Perform health checks on endpoints"""
        services_to_check = [service] if service else self.endpoints.keys()
        results = {}
        
        for svc in services_to_check:
            for endpoint in self.endpoints[svc]:
                try:
                    # Simple HEAD request for health check
                    response = requests.head(endpoint.url, timeout=5)
                    is_healthy = response.status_code < 400
                    
                    if is_healthy:
                        endpoint.last_success = time.time()
                        endpoint.consecutive_failures = 0
                        endpoint.is_healthy = True
                    else:
                        endpoint.consecutive_failures += 1
                        if endpoint.consecutive_failures >= self.failure_threshold:
                            endpoint.is_healthy = False
                    
                    results[f"{svc}:{endpoint.url}"] = is_healthy
                    
                except Exception:
                    endpoint.consecutive_failures += 1
                    endpoint.last_failure = time.time()
                    if endpoint.consecutive_failures >= self.failure_threshold:
                        endpoint.is_healthy = False
                    results[f"{svc}:{endpoint.url}"] = False
        
        return results
    
    def get_status_report(self) -> Dict:
        """Get comprehensive status report of all endpoints"""
        report = {
            'timestamp': time.time(),
            'services': {}
        }
        
        for service, endpoints in self.endpoints.items():
            service_status = {
                'healthy_count': sum(1 for ep in endpoints if ep.is_healthy),
                'total_count': len(endpoints),
                'endpoints': []
            }
            
            for ep in endpoints:
                endpoint_status = {
                    'url': ep.url,
                    'is_healthy': ep.is_healthy,
                    'consecutive_failures': ep.consecutive_failures,
                    'last_success': ep.last_success,
                    'last_failure': ep.last_failure,
                    'weight': ep.weight
                }
                service_status['endpoints'].append(endpoint_status)
            
            report['services'][service] = service_status
        
        return report

# Global instance
api_manager = APIManager()

# Convenience functions for backward compatibility
def get_solana_rpc_url() -> str:
    """Get the best available Solana RPC URL"""
    endpoint = api_manager.get_healthy_endpoint('solana_rpc')
    return endpoint.url if endpoint else os.getenv('SOLANA_RPC_URL', 'https://api.mainnet-beta.solana.com')

def get_jupiter_api_url() -> str:
    """Get the best available Jupiter API URL"""
    endpoint = api_manager.get_healthy_endpoint('jupiter_api')
    return endpoint.url if endpoint else 'https://quote-api.jup.ag/v6'

def get_price_feed_url() -> str:
    """Get the best available price feed URL"""
    endpoint = api_manager.get_healthy_endpoint('price_feeds')
    return endpoint.url if endpoint else 'https://hermes.pyth.network/api/latest_price_feeds'

def get_jito_rpc_url() -> str:
    """Get the best available Jito RPC URL"""
    endpoint = api_manager.get_healthy_endpoint('jito_rpc')
    return endpoint.url if endpoint else 'https://mainnet.block-engine.jito.wtf/api'

def get_drift_api_url() -> str:
    """Get the best available Drift API URL"""
    endpoint = api_manager.get_healthy_endpoint('drift_api')
    return endpoint.url if endpoint else 'https://api.drift.trade'
