#!/bin/bash
# Mist Protocol - Complete AWS Setup Script
# Creates all AWS resources needed for enclave deployment
#
# Usage:
#   export AWS_PROFILE="your-profile"
#   export REGION="ap-southeast-1"
#   export KEY_PAIR="mist-enclave"
#   export BACKEND_PRIVATE_KEY="suiprivkey1..."
#   ./setup-aws.sh

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

log() { echo -e "${GREEN}[+]${NC} $1"; }
warn() { echo -e "${YELLOW}[!]${NC} $1"; }
error() { echo -e "${RED}[x]${NC} $1"; exit 1; }

# Validate required variables
REGION="${REGION:-ap-southeast-1}"
AWS_PROFILE="${AWS_PROFILE:-default}"
KEY_PAIR="${KEY_PAIR:-mist-enclave}"
SECRET_NAME="${SECRET_NAME:-mist-backend-key}"

if [ -z "$BACKEND_PRIVATE_KEY" ]; then
    error "BACKEND_PRIVATE_KEY is required. Export it before running this script."
fi

log "Starting Mist Protocol AWS Setup"
log "Region: $REGION"
log "Profile: $AWS_PROFILE"
log "Key Pair: $KEY_PAIR"

# Check AWS CLI
if ! command -v aws &> /dev/null; then
    error "AWS CLI not installed"
fi

# Verify AWS credentials
log "Verifying AWS credentials..."
aws sts get-caller-identity --profile $AWS_PROFILE > /dev/null || error "AWS credentials not valid"

#######################
# 1. Create Secret
#######################
log "Creating secret in AWS Secrets Manager..."
SECRET_ARN=$(aws secretsmanager create-secret \
    --name $SECRET_NAME \
    --secret-string "$BACKEND_PRIVATE_KEY" \
    --region $REGION \
    --profile $AWS_PROFILE \
    --query 'ARN' --output text 2>/dev/null) || \
SECRET_ARN=$(aws secretsmanager describe-secret \
    --secret-id $SECRET_NAME \
    --region $REGION \
    --profile $AWS_PROFILE \
    --query 'ARN' --output text)

log "Secret ARN: $SECRET_ARN"

#######################
# 2. Create IAM Role
#######################
log "Creating IAM role..."

cat > /tmp/trust-policy.json << 'EOF'
{
  "Version": "2012-10-17",
  "Statement": [
    {
      "Effect": "Allow",
      "Principal": {
        "Service": "ec2.amazonaws.com"
      },
      "Action": "sts:AssumeRole"
    }
  ]
}
EOF

aws iam create-role \
    --role-name mist-enclave-role \
    --assume-role-policy-document file:///tmp/trust-policy.json \
    --profile $AWS_PROFILE 2>/dev/null || warn "Role may already exist"

aws iam attach-role-policy \
    --role-name mist-enclave-role \
    --policy-arn arn:aws:iam::aws:policy/SecretsManagerReadWrite \
    --profile $AWS_PROFILE 2>/dev/null || warn "Policy may already be attached"

#######################
# 3. Create Instance Profile
#######################
log "Creating instance profile..."

aws iam create-instance-profile \
    --instance-profile-name mist-enclave-profile \
    --profile $AWS_PROFILE 2>/dev/null || warn "Profile may already exist"

aws iam add-role-to-instance-profile \
    --instance-profile-name mist-enclave-profile \
    --role-name mist-enclave-role \
    --profile $AWS_PROFILE 2>/dev/null || warn "Role may already be added"

# Wait for profile to propagate
sleep 5

#######################
# 4. Create Key Pair
#######################
log "Checking key pair..."

if ! aws ec2 describe-key-pairs --key-names $KEY_PAIR --region $REGION --profile $AWS_PROFILE &>/dev/null; then
    log "Creating new key pair..."
    aws ec2 create-key-pair \
        --key-name $KEY_PAIR \
        --query 'KeyMaterial' \
        --output text \
        --region $REGION \
        --profile $AWS_PROFILE > ~/.ssh/$KEY_PAIR.pem
    chmod 400 ~/.ssh/$KEY_PAIR.pem
    log "Key saved to ~/.ssh/$KEY_PAIR.pem"
else
    warn "Key pair already exists"
fi

#######################
# 5. Create Security Group
#######################
log "Creating security group..."

SG_ID=$(aws ec2 create-security-group \
    --group-name mist-enclave-sg \
    --description "Mist Protocol Enclave" \
    --region $REGION \
    --profile $AWS_PROFILE \
    --query 'GroupId' --output text 2>/dev/null) || \
SG_ID=$(aws ec2 describe-security-groups \
    --group-names mist-enclave-sg \
    --region $REGION \
    --profile $AWS_PROFILE \
    --query 'SecurityGroups[0].GroupId' --output text)

log "Security Group: $SG_ID"

# Add rules (ignore errors if already exist)
aws ec2 authorize-security-group-ingress --group-id $SG_ID --protocol tcp --port 22 --cidr 0.0.0.0/0 --region $REGION --profile $AWS_PROFILE 2>/dev/null || true
aws ec2 authorize-security-group-ingress --group-id $SG_ID --protocol tcp --port 443 --cidr 0.0.0.0/0 --region $REGION --profile $AWS_PROFILE 2>/dev/null || true
aws ec2 authorize-security-group-ingress --group-id $SG_ID --protocol tcp --port 3000 --cidr 0.0.0.0/0 --region $REGION --profile $AWS_PROFILE 2>/dev/null || true

#######################
# 6. Get AMI
#######################
log "Getting latest Amazon Linux 2023 AMI..."

AMI_ID=$(aws ssm get-parameters \
    --names /aws/service/ami-amazon-linux-latest/al2023-ami-kernel-default-x86_64 \
    --region $REGION \
    --profile $AWS_PROFILE \
    --query 'Parameters[0].Value' --output text)

log "AMI: $AMI_ID"

#######################
# 7. Launch EC2
#######################
log "Launching EC2 instance (c5.xlarge with Nitro Enclave)..."

INSTANCE_ID=$(aws ec2 run-instances \
    --image-id $AMI_ID \
    --instance-type c5.xlarge \
    --key-name $KEY_PAIR \
    --security-group-ids $SG_ID \
    --enclave-options 'Enabled=true' \
    --block-device-mappings '[{"DeviceName":"/dev/xvda","Ebs":{"VolumeSize":30,"VolumeType":"gp3"}}]' \
    --tag-specifications "ResourceType=instance,Tags=[{Key=Name,Value=mist-enclave}]" \
    --region $REGION \
    --profile $AWS_PROFILE \
    --query 'Instances[0].InstanceId' --output text)

log "Instance ID: $INSTANCE_ID"

log "Waiting for instance to run..."
aws ec2 wait instance-running --instance-ids $INSTANCE_ID --region $REGION --profile $AWS_PROFILE

#######################
# 8. Attach IAM Profile
#######################
log "Attaching IAM instance profile..."

aws ec2 associate-iam-instance-profile \
    --instance-id $INSTANCE_ID \
    --iam-instance-profile Name=mist-enclave-profile \
    --region $REGION \
    --profile $AWS_PROFILE

#######################
# 9. Get Public IP
#######################
sleep 5
PUBLIC_IP=$(aws ec2 describe-instances \
    --instance-ids $INSTANCE_ID \
    --region $REGION \
    --profile $AWS_PROFILE \
    --query 'Reservations[0].Instances[0].PublicIpAddress' --output text)

#######################
# Done!
#######################
echo ""
echo -e "${GREEN}========================================${NC}"
echo -e "${GREEN}  AWS Setup Complete!${NC}"
echo -e "${GREEN}========================================${NC}"
echo ""
echo "Instance ID:  $INSTANCE_ID"
echo "Public IP:    $PUBLIC_IP"
echo "Secret ARN:   $SECRET_ARN"
echo ""
echo "Next steps:"
echo ""
echo "1. Wait 1-2 minutes for instance to initialize"
echo ""
echo "2. SSH and setup EC2:"
echo "   ssh -i ~/.ssh/$KEY_PAIR.pem ec2-user@$PUBLIC_IP"
echo ""
echo "   # Then run on EC2:"
echo "   sudo yum install -y aws-nitro-enclaves-cli aws-nitro-enclaves-cli-devel docker git make socat"
echo "   sudo usermod -aG ne ec2-user && sudo usermod -aG docker ec2-user"
echo "   echo '- {address: fullnode.testnet.sui.io, port: 443}' | sudo tee -a /etc/nitro_enclaves/vsock-proxy.yaml"
echo "   echo '- {address: seal-key-server-testnet-1.mystenlabs.com, port: 443}' | sudo tee -a /etc/nitro_enclaves/vsock-proxy.yaml"
echo "   echo '- {address: seal-key-server-testnet-2.mystenlabs.com, port: 443}' | sudo tee -a /etc/nitro_enclaves/vsock-proxy.yaml"
echo "   sudo bash -c 'cat > /etc/nitro_enclaves/allocator.yaml << EOF"
echo "---"
echo "memory_mib: 4096"
echo "cpu_count: 2"
echo "EOF'"
echo "   sudo systemctl enable --now nitro-enclaves-allocator docker"
echo "   exit  # Logout and login again"
echo ""
echo "3. Transfer code (from local):"
echo "   git archive --format=tar HEAD | gzip > /tmp/mist-protocol.tar.gz"
echo "   scp -i ~/.ssh/$KEY_PAIR.pem /tmp/mist-protocol.tar.gz ec2-user@$PUBLIC_IP:~"
echo ""
echo "4. Build on EC2:"
echo "   mkdir -p mist-protocol && cd mist-protocol && tar -xzf ~/mist-protocol.tar.gz"
echo "   cd nautilus && make ENCLAVE_APP=mist-protocol"
echo ""
echo "5. Run (Terminal 1): ./deploy.sh run"
echo "   Expose (Terminal 2): ./deploy.sh expose"
echo ""
echo "6. Test: curl http://$PUBLIC_IP:3000/health_check"
echo ""

# Save config for later use
cat > ~/.mist-enclave-config << EOF
INSTANCE_ID=$INSTANCE_ID
PUBLIC_IP=$PUBLIC_IP
SECRET_ARN=$SECRET_ARN
REGION=$REGION
KEY_PAIR=$KEY_PAIR
AWS_PROFILE=$AWS_PROFILE
EOF

log "Config saved to ~/.mist-enclave-config"
