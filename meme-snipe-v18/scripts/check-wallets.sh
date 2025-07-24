#!/bin/bash

# Check wallet files using correct env variable names
WALLET_FILE=$(grep "^WALLET_KEYPAIR_FILENAME=" .env | cut -d'=' -f2)
JITO_FILE=$(grep "^JITO_AUTH_KEYPAIR_FILENAME=" .env | cut -d'=' -f2)

if [ -f "$WALLET_FILE" ]; then
    echo "✅ Wallet file found: $WALLET_FILE"
else
    echo "❌ Wallet file not found: $WALLET_FILE"
fi

if [ -f "$JITO_FILE" ]; then
    echo "✅ Jito auth file found: $JITO_FILE"
else
    echo "❌ Jito auth file not found: $JITO_FILE"
fi
