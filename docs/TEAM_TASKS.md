# Mist Protocol - Sprint Stories

**Team:** Nikola (Frontend/Contract), Max (Backend/Cetus), Hung (Infrastructure)

---

## 📖 User Stories

### Epic 1: Enhanced Privacy Through Ticket Sharding

**Goal:** Improve deposit privacy by automatically splitting deposits into multiple encrypted shards, making it harder for observers to correlate deposits with swap intents.

---

### Story 1.1: Frontend Shard Generation (Nikola)

**As a** frontend developer
**I want to** generate multiple encrypted shards from a single deposit amount
**So that** user deposits are obfuscated and harder to track

#### Acceptance Criteria
- [ ] User can choose number of shards (5-10) via UI slider
- [ ] `generateShards()` function splits amount randomly
- [ ] All shards encrypt independently with SEAL
- [ ] Shards sum correctly to original amount
- [ ] UI shows shard count indicator

#### Technical Tasks
- [ ] Create `generateShards()` in `frontend/lib/seal-vault.ts`
- [ ] Add `encryptShards()` function for batch encryption
- [ ] Update `WrapCard.tsx` with shard configuration UI
- [ ] Add visual feedback for shard creation
- [ ] Write tests for shard generation

#### Files to Modify
- `frontend/lib/seal-vault.ts`
- `frontend/components/WrapCard.tsx`

#### Definition of Done
- User deposits 100 SUI → gets 5-10 encrypted shards
- Tests pass: sum verification, random distribution
- UI shows "Creating 5 shards..." indicator

---

### Story 1.2: Smart Contract Sharded Deposits + Nautilus Verification (Nikola)

**As a** smart contract developer
**I want to** accept multiple encrypted amounts and verify TEE attestations
**So that** users can create tickets and swaps are provably secure

#### Acceptance Criteria
- [ ] `wrap_sui_sharded()` accepts `vector<vector<u8>>` (multiple encrypted amounts)
- [ ] Single transaction creates all tickets
- [ ] `register_tee_enclave()` stores TEE public key + PCR values
- [ ] `complete_swap()` verifies TEE signature on swap results
- [ ] Event emitted with shard count
- [ ] All Move tests pass

#### Technical Tasks
- [ ] Create `wrap_sui_sharded()` function
- [ ] Add `register_tee_enclave()` for Nautilus attestation verification
- [ ] Add `complete_swap()` with TEE signature verification
- [ ] Store registered TEE public key in contract state
- [ ] Update `create_swap_intent_optimized()` for multiple ticket IDs
- [ ] Add `DepositShardedEvent`
- [ ] Write Move unit tests

#### Files to Modify
- `contracts/mist_protocol/sources/mist_protocol.move`

#### Definition of Done
- Contract deployed on testnet
- TEE enclave registered with attestation
- User can deposit with 5 shards in one tx
- Swap results verified via TEE signature
- Tests verify attestation and signature checks

#### Resources
- [Nautilus Documentation](https://docs.sui.io/concepts/cryptography/nautilus)
- [Nautilus Design](https://docs.sui.io/concepts/cryptography/nautilus/nautilus-design)

---

### Story 1.3: Multi-Ticket Intent Support (Nikola)

**As a** user
**I want to** select multiple tickets when creating swap intent
**So that** I can use sharded deposits for swaps

#### Acceptance Criteria
- [ ] UI allows selecting multiple tickets
- [ ] Selected tickets display total amount
- [ ] Intent creation uses multiple ticket IDs
- [ ] Backend can decrypt and sum all tickets

#### Technical Tasks
- [ ] Update swap intent UI for multi-ticket selection
- [ ] Show total of selected tickets
- [ ] Pass ticket IDs array to contract
- [ ] Integration test with Max's backend

#### Files to Modify
- Frontend swap intent components

#### Definition of Done
- User selects 3 tickets → sees total
- Creates intent → backend processes successfully

---

### Epic 2: Unified Backend & Real Swap Execution

**Goal:** Consolidate both Axum servers into one and replace mock swaps with real Cetus DEX integration.

---

### Story 2.1: Cetus Module Migration (Max)

**As a** backend developer
**I want to** move existing Cetus code into main backend structure
**So that** all code is in one place

#### Acceptance Criteria
- [ ] Cetus code in `backend/src/apps/mist-protocol/cetus/`
- [ ] `CetusClient` struct exported from `mod.rs`
- [ ] All existing Cetus functionality preserved
- [ ] Code compiles without errors

#### Technical Tasks
- [ ] Move files from `cetus-swap/backend/src/`
- [ ] Create module structure (mod.rs, api.rs, transaction.rs, types.rs)
- [ ] Implement `CetusClient` with methods:
  - `get_best_pool()`
  - `get_quote()`
  - `build_swap_ptb()`

#### Files to Create
- `backend/src/apps/mist-protocol/cetus/mod.rs`
- `backend/src/apps/mist-protocol/cetus/api.rs`
- `backend/src/apps/mist-protocol/cetus/transaction.rs`
- `backend/src/apps/mist-protocol/cetus/types.rs`

#### Definition of Done
- Cetus module compiles
- `CetusClient` can fetch pools
- PTB building works

---

### Story 2.2: Unified Axum Router (Max)

**As a** backend developer
**I want to** merge both Axum servers into one
**So that** we have single port and unified configuration

#### Acceptance Criteria
- [ ] Single server on port 3001
- [ ] All routes working (health, attestation, SEAL, Cetus)
- [ ] `AppState` includes `CetusClient`
- [ ] Pool cache implemented

#### Technical Tasks
- [ ] Merge routes in `main.rs`
- [ ] Extend `AppState` with Cetus fields
- [ ] Add routes: `/api/pools`, `/api/pool/:id`, `/api/quote`
- [ ] Update `Cargo.toml` dependencies
- [ ] Update `.env.example`

#### Files to Modify
- `backend/src/main.rs`
- `backend/src/lib.rs`
- `backend/Cargo.toml`
- `backend/.env.example`

#### Definition of Done
- `cargo run` starts server on 3001
- `curl localhost:3001/health` works
- `curl localhost:3001/api/pools` returns Cetus pools

---

### Story 2.3: Real Swap Execution (Max)

**As a** backend developer
**I want to** execute real swaps via Cetus
**So that** users get actual token swaps

#### Acceptance Criteria
- [ ] Mock swap code removed
- [ ] `execute_real_swap()` uses Cetus
- [ ] PTB combines: borrow → swap → return
- [ ] Encrypted output ticket created
- [ ] Transaction verified on-chain

#### Technical Tasks
- [ ] Implement `execute_real_swap()` in `swap_executor.rs`
- [ ] Build complete PTB (Mist borrow + Cetus swap + return)
- [ ] Add error handling
- [ ] Add transaction logging
- [ ] Test with SUI → USDC on testnet

#### Files to Modify
- `backend/src/apps/mist-protocol/swap_executor.rs`

#### Definition of Done
- Intent creates → backend swaps via Cetus → output ticket returned
- Transaction visible on Sui explorer
- User receives correct output token

---

### Story 2.4: Multi-Ticket Decryption (Max)

**As a** backend developer
**I want to** decrypt and sum multiple tickets
**So that** sharded deposits work end-to-end

#### Acceptance Criteria
- [ ] `decrypt_and_sum_tickets()` function implemented
- [ ] Handles vector of encrypted tickets
- [ ] Sum calculated correctly
- [ ] Integration test with Nikola's frontend

#### Technical Tasks
- [ ] Add function to `intent_processor.rs`
- [ ] Loop through all tickets and decrypt
- [ ] Sum all decrypted amounts
- [ ] Validate sum matches expected

#### Files to Modify
- `backend/src/apps/mist-protocol/intent_processor.rs`

#### Definition of Done
- Frontend sends 5 shards → backend decrypts → sum = 100
- Integration test passes

#### Coordination
**With Nikola:** Agree on encrypted shard format

---

### Epic 3: TEE Attestation & AWS Deployment

**Goal:** Deploy backend to AWS Nitro Enclave with verifiable attestation.

---

### Story 3.1: AWS EC2 & Nitro Setup (Hung)

**As an** infrastructure engineer
**I want to** provision EC2 with Nitro Enclave
**So that** we have TEE environment

#### Acceptance Criteria
- [ ] EC2 instance running (c5.xlarge)
- [ ] Nitro Enclave support enabled
- [ ] Security groups configured
- [ ] Nitro CLI installed
- [ ] Enclave allocator running (4GB RAM, 2 vCPUs)

#### Technical Tasks
- [ ] Provision EC2 via AWS CLI/Console
- [ ] Enable enclave in instance settings
- [ ] Configure security groups (443, 22)
- [ ] Install Nitro CLI
- [ ] Configure allocator

#### Definition of Done
- `ssh` into instance works
- `nitro-cli --version` succeeds
- Allocator status: active

---

### Story 3.2: Enclave Docker Configuration (Hung)

**As an** infrastructure engineer
**I want to** create Docker image for enclave
**So that** backend runs inside TEE

#### Acceptance Criteria
- [ ] `Dockerfile.enclave` builds successfully
- [ ] Entrypoint script configured
- [ ] EIF file generated
- [ ] Enclave runs without errors

#### Technical Tasks
- [ ] Create `backend/Dockerfile.enclave`
- [ ] Write `backend/enclave-entrypoint.sh`
- [ ] Build Docker image
- [ ] Build EIF with `nitro-cli build-enclave`
- [ ] Test enclave launch

#### Files to Create
- `backend/Dockerfile.enclave`
- `backend/enclave-entrypoint.sh`

#### Definition of Done
- `nitro-cli run-enclave` succeeds
- `nitro-cli describe-enclaves` shows running enclave

#### Coordination
**With Max:** Need backend binary that compiles

---

### Story 3.3: Generate Nautilus Attestation & Register TEE Onchain (Hung + Max)

**As an** infrastructure engineer and backend developer
**I want to** generate enclave keypair, create attestation, and register TEE onchain
**So that** Sui smart contracts can verify swap results from TEE

#### Acceptance Criteria
- [ ] Enclave generates Ed25519 keypair on first boot (sealed to disk)
- [ ] AWS Nitro enclave generates attestation document with PCRs + public key
- [ ] Backend submits attestation to `register_tee_enclave()` Move function
- [ ] TEE enclave registered onchain with verified PCR values
- [ ] Backend signs transactions using enclave keypair (no CLI)
- [ ] **Remove `tx-signer` service** - signing happens in TEE

#### Technical Tasks

**Hung (Infrastructure):**
- [ ] Generate enclave keypair in NSM during enclave initialization
- [ ] Seal keypair to persistent storage (survives enclave restarts)
- [ ] Generate NSM attestation document including public key
- [ ] Extract PCR measurements for documentation

**Max (Backend):**
- [ ] Add `sui-types` dependency for native signing
- [ ] Implement `generate_enclave_keypair()` function
- [ ] Replace `tx-signer` HTTP calls with direct signing:
  ```rust
  use sui_types::crypto::{Signature, SuiKeyPair};

  let signature = Signature::new_secure(
      &IntentMessage::new(Intent::sui_transaction(), tx_data),
      &state.tee_keypair
  );
  ```
- [ ] Implement `register_enclave()` to submit attestation to contract
- [ ] Remove `tx-signer` dependency from architecture

#### Files to Modify
- `backend/src/common.rs` (add keypair generation)
- `backend/src/apps/mist-protocol/swap_executor.rs` (replace signing)
- `backend/Cargo.toml` (add `sui-types`, remove `reqwest` for signing)
- `backend/enclave-entrypoint.sh` (initialize keypair on boot)

#### Files to Remove
- `tx-signer/` directory (entire service no longer needed)

#### Definition of Done
- Enclave generates and seals keypair
- TEE enclave registered onchain successfully
- Contract stores TEE public key
- Backend signs transactions natively (no CLI)
- `tx-signer` service removed
- PCR values documented for verification

#### Important Notes
**Security:** Keypair NEVER leaves the enclave. Signing happens inside TEE using Rust crypto libraries, not via external CLI.

#### Resources
- [Nautilus Documentation](https://docs.sui.io/concepts/cryptography/nautilus)
- [Using Nautilus](https://docs.sui.io/concepts/cryptography/nautilus/using-nautilus)
- [sui-types Signing Example](https://github.com/MystenLabs/sui/tree/main/crates/sui-types)

#### Coordination
**With Nikola:** Contract function for `register_tee_enclave()`

---

### Story 3.4: Deployment Automation (Hung)

**As an** infrastructure engineer
**I want to** automate enclave deployment
**So that** we can deploy updates easily

#### Acceptance Criteria
- [ ] `deploy.sh` script works end-to-end
- [ ] Systemd services configured
- [ ] Zero-downtime deployment
- [ ] Attestation verified after deploy

#### Technical Tasks
- [ ] Create `scripts/deploy.sh`
- [ ] Create `systemd/mist-enclave.service`
- [ ] Create `systemd/mist-vsock-proxy.service`
- [ ] Test deployment from local machine
- [ ] Write `docs/DEPLOYMENT.md`

#### Files to Create
- `scripts/deploy.sh`
- `systemd/mist-enclave.service`
- `systemd/mist-vsock-proxy.service`
- `docs/DEPLOYMENT.md`
- `docs/TROUBLESHOOTING.md`

#### Definition of Done
- Run `./scripts/deploy.sh` → new enclave deployed
- Services auto-restart
- Documentation complete

---

### Story 3.5: Display TEE Status (Nikola) [OPTIONAL]

**As a** frontend developer
**I want to** show users that swaps are secured by TEE
**So that** users have confidence in the protocol's security

#### Acceptance Criteria
- [ ] Display "🔒 Secured by TEE" badge
- [ ] Show TEE wallet address (from contract)
- [ ] Link to Sui Explorer showing TEE registration transaction

#### Technical Tasks
- [ ] Fetch registered TEE public key from contract
- [ ] Add simple status badge component
- [ ] Link to Sui Explorer for transparency

#### Files to Create
- `frontend/components/TEEBadge.tsx` (optional)

#### Definition of Done
- UI shows "Powered by Nautilus TEE" badge (optional)
- Users can see TEE is registered onchain

#### Note
This story is **optional** - Nautilus attestation verification happens entirely onchain. Users don't need to manually verify anything.

---

## 🔗 Integration Checkpoints

### Checkpoint 1: Nikola ↔ Max - SEAL Shard Format

**When:** After Story 1.1 and 2.4 complete

**Test:**
```typescript
// Nikola generates
const shards = generateShards(100); // [23, 18, 31, 15, 13]
const encrypted = await encryptShards(shards);

// Max decrypts
const total = decrypt_and_sum_tickets(encrypted);
assert(total === 100);
```

**Action Items:**
- [ ] Nikola: Provide test encrypted shards
- [ ] Max: Verify decryption works
- [ ] Both: Run integration test

---

### Checkpoint 2: Max ↔ Hung - Backend Binary

**When:** After Story 2.2 complete, before Story 3.2

**Test:**
```bash
# Max builds
cd backend && cargo build --release

# Hung packages
docker build -f Dockerfile.enclave -t mist-backend .
nitro-cli build-enclave --docker-uri mist-backend --output-file mist.eif
nitro-cli run-enclave --eif-path mist.eif
```

**Action Items:**
- [ ] Max: Document environment variables
- [ ] Hung: Test Docker build
- [ ] Both: Verify enclave runs

---

### Checkpoint 3: Nikola ↔ Hung - TEE Registration Onchain

**When:** After Story 1.2 and 3.3 complete

**Test:**
```move
// Nikola provides in Move contract
public fun register_tee_enclave(
    protocol: &mut MistProtocol,
    attestation: vector<u8>,
    public_key: vector<u8>,
    pcr0: vector<u8>,
    pcr1: vector<u8>,
    pcr2: vector<u8>,
    ctx: &mut TxContext
) {
    // Verify AWS attestation signature
    // Store public_key + PCRs
    // Emit TEERegisteredEvent
}

// Hung calls from backend
let tx = backend.register_enclave(attestation_doc, public_key, pcrs).await?;
assert!(tx.status == "success");
```

**Action Items:**
- [ ] Nikola: Implement `register_tee_enclave()` in Move contract
- [ ] Hung: Generate attestation and call registration function
- [ ] Both: Test registration transaction succeeds
- [ ] Both: Verify TEE public key stored onchain

---

## ✅ Sprint Success Criteria

### Must Have
- [ ] User can deposit with sharding (5+ shards)
- [ ] TEE enclave registered onchain with Nautilus attestation
- [ ] Smart contract verifies TEE signatures on swap results
- [ ] Backend executes real Cetus swap via TEE
- [ ] End-to-end flow works: deposit → swap → withdraw

### Nice to Have
- [ ] TEE status badge in UI (optional)
- [ ] Deployment documentation complete
- [ ] Load testing results

---

## 🚀 Quick Start Commands

### Nikola
```bash
# Frontend
cd frontend && npm run dev

# Contract
cd contracts/mist_protocol && sui move test
```

### Max
```bash
# Backend
cd backend && cargo run --release
curl localhost:3001/health
```

### Hung
```bash
# AWS
ssh -i ~/.ssh/mist-protocol.pem ec2-user@<instance-ip>
nitro-cli describe-enclaves
```

---

## 📚 Resources

### Documentation
- **Sui Move:** https://docs.sui.io/build/move
- **Cetus SDK:** https://cetus-1.gitbook.io/cetus-developer-docs
- **AWS Nitro:** https://docs.aws.amazon.com/enclaves/latest/user/nitro-enclave.html
- **SEAL:** https://github.com/Phala-Network/seal-docs

### Code References
- SEAL Integration: `frontend/lib/seal-vault.ts`
- Cetus PTB: `cetus-swap/backend/src/transaction.rs`
- Intent Processor: `backend/src/apps/mist-protocol/intent_processor.rs`

### Testnet
- Sui Faucet: https://faucet.testnet.sui.io/
- Cetus Testnet: https://app.cetus.zone/?network=testnet

---

## 📝 Notes

- Keep it simple - focus on getting stories done
- Test integration points early
- Ask for help in team chat when blocked
- Update checkboxes as you complete tasks

### How Nautilus Works

**Nautilus provides verifiable offchain computation for Sui:**

1. **Registration Phase** (Story 3.3 + 1.2):
   - TEE enclave generates AWS Nitro attestation with PCR measurements
   - Backend submits attestation to `register_tee_enclave()` Move function
   - Smart contract verifies AWS signature + PCR values onchain
   - Contract stores TEE public key for future verification

2. **Runtime Phase** (Story 2.3):
   - TEE executes swap offchain in secure enclave
   - TEE signs swap result with registered private key
   - Backend submits signed result to `complete_swap()` Move function
   - Contract verifies signature using stored public key
   - If valid → accept swap result and create output ticket

**Key Benefit:** Users don't need to verify attestations manually - the Move smart contract does it onchain. This provides transparency without requiring user interaction.

**Resources:**
- [Nautilus on Sui](https://docs.sui.io/concepts/cryptography/nautilus)
- [Nautilus Design](https://docs.sui.io/concepts/cryptography/nautilus/nautilus-design)
- [Using Nautilus](https://docs.sui.io/concepts/cryptography/nautilus/using-nautilus)
- [Nautilus Blog Post](https://blog.sui.io/nautilus-offchain-security-privacy-web3/)

---

**Version:** 2.1 (Story Format with Nautilus)
**Last Updated:** 2026-01-15

## Sources

- [Nautilus | Sui Documentation](https://docs.sui.io/concepts/cryptography/nautilus)
- [Introducing Nautilus: Bringing Verifiable Offchain Privacy to Sui](https://blog.sui.io/nautilus-offchain-security-privacy-web3/)
- [Build Tamper-Proof Oracles with Nautilus on Sui Mainnet](https://blog.sui.io/nautilus-tamper-proof-oracles/)
- [Nautilus Design | Sui Documentation](https://docs.sui.io/concepts/cryptography/nautilus/nautilus-design)
- [Scaling Confidential Compute on Sui: Nautilus and Marlin Oyster Integration](https://blog.marlin.org/scaling-confidential-compute-on-sui-nautilus-and-marlin-oyster-integration)
