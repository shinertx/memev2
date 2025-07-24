#!/usr/bin/env python3

import sqlite3
import os

# Database path 
db_path = '/home/benjaminjones/memev2/meme-snipe-v18/shared/trades_v18.db'

# Create database and tables
conn = sqlite3.connect(db_path)
cursor = conn.cursor()

# Create trades table with the exact schema from the Rust code
cursor.execute('''
    CREATE TABLE IF NOT EXISTS trades (
        id INTEGER PRIMARY KEY,
        strategy_id TEXT NOT NULL,
        token_address TEXT NOT NULL,
        symbol TEXT NOT NULL,
        amount_usd REAL NOT NULL,
        status TEXT NOT NULL, -- PENDING, OPEN, CLOSED_PROFIT, CLOSED_LOSS, CANCELED
        signature TEXT,
        entry_time INTEGER NOT NULL,
        entry_price_usd REAL NOT NULL,
        close_time INTEGER,
        close_price_usd REAL,
        pnl_usd REAL,
        confidence REAL NOT NULL,
        side TEXT NOT NULL, -- NEW
        highest_price_usd REAL -- NEW
    )
''')

# Insert some sample data for testing
sample_trades = [
    ('momentum_5m', 'So11111111111111111111111111111111111111112', 'SOL', 100.0, 'CLOSED_PROFIT', 'sample_sig_1', 1721696400, 201.5, 1721696500, 204.2, 13.43, 0.85, 'Long', 204.2),
    ('mean_revert_1h', 'EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v', 'USDC', 250.0, 'CLOSED_LOSS', 'sample_sig_2', 1721696300, 1.001, 1721696450, 0.998, -0.75, 0.72, 'Long', 1.002),
    ('korean_time_burst', 'So11111111111111111111111111111111111111112', 'SOL', 150.0, 'OPEN', 'sample_sig_3', 1721697000, 202.0, None, None, None, 0.91, 'Long', 203.1),
    ('bridge_inflow', 'So11111111111111111111111111111111111111112', 'SOL', 200.0, 'CLOSED_PROFIT', 'sample_sig_4', 1721696200, 200.8, 1721696350, 205.5, 46.89, 0.88, 'Long', 205.5),
    ('perp_basis_arb', 'EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v', 'USDC', 300.0, 'OPEN', 'sample_sig_5', 1721697100, 1.000, None, None, None, 0.76, 'Short', 0.999)
]

cursor.executemany('''
    INSERT INTO trades (strategy_id, token_address, symbol, amount_usd, status, signature, entry_time, entry_price_usd, close_time, close_price_usd, pnl_usd, confidence, side, highest_price_usd)
    VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
''', sample_trades)

conn.commit()
conn.close()

print(f"Database initialized at {db_path}")
print(f"Created {len(sample_trades)} sample trades")
