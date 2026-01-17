#!/bin/bash
# ec2-start.sh - Start XWallet EC2 instance and register enclave
# Usage: ./ec2-start.sh [--register]

# ============================================
# CONFIGURATION
# ============================================
INSTANCE_ID="i-0b5c5408896a6e0d8"
INSTANCE_NAME="xwallet-prod-111101"
REGION="ap-southeast-1"
ELASTIC_IP="18.136.64.150"
SSH_KEY="$HOME/.ssh/xwallet-keypair-sg.pem"

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
RAILWAY_SERVICE=$(grep "^RAILWAY_SERVICE=" "$ENV_FILE" | cut -d'=' -f2)
# ============================================

echo "=== Starting $INSTANCE_NAME ==="

# Check current state
STATE=$(aws ec2 describe-instances --instance-ids "$INSTANCE_ID" --region "$REGION" \
  --query 'Reservations[0].Instances[0].State.Name' --output text 2>/dev/null)

if [ "$STATE" == "running" ]; then
  echo "Instance is already running"
  echo "   IP: $ELASTIC_IP"
  echo "   Testing enclave..."
  RESPONSE=$(curl -s --max-time 5 http://$ELASTIC_IP:3000/ 2>/dev/null)
  if [ "$RESPONSE" == "Pong!" ]; then
    echo "   Enclave: Running"
    if [ "$1" != "--register" ]; then
      echo ""
      echo "To re-register enclave, run: ./ec2-start.sh --register"
      exit 0
    fi
  else
    echo "   Enclave: Not responding - will restart"
  fi
fi

# Start instance if not running
if [ "$STATE" != "running" ]; then
  echo "Starting instance..."
  aws ec2 start-instances --instance-ids "$INSTANCE_ID" --region "$REGION" > /dev/null

  echo "Waiting for instance to start..."
  aws ec2 wait instance-running --instance-ids "$INSTANCE_ID" --region "$REGION"
  echo "Instance running!"

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
RESPONSE=$(curl -s --max-time 5 http://$ELASTIC_IP:3000/ 2>/dev/null)
if [ "$RESPONSE" != "Pong!" ]; then
  echo "Enclave not running. Starting via SSH..."

  # Check SSH key exists
  if [ ! -f "$SSH_KEY" ]; then
    echo "ERROR: SSH key not found at $SSH_KEY"
    echo "Please update SSH_KEY path in script"
    exit 1
  fi

  # Run startup script on EC2
  ssh -i "$SSH_KEY" -o StrictHostKeyChecking=no -o ConnectTimeout=30 ec2-user@$ELASTIC_IP \
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
RESPONSE=$(curl -s --max-time 10 http://$ELASTIC_IP:3000/ 2>/dev/null)
if [ "$RESPONSE" != "Pong!" ]; then
  echo "ERROR: Enclave not responding"
  echo "Try manually: ssh -i $SSH_KEY ec2-user@$ELASTIC_IP"
  exit 1
fi
echo "Enclave ready!"

# ============================================
# REGISTER ENCLAVE ON-CHAIN
# ============================================
echo ""
echo "=== Registering Enclave On-Chain ==="

# Fetch attestation from enclave
echo "Fetching attestation..."
ATTESTATION_HEX=$(curl -s http://$ELASTIC_IP:3000/get_attestation | jq -r '.attestation')

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

# Extract new ENCLAVE_ID from transaction result
NEW_ENCLAVE_ID=$(echo "$RESULT" | grep -A5 "Created Objects" | grep -o '0x[a-f0-9]*' | head -1)

echo ""
echo "============================================"
echo "=== ENCLAVE STARTED AND REGISTERED ==="
echo "============================================"
echo ""
echo "Instance:     $INSTANCE_NAME"
echo "IP:           $ELASTIC_IP"
echo "API:          http://$ELASTIC_IP:3000/"
echo ""

if [ -n "$NEW_ENCLAVE_ID" ]; then
  echo "NEW ENCLAVE_ID: $NEW_ENCLAVE_ID"

  # Update backend/.env with new ENCLAVE_ID
  if [ -f "$ENV_FILE" ]; then
    # Replace ENCLAVE_ID line in .env file
    sed -i '' "s/^ENCLAVE_ID=.*/ENCLAVE_ID=$NEW_ENCLAVE_ID/" "$ENV_FILE"
    echo ""
    echo "Updated backend/.env (local)"
  else
    echo ""
    echo "WARNING: backend/.env not found at $ENV_FILE"
  fi

  # Update Railway environment variable
  echo ""
  echo "=== Updating Railway ==="
  if command -v railway &> /dev/null; then
    echo "Setting ENCLAVE_ID on Railway..."
    RAILWAY_TOKEN="$RAILWAY_TOKEN" railway variables --service "$RAILWAY_SERVICE" --set "ENCLAVE_ID=$NEW_ENCLAVE_ID" --skip-deploys

    echo ""
    echo "Redeploying Railway backend service..."
    RAILWAY_TOKEN="$RAILWAY_TOKEN" railway redeploy --service "$RAILWAY_SERVICE" -y

    echo ""
    echo "Railway updated and redeployed!"
  else
    echo "WARNING: Railway CLI not installed. Update Railway manually:"
    echo "  railway variables --service backend --set \"ENCLAVE_ID=$NEW_ENCLAVE_ID\""
    echo "  railway redeploy --service backend -y"
  fi

  echo ""
  echo "=== DONE ==="
else
  echo "Transaction successful. Check output for ENCLAVE_ID:"
  echo "$RESULT" | grep -A10 "Created Objects"
fi
