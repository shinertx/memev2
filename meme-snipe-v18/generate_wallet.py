#!/usr/bin/env python3
import json
import secrets

# Generate a 64-byte (512-bit) keypair for Solana
# This is a test wallet - DO NOT use for real funds
keypair = [secrets.randbelow(256) for _ in range(64)]

with open('test_wallet.json', 'w') as f:
    json.dump(keypair, f)

print("Generated test_wallet.json")
print("This is a TEST wallet - do not use for real funds!")
