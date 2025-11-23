# Mist Protocol Architecture

**Privacy-preserving intent-based swaps on Sui using Nautilus TEE and SEAL encryption**

---

## System Overview

```
┌─────────────┐
│   Frontend  │ User creates encrypted swap intent
│   (Next.js) │
└──────┬──────┘
       │ 1. Deposit SUI/USDC → Get encrypted tickets
       │ 2. Create swap intent with SEAL-encrypted amounts
       ▼
┌─────────────────────────────────────────────────────────┐
│           Sui Blockchain (Smart Contracts)              │
│                                                         │
│  ┌──────────────┐  ┌─────────────┐  ┌──────────────┐  │
│  │ LiquidityPool│  │ VaultEntry  │  │ IntentQueue  │  │
│  │ (Shared)     │  │ (Per-user)  │  │ (Shared)     │  │
│  │              │  │             │  │              │  │
│  │ SUI Balance  │  │ Encrypted   │  │ Pending      │  │
│  │ USDC Balance │  │ Tickets     │  │ Intents      │  │
│  └──────────────┘  └─────────────┘  └──────────────┘  │
└─────────────────────────────────────────────────────────┘
       │
       │ Backend polls IntentQueue every 5s
       ▼
┌─────────────────────────────────────────────────────────┐
│         Nautilus Backend (TEE)                          │
│                                                         │
│  1. Decrypt intent with SEAL (2-of-3 threshold)        │
│  2. Execute swap on Cetus with TEE wallet              │
│  3. Re-encrypt output with SEAL                        │
│  4. Build execute_swap transaction                      │
│  5. Send to signing service ──┐                        │
└────────────────────────────────┼────────────────────────┘
                                 │
                                 ▼
┌─────────────────────────────────────────────────────────┐
│         tx-signer (HTTP Service)                        │
│                                                         │
│  - Wraps `sui keytool sign`                            │
│  - Signs transaction digest                             │
│  - Returns signature                                    │
└────────────────────────────────────────────────────────┘
       │
       │ Backend executes signed transaction
       ▼
┌─────────────────────────────────────────────────────────┐
│           Transaction Executed On-Chain                 │
│                                                         │
│  - Consumes input tickets                              │
│  - Creates new encrypted output ticket in user vault    │
│  - Emits SwapExecutedEvent                             │
└─────────────────────────────────────────────────────────┘
```

---

## Core Components

### 1. Smart Contracts (Sui Move)

#### LiquidityPool (Shared Object)
Holds all user deposits as real tokens:

```move
public struct LiquidityPool has key {
    id: UID,
    sui_balance: Balance<SUI>,
    usdc_balance: Balance<USDC>,
    tee_authority: address,  // Only TEE can execute swaps
    paused: bool,
}
```

**Purpose:** Escrow for deposits/withdrawals, prevents direct user access

#### VaultEntry (Per-User Shared Object)
Contains user's encrypted ticket balances:

```move
public struct VaultEntry has key {
    id: UID,
    owner: address,
    tickets: ObjectBag,  // Maps ticket_id -> EncryptedTicket
    next_ticket_id: u64,
}

public struct EncryptedTicket has key, store {
    id: UID,
    ticket_id: u64,
    token_type: String,           // "SUI" or "USDC"
    encrypted_amount: vector<u8>, // SEAL threshold-encrypted
}
```

**Key Feature:** Both user AND TEE can decrypt amounts using SEAL

#### IntentQueue (Global Shared Object)
Tracks pending swap intents:

```move
public struct IntentQueue has key {
    id: UID,
    pending: Table<ID, bool>,  // intent_id -> is_pending
}

public struct SwapIntent has key {
    id: UID,
    vault_id: ID,
    locked_tickets: ObjectBag,     // Tickets moved from vault
    token_out: String,
    min_output_amount: u64,
    deadline: u64,
    user: address,
}
```

**Purpose:** Persistent queue that survives backend restarts

---

### 2. Nautilus Backend (Rust + TEE)

**Location:** `backend/`

#### Intent Processor
Polls IntentQueue every 5 seconds:

```rust
loop {
    // 1. Query all pending intents from IntentQueue
    let intents = query_pending_intents().await;

    // 2. For each intent:
    for intent in intents {
        // Decrypt tickets with SEAL
        let decrypted = decrypt_intent_with_seal(intent).await?;

        // Execute swap
        execute_swap(decrypted).await?;
    }

    tokio::time::sleep(Duration::from_secs(5)).await;
}
```

#### Swap Executor
Executes swaps and creates output tickets:

```rust
async fn execute_swap_mock(decrypted: &DecryptedSwapIntent) {
    // 1. Execute Cetus swap (currently mock: SUI → SUI)
    let output_amount = cetus_swap(decrypted.total_amount).await;

    // 2. Re-encrypt output with SEAL
    let encrypted_output = seal_encrypt(output_amount, vault_id)?;

    // 3. Build execute_swap_sui transaction
    let tx_data = build_execute_swap_tx(encrypted_output)?;

    // 4. Sign via tx-signer service
    let signature = sign_transaction(tx_data).await?;

    // 5. Execute on-chain
    sui_client.execute_transaction_block(signed_tx).await?;
}
```

**Dependencies:**
- SEAL SDK (threshold encryption)
- sui-sdk (transaction building)
- Nautilus framework (TEE attestation)

---

### 3. Transaction Signing Service

**Location:** `tx-signer/`

**Purpose:** Workaround for fastcrypto version conflict between SEAL SDK and sui-types

#### The Problem
- SEAL SDK uses `fastcrypto@d1fcb85`
- sui-types uses `fastcrypto@09f8697`
- Rust treats these as incompatible types
- Cannot sign transactions in same binary as SEAL encryption

#### The Solution
Simple HTTP service that wraps `sui keytool sign`:

```rust
// Receives unsigned transaction
POST /sign
{
    "address": "0x...",
    "tx_data_b64": "base64_encoded_tx"
}

// Returns signature
{
    "signature": "base64_encoded_signature"
}
```

**Why This Works:**
- Backend handles ALL SEAL encryption/decryption
- Signing service has NO SEAL dependency
- Separate binaries = no version conflict
- Production-ready pattern (common in Sui projects)

---

### 4. Frontend (Next.js)

**Location:** `frontend/`

#### Features
- Wallet connection (@mysten/dapp-kit)
- SEAL encryption for deposits
- SEAL decryption for ticket viewing
- Swap intent creation
- Transaction execution

#### User Flow

**Deposit:**
1. User deposits 1.0 SUI
2. Frontend encrypts amount with SEAL
3. Transaction creates encrypted ticket in user's vault
4. User can decrypt and see: "1.0 SUI"

**Swap:**
1. User selects tickets to swap (e.g., "1.0 SUI")
2. Frontend encrypts swap intent
3. Transaction creates SwapIntent and adds to IntentQueue
4. User waits for TEE to process

**View Results:**
1. User refreshes vault
2. Sees new output ticket
3. Decrypts with SEAL: "0.95 SUI" (after slippage)

---

## Data Flow

### Complete Swap Flow

```
1. USER CREATES INTENT (Frontend)
   ├─ Encrypts ticket amounts with SEAL
   ├─ Creates SwapIntent object
   └─ Adds to IntentQueue.pending

2. TEE POLLS QUEUE (Backend - every 5s)
   ├─ Queries IntentQueue for pending intents
   ├─ For each intent:
   │  ├─ Decrypt tickets with SEAL (2-of-3 threshold)
   │  ├─ Execute Cetus swap with TEE wallet
   │  ├─ Re-encrypt output with SEAL
   │  ├─ Build execute_swap transaction
   │  ├─ Call tx-signer to sign
   │  └─ Execute on-chain
   └─ Loop

3. ON-CHAIN EXECUTION
   ├─ Verify TEE authority
   ├─ Remove consumed tickets from vault
   ├─ Create new encrypted output ticket
   ├─ Remove intent from queue
   └─ Emit SwapExecutedEvent

4. USER VIEWS RESULT (Frontend)
   ├─ Refresh vault
   ├─ Decrypt new output ticket with SEAL
   └─ See swapped amount
```

---

## Security Model

### Privacy Guarantees

1. **Encrypted Balances**
   - All ticket amounts encrypted with SEAL
   - Only user + TEE can decrypt
   - On-chain observers see encrypted bytes only

2. **Unlinkable Swaps**
   - TEE executes swaps with its own wallet
   - No direct link from user address to swap transaction
   - Multiple users' swaps can be batched

3. **Threshold Encryption**
   - SEAL uses 2-of-3 key servers
   - No single party can decrypt alone
   - TEE proves it has decryption rights via attestation

### Trust Assumptions

1. **TEE Integrity**
   - AWS Nitro Enclaves provide hardware attestation
   - Code hash verified before key release
   - Users verify attestation document

2. **SEAL Key Servers**
   - Operated by Mysten Labs
   - Threshold: need 2 of 3 servers
   - Cannot collude without breaking threshold

3. **Smart Contract**
   - Only TEE authority can execute swaps
   - Ticket ownership verified on-chain
   - Intent queue prevents double-execution

---

## Technical Details

### SEAL Encryption

**Encryption ID Format:**
```
vault_id (32 bytes) + random_nonce (5 bytes) = 37 bytes total
```

**Process:**
1. Generate encryption ID
2. Encrypt amount string with SEAL (2-of-3 threshold)
3. Store encrypted bytes in ticket
4. Encryption ID embedded in encrypted object

**Decryption:**
1. Parse encrypted object to extract encryption ID
2. Call `seal_approve` to prove ownership
3. SEAL key servers verify approval
4. Return decrypted amount

### Transaction Signing Workaround

Due to fastcrypto version conflicts, signing is handled by a separate service:

**Architecture:**
```
Backend (SEAL SDK)  →  tx-signer (sui keytool)  →  Blockchain
   │                         │
   fastcrypto v1            fastcrypto v2
   (d1fcb85)                (09f8697)
```

**See:** `docs/SIGNING_SOLUTION.md` for full details

---

## Contract Functions

### User Functions

**deposit_sui / deposit_usdc**
- Deposit tokens into pool
- Create encrypted ticket in user's vault
- Emit TicketCreatedEvent

**create_swap_intent**
- Lock tickets from vault into SwapIntent
- Add intent to IntentQueue
- Set slippage protection and deadline

**unwrap_sui / unwrap_usdc**
- Burn ticket
- Withdraw tokens from pool
- User receives real tokens back

### TEE Functions (Backend Only)

**execute_swap_sui / execute_swap_usdc**
- Called by TEE after Cetus swap
- Accepts swapped tokens from TEE wallet
- Creates new encrypted output ticket
- Updates vault, removes intent from queue
- Only callable by `tee_authority` address

---

## Deployment

### Prerequisites
1. Sui CLI installed
2. AWS EC2 instance (for production TEE)
3. Node.js 20+ for frontend
4. Rust 1.70+ for backend

### Components to Deploy

**1. Smart Contracts**
```bash
cd contracts/mist_protocol
sui client publish --gas-budget 500000000
```

**2. Backend (Nautilus TEE)**
```bash
cd backend
cargo build --release
cargo run
```

**3. Signing Service**
```bash
cd tx-signer
cargo build --release
cargo run
```

**4. Frontend**
```bash
cd frontend
pnpm install
pnpm build
pnpm start
```

### Configuration

Update these files with deployed contract IDs:
- `backend/src/apps/mist-protocol/seal_config.yaml`
- `frontend/.env.local`

---

## Current Status

### ✅ Working Features

1. **Deposit Flow**
   - Users deposit SUI/USDC
   - Receive encrypted tickets
   - Can decrypt and view balances

2. **Swap Intent Creation**
   - Users create swap intents
   - Tickets locked in intent
   - Intent added to on-chain queue

3. **TEE Processing**
   - Backend polls queue every 5s
   - Decrypts intents with SEAL
   - Executes mock swaps (SUI → SUI)
   - Creates encrypted output tickets

4. **Transaction Signing**
   - tx-signer service signs transactions
   - Backend executes signed transactions
   - Full automation working

5. **Output Tickets**
   - Users receive encrypted output
   - Can decrypt and see swapped amounts

### 🚧 In Progress

1. **Cetus Integration**
   - Replace mock swap with real Cetus DEX calls
   - Code ready in `cetus-swap/` directory

2. **Production Deployment**
   - Deploy to AWS EC2 with Nitro Enclaves
   - Configure TEE attestation

### 🎯 Future Enhancements

1. **Batch Processing**
   - Execute multiple swaps in single transaction
   - Gas optimization

2. **Multi-Token Support**
   - Beyond SUI/USDC
   - Dynamic pool management

3. **Advanced Privacy**
   - Mixing protocols
   - Zero-knowledge proofs

---

## Key Innovations

1. **No Database Required**
   - 100% on-chain state via IntentQueue
   - Survives backend restarts
   - Auditable and transparent

2. **Threshold Encryption**
   - SEAL provides 2-of-3 decryption
   - User + TEE both can decrypt
   - No single point of failure

3. **TEE Wallet Separation**
   - TEE uses own wallet for swaps
   - Breaks user → swap linkage
   - Enhanced privacy

4. **Signing Service Pattern**
   - Solves fastcrypto version conflict
   - Production-ready architecture
   - Secure key isolation

---

## References

- **SEAL Documentation:** https://docs.mystenlabs.com/seal
- **Nautilus TEE:** https://docs.sui.io/concepts/cryptography/nautilus
- **Cetus Protocol:** https://cetus.zone/docs
- **Sui Move:** https://docs.sui.io/guides/developer/first-app
