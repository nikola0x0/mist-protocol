# Mist Protocol: Complete System Design

**Private Intent-Based DeFi on Sui with Verifiable TEE Execution**

Built with: Nautilus • Seal • Walrus • Cetus • Sui

---

## Table of Contents

1. [Executive Summary](#executive-summary)
2. [Architecture Overview](#architecture-overview)
3. [Core Components](#core-components)
4. [Trust Model](#trust-model)
5. [Complete User Flow](#complete-user-flow)
6. [Technical Implementation](#technical-implementation)
7. [Security Guarantees](#security-guarantees)
8. [Comparison vs Encifher](#comparison-vs-encifher)

---

## Executive Summary

### What is Mist Protocol?

Mist Protocol is a **privacy-preserving intent-based DeFi protocol** on Sui that allows users to submit encrypted trading intents that are executed by a verifiable TEE (Trusted Execution Environment) without revealing transaction details publicly.

### Key Innovation

**Verifiable Privacy Through Separation of Concerns:**

- **Seal**: User-controlled threshold encryption (2-of-3 key servers)
- **Nautilus**: Verifiable TEE computation (self-managed, reproducible)
- **Allowlist**: Transparent on-chain access control
- **Walrus**: Cost-efficient decentralized storage

### The Problem We Solve

Current privacy solutions like Encifher force users to trust a black-box centralized gateway. Mist Protocol provides:

✅ **Verifiable computation** (reproducible builds, cryptographic attestation)
✅ **Decentralized encryption** (threshold cryptography)
✅ **Transparent execution** (anyone can verify)
✅ **Self-managed infrastructure** (run your own TEE)

---

## Architecture Overview

### High-Level System Diagram

```
┌─────────────────────────────────────────────────────────────────┐
│                        USER (Frontend)                           │
│                                                                  │
│  1. Create intent: "Swap 100 USDC for SUI on Cetus"            │
│  2. Encrypt with Seal (2-of-3 threshold + allowlist policy)    │
│  3. Submit encrypted intent to blockchain                       │
│  4. Receive signed result + attestation                         │
│  5. Verify on-chain                                             │
└────┬──────────────────────────────────────────────┬────────────┘
     │                                               │
     │ Encrypted intent                              │ Verify result
     │                                               │
┌────▼───────────────────────────────────────────────▼────────────┐
│                    SUI BLOCKCHAIN (Testnet)                      │
│                                                                  │
│  Smart Contracts:                                               │
│  ┌────────────────────────────────────────────────────────┐    │
│  │  Allowlist Contract                                     │    │
│  │  - Nautilus pubkey: 0xf343dae1... ✅                   │    │
│  │  - User address: 0x123... ✅                            │    │
│  │  - Admin cap for management                             │    │
│  └────────────────────────────────────────────────────────┘    │
│                                                                  │
│  ┌────────────────────────────────────────────────────────┐    │
│  │  Escrow Contract                                        │    │
│  │  - Holds user funds                                     │    │
│  │  - Only Nautilus can execute swaps                      │    │
│  │  - Encrypted amounts stored (Seal)                      │    │
│  └────────────────────────────────────────────────────────┘    │
│                                                                  │
│  ┌────────────────────────────────────────────────────────┐    │
│  │  Nautilus Verifier Contract                             │    │
│  │  - Registered PCR values (reproducible builds)          │    │
│  │  - Nautilus public key                                  │    │
│  │  - Attestation verification logic                       │    │
│  └────────────────────────────────────────────────────────┘    │
│                                                                  │
│  ┌────────────────────────────────────────────────────────┐    │
│  │  Intent Object (Shared)                                 │    │
│  │  - encrypted_data: 0xDEADBEEF... (Seal encrypted)      │    │
│  │  - key_id: [pkg]::[allowlist_id][nonce]               │    │
│  │  - user: 0x123...                                       │    │
│  │  - status: pending/executed/failed                      │    │
│  └────────────────────────────────────────────────────────┘    │
└────┬────────────────────────────────────────────────────────────┘
     │
     │ Nautilus reads intent & calls seal_approve
     │
┌────▼────────────────────────────────────────────────────────────┐
│         AWS EC2 INSTANCE (Parent - Untrusted Helper)            │
│                                                                  │
│  Role: Networking proxy ONLY                                    │
│                                                                  │
│  Can do:                                                        │
│  ✅ Forward HTTP requests to external APIs                      │
│  ✅ Submit transactions to Sui blockchain                       │
│  ✅ Proxy Seal server requests                                  │
│  ✅ Call Cetus DEX API                                          │
│                                                                  │
│  Cannot do:                                                     │
│  ❌ Access enclave memory                                       │
│  ❌ Access private keys                                         │
│  ❌ See decrypted intents                                       │
│  ❌ Forge signatures or attestations                            │
│  ❌ Modify enclave code                                         │
│                                                                  │
│  ┌──────────────────────────────────────────────────────┐      │
│  │  Traffic Forwarder                                    │      │
│  │  Configured domains (allowed_endpoints.yaml):        │      │
│  │  - api.cetus.zone                                     │      │
│  │  - seal.mystenlabs.com                                │      │
│  │  - fullnode.testnet.sui.io                            │      │
│  │  - aggregator.walrus-testnet.mystenlabs.com          │      │
│  └──────────────────────────────────────────────────────┘      │
│                                                                  │
│         ↕ VSOCK (Virtual Socket - Encrypted Channel)            │
│                                                                  │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │    AWS NITRO ENCLAVE (Trusted Execution Environment)     │  │
│  │                                                           │  │
│  │  Hardware Isolated Environment:                          │  │
│  │  - No SSH access                                         │  │
│  │  - No direct internet                                    │  │
│  │  - No persistent storage                                 │  │
│  │  - Memory encrypted                                      │  │
│  │                                                           │  │
│  │  ┌────────────────────────────────────────────────┐     │  │
│  │  │  Nautilus Server (Rust + Axum)                 │     │  │
│  │  │                                                 │     │  │
│  │  │  Ephemeral Key Pair (Generated on boot):       │     │  │
│  │  │  - Private key: 0xABC123... (NEVER leaves)     │     │  │
│  │  │  - Public key: 0xf343dae1... (registered)      │     │  │
│  │  │                                                 │     │  │
│  │  │  Endpoints:                                     │     │  │
│  │  │  - /health_check                                │     │  │
│  │  │  - /get_attestation                             │     │  │
│  │  │  - /process_intent                              │     │  │
│  │  │                                                 │     │  │
│  │  │  Process Flow:                                  │     │  │
│  │  │  1. Read encrypted intent from blockchain      │     │  │
│  │  │  2. Call seal_approve via parent               │     │  │
│  │  │  3. Request decryption keys from Seal servers  │     │  │
│  │  │  4. Decrypt intent INSIDE enclave              │     │  │
│  │  │  5. Validate intent structure                  │     │  │
│  │  │  6. Execute swap on Cetus via parent           │     │  │
│  │  │  7. Verify execution result                    │     │  │
│  │  │  8. Sign result with private key               │     │  │
│  │  │  9. Generate NSM attestation                   │     │  │
│  │  │  10. Return signed result                      │     │  │
│  │  └────────────────────────────────────────────────┘     │  │
│  │                                                           │  │
│  │  ┌────────────────────────────────────────────────┐     │  │
│  │  │  NSM (Nitro Secure Module) - Hardware          │     │  │
│  │  │                                                 │     │  │
│  │  │  Functions:                                     │     │  │
│  │  │  - Generate attestation documents               │     │  │
│  │  │  - Provide cryptographic entropy                │     │  │
│  │  │  - Sign with AWS certificate chain              │     │  │
│  │  │                                                 │     │  │
│  │  │  Attestation Contains:                          │     │  │
│  │  │  - PCR0: Enclave image hash                     │     │  │
│  │  │  - PCR1: Kernel hash                            │     │  │
│  │  │  - PCR2: Application hash                       │     │  │
│  │  │  - Public key: 0xf343dae1...                    │     │  │
│  │  │  - Timestamp                                    │     │  │
│  │  │  - AWS certificate chain                        │     │  │
│  │  └────────────────────────────────────────────────┘     │  │
│  └──────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
        │
        │ Fetch prices, execute swaps
        │
┌───────▼─────────────────────────────────────────────────────────┐
│                  EXTERNAL SERVICES                               │
│                                                                  │
│  ┌────────────────┐  ┌────────────────┐  ┌──────────────────┐ │
│  │  Seal Servers  │  │  Cetus DEX     │  │  Walrus Storage  │ │
│  │                │  │                │  │                  │ │
│  │  - Server 1    │  │  - Swap API    │  │  - Blob storage  │ │
│  │  - Server 2    │  │  - Price feed  │  │  - Metadata      │ │
│  │  - Server 3    │  │  - Liquidity   │  │  - Erasure code  │ │
│  └────────────────┘  └────────────────┘  └──────────────────┘ │
└──────────────────────────────────────────────────────────────────┘
```

---

## Core Components

### 1. Frontend (User Interface)

**Technology:** Next.js 14, TypeScript, @mysten/dapp-kit

**Responsibilities:**
- Wallet connection (Sui wallet)
- Intent creation UI
- Seal SDK integration for encryption
- Intent submission
- Result verification display

**Key Features:**
- Simple intent builder: "Swap X for Y"
- Real-time status updates
- Attestation verification UI
- Transaction history

---

### 2. Seal Threshold Encryption

**Purpose:** User-controlled encryption with decentralization

**How it works:**

```typescript
// User encrypts intent
const sealClient = new SealClient({
  servers: [SERVER_1, SERVER_2, SERVER_3],
  threshold: 2, // Need 2 of 3 to decrypt
});

// Create key ID with allowlist namespace
const keyId = generateKeyId(PACKAGE_ID, ALLOWLIST_ID, nonce);

const { encryptedObject } = await sealClient.encrypt({
  threshold: 2,
  packageId: PACKAGE_ID,
  id: keyId, // Format: [pkg]::[allowlist_id][nonce]
  data: intentData,
});
```

**Decryption (Nautilus or User):**

```move
// On-chain allowlist check
entry fun seal_approve(id: vector<u8>, allowlist: &Allowlist, ctx: &TxContext) {
    // Check if caller is in allowlist
    assert!(allowlist.list.contains(&ctx.sender()), ENoAccess);
    // Check if key ID matches allowlist namespace
    assert!(is_prefix(allowlist.namespace(), id), ENoAccess);
}
```

**Key Properties:**
- ✅ 2-of-3 threshold (no single point of failure)
- ✅ Allowlist-based access control
- ✅ User controls who can decrypt
- ✅ Transparent on-chain policy

---

### 3. Nautilus TEE (Core Innovation)

**Purpose:** Verifiable computation without trust

#### 3.1 Enclave Setup

```bash
# Configure and deploy
cd nautilus/
export KEY_PAIR=your-aws-keypair
export AWS_ACCESS_KEY_ID=your-key
export AWS_SECRET_ACCESS_KEY=your-secret

# Deploy enclave
sh configure_enclave.sh mist-protocol
make ENCLAVE_APP=mist-protocol
make run
sh expose_enclave.sh
```

**Result:**
- Enclave running on AWS Nitro
- Ephemeral key pair generated inside TEE
- Public key: `0xf343dae1...`
- PCR values: `911c87d0...` (reproducible)

#### 3.2 Enclave Endpoints

**Endpoint 1: Health Check**
```bash
curl http://<PUBLIC_IP>:3000/health_check

Response:
{
  "pk": "f343dae1df7f2c4676612368e40bf42878e522349e4135c2caa52bc79f0fc6e2",
  "endpoints_status": {
    "api.cetus.zone": true,
    "seal.mystenlabs.com": true,
    "fullnode.testnet.sui.io": true
  }
}
```

**Endpoint 2: Get Attestation**
```bash
curl http://<PUBLIC_IP>:3000/get_attestation

Response:
{
  "attestation": "a3012663657274696669636174656..." // Hex-encoded attestation document
}
```

**Endpoint 3: Process Intent** (Main endpoint)
```bash
curl -X POST http://<PUBLIC_IP>:3000/process_intent \
  -H "Content-Type: application/json" \
  -d '{
    "payload": {
      "intent_id": "0x123...",
      "encrypted_data": "0xDEADBEEF...",
      "key_id": "[pkg]::[allowlist][nonce]"
    }
  }'

Response:
{
  "response": {
    "intent": 0,
    "timestamp_ms": 1744041600000,
    "data": {
      "executed": true,
      "swap_result": {
        "input_amount": 100,
        "output_amount": 85,
        "token_in": "USDC",
        "token_out": "SUI",
        "tx_hash": "0xABC..."
      }
    }
  },
  "signature": "b75d2d44c4a6b3c676fe087465c0e85206b101e21be6cda4...",
  "attestation": "a3012663657274696669636174656..."
}
```

#### 3.3 Enclave Code Structure

```rust
// nautilus/src/apps/mist-protocol/mod.rs

use crate::common::{IntentMessage, IntentScope, ProcessDataRequest, ProcessedDataResponse};
use crate::AppState;
use crate::EnclaveError;
use axum::extract::State;
use axum::Json;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Intent data structure
#[derive(Debug, Serialize, Deserialize)]
pub struct SwapIntent {
    pub token_in: String,
    pub token_out: String,
    pub amount: u64,
    pub min_output: u64,
    pub deadline: u64,
}

/// Request structure
#[derive(Debug, Serialize, Deserialize)]
pub struct ProcessIntentRequest {
    pub intent_id: String,
    pub encrypted_data: String,  // Seal encrypted
    pub key_id: String,           // [pkg]::[allowlist][nonce]
}

/// Response structure
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SwapExecutionResult {
    pub executed: bool,
    pub input_amount: u64,
    pub output_amount: u64,
    pub token_in: String,
    pub token_out: String,
    pub tx_hash: String,
    pub execution_price: f64,
}

/// Main processing endpoint
pub async fn process_intent(
    State(state): State<Arc<AppState>>,
    Json(request): Json<ProcessDataRequest<ProcessIntentRequest>>,
) -> Result<Json<ProcessedDataResponse<IntentMessage<SwapExecutionResult>>>, EnclaveError> {

    // STEP 1: Call seal_approve via parent EC2
    let approve_result = call_seal_approve(
        &request.payload.key_id,
        &state.eph_kp,
    ).await?;

    // STEP 2: Decrypt intent using Seal
    let decrypted_data = decrypt_with_seal(
        &request.payload.encrypted_data,
        &request.payload.key_id,
        &state.eph_kp,
    ).await?;

    // STEP 3: Parse intent
    let intent: SwapIntent = serde_json::from_slice(&decrypted_data)
        .map_err(|e| EnclaveError::GenericError(format!("Invalid intent: {}", e)))?;

    // STEP 4: Validate intent
    validate_intent(&intent)?;

    // STEP 5: Fetch current price from Cetus
    let price = fetch_cetus_price(&intent.token_in, &intent.token_out).await?;

    // STEP 6: Calculate expected output
    let expected_output = calculate_output(intent.amount, price);

    // Validate slippage
    if expected_output < intent.min_output {
        return Err(EnclaveError::GenericError("Slippage too high".to_string()));
    }

    // STEP 7: Execute swap on Cetus
    let swap_result = execute_cetus_swap(
        &intent.token_in,
        &intent.token_out,
        intent.amount,
        intent.min_output,
    ).await?;

    // STEP 8: Verify execution
    if swap_result.output_amount < intent.min_output {
        return Err(EnclaveError::GenericError("Swap output below minimum".to_string()));
    }

    // STEP 9: Sign result with enclave's private key
    Ok(Json(to_signed_response(
        &state.eph_kp,
        SwapExecutionResult {
            executed: true,
            input_amount: intent.amount,
            output_amount: swap_result.output_amount,
            token_in: intent.token_in,
            token_out: intent.token_out,
            tx_hash: swap_result.tx_hash,
            execution_price: price,
        },
        current_timestamp_ms(),
        IntentScope::ProcessData,
    )))
}

/// Call seal_approve via parent EC2
async fn call_seal_approve(
    key_id: &str,
    keypair: &Ed25519KeyPair,
) -> Result<(), EnclaveError> {
    // Build transaction
    let tx = build_seal_approve_transaction(key_id, keypair.public())?;

    // Submit via parent EC2 (VSOCK -> Parent -> Sui RPC)
    let result = submit_transaction_via_parent(tx).await?;

    Ok(())
}

/// Decrypt with Seal via parent proxy
async fn decrypt_with_seal(
    encrypted: &str,
    key_id: &str,
    keypair: &Ed25519KeyPair,
) -> Result<Vec<u8>, EnclaveError> {
    // Request decryption keys from Seal servers via parent
    let url = "https://seal.mystenlabs.com/decrypt";
    let response = reqwest::post(url)
        .json(&json!({
            "encrypted": encrypted,
            "key_id": key_id,
            "pubkey": hex::encode(keypair.public().as_bytes())
        }))
        .send()
        .await
        .map_err(|e| EnclaveError::GenericError(format!("Seal request failed: {}", e)))?;

    let decrypted = response.bytes().await
        .map_err(|e| EnclaveError::GenericError(format!("Failed to read response: {}", e)))?;

    Ok(decrypted.to_vec())
}

/// Fetch price from Cetus DEX
async fn fetch_cetus_price(token_in: &str, token_out: &str) -> Result<f64, EnclaveError> {
    let url = format!(
        "https://api.cetus.zone/v2/sui/swap/price?token_in={}&token_out={}",
        token_in, token_out
    );

    let response = reqwest::get(&url).await
        .map_err(|e| EnclaveError::GenericError(format!("Cetus API failed: {}", e)))?;

    let json: serde_json::Value = response.json().await
        .map_err(|e| EnclaveError::GenericError(format!("Invalid JSON: {}", e)))?;

    let price = json["data"]["price"].as_f64()
        .ok_or_else(|| EnclaveError::GenericError("Missing price".to_string()))?;

    Ok(price)
}

/// Execute swap on Cetus
async fn execute_cetus_swap(
    token_in: &str,
    token_out: &str,
    amount: u64,
    min_output: u64,
) -> Result<SwapResult, EnclaveError> {
    // Build swap transaction
    let tx = build_cetus_swap_tx(token_in, token_out, amount, min_output)?;

    // Submit via parent
    let result = submit_transaction_via_parent(tx).await?;

    Ok(SwapResult {
        output_amount: result.output_amount,
        tx_hash: result.tx_hash,
    })
}

#[derive(Debug)]
struct SwapResult {
    output_amount: u64,
    tx_hash: String,
}

fn validate_intent(intent: &SwapIntent) -> Result<(), EnclaveError> {
    // Validate deadline
    let now = current_timestamp_ms();
    if intent.deadline < now {
        return Err(EnclaveError::GenericError("Intent expired".to_string()));
    }

    // Validate amounts
    if intent.amount == 0 {
        return Err(EnclaveError::GenericError("Invalid amount".to_string()));
    }

    Ok(())
}

fn calculate_output(amount: u64, price: f64) -> u64 {
    (amount as f64 * price) as u64
}

fn current_timestamp_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}
```

---

### 4. Smart Contracts (Sui Move)

#### 4.1 Allowlist Contract

```move
// contracts/sources/allowlist.move

module mist_protocol::allowlist;

use std::string::String;

public struct Allowlist has key {
    id: UID,
    name: String,
    members: vector<address>,
}

public struct Cap has key {
    id: UID,
    allowlist_id: ID,
}

/// Create allowlist
public fun create_allowlist(name: String, ctx: &mut TxContext): Cap {
    let allowlist = Allowlist {
        id: object::new(ctx),
        name,
        members: vector::empty(),
    };

    let cap = Cap {
        id: object::new(ctx),
        allowlist_id: object::id(&allowlist),
    };

    transfer::share_object(allowlist);
    cap
}

/// Add member (e.g., Nautilus pubkey)
public fun add(allowlist: &mut Allowlist, cap: &Cap, member: address) {
    assert!(cap.allowlist_id == object::id(allowlist), 0);
    allowlist.members.push_back(member);
}

/// Seal approve - called by Nautilus
entry fun seal_approve(
    id: vector<u8>,
    allowlist: &Allowlist,
    ctx: &TxContext
) {
    let caller = ctx.sender();

    // Check caller is in allowlist
    assert!(allowlist.members.contains(&caller), 1);

    // Check ID has correct prefix
    let namespace = object::id_bytes(&allowlist);
    assert!(is_prefix(namespace, id), 2);
}

fun is_prefix(prefix: vector<u8>, data: vector<u8>): bool {
    if (vector::length(&prefix) > vector::length(&data)) {
        return false
    };

    let mut i = 0;
    while (i < vector::length(&prefix)) {
        if (prefix[i] != data[i]) {
            return false
        };
        i = i + 1;
    };

    true
}
```

#### 4.2 Escrow Contract

```move
// contracts/sources/escrow.move

module mist_protocol::escrow;

use sui::coin::{Self, Coin};
use sui::sui::SUI;
use sui::balance::{Self, Balance};

/// Escrow for user funds
public struct Escrow has key {
    id: UID,
    owner: address,
    balance: Balance<SUI>,
    nautilus_pubkey: address,
}

/// Create escrow
public fun create_escrow(
    nautilus_pubkey: address,
    ctx: &mut TxContext
): Escrow {
    Escrow {
        id: object::new(ctx),
        owner: ctx.sender(),
        balance: balance::zero(),
        nautilus_pubkey,
    }
}

/// Deposit funds
public fun deposit(
    escrow: &mut Escrow,
    coin: Coin<SUI>,
    ctx: &TxContext
) {
    assert!(escrow.owner == ctx.sender(), 0);
    let balance = coin::into_balance(coin);
    balance::join(&mut escrow.balance, balance);
}

/// Execute swap (only Nautilus can call)
public fun execute_swap(
    escrow: &mut Escrow,
    amount: u64,
    ctx: &TxContext
): Coin<SUI> {
    // Only Nautilus can execute
    assert!(ctx.sender() == escrow.nautilus_pubkey, 1);

    // Withdraw amount
    let withdrawn = balance::split(&mut escrow.balance, amount);
    coin::from_balance(withdrawn, ctx)
}

/// Withdraw (owner only)
public fun withdraw(
    escrow: &mut Escrow,
    amount: u64,
    ctx: &mut TxContext
): Coin<SUI> {
    assert!(escrow.owner == ctx.sender(), 0);
    let withdrawn = balance::split(&mut escrow.balance, amount);
    coin::from_balance(withdrawn, ctx)
}
```

#### 4.3 Nautilus Verifier Contract

```move
// contracts/sources/nautilus_verifier.move

module mist_protocol::nautilus_verifier;

use sui::ed25519;
use sui::nitro_attestation::NitroAttestationDocument;

/// Enclave configuration
public struct EnclaveConfig has key {
    id: UID,
    pcr0: vector<u8>,  // Enclave image hash
    pcr1: vector<u8>,  // Kernel hash
    pcr2: vector<u8>,  // Application hash
    public_key: vector<u8>,
    is_active: bool,
}

/// Register enclave
public fun register_enclave(
    pcr0: vector<u8>,
    pcr1: vector<u8>,
    pcr2: vector<u8>,
    document: NitroAttestationDocument,
    ctx: &mut TxContext
) {
    // Verify attestation and extract public key
    let pk = load_pk(&document, pcr0, pcr1, pcr2);

    let config = EnclaveConfig {
        id: object::new(ctx),
        pcr0,
        pcr1,
        pcr2,
        public_key: pk,
        is_active: true,
    };

    transfer::share_object(config);
}

/// Verify signed result from Nautilus
public fun verify_result(
    config: &EnclaveConfig,
    message: vector<u8>,
    signature: vector<u8>,
): bool {
    assert!(config.is_active, 0);

    // Verify signature with registered public key
    ed25519::verify(&signature, &message, &config.public_key)
}

fun load_pk(
    document: &NitroAttestationDocument,
    expected_pcr0: vector<u8>,
    expected_pcr1: vector<u8>,
    expected_pcr2: vector<u8>,
): vector<u8> {
    // Extract PCRs from attestation
    let (pcr0, pcr1, pcr2) = document.pcrs();

    // Verify PCRs match expected
    assert!(pcr0 == expected_pcr0, 1);
    assert!(pcr1 == expected_pcr1, 2);
    assert!(pcr2 == expected_pcr2, 3);

    // Extract and return public key
    document.public_key()
}
```

#### 4.4 Intent Contract

```move
// contracts/sources/intent.move

module mist_protocol::intent;

/// Intent object
public struct Intent has key {
    id: UID,
    user: address,
    encrypted_data: vector<u8>,  // Seal encrypted
    key_id: vector<u8>,           // [pkg]::[allowlist][nonce]
    status: u8,                   // 0=pending, 1=executed, 2=failed
    result_data: vector<u8>,      // Execution result
    signature: vector<u8>,        // Nautilus signature
}

const STATUS_PENDING: u8 = 0;
const STATUS_EXECUTED: u8 = 1;
const STATUS_FAILED: u8 = 2;

/// Create intent
public fun create_intent(
    encrypted_data: vector<u8>,
    key_id: vector<u8>,
    ctx: &mut TxContext
) {
    let intent = Intent {
        id: object::new(ctx),
        user: ctx.sender(),
        encrypted_data,
        key_id,
        status: STATUS_PENDING,
        result_data: vector::empty(),
        signature: vector::empty(),
    };

    transfer::share_object(intent);
}

/// Submit result (Nautilus only)
public fun submit_result(
    intent: &mut Intent,
    config: &EnclaveConfig,
    result_data: vector<u8>,
    signature: vector<u8>,
    ctx: &TxContext
) {
    // Verify signature
    let valid = nautilus_verifier::verify_result(
        config,
        result_data,
        signature
    );
    assert!(valid, 0);

    // Update intent
    intent.result_data = result_data;
    intent.signature = signature;
    intent.status = STATUS_EXECUTED;
}
```

---

## Trust Model

### What You DON'T Need to Trust

❌ **Nautilus operator** - Anyone can run it, all produce verifiable proofs
❌ **Parent EC2 instance** - Just a dumb proxy, can't forge signatures
❌ **Seal server operators** - 2-of-3 threshold, no single point of failure
❌ **Cetus DEX** - Execution verified inside TEE before signing

### What You DO Trust

✅ **AWS Nitro hardware** - Industry standard TEE
✅ **Mathematics** - Cryptographic signatures and attestations
✅ **Open source code** - Reproducible builds, anyone can verify
✅ **Blockchain** - Sui's consensus and execution

### Trust Comparison

```
┌─────────────────────────────────────────────────────────┐
│                     ENCIFHER                             │
│                                                          │
│  encrypt.rpc.encifher.io (Black Box)                    │
│         │                                                │
│         │  ❌ Must trust company                         │
│         │  ❌ Can't see code                             │
│         │  ❌ Can't verify                               │
│         │  ❌ Single point of failure                    │
│         │                                                │
│         ▼                                                │
│    Result (take our word for it)                        │
└─────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────┐
│                  MIST PROTOCOL                           │
│                                                          │
│  Your Nautilus Instance (Open Source)                   │
│         │                                                │
│         │  ✅ Reproducible build                         │
│         │  ✅ PCR verification                           │
│         │  ✅ Public attestation                         │
│         │  ✅ Anyone can run                             │
│         │                                                │
│         ▼                                                │
│    Signed Result + Attestation                          │
│    (cryptographic proof, verify on-chain)               │
└─────────────────────────────────────────────────────────┘
```

---

## Complete User Flow

### Phase 1: Setup (One-time)

```
┌─────────────────────────────────────────────────────────┐
│  DEPLOYMENT (Done by protocol operator)                 │
└─────────────────────────────────────────────────────────┘

1. Deploy Nautilus enclave to AWS Nitro
   ├─ Generate ephemeral key pair inside TEE
   ├─ Public key: 0xf343dae1...
   └─ PCR values: 911c87d0... (from reproducible build)

2. Register enclave on-chain
   ├─ Submit attestation document
   ├─ Verify PCR values
   └─ Store public key in EnclaveConfig

3. Deploy smart contracts
   ├─ Allowlist contract
   ├─ Escrow contract
   ├─ Intent contract
   └─ Nautilus verifier contract

4. Configure allowlist
   ├─ Add Nautilus pubkey: 0xf343dae1...
   └─ Users can add themselves as needed
```

### Phase 2: User Creates Intent

```
┌─────────────────────────────────────────────────────────┐
│  USER FRONTEND                                           │
└─────────────────────────────────────────────────────────┘

Step 1: User connects wallet
├─ Sui Wallet extension
└─ Address: 0x123...

Step 2: User creates intent
├─ Select: "Swap USDC for SUI"
├─ Input amount: 100 USDC
├─ Min output: 85 SUI
└─ Deadline: 1 hour

Step 3: Encrypt intent with Seal
┌───────────────────────────────────────────────────────┐
│ const intent = {                                      │
│   token_in: "USDC",                                   │
│   token_out: "SUI",                                   │
│   amount: 100,                                        │
│   min_output: 85,                                     │
│   deadline: Date.now() + 3600000                     │
│ };                                                    │
│                                                       │
│ const keyId = generateKeyId(                         │
│   PACKAGE_ID,                                        │
│   ALLOWLIST_ID,                                      │
│   randomNonce()                                      │
│ );                                                    │
│                                                       │
│ const { encryptedObject } = await sealClient.encrypt({│
│   threshold: 2,                                       │
│   packageId: PACKAGE_ID,                             │
│   id: keyId,                                         │
│   data: JSON.stringify(intent)                       │
│ });                                                   │
└───────────────────────────────────────────────────────┘

Step 4: Deposit funds to escrow (optional, or Nautilus uses own)
├─ Transfer 100 USDC to escrow contract
└─ Escrow controlled by Nautilus pubkey

Step 5: Submit encrypted intent on-chain
┌───────────────────────────────────────────────────────┐
│ const tx = new Transaction();                         │
│                                                       │
│ tx.moveCall({                                         │
│   target: `${PACKAGE_ID}::intent::create_intent`,    │
│   arguments: [                                        │
│     tx.pure(encryptedObject),                        │
│     tx.pure(keyId)                                   │
│   ]                                                   │
│ });                                                   │
│                                                       │
│ await signAndExecute(tx);                            │
└───────────────────────────────────────────────────────┘

Result: Intent object created on-chain (shared)
├─ ID: 0xINTENT123...
├─ User: 0x123...
├─ Encrypted data: 0xDEADBEEF...
├─ Key ID: [pkg]::[allowlist][nonce]
└─ Status: PENDING
```

### Phase 3: Nautilus Processes Intent

```
┌─────────────────────────────────────────────────────────┐
│  NAUTILUS TEE (AWS Nitro Enclave)                       │
└─────────────────────────────────────────────────────────┘

Step 1: Nautilus scans for new intents
├─ Query Sui blockchain for pending intents
└─ Find: Intent 0xINTENT123...

Step 2: Call seal_approve via parent EC2
┌───────────────────────────────────────────────────────┐
│ // Parent EC2 submits transaction:                    │
│ sui client call \                                     │
│   --function seal_approve \                           │
│   --module allowlist \                                │
│   --package $PACKAGE_ID \                             │
│   --args $KEY_ID $ALLOWLIST_ID \                      │
│   --gas-budget 10000000                               │
│                                                       │
│ // Allowlist contract checks:                         │
│ // - Is 0xf343dae1... (Nautilus) in list? ✅         │
│ // - Does key_id match allowlist namespace? ✅        │
│ // Result: Approved!                                  │
└───────────────────────────────────────────────────────┘

Step 3: Request decryption from Seal servers
┌───────────────────────────────────────────────────────┐
│ // Via parent EC2 proxy:                              │
│ POST https://seal.mystenlabs.com/decrypt              │
│ {                                                     │
│   "encrypted": "0xDEADBEEF...",                       │
│   "key_id": "[pkg]::[allowlist][nonce]",             │
│   "requester": "0xf343dae1..." // Nautilus pubkey    │
│ }                                                     │
│                                                       │
│ // Seal servers check:                                │
│ // - seal_approve called? ✅                          │
│ // - Requester in allowlist? ✅                       │
│ // - Return decryption keys (2-of-3)                  │
└───────────────────────────────────────────────────────┘

Step 4: Decrypt inside TEE
┌───────────────────────────────────────────────────────┐
│ // INSIDE ENCLAVE (private memory):                   │
│ let decrypted = decrypt(encrypted, keys);             │
│                                                       │
│ let intent = JSON.parse(decrypted);                   │
│ // {                                                  │
│ //   token_in: "USDC",                                │
│ //   token_out: "SUI",                                │
│ //   amount: 100,                                     │
│ //   min_output: 85,                                  │
│ //   deadline: 1744041600000                          │
│ // }                                                  │
└───────────────────────────────────────────────────────┘

Step 5: Validate intent
├─ Check deadline not expired ✅
├─ Check amount > 0 ✅
└─ Check token pair supported ✅

Step 6: Fetch current price from Cetus
┌───────────────────────────────────────────────────────┐
│ // Via parent proxy:                                  │
│ GET https://api.cetus.zone/v2/sui/swap/price?         │
│     token_in=USDC&token_out=SUI                       │
│                                                       │
│ Response: { "price": 0.85, "liquidity": 1000000 }    │
└───────────────────────────────────────────────────────┘

Step 7: Calculate expected output
├─ Expected: 100 / 0.85 = 117.65 SUI
├─ Min required: 85 SUI
└─ Check: 117.65 >= 85 ✅ (slippage OK)

Step 8: Execute swap on Cetus
┌───────────────────────────────────────────────────────┐
│ // Build transaction (Nautilus signs with own key):   │
│ let tx = build_cetus_swap_tx(                         │
│   from: "USDC",                                       │
│   to: "SUI",                                          │
│   amount: 100,                                        │
│   min_output: 85                                      │
│ );                                                    │
│                                                       │
│ // Parent EC2 submits to Sui:                         │
│ let result = submit_transaction(tx);                  │
│                                                       │
│ // Result:                                            │
│ // {                                                  │
│ //   tx_hash: "0xABC...",                             │
│ //   output_amount: 118,                              │
│ //   execution_price: 0.847                           │
│ // }                                                  │
└───────────────────────────────────────────────────────┘

Step 9: Verify execution
├─ Check output: 118 >= 85 ✅
├─ Check price reasonable: 0.847 ≈ 0.85 ✅
└─ Execution valid ✅

Step 10: Sign result inside TEE
┌───────────────────────────────────────────────────────┐
│ let result = SwapExecutionResult {                    │
│   executed: true,                                     │
│   input_amount: 100,                                  │
│   output_amount: 118,                                 │
│   token_in: "USDC",                                   │
│   token_out: "SUI",                                   │
│   tx_hash: "0xABC...",                                │
│   execution_price: 0.847                              │
│ };                                                    │
│                                                       │
│ // Sign with enclave's PRIVATE KEY (never leaves):    │
│ let signature = sign(result, enclave_private_key);    │
│                                                       │
│ // Generate attestation from NSM:                     │
│ let attestation = nsm_get_attestation();              │
└───────────────────────────────────────────────────────┘

Step 11: Submit result on-chain
┌───────────────────────────────────────────────────────┐
│ // Via parent:                                        │
│ sui client call \                                     │
│   --function submit_result \                          │
│   --module intent \                                   │
│   --package $PACKAGE_ID \                             │
│   --args $INTENT_ID $ENCLAVE_CONFIG \                 │
│          $RESULT_DATA $SIGNATURE                      │
│                                                       │
│ // Contract verifies signature with registered pubkey │
│ // Updates intent status to EXECUTED                  │
└───────────────────────────────────────────────────────┘
```

### Phase 4: User Retrieves Result

```
┌─────────────────────────────────────────────────────────┐
│  USER FRONTEND                                           │
└─────────────────────────────────────────────────────────┘

Step 1: Poll intent status
├─ Query: Intent 0xINTENT123...
└─ Status: EXECUTED ✅

Step 2: Retrieve result
┌───────────────────────────────────────────────────────┐
│ const intent = await suiClient.getObject({            │
│   id: INTENT_ID,                                      │
│   options: { showContent: true }                      │
│ });                                                   │
│                                                       │
│ const result = parseResult(intent.result_data);       │
│ // {                                                  │
│ //   executed: true,                                  │
│ //   input_amount: 100,                               │
│ //   output_amount: 118,                              │
│ //   token_in: "USDC",                                │
│ //   token_out: "SUI",                                │
│ //   tx_hash: "0xABC...",                             │
│ //   execution_price: 0.847                           │
│ // }                                                  │
└───────────────────────────────────────────────────────┘

Step 3: Verify result on-chain
┌───────────────────────────────────────────────────────┐
│ // Verify signature:                                  │
│ const valid = await verifySignature(                  │
│   intent.result_data,                                 │
│   intent.signature,                                   │
│   ENCLAVE_CONFIG                                      │
│ );                                                    │
│                                                       │
│ // Check: ✅ Signature valid                          │
│ // Check: ✅ From registered Nautilus                 │
│ // Check: ✅ PCR values match                         │
└───────────────────────────────────────────────────────┘

Step 4: Display to user
┌───────────────────────────────────────────────────────┐
│ ✅ Intent Executed Successfully!                      │
│                                                       │
│ Input: 100 USDC                                       │
│ Output: 118 SUI                                       │
│ Price: 0.847 USDC/SUI                                 │
│ TX: 0xABC...                                          │
│                                                       │
│ ✅ Verified by TEE                                    │
│ ✅ Attestation valid                                  │
│ ✅ PCR values match published code                    │
└───────────────────────────────────────────────────────┘

Step 5: Claim funds (if in escrow)
├─ Call escrow.withdraw()
└─ Receive 118 SUI to wallet
```

---

## Security Guarantees

### 1. Privacy Guarantees

**Intent Privacy:**
- ✅ Intent encrypted with Seal (2-of-3 threshold)
- ✅ Only Nautilus TEE can decrypt (allowlist)
- ✅ Decryption happens inside isolated enclave
- ✅ Parent EC2 never sees plaintext intent
- ✅ On-chain observers see only encrypted data

**Amount Privacy:**
- ✅ Transaction amounts encrypted
- ✅ Processed inside TEE
- ✅ Result can be encrypted again (optional)

**Execution Privacy:**
- ✅ Swap execution happens privately
- ✅ No front-running possible
- ✅ MEV protection built-in

### 2. Integrity Guarantees

**Computation Integrity:**
- ✅ Code running in TEE is verifiable (PCR values)
- ✅ Results are cryptographically signed
- ✅ Signatures verified on-chain
- ✅ Attestation proves genuine TEE

**Execution Integrity:**
- ✅ Nautilus verifies swap results before signing
- ✅ Slippage protection enforced
- ✅ Price validation inside TEE
- ✅ Can't forge execution results

### 3. Availability Guarantees

**No Single Point of Failure:**
- ✅ Anyone can run Nautilus instance
- ✅ Multiple Seal servers (2-of-3)
- ✅ Cetus DEX is decentralized
- ✅ Sui blockchain consensus

**Fault Tolerance:**
- ✅ Intent persisted on-chain
- ✅ Can retry if Nautilus fails
- ✅ Seal threshold allows 1 server down
- ✅ Escrow protects user funds

### 4. Verifiability Guarantees

**Code Verifiability:**
- ✅ Reproducible builds
- ✅ PCR values prove exact code
- ✅ Open source (can audit)
- ✅ Anyone can rebuild and verify

**Execution Verifiability:**
- ✅ Attestation from AWS hardware
- ✅ Cryptographic signatures
- ✅ On-chain verification
- ✅ Public audit trail

---

## Comparison vs Encifher

### Feature Comparison Table

| Feature | Encifher | Mist Protocol |
|---------|----------|---------------|
| **Amount Privacy** | ✅ Yes | ✅ Yes |
| **Intent Privacy** | ❌ No intents | ✅ Yes (encrypted) |
| **Recipient Privacy** | ❌ No | ✅ Yes (can add stealth) |
| **Threshold Encryption** | ❌ No (single gateway) | ✅ Yes (Seal 2-of-3) |
| **Self-Managed TEE** | ❌ No (must trust Encifher) | ✅ Yes (run your own) |
| **Verifiable Computation** | ❌ No (black box) | ✅ Yes (attestation) |
| **Reproducible Builds** | ❌ No (closed source) | ✅ Yes (open source) |
| **On-Chain Verification** | ❌ No | ✅ Yes (PCR + signatures) |
| **Decentralized** | ❌ No (single company) | ✅ Yes (anyone can run) |
| **Cost Efficiency** | ⚠️ Unknown | ✅ Yes (Walrus storage) |
| **MEV Protection** | ⚠️ Partial | ✅ Yes (encrypted intents) |
| **Open Source** | ❌ No | ✅ Yes |

### Trust Comparison

**Encifher Trust Requirements:**
```
You must trust:
├─ Encifher company (won't steal/lie)
├─ Encifher infrastructure (not compromised)
├─ Encifher gateway (black box)
└─ Encifher operators (honest)

If any fails → System fails
```

**Mist Protocol Trust Requirements:**
```
You trust:
├─ AWS Nitro hardware (industry standard)
├─ Mathematics (cryptography)
├─ Open source code (can audit)
└─ Sui blockchain (consensus)

No need to trust:
├─ Nautilus operator (verifiable)
├─ Seal servers (2-of-3 threshold)
├─ Parent EC2 (just proxy)
└─ Any specific company
```

### Architecture Comparison

**Encifher:**
```
User → encrypt.rpc.encifher.io (Black Box) → Result

       ❌ Centralized gateway
       ❌ Can't verify
       ❌ Must trust company
       ❌ Single point of failure
```

**Mist Protocol:**
```
User → Seal (2-of-3) → Sui Blockchain
              ↓
       Nautilus TEE → Verifiable Result
       (anyone can run)

       ✅ Decentralized
       ✅ Verifiable
       ✅ Trustless
       ✅ Resilient
```

---

## Why This Architecture Wins

### 1. **True Decentralization**

**Encifher:** Must use their gateway
**Mist:** Anyone can run Nautilus instance, all produce verifiable proofs

### 2. **Transparency**

**Encifher:** Black box, can't see what's happening
**Mist:** Reproducible builds, anyone can audit and verify

### 3. **Resilience**

**Encifher:** Single point of failure
**Mist:** Multiple Nautilus instances, threshold encryption, no SPOF

### 4. **Control**

**Encifher:** Users must trust Encifher
**Mist:** Users control everything (keys, servers, policies)

### 5. **Innovation**

**Encifher:** Proprietary solution
**Mist:** Combines cutting-edge Sui primitives (Nautilus + Seal + Walrus)

---

## Key Takeaways

### What Makes Mist Protocol Special?

1. **Verifiable Privacy**
   - Not just "trust us", but "verify yourself"
   - Cryptographic proofs at every step
   - Reproducible builds prove no backdoors

2. **Separation of Concerns**
   - Seal: Encryption (user-controlled)
   - Nautilus: Computation (verifiable)
   - Allowlist: Access control (transparent)
   - Each component auditable independently

3. **No Trust Required**
   - Don't trust operator → verify attestation
   - Don't trust single server → 2-of-3 threshold
   - Don't trust black box → open source + reproducible

4. **Sui Native**
   - Showcases Sui's unique capabilities
   - Positions Sui as privacy leader
   - Demonstrates ecosystem maturity

### The Value Proposition

**For Users:**
- Privacy without trust tradeoffs
- Verifiable execution
- Control over their data
- MEV protection

**For Builders:**
- Template for TEE applications
- Clear patterns for Seal integration
- Reference implementation

**For Sui:**
- Showcases advanced features
- Differentiates from competitors
- Attracts privacy-focused projects

---

## Next Steps

### For Hackathon:

**Day 1-2:** Core implementation
- Deploy Nautilus enclave
- Integrate Seal SDK
- Implement intent flow

**Day 3:** Integration
- Connect all components
- Test end-to-end
- Fix bugs

**Day 4:** Polish
- UI improvements
- Documentation
- Demo video

**Day 5:** Presentation
- Prepare slides
- Practice demo
- Submit

### Post-Hackathon:

**Phase 1:** Production hardening
- Security audit
- Performance optimization
- Mainnet deployment

**Phase 2:** Feature expansion
- Stealth addresses
- Multiple DEX support
- Advanced privacy features

**Phase 3:** Ecosystem growth
- Partnerships
- Liquidity incentives
- Community building

---

## Conclusion

Mist Protocol demonstrates that **verifiable privacy is possible** without sacrificing decentralization or transparency. By combining Nautilus TEE, Seal threshold encryption, and Sui's powerful object model, we've created a system that is:

✅ **Private** - Encrypted intents, hidden amounts
✅ **Verifiable** - Attestations, signatures, reproducible builds
✅ **Decentralized** - Anyone can run, no single point of failure
✅ **Transparent** - Open source, auditable, provable

This is the future of DeFi privacy on Sui.

---

**Let's build! 🚀**
