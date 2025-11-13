# Mist Protocol Contract - Simple Explanation

## What This Contract Does

This contract lets users **swap tokens privately** by wrapping real tokens (SUI/USDC) into encrypted versions (eSUI/eUSDC), so nobody can see how much they're trading.

---

## Main Flow (3 Steps)

### Step 1: Wrap - Turn Real Tokens into Encrypted Tokens

```
User has: 100 SUI
User calls: wrap_sui(pool, 100 SUI, encrypted_pointer)
       ↓
Contract locks 100 SUI in pool
       ↓
User gets: eSUI token (with encrypted balance pointer)
```

**What happens:**
- Real tokens get locked in the pool
- User receives an encrypted token (eSUI or eUSDC)
- The `encrypted_pointer` hides the actual amount

### Step 2: Swap - Request → TEE Executes → Update

**NEW FLOW (Updated!):**

**2a. User Requests Swap:**
```
User calls: request_swap_sui_to_usdc(esui, eusdc, swap_amount_encrypted)
       ↓
Contract emits: SwapRequestEvent
       ↓
TEE (Nautilus) sees the event
```

**2b. TEE Executes Swap on Cetus:**
```
TEE:
  1. Decrypts swap_amount_encrypted → knows user wants to swap X SUI
  2. Calls Cetus DEX: swap X SUI → Y USDC
  3. Gets swap result (Y USDC received)
  4. Creates new encrypted pointers for updated balances
```

**2c. TEE Updates Contract:**
```
TEE calls: update_after_swap_sui_to_usdc(
    pool,
    esui,
    eusdc,
    X,  // sui_spent
    Y,  // usdc_received
    new_esui_pointer,
    new_eusdc_pointer
)
       ↓
Contract:
  - Verifies caller is TEE ✓
  - Updates eSUI pointer
  - Updates eUSDC pointer
  - Emits SwapExecutedEvent
```

### Step 3: Unwrap - Turn Encrypted Tokens Back to Real Tokens

```
User has: eUSDC (encrypted balance)
User calls: unwrap_usdc(pool, eusdc, 50, recipient)
       ↓
Contract sends 50 USDC to recipient
       ↓
eUSDC token is burned
```

---

## Key Objects

### LiquidityPool (Shared)
- Holds all locked SUI and USDC
- Tracks TEE authority address
- Can be paused by admin

### eSUI Token (User Owned)
- Represents encrypted SUI balance
- Contains `balance_pointer` (encrypted amount)
- User can transfer, split, or unwrap

### eUSDC Token (User Owned)
- Represents encrypted USDC balance
- Contains `balance_pointer` (encrypted amount)
- User can transfer, split, or unwrap

---

## Main Functions

### User Functions

**Wrap:**
- `wrap_sui()` - Deposit SUI → get eSUI
- `wrap_usdc()` - Deposit USDC → get eUSDC

**Merge:**
- `merge_sui()` - Add more SUI to existing eSUI
- `merge_usdc()` - Add more USDC to existing eUSDC

**Request Swap:**
- `request_swap_sui_to_usdc()` - Ask TEE to swap SUI → USDC
- `request_swap_usdc_to_sui()` - Ask TEE to swap USDC → SUI

**Unwrap:**
- `unwrap_sui()` - Burn eSUI → get real SUI
- `unwrap_usdc()` - Burn eUSDC → get real USDC
- `unwrap_sui_partial()` - Withdraw some, keep eSUI
- `unwrap_usdc_partial()` - Withdraw some, keep eUSDC

**Transfer:**
- `transfer_esui()` - Send eSUI to someone
- `transfer_eusdc()` - Send eUSDC to someone
- `split_and_send_esui()` - Split eSUI and send part
- `split_and_send_eusdc()` - Split eUSDC and send part

### TEE-Only Functions

**Update After Swap:**
- `update_after_swap_sui_to_usdc()` - TEE updates pointers after SUI→USDC swap
- `update_after_swap_usdc_to_sui()` - TEE updates pointers after USDC→SUI swap

**Important:** Only the TEE authority can call these!

### Admin Functions

- `update_tee_authority()` - Change TEE address
- `set_pause()` - Emergency pause

---

## Events (For Backend/Frontend)

### WrapEvent
When user wraps tokens:
- user, token_type, amount, pointer

### SwapRequestEvent ⭐ NEW
When user requests a swap:
- user, esui_id, eusdc_id, from_token, to_token, swap_amount_encrypted

### SwapExecutedEvent ⭐ NEW
When TEE completes a swap:
- user, from_token, to_token, from_amount, to_amount, timestamp

### UnwrapEvent
When user unwraps tokens:
- user, token_type, amount, recipient

---

## How Privacy Works

### What's Hidden:
- **Amount in eSUI/eUSDC:** Encrypted in `balance_pointer`
- **Swap amount:** Encrypted in `swap_amount_encrypted`
- Nobody can see how much you're trading!

### What's Visible:
- You have eSUI/eUSDC tokens ✓
- You requested a swap ✓
- A swap was executed ✓
- **But amounts are encrypted!** ✗

### Who Can Decrypt:
- **Frontend/Seal:** Creates encrypted pointers
- **TEE (Nautilus):** Decrypts to execute swaps
- **Contract:** Just stores encrypted bytes (can't decrypt)

---

## Updated Swap Flow (Key Changes)

### Old Design:
- TEE directly called `swap_sui_to_usdc()` and moved tokens in pool

### New Design (Current):
1. **User requests** → Emits SwapRequestEvent
2. **TEE listens** → Sees request
3. **TEE swaps on Cetus** → Gets real tokens from DEX
4. **TEE updates contract** → Just updates pointers (no token movement in pool!)

**Why this is better:**
- TEE handles actual DEX interaction (more flexible)
- Pool doesn't need to hold swap liquidity
- Pointers track balances, real swaps happen on Cetus

---

## Integration Points

### Frontend:
1. User deposits SUI
2. **Frontend encrypts amount** using Seal → `encrypted_pointer`
3. Frontend calls `wrap_sui(pool, payment, encrypted_pointer)`
4. User receives eSUI token
5. User wants to swap → Frontend calls `request_swap_sui_to_usdc(esui, eusdc, swap_amount_encrypted)`

### Nikola's TEE (Nautilus):
1. **Listens for SwapRequestEvent**
2. Decrypts `swap_amount_encrypted` → knows amount
3. Calls Cetus DEX to execute swap
4. Gets swap result (amounts)
5. Creates new encrypted pointers
6. **Calls `update_after_swap_...()`** with new pointers

---

## What You Fixed

### Before Your Update:
- Swaps moved tokens in/out of pool
- Used `balance::create_for_testing()` (mock)

### After Your Update: ✅
- Swaps are **request → execute → update** pattern
- Pool just tracks locked deposits
- Real swaps happen on Cetus (outside contract)
- Cleaner separation of concerns!

---

## Next Steps

1. ✅ Contract code updated
2. ⏳ Test compilation: `sui move build`
3. ⏳ Fix USDC dependency issue (testnet compatibility)
4. ⏳ Deploy to testnet
5. ⏳ Test wrap/unwrap flow
6. ⏳ Integrate with Nikola's TEE

---

## Questions for Nikola

1. **Event listening:** Will your TEE listen for `SwapRequestEvent`?
2. **Cetus integration:** How will TEE call Cetus DEX?
3. **Pointer encryption:** What encryption scheme for pointers? (Seal format?)
4. **Initial tokens:** Do users need both eSUI and eUSDC before first swap?
