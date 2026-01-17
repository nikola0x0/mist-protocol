# 🚀 AWS Nitro Enclave Quickstart - Mist Protocol

## Prerequisites
- AWS CLI configured ✅
- SSH key pair created

## Step 1: Create EC2 Instance

```bash
# Set your key pair name
export KEY_PAIR="mist-protocol"   # Change this to your key name
export REGION="us-east-1"

# Create security group
aws ec2 create-security-group \
  --group-name mist-enclave-sg \
  --description "Mist Protocol Enclave" \
  --region $REGION

# Get security group ID
SG_ID=$(aws ec2 describe-security-groups \
  --group-names mist-enclave-sg \
  --query 'SecurityGroups[0].GroupId' \
  --output text \
  --region $REGION)

# Allow SSH, HTTPS, and app port
aws ec2 authorize-security-group-ingress --group-id $SG_ID --protocol tcp --port 22 --cidr 0.0.0.0/0 --region $REGION
aws ec2 authorize-security-group-ingress --group-id $SG_ID --protocol tcp --port 443 --cidr 0.0.0.0/0 --region $REGION
aws ec2 authorize-security-group-ingress --group-id $SG_ID --protocol tcp --port 3000 --cidr 0.0.0.0/0 --region $REGION

# Launch c5.xlarge with Nitro Enclave enabled (~$0.17/hr)
aws ec2 run-instances \
  --image-id ami-085ad6ae776d8f09c \
  --instance-type c5.xlarge \
  --key-name $KEY_PAIR \
  --security-group-ids $SG_ID \
  --enclave-options 'Enabled=true' \
  --block-device-mappings '[{"DeviceName":"/dev/xvda","Ebs":{"VolumeSize":30,"VolumeType":"gp3"}}]' \
  --tag-specifications 'ResourceType=instance,Tags=[{Key=Name,Value=mist-enclave}]' \
  --region $REGION

# Get instance public IP (wait ~30s for instance to start)
aws ec2 describe-instances \
  --filters "Name=tag:Name,Values=mist-enclave" "Name=instance-state-name,Values=running" \
  --query 'Reservations[0].Instances[0].PublicIpAddress' \
  --output text \
  --region $REGION
```

## Step 2: Setup EC2 Instance

```bash
# SSH into instance
ssh -i ~/.ssh/$KEY_PAIR.pem ec2-user@<PUBLIC_IP>

# Install Nitro CLI & Docker
sudo amazon-linux-extras install aws-nitro-enclaves-cli -y
sudo yum install aws-nitro-enclaves-cli-devel docker git -y

# Add user to groups
sudo usermod -aG ne ec2-user
sudo usermod -aG docker ec2-user

# Configure enclave allocator (4GB RAM, 2 CPUs for enclave)
sudo tee /etc/nitro_enclaves/allocator.yaml > /dev/null <<EOF
memory_mib: 4096
cpu_count: 2
EOF

# Enable services
sudo systemctl enable nitro-enclaves-allocator docker
sudo systemctl start nitro-enclaves-allocator docker

# IMPORTANT: Logout and login again for group changes
exit
```

## Step 3: Clone & Build

```bash
ssh -i ~/.ssh/$KEY_PAIR.pem ec2-user@<PUBLIC_IP>

# Clone repo
git clone <YOUR_REPO_URL> mist-protocol
cd mist-protocol/nautilus

# Build enclave image (takes ~10-15 min first time)
make ENCLAVE_APP=mist-protocol

# View PCR values (save these for contract registration!)
cat out/nitro.pcrs
```

## Step 4: Run Enclave

**Terminal 1 - Run Enclave:**
```bash
cd ~/mist-protocol/nautilus

# Set backend private key
export BACKEND_PRIVATE_KEY="suiprivkey1..."  # Your Sui wallet private key

# Run enclave
make run
```

**Terminal 2 - Expose Ports:**
```bash
cd ~/mist-protocol/nautilus
sh expose_enclave.sh
```

## Step 5: Test

```bash
# Health check
curl http://localhost:3000/health_check

# From your local machine
curl http://<PUBLIC_IP>:3000/health_check
curl http://<PUBLIC_IP>:3000/get_attestation
```

---

## Quick Reference

| Item | Value |
|------|-------|
| Instance Type | c5.xlarge |
| Cost | ~$0.17/hr (~$124/month) |
| Enclave RAM | 4GB |
| Enclave CPUs | 2 |
| App Port | 3000 |
| Region | us-east-1 |

## Stop/Start Instance

```bash
# Get instance ID
INSTANCE_ID=$(aws ec2 describe-instances \
  --filters "Name=tag:Name,Values=mist-enclave" \
  --query 'Reservations[0].Instances[0].InstanceId' \
  --output text \
  --region $REGION)

# Stop (to save cost when not using)
aws ec2 stop-instances --instance-ids $INSTANCE_ID --region $REGION

# Start
aws ec2 start-instances --instance-ids $INSTANCE_ID --region $REGION
```

## Systemd (Auto-start on reboot)

```bash
# Copy service files
sudo cp ~/mist-protocol/systemd/mist-enclave.service /etc/systemd/system/
sudo cp ~/mist-protocol/systemd/mist-vsock-proxy.service /etc/systemd/system/

# Edit to set BACKEND_PRIVATE_KEY
sudo nano /etc/systemd/system/mist-enclave.service

# Enable
sudo systemctl daemon-reload
sudo systemctl enable mist-vsock-proxy mist-enclave
```

## Troubleshooting

```bash
# Check enclave status
nitro-cli describe-enclaves

# View enclave logs
nitro-cli console --enclave-id $(nitro-cli describe-enclaves | jq -r '.[0].EnclaveID')

# Reset enclave
nitro-cli terminate-enclave --all
```
