# Nautilus TEE Server

Verifiable off-chain computation using AWS Nitro Enclaves.

## Overview

This component provides:
- Secure computation in AWS Nitro Enclave
- Cryptographic attestations
- On-chain verification
- Reproducible builds

## Status

- [x] Feasibility confirmed
- [ ] AWS account setup
- [ ] Enclave deployed
- [ ] Attestation working
- [ ] Integration complete

## Prerequisites

```bash
docker --version    # 24.0+
aws --version       # 2.0+
rustc --version     # 1.70+
cargo --version     # Latest
```

## AWS Nitro Requirements

### Supported Instance Types
- c6a.xlarge
- m6a.xlarge
- r6a.xlarge
- (see AWS Nitro Enclaves docs for full list)

### Required Permissions
- EC2 full access
- Nitro Enclaves permissions

## Quick Start

### 1. Set Up AWS

```bash
# Configure AWS CLI
aws configure

# Launch Nitro-compatible instance
# Use AWS Console or:
aws ec2 run-instances \
  --instance-type c6a.xlarge \
  --image-id <nitro-supported-ami> \
  --enclave-options 'Enabled=true'
```

### 2. Clone Nautilus Template

```bash
# Reference implementation
git clone https://github.com/MystenLabs/nautilus
cd nautilus

# Review the weather example
cat src/main.rs
```

### 3. Build Enclave

```bash
# Build the enclave
make ENCLAVE_APP=mist-privacy

# Verify build
ls -l build/
```

### 4. Configure

```bash
# Edit configuration
cp config.example.yaml config.yaml
vim config.yaml

# Run configuration script
./configure_enclave.sh
```

### 5. Deploy

```bash
# Register enclave on Sui
./register_enclave.sh

# Verify attestation
sui client call --function verify_attestation ...
```

## Architecture

```
┌─────────────────────────────────────┐
│        AWS Nitro Enclave            │
│  ┌──────────────────────────────┐  │
│  │   Rust Backend (Axum)        │  │
│  │   - Computation endpoints    │  │
│  │   - Attestation generation   │  │
│  │   - Signature creation       │  │
│  └──────────────────────────────┘  │
│           ↓ Attestation             │
└─────────────────────────────────────┘
           ↓
┌─────────────────────────────────────┐
│       Sui Blockchain                │
│  ┌──────────────────────────────┐  │
│  │  nautilus_verifier.move      │  │
│  │  - Verify PCR values         │  │
│  │  - Validate signatures       │  │
│  │  - Accept TEE outputs        │  │
│  └──────────────────────────────┘  │
└─────────────────────────────────────┘
```

## Endpoints

### POST /process_encrypted
Process encrypted data inside TEE

**Request:**
```json
{
  "encrypted_data": "0x...",
  "operation": "compute_sum"
}
```

**Response:**
```json
{
  "result": "0x...",
  "attestation": "0x...",
  "signature": "0x...",
  "pcr_values": {
    "pcr0": "0x...",
    "pcr1": "0x...",
    "pcr2": "0x..."
  }
}
```

## Reproducible Builds

### Build Locally
```bash
git clone <this-repo>
cd nautilus
make ENCLAVE_APP=mist-privacy
cat build/nitro.pcrs
```

### Verify PCR Values
```bash
# Get on-chain PCRs
sui client object <enclave-config-id>

# Compare with local build
diff <(cat build/nitro.pcrs) <(echo "$ONCHAIN_PCRS")
```

## Testing

### Local Testing (Mock)
```bash
# Run without enclave for development
cargo run --features mock-attestation
```

### Enclave Testing
```bash
# Run in actual enclave
nitro-cli run-enclave \
  --eif-path build/mist-privacy.eif \
  --memory 4096 \
  --cpu-count 2

# Test endpoints
curl http://localhost:3000/health
```

## Deployment

### Testnet
```bash
# Deploy enclave
./scripts/deploy.sh testnet

# Register on Sui testnet
./scripts/register_enclave.sh
```

### Mainnet (Post-Hackathon)
```bash
# Full security audit required first
./scripts/deploy.sh mainnet
```

## Monitoring

```bash
# View enclave logs
nitro-cli console --enclave-id <id>

# Check attestation status
./scripts/check_attestation.sh
```

## Troubleshooting

### Enclave Won't Start
- Check instance type supports Nitro Enclaves
- Verify enclave options enabled
- Check memory allocation (min 4GB)

### Attestation Fails
- Verify PCR values match
- Check certificate chain
- Ensure clock sync

### High Gas Costs
- Only verify attestation at registration
- Use enclave key for subsequent calls
- Cache verification results

## Cost Estimates

### AWS Costs
- c6a.xlarge: ~$0.17/hour
- Data transfer: negligible for hackathon
- Storage: minimal

### Sui Gas Costs
- Attestation verification: ~0.1 SUI (testnet free)
- Regular operations: <0.01 SUI

## Resources

- **Nautilus Docs:** https://docs.sui.io/concepts/cryptography/nautilus
- **GitHub:** https://github.com/MystenLabs/nautilus
- **Example:** https://github.com/MystenLabs/nautilus-twitter
- **Discord:** #nautilus in Sui Discord
- **Feasibility Doc:** ../NAUTILUS_FEASIBILITY.md

## Development Notes

### For Hackathon
- Start with weather example as template
- Implement ONE computation endpoint
- Focus on attestation verification
- Reproducible build is key demo point

### Fallback Plan
If AWS setup fails:
```rust
#[cfg(feature = "mock-attestation")]
fn generate_attestation() -> Attestation {
    // Mock implementation for demo
    Attestation {
        pcr0: "mock_value".into(),
        verified: true,
        note: "Demo mode - production uses real AWS Nitro".into(),
    }
}
```

## Team Notes

**Owner:** [Your name]
**Status:** Setup phase
**Next Steps:**
1. AWS account setup
2. Instance launch
3. Template deployment
4. First attestation test

**Questions?** Ask in #nautilus on Sui Discord
