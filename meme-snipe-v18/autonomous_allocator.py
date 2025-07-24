#!/usr/bin/env python3
"""
Autonomous Meta Allocator - Python Version
Paper trading mode for testing autonomous features
"""

import os
import time
import json
import redis
import logging
from datetime import datetime, timedelta
from typing import Dict, List, Tuple, Optional
from dataclasses import dataclass
from enum import Enum

# Configure logging
logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(name)s - %(levelname)s - %(message)s'
)
logger = logging.getLogger('autonomous_allocator')

class TradeMode(Enum):
    PAPER = "Paper"
    LIVE = "Live"

@dataclass
class StrategyAllocation:
    strategy_id: str
    allocation_pct: float
    trade_count: int
    sharpe_ratio: float
    total_pnl: float
    mode: TradeMode
    last_trade_time: Optional[datetime] = None
    
    def is_live(self) -> bool:
        """Check if strategy should be promoted to live trading"""
        if self.mode == TradeMode.LIVE:
            return True
            
        # Promotion criteria: 100+ trades and Sharpe > 1.25
        if self.trade_count >= 100 and self.sharpe_ratio > 1.25:
            return True
            
        return False
    
    def should_demote(self) -> bool:
        """Check if live strategy should be demoted to paper"""
        if self.mode == TradeMode.PAPER:
            return False
            
        # Demotion criteria: Sharpe < 0.8 or recent losses
        if self.sharpe_ratio < 0.8:
            return True
            
        return False

class AutonomousAllocator:
    def __init__(self):
        self.redis_client = redis.Redis(
            host=os.getenv('REDIS_HOST', 'redis'),
            port=int(os.getenv('REDIS_PORT', 6379)),
            db=0,
            decode_responses=True
        )
        self.paper_mode = os.getenv('PAPER_TRADING_MODE', 'true').lower() == 'true'
        self.strategies: Dict[str, StrategyAllocation] = {}
        self.allocation_interval = 300  # 5 minutes
        
        logger.info(f"Autonomous Allocator started in {'PAPER' if self.paper_mode else 'LIVE'} mode")
        
    def initialize_strategies(self):
        """Initialize strategy allocations from Redis"""
        try:
            # Get all strategy performance data
            strategy_keys = self.redis_client.keys("strategy:performance:*")
            
            if strategy_keys:
                for key in strategy_keys:
                    strategy_id = key.split(":")[-1]
                    perf_data = self.redis_client.hgetall(key)
                    
                    if perf_data:
                        allocation = StrategyAllocation(
                            strategy_id=strategy_id,
                            allocation_pct=float(perf_data.get('allocation_pct', 5.0)),
                            trade_count=int(perf_data.get('trade_count', 0)),
                            sharpe_ratio=float(perf_data.get('sharpe_ratio', 0.0)),
                            total_pnl=float(perf_data.get('total_pnl', 0.0)),
                            mode=TradeMode.PAPER if self.paper_mode else TradeMode.LIVE
                        )
                        self.strategies[strategy_id] = allocation
                        
                logger.info(f"Initialized {len(self.strategies)} strategies from Redis")
            else:
                # Initialize default strategies if Redis is empty
                logger.info("No existing strategies found in Redis, initializing defaults")
                self._initialize_default_strategies()
                
        except Exception as e:
            logger.error(f"Error initializing strategies: {e}")
            # Initialize default strategies if Redis connection fails
            self._initialize_default_strategies()
    
    def _initialize_default_strategies(self):
        """Initialize default paper trading strategies"""
        default_strategies = [
            "momentum_5m", "mean_revert_1h", "social_buzz", "bridge_inflow",
            "korean_time_burst", "liquidity_migration", "rug_pull_sniffer",
            "airdrop_rotation", "dev_wallet_drain", "perp_basis_arb"
        ]
        
        for strategy_id in default_strategies:
            allocation = StrategyAllocation(
                strategy_id=strategy_id,
                allocation_pct=10.0,  # Equal allocation
                trade_count=0,
                sharpe_ratio=0.0,
                total_pnl=0.0,
                mode=TradeMode.PAPER
            )
            self.strategies[strategy_id] = allocation
            
            # Store in Redis for persistence
            try:
                perf_data = {
                    'allocation_pct': str(allocation.allocation_pct),
                    'trade_count': str(allocation.trade_count),
                    'sharpe_ratio': str(allocation.sharpe_ratio),
                    'total_pnl': str(allocation.total_pnl),
                    'mode': allocation.mode.value,
                    'created_at': datetime.now().isoformat()
                }
                self.redis_client.hset(f"strategy:performance:{strategy_id}", mapping=perf_data)
            except Exception as e:
                logger.warning(f"Failed to store default strategy {strategy_id} in Redis: {e}")
            
        logger.info(f"Initialized {len(default_strategies)} default paper strategies")
    
    def collect_performance_metrics(self):
        """Collect latest performance metrics from Redis"""
        try:
            for strategy_id, allocation in self.strategies.items():
                # Get latest performance data
                perf_key = f"strategy:performance:{strategy_id}"
                perf_data = self.redis_client.hgetall(perf_key)
                
                if perf_data:
                    allocation.trade_count = int(perf_data.get('trade_count', allocation.trade_count))
                    allocation.sharpe_ratio = float(perf_data.get('sharpe_ratio', allocation.sharpe_ratio))
                    allocation.total_pnl = float(perf_data.get('total_pnl', allocation.total_pnl))
                    
                    # Check last trade time
                    last_trade = perf_data.get('last_trade_time')
                    if last_trade:
                        allocation.last_trade_time = datetime.fromisoformat(last_trade)
                        
            logger.info("Performance metrics updated")
            
        except Exception as e:
            logger.error(f"Error collecting performance metrics: {e}")
    
    def rebalance_allocations(self):
        """Intelligently rebalance strategy allocations"""
        try:
            # Collect current metrics
            self.collect_performance_metrics()
            
            # Calculate performance scores
            strategy_scores = {}
            total_score = 0
            
            for strategy_id, allocation in self.strategies.items():
                # Score based on Sharpe ratio, profitability, and recency
                sharpe_score = max(0, allocation.sharpe_ratio * 10)  # Weight Sharpe heavily
                pnl_score = max(0, allocation.total_pnl / 1000)  # PnL in SOL
                
                # Recency bonus (trades in last 24h)
                recency_score = 0
                if allocation.last_trade_time:
                    hours_since = (datetime.now() - allocation.last_trade_time).total_seconds() / 3600
                    if hours_since < 24:
                        recency_score = 5 * (1 - hours_since / 24)
                
                strategy_score = sharpe_score + pnl_score + recency_score
                strategy_scores[strategy_id] = max(1, strategy_score)  # Minimum score of 1
                total_score += strategy_scores[strategy_id]
            
            # Allocate based on scores
            for strategy_id, allocation in self.strategies.items():
                if total_score > 0:
                    new_allocation = (strategy_scores[strategy_id] / total_score) * 100
                    allocation.allocation_pct = round(new_allocation, 2)
                else:
                    allocation.allocation_pct = 100 / len(self.strategies)
            
            logger.info("Strategy allocations rebalanced")
            
        except Exception as e:
            logger.error(f"Error rebalancing allocations: {e}")
    
    def check_promotions_demotions(self):
        """Check for strategy promotions and demotions"""
        try:
            changes = []
            
            for strategy_id, allocation in self.strategies.items():
                old_mode = allocation.mode
                
                if allocation.mode == TradeMode.PAPER and allocation.is_live():
                    # Only promote if not in forced paper mode
                    if not self.paper_mode:
                        allocation.mode = TradeMode.LIVE
                        changes.append(f"{strategy_id}: PAPER -> LIVE")
                elif allocation.mode == TradeMode.LIVE and allocation.should_demote():
                    allocation.mode = TradeMode.PAPER
                    changes.append(f"{strategy_id}: LIVE -> PAPER")
            
            if changes:
                logger.info(f"Mode changes: {', '.join(changes)}")
            
        except Exception as e:
            logger.error(f"Error checking promotions/demotions: {e}")
    
    def publish_allocations(self):
        """Publish current allocations to Redis streams"""
        try:
            allocation_data = {}
            
            for strategy_id, allocation in self.strategies.items():
                allocation_data[strategy_id] = {
                    "allocation_pct": allocation.allocation_pct,
                    "mode": allocation.mode.value,
                    "trade_count": allocation.trade_count,
                    "sharpe_ratio": allocation.sharpe_ratio,
                    "total_pnl": allocation.total_pnl,
                    "updated_at": datetime.now().isoformat()
                }
            
            # Publish to allocation stream
            stream_data = {
                "allocations": json.dumps(allocation_data),
                "timestamp": datetime.now().isoformat(),
                "allocator_mode": "PAPER" if self.paper_mode else "LIVE"
            }
            
            self.redis_client.xadd("allocation_updates", stream_data)
            
            # Store in Redis hash for current state
            for strategy_id, data in allocation_data.items():
                self.redis_client.hset(f"allocation:current:{strategy_id}", mapping=data)
            
            logger.info(f"Published allocations for {len(allocation_data)} strategies")
            
        except Exception as e:
            logger.error(f"Error publishing allocations: {e}")
    
    def execute_live_trades(self):
        """Execute trades for live strategies by sending trade instructions to the executor"""
        if self.paper_mode:
            return  # Skip live trades in paper mode
            
        try:
            live_strategies = [s for s in self.strategies.values() if s.mode == TradeMode.LIVE]
            
            for strategy in live_strategies:
                # Check for recent signals from this strategy
                signals_key = f"strategy:signals:{strategy.strategy_id}"
                recent_signals = self.redis_client.lrange(signals_key, 0, 4)  # Get latest 5 signals
                
                for signal_data in recent_signals:
                    try:
                        signal = json.loads(signal_data)
                        
                        # Check if this signal is recent and hasn't been executed
                        signal_time = datetime.fromisoformat(signal.get('timestamp', ''))
                        if datetime.now() - signal_time > timedelta(minutes=5):
                            continue  # Skip old signals
                            
                        if signal.get('executed', False):
                            continue  # Skip already executed signals
                        
                        # Create trade instruction for the executor
                        trade_instruction = {
                            "instruction_type": "execute_trade",
                            "strategy_id": strategy.strategy_id,
                            "symbol": signal.get('symbol', 'SOL'),
                            "side": signal.get('side', 'BUY'),
                            "quantity": signal.get('quantity', 0.1),
                            "max_slippage_bps": int(os.getenv('SLIPPAGE_BPS', '50')),
                            "priority_fee_lamports": int(os.getenv('JITO_TIP_LAMPORTS', '100000')),
                            "timestamp": datetime.now().isoformat(),
                            "source": "autonomous_allocator"
                        }
                        
                        # Send to executor via Redis
                        self.redis_client.lpush("executor:trade_queue", json.dumps(trade_instruction))
                        
                        # Mark signal as executed
                        signal['executed'] = True
                        signal['execution_time'] = datetime.now().isoformat()
                        
                        logger.info(f"🚀 Sent LIVE trade instruction to executor: {strategy.strategy_id} "
                                  f"{signal.get('side')} {signal.get('quantity')} {signal.get('symbol')}")
                        
                    except Exception as e:
                        logger.error(f"Error processing signal for {strategy.strategy_id}: {e}")
                        
        except Exception as e:
            logger.error(f"Error executing live trades: {e}")

    def log_allocation_summary(self):
        """Log current allocation summary"""
        try:
            logger.info("=== ALLOCATION SUMMARY ===")
            
            paper_strategies = [s for s in self.strategies.values() if s.mode == TradeMode.PAPER]
            live_strategies = [s for s in self.strategies.values() if s.mode == TradeMode.LIVE]
            
            logger.info(f"Paper Strategies: {len(paper_strategies)}")
            logger.info(f"Live Strategies: {len(live_strategies)}")
            
            # Top performers
            sorted_strategies = sorted(
                self.strategies.values(), 
                key=lambda x: x.sharpe_ratio, 
                reverse=True
            )
            
            logger.info("Top 3 performers:")
            for i, strat in enumerate(sorted_strategies[:3]):
                logger.info(f"  {i+1}. {strat.strategy_id}: "
                          f"{strat.allocation_pct:.1f}% "
                          f"(Sharpe: {strat.sharpe_ratio:.2f}, "
                          f"Mode: {strat.mode.value})")
            
        except Exception as e:
            logger.error(f"Error logging summary: {e}")
    
    def run(self):
        """Main allocation loop"""
        logger.info("Starting autonomous allocation loop")
        
        # Initialize strategies
        self.initialize_strategies()
        
        while True:
            try:
                logger.info("=== ALLOCATION CYCLE START ===")
                
                # Rebalance allocations based on performance
                self.rebalance_allocations()
                
                # Check for promotions/demotions
                self.check_promotions_demotions()
                
                # Publish updated allocations
                self.publish_allocations()
                
                # Execute live trades for promoted strategies
                self.execute_live_trades()
                
                # Log summary
                self.log_allocation_summary()
                
                logger.info("=== ALLOCATION CYCLE COMPLETE ===")
                
                # Wait for next cycle
                time.sleep(self.allocation_interval)
                
            except KeyboardInterrupt:
                logger.info("Received shutdown signal")
                break
            except Exception as e:
                logger.error(f"Error in allocation loop: {e}")
                time.sleep(30)  # Wait before retrying

if __name__ == "__main__":
    allocator = AutonomousAllocator()
    allocator.run()
