# Vault Architecture - Ticket-Based Encrypted Balances

## Overview

Mist Protocol uses a **vault + ticket system** where users deposit tokens and receive encrypted tickets representing their balances. Only the user and TEE can decrypt ticket amounts.

## Core Concept

```
Real Tokens → [deposit] → Vault Tickets (encrypted) → [swap] → Updated Tickets → [unwrap] → Real Tokens
```

**Key Feature**: TEE executes swaps with its own wallet, unlinkable to specific users!

## Architecture

### 1. Mist Pool (Shared)

Holds all real tokens for all users:

```move
public struct LiquidityPool has key {
    id: UID,
    sui_balance: Balance<SUI>,
    usdc_balance: Balance<USDC>,
    tee_authority: address,  // TEE wallet address
}
```

**Purpose**: Escrow for deposits/withdrawals

### 2. User Vault (Per-User)

Each user owns their vault with encrypted tickets:

```move
public struct Vault has key {
    id: UID,
    owner: address,
    tickets: VecMap<String, Ticket>,
}

public struct Ticket has store {
    token_type: String,           // "SUI" or "USDC"
    encrypted_amount: vector<u8>, // SEAL encrypted balance
}
```

**Example**:
```
Vault (owner: 0xniko) {
  tickets: {
    "ticket_Y": { token: "USDC", amount: 0xABCD... },  // encrypted 100 USDC
    "ticket_Z": { token: "SUI",  amount: 0xDEF0... },  // encrypted 50 SUI
  }
}
```

## Complete Flow

### Step 1: Deposit (Wrap)

**User deposits 100 USDC, creates 2 tickets**:

```typescript
// Frontend
const ticket1 = await sealClient.encrypt("60000000");  // 60 USDC
const ticket2 = await sealClient.encrypt("40000000");  // 40 USDC

// Contract call
await contract.deposit_and_create_tickets(
  pool,
  100_USDC_coin,
  [
    { name: "ticket_Y", token: "USDC", encrypted_amount: ticket1 },
    { name: "ticket_Z", token: "USDC", encrypted_amount: ticket2 }
  ]
);
```

**On-chain**:
- ✅ 100 USDC locked in pool
- ✅ Vault updated with 2 tickets (names visible, amounts encrypted)

### Step 2: Swap Intent

**User requests swap using ticket_Y**:

```typescript
// Encrypt swap intent
const intent = { ticket: "ticket_Y", amount: "all", token_out: "SUI" };
const encrypted = await sealClient.encrypt(JSON.stringify(intent));

// Submit to TEE (off-chain or on-chain)
await teeBackend.submitIntent(encrypted);
```

**On-chain**: Nothing yet (intent off-chain)

### Step 3: TEE Executes

```rust
// TEE backend
// 1. Decrypt intent
let intent = decrypt(encrypted_intent);  // { ticket: Y, amount: all }

// 2. Decrypt ticket_Y balance
let ticket_y_amount = decrypt(vault.tickets["Y"].encrypted_amount);  // 60 USDC

// 3. Swap with TEE wallet
let sui_received = cetus_swap(60_USDC);  // 40 SUI

// 4. Burn ticket_Y, create ticket_X
let new_ticket = encrypt("40");  // 40 SUI encrypted

// 5. Update vault on-chain
contract.update_vault(vault, [
  ("ticket_Y", DELETE),
  ("ticket_X", { token: "SUI", amount: new_ticket })
]);
```

**On-chain**:
- ✅ Vault updated: ticket_Y deleted, ticket_X added
- ✅ TEE swapped 60 USDC → 40 SUI (visible, but NOT linked to user!)
- ❌ Amounts hidden (encrypted)

### Step 4: Unwrap

**User withdraws ticket_X**:

```typescript
await contract.unwrap_ticket(pool, vault, "ticket_X", userAddress);
```

**On-chain**:
- ✅ 40 SUI transferred to user
- ✅ ticket_X deleted from vault

## Privacy Analysis

### What Observers See:

```
Block 1000: User creates vault with tickets Y, Z
Block 1010: Vault updated: ticket_Y deleted, ticket_X added
Block 1015: TEE swapped 60 USDC → 40 SUI (on Cetus)
Block 1020: User withdrew from ticket_X

Observer analysis:
- User has tickets Y, Z (unknown amounts)
- Ticket Y changed to X (unknown amounts)
- TEE did a swap (60 USDC → 40 SUI)
- User withdrew something

CAN THEY LINK THEM? ❌ NO!
- Multiple users using TEE
- Ticket names arbitrary (Y, Z, X - no meaning)
- Timing not correlated (batching possible)
```

### VS Single Vault

```move
// Single shared vault
GlobalVault {
  balances: {
    0xniko: { "SUI": 0xABCD..., "USDC": 0xDEF0... }
  }
}
```

**Same privacy**: Amounts hidden, user ownership visible

**But worse performance**: Write contention on single object

## Your Design is Better! ✅

**Reasons**:
1. **Same privacy** as single vault
2. **Better performance** (parallel writes)
3. **Flexible tickets** (user controls naming/splitting)
4. **TEE unlinkability** (swaps with own wallet)

## Smart Contract Design

### Core Functions

```move
// User deposits and creates tickets
entry fun deposit_and_create_tickets(
    pool: &mut LiquidityPool,
    vault: &mut Vault,
    payment: Coin<SUI>,  // or USDC
    tickets: vector<TicketInput>,  // User chooses names & amounts
    ctx: &TxContext
)

// TEE updates tickets after swap
entry fun update_tickets(
    vault: &mut Vault,
    pool: &LiquidityPool,
    updates: vector<TicketUpdate>,  // Add/delete/modify tickets
    ctx: &TxContext
)

// User withdraws ticket
entry fun unwrap_ticket(
    pool: &mut LiquidityPool,
    vault: &mut Vault,
    ticket_name: String,
    recipient: address,
    ctx: &mut TxContext
)
```

## Privacy Properties

### What's Hidden ✅
- Ticket amounts (SEAL encrypted)
- Swap amounts (SEAL encrypted)
- User's total balance
- Swap strategy (TEE uses own wallet)

### What's Visible ⚠️
- User owns a vault
- Ticket names (but arbitrary)
- Token types (from ticket.token_type)
- Deposit/withdrawal amounts (unavoidable)
- TEE's swap transactions (but unlinkable to users)

### Why TEE Wallet is Key 🔑

**Without TEE wallet**:
```
User swap request → User's EncryptedSUI updated
→ Observer links user to swap ❌
```

**With TEE wallet** (your design):
```
User swap request → TEE swaps with TEE wallet → User's ticket updated
→ Observer cannot link! ✅

Multiple users:
- User A requests swap
- User B requests swap
- TEE executes 2 swaps
- Which swap is for who? Observer doesn't know!
```

## Summary

**Your per-user vault design provides**:
- ✅ Same privacy as single shared vault
- ✅ Better performance (no write contention)
- ✅ Flexible ticket management
- ✅ TEE wallet breaks user→swap linkage

**Answer to your question**: Single vault has **same privacy** but **worse performance**. Stick with per-user vaults!
