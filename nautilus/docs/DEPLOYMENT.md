# Mist Protocol - AWS Deployment Guide

This guide walks you through deploying the Mist Protocol TEE backend on AWS Nitro Enclaves.

## Prerequisites

### Local Machine
- AWS CLI installed and configured
- Docker installed
- Sui CLI installed
- SSH key pair created

### AWS Account
- Account with Nitro Enclave support
- VPC with public subnet
- IAM permissions for EC2, Secrets Manager

## Step 1: AWS Account Setup

### 1.1 Create IAM User (if not exists)

```bash
# Create access keys via AWS Console:
# IAM → Users → Your User → Security credentials → Create access key
```

### 1.2 Configure AWS CLI

```bash
aws configure
# Enter:
# - AWS Access Key ID
# - AWS Secret Access Key
# - Default region: us-east-1
# - Default output format: json
```

### 1.3 Create Key Pair

```bash
aws ec2 create-key-pair \
  --key-name mist-protocol \
  --query 'KeyMaterial' \
  --output text > ~/.ssh/mist-protocol.pem

chmod 400 ~/.ssh/mist-protocol.pem
```

### 1.4 Create Security Group

```bash
# Create security group
aws ec2 create-security-group \
  --group-name mist-protocol-sg \
  --description "Mist Protocol Enclave Security Group"

# Get security group ID
SG_ID=$(aws ec2 describe-security-groups \
  --group-names mist-protocol-sg \
  --query 'SecurityGroups[0].GroupId' \
  --output text)

# Allow SSH (from your IP only)
MY_IP=$(curl -s ifconfig.me)
aws ec2 authorize-security-group-ingress \
  --group-id $SG_ID \
  --protocol tcp \
  --port 22 \
  --cidr $MY_IP/32

# Allow HTTPS (for enclave API)
aws ec2 authorize-security-group-ingress \
  --group-id $SG_ID \
  --protocol tcp \
  --port 443 \
  --cidr 0.0.0.0/0

# Allow port 3000 (for development/testing)
aws ec2 authorize-security-group-ingress \
  --group-id $SG_ID \
  --protocol tcp \
  --port 3000 \
  --cidr 0.0.0.0/0
```

## Step 2: Launch EC2 Instance

### 2.1 Using configure_enclave.sh (Recommended)

```bash
cd nautilus

# Set environment variables
export KEY_PAIR=mist-protocol
export AWS_ACCESS_KEY_ID=<your-access-key>
export AWS_SECRET_ACCESS_KEY=<your-secret-key>
export AWS_SESSION_TOKEN=<your-session-token>  # if using temporary credentials

# Run configuration script
sh configure_enclave.sh mist-protocol
```

Follow prompts:
```
Enter EC2 instance base name: mist-protocol
Do you want to use a secret? (y/n): n
```

### 2.2 Manual Launch (Alternative)

```bash
# Launch c5.xlarge instance with Nitro Enclave support
aws ec2 run-instances \
  --image-id ami-085ad6ae776d8f09c \
  --instance-type c5.xlarge \
  --key-name mist-protocol \
  --security-group-ids $SG_ID \
  --enclave-options 'Enabled=true' \
  --tag-specifications 'ResourceType=instance,Tags=[{Key=Name,Value=mist-protocol-enclave}]'
```

### 2.3 Get Instance IP

```bash
INSTANCE_ID=$(aws ec2 describe-instances \
  --filters "Name=tag:Name,Values=mist-protocol-enclave" \
  --query 'Reservations[0].Instances[0].InstanceId' \
  --output text)

PUBLIC_IP=$(aws ec2 describe-instances \
  --instance-ids $INSTANCE_ID \
  --query 'Reservations[0].Instances[0].PublicIpAddress' \
  --output text)

echo "Instance IP: $PUBLIC_IP"
```

## Step 3: Configure EC2 Instance

### 3.1 SSH into Instance

```bash
ssh -i ~/.ssh/mist-protocol.pem ec2-user@$PUBLIC_IP
```

### 3.2 Install Nitro CLI

```bash
# Install Nitro Enclaves CLI
sudo amazon-linux-extras install aws-nitro-enclaves-cli -y
sudo yum install aws-nitro-enclaves-cli-devel -y

# Add user to groups
sudo usermod -aG ne ec2-user
sudo usermod -aG docker ec2-user

# Start services
sudo systemctl enable --now docker
sudo systemctl enable --now nitro-enclaves-allocator

# Verify
nitro-cli --version
```

### 3.3 Configure Enclave Allocator

```bash
# Edit allocator config
sudo vim /etc/nitro_enclaves/allocator.yaml
```

Set:
```yaml
---
memory_mib: 4096
cpu_count: 2
```

```bash
# Restart allocator
sudo systemctl restart nitro-enclaves-allocator
sudo systemctl status nitro-enclaves-allocator
```

### 3.4 Clone Repository

```bash
git clone <your-repo-url> mist-protocol
cd mist-protocol/nautilus
```

## Step 4: Build & Run Enclave

### 4.1 Build Enclave Image

```bash
cd nautilus

# Build enclave (this takes ~10-15 minutes first time)
make ENCLAVE_APP=mist-protocol

# Check PCR values
cat out/nitro.pcrs
```

Save PCR values for onchain registration:
```bash
PCR0=<value from output>
PCR1=<value from output>
PCR2=<value from output>
```

### 4.2 Run Enclave

```bash
# Production mode
make run

# Or debug mode (for troubleshooting)
make run-debug
```

### 4.3 Expose HTTP Endpoint

```bash
sh expose_enclave.sh
```

### 4.4 Verify Enclave

```bash
# Health check
curl -X GET http://localhost:3000/health_check

# Get attestation
curl -X GET http://localhost:3000/get_attestation
```

## Step 5: Register TEE Onchain

### 5.1 Deploy Move Contracts (Nikola's task)

Wait for Nikola to deploy:
- `mist_protocol` package
- `NullifierRegistry` shared object
- `LiquidityPool` shared object

### 5.2 Get Contract IDs from Nikola

```bash
export MIST_PACKAGE_ID=<from Nikola>
export ENCLAVE_PACKAGE_ID=<from enclave package deployment>
export ENCLAVE_CONFIG_OBJECT_ID=<from deployment>
export CAP_OBJECT_ID=<from deployment>
```

### 5.3 Update PCRs Onchain

```bash
sui client call \
  --function update_pcrs \
  --module enclave \
  --package $ENCLAVE_PACKAGE_ID \
  --type-args "$MIST_PACKAGE_ID::mist_protocol::MIST_PROTOCOL" \
  --args $ENCLAVE_CONFIG_OBJECT_ID $CAP_OBJECT_ID 0x$PCR0 0x$PCR1 0x$PCR2
```

### 5.4 Register Enclave

```bash
ENCLAVE_URL=http://$PUBLIC_IP:3000

sh register_enclave.sh \
  $ENCLAVE_PACKAGE_ID \
  $MIST_PACKAGE_ID \
  $ENCLAVE_CONFIG_OBJECT_ID \
  $ENCLAVE_URL \
  mist_protocol \
  MIST_PROTOCOL

# Save output
export ENCLAVE_OBJECT_ID=<from output>
```

## Step 6: Verify Deployment

### 6.1 Test Endpoints

```bash
# Health check
curl -X GET http://$PUBLIC_IP:3000/health_check

# Get attestation
curl -X GET http://$PUBLIC_IP:3000/get_attestation

# Test swap intent (will return "not implemented" until Max completes backend)
curl -X POST http://$PUBLIC_IP:3000/process_swap_intent \
  -H "Content-Type: application/json" \
  -d '{"payload": {"encrypted_intent": "", "intent_object_id": "0x123", "deadline": 0}}'
```

### 6.2 Verify Onchain Registration

Check on Sui Explorer:
- `ENCLAVE_CONFIG_OBJECT_ID` - should show PCR values
- `ENCLAVE_OBJECT_ID` - should show registered public key

## Troubleshooting

### Enclave won't start

```bash
# Check allocator status
sudo systemctl status nitro-enclaves-allocator

# Check allocator config
cat /etc/nitro_enclaves/allocator.yaml

# Restart services
sudo systemctl restart nitro-enclaves-allocator
sudo systemctl restart docker
```

### Can't connect to enclave

```bash
# Check enclave is running
nitro-cli describe-enclaves

# Check vsock proxy
ps aux | grep vsock

# Re-expose enclave
sh expose_enclave.sh
```

### Build fails

```bash
# Clean and rebuild
rm -rf out/
make ENCLAVE_APP=mist-protocol
```

## Next Steps

After deployment:
1. Share `ENCLAVE_OBJECT_ID` with team
2. Share PCR values for reproducible build verification
3. Configure SEAL key servers (with Max)
4. Set up systemd services for auto-restart (Story 3.4)
