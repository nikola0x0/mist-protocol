#!/bin/bash
# ec2-status.sh - Check XWallet Staging EC2 instance status
# Usage: ./ec2-status.sh

INSTANCE_ID="i-0dde9264482cfc062"
INSTANCE_NAME="xwallet-staging"
REGION="ap-southeast-1"

echo "=== XWallet Staging Instance Status ==="
echo ""

# Get instance info
INFO=$(aws ec2 describe-instances --instance-ids "$INSTANCE_ID" --region "$REGION" \
  --query 'Reservations[0].Instances[0].[State.Name,InstanceType,PublicIpAddress,InstanceLifecycle]' \
  --output text 2>/dev/null)

STATE=$(echo "$INFO" | awk '{print $1}')
TYPE=$(echo "$INFO" | awk '{print $2}')
IP=$(echo "$INFO" | awk '{print $3}')
LIFECYCLE=$(echo "$INFO" | awk '{print $4}')

echo "Name:     $INSTANCE_NAME"
echo "Type:     $TYPE"
echo "State:    $STATE"
echo "IP:       $IP (dynamic - changes on restart)"
if [ "$LIFECYCLE" == "spot" ]; then
  echo "Lifecycle: Spot Instance"
fi
echo ""

if [ "$STATE" == "running" ]; then
  echo "=== Enclave Status ==="
  RESPONSE=$(curl -s --max-time 5 http://$IP:3000/ 2>/dev/null)
  if [ "$RESPONSE" == "Pong!" ]; then
    echo "Enclave:  Running"
    echo "API:      http://$IP:3000/"
  else
    echo "Enclave:  Not responding"
    echo "          (May still be starting, wait 1-2 minutes)"
  fi
else
  echo "Instance is $STATE - enclave not available"
fi
