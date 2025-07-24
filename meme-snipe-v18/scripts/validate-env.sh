#!/bin/bash

# Script to validate .env configuration for platinum Docker setup

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

echo "🔍 Validating .env configuration..."

# Check if .env exists
if [ ! -f .env ]; then
    echo -e "${RED}❌ .env file not found!${NC}"
    echo "   Run: cp .env.example .env"
    exit 1
fi

# Required variables
REQUIRED_VARS=(
    "PAPER_TRADING_MODE"
    "REDIS_HOST"
    "REDIS_PORT"
    "DATABASE_PATH"
    "SOLANA_RPC_URL"
    "HELIUS_API_KEY"
    "WALLET_PATH"
    "JITO_AUTH_KEY_PATH"
    "MAX_POSITION_SIZE_USD"
    "STOP_LOSS_PERCENTAGE"
)

# Check required variables
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

# Check wallet files exist
source .env

if [ ! -f "my_wallet.json" ]; then
    echo -e "${RED}❌ my_wallet.json not found in project root!${NC}"
    exit 1
fi

if [ ! -f "jito_auth_key.json" ]; then
    echo -e "${RED}❌ jito_auth_key.json not found in project root!${NC}"
    exit 1
fi

# Check trading mode
if [ "$PAPER_TRADING_MODE" = "false" ]; then
    echo -e "${YELLOW}⚠️  WARNING: LIVE TRADING MODE ENABLED!${NC}"
    echo "   Ensure all risk parameters are properly configured."
fi

# Validate Docker-specific settings
if grep -q "docker-compose.prod.yml" .env 2>/dev/null; then
    echo -e "${YELLOW}⚠️  Found reference to old docker-compose.prod.yml${NC}"
    echo "   Update to use docker-compose.yml only"
fi

# Check for localhost references that should use service names
if grep -E "localhost|127\.0\.0\.1" .env | grep -v "^#"; then
    echo -e "${YELLOW}⚠️  Found localhost references:${NC}"
    grep -E "localhost|127\.0\.0\.1" .env | grep -v "^#"
    echo "   Consider using Docker service names (e.g., 'redis' instead of 'localhost')"
fi

echo -e "${GREEN}✅ .env validation complete!${NC}"

# Show current configuration summary
echo ""
echo "📋 Configuration Summary:"
echo "   Trading Mode: $(grep PAPER_TRADING_MODE .env | cut -d= -f2)"
echo "   Redis Host: $(grep REDIS_HOST .env | cut -d= -f2)"
echo "   Max Position: $(grep MAX_POSITION_SIZE_USD .env | cut -d= -f2)"
echo "   Stop Loss: $(grep STOP_LOSS_PERCENTAGE .env | cut -d= -f2)"
