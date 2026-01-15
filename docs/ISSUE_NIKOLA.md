# [Sprint] Nikola - Frontend & Smart Contract (Ticket Sharding + Nautilus)

**Owner:** @nikola0x0
**Epic:** Enhanced Privacy Through Ticket Sharding + TEE Attestation

---

## 🎯 Goals

1. Implement frontend shard generation for deposit obfuscation
2. Add smart contract support for multi-ticket deposits
3. Implement Nautilus attestation verification in Move contract
4. Support multi-ticket swap intents

---

## 📋 Tasks

### Story 1.1: Frontend Shard Generation

**Goal:** Generate multiple encrypted shards from single deposit for privacy

- [ ] Implement `generateShards()` function in `frontend/lib/seal-vault.ts`
- [ ] Add `encryptShards()` for batch SEAL encryption
- [ ] Update `WrapCard.tsx` with shard configuration UI (slider: 5-10 shards)
- [ ] Add visual feedback showing shard count
- [ ] Write tests for shard generation (verify sum, random distribution)

**Success:** User deposits 100 SUI → gets 5-10 encrypted shards → tests pass

---

### Story 1.2: Smart Contract - Sharded Deposits + Nautilus Verification

**Goal:** Accept multiple encrypted amounts and verify TEE attestations onchain

- [ ] Create `wrap_sui_sharded()` accepting `vector<vector<u8>>`
- [ ] Add `register_tee_enclave()` for Nautilus attestation verification
  - Store TEE public key + PCR values
  - Verify AWS attestation signature
- [ ] Add `complete_swap()` with TEE signature verification
- [ ] Update `create_swap_intent_optimized()` for multiple ticket IDs
- [ ] Add `DepositShardedEvent`
- [ ] Write Move unit tests

**Files:** `contracts/mist_protocol/sources/mist_protocol.move`

**Success:**
- TEE enclave registered onchain
- User deposits with 5 shards in one tx
- Swap results verified via TEE signature

**Resources:**
- [Nautilus Documentation](https://docs.sui.io/concepts/cryptography/nautilus)
- [Nautilus Design](https://docs.sui.io/concepts/cryptography/nautilus/nautilus-design)

---

### Story 1.3: Multi-Ticket Intent Support

**Goal:** Allow users to select multiple tickets for swap intents

- [ ] Update swap intent UI for multi-ticket selection
- [ ] Show total amount of selected tickets
- [ ] Pass ticket IDs array to contract
- [ ] Integration test with Max's backend decryption

**Success:** User selects 3 tickets → sees total → creates intent → backend processes

---

### Story 3.5: Display TEE Status [OPTIONAL]

**Goal:** Show users that swaps are secured by TEE

- [ ] Display "🔒 Secured by TEE" badge
- [ ] Show TEE wallet address from contract
- [ ] Link to Sui Explorer for TEE registration tx

**Files:** `frontend/components/TEEBadge.tsx` (optional)

**Success:** UI shows TEE badge (optional feature)

**Note:** This is optional - Nautilus verification happens onchain automatically

---

## 🔗 Coordination Points

### With Max:
- **SEAL Shard Format** - After Story 1.1 complete
  - Provide test encrypted shards for backend testing
  - Verify backend can decrypt and sum correctly
  - Run integration test with 5+ shards

### With Hung:
- **TEE Registration** - After Story 1.2 complete
  - Provide `register_tee_enclave()` Move function signature
  - Test registration transaction with Hung's attestation
  - Verify TEE public key stored onchain correctly

---

## ✅ Definition of Done

- [ ] User can deposit 100 SUI and get 5-10 encrypted shards
- [ ] Swap intent UI allows selecting multiple tickets
- [ ] Contract deployed on testnet with sharding support
- [ ] TEE enclave registered onchain with Nautilus attestation
- [ ] All Move tests pass
- [ ] Integration test passes with Max's backend

---

## 📚 Files to Modify

- `frontend/lib/seal-vault.ts`
- `frontend/components/WrapCard.tsx`
- `contracts/mist_protocol/sources/mist_protocol.move`
- Frontend swap intent components

---

**Estimated Complexity:** Medium
**Can Start:** Immediately (no blockers)
