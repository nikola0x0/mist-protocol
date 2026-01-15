# [Sprint] Max - Backend & Cetus Integration

**Owner:** @max
**Epic:** Unified Backend + Real Swap Execution

---

## 🎯 Goals

1. Consolidate both Axum servers into one unified backend
2. Integrate real Cetus DEX swaps (replace mock)
3. Support multi-ticket decryption for sharded deposits
4. Remove `tx-signer` service (sign natively in TEE)

---

## 📋 Tasks

### Story 2.1: Cetus Module Migration

**Goal:** Move existing Cetus code into main backend structure

- [ ] Create directory: `backend/src/apps/mist-protocol/cetus/`
- [ ] Move files from `cetus-swap/backend/src/`
- [ ] Create module structure:
  - `mod.rs` - exports CetusClient
  - `api.rs` - pool fetching from Cetus
  - `transaction.rs` - PTB building for swaps
  - `types.rs` - CetusPool, SwapQuote structs
- [ ] Implement `CetusClient` with methods:
  - `get_best_pool(token_in, token_out)`
  - `get_quote(pool, amount_in, min_out)`
  - `build_swap_ptb(pool, amount_in)`

**Success:** Cetus module compiles, can fetch pools, PTB building works

---

### Story 2.2: Unified Axum Router

**Goal:** Merge both Axum servers (port 3001 + 4001) into single server

- [ ] Merge routes in `backend/src/main.rs`
- [ ] Extend `AppState` with `CetusClient` and pool cache
- [ ] Add new routes:
  - `GET /api/pools` - List Cetus pools
  - `GET /api/pool/:id` - Get pool info
  - `POST /api/quote` - Get swap quote
- [ ] Keep existing routes:
  - `GET /health`
  - `GET /attestation`
  - `POST /api/seal/test`
- [ ] Update `Cargo.toml` with Cetus dependencies
- [ ] Update `.env.example` with Cetus config
- [ ] Single port: 3001

**Files:**
- `backend/src/main.rs`
- `backend/src/lib.rs`
- `backend/Cargo.toml`
- `backend/.env.example`

**Success:**
- `cargo run` starts server on 3001
- `curl localhost:3001/health` works
- `curl localhost:3001/api/pools` returns Cetus pools

---

### Story 2.3: Real Swap Execution via Cetus

**Goal:** Replace mock swaps with real Cetus DEX integration

- [ ] Implement `execute_real_swap()` in `swap_executor.rs`
- [ ] Build complete PTB:
  1. Borrow from Mist liquidity pool
  2. Execute Cetus swap
  3. Return output + create encrypted ticket
- [ ] Add error handling for failed swaps
- [ ] Add transaction logging
- [ ] Test SUI → USDC swap on testnet
- [ ] Remove all mock swap code

**Files:** `backend/src/apps/mist-protocol/swap_executor.rs`

**Success:**
- Intent creates → backend swaps via Cetus → output ticket returned
- Transaction visible on Sui explorer
- No mock code remaining

---

### Story 2.4: Multi-Ticket Decryption Support

**Goal:** Decrypt and sum multiple ticket shards from Nikola's frontend

- [ ] Implement `decrypt_and_sum_tickets()` in `intent_processor.rs`
- [ ] Loop through all tickets and decrypt with SEAL
- [ ] Sum all decrypted amounts
- [ ] Validate sum matches expected total
- [ ] Integration test with Nikola's frontend

**Files:** `backend/src/apps/mist-protocol/intent_processor.rs`

**Success:** Frontend sends 5 shards → backend decrypts → sum = 100

**Coordination:** With Nikola on encrypted shard format

---

### Story 3.3 (Shared with Hung): Native Transaction Signing

**Goal:** Remove `tx-signer` service, sign natively in TEE with Rust

- [ ] Add `sui-types` dependency to `Cargo.toml`
- [ ] Implement `generate_enclave_keypair()` in `common.rs`
- [ ] Replace `tx-signer` HTTP calls with direct signing:
  - Use `Signature::new_secure()` from `sui-types`
- [ ] Implement `register_enclave()` to submit attestation to contract
- [ ] Remove `tx-signer` dependency from codebase
- [ ] Remove `reqwest` for signing (keep for other HTTP)

**Files:**
- `backend/src/common.rs`
- `backend/src/apps/mist-protocol/swap_executor.rs`
- `backend/Cargo.toml`

**Files to Remove:**
- `tx-signer/` directory (entire service)

**Success:**
- Backend signs transactions natively (no CLI)
- `tx-signer` service removed
- Faster signing (no HTTP roundtrip)

**Coordination:** With Hung on keypair generation in enclave

---

## 🔗 Coordination Points

### With Nikola:
- **SEAL Shard Format** - After Story 2.4
  - Test decryption with Nikola's encrypted shards
  - Verify sum calculation correct
  - Run integration test

### With Hung:
- **Backend Binary** - After Story 2.2
  - Ensure backend builds in release mode
  - Document required environment variables
  - Hung packages into Docker image

- **Keypair Generation** - Story 3.3
  - Coordinate on enclave keypair initialization
  - Test signing works inside enclave

---

## ✅ Definition of Done

- [ ] Single Axum server running on port 3001 with all routes
- [ ] `/api/pools` returns real Cetus pools with liquidity data
- [ ] Intent processor executes real swap via Cetus
- [ ] Encrypted output ticket returned to user
- [ ] Backend decrypts and sums multiple tickets correctly
- [ ] No mock swap code remaining
- [ ] `tx-signer` service removed
- [ ] All integration tests passing

---

## 📚 Files to Modify

- `backend/src/main.rs`
- `backend/src/lib.rs`
- `backend/src/common.rs`
- `backend/src/apps/mist-protocol/swap_executor.rs`
- `backend/src/apps/mist-protocol/intent_processor.rs`
- `backend/src/apps/mist-protocol/cetus/` (new directory)
- `backend/Cargo.toml`
- `backend/.env.example`

## 📚 Files to Remove

- `tx-signer/` (entire directory)

---

**Estimated Complexity:** Medium
**Can Start:** Immediately (no blockers)
