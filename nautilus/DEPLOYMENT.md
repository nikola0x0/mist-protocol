# Mist Protocol - AWS Nitro Enclave Deployment Guide

Complete guide for deploying the Mist Protocol TEE backend on AWS Nitro Enclaves.

## Prerequisites

- AWS CLI installed and configured
- SSH key pair for EC2 access
- Sui wallet private key (Bech32 format: `suiprivkey1...`)

## Quick Start (5 commands)

```bash
# 1. Set variables
export AWS_PROFILE="your-profile"
export REGION="ap-southeast-1"
export KEY_PAIR="your-key-pair"
export BACKEND_PRIVATE_KEY="suiprivkey1..."

# 2. Run setup script (creates everything)
./setup-aws.sh

# 3. SSH to EC2 and build
ssh -i ~/.ssh/$KEY_PAIR.pem ec2-user@<PUBLIC_IP>
cd mist-protocol/nautilus && make ENCLAVE_APP=mist-protocol

# 4. Run enclave (Terminal 1)
./deploy.sh run

# 5. Expose ports (Terminal 2)
./deploy.sh expose
```

---

## Detailed Setup

### Step 1: AWS Secrets Manager

Store the backend private key securely:

```bash
aws secretsmanager create-secret \
  --name mist-backend-key \
  --secret-string "$BACKEND_PRIVATE_KEY" \
  --region $REGION \
  --profile $AWS_PROFILE
```

Save the ARN returned (e.g., `arn:aws:secretsmanager:ap-southeast-1:123456789:secret:mist-backend-key-xxxxx`)

### Step 2: IAM Role for EC2

Create a role that allows EC2 to read the secret:

```bash
# Create trust policy
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

# Create role
aws iam create-role \
  --role-name mist-enclave-role \
  --assume-role-policy-document file:///tmp/trust-policy.json \
  --profile $AWS_PROFILE

# Attach secrets read policy
aws iam attach-role-policy \
  --role-name mist-enclave-role \
  --policy-arn arn:aws:iam::aws:policy/SecretsManagerReadWrite \
  --profile $AWS_PROFILE

# Create instance profile
aws iam create-instance-profile \
  --instance-profile-name mist-enclave-profile \
  --profile $AWS_PROFILE

# Add role to profile
aws iam add-role-to-instance-profile \
  --instance-profile-name mist-enclave-profile \
  --role-name mist-enclave-role \
  --profile $AWS_PROFILE
```

### Step 3: Security Group

```bash
# Create security group
SG_ID=$(aws ec2 create-security-group \
  --group-name mist-enclave-sg \
  --description "Mist Protocol Enclave" \
  --region $REGION \
  --profile $AWS_PROFILE \
  --query 'GroupId' --output text)

# Allow SSH, HTTPS, and app port
aws ec2 authorize-security-group-ingress --group-id $SG_ID --protocol tcp --port 22 --cidr 0.0.0.0/0 --region $REGION --profile $AWS_PROFILE
aws ec2 authorize-security-group-ingress --group-id $SG_ID --protocol tcp --port 443 --cidr 0.0.0.0/0 --region $REGION --profile $AWS_PROFILE
aws ec2 authorize-security-group-ingress --group-id $SG_ID --protocol tcp --port 3000 --cidr 0.0.0.0/0 --region $REGION --profile $AWS_PROFILE

echo "Security Group: $SG_ID"
```

### Step 4: EC2 Key Pair (if needed)

```bash
aws ec2 create-key-pair \
  --key-name $KEY_PAIR \
  --query 'KeyMaterial' \
  --output text \
  --region $REGION \
  --profile $AWS_PROFILE > ~/.ssh/$KEY_PAIR.pem

chmod 400 ~/.ssh/$KEY_PAIR.pem
```

### Step 5: Launch EC2 Instance

```bash
# Get latest Amazon Linux 2023 AMI
AMI_ID=$(aws ssm get-parameters \
  --names /aws/service/ami-amazon-linux-latest/al2023-ami-kernel-default-x86_64 \
  --region $REGION \
  --profile $AWS_PROFILE \
  --query 'Parameters[0].Value' --output text)

# Launch instance with Nitro Enclave enabled
INSTANCE_ID=$(aws ec2 run-instances \
  --image-id $AMI_ID \
  --instance-type c5.xlarge \
  --key-name $KEY_PAIR \
  --security-group-ids $SG_ID \
  --enclave-options 'Enabled=true' \
  --block-device-mappings '[{"DeviceName":"/dev/xvda","Ebs":{"VolumeSize":30,"VolumeType":"gp3"}}]' \
  --tag-specifications 'ResourceType=instance,Tags=[{Key=Name,Value=mist-enclave}]' \
  --region $REGION \
  --profile $AWS_PROFILE \
  --query 'Instances[0].InstanceId' --output text)

echo "Instance ID: $INSTANCE_ID"

# Wait for instance to run
aws ec2 wait instance-running --instance-ids $INSTANCE_ID --region $REGION --profile $AWS_PROFILE

# Attach IAM profile
aws ec2 associate-iam-instance-profile \
  --instance-id $INSTANCE_ID \
  --iam-instance-profile Name=mist-enclave-profile \
  --region $REGION \
  --profile $AWS_PROFILE

# Get public IP
PUBLIC_IP=$(aws ec2 describe-instances \
  --instance-ids $INSTANCE_ID \
  --region $REGION \
  --profile $AWS_PROFILE \
  --query 'Reservations[0].Instances[0].PublicIpAddress' --output text)

echo "Public IP: $PUBLIC_IP"
echo "SSH: ssh -i ~/.ssh/$KEY_PAIR.pem ec2-user@$PUBLIC_IP"
```

### Step 6: EC2 Instance Setup

SSH into the instance and run:

```bash
ssh -i ~/.ssh/$KEY_PAIR.pem ec2-user@$PUBLIC_IP

# Install dependencies
sudo yum install -y aws-nitro-enclaves-cli aws-nitro-enclaves-cli-devel docker git make socat

# Add user to groups
sudo usermod -aG ne ec2-user
sudo usermod -aG docker ec2-user

# Configure vsock-proxy allowlist
echo '- {address: fullnode.testnet.sui.io, port: 443}' | sudo tee -a /etc/nitro_enclaves/vsock-proxy.yaml
echo '- {address: seal-key-server-testnet-1.mystenlabs.com, port: 443}' | sudo tee -a /etc/nitro_enclaves/vsock-proxy.yaml
echo '- {address: seal-key-server-testnet-2.mystenlabs.com, port: 443}' | sudo tee -a /etc/nitro_enclaves/vsock-proxy.yaml

# Configure enclave allocator
sudo tee /etc/nitro_enclaves/allocator.yaml > /dev/null << 'EOF'
---
memory_mib: 4096
cpu_count: 2
EOF

# Enable services
sudo systemctl enable --now nitro-enclaves-allocator docker

# IMPORTANT: Logout and login again
exit
```

### Step 7: Transfer Code & Build

From your local machine:

```bash
# Create tarball and transfer
git archive --format=tar HEAD | gzip > /tmp/mist-protocol.tar.gz
scp -i ~/.ssh/$KEY_PAIR.pem /tmp/mist-protocol.tar.gz ec2-user@$PUBLIC_IP:~
```

On EC2:

```bash
# Extract
mkdir -p mist-protocol && cd mist-protocol
tar -xzf ~/mist-protocol.tar.gz

# Build enclave (takes ~10-15 min first time, ~2-5 min on rebuilds)
cd nautilus
make ENCLAVE_APP=mist-protocol
```

### Step 8: Run Enclave

**Terminal 1 - Run enclave:**
```bash
cd ~/mist-protocol/nautilus
./deploy.sh run
# Or for debug output: ./deploy.sh run-debug
```

**Terminal 2 - Expose ports:**
```bash
cd ~/mist-protocol/nautilus
./deploy.sh expose
```

The expose script automatically fetches `BACKEND_PRIVATE_KEY` from AWS Secrets Manager.

### Step 9: Test

```bash
# Health check
curl http://localhost:3000/health_check

# From external
curl http://$PUBLIC_IP:3000/health_check

# Attestation (proves code is running in TEE)
curl http://$PUBLIC_IP:3000/get_attestation
```

---

## Configuration

### Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `SECRET_NAME` | AWS Secrets Manager secret name | `mist-backend-key` |
| `AWS_REGION` | AWS region for secrets | `ap-southeast-1` |
| `BACKEND_PRIVATE_KEY` | Direct key (skips AWS fetch) | - |

### Files to Update for New Deployment

1. **`nautilus/src/nautilus-server/src/apps/mist-protocol/seal_config.yaml`**
   - `package_id`: Your deployed contract package ID
   - `pool_id`: LiquidityPool object ID
   - `registry_id`: NullifierRegistry object ID

2. **`frontend/.env`**
   - `NEXT_PUBLIC_PACKAGE_ID`
   - `NEXT_PUBLIC_POOL_ID`
   - `NEXT_PUBLIC_REGISTRY_ID`
   - `NEXT_PUBLIC_ENCLAVE_URL` (optional, for relayer mode)

---

## deploy.sh Commands

| Command | Description |
|---------|-------------|
| `./deploy.sh setup` | Initial EC2 setup (packages, config) |
| `./deploy.sh build` | Build enclave image |
| `./deploy.sh run` | Start enclave |
| `./deploy.sh run-debug` | Start enclave with console output |
| `./deploy.sh expose` | Expose ports + send secrets |
| `./deploy.sh status` | Show enclave status |
| `./deploy.sh transfer` | Transfer code from local to EC2 |

---

## Costs

| Resource | Cost |
|----------|------|
| c5.xlarge | ~$0.17/hr (~$124/month) |
| Secrets Manager | ~$0.40/secret/month |
| Data transfer | Varies |

**Save costs:** Stop the instance when not in use:
```bash
aws ec2 stop-instances --instance-ids $INSTANCE_ID --region $REGION --profile $AWS_PROFILE
```

---

## Troubleshooting

### Enclave won't start
```bash
# Check allocator status
sudo systemctl status nitro-enclaves-allocator

# Check allocator config
cat /etc/nitro_enclaves/allocator.yaml

# Ensure memory_mib is set correctly (needs --- header)
```

### Connection errors in enclave logs
```bash
# Verify vsock-proxy allowlist
cat /etc/nitro_enclaves/vsock-proxy.yaml

# Should contain:
# - {address: fullnode.testnet.sui.io, port: 443}
# - {address: seal-key-server-testnet-1.mystenlabs.com, port: 443}
# - {address: seal-key-server-testnet-2.mystenlabs.com, port: 443}
```

### Can't fetch secret from AWS
```bash
# Test AWS CLI access
aws secretsmanager get-secret-value --secret-id mist-backend-key --region ap-southeast-1

# Check instance has IAM role attached
aws ec2 describe-iam-instance-profile-associations --filters "Name=instance-id,Values=$INSTANCE_ID"
```

### View enclave logs
```bash
# Must run in debug mode
./deploy.sh run-debug

# Or attach to running enclave (only works in debug mode)
sudo nitro-cli console --enclave-id $(nitro-cli describe-enclaves | jq -r '.[0].EnclaveID')
```

### Reset everything
```bash
sudo nitro-cli terminate-enclave --all
pkill -f vsock-proxy
pkill -f socat
```

---

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                        EC2 Host                              │
│  ┌─────────────────┐     ┌─────────────────────────────────┐│
│  │ expose_enclave  │     │         Nitro Enclave           ││
│  │                 │     │  ┌─────────────────────────────┐││
│  │ - Fetches secret│────▶│  │      mist-server            │││
│  │   from AWS SM   │VSOCK│  │                             │││
│  │                 │7777 │  │  - Intent processor         │││
│  │ - Port forward  │     │  │  - SEAL decryption          │││
│  │   3000 ◀───────▶│3000 │  │  - Swap execution           │││
│  │                 │     │  │                             │││
│  │ - vsock-proxy   │     │  └─────────────────────────────┘││
│  │   8101-8103 ────┼─────┼──▶ traffic_forwarder.py        ││
│  └─────────────────┘     │      └──▶ Sui RPC              ││
│                          │      └──▶ SEAL servers         ││
│                          └─────────────────────────────────┘│
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
                    ┌─────────────────┐
                    │   Sui Testnet   │
                    │  - Contracts    │
                    │  - SEAL servers │
                    └─────────────────┘
```

---

## Security Notes

1. **Backend Private Key**: Stored in AWS Secrets Manager, only accessible by the EC2 instance via IAM role
2. **Enclave Isolation**: Code runs in isolated TEE, host cannot inspect memory
3. **Attestation**: `/get_attestation` endpoint provides cryptographic proof of enclave integrity
4. **SEAL Decryption**: Only possible inside the enclave with valid TEE attestation
