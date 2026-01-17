# Backend - SEAL Integration (Task 1)

This backend focuses on implementing **SEAL threshold encryption/decryption**.

## Purpose
Integrate with Mysten Labs SEAL servers to decrypt user intents inside the TEE.

## Developer Focus
**Task 1.2:** SEAL Backend Decryption

## Key Files to Modify

### 1. `src/apps/mist-protocol/seal_integration.rs` (CREATE NEW)
Implement SEAL decryption logic:
- ElGamal key generation
- `seal_approve` transaction building
- SEAL server communication (2-of-3 threshold)
- Decryption with combined shares

### 2. `src/apps/mist-protocol/mod.rs` (UPDATE)
Replace mock decryption with real SEAL:
```rust
// OLD: decrypt_with_seal_mock()
// NEW: decrypt_with_seal_real()
```

### 3. `seal_config.yaml` (CREATE NEW)
Add SEAL server configuration:
```yaml
key_servers:
  - "0x<server_1>"
  - "0x<server_2>"
  - "0x<server_3>"

public_keys:
  - "0x<pk1>"
  - "0x<pk2>"
  - "0x<pk3>"

package_id: "0x<seal_package>"
```

## Reference Implementation
Copy patterns from:
- `src/apps/seal-example/endpoints.rs`
- `src/apps/seal-example/types.rs`

## Dependencies to Add
```toml
[dependencies]
seal-sdk = { git = "https://github.com/MystenLabs/seal", rev = "latest" }
sui-sdk-types = { git = "https://github.com/mystenlabs/sui-rust-sdk", features = ["serde"] }
sui-crypto = { git = "https://github.com/mystenlabs/sui-rust-sdk", features = ["ed25519"] }
```

## Testing

### Local Testing (Mock Mode)
```bash
RUST_LOG=info cargo run --bin nautilus-server

# Test with mock encrypted data
curl -X POST http://localhost:3000/process_data \
  -H "Content-Type: application/json" \
  -d '{
    "payload": {
      "intent_id": "test-123",
      "encrypted_data": "0xdeadbeef...",
      "key_id": "test-key"
    }
  }'
```

### Real SEAL Testing
1. Get encrypted data from frontend (Task 1.1)
2. Verify `seal_approve` transaction succeeds
3. Check SEAL server responses
4. Verify decryption output

## Prerequisites Needed

### From Mysten Labs
- [ ] SEAL testnet server endpoints
- [ ] SEAL server public keys
- [ ] SEAL package ID on testnet
- [ ] Documentation/examples

### From Frontend Team
- [ ] Encrypted intent format
- [ ] Key ID format
- [ ] Test encrypted data samples

## Success Criteria
- ✅ ElGamal keys generated on startup
- ✅ `seal_approve` transaction builds and executes
- ✅ SEAL servers respond with encrypted shares
- ✅ Threshold decryption (2-of-3) works
- ✅ Decrypted SwapIntent is valid JSON
- ✅ End-to-end with frontend encryption

## Port Configuration
This backend runs on **port 3000** (default)

To change:
```bash
PORT=3001 cargo run --bin nautilus-server
```

## Coordination with Cetus Team
- Share decrypted `SwapIntent` format
- Ensure compatibility with swap execution
- Test with their mock data first

## Timeline
**Estimated:** 6-8 hours
- Setup SEAL SDK: 1-2 hours
- Implement decryption: 3-4 hours
- Testing & debugging: 2-3 hours

---

**Owner:** [SEAL Developer Name]
**Started:** [Date]
**Status:** Ready for implementation
