# Developer Guide - Quick Start

## 🎯 Two Parallel Tasks

### Developer 1: SEAL Integration
**Directory:** `backend-seal/`
**Port:** 3000
**Focus:** Decrypt encrypted intents with SEAL threshold encryption

### Developer 2: Cetus Integration
**Directory:** `backend-cetus/`
**Port:** 3001
**Focus:** Execute swaps on Cetus DEX with TEE wallet

---

## 🚀 Quick Start

### SEAL Developer (Task 1)

```bash
cd backend-seal

# 1. Read your README
cat README.md

# 2. Build
cargo build

# 3. Run
RUST_LOG=info cargo run --bin nautilus-server

# 4. Test endpoint
curl http://localhost:3000/health_check
```

**Next Steps:**
1. Review `src/apps/seal-example/` for reference
2. Create `src/apps/mist-protocol/seal_integration.rs`
3. Update `src/apps/mist-protocol/mod.rs` to use real SEAL
4. Get SEAL server config from Mysten Labs
5. Test with frontend encrypted data

---

### Cetus Developer (Task 2)

```bash
cd backend-cetus

# 1. Read your README
cat README.md

# 2. Build
cargo build

# 3. Run (different port!)
PORT=3001 RUST_LOG=info cargo run --bin nautilus-server

# 4. Test endpoint
curl http://localhost:3001/health_check
```

**Next Steps:**
1. Create `src/apps/mist-protocol/wallet.rs`
2. Generate and fund TEE wallet
3. Update `src/apps/mist-protocol/cetus.rs`
4. Get Cetus testnet config
5. Test swaps with mock data

---

## 📁 Key Files

### SEAL Backend (`backend-seal/`)
```
backend-seal/
├── src/apps/mist-protocol/
│   ├── mod.rs                    # Main endpoint (UPDATE)
│   ├── seal_integration.rs       # SEAL logic (CREATE)
│   └── types.rs                  # Already exists
├── seal_config.yaml              # SEAL servers (CREATE)
└── README.md                     # Your guide
```

### Cetus Backend (`backend-cetus/`)
```
backend-cetus/
├── src/apps/mist-protocol/
│   ├── mod.rs                    # Main endpoint (UPDATE)
│   ├── wallet.rs                 # Wallet mgmt (CREATE)
│   ├── cetus.rs                  # Already exists (UPDATE)
│   └── types.rs                  # Already exists
├── cetus_config.yaml             # Cetus config (CREATE)
└── README.md                     # Your guide
```

---

## 🔄 Data Flow

### Current (Both backends work independently)
```
SEAL Backend:
Encrypted Intent → [SEAL Decrypt] → SwapIntent JSON

Cetus Backend:
SwapIntent JSON → [Execute Swap] → Transaction Hash
```

### After Integration (Merge both)
```
Encrypted Intent → [SEAL] → [Cetus] → Signed Result
```

---

## 🧪 Testing

### SEAL Testing
```bash
# In backend-seal/
curl -X POST http://localhost:3000/process_data \
  -H "Content-Type: application/json" \
  -d '{
    "payload": {
      "intent_id": "test-123",
      "encrypted_data": "0x<from_frontend>",
      "key_id": "test-key"
    }
  }'

# Expected: Decrypted SwapIntent
```

### Cetus Testing
```bash
# In backend-cetus/

# 1. Check wallet address in logs
# 2. Fund it: curl https://faucet.testnet.sui.io/gas -d '{"recipient":"0x..."}'
# 3. Test swap
curl -X POST http://localhost:3001/process_data \
  -H "Content-Type: application/json" \
  -d '{
    "payload": {
      "intent_id": "swap-1",
      "encrypted_data": "{\"token_in\":\"SUI\",\"token_out\":\"USDC\",\"amount\":1000000000,\"min_output\":800000,\"deadline\":1700000000}",
      "key_id": "test"
    }
  }'

# Expected: Transaction hash
```

---

## 🤝 Coordination

### Share Data Structures
Both backends use the same types (already defined in `types.rs`):
- `SwapIntent`
- `SwapExecutionResult`
- `ProcessIntentRequest`

### Test Data Exchange
**SEAL Developer:** Share decrypted output → Cetus tests with it
**Cetus Developer:** Share swap results format → SEAL returns it

---

## 📋 Prerequisites Checklist

### SEAL Developer Needs:
- [ ] SEAL server endpoints (Mysten Labs)
- [ ] SEAL package ID on testnet
- [ ] SEAL server public keys
- [ ] Sample encrypted data from frontend

### Cetus Developer Needs:
- [ ] Cetus router package ID
- [ ] SUI/USDC pool ID
- [ ] Test SUI from faucet (5-10 SUI)
- [ ] Test USDC (ask in Discord or mock)

---

## 🐛 Common Issues

### Port Already in Use
```bash
lsof -i :3000  # or :3001
kill -9 <PID>
```

### Dependency Conflicts
Make sure both use same Sui revision:
```toml
# In Cargo.toml
git = "https://github.com/MystenLabs/sui.git"
rev = "framework/testnet"  # Keep same!
```

### Can't Build
```bash
rm -rf target/
cargo clean
cargo build
```

---

## 📚 Documentation

- **IMPLEMENTATION_TASKS.md** - Detailed task breakdown
- **BACKEND_ORGANIZATION.md** - Organization strategy
- **backend-seal/README.md** - SEAL specific guide
- **backend-cetus/README.md** - Cetus specific guide

---

## ✅ Success Criteria

### SEAL Backend Complete When:
- ✅ Decrypts real encrypted data from frontend
- ✅ SEAL servers respond correctly
- ✅ Returns valid SwapIntent

### Cetus Backend Complete When:
- ✅ TEE wallet funded and working
- ✅ Swaps execute on Cetus testnet
- ✅ Transaction confirmed on explorer

### Ready to Merge When:
- ✅ Both work independently
- ✅ Tested with each other's data
- ✅ Data formats match
- ✅ No conflicts

---

## 🚢 Deployment (After Merge)

When both backends are ready:
1. Merge code into `backend/`
2. Test full integration
3. Deploy to Nautilus TEE
4. Connect with frontend

---

## 💬 Communication

**Daily Sync:** Share progress, blockers, data formats
**Data Exchange:** Test with each other's output
**Integration:** Plan merge strategy together

---

**Questions?** Check your backend-specific README or ask in team chat!

**Last Updated:** 2025-11-15
