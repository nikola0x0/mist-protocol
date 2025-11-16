# Backend Organization - Parallel Development

## Directory Structure

```
mist-protocol/
├── backend/              # Original (backup/reference)
├── backend-seal/         # Task 1: SEAL Decryption
├── backend-cetus/        # Task 2: Cetus Swap Execution
└── BACKEND_ORGANIZATION.md
```

---

## Backend Split Strategy

### Why Two Backends?

1. **Parallel Development:** Two developers work independently
2. **Clear Separation:** SEAL logic vs Cetus logic
3. **Easier Testing:** Test each component separately
4. **Merge Later:** Combine when both complete

---

## Backend-SEAL (Port 3000)

**Developer:** SEAL Integration Developer
**Focus:** Decrypt encrypted intents using SEAL threshold encryption

### Responsibilities
- Integrate SEAL SDK
- Implement `seal_approve` transaction
- Connect to SEAL key servers (2-of-3)
- Decrypt intents inside TEE

### Key Changes
- `src/apps/mist-protocol/seal_integration.rs` (new)
- `src/apps/mist-protocol/mod.rs` (update decryption)
- `seal_config.yaml` (new)

### Testing
- Mock encrypted data → Real decryption
- Coordinate with frontend for real encrypted intents

### Dependencies
- seal-sdk
- sui-sdk-types
- sui-crypto

---

## Backend-Cetus (Port 3001)

**Developer:** Cetus Integration Developer
**Focus:** Execute real swaps on Cetus DEX with TEE wallet

### Responsibilities
- Create TEE wallet management
- Integrate Cetus API
- Build swap transactions
- Execute on-chain

### Key Changes
- `src/apps/mist-protocol/wallet.rs` (new)
- `src/apps/mist-protocol/cetus.rs` (update)
- `src/apps/mist-protocol/mod.rs` (use real wallet)
- `cetus_config.yaml` (new)

### Testing
- Fund wallet on testnet
- Test swaps with mock intent data
- Verify transactions on explorer

### Dependencies
- sui-sdk
- sui-json-rpc-types
- reqwest (for Cetus API)

---

## Development Workflow

### Phase 1: Independent Development (Parallel)

**SEAL Developer:**
```bash
cd backend-seal
cargo build
PORT=3000 cargo run --bin nautilus-server

# Test decryption
curl -X POST http://localhost:3000/process_data ...
```

**Cetus Developer:**
```bash
cd backend-cetus
cargo build
PORT=3001 cargo run --bin nautilus-server

# Test swap execution
curl -X POST http://localhost:3001/process_data ...
```

### Phase 2: Integration Testing

**Test Together:**
1. SEAL developer decrypts real frontend data
2. Passes decrypted SwapIntent to Cetus developer
3. Cetus developer executes swap
4. Verify end-to-end flow

### Phase 3: Merge

Once both work independently:
```bash
# Merge into single backend
cd backend-seal
cp ../backend-cetus/src/apps/mist-protocol/wallet.rs src/apps/mist-protocol/
cp ../backend-cetus/src/apps/mist-protocol/cetus.rs src/apps/mist-protocol/
cp ../backend-cetus/cetus_config.yaml .

# Update mod.rs to use both SEAL and Cetus
# Test full integration
```

---

## Port Assignments

| Backend | Port | Purpose |
|---------|------|---------|
| backend-seal | 3000 | SEAL decryption |
| backend-cetus | 3001 | Cetus swap execution |
| backend (final) | 3000 | Merged version |

---

## Data Flow Between Backends

### During Development (Separate)

**SEAL Backend Output:**
```json
{
  "decrypted_intent": {
    "token_in": "USDC",
    "token_out": "SUI",
    "amount": 100000000,
    "min_output": 100000000000,
    "deadline": 1700000000
  }
}
```

**Cetus Backend Input:**
Use this decrypted intent to test swap execution.

### After Merge (Integrated)

```
Request → SEAL Decrypt → Cetus Swap → Signed Response
```

---

## Shared Interfaces

Both backends must agree on:

### 1. SwapIntent Structure
```rust
pub struct SwapIntent {
    pub token_in: String,
    pub token_out: String,
    pub amount: u64,
    pub min_output: u64,
    pub deadline: u64,
}
```

### 2. SwapExecutionResult Structure
```rust
pub struct SwapExecutionResult {
    pub executed: bool,
    pub input_amount: u64,
    pub output_amount: u64,
    pub token_in: String,
    pub token_out: String,
    pub tx_hash: Option<String>,
    pub error: Option<String>,
}
```

### 3. ProcessIntentRequest
```rust
pub struct ProcessIntentRequest {
    pub intent_id: String,
    pub encrypted_data: String,
    pub key_id: String,
}
```

---

## Testing Strategy

### Independent Testing

**SEAL Backend:**
```bash
# Test with mock encrypted data
echo '{"payload":{"intent_id":"test","encrypted_data":"{...}","key_id":"test-key"}}' | \
  curl -X POST http://localhost:3000/process_data -d @-
```

**Cetus Backend:**
```bash
# Test with mock decrypted data (skip decryption)
MODE=mock PORT=3001 cargo run
```

### Integration Testing

**Step 1:** SEAL decrypts
```bash
curl -X POST http://localhost:3000/process_data \
  -d '{"payload":{"encrypted_data":"0xreal_from_frontend",...}}'

# Output: {"decrypted_intent": {...}}
```

**Step 2:** Cetus executes
```bash
# Use decrypted output as input
curl -X POST http://localhost:3001/process_data \
  -d '{"payload":{"encrypted_data":"<decrypted_json>",...}}'

# Output: {"result":{"tx_hash":"0x..."}}
```

---

## Coordination Points

### Daily Sync (Recommended)
- Share data structures
- Test with each other's sample data
- Coordinate interface changes

### Merge Checklist
- [ ] Both backends work independently
- [ ] Data structures match
- [ ] Mock mode works end-to-end
- [ ] Real mode tested separately
- [ ] Git conflicts resolved
- [ ] Tests passing

---

## Git Strategy

### Branch Setup
```bash
# SEAL Developer
git checkout -b feature/seal-integration
cd backend-seal
# work...
git add backend-seal/
git commit -m "feat(seal): implement SEAL decryption"

# Cetus Developer
git checkout -b feature/cetus-integration
cd backend-cetus
# work...
git add backend-cetus/
git commit -m "feat(cetus): implement Cetus swap execution"
```

### Merge Strategy
```bash
# After both features complete
git checkout main
git merge feature/seal-integration
git merge feature/cetus-integration

# Create unified backend
cd backend
# Copy components from both
# Test integration
git commit -m "feat: merge SEAL and Cetus backends"
```

---

## Troubleshooting

### Port Conflicts
```bash
# If port 3000/3001 already in use
lsof -i :3000
kill -9 <PID>

# Or change port
PORT=3002 cargo run
```

### Dependency Conflicts
```bash
# If seal-sdk and sui-sdk versions conflict
# Use same Sui revision in both
git = "https://github.com/MystenLabs/sui.git"
rev = "framework/testnet"  # Same for both!
```

### Data Format Mismatch
- Check `SwapIntent` serialization
- Use `serde_json` for compatibility
- Test with sample data exchange

---

## Timeline

**Week 1:** Independent Development
- SEAL: Implement decryption (6-8 hours)
- Cetus: Implement swaps (10-13 hours)

**Week 2:** Integration
- Test together (3-4 hours)
- Merge code (2-3 hours)
- End-to-end testing (3-4 hours)

**Total:** ~24-32 hours (with 2 developers in parallel)

---

## Success Criteria

### SEAL Backend Complete
- ✅ Decrypts real encrypted data from frontend
- ✅ SEAL server communication works
- ✅ Returns valid SwapIntent JSON

### Cetus Backend Complete
- ✅ TEE wallet executes swaps on testnet
- ✅ Transactions confirmed on-chain
- ✅ Returns transaction hash

### Integration Complete
- ✅ Encrypted intent → Decrypted → Executed → Signed result
- ✅ Both backends merged into one
- ✅ End-to-end test passes

---

**Last Updated:** 2025-11-15
**Status:** Ready for parallel development
