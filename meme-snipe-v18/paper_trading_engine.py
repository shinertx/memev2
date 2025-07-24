#!/usr/bin/env python3
"""
Paper Trading Engine - Executes real simulated trades
Processes strategy signals and executes paper trades with realistic market conditions
"""

import os
import time
import json
import redis
import logging
import random
from datetime import datetime, timedelta
from typing import Dict, List, Tuple, Optional
from dataclasses import dataclass, asdict
from enum import Enum
import uuid

# Configure logging
logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(name)s - %(levelname)s - %(message)s'
)
logger = logging.getLogger('paper_trading_engine')

class OrderSide(Enum):
    BUY = "BUY"
    SELL = "SELL"

class OrderStatus(Enum):
    PENDING = "PENDING"
    FILLED = "FILLED"
    CANCELLED = "CANCELLED"

@dataclass
class PaperOrder:
    order_id: str
    strategy_id: str
    symbol: str
    side: OrderSide
    quantity: float
    price: float
    timestamp: datetime
    status: OrderStatus = OrderStatus.PENDING
    fill_price: Optional[float] = None
    fill_time: Optional[datetime] = None
    fees: Optional[float] = None
    slippage_pct: Optional[float] = None
    fee_pct: Optional[float] = None
    
    def to_dict(self) -> dict:
        return {
            'order_id': self.order_id,
            'strategy_id': self.strategy_id,
            'symbol': self.symbol,
            'side': self.side.value,
            'quantity': self.quantity,
            'price': self.price,
            'timestamp': self.timestamp.isoformat(),
            'status': self.status.value,
            'fill_price': self.fill_price,
            'fill_time': self.fill_time.isoformat() if self.fill_time else None
        }

@dataclass
class Position:
    strategy_id: str
    symbol: str
    quantity: float
    avg_price: float
    unrealized_pnl: float = 0.0
    realized_pnl: float = 0.0
    
    def to_dict(self) -> dict:
        return asdict(self)

class PaperTradingEngine:
    def __init__(self):
        self.redis_client = redis.Redis(host='redis', port=6379, decode_responses=True)
        self.positions: Dict[str, Dict[str, Position]] = {}  # strategy_id -> symbol -> position
        self.orders: Dict[str, PaperOrder] = {}
        self.starting_balance = 100000.0  # $100k starting balance per strategy
        self.strategy_balances: Dict[str, float] = {}
        
    def get_current_price(self, symbol: str = "SOL") -> float:
        """Get current market price from Redis streams (now using REAL data)"""
        try:
            # Get latest SOL price from Redis stream (now real Pyth/Coinbase data)
            price_events = self.redis_client.xrevrange("events:sol_price", count=1)
            if price_events:
                event_data = json.loads(price_events[0][1]['event'])
                price = float(event_data['price_usd'])
                source = event_data.get('source', 'unknown')
                
                # Log when using real vs fallback data
                if source == 'real_market_data':
                    logger.debug(f"Using REAL market price: ${price:.4f}")
                else:
                    logger.warning(f"Using fallback price: ${price:.4f}")
                    
                return price
        except Exception as e:
            logger.warning(f"Could not get price from Redis: {e}")
        
        # Emergency fallback - conservative estimate
        fallback_price = 240.0
        logger.error(f"Using emergency fallback price: ${fallback_price:.4f}")
        return fallback_price
    
    def generate_strategy_signal(self, strategy_id: str, market_data: dict) -> Optional[Tuple[OrderSide, float]]:
        """Generate trading signals based on strategy type and market data"""
        current_price = market_data.get('sol_price', 150.0)
        
        # Simple signal generation based on strategy type
        if strategy_id == "momentum_5m":
            # Momentum strategy: buy on price increases
            if random.random() < 0.1:  # 10% chance of signal
                return (OrderSide.BUY if random.random() > 0.5 else OrderSide.SELL, 
                       random.uniform(0.1, 2.0))  # 0.1-2.0 SOL
                
        elif strategy_id == "mean_revert_1h":
            # Mean reversion: buy low, sell high
            if current_price < 150:  # Below "mean"
                if random.random() < 0.15:
                    return (OrderSide.BUY, random.uniform(0.5, 3.0))
            elif current_price > 160:  # Above "mean"
                if random.random() < 0.15:
                    return (OrderSide.SELL, random.uniform(0.5, 2.0))
                    
        elif strategy_id == "bridge_inflow":
            # Bridge strategy: trade on bridge volume
            try:
                bridge_events = self.redis_client.xrevrange("events:bridge", count=1)
                if bridge_events and random.random() < 0.2:
                    return (OrderSide.BUY, random.uniform(1.0, 4.0))
            except:
                pass
                
        elif strategy_id == "social_buzz":
            # Social strategy: trade on social sentiment
            try:
                social_events = self.redis_client.xrevrange("events:social", count=1)
                if social_events and random.random() < 0.12:
                    return (OrderSide.BUY if random.random() > 0.4 else OrderSide.SELL,
                           random.uniform(0.2, 1.5))
            except:
                pass
                
        # Other strategies with basic signals
        elif random.random() < 0.08:  # 8% base signal rate
            return (OrderSide.BUY if random.random() > 0.5 else OrderSide.SELL,
                   random.uniform(0.1, 2.0))
        
        return None
    
    def calculate_realistic_slippage(self, side: OrderSide, quantity: float, current_price: float) -> float:
        """Calculate realistic slippage based on order size and market conditions"""
        # Base slippage increases with order size
        size_impact = min(quantity * current_price / 50000, 0.02)  # Max 2% for very large orders
        
        # Market volatility factor (simulated)
        volatility_factor = random.uniform(0.8, 1.5)
        
        # Time of day factor (higher slippage during low liquidity hours)
        hour = datetime.now().hour
        if 22 <= hour or hour <= 6:  # Low liquidity hours
            time_factor = 1.5
        else:
            time_factor = 1.0
        
        # Base slippage (typical for DEX)
        base_slippage = random.uniform(0.0005, 0.003)  # 0.05-0.3%
        
        total_slippage = (base_slippage + size_impact) * volatility_factor * time_factor
        
        # Cap maximum slippage at 5%
        return min(total_slippage, 0.05)
    
    def calculate_trading_fees(self, order_value: float) -> float:
        """Calculate realistic trading fees"""
        # Jupiter/Raydium typical fees
        platform_fee = 0.0025  # 0.25% platform fee
        
        # Jito tip (converted to percentage based on typical tips)
        jito_tip_usd = int(os.getenv('JITO_TIP_LAMPORTS', '100000')) / 1e9 * 200  # Assume SOL at $200
        jito_tip_pct = jito_tip_usd / order_value if order_value > 0 else 0
        
        # Network fees (gas)
        network_fee_usd = 0.01  # ~$0.01 per transaction on Solana
        network_fee_pct = network_fee_usd / order_value if order_value > 0 else 0
        
        total_fee_pct = platform_fee + jito_tip_pct + network_fee_pct
        
        # Cap fees at 2% for very small orders
        return min(total_fee_pct, 0.02)

    def place_order(self, strategy_id: str, symbol: str, side: OrderSide, quantity: float) -> PaperOrder:
        """Place a paper trading order with realistic fees and slippage"""
        current_price = self.get_current_price(symbol)
        
        # Calculate realistic slippage
        slippage_pct = self.calculate_realistic_slippage(side, quantity, current_price)
        
        if side == OrderSide.BUY:
            execution_price = current_price * (1 + slippage_pct)
        else:
            execution_price = current_price * (1 - slippage_pct)
        
        # Calculate order value and fees
        order_value = quantity * execution_price
        fee_pct = self.calculate_trading_fees(order_value)
        total_fees = order_value * fee_pct
        
        # Adjust quantity to account for fees (for buy orders)
        if side == OrderSide.BUY:
            # Fees reduce the amount of tokens received
            net_quantity = quantity * (1 - fee_pct)
        else:
            # Fees reduce the USD received
            net_quantity = quantity
            execution_price = execution_price * (1 - fee_pct)
        
        order_id = str(uuid.uuid4())[:8]
        order = PaperOrder(
            order_id=order_id,
            strategy_id=strategy_id,
            symbol=symbol,
            side=side,
            quantity=net_quantity,  # Use net quantity after fees
            price=execution_price,
            timestamp=datetime.now()
        )
        
        # Add fee information to order
        order.fees = total_fees
        order.slippage_pct = slippage_pct
        order.fee_pct = fee_pct
        
        # Simulate order execution (immediate for paper trading)
        order.status = OrderStatus.FILLED
        order.fill_price = execution_price
        order.fill_time = datetime.now()
        
        logger.info(f"Executed {side.value} order: {net_quantity:.4f} {symbol} at ${execution_price:.4f} "
                   f"(slippage: {slippage_pct*100:.3f}%, fees: ${total_fees:.2f})")
        
        self.orders[order_id] = order
        self.update_position(order)
        
        logger.info(f"Paper order executed: {strategy_id} {side.value} {quantity:.3f} {symbol} @ ${execution_price:.2f}")
        
        return order_id
    
    def update_position(self, order: PaperOrder):
        """Update position based on filled order"""
        strategy_id = order.strategy_id
        symbol = order.symbol
        
        # Initialize strategy tracking
        if strategy_id not in self.positions:
            self.positions[strategy_id] = {}
        if strategy_id not in self.strategy_balances:
            self.strategy_balances[strategy_id] = self.starting_balance
            
        # Initialize position if doesn't exist
        if symbol not in self.positions[strategy_id]:
            self.positions[strategy_id][symbol] = Position(
                strategy_id=strategy_id,
                symbol=symbol,
                quantity=0.0,
                avg_price=0.0
            )
        
        position = self.positions[strategy_id][symbol]
        
        if order.side == OrderSide.BUY:
            # Calculate new average price
            total_cost = (position.quantity * position.avg_price) + (order.quantity * order.fill_price)
            total_quantity = position.quantity + order.quantity
            
            if total_quantity > 0:
                position.avg_price = total_cost / total_quantity
            position.quantity = total_quantity
            
            # Update balance
            self.strategy_balances[strategy_id] -= order.quantity * order.fill_price
            
        else:  # SELL
            if position.quantity >= order.quantity:
                # Calculate realized PnL
                realized_pnl = order.quantity * (order.fill_price - position.avg_price)
                position.realized_pnl += realized_pnl
                position.quantity -= order.quantity
                
                # Update balance
                self.strategy_balances[strategy_id] += order.quantity * order.fill_price
                
                logger.info(f"Realized PnL for {strategy_id}: ${realized_pnl:.2f}")
            else:
                logger.warning(f"Insufficient position for sell order: {strategy_id}")
    
    def update_performance_metrics(self):
        """Update strategy performance metrics in Redis"""
        current_price = self.get_current_price()
        
        for strategy_id in self.strategy_balances.keys():
            # Calculate total value (cash + positions)
            cash_balance = self.strategy_balances[strategy_id]
            position_value = 0.0
            total_realized_pnl = 0.0
            total_unrealized_pnl = 0.0
            
            if strategy_id in self.positions:
                for symbol, position in self.positions[strategy_id].items():
                    position_value += position.quantity * current_price
                    total_realized_pnl += position.realized_pnl
                    
                    # Calculate unrealized PnL
                    if position.quantity > 0:
                        unrealized_pnl = position.quantity * (current_price - position.avg_price)
                        position.unrealized_pnl = unrealized_pnl
                        total_unrealized_pnl += unrealized_pnl
            
            total_value = cash_balance + position_value
            total_pnl = total_realized_pnl + total_unrealized_pnl
            
            # Count trades
            trade_count = len([o for o in self.orders.values() 
                             if o.strategy_id == strategy_id and o.status == OrderStatus.FILLED])
            
            # Calculate simple Sharpe ratio (returns / volatility approximation)
            returns = total_pnl / self.starting_balance
            sharpe_ratio = max(0.0, returns * 10)  # Simplified calculation
            
            # Update Redis
            performance_data = {
                'allocation_pct': '10.0',  # Maintain current allocation
                'trade_count': str(trade_count),
                'sharpe_ratio': f'{sharpe_ratio:.4f}',
                'total_pnl': f'{total_pnl:.2f}',
                'unrealized_pnl': f'{total_unrealized_pnl:.2f}',
                'realized_pnl': f'{total_realized_pnl:.2f}',
                'cash_balance': f'{cash_balance:.2f}',
                'position_value': f'{position_value:.2f}',
                'total_value': f'{total_value:.2f}',
                'mode': 'Paper',
                'last_updated': datetime.now().isoformat()
            }
            
            self.redis_client.hset(f'strategy:performance:{strategy_id}', mapping=performance_data)
            
            # Store position data
            if strategy_id in self.positions:
                for symbol, position in self.positions[strategy_id].items():
                    position_key = f'position:{strategy_id}:{symbol}'
                    self.redis_client.hset(position_key, mapping=position.to_dict())
    
    def process_trading_cycle(self):
        """Main trading cycle - generate signals and execute trades"""
        # Get current market data
        market_data = {
            'sol_price': self.get_current_price(),
            'timestamp': datetime.now()
        }
        
        # Get active strategies
        strategies = [
            'momentum_5m', 'mean_revert_1h', 'bridge_inflow', 'social_buzz',
            'liquidity_migration', 'korean_time_burst', 'airdrop_rotation',
            'dev_wallet_drain', 'rug_pull_sniffer', 'perp_basis_arb'
        ]
        
        trades_executed = 0
        
        for strategy_id in strategies:
            # Generate trading signal
            signal = self.generate_strategy_signal(strategy_id, market_data)
            
            if signal:
                side, quantity = signal
                order_id = self.place_paper_order(strategy_id, side, quantity)
                trades_executed += 1
        
        # Update performance metrics
        self.update_performance_metrics()
        
        if trades_executed > 0:
            logger.info(f"Trading cycle complete: {trades_executed} trades executed")
        
        return trades_executed

def main():
    """Main execution loop"""
    logger.info("🚀 Starting Paper Trading Engine...")
    
    engine = PaperTradingEngine()
    
    # Initialize strategy balances
    strategies = [
        'momentum_5m', 'mean_revert_1h', 'bridge_inflow', 'social_buzz',
        'liquidity_migration', 'korean_time_burst', 'airdrop_rotation',
        'dev_wallet_drain', 'rug_pull_sniffer', 'perp_basis_arb'
    ]
    
    for strategy_id in strategies:
        engine.strategy_balances[strategy_id] = engine.starting_balance
        logger.info(f"Initialized {strategy_id} with ${engine.starting_balance:,.2f}")
    
    logger.info("Paper trading engine ready! Starting trade execution...")
    
    cycle_count = 0
    while True:
        try:
            trades = engine.process_trading_cycle()
            cycle_count += 1
            
            if cycle_count % 10 == 0:  # Status update every 10 cycles
                total_trades = len([o for o in engine.orders.values() if o.status == OrderStatus.FILLED])
                logger.info(f"Status: {total_trades} total trades executed across all strategies")
            
            time.sleep(2)  # Execute trading cycle every 2 seconds
            
        except KeyboardInterrupt:
            logger.info("Shutting down paper trading engine...")
            break
        except Exception as e:
            logger.error(f"Error in trading cycle: {e}")
            time.sleep(5)

if __name__ == "__main__":
    main()
