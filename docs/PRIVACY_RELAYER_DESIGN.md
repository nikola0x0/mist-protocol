# Privacy Relayer Design for Mist Protocol

## Problem

Currently, users call `create_swap_intent()` directly from their wallet:

```
User wallet → create_swap_intent() tx → Intent on-chain
                      ↑
              This link is visible!
```

Even though the intent contents are SEAL-encrypted, an observer can see:
- Which wallet created which intent
- When the intent was created
- Link the wallet to the eventual stealth output

## Solution: Privacy Relayer

Users submit intents **off-chain** to a relayer, who posts them on-chain:

```
User → encrypted intent (HTTPS) → Relayer → batch submit → TEE executes
                                     ↑
                        Many users, one relayer wallet
                        Can't tell who submitted what
```

## Architecture

```
┌───────────────────────────────────────────────────────────────────┐
│ PRIVACY RELAYER FLOW                                              │
├───────────────────────────────────────────────────────────────────┤
│                                                                   │
│  FRONTEND                                                         │
│  ┌─────────────────────────────────────────────────────────────┐  │
│  │ 1. User selects deposit note (knows nullifier)               │  │
│  │ 2. Generate stealth addresses for output/remainder           │  │
│  │ 3. SEAL encrypt: {nullifier, amounts, stealth_addrs}         │  │
│  │ 4. POST to relayer: /submit_intent                           │  │
│  │    - Body: { encrypted_details, token_in, token_out }        │  │
│  │    - NO SIGNATURE NEEDED (nullifier = authorization)         │  │
│  └─────────────────────────────────────────────────────────────┘  │
│                              │                                    │
│                              ▼                                    │
│  RELAYER (runs in TEE)                                            │
│  ┌─────────────────────────────────────────────────────────────┐  │
│  │ /submit_intent endpoint:                                     │  │
│  │   1. Validate format (encrypted blob, token types)           │  │
│  │   2. Add to pending_intents queue                            │  │
│  │   3. Return { intent_id, position_in_queue }                 │  │
│  │                                                              │  │
│  │ Batch submitter (every 10 seconds OR when queue > N):        │  │
│  │   1. Collect pending intents                                 │  │
│  │   2. Build single tx with multiple create_swap_intent calls  │  │
│  │   3. Submit from RELAYER wallet (breaks user link!)          │  │
│  │   4. Clear pending queue                                     │  │
│  │                                                              │  │
│  │ Intent processor (existing):                                 │  │
│  │   - Polls chain for SwapIntent objects                       │  │
│  │   - SEAL decrypt, validate nullifier, execute swap           │  │
│  └─────────────────────────────────────────────────────────────┘  │
│                              │                                    │
│                              ▼                                    │
│  BLOCKCHAIN (What observer sees)                                  │
│  ┌─────────────────────────────────────────────────────────────┐  │
│  │ Transaction from Relayer wallet:                             │  │
│  │   - create_swap_intent (encrypted_blob_1)                    │  │
│  │   - create_swap_intent (encrypted_blob_2)                    │  │
│  │   - create_swap_intent (encrypted_blob_3)                    │  │
│  │                                                              │  │
│  │ Observer CANNOT tell:                                        │  │
│  │   - Which user submitted which intent                        │  │
│  │   - Intent contents (SEAL encrypted)                         │  │
│  │   - Link between users and stealth outputs                   │  │
│  └─────────────────────────────────────────────────────────────┘  │
│                                                                   │
└───────────────────────────────────────────────────────────────────┘
```

## Key Insight: Nullifier = Authorization

The nullifier serves as implicit authorization:
- Only the depositor knows the nullifier (stored locally)
- Without valid nullifier, the swap fails (nullifier not in any deposit)
- Relayer doesn't know if nullifier is valid (it's encrypted)
- TEE validates when decrypting

**No signature needed from user!** This is crucial for privacy:
- User signature → reveals user public key
- No signature → relayer can't identify user

## API Design

### Relayer Endpoints

```
POST /api/submit_intent
  Body: {
    encrypted_details: base64 string,  // SEAL encrypted blob
    token_in: "SUI",
    token_out: "SUI",
    deadline: unix_timestamp           // Optional, defaults to +1 hour
  }
  Response: {
    intent_id: uuid,
    queue_position: number,
    estimated_submission_time: unix_timestamp
  }

GET /api/intent_status/:intent_id
  Response: {
    status: "pending" | "submitted" | "executed" | "failed",
    tx_digest?: string,                // If submitted
    error?: string                     // If failed
  }

GET /api/queue_info
  Response: {
    pending_count: number,
    next_batch_in_seconds: number,
    min_batch_size: number
  }
```

## Privacy Amplification: Batching

Batching multiple intents improves privacy:

| Batch Size | Anonymity Set | Privacy Level |
|------------|---------------|---------------|
| 1          | 1             | None (same as direct) |
| 5          | 5             | Low |
| 20         | 20            | Medium |
| 100        | 100           | High |

**Configuration:**
```yaml
batching:
  min_batch_size: 5        # Wait for at least 5 intents
  max_wait_seconds: 60     # Or submit after 60 seconds
  max_batch_size: 50       # Don't exceed 50 (gas limits)
```

## Spam Protection

Without user signatures, we need spam protection:

### Option 1: Rate Limiting by IP (Simple)
```rust
// 10 intents per IP per hour
rate_limit:
  window: 3600
  max_requests: 10
```
- Pros: Simple, no user cost
- Cons: VPN bypass, limits privacy

### Option 2: Proof of Deposit Commitment (Recommended)
User provides hash(nullifier) with intent:
```rust
POST /api/submit_intent
  Body: {
    encrypted_details: ...,
    nullifier_commitment: hash(nullifier),  // For spam check
    ...
  }
```
Relayer checks:
1. Nullifier commitment not seen before
2. Adds to seen set (bloom filter)
3. If already seen → reject (duplicate/spam)

- Pros: One intent per deposit, no info leaked
- Cons: Commitment reveals "deposit with this nullifier exists"

### Option 3: Small Payment (Economic)
User pays small fee to relayer:
```rust
POST /api/submit_intent
  Body: {
    encrypted_details: ...,
    payment_tx: tx_digest,  // Tx sending 0.001 SUI to relayer
    ...
  }
```
- Pros: Economic spam protection
- Cons: Payment tx links user wallet (defeats purpose!)

**Recommendation: Option 2** - Nullifier commitment is privacy-preserving and prevents duplicate intents.

## Implementation Plan

### Phase 1: Backend Changes

1. Add `/api/submit_intent` endpoint
2. Add pending intents queue (in-memory + optional persistence)
3. Add batch submitter task
4. Update intent processor to handle relayer-submitted intents

### Phase 2: Move Contract Changes

Update `create_swap_intent` to accept relayer address:
```move
/// Allow relayer to submit intents on behalf of users
/// No change needed! Entry function already works with any sender.
/// The relayer wallet just becomes the tx sender.
```

Actually, **no Move changes needed** - the existing `create_swap_intent` function doesn't store sender address, so relayer submission works out of the box!

### Phase 3: Frontend Changes

1. Add `submitIntentToRelayer()` function
2. Update `createSwapIntent` hook to use relayer
3. Add intent status polling
4. Show queue position to user

### Phase 4: Privacy Enhancements

1. Add batching configuration
2. Add nullifier commitment for spam protection
3. Add intent status endpoint
4. Add queue info endpoint

## Security Considerations

1. **Relayer Trust**: Relayer can delay/drop intents, but cannot:
   - Steal funds (encrypted, only TEE decrypts)
   - Link users to intents (no signature)
   - Modify intent contents (SEAL integrity)

2. **MEV Protection**: Batching helps, but relayer knows order
   - Future: Commit-reveal scheme for order fairness

3. **DoS on Relayer**: Rate limiting + nullifier commitment

4. **Relayer Downtime**:
   - Users can fallback to direct submission (loses privacy)
   - Multiple relayers for redundancy (future)

## File Structure

```
backend/src/
├── apps/mist-protocol/
│   ├── relayer/                    # NEW
│   │   ├── mod.rs
│   │   ├── submit_intent.rs        # /submit_intent endpoint
│   │   ├── intent_queue.rs         # Pending intents queue
│   │   ├── batch_submitter.rs      # Batch submission task
│   │   └── spam_protection.rs      # Nullifier commitment check
│   ├── intent_processor.rs         # Existing
│   └── ...

frontend/lib/
├── relayer.ts                      # NEW - Relayer API client
```

## Example Flow

```
1. Alice deposits 10 SUI
   → Deposit tx visible: Alice → 10 SUI
   → Nullifier stored locally (secret)

2. Alice wants to swap 5 SUI
   → Frontend: SEAL encrypt {nullifier, 5 SUI, stealth_out, stealth_rem}
   → POST /api/submit_intent (no signature!)
   → Relayer returns: { intent_id: "abc", queue_position: 3 }

3. 10 seconds later (batch time)
   → Relayer submits batch tx with 5 intents
   → On-chain: Relayer → create_swap_intent (5 intents)
   → Observer cannot tell which intent is Alice's

4. TEE processes intents
   → SEAL decrypt Alice's intent
   → Validates nullifier matches a deposit
   → Executes swap, sends to stealth addresses

5. Alice receives funds at stealth address
   → Observer sees: Relayer → Intent batch → Stealth outputs
   → No link to Alice's wallet!
```

## Next Steps

1. [ ] Implement `/api/submit_intent` endpoint in backend
2. [ ] Implement intent queue with batching
3. [ ] Update frontend to use relayer
4. [ ] Add spam protection with nullifier commitment
5. [ ] Test end-to-end privacy flow
