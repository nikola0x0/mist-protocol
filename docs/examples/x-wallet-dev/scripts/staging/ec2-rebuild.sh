#!/bin/bash
# ec2-rebuild.sh - Rebuild XWallet Staging enclave on EC2 from local code
# Usage: ./ec2-rebuild.sh [--register]

set -e

# ============================================
# CONFIGURATION (STAGING)
# ============================================
INSTANCE_ID="i-0dde9264482cfc062"
INSTANCE_NAME="xwallet-staging"
REGION="ap-southeast-1"
SSH_KEY="$HOME/.ssh/xwallet-keypair-sg.pem"
REMOTE_DIR="~/nautilus-xwallet"

# Local paths
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
LOCAL_NAUTILUS_DIR="$SCRIPT_DIR/../../nautilus-xwallet"
ENV_FILE="$SCRIPT_DIR/../../backend/.env"

# Load values from .env
if [ -f "$ENV_FILE" ]; then
  TWITTER_BEARER_TOKEN=$(grep "^TWITTER_BEARER_TOKEN=" "$ENV_FILE" | cut -d'=' -f2)
  SUI_RPC_URL=$(grep "^SUI_RPC_URL=" "$ENV_FILE" | cut -d'=' -f2)
  XWALLET_REGISTRY_ID=$(grep "^XWALLET_REGISTRY_ID=" "$ENV_FILE" | cut -d'=' -f2)
else
  echo "ERROR: backend/.env not found at $ENV_FILE"
  exit 1
fi

# Secrets to pass to enclave (JSON format)
SECRETS_JSON=$(cat <<EOF
{
  "API_KEY": "$TWITTER_BEARER_TOKEN",
  "SUI_RPC_URL": "$SUI_RPC_URL",
  "USDC_TYPE": "0xa1ec7fc00a6f40db9693ad1415d0c193ad3906494428cf252621037bd7117e29::usdc::USDC",
  "WAL_TYPE": "0x8270feb7375eee355e64fdb69c50abb6b5f9393a722883c1cf45f8e26048810a::wal::WAL",
  "XWALLET_REGISTRY_ID": "$XWALLET_REGISTRY_ID"
}
EOF
)

# ============================================
# PRE-FLIGHT CHECKS
# ============================================
echo "=== XWallet Staging Enclave Rebuild ==="
echo ""

# Check SSH key
if [ ! -f "$SSH_KEY" ]; then
    echo "ERROR: SSH key not found at $SSH_KEY"
    exit 1
fi

# Check local directory exists
if [ ! -d "$LOCAL_NAUTILUS_DIR" ]; then
    echo "ERROR: Local nautilus-xwallet directory not found at $LOCAL_NAUTILUS_DIR"
    exit 1
fi

# Get current public IP (spot instance - IP changes on restart)
PUBLIC_IP=$(aws ec2 describe-instances --instance-ids "$INSTANCE_ID" --region "$REGION" \
  --query 'Reservations[0].Instances[0].PublicIpAddress' --output text 2>/dev/null)

if [ -z "$PUBLIC_IP" ] || [ "$PUBLIC_IP" == "None" ]; then
    echo "ERROR: Could not get public IP. Is the instance running?"
    echo "Try: ./ec2-start.sh first"
    exit 1
fi

echo "Instance IP: $PUBLIC_IP"

# Check if instance is reachable via SSH
echo "Checking EC2 instance connectivity..."
if ! ssh -i "$SSH_KEY" -o StrictHostKeyChecking=no -o ConnectTimeout=10 "ec2-user@$PUBLIC_IP" "echo 'SSH OK'" 2>/dev/null; then
    echo "ERROR: Cannot connect to EC2 instance at $PUBLIC_IP"
    echo "Make sure the instance is running. Try: ./ec2-start.sh --skip-register"
    exit 1
fi
echo "EC2 instance is reachable"

# ============================================
# STEP 1: SYNC CODE TO EC2
# ============================================
echo ""
echo "=== Step 1: Syncing code to EC2 ==="
echo "From: $LOCAL_NAUTILUS_DIR"
echo "To:   ec2-user@$PUBLIC_IP:$REMOTE_DIR"
echo ""

rsync -avz --progress \
    --exclude 'target' \
    --exclude 'out' \
    --exclude '.git' \
    --exclude '.DS_Store' \
    --exclude '*.eif' \
    --exclude '*.pcrs' \
    -e "ssh -i $SSH_KEY -o StrictHostKeyChecking=no" \
    "$LOCAL_NAUTILUS_DIR/" \
    "ec2-user@$PUBLIC_IP:$REMOTE_DIR/"

echo "Code synced successfully!"

# ============================================
# STEP 2: REBUILD ENCLAVE ON EC2
# ============================================
echo ""
echo "=== Step 2: Rebuilding enclave on EC2 ==="
echo "This may take several minutes..."
echo ""

ssh -i "$SSH_KEY" -o StrictHostKeyChecking=no "ec2-user@$PUBLIC_IP" << 'ENDSSH'
set -e
cd ~/nautilus-xwallet

echo ">>> Setting execute permissions..."
chmod +x *.sh 2>/dev/null || true

echo ">>> Terminating existing enclave..."
sudo nitro-cli terminate-enclave --all 2>/dev/null || true

echo ">>> Building new enclave image..."
sudo make ENCLAVE_APP=xwallet

echo ">>> Build complete!"
ENDSSH

# ============================================
# STEP 3: START ENCLAVE
# ============================================
echo ""
echo "=== Step 3: Starting enclave ==="

ENCLAVE_OUTPUT=$(ssh -i "$SSH_KEY" -o StrictHostKeyChecking=no "ec2-user@$PUBLIC_IP" \
    "sudo nitro-cli run-enclave --eif-path ~/nautilus-xwallet/out/nitro.eif --memory 2048 --cpu-count 2")

echo "$ENCLAVE_OUTPUT"

# Extract CID
ENCLAVE_CID=$(echo "$ENCLAVE_OUTPUT" | grep -o '"EnclaveCID": [0-9]*' | grep -o '[0-9]*')

if [ -z "$ENCLAVE_CID" ]; then
    echo "ERROR: Failed to get Enclave CID"
    exit 1
fi

echo ""
echo "Enclave started with CID: $ENCLAVE_CID"

# ============================================
# STEP 4: CONFIGURE ENCLAVE
# ============================================
echo ""
echo "=== Step 4: Configuring enclave (CID=$ENCLAVE_CID) ==="

# Create a temp file with secrets
SECRETS_FILE=$(mktemp)
echo "$SECRETS_JSON" > "$SECRETS_FILE"

# Copy secrets to EC2 and configure
scp -i "$SSH_KEY" -o StrictHostKeyChecking=no "$SECRETS_FILE" "ec2-user@$PUBLIC_IP:/tmp/secrets.json"
rm "$SECRETS_FILE"

ssh -i "$SSH_KEY" -o StrictHostKeyChecking=no "ec2-user@$PUBLIC_IP" << ENDSSH
set -e
CID=$ENCLAVE_CID

echo ">>> Waiting for enclave to initialize (5s)..."
sleep 5

echo ">>> Setting VSOCK permissions..."
sudo chmod 666 /dev/vsock

echo ">>> Sending secrets to enclave (CID=\$CID)..."
cat /tmp/secrets.json | /usr/local/bin/socat-vsock - VSOCK-CONNECT:\$CID:7777
rm /tmp/secrets.json

echo ">>> Killing old processes..."
pkill -f "socat-vsock.*3000" 2>/dev/null || true
pkill -f "vsock-proxy" 2>/dev/null || true
sleep 2

echo ">>> Updating vsock-proxy allowlist..."
sudo bash -c 'cat > /etc/nitro_enclaves/vsock-proxy.yaml << ALLOWLIST
allowlist:
- {address: api.twitter.com, port: 443}
- {address: api.x.com, port: 443}
- {address: kms.ap-southeast-1.amazonaws.com, port: 443}
- {address: kms-fips.ap-southeast-1.amazonaws.com, port: 443}
- {address: fullnode.testnet.sui.io, port: 443}
ALLOWLIST'

echo ">>> Starting port forwarding (CID=\$CID)..."
nohup /usr/local/bin/socat-vsock TCP4-LISTEN:3000,reuseaddr,fork VSOCK-CONNECT:\$CID:3000 > /tmp/socat-3000.log 2>&1 &

echo ">>> Starting vsock-proxy for Twitter API (port 8101)..."
nohup vsock-proxy 8101 api.twitter.com 443 --config /etc/nitro_enclaves/vsock-proxy.yaml > /tmp/vsock-proxy-twitter.log 2>&1 &

echo ">>> Starting vsock-proxy for Sui RPC (port 8102)..."
nohup vsock-proxy 8102 fullnode.testnet.sui.io 443 --config /etc/nitro_enclaves/vsock-proxy.yaml > /tmp/vsock-proxy-sui.log 2>&1 &

sleep 3

echo ">>> Testing local connection..."
curl -s http://localhost:3000/ || echo "Local test failed"

echo ""
echo ">>> Configuration complete!"
ENDSSH

# ============================================
# STEP 5: VERIFY ENCLAVE
# ============================================
echo ""
echo "=== Step 5: Verifying enclave ==="
sleep 3

RESPONSE=$(curl -s --max-time 10 http://$PUBLIC_IP:3000/ 2>/dev/null)
if [ "$RESPONSE" == "Pong!" ]; then
    echo "Enclave is running and responding!"
else
    echo "WARNING: Enclave not responding yet"
    echo "Response: $RESPONSE"
    echo ""
    echo "Debug commands:"
    echo "  ssh -i $SSH_KEY ec2-user@$PUBLIC_IP"
    echo "  sudo nitro-cli describe-enclaves"
    echo "  curl http://localhost:3000/"
    echo "  cat /tmp/socat-3000.log"
fi

# ============================================
# STEP 6: REGISTER (OPTIONAL)
# ============================================
if [ "$1" == "--register" ]; then
    echo ""
    echo "=== Step 6: Registering enclave on-chain ==="
    "$SCRIPT_DIR/ec2-start.sh" --register
else
    echo ""
    echo "============================================"
    echo "=== STAGING REBUILD COMPLETE ==="
    echo "============================================"
    echo ""
    echo "Instance:    $INSTANCE_NAME"
    echo "IP:          $PUBLIC_IP"
    echo "API:         http://$PUBLIC_IP:3000/"
    echo "Enclave CID: $ENCLAVE_CID"
    echo ""
    echo "NOTE: Enclave attestation (PCRs) has changed!"
    echo "To register the new enclave on-chain, run:"
    echo "  ./ec2-rebuild.sh --register"
    echo "  OR"
    echo "  ./ec2-start.sh --register"
fi
