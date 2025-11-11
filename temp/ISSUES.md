# GitHub Issues to Create

Copy these into GitHub manually or update token permissions.

---

## Issue #1: [Nikola] Nautilus + TEE Setup with Intent Wallet

**Labels:** `nautilus`, `tee`, `nikola`

### Description
Set up Nautilus TEE for intent-based asset management and wallet creation.

### Tasks
- [ ] Deploy Nautilus TEE enclave on AWS Nitro
- [ ] Implement intent parsing (assets → wallet logic)
- [ ] Create wallet generation endpoint
- [ ] Test attestation verification
- [ ] Document PCR values for reproducibility

### Flow
```
Intent → Nautilus TEE → Assets → Wallet
```

### Integration Point
Output: Wallet ready for DEX input (Step 2)

### Track
**Nikola** - Nautilus + Infrastructure

### Priority
HIGH - Foundational component

### Estimated Time
6-8 hours

---

## Issue #2: [Nikola] DEX Integration - Cetus SWAP

**Labels:** `dex`, `cetus`, `nikola`

### Description
Integrate with Cetus DEX for swap functionality (USDC/SUI, CETUS/WMNT/FlowX).

### Tasks
- [ ] Research Cetus DEX SDK/API
- [ ] Implement swap function (USDC/SUI, CETUS/WMNT/FlowX)
- [ ] Handle slippage and price quotes
- [ ] Test swap transactions on testnet
- [ ] Error handling for failed swaps

### Integration
Input: Wallet from Nautilus (Step 1)
Output: Swap data for Intent creation (Step 4)

### Track
**Nikola** - DEX Integration

### Priority
HIGH

### Estimated Time
4-6 hours

### Depends On
Issue #1 (Nautilus setup)

---

## Issue #3: [Max] Escrow Contract - Encrypted Amount Storage

**Labels:** `contracts`, `move`, `escrow`, `max`

### Description
Create Sui Move escrow contract that accepts deposits and returns encrypted amount (eUSDC).

### Tasks
- [ ] Write Move contract for escrow deposit
- [ ] Implement deposit function (USDC → eUSDC)
- [ ] Add encryption for amount storage
- [ ] Deploy to Sui testnet
- [ ] Write unit tests
- [ ] Document contract address

### Contract Logic
```move
deposit() → return encrypted_amount (eUSDC)
```

### Track
**Max** - Smart Contracts

### Priority
HIGH - Core contract

### Estimated Time
4-6 hours

---

## Issue #4: [Max] Intent Creation - DEX Interaction

**Labels:** `backend`, `intent`, `max`

### Description
Create intent system to interact with DEX for swaps.

### Tasks
- [ ] Design intent data structure
- [ ] Implement intent creation logic
- [ ] Connect to DEX swap endpoints
- [ ] Handle intent signing
- [ ] Test intent execution flow

### Flow
```
Escrow → Intent → DEX Interaction
```

### Integration
Input: Encrypted amount from Escrow (Step 3)
Output: Intent for Backend (Step 4)

### Track
**Max** - Backend/Frontend

### Priority
HIGH

### Estimated Time
4 hours

### Depends On
Issue #3 (Escrow contract)

---

## Issue #5: [Max] Backend - Intent Processing (No TEE Yet)

**Labels:** `backend`, `intent`, `max`

### Description
Build backend to listen for intent events, execute transactions with decrypted amounts, and provide decryption for users.

### Tasks
- [ ] Set up Node.js backend with Express
- [ ] Listen for intent events from DEX
- [ ] Implement transaction execution with decrypted amount
- [ ] Create decryption endpoint for users
- [ ] Add event logging and monitoring
- [ ] Test end-to-end flow

### Functionality
- **Listen**: Intent events from DEX
- **Execute**: Transactions with decrypted amount
- **Provide**: Decryption service for users

### Note
Initial implementation without TEE - will add Nautilus integration later

### Track
**Max** - Backend

### Priority
HIGH

### Estimated Time
6 hours

### Depends On
Issue #4 (Intent creation)

---

## Issue #6: [Max] Walrus Integration - Data Access Layer

**Labels:** `walrus`, `storage`, `max`

### Description
Integrate Walrus for storing encrypted transaction metadata and intent data.

### Tasks
- [ ] Set up Walrus SDK
- [ ] Implement upload function for encrypted data
- [ ] Implement retrieval function
- [ ] Store intent metadata on Walrus
- [ ] Reference Walrus blob IDs in contracts
- [ ] Test upload/download flow

### Use Cases
- Store encrypted transaction metadata
- Store intent execution logs
- Cost-efficient data access layer

### Track
**Max** - Storage Integration

### Priority
MEDIUM

### Estimated Time
3-4 hours

### Depends On
Issue #5 (Backend setup)

---

## Issue #7: [Integration] End-to-End Flow Testing

**Labels:** `integration`, `testing`

### Description
Test complete flow from Nautilus intent through DEX swap to backend execution.

### Flow to Test
```
Nikola: Intent → Nautilus TEE → Wallet → DEX Input
Max: Escrow → Intent → Backend → Execute Tx → Walrus Storage
```

### Integration Points
1. Nautilus wallet → DEX input (Nikola → Max handoff)
2. Escrow output → Intent creation (Max internal)
3. Backend → Walrus storage (Max internal)

### Test Scenarios
- [ ] Full happy path (intent → swap → execution)
- [ ] Failed swap handling
- [ ] Decryption endpoint works
- [ ] Walrus storage retrieval
- [ ] Event monitoring works

### Track
**Both** - Team Integration

### Priority
HIGH

### Estimated Time
4 hours

### Depends On
All previous issues

---

## Summary

**Nikola's Track (10-14 hours):**
1. Nautilus + TEE Setup (6-8h)
2. Cetus DEX Integration (4-6h)

**Max's Track (17-20 hours):**
3. Escrow Contract (4-6h)
4. Intent Creation (4h)
5. Backend Processing (6h)
6. Walrus Integration (3-4h)

**Integration (4 hours):**
7. End-to-End Testing

**Total:** ~31-38 hours of work
