#!/bin/bash

echo "Adding missing critical environment variables to .env..."

# These are ACTUALLY used in the code
cat >> .env << 'EOF'

# === MISSING CRITICAL VARIABLES ===
# Data Consumer Configuration
BYBIT_API_ENABLED=false
DB_PATH=/app/shared/trades_v18.db  # Same as DATABASE_PATH
SOCIAL_CHECK_INTERVAL=60

# Missing API Keys (some might be duplicates with different names)
HELIUS_RPC_URL=https://mainnet.helius-rpc.com/?api-key=b82ea58f-d4d4-4f2d-b4ea-afb52d2dde23
JUPITER_API_KEY=
DRIFT_API_KEY=

# Risk Guardian Configuration
MAX_PORTFOLIO_VAR=0.20
MAX_POSITION_COUNT=10

# Wallet Configuration
WALLET_ADDRESS=  # Your wallet's public address
EOF

echo "✅ Added missing variables to .env"
