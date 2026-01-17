# Mist Protocol v2: Private DeFi Swaps

## Overview

Privacy-preserving DEX swaps on Sui. Minimal design - like tickets but with nullifier-based privacy.

**Core components:**
- **Deposits**: Like tickets, but NO owner field
- **Nullifiers**: Break link between deposit and swap
- **SEAL encryption**: Only TEE can decrypt
- **Stealth addresses**: Unlinkable outputs
- **Nautilus TEE**: Hardware-attested execution

**Goal**: Swap privately. Observers can't link your deposit to your swap output.

**UX**: Single transaction signature per action.

---

## Privacy Model

| Phase       | What's Visible                    | What's Hidden                    |
| ----------- | --------------------------------- | -------------------------------- |
| Deposit     | Wallet, amount                    | Nullifier (encrypted)            |
| Swap Intent | Encrypted blob only               | Everything                       |
| Execution   | Nullifier spent, stealth outputs  | Which deposit it came from       |

**How nullifiers break the link:**
```
Deposits (all visible):
  Wallet A → Deposit 1 (100 SUI, nullifier hidden)
  Wallet B → Deposit 2 (100 SUI, nullifier hidden)
  Wallet C → Deposit 3 (100 SUI, nullifier hidden)

Swap executed:
  Nullifier 0xABC spent → funds to stealth address

Question: Which wallet swapped?
Answer: Can't tell! Nullifier 0xABC doesn't reveal which deposit.
```

**Privacy = anonymity set of all similar deposits.**

---

## Core Concepts

### 1. Nullifier

Random 32-byte value generated at deposit time. Stored encrypted, revealed at swap time.

```
nullifier = random(32 bytes)
```

**Why it works:**
- Deposit: nullifier hidden in SEAL encryption
- Swap: nullifier revealed
- Observer can't link nullifier → deposit (would need to decrypt ALL deposits)

### 2. SEAL Encryption

```
encrypted_data = SEAL.encrypt(amount, nullifier)
```

- Only TEE can decrypt (threshold 2-of-3)
- TEE scans all deposits to find matching nullifier

### 3. Stealth Address

One-time address for receiving output. Recipient scans events to find their funds.

---

## Data Structures (Minimal)

### Deposit

Like a ticket, but **NO owner field**.

```move
struct Deposit has key, store {
    id: UID,
    encrypted_data: vector<u8>,   // SEAL(amount, nullifier)
    token_type: vector<u8>,       // b"SUI" or b"USDC"
    amount: u64,                  // Visible (from deposit tx anyway)
}
```

### NullifierRegistry

Tracks spent nullifiers.

```move
struct NullifierRegistry has key {
    id: UID,
    spent: Table<vector<u8>, bool>,
}
```

### SwapIntent

**NO deposit reference!** Just encrypted blob.

```move
struct SwapIntent has key {
    id: UID,
    encrypted_details: vector<u8>,  // SEAL(nullifier, amounts, stealth_addrs)
    deadline: u64,
}
```

### LiquidityPool

Holds all deposited tokens.

```move
struct LiquidityPool has key {
    id: UID,
    sui_balance: Balance<SUI>,
    tee_authority: address,
}
```

---

## User Flow

### 1. DEPOSIT (1 tx)

```
USER                                CONTRACT
 │                                     │
 ├─ Generate nullifier (random)        │
 ├─ SEAL encrypt (amount, nullifier)   │
 ├─ Build PTB ────────────────────────►│
 │                                     ├─ Lock SUI in pool
 │                                     ├─ Create Deposit {
 │                                     │    encrypted_data,
 │                                     │    amount,
 │                                     │  }
 │                                     │  (NO owner!)
 │                                     │
 ├─ Save locally: { nullifier, amount }│
 │  (USER MUST BACKUP!)                │
```

**Observer sees:** Wallet A deposited 100 SUI
**Observer doesn't see:** The nullifier

### 2. SWAP INTENT (1 tx)

```
USER                                CONTRACT
 │                                     │
 ├─ Load saved { nullifier, amount }   │
 ├─ Generate stealth addresses         │
 ├─ SEAL encrypt {                     │
 │    nullifier,                       │
 │    input_amount,                    │
 │    output_stealth,                  │
 │    remainder_stealth,               │
 │  }                                  │
 ├─ Build PTB ────────────────────────►│
 │  (NO deposit reference!)            ├─ Create SwapIntent {
 │                                     │    encrypted_details,
 │                                     │    deadline,
 │                                     │  }
```

**Observer sees:** Someone created encrypted swap intent
**Observer doesn't see:** Nullifier, amounts, addresses

### 3. TEE EXECUTION

```
TEE                                 CONTRACT
 │                                     │
 ├─ Fetch SwapIntents                  │
 ├─ SEAL decrypt → get nullifier       │
 │                                     │
 ├─ SCAN ALL DEPOSITS:                 │
 │   for each deposit:                 │
 │     decrypt → get nullifier_d       │
 │     if nullifier_d == nullifier:    │
 │       found!                        │
 │                                     │
 ├─ Verify amount, deadline            │
 ├─ Execute swap on Cetus              │
 │                                     │
 ├─ Call execute_swap ────────────────►│
 │   (nullifier, amounts, stealth)     ├─ Check nullifier not spent
 │   NO deposit ID!                    ├─ Mark nullifier spent
 │                                     ├─ Send to stealth addresses
```

**Observer sees:** Nullifier 0xABC spent, funds to stealth addresses
**Observer can't link:** Nullifier → which deposit

### 4. USER RECEIVES OUTPUT

User scans stealth events, derives keys, spends funds normally.

---

## Privacy Analysis

### What Links Are Broken

```
DEPOSIT:     0x123 deposits 100 SUI → Commitment C₁
                    ↓
             NO LINK (no owner in commitment)
                    ↓
SWAP INTENT: Nullifier N₁ submitted → SwapIntent
                    ↓
             NO LINK (nullifier doesn't reveal commitment)
                    ↓
EXECUTION:   TEE sends to 0xStealth1, 0xStealth2
                    ↓
             NO LINK (stealth addresses unlinkable)
                    ↓
USER:        Controls 0xStealth1, 0xStealth2
```

### Anonymity Set

- **Deposits**: All deposits of the same token type
- **Swaps**: All swaps processed by TEE
- **Outputs**: All stealth addresses created

### Attack Vectors & Mitigations

| Attack             | Description                            | Mitigation                                       |
| ------------------ | -------------------------------------- | ------------------------------------------------ |
| Timing correlation | Deposit and swap happen close together | Wait before swapping; batch processing           |
| Amount correlation | 100 SUI deposit, 100 SUI swap          | Split deposits; use only partial amounts         |
| Graph analysis     | Link deposit→swap→output               | True unlinkability - nullifier can't link to commitment |
| TEE compromise     | TEE leaks secrets                      | Nautilus attestation + SEAL threshold (2-of-3)   |
| Front-running      | Someone uses your nullifier            | Nullifier hidden in SEAL encryption              |
| **localStorage theft** | **XSS/malware steals secret+nullifier** | **⚠️ RISK: Attacker can steal funds (same as Tornado Cash)** |
| Intent spam        | DoS TEE with many intents              | Rate limiting; intent fees (optional)            |
| Note loss          | User loses secret/nullifier            | Funds unrecoverable - user must backup           |

**Important security note:** Like Tornado Cash, users must securely backup their deposit "note" (secret + nullifier). If compromised, funds can be stolen. This is the tradeoff for true privacy.

---

## Contract Implementation (Minimal)

```move
module mist_protocol::mist_protocol {
    use sui::coin::{Self, Coin};
    use sui::balance::{Self, Balance};
    use sui::sui::SUI;
    use sui::table::{Self, Table};

    // ============ ERRORS ============
    const E_NULLIFIER_SPENT: u64 = 1;
    const E_NOT_TEE: u64 = 2;

    // ============ STRUCTS ============

    public struct Deposit has key, store {
        id: UID,
        encrypted_data: vector<u8>,   // SEAL(amount, nullifier)
        token_type: vector<u8>,
        amount: u64,
    }

    public struct NullifierRegistry has key {
        id: UID,
        spent: Table<vector<u8>, bool>,
    }

    public struct SwapIntent has key {
        id: UID,
        encrypted_details: vector<u8>,  // SEAL(nullifier, amounts, stealth_addrs)
        deadline: u64,
    }

    public struct LiquidityPool has key {
        id: UID,
        sui_balance: Balance<SUI>,
        tee_authority: address,
    }

    // ============ DEPOSIT ============

    entry fun deposit_sui(
        pool: &mut LiquidityPool,
        payment: Coin<SUI>,
        encrypted_data: vector<u8>,
        ctx: &mut TxContext,
    ) {
        let amount = coin::value(&payment);
        balance::join(&mut pool.sui_balance, coin::into_balance(payment));

        let deposit = Deposit {
            id: object::new(ctx),
            encrypted_data,
            token_type: b"SUI",
            amount,
        };

        transfer::share_object(deposit);
    }

    // ============ SWAP INTENT ============

    entry fun create_swap_intent(
        encrypted_details: vector<u8>,
        deadline: u64,
        ctx: &mut TxContext,
    ) {
        let intent = SwapIntent {
            id: object::new(ctx),
            encrypted_details,
            deadline,
        };
        transfer::share_object(intent);
    }

    // ============ TEE EXECUTION ============

    entry fun execute_swap(
        registry: &mut NullifierRegistry,
        pool: &mut LiquidityPool,
        intent: SwapIntent,
        nullifier: vector<u8>,
        output_amount: u64,
        output_stealth: address,
        remainder_amount: u64,
        remainder_stealth: address,
        ctx: &mut TxContext,
    ) {
        assert!(tx_context::sender(ctx) == pool.tee_authority, E_NOT_TEE);
        assert!(!table::contains(&registry.spent, nullifier), E_NULLIFIER_SPENT);

        table::add(&mut registry.spent, nullifier, true);

        // Send output
        transfer::public_transfer(
            coin::from_balance(balance::split(&mut pool.sui_balance, output_amount), ctx),
            output_stealth
        );

        // Send remainder (if any)
        if (remainder_amount > 0) {
            transfer::public_transfer(
                coin::from_balance(balance::split(&mut pool.sui_balance, remainder_amount), ctx),
                remainder_stealth
            );
        };

        // Cleanup
        let SwapIntent { id, encrypted_details: _, deadline: _ } = intent;
        object::delete(id);
    }
}
```

**That's it!** ~80 lines of Move code for the core privacy mechanism.

---

## Frontend Implementation (Minimal)

### Deposit

```typescript
interface DepositNote {
  nullifier: string;       // Random 32 bytes - MUST BACKUP!
  amount: bigint;
  tokenType: "SUI" | "USDC";
}

async function deposit(amount: bigint, wallet: WalletAdapter): Promise<DepositNote> {
  // 1. Generate random nullifier
  const nullifier = crypto.getRandomValues(new Uint8Array(32));

  // 2. SEAL encrypt
  const encrypted = await sealEncrypt({ amount: amount.toString(), nullifier: toHex(nullifier) });

  // 3. Build & execute PTB
  const tx = new Transaction();
  tx.moveCall({
    target: `${PACKAGE_ID}::mist_protocol::deposit_sui`,
    arguments: [
      tx.object(POOL_ID),
      tx.splitCoins(tx.gas, [tx.pure.u64(amount)]),
      tx.pure.vector("u8", Array.from(encrypted)),
    ],
  });
  await wallet.signAndExecuteTransaction({ transaction: tx });

  // 4. Save note (USER MUST BACKUP!)
  const note: DepositNote = { nullifier: toHex(nullifier), amount, tokenType: "SUI" };
  saveNote(note);
  return note;
}
```

### Swap

```typescript
async function swap(note: DepositNote, inputAmount: bigint, wallet: WalletAdapter) {
  // 1. Generate stealth addresses
  const outputStealth = generateStealthAddress();
  const remainderStealth = generateStealthAddress();

  // 2. SEAL encrypt (NO deposit reference!)
  const encrypted = await sealEncrypt({
    nullifier: note.nullifier,
    inputAmount: inputAmount.toString(),
    outputStealth: outputStealth.address,
    remainderStealth: remainderStealth.address,
  });

  // 3. Build & execute PTB
  const tx = new Transaction();
  tx.moveCall({
    target: `${PACKAGE_ID}::mist_protocol::create_swap_intent`,
    arguments: [
      tx.pure.vector("u8", Array.from(encrypted)),
      tx.pure.u64(Date.now() + 3600000), // 1 hour deadline
    ],
  });
  await wallet.signAndExecuteTransaction({ transaction: tx });

  // 4. Save stealth keys for scanning later
  saveStealthKeys(outputStealth, remainderStealth);
}
```

### TEE Backend

```rust
async fn process_intent(intent: SwapIntent) {
    // 1. Decrypt intent
    let details = seal_decrypt(&intent.encrypted_details);

    // 2. Scan ALL deposits to find matching nullifier
    for deposit in fetch_all_deposits() {
        let data = seal_decrypt(&deposit.encrypted_data);
        if data.nullifier == details.nullifier {
            // Found! Verify and execute
            assert!(data.amount >= details.input_amount);
            execute_swap(details.nullifier, details.output_stealth, ...);
            return;
        }
    }
}
```

---

## Comparison: v1 vs v2

| Aspect           | v1 (Tickets)              | v2 (Nullifiers)           |
| ---------------- | ------------------------- | ------------------------- |
| Deposit          | Ticket in vault           | Deposit object (no owner) |
| Swap reference   | Ticket ID (visible)       | Nullifier (can't link)    |
| Privacy          | Swap linked to deposit    | Swap unlinkable           |
| User stores      | Nothing (vault tracks)    | Nullifier (must backup)   |

---

## Security Considerations

### Security Model: True Unlinkability (Tornado Cash Style)

**Design choice**: For true unlinkability, user MUST hold secrets locally. This is the same model as Tornado Cash.

**What's stored client-side** (⚠️ SENSITIVE):
```typescript
// localStorage contains deposit "note":
{
  secret: "0xabc...",       // 32 bytes - SENSITIVE!
  nullifier: "0xdef...",    // 32 bytes - SENSITIVE!
  amount: 100,
  tokenType: "SUI",
  commitmentHash: "0x...",
  timestamp: 1234567890
}
// If stolen, attacker can drain funds!
```

**Attack analysis**:

| Attack Vector | Result | Severity |
|--------------|--------|----------|
| XSS reads localStorage | **Attacker can steal funds** | 🔴 Critical |
| Malicious extension | **Attacker can steal funds** | 🔴 Critical |
| Malware | **Attacker can steal funds** | 🔴 Critical |
| TEE compromise | Reveals current intents | 🟡 Medium |
| SEAL key server (2/3) | Would allow TEE spoofing | 🟡 Medium |

**This is the price of true privacy.** Just like Tornado Cash, users must:
1. Backup their deposit note securely (hardware wallet, encrypted file, paper)
2. Never share the note
3. Use trusted devices only

### User Responsibility

Users MUST:

- **Backup deposit notes** - loss = funds lost forever
- **Secure browser environment** - XSS = funds stolen
- **Store stealth keys** - for scanning outputs

**Recovery scenarios**:
- Note lost → Funds UNRECOVERABLE
- Note stolen → Funds can be stolen by attacker
- Stealth keys lost → Output funds unrecoverable
- Wallet lost → Standard recovery (seed phrase), notes still needed

### Why This Tradeoff?

| Approach | Privacy | Security | UX |
|----------|---------|----------|-----|
| TEE-mediated secrets | 🟡 Commitment ID links | ✅ Safe from XSS | ✅ 1 sign |
| **User-held secrets (chosen)** | ✅ True unlinkability | ⚠️ XSS risk | ✅ 1 sign |
| Wallet-derived secrets | ✅ True unlinkability | ✅ Safe from XSS | 🟡 2 signs |

We chose **user-held secrets** because:
1. Privacy is the primary goal
2. Same model as Tornado Cash (proven)
3. Single-sign UX maintained
4. Users who want privacy understand the responsibility

### TEE Trust

- TEE scans all commitments (O(n)) for privacy
- Nautilus attestation ensures correct code
- SEAL threshold (2-of-3) prevents single point of failure
- TEE only reveals nullifier, not which commitment

---

## Summary

**Mist Protocol v2** achieves Tornado Cash-level privacy for DEX swaps on Sui.

### What We Built

1. **True unlinkability**: Nullifier can't be linked to commitment
2. **Swap privacy**: All details SEAL encrypted
3. **Output privacy**: Stealth addresses
4. **Single-sign UX**: 1 PTB per action
5. **TEE scanning**: O(n) commitment search for privacy

### Privacy Guarantees

```
Deposit ──X──► Swap Intent ──X──► Output
   │              │               │
   │      (no commitment ref)     │
   │              │               │
   └──────── UNLINKABLE ──────────┘
```

### Security Model

| Aspect | Approach |
|--------|----------|
| Secrets | User holds locally (like Tornado Cash note) |
| Encryption | SEAL threshold (2-of-3) |
| Execution | Nautilus TEE (AWS Nitro) |
| XSS Risk | ⚠️ User must secure browser |

### Trade-offs

| Trade-off | Reason |
|-----------|--------|
| User holds secrets | Required for true unlinkability |
| XSS can steal funds | Same as Tornado Cash - privacy has costs |
| TEE scans all commits | O(n) but ensures no commitment reference |
| Deposit amounts visible | Unavoidable in any system |

**Privacy level**: Tornado Cash equivalent for DEX swaps.
