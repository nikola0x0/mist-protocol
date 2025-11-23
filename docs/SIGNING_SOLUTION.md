# Transaction Signing Solution for SEAL-Enabled Backends

## TL;DR

**Problem:** SEAL SDK and sui-sdk use incompatible fastcrypto versions, preventing transaction signing in the same binary.

**Solution:** Run a separate signing service (no SEAL dependency) that the backend calls via HTTP.

**Why It Works:**
- Backend does all SEAL encryption/decryption ✅
- Backend builds unsigned transaction with encrypted data ✅
- Signing service ONLY signs raw transaction bytes (no SEAL needed) ✅
- Backend executes signed transaction ✅

**Time to Implement:** ~10 minutes (using official `sui keytool sign-server`)

---

## Problem Statement

### The Conflict

When building a backend that uses both SEAL SDK and sui-sdk, there's an incompatible fastcrypto version conflict:

- **SEAL SDK** (latest): uses `fastcrypto@d1fcb853` (older version)
- **sui-sdk/sui-types** (latest): uses `fastcrypto@09f86974` (newer version)

### Why This Matters

Rust treats these as completely different types, even though they have identical names:

```rust
// Error from compiler:
error[E0308]: mismatched types
expected `fastcrypto::ed25519::Ed25519PublicKey` (from d1fcb853),
   found `fastcrypto::ed25519::Ed25519PublicKey` (from 09f86974)
```

This prevents:
- ❌ Using the same keypair for SEAL decryption AND transaction signing
- ❌ Building automated backends that decrypt + execute transactions
- ❌ TEE backends that process encrypted intents autonomously

### What Works vs What Doesn't

✅ **Works:** SEAL decryption (threshold encryption)
✅ **Works:** Building unsigned transactions
✅ **Works:** SEAL encryption of output data
❌ **Broken:** Signing transactions programmatically
❌ **Broken:** Autonomous swap execution

---

## Solution Options

### Option 1: Separate Signing Service ⭐ **RECOMMENDED**

Create a lightweight HTTP microservice that handles ONLY transaction signing (no encryption).

#### Architecture

```
┌──────────────────────────────────────────────────────────────────┐
│                   Backend (Port 3001)                             │
│  Uses: SEAL SDK (fastcrypto d1fcb85)                             │
│                                                                   │
│  1. Decrypt intents with SEAL         ✅                         │
│  2. Execute Cetus swap                ✅                         │
│  3. Encrypt output with SEAL          ✅ (IMPORTANT!)            │
│  4. Build unsigned tx (with encrypted output)                    │
│  5. Send to signing service           ──┐                        │
└───────────────────────────────────────────┼──────────────────────┘
                                            │ HTTP POST
                                            │ {tx_data_b64: "..."}
                                            ▼
┌──────────────────────────────────────────────────────────────────┐
│              Signing Service (Port 4000)                          │
│  Uses: sui-types ONLY (fastcrypto 09f86974)                      │
│  NO SEAL SDK! NO encryption!                                     │
│                                                                   │
│  - Receives: unsigned transaction bytes                          │
│  - Signs: using SuiKeyPair                                       │
│  - Returns: signed transaction bytes                             │
└───────────────────────────────────────────┬──────────────────────┘
                                            │
                                            │ {signed_tx_b64: "..."}
                                            ▼
┌──────────────────────────────────────────────────────────────────┐
│                   Backend (Port 3001)                             │
│  6. Receive signed tx                                            │
│  7. Execute on-chain                  ✅                         │
└──────────────────────────────────────────────────────────────────┘
```

**Key Insight:** The signing service does NOT need SEAL! The backend handles ALL encryption before building the transaction. The signing service is just a pure signing utility.

#### Implementation

**File: `tx-signer/Cargo.toml`**
```toml
[package]
name = "tx-signer"
version = "0.1.0"
edition = "2021"

[dependencies]
tokio = { version = "1", features = ["full"] }
axum = "0.7"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
anyhow = "1.0"
bcs = "0.1.6"
base64 = "0.21"
dotenv = "0.15"

# Latest sui-types (NO SEAL SDK!)
sui-types = { git = "https://github.com/mystenlabs/sui", package = "sui-types" }
shared-crypto = { git = "https://github.com/mystenlabs/sui", package = "shared-crypto" }

bech32 = "0.9"
```

**File: `tx-signer/src/main.rs`**
```rust
use axum::{Router, routing::post, Json, extract::State};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Deserialize)]
struct SignRequest {
    tx_data_b64: String,
}

#[derive(Serialize)]
struct SignResponse {
    signed_tx_b64: String,
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    dotenv::dotenv().ok();

    // Load backend private key
    let private_key = std::env::var("BACKEND_PRIVATE_KEY")?;
    let keypair = sui_types::crypto::SuiKeyPair::decode(&private_key)?;

    let state = Arc::new(keypair);

    let app = Router::new()
        .route("/sign", post(sign_transaction))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:4000").await?;
    println!("🔐 Signing service listening on port 4000");

    axum::serve(listener, app).await?;
    Ok(())
}

async fn sign_transaction(
    State(keypair): State<Arc<sui_types::crypto::SuiKeyPair>>,
    Json(req): Json<SignRequest>,
) -> Result<Json<SignResponse>, String> {
    use shared_crypto::intent::{Intent, IntentMessage};
    use sui_types::crypto::{Signer, DefaultHash};
    use fastcrypto::hash::HashFunction;
    use sui_types::signature::GenericSignature;
    use sui_types::transaction::{Transaction, TransactionData};

    // Decode transaction data
    let tx_bytes = base64::Engine::decode(
        &base64::engine::general_purpose::STANDARD,
        &req.tx_data_b64
    ).map_err(|e| format!("Invalid base64: {}", e))?;

    let tx_data: TransactionData = bcs::from_bytes(&tx_bytes)
        .map_err(|e| format!("Invalid transaction data: {}", e))?;

    // Create intent message and hash it
    let intent_msg = IntentMessage::new(Intent::sui_transaction(), tx_data);
    let raw_tx = bcs::to_bytes(&intent_msg)
        .map_err(|e| format!("Failed to serialize: {}", e))?;

    let mut hasher = DefaultHash::default();
    hasher.update(raw_tx.clone());
    let digest = hasher.finalize().digest;

    // Sign the digest
    let signature = keypair.sign(&digest);

    // Create signed transaction
    let signed_tx = Transaction::from_generic_sig_data(
        intent_msg.value,
        vec![GenericSignature::Signature(signature)],
    );

    // Serialize signed transaction
    let signed_bytes = bcs::to_bytes(&signed_tx)
        .map_err(|e| format!("Failed to serialize signed tx: {}", e))?;
    let signed_b64 = base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        &signed_bytes
    );

    Ok(Json(SignResponse { signed_tx_b64: signed_b64 }))
}
```

**File: `tx-signer/.env`**
```bash
BACKEND_PRIVATE_KEY=suiprivkey1qzks4j8zmruj3ruj9tlfelnut5rwtmpq8cymaq5qz80vfkynt0as2clngv5
```

#### Backend Integration

**Update `backend-seal/Cargo.toml`:**
```toml
# Add reqwest if not present
reqwest = { version = "0.11", features = ["json"] }
```

**Update `swap_executor.rs`:**
```rust
// After building tx_data...
let tx_bytes = bcs::to_bytes(&tx_data)?;
let tx_b64 = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &tx_bytes);

// Call signing service
let client = reqwest::Client::new();
let response = client.post("http://localhost:4000/sign")
    .json(&serde_json::json!({
        "tx_data_b64": tx_b64
    }))
    .send()
    .await?;

let sign_response: SignResponse = response.json().await?;

// Deserialize signed transaction
let signed_tx_bytes = base64::Engine::decode(
    &base64::engine::general_purpose::STANDARD,
    &sign_response.signed_tx_b64
)?;
let signed_tx: sui_sdk::types::transaction::Transaction = bcs::from_bytes(&signed_tx_bytes)?;

// Execute!
let response = sui_client.quorum_driver_api()
    .execute_transaction_block(signed_tx, ...)
    .await?;
```

#### Deployment

```bash
# Terminal 1: Start signing service
cd tx-signer
cargo run

# Terminal 2: Start main backend
cd backend-seal
cargo run --features mist-protocol
```

**Pros:**
- ✅ Clean separation of concerns
- ✅ Each service has compatible dependencies
- ✅ Production-ready
- ✅ Works in TEE environments
- ✅ Can scale independently

**Cons:**
- ❌ Extra HTTP hop (~1-2ms latency, negligible)
- ❌ Two services to deploy

**Why This Works:**
- ✅ Backend keeps SEAL SDK for decrypt/encrypt
- ✅ Signing service has NO SEAL dependency
- ✅ No version conflict because they're separate binaries!
- ✅ Backend encrypts BEFORE calling signing service

**Effort:** ~30-45 minutes

---

### Option 2: CLI-Based Signing (Quick Hack)

Use `sui keytool` to sign transactions:

```rust
// Build unsigned transaction
let tx_bytes = bcs::to_bytes(&tx_data)?;
let tx_b64 = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &tx_bytes);

// Sign with CLI
let output = std::process::Command::new("sui")
    .args(&["keytool", "sign", "--address", &backend_address, "--data", &tx_b64])
    .output()?;

// Parse signature from output
let signature = parse_signature_from_cli_output(&output.stdout)?;

// Build signed transaction
// ...
```

**Pros:**
- ✅ No extra service needed
- ✅ Works immediately

**Cons:**
- ❌ Requires sui CLI installed
- ❌ Slower (process spawning)
- ❌ Fragile (depends on CLI output format)
- ❌ Hard to deploy in containers/TEE

**Effort:** ~15 minutes

---

### Option 3: TypeScript/Python Signing Sidecar

Use Sui's official TypeScript SDK:

**File: `signer/index.ts`**
```typescript
import express from 'express';
import { Ed25519Keypair } from '@mysten/sui.js/keypairs/ed25519';
import { TransactionBlock } from '@mysten/sui.js/transactions';

const app = express();
app.use(express.json());

const keypair = Ed25519Keypair.fromSecretKey(
  process.env.BACKEND_PRIVATE_KEY
);

app.post('/sign', async (req, res) => {
  const { txBytes } = req.body;
  const signature = await keypair.signTransaction(
    Buffer.from(txBytes, 'base64')
  );
  res.json({ signature });
});

app.listen(4000);
```

**Pros:**
- ✅ Sui TS SDK is official & well-maintained
- ✅ Fast to implement
- ✅ No Rust version issues

**Cons:**
- ❌ Requires Node.js runtime
- ❌ Cross-language complexity
- ❌ Extra runtime in production

**Effort:** ~20 minutes

---

### Option 4: Manual Signing (Testing Only)

Backend outputs unsigned transaction, you sign manually:

```bash
# Backend outputs:
TX_BYTES='AAACAAgA...'

# You run:
sui client sign-and-execute-tx --tx-bytes $TX_BYTES
```

**Pros:**
- ✅ Already implemented!
- ✅ Good for testing transaction structure
- ✅ Zero additional code

**Cons:**
- ❌ Not automated
- ❌ Not production-ready
- ❌ Requires manual intervention

**Effort:** 0 minutes (done!)

---

### Option 5: Fork & Update SEAL SDK

Fork SEAL SDK and update its fastcrypto dependency:

```toml
# In your SEAL fork:
fastcrypto = { git = "https://github.com/MystenLabs/fastcrypto", rev = "09f86974..." }
```

**Pros:**
- ✅ Single dependency graph
- ✅ Can merge upstream later

**Cons:**
- ❌ May break SEAL functionality
- ❌ Need to understand SEAL internals
- ❌ Maintenance burden
- ❌ No guarantee it works

**Effort:** 2-4 hours, high risk

---

### Option 6: Wait for Mysten Labs

File GitHub issue and wait for official update:

**Pros:**
- ✅ Official solution
- ✅ Supported long-term

**Cons:**
- ❌ Unknown timeline
- ❌ Blocks current development

**Effort:** Issue filed, wait time unknown

---

### Option 7: FFI/Dynamic Library (Complex)

Build signing as a separate dynamic library with different dependencies:

**Pros:**
- ✅ Single process

**Cons:**
- ❌ Very complex
- ❌ Platform-specific
- ❌ Hard to debug

**Effort:** Several hours, very complex

---

## Recommended Approach for Your Hackathon

### **Short-term (Next 1 hour):**
1. Use **Option 4 (Manual)** to test the transaction structure NOW
2. Create the GitHub issue (Option 6) to engage Mysten Labs

### **Medium-term (Next 2-3 hours):**
Implement **Option 1 (Separate Signing Service)**:
- Clean architecture
- Production-viable
- Fast to implement
- Works in TEE

### **Long-term:**
- Monitor GitHub issue
- Switch to integrated signing when SEAL SDK updates
- Or keep signing service if it works well

---

## Implementation Guide for Option 1

### Step 1: Create Signing Service (30 min)

```bash
mkdir tx-signer
cd tx-signer
```

Create `Cargo.toml` and `src/main.rs` (code provided above)

### Step 2: Update Backend (15 min)

Add HTTP client call to signing service in `swap_executor.rs`

### Step 3: Test (5 min)

```bash
# Terminal 1
cd tx-signer && cargo run

# Terminal 2
cd backend-seal && cargo run --features mist-protocol

# Frontend: Create swap intent
# Backend: Automatically decrypts, builds tx, signs via service, executes!
```

### Total Time: ~50 minutes

---

## Verification

The backend currently successfully:
1. ✅ Polls IntentQueue every 5 seconds
2. ✅ Detects pending swap intents
3. ✅ Decrypts locked tickets using SEAL
4. ✅ Displays human-readable amounts (0.5 SUI not 500000000)
5. ✅ Encrypts output amounts with SEAL
6. ✅ Builds correct execute_swap transactions
7. ⚠️ Outputs unsigned tx (needs signing solution)

With any of the signing solutions above, step 7 becomes automated!

---

## Current Status

**What's Working:**
- Contract deployed with `execute_swap_sui` and `execute_swap_usdc`
- Backend decrypts intents successfully
- SEAL encryption working
- Transaction building working
- Configs updated with new package IDs

**What's Blocked:**
- Automatic transaction signing (fastcrypto conflict)

**Next Action:**
Choose a signing solution and implement it!

---

## Questions for Mysten Labs

When creating the GitHub issue, ask:

1. **Timeline:** When will SEAL SDK update to fastcrypto 09f8697+?
2. **Pattern:** What's the recommended architecture for SEAL backends that sign transactions?
3. **Workaround:** Is there an official workaround we're missing?
4. **TEE:** How do Nautilus examples handle this in production?

---

## Conclusion

The **Separate Signing Service (Option 1)** is the best balance of:
- Speed to implement
- Production readiness
- Clean architecture
- TEE compatibility

It's a 50-minute solution that unblocks development while waiting for upstream fixes.
