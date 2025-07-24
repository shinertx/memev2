#!/bin/bash

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

echo "🔍 Validating .env configuration..."

if [ ! -f .env ]; then
    echo -e "${RED}❌ .env file not found!${NC}"
    exit 1
fi

# Required variables
REQUIRED_VARS=(
    "PAPER_TRADING_MODE"
    "REDIS_URL"
    "DATABASE_PATH"
    "SOLANA_RPC_URL"
    "HELIUS_API_KEY"
    "WALLET_KEYPAIR_FILENAME"
    "JITO_AUTH_KEYPAIR_FILENAME"
    "MAX_POSITION_SIZE_USD"
    "STOP_LOSS_PERCENTAGE"
    "WALLET_ADDRESS"
)

missing_vars=()
for var in "${REQUIRED_VARS[@]}"; do
    if ! grep -q "^${var}=" .env; then
        missing_vars+=("$var")
    fi
done

if [ ${#missing_vars[@]} -gt 0 ]; then
    echo -e "${RED}❌ Missing required variables:${NC}"
    printf '%s\n' "${missing_vars[@]}"
    exit 1
fi

# Check wallet files
WALLET_FILE=$(grep "^WALLET_KEYPAIR_FILENAME=" .env | cut -d'=' -f2)
JITO_FILE=$(grep "^JITO_AUTH_KEYPAIR_FILENAME=" .env | cut -d'=' -f2)

if [ ! -f "$WALLET_FILE" ]; then
    echo -e "${RED}❌ $WALLET_FILE not found!${NC}"
    exit 1
fi

if [ ! -f "$JITO_FILE" ]; then
    echo -e "${RED}❌ $JITO_FILE not found!${NC}"
    exit 1
fi

echo -e "${GREEN}✅ .env validation complete!${NC}"
