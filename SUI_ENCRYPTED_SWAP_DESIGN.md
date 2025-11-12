# Encifher Encrypted Swap - System Design (Sui)

## Table of Contents
1. [Vision](#vision)
2. [Core Concepts](#core-concepts)
3. [Architecture Overview](#architecture-overview)
4. [Sui Object Model](#sui-object-model)
5. [Technology Integration](#technology-integration)
6. [Swap Flow Design](#swap-flow-design)
7. [Privacy Model](#privacy-model)
8. [Trust & Security](#trust--security)
9. [Scalability & Performance](#scalability--performance)

---

## Vision

### What We're Building

A **privacy-preserving decentralized exchange** on Sui that allows users to:
- Swap tokens with **hidden amounts**
- Use **recipient-hiding payments** (stealth addresses)
- Maintain **verifiable computation** (no black boxes)
- Store data **cost-efficiently** (20x cheaper than on-chain)
- Trust **no single entity** (threshold cryptography)

### Why Sui?

**Sui's Unique Advantages:**
- **Object-centric model**: Natural access control and ownership
- **Parallel execution**: Fast transaction processing
- **Move language**: Type-safe smart contracts
- **Native tools**: Seal, Nautilus, Walrus built for Sui

**Comparison:**
```
Traditional DEX (Uniswap):
  ❌ All amounts public
  ❌ MEV exploitation
  ❌ No privacy

Tornado Cash:
  ✓ Amount privacy
  ❌ Fixed denominations
  ❌ Centralized relayer
  ❌ Recipient still visible

Encifher on Sui:
  ✓ Amount privacy (Seal threshold encryption)
  ✓ Recipient privacy (stealth addresses)
  ✓ Verifiable computation (Nautilus)
  ✓ No single point of failure
  ✓ Cost-efficient storage (Walrus)
```

---

## Core Concepts

### 1. Threshold Encryption (Seal)

**Problem:** Single encryption gateway = single point of failure

**Solution:** Distribute trust across multiple key servers

**How it works:**
```
┌─────────────────────────────────────────────────┐
│         Alice wants to encrypt "100 USDC"       │
└────────────────┬────────────────────────────────┘
                 ↓
        ┌────────┴────────┐
        │  Choose Servers │
        │  Server 1 (US)  │
        │  Server 2 (EU)  │
        │  Server 3 (Asia)│
        └────────┬────────┘
                 ↓
        ┌────────┴────────────────┐
        │  Threshold: 2-of-3      │
        │  (Need 2 to decrypt)    │
        └────────┬────────────────┘
                 ↓
┌────────────────┴────────────────────────────────┐
│              Encrypt with Seal                  │
│                                                 │
│  Server 1 encrypts with BLS key → Share 1      │
│  Server 2 encrypts with BLS key → Share 2      │
│  Server 3 encrypts with BLS key → Share 3      │
│                                                 │
│  Result: Encrypted Object (threshold 2-of-3)   │
└────────────────┬────────────────────────────────┘
                 ↓
        ┌────────┴────────┐
        │  To decrypt:    │
        │  Need 2+ servers│
        │  to cooperate   │
        └─────────────────┘
```

**Key Properties:**
- **Privacy**: Safe if < threshold servers compromised
- **Liveness**: Available if ≥ threshold servers online
- **User Choice**: You choose which servers to trust
- **Identity-Based**: Encryption tied to [PackageID][Identity]

### 2. Verifiable TEE (Nautilus)

**Problem:** "Trust us, we computed correctly" - no proof

**Solution:** Self-managed TEE with on-chain attestation

**How it works:**
```
┌─────────────────────────────────────────────────┐
│         Developer Deploys Enclave               │
└────────────────┬────────────────────────────────┘
                 ↓
        ┌────────┴────────────────┐
        │  1. Build Code          │
        │     Source → Binary     │
        │                         │
        │  2. Measure Binary      │
        │     Generate PCRs       │
        │     (Hash of code)      │
        └────────┬────────────────┘
                 ↓
┌────────────────┴────────────────────────────────┐
│         3. Deploy to AWS Nitro Enclave          │
│                                                 │
│    ┌─────────────────────────────────┐         │
│    │   Enclave (Isolated VM)         │         │
│    │                                 │         │
│    │   - Runs code                   │         │
│    │   - No external access          │         │
│    │   - Generates attestation       │         │
│    │   - Signs with private key      │         │
│    └─────────────────────────────────┘         │
└────────────────┬────────────────────────────────┘
                 ↓
        ┌────────┴────────────────┐
        │  4. Register On-Chain   │
        │     - Store PCR values  │
        │     - Store public key  │
        │     - Verify attestation│
        └────────┬────────────────┘
                 ↓
┌────────────────┴────────────────────────────────┐
│         5. Users Verify                         │
│                                                 │
│  Anyone can:                                    │
│  - Build same code from source                  │
│  - Get same PCR values                          │
│  - Compare with on-chain PCRs                   │
│  - Prove code matches source                    │
└─────────────────────────────────────────────────┘
```

**Key Properties:**
- **Verifiable**: Reproducible builds prove source code
- **Isolated**: Code runs in secure, tamper-proof environment
- **Self-managed**: You deploy and control the TEE
- **Transparent**: Anyone can verify PCR values match source

### 3. Cost-Efficient Storage (Walrus)

**Problem:** Storing data on-chain is expensive

**Solution:** Decentralized storage with erasure coding

**Cost Comparison:**
```
Scenario: Store 1 MB order metadata

Traditional Blockchain:
┌──────────────────────────────────────┐
│  Replication: 100x (100 validators)  │
│  Effective size: 100 MB              │
│  Cost: $1,000                        │
└──────────────────────────────────────┘

Walrus:
┌──────────────────────────────────────┐
│  Erasure coding: 5x                  │
│  Effective size: 5 MB                │
│  Cost: $50                           │
│                                      │
│  Savings: 95%! ✓                     │
└──────────────────────────────────────┘

On-chain reference:
┌──────────────────────────────────────┐
│  Store only blob ID: 32 bytes        │
│  Cost: $0.001                        │
└──────────────────────────────────────┘
```

**How it works:**
```
1. Encode Data
   ┌─────────┐
   │ 1 MB    │ → Erasure code → ┌─────────────┐
   │ data    │                   │ 5 MB shards │
   └─────────┘                   └─────────────┘

2. Distribute Shards
   Shard 1 → Storage Node A
   Shard 2 → Storage Node B
   Shard 3 → Storage Node C
   ...

3. Store Reference On-Chain
   Blob ID: 0xabc123... (32 bytes)

4. Retrieve Data
   - Fetch 1/3 of shards (any combination)
   - Reconstruct original 1 MB
   - Verify integrity with Blob ID
```

**Key Properties:**
- **Decentralized**: No single point of failure
- **Efficient**: 5x overhead vs 100x+ replication
- **Secure**: Can tolerate up to 1/3 Byzantine nodes
- **Available**: Reconstruct from any 1/3 of shards

### 4. Stealth Addresses

**Problem:** Recipient address visible on blockchain

**Solution:** Generate one-time address per payment

**How it works:**
```
Setup Phase:
┌─────────────────────────────────────────────────┐
│  Bob publishes:                                 │
│  - Scan key (public)                            │
│  - Spend key (public)                           │
└─────────────────────────────────────────────────┘

Payment Phase:
┌─────────────────────────────────────────────────┐
│  1. Alice generates ephemeral key pair          │
│     (ephemeral_private, ephemeral_public)       │
│                                                 │
│  2. Alice computes shared secret (ECDH)         │
│     shared_secret = ECDH(ephemeral_private,     │
│                          Bob's_scan_key)        │
│                                                 │
│  3. Alice derives stealth address               │
│     stealth_address = derive(shared_secret,     │
│                              Bob's_spend_key)   │
│                                                 │
│  4. Alice sends payment to stealth_address      │
│     + includes ephemeral_public on-chain        │
└─────────────────────────────────────────────────┘

Discovery Phase:
┌─────────────────────────────────────────────────┐
│  Bob scans blockchain:                          │
│                                                 │
│  For each payment:                              │
│    1. Compute shared_secret using               │
│       his scan_private + ephemeral_public       │
│                                                 │
│    2. Check if derived address matches          │
│                                                 │
│    3. If match: This payment is for him!        │
│                                                 │
│    4. Compute private key to spend              │
└─────────────────────────────────────────────────┘
```

**What observers see:**
```
On-Chain Data:
  - Payment to address: 0xStealthAddress123...
  - Ephemeral public key: 0xEphemeral456...
  - Encrypted amount: [sealed data]

What they DON'T know:
  ❌ Who is the recipient?
  ❌ How much was sent?
  ❌ Can Bob spend this?
```

---

## Architecture Overview

### System Components

```
┌─────────────────────────────────────────────────────────────────┐
│                         USER LAYER                              │
│                                                                 │
│  Components:                                                    │
│  - Sui Wallet (Sui Wallet, Suiet, Ethos)                      │
│  - Web Frontend (Next.js)                                      │
│  - dApp Kit (@mysten/dapp-kit)                                │
│                                                                 │
│  User Actions:                                                  │
│  - Enter swap amount                                           │
│  - Sign transactions                                           │
│  - Scan for stealth payments                                   │
│  - Claim swapped tokens                                        │
└────────────────────┬────────────────────────────────────────────┘
                     │
                     ↓
┌─────────────────────────────────────────────────────────────────┐
│                    ENCRYPTION LAYER                             │
│                                                                 │
│  ┌──────────────────────────────────────────────────┐          │
│  │              Seal Key Servers                    │          │
│  │                                                  │          │
│  │  Server 1 (US-East)    - BLS key 1              │          │
│  │  Server 2 (EU-West)    - BLS key 2              │          │
│  │  Server 3 (Asia-Pac)   - BLS key 3              │          │
│  │                                                  │          │
│  │  Threshold: 2-of-3                               │          │
│  │  Identity: [PackageID][OrderID]                  │          │
│  └──────────────────────────────────────────────────┘          │
│                                                                 │
│  Functions:                                                     │
│  - Encrypt amounts (100 USDC)                                  │
│  - Return encrypted handles                                    │
│  - Provide decryption keys (when authorized)                   │
│  - Verify access policies on-chain                             │
└────────────────────┬────────────────────────────────────────────┘
                     │
                     ↓
┌─────────────────────────────────────────────────────────────────┐
│                   COMPUTATION LAYER                             │
│                                                                 │
│  ┌──────────────────────────────────────────────────┐          │
│  │         Nautilus TEE (AWS Nitro Enclave)         │          │
│  │                                                  │          │
│  │  Responsibilities:                               │          │
│  │  - Aggregate encrypted orders                    │          │
│  │  - Decrypt batch total (via Seal)                │          │
│  │  - Execute DEX swap                              │          │
│  │  - Re-encrypt output                             │          │
│  │  - Sign results                                  │          │
│  │                                                  │          │
│  │  Verification:                                   │          │
│  │  - PCR values stored on-chain                    │          │
│  │  - Reproducible builds                           │          │
│  │  - Attestation documents                         │          │
│  └──────────────────────────────────────────────────┘          │
└────────────────────┬────────────────────────────────────────────┘
                     │
                     ↓
┌─────────────────────────────────────────────────────────────────┐
│                    BLOCKCHAIN LAYER (Sui)                       │
│                                                                 │
│  ┌──────────────────────────────────────────────────┐          │
│  │              Shared Objects                      │          │
│  │                                                  │          │
│  │  1. OrderManager                                 │          │
│  │     - Stores all encrypted orders                │          │
│  │     - Manages trading epochs                     │          │
│  │     - Tracks settlement status                   │          │
│  │                                                  │          │
│  │  2. EnclaveConfig                                │          │
│  │     - Stores PCR values (code hashes)            │          │
│  │     - Version management                         │          │
│  │                                                  │          │
│  │  3. Enclave Registry                             │          │
│  │     - Active enclave instances                   │          │
│  │     - Public keys for verification               │          │
│  │                                                  │          │
│  │  4. Solver (Liquidity Vault)                     │          │
│  │     - USDC/USDT reserves                         │          │
│  │     - Executes token swaps                       │          │
│  │                                                  │          │
│  │  5. Seal Key Server Registry                     │          │
│  │     - Server URLs and public keys                │          │
│  │     - Weights for threshold                      │          │
│  └──────────────────────────────────────────────────┘          │
│                                                                 │
│  Move Smart Contracts:                                          │
│  - order_manager.move (order logic)                            │
│  - seal_policy.move (access control)                           │
│  - nautilus_verifier.move (TEE verification)                   │
│  - stealth_payment.move (recipient privacy)                    │
│  - solver.move (liquidity management)                          │
└────────────────────┬────────────────────────────────────────────┘
                     │
                     ↓
┌─────────────────────────────────────────────────────────────────┐
│                     STORAGE LAYER                               │
│                                                                 │
│  ┌──────────────────────────────────────────────────┐          │
│  │                Walrus Storage                    │          │
│  │                                                  │          │
│  │  Stores:                                         │          │
│  │  - Large order metadata                          │          │
│  │  - Historical data                               │          │
│  │  - Encrypted user preferences                    │          │
│  │                                                  │          │
│  │  Returns: 32-byte blob IDs                       │          │
│  │  On-chain: Only blob IDs stored                  │          │
│  └──────────────────────────────────────────────────┘          │
└────────────────────┬────────────────────────────────────────────┘
                     │
                     ↓
┌─────────────────────────────────────────────────────────────────┐
│                      DEX LAYER                                  │
│                                                                 │
│  ┌──────────────────────────────────────────────────┐          │
│  │              Cetus DEX (CLMM)                    │          │
│  │                                                  │          │
│  │  Features:                                       │          │
│  │  - Concentrated liquidity pools                  │          │
│  │  - Multiple fee tiers                            │          │
│  │  - Deep liquidity                                │          │
│  │                                                  │          │
│  │  Process:                                        │          │
│  │  1. Receive decrypted batch total               │          │
│  │  2. Execute swap: USDC → USDT                    │          │
│  │  3. Return output amount                         │          │
│  └──────────────────────────────────────────────────┘          │
└─────────────────────────────────────────────────────────────────┘
```

### Data Flow Layers

```
┌──────────────────────────────────────────────────────────┐
│  Layer 1: USER INPUT                                     │
│  "I want to swap 100 USDC for USDT"                     │
└────────────────────┬─────────────────────────────────────┘
                     ↓
┌──────────────────────────────────────────────────────────┐
│  Layer 2: ENCRYPTION                                     │
│  100 USDC → Seal (2-of-3) → Encrypted Handle            │
└────────────────────┬─────────────────────────────────────┘
                     ↓
┌──────────────────────────────────────────────────────────┐
│  Layer 3: ON-CHAIN SUBMISSION                            │
│  Transaction → OrderManager → Store encrypted order       │
└────────────────────┬─────────────────────────────────────┘
                     ↓
┌──────────────────────────────────────────────────────────┐
│  Layer 4: BATCH AGGREGATION                              │
│  Nautilus TEE aggregates multiple orders                 │
│  Alice (50) + Bob (75) + You (100) = Total (225)        │
└────────────────────┬─────────────────────────────────────┘
                     ↓
┌──────────────────────────────────────────────────────────┐
│  Layer 5: DECRYPTION (in TEE)                            │
│  Seal servers provide keys → Decrypt total: 225 USDC     │
└────────────────────┬─────────────────────────────────────┘
                     ↓
┌──────────────────────────────────────────────────────────┐
│  Layer 6: DEX EXECUTION                                  │
│  Cetus swap: 225 USDC → 224.98 USDT (after fees)        │
└────────────────────┬─────────────────────────────────────┘
                     ↓
┌──────────────────────────────────────────────────────────┐
│  Layer 7: RE-ENCRYPTION                                  │
│  224.98 USDT → Seal → New encrypted handles              │
│  Split proportionally to users                           │
└────────────────────┬─────────────────────────────────────┘
                     ↓
┌──────────────────────────────────────────────────────────┐
│  Layer 8: SETTLEMENT                                     │
│  Update on-chain: Users can claim encrypted balances     │
└────────────────────┬─────────────────────────────────────┘
                     ↓
┌──────────────────────────────────────────────────────────┐
│  Layer 9: USER CLAIMS                                    │
│  User decrypts balance → Claims 99.99 USDT               │
└──────────────────────────────────────────────────────────┘
```

---

## Sui Object Model

### Object Categories

**1. Configuration Objects (Shared, Admin-Controlled)**
```
EnclaveConfig
├─ Purpose: Store TEE software measurements (PCR values)
├─ Lifecycle: Created once, updated with new versions
├─ Access: Anyone can read, only admin can update
└─ Contains:
   ├─ PCR0, PCR1, PCR2 (software hashes)
   ├─ Version number
   └─ Active status

SealKeyServerConfig
├─ Purpose: Registry of authorized Seal servers
├─ Lifecycle: Created at deployment, updated by governance
├─ Access: Public read, admin write
└─ Contains:
   ├─ Server URLs
   ├─ BLS public keys
   └─ Weight (threshold contribution)
```

**2. Instance Objects (Shared, Dynamic)**
```
OrderManager
├─ Purpose: Manage all swap orders for current epoch
├─ Lifecycle: Persistent, state changes per epoch
├─ Access: Anyone can read, authorized relayer can modify
└─ Contains:
   ├─ Current epoch number
   ├─ Vector of orders
   ├─ Aggregated total (encrypted)
   ├─ Epoch status (Pending/Aggregated/Settled)
   ├─ Relayer address
   └─ Solver address

Enclave (per instance)
├─ Purpose: Represent one deployed TEE instance
├─ Lifecycle: Created at registration, can be deactivated
├─ Access: Public read, admin can deactivate
└─ Contains:
   ├─ Config version reference
   ├─ Public key (for signature verification)
   ├─ Registration timestamp
   └─ Active status

Solver (Liquidity Vault)
├─ Purpose: Hold reserves and execute swaps
├─ Lifecycle: Persistent liquidity pool
├─ Access: Public read, only OrderManager can execute swaps
└─ Contains:
   ├─ USDC balance
   ├─ USDT balance
   ├─ Owner address
   └─ Fee parameters
```

**3. User-Specific Objects (Shared, User-Controlled)**
```
UserBalance
├─ Purpose: Store user's encrypted token balances
├─ Lifecycle: Created on first swap, updated per claim
├─ Access: Public read, only user can claim
└─ Contains:
   ├─ User address
   ├─ Token type
   ├─ Encrypted balance handle
   └─ Last updated timestamp

StealthPayment
├─ Purpose: One-time payment with hidden recipient
├─ Lifecycle: Created at payment, claimed by recipient
├─ Access: Anyone can scan, only recipient can claim
└─ Contains:
   ├─ Ephemeral public key (ECDH)
   ├─ Encrypted amount (Seal)
   ├─ Walrus blob ID (metadata)
   ├─ Timestamp
   ├─ Claimed status
   └─ Locked coins
```

**4. Transient Structs (Not Objects, Embedded Data)**
```
Order (within OrderManager)
├─ Purpose: Single swap order
├─ Storage: Inside OrderManager.orders vector
└─ Contains:
   ├─ User address
   ├─ Encrypted amount handle (u128)
   ├─ Deadline
   ├─ Timestamp
   └─ Claimed status

EncryptedData (within various objects)
├─ Purpose: Seal-encrypted data
├─ Storage: As bytes vector in parent object
└─ Contains:
   ├─ Ciphertext
   ├─ Encrypted shares
   ├─ Threshold value
   └─ Server references
```

### Object Relationships

```
┌──────────────────────────────────────────────────────────┐
│                    EnclaveConfig                         │
│  (Defines: What code is trusted)                         │
│  - PCR values                                            │
│  - Version                                               │
└────────────────────┬─────────────────────────────────────┘
                     │
                     │ references
                     ↓
┌──────────────────────────────────────────────────────────┐
│                     Enclave                              │
│  (Instance: Specific TEE running that code)              │
│  - Config version                                        │
│  - Public key                                            │
└────────────────────┬─────────────────────────────────────┘
                     │
                     │ used by
                     ↓
┌──────────────────────────────────────────────────────────┐
│                  OrderManager                            │
│  (Coordinates: All swap orders)                          │
│  - Orders vector                                         │
│  - Epoch total (encrypted)                               │
└────────────────────┬─────────────────────────────────────┘
                     │
                     │ contains
                     ↓
            ┌────────┴────────┐
            │      Order      │
            │  - User         │
            │  - Handle       │
            │  - Deadline     │
            └────────┬────────┘
                     │
                     │ belongs to
                     ↓
┌──────────────────────────────────────────────────────────┐
│                   UserBalance                            │
│  (Stores: User's encrypted balance)                      │
│  - User address                                          │
│  - Encrypted balance                                     │
└──────────────────────────────────────────────────────────┘

Parallel structure for Seal:

┌──────────────────────────────────────────────────────────┐
│              SealKeyServerConfig                         │
│  (Registry: Trusted key servers)                         │
│  - Server 1, 2, 3 info                                   │
└────────────────────┬─────────────────────────────────────┘
                     │
                     │ used for encryption/decryption
                     ↓
            All encrypted data
            in orders, balances, etc.
```

### Object Access Patterns

**Public Readable:**
- All objects (transparency)
- Anyone can see encrypted handles
- Cannot decrypt without authorization

**User Writable:**
- Place order (any user)
- Claim tokens (only order owner)
- Stealth payment (sender creates, recipient claims)

**Admin Writable:**
- Update EnclaveConfig (admin cap holder)
- Register new Enclave (admin)
- Update Seal server registry (governance)

**Relayer Writable:**
- Aggregate orders (authorized relayer only)
- Settle orders (authorized relayer only)
- Must verify TEE signatures

---

## Technology Integration

### Seal Integration Points

**1. Encryption (Client-Side)**
```
User Flow:
  Enter amount (100 USDC)
       ↓
  Choose Seal servers (1, 2, 3)
       ↓
  Set threshold (2-of-3)
       ↓
  Seal SDK encrypts
       ↓
  Returns: Encrypted object + backup key
       ↓
  Submit encrypted handle on-chain
```

**2. Access Control (On-Chain)**
```
Move Contract Defines Policies:

seal_approve_timelock:
  - Decrypt after specific time
  - Used for: time-locked reveals, auctions

seal_approve_owner:
  - Only owner can decrypt
  - Used for: user balances, private data

seal_approve_allowlist:
  - Members of allowlist can decrypt
  - Used for: shared access, subscriptions
```

**3. Decryption (Session-Based)**
```
User Flow:
  Create session key (wallet signs)
       ↓
  Build transaction with seal_approve call
       ↓
  Seal servers evaluate transaction
       ↓
  If approved: Return decryption keys
       ↓
  Client decrypts locally
```

### Nautilus Integration Points

**1. Enclave Deployment**
```
Developer Flow:
  Write enclave code (Rust)
       ↓
  Build reproducibly
       ↓
  Measure: Generate PCR values
       ↓
  Deploy to AWS Nitro Enclave
       ↓
  Register PCRs on-chain
       ↓
  Register enclave instance with attestation
```

**2. Computation Verification**
```
Enclave Process:
  Receive encrypted orders
       ↓
  Decrypt with Seal (session key)
       ↓
  Aggregate: Sum all amounts
       ↓
  Sign result with enclave key
       ↓
  Return: Encrypted total + signature

On-Chain Verification:
  Verify signature with enclave's public key
       ↓
  Check enclave is registered and active
       ↓
  Check PCRs match trusted version
       ↓
  Accept result
```

**3. Reproducible Builds**
```
Verification Flow:
  Download source code
       ↓
  Build locally
       ↓
  Generate PCR values
       ↓
  Compare with on-chain PCRs
       ↓
  If match: Code is verified
```

### Walrus Integration Points

**1. Data Storage**
```
Store Large Data:
  Order metadata
       ↓
  Erasure encode (5x)
       ↓
  Distribute shards to storage nodes
       ↓
  Get blob ID (32 bytes)
       ↓
  Store blob ID on-chain
```

**2. Data Retrieval**
```
Fetch Data:
  Read blob ID from chain
       ↓
  Request shards from storage nodes
       ↓
  Receive 1/3+ of shards (any combination)
       ↓
  Reconstruct original data
       ↓
  Verify with blob ID
```

**3. Cost Optimization**
```
What to Store on Walrus:
  ✓ Large order metadata
  ✓ Historical transaction data
  ✓ User preferences
  ✓ Analytics data

What to Keep On-Chain:
  ✓ Encrypted handles (small)
  ✓ Blob IDs (32 bytes)
  ✓ User addresses
  ✓ Timestamps
```

### Cetus Integration Points

**1. Pool Selection**
```
Choose Pool:
  Token pair: USDC/USDT
       ↓
  Fee tier: 0.01% (most efficient)
       ↓
  Verify liquidity depth
       ↓
  Check price impact
```

**2. Swap Execution**
```
Execute Swap:
  Decrypt batch total (225 USDC)
       ↓
  Call Cetus pool with amount
       ↓
  Specify: exact input, minimum output
       ↓
  Receive: 224.98 USDT (after 0.01% fee)
```

**3. Result Processing**
```
Post-Swap:
  Get output amount
       ↓
  Re-encrypt with Seal
       ↓
  Split proportionally to users
       ↓
  Update on-chain balances (encrypted)
```

---

## Swap Flow Design

### Phase 1: Order Placement

```
┌────────────────────────────────────────────────────────┐
│  USER                                                  │
│                                                        │
│  1. Connect Sui wallet                                 │
│  2. Enter: 100 USDC → USDT                            │
│  3. Click "Swap Privately"                            │
└────────────────┬───────────────────────────────────────┘
                 ↓
┌────────────────────────────────────────────────────────┐
│  FRONTEND                                              │
│                                                        │
│  1. Validate input                                     │
│  2. Check user balance                                 │
│  3. Prepare swap parameters                            │
└────────────────┬───────────────────────────────────────┘
                 ↓
┌────────────────────────────────────────────────────────┐
│  SEAL ENCRYPTION                                       │
│                                                        │
│  1. Contact 3 key servers                              │
│  2. Encrypt 100 with threshold 2-of-3                  │
│  3. Get encrypted handle                               │
│  4. Store backup key (optional)                        │
└────────────────┬───────────────────────────────────────┘
                 ↓
┌────────────────────────────────────────────────────────┐
│  (OPTIONAL) WALRUS STORAGE                             │
│                                                        │
│  1. Prepare metadata (user, time, params)              │
│  2. Store on Walrus                                    │
│  3. Get blob ID (32 bytes)                             │
└────────────────┬───────────────────────────────────────┘
                 ↓
┌────────────────────────────────────────────────────────┐
│  SUI TRANSACTION                                       │
│                                                        │
│  Transaction:                                          │
│    Call: order_manager::place_order                    │
│    Args:                                               │
│      - OrderManager (shared object)                    │
│      - deadline (u64)                                  │
│      - encrypted_handle (u128)                         │
│      - Clock (shared object 0x6)                       │
│                                                        │
│  User signs → Submit → Wait for confirmation           │
└────────────────┬───────────────────────────────────────┘
                 ↓
┌────────────────────────────────────────────────────────┐
│  ON-CHAIN EXECUTION                                    │
│                                                        │
│  OrderManager receives:                                │
│    - New order with encrypted handle                   │
│    - Add to orders vector                              │
│    - Emit OrderPlaced event                            │
│                                                        │
│  Result: Order stored, waiting for batch               │
└────────────────────────────────────────────────────────┘
```

### Phase 2: Order Aggregation

```
┌────────────────────────────────────────────────────────┐
│  EVENT MONITORING                                      │
│                                                        │
│  Backend watches:                                      │
│    - OrderPlaced events                                │
│    - Count orders in current epoch                     │
│    - Check threshold (e.g., 3+ orders)                 │
│                                                        │
│  Trigger: Threshold met → Start aggregation            │
└────────────────┬───────────────────────────────────────┘
                 ↓
┌────────────────────────────────────────────────────────┐
│  FETCH ORDERS                                          │
│                                                        │
│  Backend queries OrderManager:                         │
│    - Get all orders for current epoch                  │
│    - Extract encrypted handles                         │
│                                                        │
│  Example:                                              │
│    Order 1: Alice  - handle_A (Encrypt(50))           │
│    Order 2: Bob    - handle_B (Encrypt(75))           │
│    Order 3: You    - handle_C (Encrypt(100))          │
└────────────────┬───────────────────────────────────────┘
                 ↓
┌────────────────────────────────────────────────────────┐
│  NAUTILUS TEE COMPUTATION                              │
│                                                        │
│  Send to enclave:                                      │
│    - List of encrypted handles                         │
│                                                        │
│  Inside TEE (AWS Nitro Enclave):                       │
│    1. Request decryption keys from Seal                │
│       (enclave has session key)                        │
│                                                        │
│    2. Decrypt each handle:                             │
│       handle_A → 50 USDC                               │
│       handle_B → 75 USDC                               │
│       handle_C → 100 USDC                              │
│                                                        │
│    3. Aggregate:                                       │
│       total = 50 + 75 + 100 = 225 USDC                │
│                                                        │
│    4. Re-encrypt total with Seal:                      │
│       225 → Encrypt(225) → new_handle                  │
│                                                        │
│    5. Sign with enclave key:                           │
│       signature = sign(new_handle, enclave_key)        │
│                                                        │
│  Return:                                               │
│    - Encrypted total handle                            │
│    - Signature                                         │
└────────────────┬───────────────────────────────────────┘
                 ↓
┌────────────────────────────────────────────────────────┐
│  SUBMIT AGGREGATION ON-CHAIN                           │
│                                                        │
│  Transaction:                                          │
│    Call: order_manager::aggregate_orders               │
│    Args:                                               │
│      - OrderManager (shared)                           │
│      - Enclave (shared, for verification)              │
│      - aggregated_handle (u128)                        │
│      - signature (vector<u8>)                          │
│                                                        │
│  On-chain verification:                                │
│    1. Verify enclave signature                         │
│    2. Check enclave is active                          │
│    3. Store aggregated total                           │
│    4. Update status to "Aggregated"                    │
│    5. Emit OrdersAggregated event                      │
└────────────────────────────────────────────────────────┘
```

### Phase 3: DEX Execution

```
┌────────────────────────────────────────────────────────┐
│  DECRYPT FOR DEX                                       │
│                                                        │
│  Backend/Enclave:                                      │
│    1. Has encrypted total: Encrypt(225)                │
│                                                        │
│    2. Request Seal decryption:                         │
│       - Create session key                             │
│       - Build seal_approve transaction                 │
│       - Get keys from 2+ Seal servers                  │
│                                                        │
│    3. Decrypt:                                         │
│       Encrypt(225) → 225 USDC (plaintext)              │
└────────────────┬───────────────────────────────────────┘
                 ↓
┌────────────────────────────────────────────────────────┐
│  CETUS DEX SWAP                                        │
│                                                        │
│  Transaction:                                          │
│    Call: cetus::pool::swap                             │
│    Args:                                               │
│      - Pool object (USDC/USDT, 0.01% fee)             │
│      - Amount in: 225 USDC                             │
│      - Minimum out: 220 USDT (slippage protection)     │
│      - Direction: a_to_b (USDC → USDT)                │
│                                                        │
│  Cetus executes:                                       │
│    1. Transfer 225 USDC from caller                    │
│    2. Calculate output (considering price & fees)      │
│    3. Transfer 224.98 USDT to caller                   │
│                                                        │
│  Result: 224.98 USDT received                          │
└────────────────┬───────────────────────────────────────┘
                 ↓
┌────────────────────────────────────────────────────────┐
│  RE-ENCRYPT OUTPUT                                     │
│                                                        │
│  Enclave:                                              │
│    1. Has: 224.98 USDT (plaintext)                     │
│                                                        │
│    2. Calculate per-user amounts:                      │
│       Total input: 225 USDC                            │
│       Alice: 50/225 * 224.98 = 49.99 USDT              │
│       Bob:   75/225 * 224.98 = 74.99 USDT              │
│       You:  100/225 * 224.98 = 99.99 USDT              │
│                                                        │
│    3. Encrypt each with Seal:                          │
│       49.99 → handle_A'                                │
│       74.99 → handle_B'                                │
│       99.99 → handle_C'                                │
│                                                        │
│    4. Sign distribution:                               │
│       signature = sign([handle_A', B', C'], key)       │
└────────────────┬───────────────────────────────────────┘
                 ↓
┌────────────────────────────────────────────────────────┐
│  SETTLE ON-CHAIN                                       │
│                                                        │
│  Transaction:                                          │
│    Call: order_manager::settle_orders                  │
│    Args:                                               │
│      - OrderManager (shared)                           │
│      - Solver (shared, for token transfers)            │
│      - User handles: [handle_A', B', C']               │
│      - Signature                                       │
│                                                        │
│  On-chain execution:                                   │
│    1. Verify signature                                 │
│    2. Update user balances (encrypted)                 │
│    3. Mark epoch as "Settled"                          │
│    4. Emit OrdersSettled event                         │
└────────────────────────────────────────────────────────┘
```

### Phase 4: User Claims

```
┌────────────────────────────────────────────────────────┐
│  USER VIEWS BALANCE                                    │
│                                                        │
│  Frontend:                                             │
│    1. Query UserBalance object                         │
│    2. See encrypted handle                             │
│    3. Show: "Balance: [Encrypted]"                     │
│                                                        │
│  User clicks "Decrypt & Claim"                         │
└────────────────┬───────────────────────────────────────┘
                 ↓
┌────────────────────────────────────────────────────────┐
│  DECRYPT BALANCE (CLIENT-SIDE)                         │
│                                                        │
│  Process:                                              │
│    1. Create session key (wallet signs)                │
│                                                        │
│    2. Build seal_approve transaction:                  │
│       - Call seal_approve_owner                        │
│       - Prove ownership                                │
│                                                        │
│    3. Seal servers evaluate:                           │
│       - Check if user == owner                         │
│       - If yes: Return decryption keys                 │
│                                                        │
│    4. Decrypt locally:                                 │
│       handle_C' → 99.99 USDT                           │
│                                                        │
│  Display: "Your balance: 99.99 USDT"                   │
└────────────────┬───────────────────────────────────────┘
                 ↓
┌────────────────────────────────────────────────────────┐
│  CLAIM TOKENS                                          │
│                                                        │
│  Transaction:                                          │
│    Call: order_manager::claim_tokens                   │
│    Args:                                               │
│      - OrderManager (shared)                           │
│      - UserBalance (shared)                            │
│                                                        │
│  On-chain execution:                                   │
│    1. Verify order settled                             │
│    2. Find user's order                                │
│    3. Mark as claimed                                  │
│    4. Transfer USDT tokens to user                     │
│       (from Solver vault)                              │
│                                                        │
│  Result:                                               │
│    User receives 99.99 USDT in wallet                  │
└────────────────────────────────────────────────────────┘
```

---

## Privacy Model

### What Information is Hidden

**Amount Privacy (via Seal):**
```
Visible on-chain:
  ✓ Order exists
  ✓ Encrypted handle: 9876543210...
  ✓ User address: 0xYourAddress...
  ✓ Timestamp

Hidden (encrypted):
  ✗ Actual amount: 100 USDC
  ✗ User balance: 99.99 USDT
  ✗ Batch total: 225 USDC
  ✗ Individual splits
```

**Recipient Privacy (via Stealth Addresses):**
```
Visible on-chain:
  ✓ Payment to stealth address
  ✓ Ephemeral public key
  ✓ Encrypted amount

Hidden:
  ✗ Who is the recipient?
  ✗ Can recipient spend?
  ✗ Link to recipient's identity
```

**Computation Privacy (via Nautilus):**
```
Visible:
  ✓ Enclave public key
  ✓ PCR values (code hash)
  ✓ Signature on result

Hidden (inside TEE):
  ✗ Decrypted amounts during aggregation
  ✗ Intermediate computations
  ✗ Private keys used for signing
```

### Privacy Degradation Points

**Where Privacy is Lost:**

**1. Final DEX Execution**
```
Problem:
  Cetus DEX is public → sees batch total

Impact:
  - Observers see: "Someone swapped 225 USDC"
  - Don't know: Who, how many people, individual amounts

Mitigation:
  - Batch multiple orders → hide individual size
  - Random delay → harder to correlate
```

**2. Network Analysis**
```
Problem:
  Transaction timing correlation

Impact:
  - If only 3 orders placed in epoch
  - Then 3 claims happen → link orders to users

Mitigation:
  - Larger batches
  - Delayed claims
  - Dummy transactions
```

**3. Amount Correlation**
```
Problem:
  Unique amounts are fingerprintable

Impact:
  - If you swap 100.0001 USDC (unique)
  - Later claim 99.9901 USDT
  - Can correlate input/output

Mitigation:
  - Round amounts
  - Add random noise
  - Use standard denominations
```

### Privacy Guarantees

**Strong Guarantees:**
- ✅ Amounts hidden from blockchain observers
- ✅ Amounts hidden from Seal servers (< threshold)
- ✅ Recipients hidden with stealth addresses
- ✅ Computation integrity verified
- ✅ No single point of decryption

**Moderate Guarantees:**
- ⚠️ Addresses visible (sender & receiver of orders)
- ⚠️ Batch total visible during DEX execution
- ⚠️ Timing analysis possible with small batches

**No Guarantees:**
- ❌ Not Zcash-level anonymity
- ❌ Not hiding transaction graph
- ❌ Not hiding token types (USDC/USDT)

---

## Trust & Security

### Trust Model

**Who You Trust:**

**Threshold Trust (Seal):**
```
Seal Key Servers (2-of-3)

You trust that:
  - Fewer than 2 servers are compromised
  - Servers verify access policies correctly
  - Servers don't collude

Risk if broken:
  - Privacy loss (amounts revealed)
  - No fund loss (just decryption)
```

**Verifiable Trust (Nautilus):**
```
TEE Enclave

You trust:
  - AWS Nitro Enclave security
  - Reproducible builds match source
  - PCR verification is correct

You can verify:
  ✓ Build code yourself
  ✓ Compare PCR values
  ✓ Check attestation documents

Risk if broken:
  - Wrong aggregation
  - Incorrect splits
```

**Byzantine Trust (Walrus):**
```
Storage Nodes (< 1/3 Byzantine)

You trust:
  - More than 2/3 nodes honest
  - Erasure coding is correct
  - Blob IDs authentic

Security:
  ✓ Can reconstruct from any 1/3
  ✓ Verifiable with blob ID
  ✓ Decentralized

Risk if broken:
  - Data loss (not fund loss)
```

**Smart Contract Trust:**
```
Move Contracts (Audited)

You trust:
  - Sui validators (consensus)
  - Move type system (safety)
  - Contract logic (audited)

Security:
  ✓ Open source
  ✓ Formally verified (optional)
  ✓ Immutable (once deployed)
```

### Attack Scenarios

**Attack 1: Seal Server Compromise**
```
Scenario:
  Attacker compromises 2+ Seal servers

Impact:
  - Can decrypt all encrypted amounts
  - Privacy breach

Mitigation:
  - Choose diverse server operators
  - Different jurisdictions
  - Regular key rotation
  - User controls server selection
```

**Attack 2: Nautilus Enclave Exploit**
```
Scenario:
  Zero-day in AWS Nitro Enclaves

Impact:
  - Attacker could manipulate aggregation
  - Steal decryption keys

Mitigation:
  - PCR verification (detects code changes)
  - Attestation documents (verifiable)
  - AWS security team response
  - Can deactivate compromised enclave on-chain
```

**Attack 3: Front-Running**
```
Scenario:
  MEV bot sees large order event, front-runs on Cetus

Impact:
  - Price manipulation
  - User gets worse execution

Mitigation:
  - Amounts are encrypted (bot can't size)
  - Batch execution (harder to predict)
  - Private mempool (optional)
```

**Attack 4: Timing Analysis**
```
Scenario:
  Observer correlates order placement with claims

Impact:
  - Can link users to orders
  - Partial deanonymization

Mitigation:
  - Larger batches (more mixing)
  - Delayed claims (break timing)
  - Dummy transactions (noise)
```

**Attack 5: Walrus Data Loss**
```
Scenario:
  > 1/3 storage nodes fail simultaneously

Impact:
  - Cannot reconstruct metadata
  - Orders still on-chain (safe)

Mitigation:
  - Erasure coding (tolerates 2/3 failure)
  - Redundancy across diverse nodes
  - Fallback: store critical data on-chain
```

---

## Scalability & Performance

### Throughput Analysis

**Orders Per Epoch:**
```
Current design: Batching every N orders

Bottlenecks:
  1. Nautilus TEE computation: ~1 second
  2. Seal encryption/decryption: ~200ms per handle
  3. Sui transaction confirmation: ~0.5 seconds

Estimates:
  Small batch (10 orders):  ~5 seconds
  Medium batch (100 orders): ~25 seconds
  Large batch (1000 orders): ~210 seconds

Optimization:
  - Parallel Seal operations
  - Pre-compute aggregations
  - Optimize TEE code
```

**Cost Analysis:**
```
Per-Order Costs:

1. Seal Encryption:
   - 2-of-3 threshold
   - Cost: ~$0.001 (server fees)

2. Walrus Storage (optional):
   - 1 KB metadata
   - Cost: ~$0.0001 (5x coding)

3. Sui Transaction:
   - Gas: ~0.001 SUI (~$0.001)

4. Nautilus Computation:
   - AWS Nitro: ~$0.10/hour
   - Per order: ~$0.0001 (amortized)

Total per order: ~$0.0021

Compare to:
  - Regular DEX: ~$0.50 (gas)
  - Tornado Cash: ~$2-5 (gas + relayer)
```

### Scalability Solutions

**Horizontal Scaling:**
```
Multiple OrderManager Instances:
  - OrderManager_USDC_USDT_1
  - OrderManager_USDC_USDT_2
  - OrderManager_USDC_USDT_3

Benefits:
  - Parallel order processing
  - Reduced contention on shared objects
  - Load distribution

Challenges:
  - Liquidity fragmentation
  - User experience (which to use?)
```

**Optimistic Aggregation:**
```
Idea:
  Aggregate orders optimistically without full decryption

Process:
  1. Estimate total from recent averages
  2. Execute DEX swap
  3. Adjust if estimate was wrong

Benefits:
  - Faster execution
  - Reduced Seal calls

Risks:
  - Wrong estimates → failed swaps
  - More complex error handling
```

**Lazy Decryption:**
```
Idea:
  Users decrypt balances only when needed

Benefits:
  - Reduced Seal server load
  - Better privacy (less decryption requests)
  - Lower costs

Implementation:
  - Store encrypted in UserBalance
  - Decrypt on-demand in UI
  - Cache decrypted values locally
```

---

## Conclusion

### Key Design Principles

**1. No Single Point of Failure**
- Seal: 2-of-3 threshold
- Walrus: 1/3 reconstruction
- Multiple Nautilus instances

**2. Verifiable at Every Step**
- Seal: On-chain policies
- Nautilus: PCR values + attestations
- Smart contracts: Open source Move code

**3. User Control**
- Choose Seal servers
- Choose when to decrypt
- Choose when to claim

**4. Cost Efficiency**
- Walrus: 20x cheaper than on-chain
- Batching: Amortize TEE costs
- Optimized gas usage

**5. Privacy by Design**
- Amounts encrypted by default
- Stealth addresses for recipients
- Minimal on-chain footprint

### Design Trade-offs

**Privacy vs Performance:**
```
More Privacy:
  + Stealth addresses
  + Larger batches
  - Slower execution
  - Higher complexity

Less Privacy:
  + Faster swaps
  + Simpler UX
  - Amounts visible
```

**Decentralization vs Efficiency:**
```
More Decentralized:
  + 5-of-9 Seal threshold
  + More storage nodes
  - Slower operations
  - Higher costs

Less Decentralized:
  + Single server
  + Faster
  - Single point of failure
```

**Trust vs Verification:**
```
More Verification:
  + ZK proofs for everything
  + On-chain decryption
  - Very expensive gas
  - Slower

Less Verification:
  + Trust TEE
  + Trust servers
  - Faster, cheaper
  - Some trust required
```

### Future Enhancements

**Phase 1 (Current Design):**
- ✅ Encrypted amounts (Seal)
- ✅ Verifiable aggregation (Nautilus)
- ✅ Cost-efficient storage (Walrus)
- ✅ Basic order matching

**Phase 2 (Next):**
- 🚀 Cetus DEX integration
- 🚀 Stealth addresses
- 🚀 Multi-hop routing
- 🚀 Advanced order types

**Phase 3 (Future):**
- 🔮 Zero-knowledge proofs
- 🔮 Cross-chain swaps
- 🔮 MEV-resistant mempool
- 🔮 Compliance modules

---

**End of Design Document**

This document describes the system architecture and design principles. For implementation details, see the code documentation.
