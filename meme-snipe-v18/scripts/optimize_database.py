#!/usr/bin/env python3
"""
Database optimization script for MemeSnipe v18
Creates indexes and optimizes SQLite database for trading performance
"""

import sqlite3
import logging
import time
from datetime import datetime

# Configure logging
logging.basicConfig(level=logging.INFO, format='%(asctime)s - %(levelname)s - %(message)s')
logger = logging.getLogger(__name__)

DB_PATH = 'shared/trades_v18.db'

def optimize_database():
    """Apply performance optimizations to the trading database"""
    logger.info("🔧 Starting database optimization...")
    
    try:
        conn = sqlite3.connect(DB_PATH)
        cursor = conn.cursor()
        
        # Enable WAL mode for better concurrency
        logger.info("Setting WAL mode...")
        cursor.execute("PRAGMA journal_mode=WAL;")
        
        # Set performance pragmas
        logger.info("Applying performance settings...")
        cursor.execute("PRAGMA synchronous=NORMAL;")
        cursor.execute("PRAGMA cache_size=10000;")  # 10MB cache
        cursor.execute("PRAGMA temp_store=MEMORY;")
        cursor.execute("PRAGMA mmap_size=268435456;")  # 256MB mmap
        
        # Create performance indexes
        logger.info("Creating performance indexes...")
        
        indexes = [
            # Primary trading indexes
            ("idx_trades_status", "CREATE INDEX IF NOT EXISTS idx_trades_status ON trades(status);"),
            ("idx_trades_timestamp", "CREATE INDEX IF NOT EXISTS idx_trades_timestamp ON trades(timestamp);"),
            ("idx_trades_strategy", "CREATE INDEX IF NOT EXISTS idx_trades_strategy ON trades(strategy_id);"),
            ("idx_trades_symbol", "CREATE INDEX IF NOT EXISTS idx_trades_symbol ON trades(symbol);"),
            ("idx_trades_pnl", "CREATE INDEX IF NOT EXISTS idx_trades_pnl ON trades(pnl_usd);"),
            
            # Composite indexes for common queries
            ("idx_trades_status_timestamp", "CREATE INDEX IF NOT EXISTS idx_trades_status_timestamp ON trades(status, timestamp);"),
            ("idx_trades_strategy_timestamp", "CREATE INDEX IF NOT EXISTS idx_trades_strategy_timestamp ON trades(strategy_id, timestamp);"),
            ("idx_trades_strategy_status", "CREATE INDEX IF NOT EXISTS idx_trades_strategy_status ON trades(strategy_id, status);"),
            
            # Position management indexes
            ("idx_positions_status", "CREATE INDEX IF NOT EXISTS idx_positions_status ON positions(status);"),
            ("idx_positions_symbol", "CREATE INDEX IF NOT EXISTS idx_positions_symbol ON positions(symbol);"),
            ("idx_positions_timestamp", "CREATE INDEX IF NOT EXISTS idx_positions_timestamp ON positions(created_at);"),
            
            # Performance tracking indexes
            ("idx_perf_strategy", "CREATE INDEX IF NOT EXISTS idx_perf_strategy ON strategy_performance(strategy_id);"),
            ("idx_perf_timestamp", "CREATE INDEX IF NOT EXISTS idx_perf_timestamp ON strategy_performance(timestamp);"),
            ("idx_perf_strategy_time", "CREATE INDEX IF NOT EXISTS idx_perf_strategy_time ON strategy_performance(strategy_id, timestamp);"),
            
            # Risk management indexes
            ("idx_risk_events_timestamp", "CREATE INDEX IF NOT EXISTS idx_risk_events_timestamp ON risk_events(timestamp);"),
            ("idx_risk_events_type", "CREATE INDEX IF NOT EXISTS idx_risk_events_type ON risk_events(event_type);"),
            ("idx_risk_events_severity", "CREATE INDEX IF NOT EXISTS idx_risk_events_severity ON risk_events(severity);")
        ]
        
        for idx_name, sql in indexes:
            try:
                cursor.execute(sql)
                logger.info(f"✅ Created index: {idx_name}")
            except sqlite3.Error as e:
                logger.warning(f"⚠️  Index {idx_name} creation warning: {e}")
        
        # Analyze tables for query optimization
        logger.info("Analyzing tables for query optimization...")
        tables = ['trades', 'positions', 'strategy_performance', 'risk_events']
        for table in tables:
            try:
                cursor.execute(f"ANALYZE {table};")
                logger.info(f"✅ Analyzed table: {table}")
            except sqlite3.Error as e:
                logger.warning(f"⚠️  Table {table} analysis warning: {e}")
        
        # Create materialized view for common aggregations
        logger.info("Creating performance views...")
        try:
            cursor.execute("""
                CREATE VIEW IF NOT EXISTS strategy_daily_performance AS
                SELECT 
                    strategy_id,
                    DATE(timestamp) as date,
                    COUNT(*) as trade_count,
                    SUM(pnl_usd) as daily_pnl,
                    AVG(pnl_usd) as avg_pnl,
                    COUNT(CASE WHEN pnl_usd > 0 THEN 1 END) as winning_trades,
                    COUNT(CASE WHEN pnl_usd < 0 THEN 1 END) as losing_trades
                FROM trades 
                WHERE status = 'completed'
                GROUP BY strategy_id, DATE(timestamp);
            """)
            logger.info("✅ Created strategy_daily_performance view")
        except sqlite3.Error as e:
            logger.warning(f"⚠️  View creation warning: {e}")
        
        # Commit all changes
        conn.commit()
        
        # VACUUM to reclaim space and optimize layout
        logger.info("Running VACUUM to optimize database layout...")
        cursor.execute("VACUUM;")
        
        # Get database stats
        cursor.execute("SELECT name FROM sqlite_master WHERE type='index';")
        index_count = len(cursor.fetchall())
        
        cursor.execute("SELECT name FROM sqlite_master WHERE type='table';")
        table_count = len(cursor.fetchall())
        
        cursor.execute("PRAGMA page_count;")
        page_count = cursor.fetchone()[0]
        
        cursor.execute("PRAGMA page_size;")
        page_size = cursor.fetchone()[0]
        
        db_size_mb = (page_count * page_size) / (1024 * 1024)
        
        logger.info(f"📊 Database optimization complete:")
        logger.info(f"   - Tables: {table_count}")
        logger.info(f"   - Indexes: {index_count}")
        logger.info(f"   - Size: {db_size_mb:.2f} MB")
        logger.info(f"   - Pages: {page_count}")
        
        conn.close()
        
    except Exception as e:
        logger.error(f"❌ Database optimization failed: {e}")
        raise

def validate_database_structure():
    """Validate that all required tables and columns exist"""
    logger.info("🔍 Validating database structure...")
    
    try:
        conn = sqlite3.connect(DB_PATH)
        cursor = conn.cursor()
        
        # Check for required tables
        required_tables = ['trades', 'positions', 'strategy_performance']
        cursor.execute("SELECT name FROM sqlite_master WHERE type='table';")
        existing_tables = {row[0] for row in cursor.fetchall()}
        
        missing_tables = set(required_tables) - existing_tables
        if missing_tables:
            logger.warning(f"⚠️  Missing tables: {missing_tables}")
            logger.info("Run init_db.py to create missing tables")
        else:
            logger.info("✅ All required tables present")
        
        # Check trades table structure
        if 'trades' in existing_tables:
            cursor.execute("PRAGMA table_info(trades);")
            columns = {row[1] for row in cursor.fetchall()}
            required_columns = {'id', 'strategy_id', 'symbol', 'side', 'status', 'timestamp', 'pnl_usd'}
            missing_columns = required_columns - columns
            if missing_columns:
                logger.warning(f"⚠️  Missing columns in trades table: {missing_columns}")
            else:
                logger.info("✅ Trades table structure valid")
        
        conn.close()
        
    except Exception as e:
        logger.error(f"❌ Database validation failed: {e}")
        raise

def main():
    """Main optimization routine"""
    start_time = time.time()
    
    logger.info("🚀 Starting MemeSnipe v18 database optimization...")
    
    try:
        # Validate structure first
        validate_database_structure()
        
        # Apply optimizations
        optimize_database()
        
        duration = time.time() - start_time
        logger.info(f"✅ Database optimization completed in {duration:.2f} seconds")
        
    except Exception as e:
        logger.error(f"❌ Optimization failed: {e}")
        return False
    
    return True

if __name__ == "__main__":
    success = main()
    exit(0 if success else 1)
