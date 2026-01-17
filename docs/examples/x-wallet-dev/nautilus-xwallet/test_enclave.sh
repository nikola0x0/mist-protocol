#!/bin/bash

# XWallet Enclave Test Script
# Usage: ./test_enclave.sh [tweet_url]

set -e

ENCLAVE_URL="http://localhost:3000"

echo "Testing XWallet Enclave..."
echo ""

# Test 1: Health Check
echo "1. Health Check:"
curl -s $ENCLAVE_URL/health_check | jq
echo ""
echo ""

# Test 2: Get Attestation
echo "2. Get Attestation (Public Key):"
ATTESTATION=$(curl -s $ENCLAVE_URL/get_attestation)
echo $ATTESTATION | jq -r '.attestation' | head -c 100
echo "... (truncated)"
echo ""
echo ""

# Test 3: Process Data (if tweet URL provided)
if [ -n "$1" ]; then
    echo "3. Process Transfer Tweet:"
    echo "Tweet URL: $1"
    echo ""

    RESPONSE=$(curl -s -X POST $ENCLAVE_URL/process_data \
      -H "Content-Type: application/json" \
      -d "{\"payload\": {\"tweet_url\": \"$1\"}}")

    echo "Response:"
    echo $RESPONSE | jq
    echo ""

    # Decode bytes to readable strings
    echo "Decoded Data:"
    FROM_XID=$(echo $RESPONSE | jq -r '.response.data.from_xid | tostring')
    TO_XID=$(echo $RESPONSE | jq -r '.response.data.to_xid | tostring')
    AMOUNT=$(echo $RESPONSE | jq -r '.response.data.amount')
    COIN_TYPE=$(echo $RESPONSE | jq -r '.response.data.coin_type | tostring')

    echo "  From XID: $FROM_XID"
    echo "  To XID: $TO_XID"
    echo "  Amount: $AMOUNT (MIST)"
    echo "  Amount (SUI): $(echo "scale=2; $AMOUNT / 1000000000" | bc)"
    echo "  Coin Type: $COIN_TYPE"
    echo ""

    echo "Transfer parsed successfully!"
else
    echo "3. Process Data: SKIPPED"
    echo "   Usage: ./test_enclave.sh <tweet_url>"
    echo ""
    echo "   Example tweet formats:"
    echo "   - @NautilusWallet send 5 SUI to @alice"
    echo "   - @xwallet send 10.5 USDC to @bob"
    echo "   - @wallet send 0.1 SUI to @charlie"
fi

echo ""
echo "Testing complete!"
