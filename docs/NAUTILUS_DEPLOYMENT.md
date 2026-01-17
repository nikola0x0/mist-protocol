# Deploying Mist Protocol Backend to Nautilus

This guide explains how to deploy the Mist Protocol backend to AWS Nitro Enclaves using the Nautilus framework.

## Overview

Nautilus provides:
- Reproducible enclave builds (deterministic PCRs)
- Traffic forwarding for enclave network access
- Attestation document generation for on-chain registration
- SEAL threshold encryption integration

## Current State vs. Required Changes

The Mist Protocol backend (`backend/`) is already structured similarly to Nautilus apps but needs modifications for enclave deployment.

---

## 1. Directory Structure

Create a Nautilus-compatible structure:

```
mist-protocol/
├── nautilus/                          # New directory for enclave
│   ├── Cargo.toml                     # Workspace for init, aws, system
│   ├── Containerfile                  # Reproducible build definition
│   ├── Makefile                       # Build commands
│   ├── configure_enclave.sh           # EC2 setup script
│   ├── expose_enclave.sh              # Port exposure script
│   ├── register_enclave.sh            # On-chain registration
│   ├── src/
│   │   ├── init/                      # Enclave init binary (boilerplate)
│   │   ├── aws/                       # AWS NSM integration (boilerplate)
│   │   ├── system/                    # System utilities (boilerplate)
│   │   └── nautilus-server/           # Mist backend adapted
│   │       ├── Cargo.toml
│   │       ├── run.sh                 # Enclave startup script
│   │       ├── traffic_forwarder.py   # Network proxy
│   │       └── src/
│   │           ├── main.rs
│   │           ├── lib.rs
│   │           ├── common.rs
│   │           └── apps/
│   │               └── mist-protocol/
│   │                   ├── mod.rs
│   │                   ├── intent_processor.rs
│   │                   ├── swap_executor.rs
│   │                   ├── seal_types.rs
│   │                   ├── seal_config.yaml
│   │                   └── allowed_endpoints.yaml  # NEW
│   └── move/
│       └── enclave/                   # Enclave registration contracts
```

---

## 2. Files to Create

### `allowed_endpoints.yaml`

Create at `nautilus/src/nautilus-server/src/apps/mist-protocol/allowed_endpoints.yaml`:

```yaml
# External endpoints the enclave needs access to
# The enclave has NO internet access - all traffic must be explicitly allowed
endpoints:
  - fullnode.testnet.sui.io
  - seal-key-server-testnet-1.mystenlabs.com
  - seal-key-server-testnet-2.mystenlabs.com
```

### Boilerplate Files (Copy from Example)

Copy these from `docs/examples/x-wallet-dev/nautilus-xwallet/`:

| File | Purpose |
|------|---------|
| `src/init/` | Init binary for enclave bootstrap |
| `src/aws/` | AWS NSM driver integration |
| `src/system/` | System utilities |
| `Containerfile` | Reproducible enclave build with StageX |
| `Makefile` | Build commands for EIF image |
| `traffic_forwarder.py` | Forwards traffic from enclave to vsock |
| `run.sh` | Init script inside enclave |
| `configure_enclave.sh` | EC2 setup script |
| `expose_enclave.sh` | Exposes enclave ports |
| `register_enclave.sh` | On-chain registration helper |

---

## 3. Backend Code Modifications

### `main.rs` Changes

```rust
// BEFORE (local development)
dotenv::dotenv().ok();
let backend_kp = load_backend_keypair()?;

// AFTER (enclave compatible)
// Remove dotenv - secrets come via VSOCK in enclave
// Private key passed as environment variable from host
let private_key = std::env::var("BACKEND_PRIVATE_KEY")
    .expect("BACKEND_PRIVATE_KEY must be set");
```

### Port Change

```rust
// Change from 3001 to 3000 (Nautilus standard)
let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
```

### Feature Flag for Local vs. Enclave

Add to `Cargo.toml`:
```toml
[features]
default = ["mist-protocol"]
mist-protocol = [...]
local-dev = []  # For local testing without enclave
```

---

## 4. Move Contracts for Enclave Registration

Create `move/enclave/sources/enclave.move`:

```move
module enclave::enclave {
    use sui::object::{Self, UID};
    use sui::tx_context::TxContext;

    /// Stores expected PCR values for attestation verification
    public struct EnclaveConfig has key {
        id: UID,
        pcr0: vector<u8>,
        pcr1: vector<u8>,
        pcr2: vector<u8>,
        config_version: u64,
    }

    /// Registered enclave instance with verified public key
    public struct Enclave has key {
        id: UID,
        public_key: vector<u8>,
        config_version: u64,
    }

    /// Update PCRs (admin only)
    public fun update_pcrs(
        config: &mut EnclaveConfig,
        cap: &EnclaveCap,
        pcr0: vector<u8>,
        pcr1: vector<u8>,
        pcr2: vector<u8>,
    ) { ... }

    /// Register enclave with attestation document
    public fun register_enclave(
        config: &EnclaveConfig,
        attestation: vector<u8>,
        ctx: &mut TxContext,
    ): Enclave { ... }
}
```

---

## 5. Deployment Steps

### Step 1: Local Build (Get PCRs)

```bash
cd nautilus
make ENCLAVE_APP=mist-protocol

# View the PCR values
cat out/nitro.pcrs
# Example output:
# 911c87d0abc8c9840a0810d57dfb718865f35dc42010a2d5b30e7840b03edeea83a26aad51593ade1e47ab6cced4653e PCR0
# 911c87d0abc8c9840a0810d57dfb718865f35dc42010a2d5b30e7840b03edeea83a26aad51593ade1e47ab6cced4653e PCR1
# 21b9efbc184807662e966d34f390821309eeac6802309798826296bf3e8bec7c10edb30948c90ba67310f7b964fc500a PCR2

# Save these for on-chain registration
export PCR0=911c87d0abc8c9840a0810d57dfb718865f35dc42010a2d5b30e7840b03edeea83a26aad51593ade1e47ab6cced4653e
export PCR1=...
export PCR2=...
```

### Step 2: AWS Setup

```bash
# Set required environment variables
export KEY_PAIR=<your-aws-key-pair-name>
export AWS_ACCESS_KEY_ID=<your-access-key>
export AWS_SECRET_ACCESS_KEY=<your-secret-key>
export AWS_SESSION_TOKEN=<your-session-token>  # If using temporary credentials

# Optional: Change region (default is us-east-1)
export REGION=us-east-1
export AMI_ID=ami-085ad6ae776d8f09c  # Amazon Linux for us-east-1

# Run the configuration script
sh configure_enclave.sh mist-protocol
```

The script will:
- Launch an m5.xlarge EC2 instance with Nitro Enclaves enabled
- Configure security groups (ports 22, 443, 3000)
- Set up vsock-proxy for allowed endpoints
- Generate `run.sh` and `expose_enclave.sh` with endpoint configuration

### Step 3: Connect to EC2 and Build

```bash
# SSH into the instance
ssh -i ~/.ssh/<your-key>.pem ec2-user@<PUBLIC_IP>

# Clone the repository
git clone <your-repo-url>
cd mist-protocol/nautilus

# Build the enclave image
make ENCLAVE_APP=mist-protocol

# Run the enclave
make run

# In another terminal, expose the enclave
sh expose_enclave.sh
```

### Step 4: Deploy Move Contracts

```bash
# Deploy enclave package
cd move/enclave
sui client publish

# Record the package ID
export ENCLAVE_PACKAGE_ID=0x...

# Deploy your app package (if separate)
cd ../mist-protocol
sui client publish
export APP_PACKAGE_ID=0x...
```

### Step 5: Register Enclave On-Chain

```bash
# Update PCRs in config
sui client call \
  --function update_pcrs \
  --module enclave \
  --package $ENCLAVE_PACKAGE_ID \
  --args $ENCLAVE_CONFIG_OBJECT_ID $CAP_OBJECT_ID 0x$PCR0 0x$PCR1 0x$PCR2

# Register enclave public key via attestation
sh register_enclave.sh \
  $ENCLAVE_PACKAGE_ID \
  $APP_PACKAGE_ID \
  $ENCLAVE_CONFIG_OBJECT_ID \
  http://<PUBLIC_IP>:3000 \
  mist_protocol \
  MIST_PROTOCOL
```

---

## 6. Key Differences: Local vs. Enclave

| Aspect | Local Development | Nautilus Enclave |
|--------|-------------------|------------------|
| **Private Key** | `.env` file | Passed via VSOCK from host |
| **NSM Attestation** | Mocked/returns error | Real AWS attestation |
| **Network** | Direct HTTPS | Traffic forwarded via vsock-proxy |
| **Port** | 3001 | 3000 (Nautilus standard) |
| **SEAL Servers** | Direct connection | Via allowed_endpoints.yaml |
| **Build** | `cargo build` | `make ENCLAVE_APP=...` (deterministic) |

---

## 7. tx-signer Consideration

The current architecture uses a separate `tx-signer` service due to fastcrypto version conflicts between SEAL SDK and sui-types.

**Options for Nautilus deployment:**

1. **Run tx-signer on host**: The enclave calls tx-signer on the host via VSOCK
2. **Integrate signing**: Since enclave is trusted, may be able to restructure dependencies
3. **Remote signing service**: Call an external signing endpoint (add to allowed_endpoints.yaml)

---

## 8. Testing

### Test Enclave Locally (Debug Mode)

```bash
# Build and run in debug mode (PCRs will be zeros)
make ENCLAVE_APP=mist-protocol
make run-debug  # Attaches console for logs
```

### Test Endpoints

```bash
# Health check
curl http://<PUBLIC_IP>:3000/health_check

# Get attestation (only works in real enclave)
curl http://<PUBLIC_IP>:3000/get_attestation
```

---

## 9. Troubleshooting

### Traffic Forwarder Errors
Ensure all required domains are in `allowed_endpoints.yaml`. Check with:
```bash
curl http://<PUBLIC_IP>:3000/health_check
# Returns status for each configured endpoint
```

### Docker Not Running
EC2 instance may still be initializing. Wait 2-3 minutes after launch.

### Cannot Connect to Enclave
```bash
# Check if enclave is running
nitro-cli describe-enclaves

# Reset and restart
sh reset_enclave.sh
make run
sh expose_enclave.sh
```

### PCR Mismatch
PCRs change if any code in `/src` changes. Rebuild and re-register after code changes:
```bash
make ENCLAVE_APP=mist-protocol
cat out/nitro.pcrs
# Update PCRs on-chain
```

---

## 10. Security Considerations

1. **PCR Verification**: On-chain contracts should verify attestation PCRs match registered values
2. **Key Management**: Backend private key should be stored in AWS Secrets Manager
3. **Network Isolation**: Enclave has no direct internet - only allowed endpoints work
4. **Reproducible Builds**: Anyone can verify the enclave binary matches source code

---

## References

- [Nautilus Documentation](docs/examples/x-wallet-dev/nautilus-xwallet/UsingNautilus.md)
- [AWS Nitro Enclaves Guide](https://docs.aws.amazon.com/enclaves/latest/user/)
- [SEAL SDK Documentation](https://github.com/MystenLabs/seal)
