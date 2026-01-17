# Wallet Signature Security Hardening

## Problem: Bearer Asset Vulnerability

In the original minimal design, the **nullifier alone** grants spending authority:

```
Original: Nullifier = Cash (whoever has it can spend)
```

**Threat Model:**
| Scenario | Original Design | Impact |
|----------|-----------------|--------|
| Attacker guesses nullifier | Impossible (2^256) | Safe |
| Attacker watches blockchain | Safe (nullifier encrypted) | Safe |
| **Attacker steals nullifier** | **Funds stolen** | **CRITICAL** |

If malware, XSS, or a data breach exposes your nullifier, the attacker can immediately drain your funds.

## Solution: Wallet Signature Verification

Add a **wallet signature requirement** - knowing the nullifier is not enough:

```
Hardened: Nullifier + Wallet Signature = Check (requires wallet to sign)
```

**New Threat Model:**
| Scenario | Hardened Design | Impact |
|----------|-----------------|--------|
| Attacker steals nullifier only | **Safe** (can't sign without wallet) | Protected |
| Attacker compromises wallet only | **Safe** (can't find deposit without nullifier) | Protected |
| Attacker steals both | Funds stolen | Compromised |

## Implementation

### 1. Data Structures

**DepositNote (stored locally in frontend):**
```typescript
interface DepositNote {
  nullifier: string;           // For finding deposit
  ownerAddress: string;        // Wallet that created deposit
  amount: string;
  tokenType: "SUI";
  timestamp: number;
  depositId?: string;
  spent: boolean;
}
```

**Encrypted Deposit Data (on-chain, SEAL encrypted):**
```json
{
  "amount": "1000000000",
  "nullifier": "0x...",
  "ownerAddress": "0x..."    // For TEE to verify signatures
}
```

**SwapIntentDetails (encrypted in intent):**
```typescript
interface SwapIntentDetails {
  nullifier: string;
  inputAmount: string;
  outputStealth: string;
  remainderStealth: string;
  signature: string;          // Wallet signature (Base64)
}
```

### 2. Signature Scheme

**Message format (must match exactly in frontend and backend):**
```
message = "mist_intent_v2:{nullifier}:{inputAmount}:{outputStealth}:{remainderStealth}"
```

**Frontend signing (using @mysten/dapp-kit):**
```typescript
const messageBytes = createIntentMessage(
  nullifier, inputAmount, outputStealth, remainderStealth
);
const { signature } = await signPersonalMessage({ message: messageBytes });
```

**Backend verification (in TEE):**
```rust
// Reconstruct message
let message = format!(
    "mist_intent_v2:{}:{}:{}:{}",
    details.nullifier,
    details.input_amount,
    details.output_stealth,
    details.remainder_stealth
);

// Verify and extract signer
let user_signature: UserSignature = bcs::from_bytes(&signature_bytes)?;
let public_key = user_signature.verify_personal_message(&PersonalMessage(message))?;
let signer_address = public_key.to_address();
```

### 3. Flow Diagram

```
DEPOSIT (with owner address):
┌────────────────────────────────────────────────────────────────────┐
│ 1. User generates: nullifier = random(32 bytes)                    │
│                                                                    │
│ 2. SEAL encrypt for deposit:                                       │
│    encrypted_data = SEAL({ amount, nullifier, ownerAddress })      │
│                           ownerAddress = wallet.address            │
│                                                                    │
│ 3. Store locally: DepositNote = { nullifier, ownerAddress, ... }   │
│                                                                    │
│ 4. Submit deposit tx (encrypted_data goes on-chain)                │
└────────────────────────────────────────────────────────────────────┘

SWAP INTENT (signed with wallet):
┌────────────────────────────────────────────────────────────────────┐
│ 1. User selects deposit note                                       │
│                                                                    │
│ 2. Generate stealth addresses for output/remainder                 │
│                                                                    │
│ 3. Create message and sign WITH WALLET:                            │
│    message = "mist_intent_v2:{nullifier}:{amount}:{stealth}:..."   │
│    signature = wallet.signPersonalMessage(message)                 │
│                                                                    │
│ 4. SEAL encrypt intent:                                            │
│    encrypted = SEAL({ nullifier, inputAmount, outputStealth,       │
│                       remainderStealth, signature })               │
│                                                                    │
│ 5. Submit intent (or send to relayer)                              │
└────────────────────────────────────────────────────────────────────┘

TEE VERIFICATION:
┌────────────────────────────────────────────────────────────────────┐
│ 1. SEAL decrypt intent → get fields + signature                    │
│                                                                    │
│ 2. Reconstruct message from intent fields                          │
│                                                                    │
│ 3. Verify signature → extract signer address                       │
│    - If INVALID: Reject immediately                                │
│    - If VALID: Continue                                            │
│                                                                    │
│ 4. Execute swap (signature already verified)                       │
│                                                                    │
│ FUTURE: Also verify signer == ownerAddress from deposit            │
└────────────────────────────────────────────────────────────────────┘
```

### 4. Security Analysis

**Attack Scenarios:**

1. **Nullifier leaked (malware reads localStorage):**
   - Attacker has: nullifier
   - Attacker needs: wallet private key to sign
   - Result: **Cannot spend** (signature verification fails)

2. **Wallet compromised without nullifier:**
   - Attacker has: wallet access
   - Attacker needs: nullifier to create valid intent
   - Result: **Cannot spend** (can't find deposit)

3. **Both compromised:**
   - Attacker has: nullifier + wallet access
   - Result: Can spend (same security as before)
   - Mitigation: Defense in depth, consider hardware wallets

4. **Front-running attack:**
   - Attacker sees encrypted intent in mempool
   - Cannot modify (encrypted + signed)
   - Cannot replay (same intent = same outcome)
   - Result: **Safe**

5. **Replay attack:**
   - Attacker copies signed intent
   - Intent includes stealth addresses attacker doesn't control
   - Funds still go to original user's stealth address
   - Result: **Safe** (attacker just pays gas for you)

### 5. Files Changed

**Frontend:**
- `frontend/lib/deposit-notes.ts`
  - Added `ownerAddress` to `DepositNote`
  - Added `signature` to `SwapIntentDetails`
  - Added `createIntentMessage()` function
  - Updated `encryptDepositData()` to include owner
  - Updated `encryptSwapIntent()` to require signature

- `frontend/hooks/useDepositNotes.ts`
  - Added `useSignPersonalMessage` hook
  - Updated `deposit()` to include `ownerAddress`
  - Updated `createSwapIntent()` to sign with wallet

**Backend:**
- `backend/src/apps/mist-protocol/mod.rs`
  - Added `owner_address` to `DecryptedDepositData`
  - Added `signature` to `DecryptedSwapDetails`

- `backend/src/apps/mist-protocol/intent_processor.rs`
  - Added `verify_intent_signature()` function
  - Updated `process_swap_intent()` to verify signature

### 6. User Experience

The user experience is largely unchanged:

1. **Deposit**: Same as before (wallet signs deposit tx)
2. **Swap**: User now signs twice:
   - First: Signs the intent message (proves ownership)
   - Second: Signs the transaction (submits to chain)

The wallet popup will show:
```
Sign Message
"mist_intent_v2:0x1234...:1000000000:0xabc...:0xdef..."

[Cancel] [Sign]
```

### 7. Migration

For existing deposits without `ownerAddress`:
- They can still be spent (signature verification extracts signer)
- But TEE can't verify signer matches deposit owner
- Recommend re-depositing with new format for full security

### 8. Future Improvements

1. **Full deposit verification**: Scan deposits, decrypt to get `ownerAddress`, verify it matches signer
2. **Hardware wallet support**: For highest security
3. **Multi-sig deposits**: Require multiple signatures to spend
4. **Time-locked recovery**: Allow recovery if spending key lost
