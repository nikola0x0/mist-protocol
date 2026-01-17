#!/bin/bash
# ec2-status.sh - Check XWallet EC2 instance status
# Usage: ./ec2-status.sh

INSTANCE_ID="i-0b5c5408896a6e0d8"
INSTANCE_NAME="xwallet-prod-111101"
REGION="ap-southeast-1"
ELASTIC_IP="18.136.64.150"

echo "=== XWallet Instance Status ==="
echo ""

# Get instance info
INFO=$(aws ec2 describe-instances --instance-ids "$INSTANCE_ID" --region "$REGION" \
  --query 'Reservations[0].Instances[0].[State.Name,InstanceType,PublicIpAddress]' \
  --output text 2>/dev/null)

STATE=$(echo "$INFO" | awk '{print $1}')
TYPE=$(echo "$INFO" | awk '{print $2}')
IP=$(echo "$INFO" | awk '{print $3}')

echo "Name:     $INSTANCE_NAME"
echo "Type:     $TYPE"
echo "State:    $STATE"
echo "IP:       $ELASTIC_IP (Elastic IP)"
echo ""

if [ "$STATE" == "running" ]; then
  echo "=== Enclave Status ==="
  RESPONSE=$(curl -s --max-time 5 http://$ELASTIC_IP:3000/ 2>/dev/null)
  if [ "$RESPONSE" == "Pong!" ]; then
    echo "Enclave:  Running"
    echo "API:      http://$ELASTIC_IP:3000/"
  else
    echo "Enclave:  Not responding"
    echo "          (May still be starting, wait 1-2 minutes)"
  fi
else
  echo "Instance is $STATE - enclave not available"
fi
