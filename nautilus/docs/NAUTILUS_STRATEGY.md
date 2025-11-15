# Nautilus Strategy for Mist Protocol (Based on PRD)

**Date:** 2025-11-11
**Based on:** hackathon-prd.md analysis

---

## What the PRD Says About Nautilus

From the PRD (lines 90-113):

```
### 3. Nautilus TEE Computation ⭐ KEY DIFFERENTIATOR

Why: Self-managed verifiable computation vs black box

User Flow:
1. User submits encrypted transaction
2. Nautilus TEE processes in secure enclave
3. TEE returns signed result with attestation
4. Contract verifies attestation on-chain

Technical Implementation:
- Deploy simple Nautilus enclave on AWS Nitro
- Implement basic computation endpoint (Rust + Axum)
- Attestation verification in Move contract
- Reproducible build for demo transparency

Deliverable: Working TEE that proves correct execution with on-chain verification

Complexity: High (AWS setup, enclave deployment)

Fallback: Mock attestation if AWS setup fails
```

---

## The Real Purpose (From PRD Context)

Looking at the **ENTIRE PRD**, here's what Nautilus actually does:

### Primary Use Case: Process Encrypted Stealth Payments

From the architecture and flows, Nautilus is supposed to:

1. **Receive encrypted stealth payment data**
   - Encrypted amount (from Seal)
   - Stealth address metadata
   - Transaction intent

2. **Process inside TEE**
   - Decrypt amount securely
   - Validate payment structure
   - Compute any necessary values
   - Sign the result

3. **Return verified computation**
   - Signed output
   - Attestation document
   - PCR values for verification

4. **Enable on-chain verification**
   - Move contract verifies attestation
   - Proves computation was done in TEE
   - No trust required

---

## Why This Matters vs Encifher

From PRD comparison section (lines 1468-1477):

```
│ Feature             │ Encifher │ Our Solution │
├────────────────────┼──────────┼──────────────┤
│ Amount Privacy     │    ✅    │      ✅      │
│ Recipient Privacy  │    ❌    │      ✅      │  <- Stealth addresses
│ Threshold Crypto   │    ❌    │      ✅      │  <- Seal
│ Self-Managed TEE   │    ❌    │      ✅      │  <- Nautilus!
│ Verifiable         │    ❌    │      ✅      │  <- Nautilus!
│ Cost Efficient     │    ❌    │      ✅      │  <- Walrus
│ Decentralized      │    ❌    │      ✅      │  <- Everything together
```

**Nautilus's role:** Provide **verifiable, self-managed TEE** vs Encifher's black box.

---

## What We Should Actually Build

Based on PRD + our parallel-tasks diagram:

### Nautilus Computation Endpoint

```rust
// nautilus/src/apps/mist-protocol/mod.rs

#[derive(Debug, Serialize, Deserialize)]
pub struct StealthPaymentRequest {
    pub encrypted_amount: String,      // Seal encrypted
    pub stealth_address: String,       // Generated stealth address
    pub ephemeral_pubkey: String,      // For recipient discovery
    pub walrus_blob_id: String,        // Metadata on Walrus
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StealthPaymentResponse {
    pub verified_amount: u64,          // Decrypted amount
    pub payment_valid: bool,           // Validation result
    pub computation_proof: String,     // Proof of correct computation
}

pub async fn process_stealth_payment(
    State(state): State<Arc<AppState>>,
    Json(request): Json<ProcessDataRequest<StealthPaymentRequest>>,
) -> Result<Json<ProcessedDataResponse<IntentMessage<StealthPaymentResponse>>>, EnclaveError> {

    // 1. Decrypt amount using Seal keys stored in TEE
    let amount = decrypt_seal_amount(&request.payload.encrypted_amount)?;

    // 2. Validate stealth payment structure
    let valid = validate_stealth_payment(
        &request.payload.stealth_address,
        &request.payload.ephemeral_pubkey,
        amount,
    )?;

    // 3. Sign the computation result
    Ok(Json(to_signed_response(
        &state.eph_kp,
        StealthPaymentResponse {
            verified_amount: amount,
            payment_valid: valid,
            computation_proof: generate_proof(amount, valid),
        },
        current_timestamp_ms(),
        IntentScope::ProcessData,
    )))
}
```

---

## Day-by-Day Implementation (From PRD)

### **Day 1 Afternoon (4 hours)** - Setup

From PRD lines 628-655:

```
- [ ] Deploy simple "hello world" Nautilus enclave (if doing TEE)
- [ ] Verify Seal testnet servers are accessible
```

**What to do:**
- Set up AWS account ✅ (you have this!)
- Install AWS CLI
- Configure AWS credentials
- Clone nautilus-framework
- Test weather example locally

### **Day 4 Morning (4 hours)** - Nautilus

From PRD lines 806-838:

```
- [ ] Deploy Nautilus enclave with attestation
- [ ] Implement computation endpoint
- [ ] Deploy nautilus_verifier.move
- [ ] Test attestation verification
```

**What to do:**
1. Run `configure_enclave.sh` with your AWS credentials
2. Deploy enclave to AWS Nitro
3. Implement stealth payment processing endpoint
4. Test attestation generation
5. Deploy Move verifier contract
6. Test on-chain verification

---

## Complexity vs Priority Analysis

### From PRD (line 110-112):

```
Complexity: High (AWS setup, enclave deployment)
Fallback: Mock attestation if AWS setup fails
```

### From PRD Priority Rankings (lines 1537-1549):

```
MUST HAVE (Core Demo):
1. ✅ Stealth addresses (KEY DIFFERENTIATOR)
2. ✅ Basic Seal encryption (2-of-3)
3. ✅ Working UI (send + receive)
4. ✅ One complete user flow
5. ✅ Move contracts deployed

SHOULD HAVE (Impressive Demo):
6. 🎯 Nautilus TEE integration      <- HERE!
7. 🎯 Walrus storage integration
8. 🎯 Attestation verification
9. 🎯 Cost comparison UI
```

**Nautilus is in "SHOULD HAVE" = Do it if time permits, not critical for demo.**

---

## Decision Matrix

### **OPTION A: Full Nautilus (With AWS)**

**Timeline:**
- Day 1: AWS setup (2-4 hours)
- Day 4: Nautilus implementation (4-8 hours)
- **Total:** 6-12 hours

**Pros:**
- ✅ Real TEE attestation
- ✅ Verifiable computation
- ✅ Strong vs Encifher comparison
- ✅ Shows technical depth
- ✅ Reproducible builds

**Cons:**
- ⚠️ High complexity
- ⚠️ Debugging difficulty
- ⚠️ Risk of failure
- ⚠️ Time pressure

**Success Probability:** 60-70%

---

### **OPTION B: Mock Nautilus (No AWS)**

**Timeline:**
- Day 1: Mock backend setup (1-2 hours)
- Day 4: Polish mock (1 hour)
- **Total:** 2-3 hours

**Pros:**
- ✅ Fast implementation
- ✅ No AWS complexity
- ✅ Focus on stealth addresses
- ✅ More time for polish
- ✅ Still demonstrates concept

**Cons:**
- ❌ No real attestation
- ❌ Weaker technical story
- ❌ Can't verify on-chain
- ❌ Less impressive

**Success Probability:** 95%

---

### **OPTION C: Hybrid (PRD Recommended)**

From PRD lines 1551-1558:

```
If Only 3 Days:
Day 1: Setup + Seal integration
Day 2: Stealth addresses (all day)
Day 3: UI + Demo video

Cut: Nautilus, Walrus (mention in slides only)
```

**Timeline:**
- Days 1-3: Build without Nautilus
- Day 3 Evening: Decision point
- Day 4: Add Nautilus if ahead
- **Fallback:** Document in slides

**Pros:**
- ✅ Guaranteed working demo
- ✅ Option to add real Nautilus
- ✅ Risk mitigation
- ✅ Best of both worlds

**Cons:**
- ⚠️ May not have time for Nautilus
- ⚠️ Requires discipline to stick to plan

**Success Probability:** 85-90%

---

## My Recommendation (Based on PRD)

### **Go with Option C: Hybrid Approach**

**Reasoning:**

1. **PRD Priority is Clear:**
   - Stealth addresses = MUST HAVE
   - Seal encryption = MUST HAVE
   - Nautilus = SHOULD HAVE
   - Walrus = SHOULD HAVE

2. **PRD Explicitly Says:**
   > "Fallback: Mock attestation if AWS setup fails" (line 112)

   The PRD authors **expected** Nautilus might not work!

3. **Success Metrics:**
   From lines 1759-1760:
   > "Stealth addresses alone are novel on Sui and better than Encifher.
   > Combined with threshold crypto, it's a strong hackathon project."

4. **Time Allocation:**
   - Day 1: Setup + Infrastructure
   - Day 2: Seal Integration
   - Day 3: **Stealth Addresses (ENTIRE DAY)**
   - Day 4: Nautilus + Walrus
   - Day 5: Polish

   Stealth addresses get a full day = highest priority.

---

## Concrete Action Plan

### **Today (Day 1):**

✅ **Install AWS CLI**
```bash
brew install awscli  # Mac
aws configure
```

✅ **Set Up AWS Credentials**
```bash
export AWS_ACCESS_KEY_ID=<your-key>
export AWS_SECRET_ACCESS_KEY=<your-secret>
export KEY_PAIR=<your-keypair>
```

✅ **Test Nautilus Locally (Optional)**
```bash
cd nautilus/nautilus-framework
make ENCLAVE_APP=weather-example
# See if it builds
```

✅ **Focus on Core Setup:**
- Next.js project
- Wallet integration
- Move contracts skeleton
- Seal SDK testing

### **Days 2-3:**

🎯 **Build Core Features WITHOUT Nautilus:**
- Day 2: Seal encryption/decryption
- Day 3: Stealth addresses (FULL DAY)
- Result: Working private payments

### **Day 3 Evening (DECISION POINT):**

**Ask yourself:**
- [ ] Are stealth payments working perfectly?
- [ ] Is Seal encryption working?
- [ ] Is UI functional?
- [ ] Are we ahead of schedule?

**IF YES → Deploy Nautilus on Day 4**
**IF NO → Use mock + document in slides**

### **Day 4 (IF DOING NAUTILUS):**

**Morning:**
```bash
cd nautilus/nautilus-framework
export APP_NAME=mist-protocol

# Configure enclave
sh configure_enclave.sh mist-protocol

# Deploy to AWS
make ENCLAVE_APP=mist-protocol
make run

# Expose endpoint
sh expose_enclave.sh
```

**Afternoon:**
- Register enclave on-chain
- Test attestation
- Integrate with frontend
- Add verification to Move contracts

### **Day 4 (IF NOT DOING NAUTILUS):**

**Create mock backend:**
```typescript
// nautilus/mock-backend/index.ts
export async function mockNautilusVerification(data: any) {
  return {
    verified: true,
    signature: "mock_signature",
    attestation: "mock_attestation",
    note: "Production would use real AWS Nitro TEE",
  };
}
```

**Update presentation slides:**
- Explain how Nautilus would work
- Show architecture diagram
- Compare with Encifher
- Emphasize "self-managed" vs "black box"

---

## What to Tell Judges

### **If You Have Real Nautilus:**

> "We've deployed our own Nautilus TEE on AWS Nitro Enclaves. Anyone can verify our computation by rebuilding the enclave and checking the PCR values match what's registered on-chain. This is the key advantage over Encifher's black-box gateway—complete transparency and verifiability."

### **If You Have Mock Nautilus:**

> "We've implemented the architecture for Nautilus TEE integration. Due to time constraints, we're demonstrating with a mock backend, but the production version would run in AWS Nitro Enclaves with real attestation verification. The key innovation is the architectural pattern—self-managed, verifiable TEE rather than Encifher's centralized gateway."

**Judges will understand** - hackathons are about demonstrating understanding, not perfect production systems.

---

## Encifher Comparison (Key Talking Point)

From PRD (lines 1393-1403):

```
Current solutions like Encifher:
❌ Centralized gateways (single point of failure)
❌ Black box computation (can't verify)
❌ Recipients visible on-chain
❌ Expensive on-chain storage
❌ No user control
```

**Our solution with Nautilus (even mocked):**
```
✅ Self-managed TEE (you run it)
✅ Reproducible builds (anyone can verify)
✅ Recipients hidden (stealth addresses)
✅ Cost-efficient (Walrus)
✅ Full user control (your keys, your servers)
```

**The architectural innovation is what matters**, not whether you deployed to AWS in 5 days.

---

## Final Answer

### **What should we do with Nautilus?**

**SHORT TERM (Days 1-3):**
1. Install AWS CLI today ✅
2. Configure credentials ✅
3. Keep it as an option ✅
4. **Focus on stealth addresses + Seal** ✅

**MEDIUM TERM (Day 3 Evening):**
1. Assess progress ✅
2. Decide: Real or mock ✅
3. Commit to one path ✅

**LONG TERM (Day 4):**
1. **IF ahead:** Deploy real Nautilus
2. **IF behind:** Use mock + slides
3. **EITHER WAY:** Have working demo

### **You have AWS credit ($100) and time (5 days)**

**My vote:** Try for real Nautilus, but don't let it block the demo.

---

## Next Immediate Steps

1. ✅ Install AWS CLI
2. ✅ Configure AWS credentials
3. ✅ Test local Nautilus build
4. ✅ Focus on frontend + Seal integration
5. ✅ Build stealth addresses
6. 🔜 Day 3 evening: Decide on Nautilus

**Let's start with AWS setup while we build the core features in parallel.**

Ready to proceed?
