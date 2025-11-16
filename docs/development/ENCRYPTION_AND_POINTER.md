### High-Level System Design

```
┌─────────────────────────────────────────────────────────┐
│                   USER'S BROWSER                         │
│  - Encrypts amounts before sending to blockchain        │
│  - Decrypts to view own balances                        │
│  - Manages user's view key                              │
└─────────────────────────────────────────────────────────┘
                        ↕
┌─────────────────────────────────────────────────────────┐
│                  SUI BLOCKCHAIN                          │
│  - Stores pointers (32 bytes each)                      │
│  - Manages real token custody (pool)                    │
│  - NO access to plaintext amounts                       │
└─────────────────────────────────────────────────────────┘
                        ↕
┌─────────────────────────────────────────────────────────┐
│              TEE SERVER (AWS Nitro)                      │
│  - Decrypts encrypted data                              │
│  - Executes swaps on Cetus DEX                          │
│  - Re-encrypts results with fresh randomness            │
│  - Updates blockchain with new pointers                 │
└─────────────────────────────────────────────────────────┘
                        ↕
┌─────────────────────────────────────────────────────────┐
│         DATA AVAILABILITY LAYER (DA)                     │
│  - Stores full ciphertexts (large data)                 │
│  - Indexed by pointers                                  │
│  - Can be IPFS, Arweave, or database                    │
└─────────────────────────────────────────────────────────┘
```

## How It Compares to Encifher

### Encifher's Design (Solana)

From Encifher documentation:
> "The actual encrypted data lives off-chain in a secure layer, while the blockchain only sees lightweight cryptographically binded 'pointers' - ciphertext shadows"

### Our Implementation (Sui)

We replicate Encifher's exact architecture on Sui:

| Component | Encifher (Solana) | Our Implementation (Sui) |
|-----------|-------------------|--------------------------|
| **Blockchain** | Solana | Sui |
| **Storage Model** | Pointers on-chain | Pointers on-chain |
| **Encryption** | Threshold ElGamal | Threshold ElGamal |
| **DA Layer** | Off-chain storage | Off-chain storage (IPFS/DB) |
| **TEE** | AWS Nitro Enclaves | AWS Nitro Enclaves |
| **DEX Integration** | Jupiter | Cetus |
| **Token Pairs** | Any SPL tokens | SUI ↔ USDC (MVP) |

**Core Architecture: Identical** ✅

---

## Key Components

### 1. Ciphertext vs Pointer

#### Ciphertext (The Encrypted Data)
- **Size:** 128-256 bytes (LARGE)
- **Contains:** Actual encrypted amount + randomness + cryptographic data
- **Stored:** Off-chain in DA layer
- **Can decrypt:** ✅ Yes (with private key)
- **Example:** `[172, 45, 88, 91, 234, 12, 67, 89, ... 256 bytes]`

#### Pointer (Hash of Ciphertext)
- **Size:** 32 bytes (SMALL, fixed)
- **Contains:** SHA-256 hash of ciphertext
- **Stored:** On-chain in smart contract
- **Can decrypt:** ❌ No (it's just a hash)
- **Example:** `[abc123def456789abc123def456789...]`

#### Why Both?

```
Problem: Blockchain storage is expensive
Solution: Store small pointers on-chain, large ciphertexts off-chain

Analogy:
- Ciphertext = The actual safe (heavy, contains treasure)
- Pointer = Map coordinates to find the safe (small note)
```

### 2. Encrypted Tokens (eTokens)

#### EncryptedSUI
```move
struct EncryptedSUI has key, store {
    id: UID,
    balance_pointer: vector<u8>,  // 32-byte pointer
}
```

Users own eSUI tokens in their wallet. The token contains only a pointer to the encrypted balance, not the actual amount.

#### EncryptedUSDC
```move
struct EncryptedUSDC has key, store {
    id: UID,
    balance_pointer: vector<u8>,
}
```

### 3. Liquidity Pool

```move
struct LiquidityPool has key {
    id: UID,
    sui_balance: Balance<SUI>,      // Real SUI locked
    usdc_balance: Balance<USDC>,    // Real USDC locked
    tee_authority: address,          // Authorized TEE
    paused: bool,
}
```

The pool holds all real tokens while users hold encrypted IOUs (eTokens).

---

## Encryption Workflow

### 1. WRAP (Deposit)

**Scenario:** Alice deposits 500 SUI

```
CLIENT (Alice's Browser):
1. Encrypt 500 with public key
   → Ciphertext: [172, 45, 88, ... 256 bytes]
2. Compute pointer
   → Pointer: SHA256(Ciphertext) = [abc123... 32 bytes]
3. Upload ciphertext to DA
   → DA["abc123..."] = Ciphertext
4. Call blockchain
   → contract.wrap_sui(500 SUI, pointer)

BLOCKCHAIN:
5. Receive 500 real SUI ✓
6. Lock in pool
7. Create eSUI token with pointer
8. Transfer eSUI to Alice
9. Emit WrapEvent

TEE SERVER:
10. Listen to WrapEvent
11. Fetch ciphertext from DA using pointer
12. Decrypt: 500
13. Verify: encrypted amount == real deposit? ✓
```

**Result:**
- Pool has: 500 real SUI
- Alice has: eSUI token (pointer → encrypted "500")
- DA has: Ciphertext encrypting 500

### 2. VIEW BALANCE

**Scenario:** Alice checks her balance

```
CLIENT:
1. Query blockchain for Alice's eSUI objects
   → Get eSUI object
2. Extract pointer from object
   → pointer = [abc123...]
3. Fetch ciphertext from DA
   → DA[pointer] = Ciphertext
4. Decrypt locally with view key
   → Decrypt(Ciphertext) = 500 SUI
5. Display: "Your balance: 500 SUI"
```

**Key:** Only Alice can decrypt because only she has the view key!

### 3. SWAP

**Scenario:** Alice swaps 300 SUI → USDC

```
CLIENT:
1. Encrypt swap request: 300
2. Send to blockchain
   → contract.request_swap(encrypted_300)

BLOCKCHAIN:
3. Emit SwapRequestEvent

TEE SERVER:
4. See SwapRequestEvent
5. Fetch Alice's current balance from DA
   → Decrypt: 500 SUI
6. Decrypt swap request
   → 300 SUI
7. Validate: 300 ≤ 500? ✓
8. Execute on Cetus DEX
   → Swap 300 SUI → 600 USDC
9. Compute new balances
   → new_sui = 500 - 300 = 200
   → new_usdc = 0 + 600 = 600
10. Encrypt with FRESH randomness
    → C_sui = Encrypt(200, random1)
    → C_usdc = Encrypt(600, random2)
11. Compute new pointers
    → pointer_sui = Hash(C_sui)
    → pointer_usdc = Hash(C_usdc)
12. Upload to DA
    → DA[pointer_sui] = C_sui
    → DA[pointer_usdc] = C_usdc
13. Update blockchain
    → contract.update_after_swap(pointer_sui, pointer_usdc)

BLOCKCHAIN:
14. Verify caller is TEE ✓
15. Update Alice's eSUI pointer
16. Update Alice's eUSDC pointer
17. Emit SwapExecutedEvent
```

**Result:**
- Alice's eSUI now points to encrypted "200"
- Alice's eUSDC now points to encrypted "600"
- Public sees: swap happened, but not amounts

### 4. UNWRAP (Withdraw)

**Scenario:** Alice withdraws 100 SUI

```
CLIENT:
1. Call blockchain
   → contract.unwrap_sui(100, alice_wallet)

BLOCKCHAIN:
2. Verify pool has 100 SUI ✓
3. Send 100 real SUI to alice_wallet
4. Update eSUI pointer (now encrypts 100)
5. Emit UnwrapEvent
```

**Result:**
- Alice receives: 100 real SUI in wallet
- Alice's eSUI: now points to encrypted "100"

---

## Smart Contract Operations

### Core Functions

#### 1. Wrap Functions
```move
// Deposit SUI, get eSUI
public entry fun wrap_sui(
    pool: &mut LiquidityPool,
    payment: Coin<SUI>,
    encrypted_pointer: vector<u8>,
    ctx: &mut TxContext
)

// Deposit USDC, get eUSDC
public entry fun wrap_usdc(
    pool: &mut LiquidityPool,
    payment: Coin<USDC>,
    encrypted_pointer: vector<u8>,
    ctx: &mut TxContext
)
```

#### 2. Merge Functions
```move
// Add more SUI to existing eSUI
public entry fun merge_sui(
    pool: &mut LiquidityPool,
    esui: &mut EncryptedSUI,
    payment: Coin<SUI>,
    new_pointer: vector<u8>,
    ctx: &mut TxContext
)
```

#### 3. Swap Functions
```move
// User requests swap
public entry fun request_swap_sui_to_usdc(
    esui: &EncryptedSUI,
    eusdc: &EncryptedUSDC,
    swap_amount_encrypted: vector<u8>,
    ctx: &mut TxContext
)

// TEE updates after Cetus swap
public entry fun update_after_swap_sui_to_usdc(
    pool: &mut LiquidityPool,
    esui: &mut EncryptedSUI,
    eusdc: &mut EncryptedUSDC,
    sui_spent: u64,
    usdc_received: u64,
    new_esui_pointer: vector<u8>,
    new_eusdc_pointer: vector<u8>,
    ctx: &mut TxContext
)
```

#### 4. Unwrap Functions
```move
// Withdraw all
public entry fun unwrap_sui(
    pool: &mut LiquidityPool,
    esui: EncryptedSUI,
    amount: u64,
    recipient: address,
    ctx: &mut TxContext
)

// Withdraw partial
public entry fun unwrap_sui_partial(
    pool: &mut LiquidityPool,
    esui: &mut EncryptedSUI,
    amount: u64,
    recipient: address,
    new_pointer: vector<u8>,
    ctx: &mut TxContext
)
```

### Key Design Decision: Option C

**TEE swaps on Cetus off-chain, then reports results**

The smart contract does NOT execute swaps. Instead:
1. User requests swap (on-chain)
2. TEE sees request, swaps on Cetus (off-chain)
3. TEE reports results (on-chain)
4. Contract updates pointers only

This allows using Cetus SDK in TEE while keeping contract simple.

---

## Server Infrastructure

### Minimum Setup for MVP

#### 1. TEE Server (Required)
```
Hardware: AWS EC2 with Nitro Enclaves
Type: t3.medium (2 vCPU, 4GB RAM)
Cost: ~$50/month

Responsibilities:
- Monitor Sui blockchain events
- Decrypt encrypted data
- Execute swaps on Cetus
- Re-encrypt results
- Update blockchain
- Pay gas fees
```

#### 2. DA Storage (Required)
```
Option A: IPFS via Pinata
Cost: ~$10/month
Setup: Hosted service, no server needed

Option B: PostgreSQL
Cost: Free (on same server as TEE)
Setup: Docker container

Option C: Arweave
Cost: Pay-per-upload ($0.001/MB)
Setup: Upload to network
```

#### 3. API Server (Optional)
```
Not needed! Client queries blockchain directly
```

**Total Cost: ~$60/month**

### Production Setup (Decentralized)

For production, use threshold network:
- 5 TEE servers (run by different parties)
- Require 3-of-5 to decrypt (threshold cryptography)
- No single party can decrypt alone
- More secure, more decentralized

---

## Implementation Guide

### Phase 1: Smart Contract (Week 1-2)

```
Files needed: 1 file only
- sources/encrypted_swap.move

Functions to implement:
✓ wrap_sui, wrap_usdc
✓ merge_sui, merge_usdc
✓ request_swap_sui_to_usdc, request_swap_usdc_to_sui
✓ update_after_swap_sui_to_usdc, update_after_swap_usdc_to_sui
✓ unwrap_sui, unwrap_usdc
✓ Admin functions
```

### Phase 2: TEE Service (Week 3-4)

```
Language: Python or Rust
Libraries needed:
- sui-sdk (blockchain interaction)
- cetus-sdk (DEX swaps)
- cryptography (encryption/decryption)

Core functionality:
1. Event listener (monitor blockchain)
2. Encryption handler (encrypt/decrypt)
3. Cetus integration (execute swaps)
4. DA storage client (read/write ciphertexts)
```

### Phase 3: Client App (Week 5-6)

```
Framework: React/Next.js
Libraries needed:
- @mysten/sui.js (Sui SDK)
- Encryption library (ElGamal)
- IPFS client (if using IPFS)

Features:
1. Wrap/unwrap interface
2. Swap interface
3. Balance viewer (decrypt locally)
4. Transaction history
```

### Phase 4: Testing & Deployment (Week 7-8)

```
1. Local testing with Sui devnet
2. Deploy contract to testnet
3. Run TEE service on AWS
4. Test full flow end-to-end
5. Deploy to mainnet
```

---

## Security Considerations

### Trust Model

**What Users Trust:**
1. **TEE Hardware** - Intel TDX/AWS Nitro provides isolation
2. **Threshold Network** - Multiple TEE nodes, no single point of failure
3. **Smart Contract** - Open source, auditable
4. **Blockchain** - Sui's security guarantees

**What Users DON'T Need to Trust:**
- Individual TEE operator (threshold protects)
- DA storage provider (data is encrypted)
- Front-end provider (can run locally)

### Attack Vectors & Mitigations

#### 1. TEE Compromise
**Risk:** Malicious TEE steals funds
**Mitigation:** 
- Threshold cryptography (need 3-of-5 TEE nodes)
- Hardware attestation (verify TEE integrity)
- Regular audits

#### 2. Front-Running
**Risk:** MEV bots see and copy trades
**Mitigation:**
- Amounts are encrypted (bots can't see)
- Ephemeral accounts (can't track)

#### 3. Pool Insolvency
**Risk:** Pool doesn't have enough real tokens
**Mitigation:**
- Invariant checks: `real_tokens >= sum(encrypted_balances)`
- Emergency pause mechanism
- Regular audits

#### 4. Pointer Forgery
**Risk:** User creates fake pointer claiming large balance
**Mitigation:**
- TEE verifies all operations
- ZK proofs (future enhancement)
- Event monitoring & alerts

### Privacy Guarantees

**What's Private:**
- ✅ User balances (encrypted)
- ✅ Swap amounts (encrypted)
- ✅ Trading patterns (unlinkable)

**What's NOT Private:**
- ❌ That user interacted (on-chain)
- ❌ Token types (needed for routing)
- ❌ Timing of actions

**Privacy Level:** Similar to Tornado Cash but for trading (not mixing)

---

## Data Storage Summary

### After Complete Workflow

**Example: Alice's journey**
1. Deposits 500 SUI
2. Swaps 300 SUI → 600 USDC
3. Withdraws 100 SUI

**Final State:**

```
┌─────────────────────────────────────────────┐
│ BLOCKCHAIN (Public)                         │
├─────────────────────────────────────────────┤
│ Alice owns:                                 │
│   EncryptedSUI {                            │
│     id: obj_123,                            │
│     balance_pointer: [ptr_sui_final]        │
│   }                                         │
│   EncryptedUSDC {                           │
│     id: obj_456,                            │
│     balance_pointer: [ptr_usdc_final]       │
│   }                                         │
│                                             │
│ Pool holds:                                 │
│   - 100 SUI (500 - 300 - 100)              │
│   - 600 USDC                                │
└─────────────────────────────────────────────┘

┌─────────────────────────────────────────────┐
│ DA STORAGE (Off-chain)                      │
├─────────────────────────────────────────────┤
│ ptr_sui_final → Encrypt(100)               │
│ ptr_usdc_final → Encrypt(600)              │
│                                             │
│ (Old pointers still exist but unused)      │
└─────────────────────────────────────────────┘
```

---

## Key Takeaways

### What Makes This Work

1. **Two-Layer Storage**
   - Small pointers on expensive blockchain
   - Large ciphertexts on cheap off-chain storage

2. **Client-Side Encryption**
   - Users encrypt before sending
   - Browser never sends plaintext to blockchain

3. **TEE Processing**
   - Secure hardware decrypts safely
   - Executes swaps on Cetus
   - Re-encrypts with fresh randomness

4. **Pointer-Based Updates**
   - Blockchain only updates 32-byte pointers
   - Fast, cheap, private

**Document Version:** 1.0  
**Last Updated:** November 2025  
**Author:** MaxLuong  
**Project:** Privacy DEX on Sui (Encifher-inspired)
