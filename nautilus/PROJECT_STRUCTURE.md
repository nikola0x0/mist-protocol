# Nautilus Track Project Structure

## Overview
This directory contains Nikola's track for implementing the Nautilus TEE (Trusted Execution Environment) component of Mist Protocol.

## Directory Structure

```
nautilus/
├── docs/                           # Research and analysis documents
│   ├── ANALYSIS.md                 # Initial Nautilus analysis
│   ├── NAUTILUS_DEEP_DIVE.md       # Technical deep dive
│   ├── NAUTILUS_FEASIBILITY.md     # Feasibility assessment
│   ├── NAUTILUS_STRATEGY.md        # Implementation strategy
│   ├── NAUTILUS_FOR_MIST_PROTOCOL.md  # How Nautilus fits into Mist
│   └── MIST_PROTOCOL_DESIGN.md     # Overall protocol design
│
├── nautilus-framework/             # Main implementation directory
│   ├── src/
│   │   └── nautilus-server/
│   │       └── src/
│   │           └── apps/
│   │               ├── weather-example/     # Reference: Working example
│   │               ├── twitter-example/     # Reference: Another example
│   │               └── mist-protocol/       # YOUR IMPLEMENTATION HERE
│   │                   ├── mod.rs           # Main logic
│   │                   └── allowed_endpoints.yaml  # Seal + Cetus endpoints
│   │
│   ├── move/                        # Smart contracts
│   │   ├── enclave/                 # Base enclave verification
│   │   ├── weather-example/         # Reference contract
│   │   └── mist-protocol/           # YOUR SMART CONTRACT HERE
│   │
│   ├── configure_enclave.sh         # AWS setup script
│   ├── expose_enclave.sh            # Generated: exposes enclave to internet
│   └── src/nautilus-server/run.sh   # Generated: runs inside enclave
│
├── nautilus-twitter-example/        # Reference: Full end-to-end example
│
├── IMPLEMENTATION_PLAN.md           # Step-by-step implementation guide
├── TROUBLESHOOTING.md               # Deployment guide (VERIFIED WORKING)
└── PROJECT_STRUCTURE.md             # This file

```

## Key Documents

### Must Read (in order)
1. **IMPLEMENTATION_PLAN.md** - Your comprehensive plan for Days 1-4
2. **TROUBLESHOOTING.md** - VERIFIED working deployment process
3. **docs/MIST_PROTOCOL_DESIGN.md** - End-to-end system design

### Reference Documents
- **docs/NAUTILUS_FOR_MIST_PROTOCOL.md** - How Nautilus integrates with Mist Protocol
- **docs/NAUTILUS_STRATEGY.md** - Hybrid strategy (real TEE vs mock backend)
- **nautilus-framework/UsingNautilus.md** - Official Nautilus documentation

### Examples to Reference
- **nautilus-framework/src/nautilus-server/src/apps/weather-example/** - Simple API call example
- **nautilus-twitter-example/** - Full Twitter OAuth example with frontend

## Your Implementation: mist-protocol

### What You Need to Build

**Location**: `nautilus-framework/src/nautilus-server/src/apps/mist-protocol/`

**Files to Create**:
1. `mod.rs` - Main Rust logic for:
   - Receiving encrypted swap intent
   - Decrypting with Seal
   - Executing swap on Cetus DEX
   - Signing and returning verifiable result

2. `allowed_endpoints.yaml` - Network access for:
   ```yaml
   endpoints:
     - seal-server-1.sui-testnet.io
     - seal-server-2.sui-testnet.io
     - cetus-api.sui-testnet.io
     - fullnode.testnet.sui.io
   ```

**Smart Contract Location**: `nautilus-framework/move/mist-protocol/`

### Data Flow (from IMPLEMENTATION_PLAN.md)

```
User → Frontend → Mist Contract (on Sui)
                        ↓
                  Nautilus TEE (this is YOUR code)
                        ↓
         1. Receive encrypted intent
         2. Call Seal → decrypt intent
         3. Call Cetus DEX → execute swap
         4. Sign result with enclave key
         5. Return signed proof
                        ↓
                  Mist Contract verifies signature
                        ↓
                  Mint receipt NFT for user
```

## Current State

✅ **Completed**:
- AWS account setup with proper IAM permissions
- EC2 key pair created (`nautilus-demo-key`)
- Weather-example successfully deployed and tested
- Full deployment process documented in TROUBLESHOOTING.md

⏳ **Next Steps** (Day 4 - Enclave Implementation):
1. Create `mist-protocol` app directory structure
2. Implement `mod.rs` with Seal + Cetus integration
3. Configure `allowed_endpoints.yaml`
4. Deploy and test on AWS Nitro Enclave

## AWS Resources

- **Profile**: `nikola0x0-user`
- **Region**: `us-east-1`
- **Key Pair**: `nautilus-demo-key`
- **Security Group**: `instance-script-sg` (sg-03a148c1dde51aca4)
- **Test Instance**: i-036dbd2c53a6fdf31 (STOPPED - can restart for reference)

## Important Notes

1. **Always use secrets**: Store Seal credentials, DEX keys in AWS Secrets Manager
2. **Rebuild after changes**: `run.sh` is baked into the EIF during `make`
3. **Use screen for persistence**: Enclaves in foreground stop when SSH disconnects
4. **Verify IAM profile**: EC2 needs IAM role to access secrets

## Quick Reference Commands

```bash
# Start working on mist-protocol
cd nautilus-framework/src/nautilus-server/src/apps/

# Deploy to AWS (when ready)
cd nautilus-framework
export KEY_PAIR=nautilus-demo-key AWS_PROFILE=nikola0x0-user
sh configure_enclave.sh mist-protocol

# Test deployed enclave
curl -H 'Content-Type: application/json' \
  -d '{"payload": {"intent_id": "...", "encrypted_data": "...", "key_id": "..."}}' \
  -X POST http://<PUBLIC_IP>:3000/process_intent
```

## Getting Help

- **Nautilus Discord**: https://discord.com/channels/916379725201563759/1361500579603546223
- **Official Docs**: https://docs.sui.io/concepts/cryptography/nautilus
- **Your Implementation Plan**: IMPLEMENTATION_PLAN.md
- **Deployment Guide**: TROUBLESHOOTING.md
