# Setup Guide

Complete guide to setting up and running Mist Protocol locally or in production.

---

## Prerequisites

### Required Tools

```bash
# Sui CLI
curl --proto '=https' --tlsv1.2 -sSf https://docs.sui.io/install.sh | sh

# Node.js 20+
node --version

# pnpm
npm install -g pnpm

# Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### Verify Installation

```bash
sui --version      # Should show 1.60.0+
node --version     # Should show v20.0.0+
pnpm --version     # Should show 8.0.0+
cargo --version    # Should show 1.70.0+
```

---

## Local Development Setup

### 1. Clone Repository

```bash
git clone https://github.com/nikola0x0/mist-protocol.git
cd mist-protocol
```

### 2. Deploy Smart Contracts

```bash
cd contracts/mist_protocol
sui client publish --gas-budget 500000000
```

**Save the output!** You'll need:
- Package ID
- IntentQueue object ID
- LiquidityPool object ID

### 3. Configure Backend

Create `backend/.env`:

```bash
# Backend wallet private key (Bech32 format)
BACKEND_PRIVATE_KEY=suiprivkey1...

# Sui RPC endpoint
SUI_RPC_URL=https://fullnode.testnet.sui.io:443
```

Update `backend/src/apps/mist-protocol/seal_config.yaml`:

```yaml
package_id: "0x..."              # From contract deployment
intent_queue_id: "0x..."         # From contract deployment
liquidity_pool_id: "0x..."       # From contract deployment

# SEAL key servers (testnet)
key_servers:
  - "0x8e..."
  - "0x9a..."
  - "0xa1..."

server_pk_map:
  "0x8e...": "base64_public_key_1"
  "0x9a...": "base64_public_key_2"
  "0xa1...": "base64_public_key_3"
```

**Get SEAL key server IDs:**
```bash
# For testnet
https://docs.mystenlabs.com/seal
```

### 4. Configure Frontend

Create `frontend/.env.local`:

```bash
NEXT_PUBLIC_PACKAGE_ID=0x...
NEXT_PUBLIC_POOL_ID=0x...
NEXT_PUBLIC_INTENT_QUEUE_ID=0x...
NEXT_PUBLIC_NETWORK=testnet
```

### 5. Import Backend Key to Sui Keystore

```bash
# The tx-signer service uses your local Sui keystore
sui keytool import "$BACKEND_PRIVATE_KEY" ed25519 --alias backend

# Verify it's imported
sui keytool list | grep backend
```

---

## Running Locally

### Terminal 1: Signing Service

```bash
cd tx-signer
cargo run

# Should show:
# 🔐 Transaction Signing Service
# ✅ Sui CLI found
# Listening on http://127.0.0.1:4000
```

### Terminal 2: Backend

```bash
cd backend
cargo run

# Should show:
# ✅ Backend starting...
# 🔑 Backend Wallet: 0x...
# 🚀 Backend listening on port 3001
# 📊 Poll cycle #1
```

### Terminal 3: Frontend

```bash
cd frontend
pnpm dev

# Open browser: http://localhost:3000
```

---

## Testing the Flow

### 1. Deposit SUI

1. Connect wallet on frontend
2. Click "Deposit"
3. Enter amount (e.g., 0.5 SUI)
4. Approve transaction
5. See encrypted ticket in vault

### 2. Decrypt Balance

1. Click "Decrypt" on ticket
2. Sign personal message
3. View decrypted amount

### 3. Create Swap Intent

1. Select ticket(s) to swap
2. Choose output token (SUI or USDC)
3. Set slippage (e.g., 5%)
4. Create intent transaction
5. Backend will process automatically within 5 seconds

### 4. View Swap Result

1. Wait for backend to process (~5-10 seconds)
2. Refresh vault
3. See new output ticket
4. Decrypt to see swapped amount

---

## Production Deployment (AWS EC2)

### EC2 Instance Setup

**Instance Type:** t3.medium or larger
**AMI:** Ubuntu 24.04 LTS
**Security Groups:**
- Port 3001 (Backend API)
- Port 3000 (Frontend)
- Port 4000 (Internal - tx-signer, localhost only)

### Installation Script

```bash
#!/bin/bash
# Install Sui CLI
curl --proto '=https' --tlsv1.2 -sSf https://docs.sui.io/install.sh | sh
export PATH="$HOME/.sui/bin:$PATH"

# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env

# Clone and build
git clone https://github.com/nikola0x0/mist-protocol.git
cd mist-protocol

# Import backend key
sui keytool import "$BACKEND_PRIVATE_KEY" ed25519 --alias backend

# Build backend
cd backend
cargo build --release

# Build tx-signer
cd ../tx-signer
cargo build --release

# Build frontend
cd ../frontend
pnpm install
pnpm build
```

### Systemd Services

**tx-signer.service:**
```ini
[Unit]
Description=Transaction Signing Service
After=network.target

[Service]
Type=simple
User=ubuntu
WorkingDirectory=/home/ubuntu/mist-protocol/tx-signer
ExecStart=/home/ubuntu/mist-protocol/tx-signer/target/release/tx-signer
Restart=always
RestartSec=3

[Install]
WantedBy=multi-user.target
```

**backend.service:**
```ini
[Unit]
Description=Mist Protocol Backend
After=network.target tx-signer.service
Requires=tx-signer.service

[Service]
Type=simple
User=ubuntu
WorkingDirectory=/home/ubuntu/mist-protocol/backend
Environment="BACKEND_PRIVATE_KEY=suiprivkey1..."
Environment="SUI_RPC_URL=https://fullnode.testnet.sui.io:443"
ExecStart=/home/ubuntu/mist-protocol/backend/target/release/nautilus-server
Restart=always
RestartSec=3

[Install]
WantedBy=multi-user.target
```

**Enable services:**
```bash
sudo systemctl daemon-reload
sudo systemctl enable --now tx-signer.service
sudo systemctl enable --now backend.service
```

---

## Troubleshooting

### Backend not polling intents

**Check:**
```bash
# Verify mist-protocol feature is enabled (it's default)
cargo build

# Check logs
journalctl -u backend.service -f
```

### Signing service unreachable

**Check:**
```bash
# Verify tx-signer is running
curl http://127.0.0.1:4000/health

# Check if backend key is imported
sui keytool list | grep backend
```

### Frontend can't decrypt tickets

**Possible causes:**
1. Wait 5-10 seconds after transaction (RPC propagation)
2. Refresh the vault list
3. Check browser console for errors
4. Verify you're using the correct wallet address

### Transaction fails with "Not authorized"

**Fix:** Make sure the backend wallet address matches `tee_authority` in the contract:

```bash
# Get backend address
grep BACKEND_PRIVATE_KEY backend/.env

# Decode and check address
sui keytool show <address>

# Should match the address in seal_policy.move:
# const BACKEND_ADDRESS: address = @0x...
```

---

## Configuration Reference

### Backend Environment Variables

```bash
BACKEND_PRIVATE_KEY=suiprivkey1...  # Required
SUI_RPC_URL=https://...            # Required
```

### Frontend Environment Variables

```bash
NEXT_PUBLIC_PACKAGE_ID=0x...
NEXT_PUBLIC_POOL_ID=0x...
NEXT_PUBLIC_INTENT_QUEUE_ID=0x...
NEXT_PUBLIC_NETWORK=testnet
```

---

## Development Tips

### Rebuild after contract changes

```bash
# 1. Republish contract
cd contracts/mist_protocol
sui client publish --gas-budget 500000000

# 2. Update backend config
vim backend/src/apps/mist-protocol/seal_config.yaml

# 3. Update frontend config
vim frontend/.env.local

# 4. Restart backend
cd backend && cargo run
```

### View backend logs

```bash
# Backend prints detailed logs for each step:
# 📊 Poll cycle #N
# ✅ Successfully decrypted intent
# 💱 Mock swap: X → Y
# 🔐 Encrypting output amount with SEAL...
# ✅ Transaction signed successfully!
# ✅ Swap executed successfully!
```

### Monitor signing service

```bash
# Signing service shows each signature request:
# 📝 Signing request for address: 0x...
# ✅ Transaction signed successfully!
```

---

## Next Steps

After setup is complete:

1. **Test the complete flow** (deposit → swap → decrypt)
2. **Integrate real Cetus swaps** (replace mock in swap_executor.rs)
3. **Deploy to AWS Nitro Enclaves** (production TEE)
4. **Add monitoring** (Prometheus, Grafana)

See `docs/ARCHITECTURE.md` for system design details.
