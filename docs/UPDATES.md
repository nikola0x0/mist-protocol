# Mist Protocol - Planned Updates

This document outlines the upcoming updates and improvements needed for the Mist Protocol.

---

## 1. Enhanced Ticket Division for Amount Obfuscation

### Current Implementation

Currently, when a user deposits tokens, a single `EncryptedTicket` is created in their `VaultEntry`:

```move
// contracts/mist_protocol/sources/mist_protocol.move
public fun wrap_sui(
    vault: &mut VaultEntry,
    pool: &mut LiquidityPool,
    payment: Coin<SUI>,
    encrypted_amount: vector<u8>,
    ctx: &mut TxContext
)
```

When creating a swap intent, the user selects which tickets to use. If they have one large ticket (e.g., 100 SUI encrypted), the intent carries that single ticket, making it potentially mappable by observers tracking intent sizes.

### Problem

- **Pattern Analysis**: External observers can correlate deposit events with swap intents by matching the number and timing of tickets
- **Amount Inference**: If a user always deposits X SUI and creates intents with 1 ticket, observers can infer the approximate deposit amount range
- **Linkability**: Single large tickets create a fingerprint that persists through the swap process

### Proposed Solution: Automatic Ticket Sharding

#### 1.1 Deposit-Time Sharding

When a user deposits, automatically divide the amount into multiple smaller tickets:

```move
// Enhanced wrap function
public fun wrap_sui_sharded(
    vault: &mut VaultEntry,
    pool: &mut LiquidityPool,
    payment: Coin<SUI>,
    encrypted_amounts: vector<vector<u8>>,  // Multiple encrypted shards
    ctx: &mut TxContext
)
```

**Shard Generation Algorithm (Frontend)**:
```typescript
function generateShards(totalAmount: number, targetShards: number = 5): number[] {
    const shards: number[] = [];
    let remaining = totalAmount;

    for (let i = 0; i < targetShards - 1; i++) {
        // Random proportion between 10% and 30% of remaining
        const proportion = 0.1 + Math.random() * 0.2;
        const shard = Math.floor(remaining * proportion);
        shards.push(shard);
        remaining -= shard;
    }
    shards.push(remaining);  // Last shard gets the remainder

    // Shuffle to remove ordering patterns
    return shuffleArray(shards);
}
```

#### 1.2 Intent-Time Recomposition

When creating a swap intent, select a subset of tickets that sum to the desired amount:

```move
// Smart ticket selection for intent
public fun create_swap_intent_optimized(
    queue: &mut IntentQueue,
    vault: &mut VaultEntry,
    ticket_ids: vector<u64>,          // Selected subset
    encrypted_total: vector<u8>,       // SEAL-encrypted sum for verification
    token_out: String,
    min_output_amount: u64,
    deadline: u64,
    ctx: &mut TxContext
)
```

#### 1.3 SEAL Homomorphic Addition (Future Enhancement)

For true privacy, compute the sum within SEAL encryption without revealing individual amounts:

```
Challenge: SEAL threshold encryption doesn't natively support homomorphic operations.

Potential Solutions:
a) FHE Integration: Use BFV/CKKS scheme for encrypted arithmetic
b) MPC Summation: TEE computes sum after decrypting all tickets
c) ZK Proof: User proves sum correctness via ZK-SNARK
```

**Current Feasible Approach (MPC Summation)**:

The TEE already decrypts all locked tickets in `intent_processor.rs:120-180`. We can leverage this:

```rust
// backend/src/apps/mist-protocol/intent_processor.rs
async fn decrypt_and_sum_tickets(
    intent: &SwapIntent,
    seal_client: &SealClient,
) -> Result<u64> {
    let mut total = 0u64;

    for ticket in &intent.locked_tickets {
        let decrypted = seal_client.decrypt(&ticket.encrypted_amount).await?;
        total += decrypted.parse::<u64>()?;
    }

    Ok(total)  // Sum computed inside TEE, never exposed
}
```

### Implementation Steps

1. **Frontend Changes** (`frontend/lib/seal-vault.ts`):
   - Add `generateShards()` function
   - Modify `WrapCard.tsx` to encrypt multiple shards
   - Add configuration for shard count (user preference or automatic)

2. **Smart Contract Changes** (`contracts/mist_protocol/sources/mist_protocol.move`):
   - Add `wrap_sui_sharded()` function
   - Modify ticket creation loop
   - Add batch event emission

3. **Backend Changes** (`backend/src/apps/mist-protocol/intent_processor.rs`):
   - Already handles multiple tickets
   - Add logging for shard statistics

### Privacy Improvement Metrics

| Scenario | Before | After |
|----------|--------|-------|
| Single 100 SUI deposit | 1 ticket (identifiable) | 5-10 tickets (blended) |
| Swap 50 SUI | 1 ticket used | 2-5 tickets used |
| Observer correlation | High | Low (k-anonymity ≥ 5) |

---

## 2. Axum Server Pipeline Consolidation

### Current Architecture

The system currently has two separate flows:

```
Flow 1: Frontend → Axum Backend
- Endpoints: /health, /attestation, /test-seal
- Port: 3001
- File: backend/src/main.rs

Flow 2: Axum Backend → Cetus → Asset Return
- Components: intent_processor, swap_executor
- Reference: cetus-swap/backend/
- Port: 4001 (separate process)
```

### Problem

- Two separate Axum servers require manual coordination
- Configuration scattered across multiple files
- No unified request routing
- Cetus integration is in a separate crate

### Proposed Solution: Unified Pipeline

Merge all functionality into a single Axum application:

```
┌─────────────────────────────────────────────────────────────┐
│                     Unified Axum Server                      │
│                         (Port 3001)                          │
├──────────────────┬────────────────────┬────────────────────┤
│   Health Routes  │   SEAL Routes      │    Swap Routes     │
│   GET /health    │   POST /test-seal  │   GET /api/pools   │
│   GET /attest    │   POST /decrypt    │   POST /api/swap   │
└──────────────────┴────────────────────┴────────────────────┘
                              │
                              ▼
              ┌───────────────────────────────┐
              │     Background Task Manager    │
              ├───────────────────────────────┤
              │  IntentProcessor (5s polling) │
              │  SwapExecutor (Cetus calls)   │
              │  MetricsCollector (optional)  │
              └───────────────────────────────┘
```

### Implementation Plan

#### 2.1 Merge Cetus Module

Move `cetus-swap/backend/src/` into main backend:

```
backend/src/apps/
├── mist-protocol/
│   ├── mod.rs
│   ├── intent_processor.rs
│   ├── swap_executor.rs      # Enhanced with Cetus
│   ├── seal_encryption.rs
│   └── cetus/               # NEW: Merged from cetus-swap
│       ├── mod.rs
│       ├── api.rs           # Pool fetching
│       ├── transaction.rs   # PTB building
│       └── types.rs         # CetusPool, etc.
```

#### 2.2 Unified Router

```rust
// backend/src/main.rs

use axum::{Router, routing::{get, post}};

fn create_router(state: AppState) -> Router {
    Router::new()
        // Health & Attestation
        .route("/health", get(health_check))
        .route("/attestation", get(get_attestation))

        // SEAL Operations
        .route("/api/seal/test", post(test_seal_encryption))
        .route("/api/seal/decrypt", post(decrypt_amount))

        // Cetus Integration
        .route("/api/pools", get(fetch_cetus_pools))
        .route("/api/pool/:id", get(get_pool_info))
        .route("/api/quote", post(get_swap_quote))

        // Intent Management (optional direct endpoints)
        .route("/api/intents", get(list_pending_intents))
        .route("/api/intents/:id", get(get_intent_status))

        .with_state(state)
}
```

#### 2.3 Enhanced AppState

```rust
// backend/src/lib.rs

pub struct AppState {
    // Existing
    pub sui_client: Arc<SuiClient>,
    pub keypair: Arc<Keypair>,
    pub seal_config: Arc<SealConfig>,

    // New: Cetus integration
    pub cetus_client: Arc<CetusClient>,
    pub pool_cache: Arc<RwLock<HashMap<String, CetusPool>>>,

    // New: Metrics
    pub metrics: Arc<Metrics>,
}
```

#### 2.4 Complete Swap Pipeline

```rust
// backend/src/apps/mist-protocol/swap_executor.rs

pub async fn execute_real_swap(
    state: &AppState,
    intent: &DecryptedSwapIntent,
) -> Result<String> {
    // 1. Get best pool from Cetus
    let pool = state.cetus_client
        .get_best_pool(&intent.token_in, &intent.token_out)
        .await?;

    // 2. Calculate swap with slippage
    let quote = state.cetus_client
        .get_quote(&pool, intent.total_amount, intent.min_output_amount)
        .await?;

    // 3. Build PTB for Cetus swap
    let mut ptb = ProgrammableTransactionBuilder::new();

    // Split input coin from pool
    let input_coin = ptb.programmable_move_call(
        MIST_PACKAGE,
        "mist_protocol",
        "borrow_from_pool",
        vec![],
        vec![pool_arg, amount_arg],
    );

    // Execute Cetus swap
    let output_coin = build_cetus_swap_call(
        &mut ptb,
        &pool,
        input_coin,
        quote.min_output,
    )?;

    // Return output to pool and create encrypted ticket
    ptb.programmable_move_call(
        MIST_PACKAGE,
        "mist_protocol",
        "complete_swap",
        vec![],
        vec![pool_arg, output_coin, encrypted_output_arg],
    );

    // 4. Sign and execute
    let tx_bytes = ptb.finish();
    let signature = sign_via_signer(&tx_bytes).await?;
    let result = state.sui_client
        .execute_transaction_block(tx_bytes, signature)
        .await?;

    Ok(result.digest.to_string())
}
```

### Files to Modify

| File | Changes |
|------|---------|
| `backend/Cargo.toml` | Add cetus dependencies |
| `backend/src/main.rs` | Merge routers, add Cetus routes |
| `backend/src/lib.rs` | Extend AppState with CetusClient |
| `backend/src/apps/mist-protocol/swap_executor.rs` | Replace mock with real Cetus calls |
| `backend/src/apps/mist-protocol/cetus/` | NEW directory (move from cetus-swap) |

---

## 3. TEE Wallet as Pool - Privacy Analysis

### Current Design

```
User Wallet ─────────────────────────────────────────────→ VaultEntry
    │                                                           │
    ├─ Deposits SUI ─────→ LiquidityPool (Shared Object) ←──────┤
    │                            │                              │
    │                      TEE Backend                          │
    │                      (Separate Wallet)                    │
    │                            │                              │
    └─ Receives output ←─────────┴──────────────────────────────┘
```

**Key Properties**:
- LiquidityPool is a shared object owned by the protocol
- TEE has authority to execute swaps but doesn't hold funds
- User deposits go directly to the pool
- Separation between TEE authority and fund custody

### Proposed: TEE Wallet as Pool

```
User Wallet ──────────────────────────────────────────────→ VaultEntry
    │                                                           │
    ├─ Deposits SUI ─────→ TEE Wallet (Custodial) ←─────────────┤
    │                            │                              │
    │                      TEE Backend                          │
    │                      (Same Wallet = Pool)                 │
    │                            │                              │
    └─ Receives output ←─────────┘
```

### Privacy Impact Analysis

#### Positive Effects

| Aspect | Impact | Explanation |
|--------|--------|-------------|
| On-chain footprint | **Improved** | Fewer contract interactions, simpler tx flow |
| Mixing efficiency | **Improved** | All funds in one wallet, natural mixing |
| Deposit privacy | **Neutral** | Still encrypted with SEAL |

#### Negative Effects / Design Flaws

| Concern | Severity | Explanation |
|---------|----------|-------------|
| **Centralization Risk** | HIGH | Single wallet = single point of failure |
| **Regulatory Risk** | HIGH | TEE becomes custodian, triggers MSB/money transmitter laws |
| **Attestation Surface** | MEDIUM | Wallet key must be inside TEE, increases attack surface |
| **Key Rotation** | HIGH | How to rotate TEE wallet without losing funds? |
| **Audit Trail** | MEDIUM | All transactions from one address = easier chain analysis |

### Detailed Concerns

#### 3.1 Custodial Classification

If the TEE wallet holds user funds directly:

```
Legal Status:
- US: Money Services Business (MSB) under FinCEN
- EU: Crypto Asset Service Provider (CASP) under MiCA
- Requirement: KYC/AML compliance

Current Design (Non-Custodial):
- Smart contract holds funds (code is law)
- TEE only has execution authority
- Users maintain self-custody via encrypted tickets
```

#### 3.2 Key Management Vulnerability

```
Current:
  TEE Key (Ed25519) ─→ Signs transactions
                    └─→ No funds at risk if compromised
                         (only execution authority)

Proposed:
  TEE Key (Ed25519) ─→ Signs transactions
                    └─→ ALL FUNDS at risk if compromised
                         (custody + authority combined)
```

#### 3.3 Chain Analysis

```
Current:
  Deposit:    User A → Pool (shared object)
  Swap:       TEE executes (no visible transfer)
  Withdrawal: Pool → User B

Proposed:
  Deposit:    User A → TEE Wallet (0x9bf6...)
  Swap:       TEE Wallet internal
  Withdrawal: TEE Wallet (0x9bf6...) → User B

Analysis:
  - All deposits to same address = easily trackable
  - Balance visible on chain (though amounts encrypted)
  - Transaction graph reveals total pool volume
```

### Recommendation

**Do NOT use TEE wallet as pool.** The current design is superior because:

1. **Separation of Concerns**: Authority (TEE) vs Custody (Smart Contract)
2. **Legal Safety**: Non-custodial by design
3. **Security**: Compromised TEE can't steal funds, only delay execution
4. **Decentralization Path**: Can add multiple TEE backends without changing custody

### Alternative Enhancement

Instead of TEE-as-pool, improve privacy via:

```
Multi-Pool Sharding:
  Pool_A (0x111...) ← 33% of deposits
  Pool_B (0x222...) ← 33% of deposits
  Pool_C (0x333...) ← 34% of deposits

Benefits:
- k-anonymity across pools
- No single balance to track
- TEE rotates between pools
```

---

## 4. AWS EC2 Deployment with Nautilus Attestation

### Overview

Deploy the Mist Protocol backend to AWS EC2 with Nitro Enclaves for TEE attestation.

### Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                        AWS EC2 Instance                          │
│                     (c5.xlarge or larger)                        │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  ┌────────────────────┐     ┌────────────────────────────────┐ │
│  │   Parent Instance   │     │      Nitro Enclave             │ │
│  │                     │     │                                │ │
│  │  ┌───────────────┐ │     │  ┌────────────────────────┐   │ │
│  │  │  tx-signer    │◄├─────┤──►│   Mist Backend         │   │ │
│  │  │  (Port 4000)  │ │vsock│  │   (SEAL + Intent Proc) │   │ │
│  │  └───────────────┘ │     │  │                        │   │ │
│  │                     │     │  │  Keypair (sealed)      │   │ │
│  │  ┌───────────────┐ │     │  │  SEAL decryption       │   │ │
│  │  │  nginx proxy  │ │     │  │  Swap execution        │   │ │
│  │  │  (Port 443)   │ │     │  └────────────────────────┘   │ │
│  │  └───────────────┘ │     │                                │ │
│  │         │          │     │  Attestation via NSM API       │ │
│  └─────────┼──────────┘     └────────────────────────────────┘ │
│            │                                                     │
└────────────┼─────────────────────────────────────────────────────┘
             │
             ▼
        Internet
```

### 4.1 Prerequisites

```bash
# EC2 Instance Requirements
- Instance Type: c5.xlarge or c5.2xlarge (enclave-enabled)
- AMI: Amazon Linux 2023 or Ubuntu 24.04
- Enclave Support: Enabled in instance settings
- Memory: Allocate 4GB+ to enclave

# Install Nitro CLI
sudo amazon-linux-extras install aws-nitro-enclaves-cli -y
sudo systemctl enable nitro-enclaves-allocator
sudo systemctl start nitro-enclaves-allocator
```

### 4.2 Enclave Configuration

```yaml
# enclave-config.yaml
---
# Memory and CPU allocation for enclave
memory_mib: 4096
cpu_count: 2

# Enable debug mode during development (disable in production!)
debug_mode: false
```

### 4.3 Dockerfile for Enclave

```dockerfile
# Dockerfile.enclave
FROM rust:1.75-slim as builder

WORKDIR /app
COPY backend/ .
COPY Cargo.lock .

# Build with enclave-specific features
RUN cargo build --release --features nitro-enclave

FROM debian:bookworm-slim

# Install minimal runtime
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/mist-backend /usr/local/bin/

# Entrypoint script for vsock communication
COPY enclave-entrypoint.sh /entrypoint.sh
RUN chmod +x /entrypoint.sh

ENTRYPOINT ["/entrypoint.sh"]
```

### 4.4 Enclave Entry Script

```bash
#!/bin/bash
# enclave-entrypoint.sh

# Initialize NSM (Nitro Secure Module)
export NSM_PATH=/dev/nsm

# Load sealed keypair from attestation
if [ -f /sealed/keypair.enc ]; then
    # Decrypt keypair using enclave-specific key
    /usr/local/bin/unseal-key /sealed/keypair.enc > /tmp/keypair
    export BACKEND_PRIVATE_KEY=$(cat /tmp/keypair)
    rm /tmp/keypair
fi

# Start backend with vsock listener
exec /usr/local/bin/mist-backend \
    --listen-vsock 5000 \
    --network testnet
```

### 4.5 Build and Deploy Enclave

```bash
# Build enclave image
nitro-cli build-enclave \
    --docker-uri mist-backend:latest \
    --output-file mist-enclave.eif

# Run enclave
nitro-cli run-enclave \
    --eif-path mist-enclave.eif \
    --memory 4096 \
    --cpu-count 2 \
    --enclave-cid 16

# Verify attestation
nitro-cli describe-enclaves
```

### 4.6 Attestation Flow

```rust
// backend/src/common.rs - Enhanced for Nitro

use aws_nitro_enclaves_nsm_api::{Request, Response};

pub async fn get_attestation(
    State(state): State<AppState>,
) -> Result<Json<AttestationResponse>, AppError> {
    // Build attestation request with public key
    let public_key = state.keypair.public_key().as_bytes().to_vec();

    let request = Request::Attestation {
        user_data: Some(b"mist-protocol-v1".to_vec()),
        nonce: Some(generate_nonce()),
        public_key: Some(public_key),
    };

    // Call NSM API
    let fd = std::fs::File::open("/dev/nsm")?;
    let response = nsm_process_request(fd, request)?;

    match response {
        Response::Attestation { document } => {
            // Document contains:
            // - PCRs (Platform Configuration Registers)
            // - Module ID
            // - Digest of enclave image
            // - Timestamp
            // - Public key (embedded)
            // - Certificate chain

            Ok(Json(AttestationResponse {
                document: hex::encode(&document),
                public_key: hex::encode(state.keypair.public_key().as_bytes()),
            }))
        }
        _ => Err(AppError::AttestationFailed),
    }
}
```

### 4.7 Attestation Verification (Client-Side)

```typescript
// frontend/lib/attestation.ts

import { verify } from '@aws-sdk/client-nitro-enclaves-attestation';

interface AttestationDocument {
    moduleId: string;
    digest: string;
    timestamp: number;
    pcrs: { [key: number]: string };
    publicKey: string;
    certificate: string;
}

export async function verifyAttestation(
    documentHex: string,
    expectedPcrs: { [key: number]: string }
): Promise<{ valid: boolean; publicKey: string }> {
    const document = Buffer.from(documentHex, 'hex');

    // Verify certificate chain (AWS root CA)
    const verified = await verify(document, {
        rootCertificates: [AWS_NITRO_ROOT_CA],
    });

    if (!verified) {
        return { valid: false, publicKey: '' };
    }

    // Parse CBOR-encoded attestation
    const attestation: AttestationDocument = cbor.decode(document);

    // Verify PCR values match expected (code integrity)
    for (const [pcr, expected] of Object.entries(expectedPcrs)) {
        if (attestation.pcrs[parseInt(pcr)] !== expected) {
            console.error(`PCR${pcr} mismatch`);
            return { valid: false, publicKey: '' };
        }
    }

    return {
        valid: true,
        publicKey: attestation.publicKey,
    };
}
```

### 4.8 Systemd Services

```ini
# /etc/systemd/system/mist-enclave.service
[Unit]
Description=Mist Protocol Enclave
After=nitro-enclaves-allocator.service
Requires=nitro-enclaves-allocator.service

[Service]
Type=simple
ExecStart=/usr/bin/nitro-cli run-enclave \
    --eif-path /opt/mist/mist-enclave.eif \
    --memory 4096 \
    --cpu-count 2 \
    --enclave-cid 16
ExecStop=/usr/bin/nitro-cli terminate-enclave --all
Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
```

```ini
# /etc/systemd/system/mist-vsock-proxy.service
[Unit]
Description=Mist Protocol vsock Proxy
After=mist-enclave.service
Requires=mist-enclave.service

[Service]
Type=simple
ExecStart=/opt/mist/vsock-proxy \
    --vsock-cid 16 \
    --vsock-port 5000 \
    --tcp-port 3001
Restart=always
RestartSec=5

[Install]
WantedBy=multi-user.target
```

### 4.9 Deployment Script

```bash
#!/bin/bash
# deploy.sh

set -e

EC2_HOST="ec2-xx-xx-xx-xx.compute.amazonaws.com"
SSH_KEY="~/.ssh/mist-protocol.pem"

echo "Building enclave image..."
docker build -f Dockerfile.enclave -t mist-backend:latest ./backend

nitro-cli build-enclave \
    --docker-uri mist-backend:latest \
    --output-file mist-enclave.eif

echo "Uploading to EC2..."
scp -i $SSH_KEY mist-enclave.eif ec2-user@$EC2_HOST:/opt/mist/

echo "Deploying enclave..."
ssh -i $SSH_KEY ec2-user@$EC2_HOST << 'EOF'
    sudo systemctl stop mist-enclave || true
    sudo systemctl stop mist-vsock-proxy || true

    # Terminate existing enclaves
    sudo nitro-cli terminate-enclave --all || true

    # Start new enclave
    sudo systemctl start mist-enclave
    sleep 5

    # Verify enclave is running
    nitro-cli describe-enclaves

    # Start proxy
    sudo systemctl start mist-vsock-proxy

    echo "Deployment complete!"
EOF

echo "Verifying attestation..."
curl -s https://$EC2_HOST/attestation | jq .
```

### 4.10 Security Hardening

```bash
# Security Group Rules
- Inbound 443 (HTTPS): 0.0.0.0/0
- Inbound 22 (SSH): Your IP only
- Outbound 443 (HTTPS): 0.0.0.0/0 (for Sui RPC, SEAL servers)

# Enclave-specific
- tx-signer runs in parent (not enclave) on localhost:4000
- Enclave communicates via vsock only
- No network access from enclave directly
- All external calls proxied through parent
```

### 4.11 Monitoring

```yaml
# prometheus.yml (on parent instance)
scrape_configs:
  - job_name: 'mist-backend'
    static_configs:
      - targets: ['localhost:3001']
    metrics_path: '/metrics'

  - job_name: 'enclave-health'
    static_configs:
      - targets: ['localhost:9100']
    metrics_path: '/enclave/health'
```

### Deployment Checklist

- [ ] EC2 instance with Nitro Enclave support enabled
- [ ] Security groups configured (443 inbound, 22 restricted)
- [ ] Nitro CLI installed and allocator running
- [ ] Enclave EIF built and uploaded
- [ ] Systemd services configured
- [ ] vsock proxy running
- [ ] Attestation endpoint returning valid document
- [ ] PCR values documented for client verification
- [ ] SEAL key servers reachable from parent instance
- [ ] tx-signer service running on localhost:4000
- [ ] SSL certificate configured (Let's Encrypt or ACM)
- [ ] Monitoring dashboards set up
- [ ] Alerting configured for enclave restarts

---

## Summary

| Update | Priority | Complexity | Privacy Impact |
|--------|----------|------------|----------------|
| 1. Ticket Sharding | High | Medium | +++ |
| 2. Axum Consolidation | High | Low | Neutral |
| 3. TEE as Pool | N/A | N/A | --- (Not recommended) |
| 4. AWS Deployment | High | High | +++ (Attestation) |

### Recommended Order

1. **Axum Consolidation** - Unblock other work
2. **AWS Deployment** - Get production-ready TEE
3. **Ticket Sharding** - Enhance privacy
4. (Skip TEE as Pool - design flaw)
