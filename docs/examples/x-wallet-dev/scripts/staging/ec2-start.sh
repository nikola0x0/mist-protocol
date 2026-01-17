#!/bin/bash
# ec2-start.sh - Start XWallet Staging EC2 instance and register enclave
# Usage: ./ec2-start.sh [--register] [--skip-register] [--skip-deploy]
#
# Options:
#   --register       Force re-register enclave even if running
#   --skip-register  Skip enclave registration (just start)
#   --skip-deploy    Skip Railway deployment

# ============================================
# CONFIGURATION (STAGING)
# ============================================
INSTANCE_ID="i-0dde9264482cfc062"
INSTANCE_NAME="xwallet-staging"
REGION="ap-southeast-1"
SSH_KEY="$HOME/.ssh/xwallet-keypair-sg.pem"

# Railway staging config
RAILWAY_SERVICE_STAGING="backend"

# Load config from backend/.env
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ENV_FILE="$SCRIPT_DIR/../../backend/.env"

if [ ! -f "$ENV_FILE" ]; then
  echo "ERROR: backend/.env not found at $ENV_FILE"
  exit 1
fi

# Read values from .env file
ENCLAVE_PACKAGE_ID=$(grep "^ENCLAVE_PACKAGE_ID=" "$ENV_FILE" | cut -d'=' -f2)
XWALLET_PACKAGE_ID=$(grep "^XWALLET_PACKAGE_ID=" "$ENV_FILE" | cut -d'=' -f2)
ENCLAVE_CONFIG_ID=$(grep "^ENCLAVE_CONFIG_ID=" "$ENV_FILE" | cut -d'=' -f2)
RAILWAY_TOKEN=$(grep "^RAILWAY_TOKEN=" "$ENV_FILE" | cut -d'=' -f2)
TWITTER_BEARER_TOKEN=$(grep "^TWITTER_BEARER_TOKEN=" "$ENV_FILE" | cut -d'=' -f2)
SUI_RPC_URL=$(grep "^SUI_RPC_URL=" "$ENV_FILE" | cut -d'=' -f2)
XWALLET_REGISTRY_ID=$(grep "^XWALLET_REGISTRY_ID=" "$ENV_FILE" | cut -d'=' -f2)

if [ -z "$TWITTER_BEARER_TOKEN" ]; then
  echo "ERROR: TWITTER_BEARER_TOKEN not found in backend/.env"
  exit 1
fi

# Secrets to pass to enclave (JSON format) - matches production
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

echo "=== Starting $INSTANCE_NAME (Staging) ==="

# Check current state
STATE=$(aws ec2 describe-instances --instance-ids "$INSTANCE_ID" --region "$REGION" \
  --query 'Reservations[0].Instances[0].State.Name' --output text 2>/dev/null)

# Get current public IP (spot instance - IP changes on restart)
PUBLIC_IP=$(aws ec2 describe-instances --instance-ids "$INSTANCE_ID" --region "$REGION" \
  --query 'Reservations[0].Instances[0].PublicIpAddress' --output text 2>/dev/null)

if [ "$STATE" == "running" ]; then
  echo "Instance is already running"
  echo "   IP: $PUBLIC_IP"
  echo "   Testing enclave..."
  RESPONSE=$(curl -s --max-time 5 http://$PUBLIC_IP:3000/ 2>/dev/null)
  if [ "$RESPONSE" == "Pong!" ]; then
    echo "   Enclave: Running ✓"
    if [ "$1" != "--register" ]; then
      echo ""
      echo "============================================"
      echo "Staging enclave is already running!"
      echo "============================================"
      echo "IP:  $PUBLIC_IP"
      echo "API: http://$PUBLIC_IP:3000/"
      echo ""
      echo "To re-register enclave: ./ec2-start.sh --register"
      exit 0
    fi
  else
    echo "   Enclave: Not responding - will restart"
  fi
fi

# Start instance if not running
if [ "$STATE" != "running" ]; then
  # Check if this is a spot instance with disabled request (happens after stop)
  SPOT_REQUEST_ID=$(aws ec2 describe-instances --instance-ids "$INSTANCE_ID" --region "$REGION" \
    --query 'Reservations[0].Instances[0].SpotInstanceRequestId' --output text 2>/dev/null)

  if [ -n "$SPOT_REQUEST_ID" ] && [ "$SPOT_REQUEST_ID" != "None" ]; then
    SPOT_STATE=$(aws ec2 describe-spot-instance-requests --spot-instance-request-ids "$SPOT_REQUEST_ID" \
      --region "$REGION" --query 'SpotInstanceRequests[0].State' --output text 2>/dev/null)

    if [ "$SPOT_STATE" == "disabled" ]; then
      echo "Spot request is disabled (normal after stop). Starting will re-enable it..."
    fi
  fi

  echo "Starting instance..."
  START_RESULT=$(aws ec2 start-instances --instance-ids "$INSTANCE_ID" --region "$REGION" 2>&1)

  if [ $? -ne 0 ]; then
    # Check for spot-specific error
    if echo "$START_RESULT" | grep -q "IncorrectSpotRequestState"; then
      echo ""
      echo "ERROR: Spot instance cannot be started."
      echo "This can happen if the spot request was cancelled or there's a capacity issue."
      echo ""
      echo "Checking spot request status..."
      aws ec2 describe-spot-instance-requests --spot-instance-request-ids "$SPOT_REQUEST_ID" \
        --region "$REGION" --query 'SpotInstanceRequests[0].{State:State,Status:Status.Code,Message:Status.Message}' \
        --output table 2>/dev/null
      echo ""
      echo "You may need to:"
      echo "  1. Wait a few minutes and try again"
      echo "  2. Or terminate this instance and create a new spot request"
      exit 1
    else
      echo "ERROR: Failed to start instance"
      echo "$START_RESULT"
      exit 1
    fi
  fi

  echo "Waiting for instance to start..."
  aws ec2 wait instance-running --instance-ids "$INSTANCE_ID" --region "$REGION"
  echo "Instance running!"

  # Get new public IP after start
  PUBLIC_IP=$(aws ec2 describe-instances --instance-ids "$INSTANCE_ID" --region "$REGION" \
    --query 'Reservations[0].Instances[0].PublicIpAddress' --output text 2>/dev/null)
  echo ""
  echo "============================================"
  echo "NEW PUBLIC IP: $PUBLIC_IP"
  echo "============================================"
  echo "(IP changed because this is a spot instance)"
  echo ""

  # Wait for SSH to be ready
  echo "Waiting for SSH to be ready (30s)..."
  sleep 30
fi

# ============================================
# START ENCLAVE
# ============================================
echo ""
echo "=== Starting Enclave ==="

# Check if enclave is already running
RESPONSE=$(curl -s --max-time 5 http://$PUBLIC_IP:3000/ 2>/dev/null)
if [ "$RESPONSE" != "Pong!" ]; then
  echo "Enclave not running. Starting via SSH..."

  # Check SSH key exists
  if [ ! -f "$SSH_KEY" ]; then
    echo "ERROR: SSH key not found at $SSH_KEY"
    echo "Please update SSH_KEY path in script"
    exit 1
  fi

  # Always copy secrets to EC2 (needed for enclave startup)
  echo "Copying secrets to EC2..."
  SECRETS_FILE=$(mktemp)
  echo "$SECRETS_JSON" > "$SECRETS_FILE"
  scp -i "$SSH_KEY" -o StrictHostKeyChecking=no "$SECRETS_FILE" "ec2-user@$PUBLIC_IP:/tmp/enclave-secrets.json"
  rm "$SECRETS_FILE"

  # Check if startup script exists on EC2, create if not
  ssh -i "$SSH_KEY" -o StrictHostKeyChecking=no -o ConnectTimeout=30 ec2-user@$PUBLIC_IP \
    "test -f /home/ec2-user/enclave-startup.sh" 2>/dev/null

  if [ $? -ne 0 ]; then
    echo "Creating enclave-startup.sh on EC2..."

    ssh -i "$SSH_KEY" -o StrictHostKeyChecking=no ec2-user@$PUBLIC_IP << 'EOFSTARTUP'
cat > /home/ec2-user/enclave-startup.sh << 'EOF'
#!/bin/bash
set -e

echo "=== Stopping any existing enclave ==="
nitro-cli terminate-enclave --all 2>/dev/null || true

echo "=== Starting enclave ==="
RESULT=$(nitro-cli run-enclave --eif-path /home/ec2-user/nautilus-xwallet/out/nitro.eif --memory 2048 --cpu-count 2)
echo "$RESULT"

CID=$(echo "$RESULT" | grep -o '"EnclaveCID": [0-9]*' | grep -o '[0-9]*')
if [ -z "$CID" ]; then
    echo "ERROR: Failed to get enclave CID"
    exit 1
fi
echo "Enclave CID: $CID"

echo "=== Waiting for enclave to initialize (5s) ==="
sleep 5

chmod 666 /dev/vsock

echo "=== Sending secrets to enclave ==="
if [ -f /tmp/enclave-secrets.json ]; then
    cat /tmp/enclave-secrets.json | /usr/local/bin/socat-vsock - VSOCK-CONNECT:$CID:7777
    rm /tmp/enclave-secrets.json
    echo "Secrets sent"
else
    echo "WARNING: /tmp/enclave-secrets.json not found"
fi

echo "=== Killing old processes ==="
pkill -f "socat-vsock.*3000" 2>/dev/null || true
pkill -f "vsock-proxy" 2>/dev/null || true
sleep 2

echo "=== Updating vsock-proxy allowlist ==="
sudo bash -c 'cat > /etc/nitro_enclaves/vsock-proxy.yaml << ALLOWLIST
allowlist:
- {address: api.twitter.com, port: 443}
- {address: api.x.com, port: 443}
- {address: kms.ap-southeast-1.amazonaws.com, port: 443}
- {address: kms-fips.ap-southeast-1.amazonaws.com, port: 443}
- {address: fullnode.testnet.sui.io, port: 443}
ALLOWLIST'

echo "=== Starting port forwarding ==="
nohup /usr/local/bin/socat-vsock TCP4-LISTEN:3000,reuseaddr,fork VSOCK-CONNECT:$CID:3000 > /tmp/socat-3000.log 2>&1 &

echo "=== Starting vsock-proxy for Twitter API (port 8101) ==="
nohup vsock-proxy 8101 api.twitter.com 443 --config /etc/nitro_enclaves/vsock-proxy.yaml > /tmp/vsock-proxy-twitter.log 2>&1 &

echo "=== Starting vsock-proxy for Sui RPC (port 8102) ==="
nohup vsock-proxy 8102 fullnode.testnet.sui.io 443 --config /etc/nitro_enclaves/vsock-proxy.yaml > /tmp/vsock-proxy-sui.log 2>&1 &

for i in {1..20}; do
    RESPONSE=$(curl -s --max-time 3 http://localhost:3000/ 2>/dev/null)
    if [ "$RESPONSE" == "Pong!" ]; then
        echo "Enclave ready!"
        exit 0
    fi
    sleep 2
done
echo "ERROR: Enclave not responding"
exit 1
EOF
chmod +x /home/ec2-user/enclave-startup.sh
EOFSTARTUP
  fi

  # Run startup script on EC2
  ssh -i "$SSH_KEY" -o StrictHostKeyChecking=no -o ConnectTimeout=30 ec2-user@$PUBLIC_IP \
    "sudo /home/ec2-user/enclave-startup.sh" 2>&1

  if [ $? -ne 0 ]; then
    echo "ERROR: Failed to start enclave via SSH"
    exit 1
  fi

  # Wait and verify
  echo "Waiting for enclave to be ready..."
  sleep 5
fi

# Verify enclave is running
RESPONSE=$(curl -s --max-time 10 http://$PUBLIC_IP:3000/ 2>/dev/null)
if [ "$RESPONSE" != "Pong!" ]; then
  echo "ERROR: Enclave not responding"
  echo "Try manually: ssh -i $SSH_KEY ec2-user@$PUBLIC_IP"
  exit 1
fi
echo "Enclave ready!"

# Skip registration if requested
if [ "$1" == "--skip-register" ]; then
  echo ""
  echo "============================================"
  echo "=== STAGING ENCLAVE STARTED ==="
  echo "============================================"
  echo "Instance: $INSTANCE_NAME"
  echo "IP:       $PUBLIC_IP"
  echo "API:      http://$PUBLIC_IP:3000/"
  echo ""
  echo "Skipped registration (use --register to register)"
  exit 0
fi

# ============================================
# REGISTER ENCLAVE ON-CHAIN
# ============================================
echo ""
echo "=== Registering Enclave On-Chain ==="

# Fetch attestation from enclave
echo "Fetching attestation..."
ATTESTATION_HEX=$(curl -s http://$PUBLIC_IP:3000/get_attestation | jq -r '.attestation')

if [ -z "$ATTESTATION_HEX" ] || [ "$ATTESTATION_HEX" == "null" ]; then
  echo "ERROR: Failed to get attestation from enclave"
  exit 1
fi
echo "Got attestation (length: ${#ATTESTATION_HEX})"

# Convert hex to Sui vector format
echo "Converting attestation to Sui format..."
ATTESTATION_ARRAY=$(python3 - <<EOF
hex_string = "$ATTESTATION_HEX"
byte_values = [str(int(hex_string[i:i+2], 16)) for i in range(0, len(hex_string), 2)]
rust_array = [f"{byte}u8" for byte in byte_values]
print(f"[{', '.join(rust_array)}]")
EOF
)

# Register enclave on Sui
echo "Registering enclave on Sui testnet..."
echo ""

RESULT=$(sui client ptb \
  --assign v "vector$ATTESTATION_ARRAY" \
  --move-call "0x2::nitro_attestation::load_nitro_attestation" v @0x6 \
  --assign result \
  --move-call "${ENCLAVE_PACKAGE_ID}::enclave::register_enclave<${XWALLET_PACKAGE_ID}::core::XWALLET>" @${ENCLAVE_CONFIG_ID} result \
  --gas-budget 100000000 2>&1)

if [ $? -ne 0 ]; then
  echo "ERROR: Failed to register enclave"
  echo "$RESULT"
  exit 1
fi

# Extract transaction digest
TX_DIGEST=$(echo "$RESULT" | grep "Transaction Digest:" | awk '{print $3}')

# Extract new ENCLAVE_ID using jq (more reliable)
if command -v jq &> /dev/null; then
  NEW_ENCLAVE_ID=$(sui client tx-block "$TX_DIGEST" --json 2>/dev/null | \
    jq -r '.objectChanges[] | select(.type == "created") | select(.objectType | contains("enclave::Enclave")) | .objectId' 2>/dev/null)
fi

# Fallback to grep if jq method fails
if [ -z "$NEW_ENCLAVE_ID" ]; then
  NEW_ENCLAVE_ID=$(echo "$RESULT" | grep -A5 "Created Objects" | grep -o '0x[a-f0-9]*' | head -1)
fi

echo ""
echo "============================================"
echo "=== STAGING ENCLAVE STARTED AND REGISTERED ==="
echo "============================================"
echo ""
echo "Instance:     $INSTANCE_NAME"
echo "IP:           $PUBLIC_IP"
echo "API:          http://$PUBLIC_IP:3000/"
echo "TX:           $TX_DIGEST"
echo ""

if [ -n "$NEW_ENCLAVE_ID" ]; then
  echo "============================================"
  echo "NEW ENCLAVE_ID: $NEW_ENCLAVE_ID"
  echo "============================================"

  # ============================================
  # UPDATE backend/.env (local)
  # ============================================
  if [ -f "$ENV_FILE" ]; then
    echo ""
    echo "=== Updating backend/.env (local) ==="

    # Update ENCLAVE_ID
    if grep -q "^ENCLAVE_ID=" "$ENV_FILE"; then
      sed -i '' "s|^ENCLAVE_ID=.*|ENCLAVE_ID=$NEW_ENCLAVE_ID|" "$ENV_FILE"
    else
      echo "ENCLAVE_ID=$NEW_ENCLAVE_ID" >> "$ENV_FILE"
    fi

    # Update ENCLAVE_URL (IP changes on spot instance)
    if grep -q "^ENCLAVE_URL=" "$ENV_FILE"; then
      sed -i '' "s|^ENCLAVE_URL=.*|ENCLAVE_URL=http://$PUBLIC_IP:3000|" "$ENV_FILE"
    else
      echo "ENCLAVE_URL=http://$PUBLIC_IP:3000" >> "$ENV_FILE"
    fi

    echo "Updated backend/.env:"
    echo "  ENCLAVE_ID=$NEW_ENCLAVE_ID"
    echo "  ENCLAVE_URL=http://$PUBLIC_IP:3000"
  fi

  # Skip deploy if requested
  if [ "$1" == "--skip-deploy" ] || [ "$2" == "--skip-deploy" ]; then
    echo ""
    echo "Skipped Railway deployment (--skip-deploy)"
    exit 0
  fi

  # ============================================
  # UPDATE RAILWAY STAGING
  # ============================================
  echo ""
  echo "=== Updating Railway ==="

  if command -v railway &> /dev/null; then
    echo "Setting ENCLAVE_ID and ENCLAVE_URL on Railway staging..."

    # Update both ENCLAVE_ID and ENCLAVE_URL (IP changes on spot instance)
    RAILWAY_TOKEN="$RAILWAY_TOKEN" railway variables --service "$RAILWAY_SERVICE_STAGING" \
      --set "ENCLAVE_ID=$NEW_ENCLAVE_ID" \
      --set "ENCLAVE_URL=http://$PUBLIC_IP:3000" \
      --skip-deploys 2>/dev/null

    if [ $? -eq 0 ]; then
      echo ""
      echo "Redeploying Railway staging service..."
      RAILWAY_TOKEN="$RAILWAY_TOKEN" railway redeploy --service "$RAILWAY_SERVICE_STAGING" -y 2>/dev/null

      if [ $? -eq 0 ]; then
        echo ""
        echo "Railway staging updated and redeployed!"
      else
        echo ""
        echo "WARNING: Failed to redeploy. You may need to redeploy manually."
      fi
    else
      echo ""
      echo "WARNING: Failed to update Railway variables."
      echo "You may need to run manually:"
      echo "  railway variables --service $RAILWAY_SERVICE_STAGING --set \"ENCLAVE_ID=$NEW_ENCLAVE_ID\" --set \"ENCLAVE_URL=http://$PUBLIC_IP:3000\""
      echo "  railway redeploy --service $RAILWAY_SERVICE_STAGING -y"
    fi
  else
    echo "WARNING: Railway CLI not installed."
    echo ""
    echo "Install Railway CLI:"
    echo "  npm install -g @railway/cli"
    echo ""
    echo "Then run manually:"
    echo "  railway variables --service $RAILWAY_SERVICE_STAGING --set \"ENCLAVE_ID=$NEW_ENCLAVE_ID\" --set \"ENCLAVE_URL=http://$PUBLIC_IP:3000\""
    echo "  railway redeploy --service $RAILWAY_SERVICE_STAGING -y"
  fi

  echo ""
  echo "=== DONE ==="
else
  echo "Transaction successful but could not extract ENCLAVE_ID."
  echo "Check transaction on explorer:"
  echo "  https://suiscan.xyz/testnet/tx/$TX_DIGEST"
fi
