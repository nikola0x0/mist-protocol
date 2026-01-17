#!/bin/bash
# ec2-stop.sh - Stop XWallet Staging EC2 instance
# Usage: ./ec2-stop.sh

INSTANCE_ID="i-0dde9264482cfc062"
INSTANCE_NAME="xwallet-staging"
REGION="ap-southeast-1"

echo "=== Stopping $INSTANCE_NAME (Staging) ==="

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
echo "NOTE: This is a Spot instance - IP will change on next start"
echo "Run ./ec2-start.sh to start again"
