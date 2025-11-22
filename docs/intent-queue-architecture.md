# Mist Protocol - IntentQueue Architecture

## Overview

This document describes the on-chain swap intent queue system for Mist Protocol, enabling privacy-preserving swaps without requiring a database.

## Problem Statement

### Current Issues
- Swap intents are only emitted as events (ephemeral)
- No persistent state for pending swaps
- Backend cannot reliably track which swaps need processing
- Events can be missed on backend restart
- No clear "pending" vs "completed" state

### Requirements
- ✅ 100% on-chain (no database)
- ✅ Reliable (survives backend restarts)
- ✅ Efficient (minimal RPC queries)
- ✅ Clear state tracking
- ✅ TEE can process asynchronously

## Architecture

### Core Components

#### 1. IntentQueue (Global Shared Object)
```rust
public struct IntentQueue has key {
    id: UID,
    pending: Table<ID, bool>,  // Maps intent_id -> true for pending intents
}
```

**Purpose:** Central registry of all pending swap intents
**Ownership:** Shared object (both users and TEE can access)
**Created:** Once at package deployment in `init()`

#### 2. SwapIntent (Individual Shared Objects)
```rust
public struct SwapIntent has key, store {
    id: UID,
    vault_id: ID,                // Which vault's tickets to use
    ticket_ids_in: vector<u64>,  // Which tickets to consume
    token_out: String,           // Target token ("SUI" or "USDC")
    min_output_amount: u64,      // Slippage protection
    deadline: u64,               // Unix timestamp
    user: address,               // Who created the intent
}
```

**Purpose:** Represents a single swap request
**Ownership:** Shared object (TEE needs to read it)
**Lifetime:** Created → Processed → Removed from queue (object can be deleted or kept for history)

#### 3. SwapIntentEvent (Event)
```rust
public struct SwapIntentEvent has copy, drop {
    vault_id: ID,
    ticket_ids_in: vector<u64>,
    token_out: String,
    min_output_amount: u64,
    deadline: u64,
    user: address,
}
```

**Purpose:** Notification for indexing/logging
**Note:** Backend doesn't rely on events, uses queue instead

## Data Flow

### Phase 1: User Creates Swap Intent

```
┌─────────────────────────────────────────────────────────┐
│ User Frontend                                            │
│  1. Select tickets: [0, 1, 2]                           │
│  2. Set config: tokenOut="USDC", minOutput=95000000     │
│  3. Call create_swap_intent()                           │
└─────────────────────────────────────────────────────────┘
                        ↓
┌─────────────────────────────────────────────────────────┐
│ Smart Contract (create_swap_intent)                     │
│  1. Verify vault ownership                              │
│  2. Verify all tickets exist in vault                   │
│  3. Create SwapIntent object                            │
│  4. intent_id = object::id(&intent)                     │
│  5. queue.pending.add(intent_id, true)                  │
│  6. transfer::share_object(intent)                      │
│  7. event::emit(SwapIntentEvent)                        │
└─────────────────────────────────────────────────────────┘
                        ↓
              IntentQueue Updated
         pending: { intent_abc: true }
```

### Phase 2: Backend Processes Intent

```
┌─────────────────────────────────────────────────────────┐
│ Backend Polling Loop (every 5 seconds)                  │
│  1. Query IntentQueue object                            │
│  2. Extract pending intent IDs                          │
│  3. For each intent_id in queue.pending:                │
└─────────────────────────────────────────────────────────┘
                        ↓
┌─────────────────────────────────────────────────────────┐
│ Get SwapIntent Object                                    │
│  intent = sui_client.get_object(intent_id)              │
│  - vault_id: 0xabc...                                   │
│  - ticket_ids_in: [0, 1, 2]                             │
│  - token_out: "USDC"                                    │
│  - min_output: 95000000                                 │
└─────────────────────────────────────────────────────────┘
                        ↓
┌─────────────────────────────────────────────────────────┐
│ Decrypt Ticket Amounts                                   │
│  1. Get vault object                                    │
│  2. Build seal_approve_tee PTB for tickets [0,1,2]     │
│  3. Fetch SEAL keys from key servers                    │
│  4. Decrypt each ticket's encrypted_amount              │
│  5. Total: ticket[0]=0.1 + ticket[1]=0.2 + ticket[2]=0.1│
│     = 0.4 SUI                                           │
└─────────────────────────────────────────────────────────┘
                        ↓
┌─────────────────────────────────────────────────────────┐
│ Execute Swap on Cetus                                    │
│  1. Swap 0.4 SUI → 150 USDC (example)                   │
│  2. Encrypt output amount: 150000000 with SEAL          │
└─────────────────────────────────────────────────────────┘
                        ↓
┌─────────────────────────────────────────────────────────┐
│ Update Vault On-Chain                                    │
│  Call execute_swap(vault, [0,1,2], encrypted_output)   │
│  - Remove consumed tickets [0, 1, 2]                    │
│  - Create new ticket #3: USDC, encrypted_amount         │
│  - Emit SwapExecutedEvent                               │
└─────────────────────────────────────────────────────────┘
                        ↓
┌─────────────────────────────────────────────────────────┐
│ Mark Intent Complete                                     │
│  Call mark_intent_completed(queue, intent_id, enclave)  │
│  - queue.pending.remove(intent_id)                      │
│  - Intent no longer shows as pending                    │
└─────────────────────────────────────────────────────────┘
```

### Phase 3: User Withdraws

```
User loads tickets from vault → See new ticket #3 (USDC)
  ↓
User decrypts ticket #3 → 150 USDC
  ↓
Calls unwrap_ticket(vault, pool, 3, 150000000, None)
  ↓
Receives 150 USDC coins in wallet
```

## Privacy Model

### What's Encrypted (SEAL Threshold Encryption)
- ✅ **Ticket amounts** - Input ticket amounts (SUI/USDC)
- ✅ **Output amounts** - Swap result amounts
- Only decryptable by:
  - Vault owner (via seal_approve_user)
  - Registered TEE (via seal_approve_tee)

### What's Public (On-Chain)
- Ticket IDs (just references: [0, 1, 2])
- Token types ("SUI", "USDC")
- Swap direction (SUI → USDC)
- Slippage parameters (min_output_amount)
- Deadline (timestamp)

### Why This Is Secure
The **amounts are private**. An observer can see:
- "User wants to swap SUI tickets to USDC"
- "User has slippage tolerance of X"

But **cannot see**:
- How much SUI is being swapped
- How much USDC will be received
- User's total balance

## Smart Contract Design

### New Structs

```rust
/// Global queue for tracking pending intents
public struct IntentQueue has key {
    id: UID,
    pending: Table<ID, bool>,
}

/// Individual swap intent
public struct SwapIntent has key, store {
    id: UID,
    vault_id: ID,
    ticket_ids_in: vector<u64>,
    token_out: String,
    min_output_amount: u64,
    deadline: u64,
    user: address,
}
```

### New Functions

#### create_swap_intent
```rust
entry fun create_swap_intent(
    queue: &mut IntentQueue,
    vault: &VaultEntry,
    ticket_ids_in: vector<u64>,
    token_out: String,
    min_output_amount: u64,
    deadline: u64,
    ctx: &mut TxContext,
) {
    // 1. Verify ownership
    assert!(seal_policy::owner(vault) == ctx.sender(), E_NOT_OWNER);

    // 2. Verify tickets exist
    verify_tickets_exist(vault, &ticket_ids_in);

    // 3. Create intent object
    let intent = SwapIntent {
        id: object::new(ctx),
        vault_id: object::id(vault),
        ticket_ids_in,
        token_out,
        min_output_amount,
        deadline,
        user: ctx.sender(),
    };

    let intent_id = object::id(&intent);

    // 4. Add to pending queue
    queue.pending.add(intent_id, true);

    // 5. Share intent (TEE can read)
    transfer::share_object(intent);

    // 6. Emit event
    event::emit(SwapIntentEvent { ... });
}
```

#### mark_intent_completed
```rust
entry fun mark_intent_completed(
    queue: &mut IntentQueue,
    intent_id: ID,
    enclave: &Enclave<MIST_PROTOCOL>,
    ctx: &TxContext,
) {
    // Only TEE can mark complete
    let tee_address = pk_to_address(enclave.pk());
    assert!(ctx.sender().to_bytes() == tee_address, E_NOT_AUTHORIZED);

    // Remove from pending queue
    assert!(queue.pending.contains(intent_id), E_INTENT_NOT_FOUND);
    queue.pending.remove(intent_id);
}
```

### Updated init()
```rust
fun init(_witness: MIST_PROTOCOL, ctx: &mut TxContext) {
    // Create pool (existing)
    let pool = LiquidityPool { ... };
    transfer::share_object(pool);

    // Create intent queue (NEW)
    let queue = IntentQueue {
        id: object::new(ctx),
        pending: table::new(ctx),
    };
    transfer::share_object(queue);

    // AdminCap (existing)
    ...
}
```

## Backend Implementation

### Event Polling Loop
```rust
// main.rs - Background task
async fn intent_processor_loop() {
    let sui_client = SuiClient::new(...);
    let queue_id = env::var("INTENT_QUEUE_ID")?;

    loop {
        // 1. Query queue object
        let queue_obj = sui_client.get_object(queue_id).await?;
        let pending_ids = extract_pending_ids(&queue_obj);

        info!("Found {} pending intents", pending_ids.len());

        // 2. Process each intent
        for intent_id in pending_ids {
            match process_swap_intent(intent_id).await {
                Ok(_) => info!("Processed intent {}", intent_id),
                Err(e) => error!("Failed to process {}: {}", intent_id, e),
            }
        }

        // 3. Wait before next poll
        tokio::time::sleep(Duration::from_secs(5)).await;
    }
}

async fn process_swap_intent(intent_id: String) -> Result<()> {
    // 1. Get intent details
    let intent = sui_client.get_object(intent_id).await?;
    let vault_id = intent.vault_id;
    let ticket_ids = intent.ticket_ids_in;

    // 2. Decrypt tickets (existing logic)
    let decrypted_amounts = decrypt_tickets(vault_id, ticket_ids).await?;
    let total_amount = decrypted_amounts.iter().sum();

    // 3. Execute swap on Cetus
    let output_amount = execute_cetus_swap(
        total_amount,
        intent.token_out,
        intent.min_output_amount,
    ).await?;

    // 4. Encrypt output amount
    let encrypted_output = encrypt_with_seal(output_amount).await?;

    // 5. Call execute_swap on-chain
    let tx = build_execute_swap_tx(
        vault_id,
        ticket_ids,
        encrypted_output,
        intent.token_out,
    );
    sui_client.execute_transaction(tx).await?;

    // 6. Mark intent as completed
    let tx = build_mark_completed_tx(intent_id);
    sui_client.execute_transaction(tx).await?;

    Ok(())
}
```

## Deployment Steps

1. **Update contracts:**
   - Add IntentQueue + SwapIntent structs
   - Update create_swap_intent
   - Add mark_intent_completed
   - Update init()

2. **Deploy to testnet:**
   ```bash
   sui client publish --gas-budget 100000000
   ```

3. **Save IntentQueue ID:**
   - Extract from deployment output
   - Add to backend .env: `INTENT_QUEUE_ID=0x...`
   - Add to frontend .env: `NEXT_PUBLIC_PACKAGE_ID=0x...`

4. **Update backend:**
   - Add intent polling loop
   - Replace HTTP endpoint with event processor
   - Use existing SEAL decryption logic

5. **Update frontend:**
   - Already done! Just need new package ID
   - create_swap_intent signature already updated

6. **Test flow:**
   - User creates vault
   - User deposits SUI (creates encrypted ticket)
   - User creates swap intent → SwapIntent object created
   - Backend polls → Finds pending intent
   - Backend decrypts → Executes swap → Marks complete
   - User sees output ticket in vault

## Efficiency Analysis

### RPC Query Cost

**Per polling cycle (every 5 seconds):**
- 1 query: Get IntentQueue object
- N queries: Get each SwapIntent object (where N = pending intents)
- Total: 1 + N queries

**Example scenarios:**
- 0 pending: 1 query (just queue check)
- 5 pending: 6 queries (queue + 5 intents)
- 100 pending: 101 queries (still efficient)

### Comparison to Events

**Event polling:**
- Query events with pagination
- Filter by processed state (need local tracking)
- Risk of missing events on restart

**Object queue:**
- Single query gets all pending IDs
- No local state needed
- Guaranteed delivery (on-chain state)

## Privacy Guarantees

### What TEE Learns
✅ Vault ID (which user is swapping)
✅ Ticket amounts (decrypted via SEAL)
✅ Token types (already public)
✅ Swap result (output amount)

### What Public Chain Sees
- Vault ID: `0xabc...`
- Ticket IDs: `[0, 1, 2]`
- Token direction: `SUI → USDC`
- Slippage: `min_output_amount: 95000000`
- Deadline: `1732000000`
- ❌ **Cannot see amounts** (SEAL encrypted in tickets)

### Privacy Model
The amounts are encrypted in the tickets themselves. The swap intent only contains:
- References to tickets (IDs)
- Swap parameters (public configuration)

This provides **amount privacy** while keeping swap intentions transparent.

## Error Handling

### Backend Resilience
```rust
// If swap fails, intent stays in queue
match process_swap_intent(intent_id).await {
    Ok(_) => {
        // Success: mark complete
        mark_completed(intent_id).await?;
    }
    Err(e) => {
        // Error: log and retry next cycle
        error!("Swap failed: {}, will retry", e);
        // Intent remains in queue.pending
    }
}
```

### Timeout Handling
```rust
// Backend can check deadline
if intent.deadline < current_timestamp() {
    warn!("Intent {} expired, skipping", intent_id);
    mark_completed(intent_id).await?; // Remove from queue
    continue;
}
```

### Gas Management
- TEE pays gas for execute_swap
- TEE pays gas for mark_intent_completed
- User only pays for create_swap_intent

## Future Enhancements

### Potential Improvements
1. **Priority queue** - Process high-value swaps first
2. **Batch processing** - Process multiple intents in one transaction
3. **Partial fills** - Split large swaps across multiple Cetus pools
4. **Retry logic** - Exponential backoff for failed swaps
5. **Intent cancellation** - Allow users to cancel pending intents

### Scaling Considerations
- Table can store millions of intent IDs efficiently
- Backend can process N intents in parallel
- No contention on queue (reads don't block writes)
- Can run multiple backend instances (all poll same queue)

## Summary

### Key Benefits
✅ **No database** - All state on Sui blockchain
✅ **Reliable** - Intents persist across restarts
✅ **Efficient** - Single query per poll cycle
✅ **Clear state** - Pending vs completed tracked on-chain
✅ **Simple** - Minimal backend complexity

### Architecture Pattern
This follows the **on-chain queue pattern** common in blockchain systems:
- Users create work items (SwapIntent objects)
- Workers poll for pending items (Backend)
- Workers mark items complete on-chain
- No centralized state required

### Privacy Preserved
- Ticket amounts remain SEAL encrypted
- Only TEE can decrypt via seal_approve_tee
- Swap intents are public but amounts are private
- End-to-end privacy-preserving swaps

---

## Next Steps

1. Implement IntentQueue + SwapIntent structs
2. Update create_swap_intent to use queue
3. Add mark_intent_completed function
4. Deploy contracts
5. Update backend polling loop
6. Test end-to-end flow
