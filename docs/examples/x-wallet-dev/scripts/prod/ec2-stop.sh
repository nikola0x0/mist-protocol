#!/bin/bash
# ec2-stop.sh - Stop XWallet EC2 instance
# Usage: ./ec2-stop.sh

INSTANCE_ID="i-0b5c5408896a6e0d8"
INSTANCE_NAME="xwallet-prod-111101"
REGION="ap-southeast-1"

echo "=== Stopping $INSTANCE_NAME ==="

# Check current state
STATE=$(aws ec2 describe-instances --instance-ids "$INSTANCE_ID" --region "$REGION" \
  --query 'Reservations[0].Instances[0].State.Name' --output text 2>/dev/null)

if [ "$STATE" == "stopped" ]; then
  echo "Instance is already stopped"
  exit 0
fi

# Stop instance
echo "Stopping instance..."
aws ec2 stop-instances --instance-ids "$INSTANCE_ID" --region "$REGION" > /dev/null

# Wait for stopped
echo "Waiting for instance to stop..."
aws ec2 wait instance-stopped --instance-ids "$INSTANCE_ID" --region "$REGION"

echo ""
echo "Instance stopped!"
echo "Run ./ec2-start.sh to start again"
