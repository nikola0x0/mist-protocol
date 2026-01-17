#!/bin/bash
# Copyright (c), Mysten Labs, Inc.
# SPDX-License-Identifier: Apache-2.0

# Mist Protocol Enclave Exposure Script
# Gets the enclave CID and forwards ports + sends secrets

set -e

# Check for BACKEND_PRIVATE_KEY
if [ -z "$BACKEND_PRIVATE_KEY" ]; then
    echo "ERROR: BACKEND_PRIVATE_KEY environment variable is not set"
    echo "Usage: BACKEND_PRIVATE_KEY='suiprivkey1...' ./expose_enclave.sh"
    exit 1
fi

# Gets the enclave id and CID
# expects there to be only one enclave running
ENCLAVE_ID=$(nitro-cli describe-enclaves | jq -r ".[0].EnclaveID")
ENCLAVE_CID=$(nitro-cli describe-enclaves | jq -r ".[0].EnclaveCID")

if [ "$ENCLAVE_CID" == "null" ] || [ -z "$ENCLAVE_CID" ]; then
    echo "ERROR: No enclave running. Start with 'make run' first."
    exit 1
fi

echo "Enclave ID: $ENCLAVE_ID"
echo "Enclave CID: $ENCLAVE_CID"

sleep 3

# Create secrets JSON with backend private key
echo "{\"BACKEND_PRIVATE_KEY\": \"$BACKEND_PRIVATE_KEY\"}" > secrets.json
echo "Sending secrets to enclave..."

cat secrets.json | socat - VSOCK-CONNECT:$ENCLAVE_CID:7777
rm -f secrets.json

echo "Starting port forwarders..."

# Forward app port (3000) - host:3000 <-> enclave:3000
socat TCP4-LISTEN:3000,reuseaddr,fork VSOCK-CONNECT:$ENCLAVE_CID:3000 &
echo "  Port 3000 forwarded"

# Forward VSOCK proxies for external HTTPS traffic
# These connect enclave's traffic forwarders to external endpoints
vsock-proxy 8101 fullnode.testnet.sui.io 443 &
echo "  VSOCK proxy 8101 -> fullnode.testnet.sui.io:443"

vsock-proxy 8102 seal-key-server-testnet-1.mystenlabs.com 443 &
echo "  VSOCK proxy 8102 -> seal-key-server-testnet-1.mystenlabs.com:443"

vsock-proxy 8103 seal-key-server-testnet-2.mystenlabs.com 443 &
echo "  VSOCK proxy 8103 -> seal-key-server-testnet-2.mystenlabs.com:443"

echo ""
echo "Enclave exposed! Test with:"
echo "  curl http://localhost:3000/health_check"
echo ""
echo "Press Ctrl+C to stop port forwarding"
wait
