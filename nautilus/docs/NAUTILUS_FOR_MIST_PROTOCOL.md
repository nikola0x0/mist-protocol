# What We Actually Do With Nautilus in Mist Protocol

**Question:** If we set up AWS Nitro, what's the actual purpose for our project and how do we achieve that?

---

## Your Architecture (From the Diagram)

```
Nikola's Track:
1. Nautilus + TEE → Intent → Assets → Wallet
2. Input wallet into DEX (SWAP on Cetus: USDC/SUI, CETUS/WMNT/FlowX)

Max's Track:
3. Escrow Contract (deposit → return encrypted eUSDC)
4. Create Intent (interact with DEX)
5. Backend (no TEE yet):
   - Listen for intent events
   - Execute tx with decrypted amount
   - Provide decrypt for user
   - Walrus (Data Access Layer)
```

---

## THE REAL QUESTION: What Does Nautilus Actually Do Here?

Looking at your architecture, I need to clarify: **Nautilus might not be the right fit for this workflow**.

Here's why:

### What Nautilus IS Good For:
✅ **Off-chain computation that needs verification**
- Example: Fetching weather data and proving it's real
- Example: Twitter API calls with verifiable responses
- Example: Complex calculations that need TEE guarantees

### What Nautilus IS NOT:
❌ **Not a wallet generator** (you don't need TEE for that)
❌ **Not a DEX integration layer** (Cetus has public APIs)
❌ **Not a backend server** (regular Node.js/Rust backend is better)

---

## Actual Use Cases for Nautilus in Privacy DeFi

Let me propose **what Nautilus could actually solve** in a privacy-preserving DeFi context:

### Option 1: Private Swap Intent Matching (Most Relevant)

**Problem:** How do you match trading intents without revealing amounts?

**Nautilus Solution:**
```
User A → Submit encrypted intent to Nautilus TEE
User B → Submit encrypted intent to Nautilus TEE
         ↓
Nautilus TEE:
  1. Decrypt both intents inside secure enclave
  2. Match trades (e.g., A wants to sell 100 USDC, B wants to buy 100 USDC)
  3. Execute swap logic
  4. Return signed proof of match
         ↓
On-chain verification:
  - Verify Nautilus signature
  - Execute matched swap on DEX
  - Neither party sees other's original amount
```

**This is useful for:**
- Privacy-preserving order books
- Dark pools
- MEV protection

### Option 2: Encrypted Balance Computation

**Problem:** How do you compute on encrypted balances?

**Nautilus Solution:**
```
User → Deposit into escrow (encrypted amount)
     ↓
User → Request computation from Nautilus
     ↓
Nautilus TEE:
  1. Decrypt amount inside enclave
  2. Perform computation (interest, fees, swap amounts)
  3. Re-encrypt result
  4. Return signed encrypted result
     ↓
On-chain verification:
  - Verify Nautilus signature
  - Store encrypted result
```

### Option 3: Verifiable Price Oracle with Privacy

**Problem:** You need accurate DEX prices without revealing your trade intent

**Nautilus Solution:**
```
Nautilus TEE:
  1. Fetch real-time prices from Cetus DEX API
  2. Compute best swap route
  3. Sign the price data
     ↓
Your contract:
  - Verify Nautilus signature
  - Execute swap at verified price
  - No frontrunning because intent was private
```

---

## RECOMMENDED: What You Should Actually Build

Based on your diagram, here's what makes sense:

### Architecture Revision

**WITHOUT Nautilus (Simpler, Still Private):**

```
1. Escrow Contract with Seal Encryption
   ↓
   User deposits → encrypted amount stored on-chain

2. Intent Creation (Frontend)
   ↓
   User creates swap intent → encrypted with Seal

3. Backend Server (Regular Node.js/Rust)
   ↓
   - Holds decryption keys temporarily
   - Listens for intent events
   - Decrypts amount
   - Executes swap on Cetus
   - Stores metadata on Walrus

4. User retrieves result
   ↓
   - Backend provides decrypted result
   - User verifies on-chain
```

**Privacy guarantees:**
- ✅ Amounts encrypted with Seal (threshold encryption)
- ✅ Only backend can decrypt (with 2-of-3 threshold keys)
- ✅ Metadata stored on Walrus (decentralized)
- ✅ Simpler, faster, debuggable

**WITH Nautilus (More Complex, Better Guarantees):**

```
1. Escrow Contract with Seal Encryption
   ↓
   User deposits → encrypted amount stored on-chain

2. Intent Creation (Frontend)
   ↓
   User creates swap intent → encrypted with Seal

3. Nautilus TEE (AWS Nitro)
   ↓
   - Holds decryption keys inside enclave
   - Listens for intent events
   - Decrypts amount IN TEE (secure)
   - Computes optimal swap route
   - Signs the execution plan

4. Backend Server
   ↓
   - Receives signed plan from Nautilus
   - Executes swap on Cetus
   - Stores metadata on Walrus

5. On-chain verification
   ↓
   - Verify Nautilus signature
   - Prove computation was done in TEE
   - User gets verifiable privacy
```

**Additional guarantees:**
- ✅ Decryption keys never leave TEE
- ✅ Computation is verifiable (attestation)
- ✅ Reproducible builds prove no backdoors
- ✅ Better than Encifher (self-managed)

---

## What Nautilus Would Actually Do (Concrete Implementation)

### Endpoint 1: `/process_intent`

**Input:**
```json
{
  "payload": {
    "encrypted_amount": "a3f5b2...",  // Seal encrypted
    "token_in": "USDC",
    "token_out": "SUI",
    "user_address": "0x123..."
  }
}
```

**Nautilus TEE Process:**
1. Decrypt `encrypted_amount` using Seal keys stored in enclave
2. Fetch current prices from Cetus DEX
3. Calculate optimal swap route
4. Compute expected output amount
5. Sign the execution plan

**Output:**
```json
{
  "response": {
    "intent": 0,
    "timestamp_ms": 1744041600000,
    "data": {
      "swap_route": ["USDC", "SUI"],
      "expected_output": 123.45,
      "price": 0.85,
      "slippage": 0.5
    }
  },
  "signature": "b75d2d44c4a6b3c676fe087465c0e85206b101e21be6cda4..."
}
```

**Backend receives this, executes swap on Cetus, submits to blockchain for verification.**

---

## Honest Assessment: Do You Actually Need Nautilus?

### ❌ You DON'T need Nautilus if:
- You just want to integrate with Cetus DEX (use their SDK)
- You just want to encrypt amounts (use Seal directly)
- You just want a backend to execute swaps (use Node.js/Rust)
- Time is limited (Nautilus adds complexity)

### ✅ You DO need Nautilus if:
- You want **verifiable** privacy (prove computation happened in TEE)
- You want to store decryption keys securely (better than regular backend)
- You want to demonstrate understanding of TEE architecture
- You want a competitive advantage over Encifher

---

## My Recommendation for Hackathon

### **Day 1-3: Build Without Nautilus**

```
Priority 1: Get basic flow working
- Escrow contract (encrypted deposits)
- Intent creation (frontend)
- Backend server (Node.js)
- Cetus DEX integration
- Walrus storage

Result: Working demo with encrypted amounts
```

### **Day 4: Add Nautilus IF Time Permits**

```
Optional Enhancement:
- Deploy Nautilus enclave
- Move decryption logic into TEE
- Add attestation verification
- Show judges the difference

Demo: "Here's the basic version, here's the TEE-enhanced version"
```

### **Alternative: Document Nautilus Without Building**

```
Strategy: Architectural Understanding
- Build without Nautilus
- Document how Nautilus would enhance it
- Show understanding in presentation
- Compare to Encifher

Judges care about:
1. Does the project work? ✅
2. Do you understand the tech? ✅
3. What's the innovation? ✅
```

---

## What I Recommend We Do RIGHT NOW

### Option A: Skip Nautilus, Focus on Core Features
**Pros:**
- ✅ Working demo guaranteed
- ✅ Less complexity
- ✅ More time for polish
- ✅ Still demonstrates privacy

**Cons:**
- ❌ Less impressive technically
- ❌ Weaker vs Encifher comparison

### Option B: Build with Nautilus
**Pros:**
- ✅ Actual TEE implementation
- ✅ Verifiable computation
- ✅ Strong technical demonstration
- ✅ True decentralization

**Cons:**
- ⚠️ 1-2 days setup time
- ⚠️ Debugging difficulty
- ⚠️ Risk of not finishing

### Option C: Hybrid (My Recommendation)
**Approach:**
1. Days 1-3: Build core flow without Nautilus
2. Day 3 evening: Decide if ahead of schedule
3. Day 4: Add Nautilus if time permits
4. Fallback: Document Nautilus architecture

**Best of both worlds:**
- ✅ Working demo guaranteed
- ✅ Option to add Nautilus
- ✅ Good presentation either way

---

## Concrete Next Steps

### If we go with Nautilus:

**What we'll build:**
```rust
// nautilus/src/apps/mist-protocol/mod.rs

pub struct SwapIntentRequest {
    pub encrypted_amount: String,  // Seal encrypted
    pub token_in: String,
    pub token_out: String,
    pub user_address: String,
}

pub struct SwapIntentResponse {
    pub swap_route: Vec<String>,
    pub expected_output: f64,
    pub price: f64,
    pub slippage: f64,
}

pub async fn process_intent(
    State(state): State<Arc<AppState>>,
    Json(request): Json<ProcessDataRequest<SwapIntentRequest>>,
) -> Result<Json<ProcessedDataResponse<IntentMessage<SwapIntentResponse>>>, EnclaveError> {
    // 1. Decrypt amount using Seal (inside TEE)
    let amount = decrypt_seal_amount(&request.payload.encrypted_amount)?;

    // 2. Fetch Cetus prices
    let price = fetch_cetus_price(&request.payload.token_in, &request.payload.token_out).await?;

    // 3. Calculate swap
    let expected_output = amount * price;

    // 4. Sign response
    Ok(Json(to_signed_response(
        &state.eph_kp,
        SwapIntentResponse {
            swap_route: vec![request.payload.token_in, request.payload.token_out],
            expected_output,
            price,
            slippage: 0.5,
        },
        current_timestamp_ms(),
        IntentScope::ProcessData,
    )))
}
```

### If we skip Nautilus:

**What we'll build:**
```typescript
// backend/src/intent-processor.ts

export class IntentProcessor {
  async processIntent(encryptedIntent: string) {
    // 1. Decrypt with Seal
    const amount = await seal.decrypt(encryptedIntent);

    // 2. Fetch Cetus prices
    const price = await cetus.getPrice('USDC', 'SUI');

    // 3. Execute swap
    const tx = await cetus.swap(amount, 'USDC', 'SUI');

    // 4. Store on Walrus
    await walrus.store({ tx, amount, price });

    return tx;
  }
}
```

---

## Final Answer to Your Question

**"What do we actually do if we have AWS Nitro setup?"**

**We build a Nautilus enclave that:**
1. Securely decrypts Seal-encrypted amounts inside TEE
2. Fetches Cetus DEX prices and computes optimal swaps
3. Signs the execution plan with verifiable attestation
4. Provides cryptographic proof that computation was done correctly

**Purpose:** Verifiable privacy - users can prove their amounts were processed securely without revealing them publicly.

**Advantage over Encifher:** Self-managed, reproducible, verifiable vs. black-box service.

**Reality check:** This is cool but complex. For hackathon, simpler backend might be smarter.

**My vote:** Start without Nautilus, add it Day 4 if ahead of schedule.

---

What do you think? Should we:
1. **Go full Nautilus** (set up AWS now)
2. **Build without Nautilus** (focus on core features)
3. **Hybrid approach** (core first, Nautilus if time permits)
