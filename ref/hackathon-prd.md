# PRD: Private DeFi Protocol on Sui (Hackathon Version)

## Project Overview

**Goal:** Build a working privacy-preserving DeFi protocol on Sui in 3-5 days that demonstrates the key advantages of the Sui stack (Nautilus + Seal + Walrus) over existing solutions like Encifher.

**Scope:** Hackathon MVP - Focus on core differentiators, cut non-essential features

**Team Size:** Assume 2-4 developers

**Timeline:** 3-5 days

---

## Executive Summary

### What We're Building

A privacy-preserving DeFi protocol on Sui that provides:

- **Stealth addresses** (recipient privacy - Encifher can't do this)
- **Threshold encryption** (Seal - true decentralization)
- **Verifiable TEE computation** (Nautilus - self-managed)
- **Cost-efficient storage** (Walrus - 20x cheaper)
  `

### Why This Wins

1. **Better privacy than Encifher** - Recipient hiding + threshold crypto
2. **Verifiable & decentralized** - No single point of failure
3. **Sui native** - Uses 3 major Sui components
4. **Practical & achievable** - Can build in 3-5 days

---

## Name: Mist Protocol

---

## Core Features (Must Have for Demo)

### 1. Private Payments with Stealth Addresses ⭐ KEY DIFFERENTIATOR

**Why:** Encifher can't do this - immediate competitive advantage

**User Flow:**

1. Alice generates stealth address for Bob
2. Alice sends encrypted payment to stealth address
3. Bob scans chain and discovers payment (only he can)
4. Bob decrypts and claims funds

**Technical Implementation:**

- Stealth address generation using ECDH (Elliptic Curve Diffie-Hellman)
- Encrypted amount with Seal (2-of-3 threshold)
- Scan key mechanism for recipient discovery
- Move contract for stealth payment storage

**Deliverable:** Working demo where payment recipient is hidden from blockchain observers

**Complexity:** Medium-High (cryptography required)

---

### 2. Seal Integration (Threshold Encryption) ⭐ KEY DIFFERENTIATOR

**Why:** Shows true decentralization vs Encifher's single gateway

**User Flow:**

1. User encrypts transaction data
2. Data encrypted with 2-of-3 threshold
3. User submits to chain (only handle stored)
4. Authorized party decrypts with session key

**Technical Implementation:**

- Seal SDK integration (@mysten/seal-sdk)
- 2-of-3 key server setup (use testnet servers)
- Session key management in frontend
- Basic Move access policy (owner-only or time-locked)

**Deliverable:** Encryption/decryption with threshold cryptography working end-to-end

**Complexity:** Medium (SDK available)

---

### 3. Nautilus TEE Computation ⭐ KEY DIFFERENTIATOR

**Why:** Self-managed verifiable computation vs black box

**User Flow:**

1. User submits encrypted transaction
2. Nautilus TEE processes in secure enclave
3. TEE returns signed result with attestation
4. Contract verifies attestation on-chain

**Technical Implementation:**

- Deploy simple Nautilus enclave on AWS Nitro
- Implement basic computation endpoint (Rust + Axum)
- Attestation verification in Move contract
- Reproducible build for demo transparency

**Deliverable:** Working TEE that proves correct execution with on-chain verification

**Complexity:** High (AWS setup, enclave deployment)

**Fallback:** Mock attestation if AWS setup fails

---

### 4. Walrus Storage Integration ⭐ KEY DIFFERENTIATOR

**Why:** Shows cost efficiency (20x cheaper than on-chain)

**User Flow:**

1. User encrypts large data (e.g., order details)
2. Store encrypted data on Walrus
3. Only store blob ID on-chain (32 bytes vs MB)
4. Retrieve and decrypt when needed

**Technical Implementation:**

- Walrus client integration
- Store encrypted data to Walrus testnet
- Reference blob IDs in Move contracts
- Fetch and decrypt on demand

**Deliverable:** Demo showing on-chain cost savings with side-by-side comparison

**Complexity:** Low-Medium (SDK available)

---

### 5. Basic Web UI

**Pages:**

1. **Home** - Explanation, features, connect wallet
2. **Send Private Payment** - Stealth address input, amount, send
3. **Receive Payments** - Scan for payments, decrypt, claim
4. **Transaction History** - View encrypted transactions
5. **Encrypt/Decrypt Demo** - Showcase Seal threshold encryption

**Tech Stack:**

- Next.js 14 + TypeScript
- @mysten/dapp-kit (Sui wallet integration)
- @mysten/seal-sdk (encryption)
- @mysten/wallet-standard
- Walrus SDK
- Tailwind CSS (styling)
- Framer Motion (animations)

**UI Components:**

- Wallet connection button
- Stealth address generator
- Encryption widget
- Payment scanner
- Transaction list
- Loading states
- Error handling

**Deliverable:** Clean, professional UI showcasing all features

**Complexity:** Medium

---

## Move Smart Contracts

### Contract 1: `stealth_payment.move`

**Purpose:** Handle stealth payments with hidden recipients

```rust
module privacy_protocol::stealth_payment {
    use sui::object::{Self, UID};
    use sui::tx_context::{Self, TxContext};
    use sui::transfer;
    use sui::coin::{Self, Coin};
    use sui::sui::SUI;

    /// Stealth payment record (shared object for scanning)
    struct StealthPayment has key {
        id: UID,
        ephemeral_pubkey: vector<u8>,  // ECDH public key for recipient discovery
        encrypted_amount: vector<u8>,  // Seal encrypted amount
        walrus_blob_id: vector<u8>,    // Additional metadata on Walrus
        timestamp: u64,
        claimed: bool,
    }

    /// Create stealth payment
    public entry fun send_stealth_payment(
        ephemeral_pubkey: vector<u8>,
        encrypted_amount: vector<u8>,
        walrus_blob_id: vector<u8>,
        payment: Coin<SUI>,  // Actual funds
        ctx: &mut TxContext
    ) {
        // TODO: Lock funds in contract
        // For hackathon: just store metadata

        let stealth_payment = StealthPayment {
            id: object::new(ctx),
            ephemeral_pubkey,
            encrypted_amount,
            walrus_blob_id,
            timestamp: tx_context::epoch(ctx),
            claimed: false,
        };

        // Share object so recipients can scan
        transfer::share_object(stealth_payment);

        // Transfer payment to contract or recipient
        transfer::public_transfer(payment, tx_context::sender(ctx));
    }

    /// Claim payment (only recipient can decrypt and prove ownership)
    public entry fun claim_payment(
        payment: &mut StealthPayment,
        proof: vector<u8>,  // Proof of ownership (simplified for hackathon)
        ctx: &mut TxContext
    ) {
        assert!(!payment.claimed, 0);

        // TODO: Verify proof of ownership
        // For hackathon: simplified verification

        payment.claimed = true;

        // TODO: Transfer actual funds to claimer
    }

    /// Scan function helper (returns payment if matches scan key)
    public fun check_payment(
        payment: &StealthPayment,
        scan_key_hash: vector<u8>,
    ): bool {
        // TODO: Check if ephemeral_pubkey matches scan key
        // For hackathon: placeholder
        true
    }
}
```

**Key Features:**

- Shared objects for scanning
- Ephemeral keys for recipient discovery
- Seal encrypted amounts
- Walrus references for metadata

---

### Contract 2: `nautilus_verifier.move`

**Purpose:** Verify TEE attestations from Nautilus enclaves

```rust
module privacy_protocol::nautilus_verifier {
    use sui::object::{Self, UID};
    use sui::tx_context::{Self, TxContext};
    use sui::transfer;

    /// Registered Nautilus enclave configuration
    struct EnclaveConfig has key {
        id: UID,
        pcr0: vector<u8>,  // Platform Configuration Register 0
        pcr1: vector<u8>,  // Platform Configuration Register 1
        pcr2: vector<u8>,  // Platform Configuration Register 2
        public_key: vector<u8>,  // Enclave signing key
        is_active: bool,
        registered_at: u64,
    }

    /// Computation result from TEE
    struct ComputationResult has key {
        id: UID,
        enclave_id: address,
        input_hash: vector<u8>,
        output: vector<u8>,
        attestation: vector<u8>,
        signature: vector<u8>,
        timestamp: u64,
    }

    /// Verify TEE attestation
    public fun verify_attestation(
        config: &EnclaveConfig,
        attestation: vector<u8>,
        signature: vector<u8>,
    ): bool {
        // Simplified verification for hackathon
        // TODO: Full attestation document parsing
        // TODO: Verify PCR values match
        // TODO: Verify signature with public key

        config.is_active
    }

    /// Register new enclave
    public entry fun register_enclave(
        pcr0: vector<u8>,
        pcr1: vector<u8>,
        pcr2: vector<u8>,
        public_key: vector<u8>,
        ctx: &mut TxContext
    ) {
        let config = EnclaveConfig {
            id: object::new(ctx),
            pcr0,
            pcr1,
            pcr2,
            public_key,
            is_active: true,
            registered_at: tx_context::epoch(ctx),
        };

        transfer::share_object(config);
    }

    /// Submit computation result with attestation
    public entry fun submit_result(
        enclave_id: address,
        input_hash: vector<u8>,
        output: vector<u8>,
        attestation: vector<u8>,
        signature: vector<u8>,
        ctx: &mut TxContext
    ) {
        // TODO: Verify attestation before accepting

        let result = ComputationResult {
            id: object::new(ctx),
            enclave_id,
            input_hash,
            output,
            attestation,
            signature,
            timestamp: tx_context::epoch(ctx),
        };

        transfer::share_object(result);
    }

    /// Deactivate compromised enclave
    public entry fun deactivate_enclave(
        config: &mut EnclaveConfig,
        ctx: &mut TxContext
    ) {
        // TODO: Add admin check
        config.is_active = false;
    }
}
```

**Key Features:**

- PCR value storage (reproducible builds)
- Attestation verification
- Enclave registry
- Computation result tracking

---

### Contract 3: `seal_policy.move`

**Purpose:** Access control policies for Seal decryption

```rust
module privacy_protocol::seal_policy {
    use sui::tx_context::{Self, TxContext};
    use sui::clock::{Self, Clock};
    use sui::vec_map::{Self, VecMap};

    /// Allowlist for access control
    struct Allowlist has key {
        id: UID,
        members: VecMap<address, bool>,
        owner: address,
    }

    /// Time-locked access policy
    /// ID format: [package_id][bcs_encoded_unlock_time]
    public entry fun seal_approve_timelock(
        id: vector<u8>,
        clock: &Clock,
        _ctx: &TxContext
    ) {
        // Extract timestamp from ID
        // For hackathon: simplified parsing
        let len = vector::length(&id);
        assert!(len >= 8, 0);

        // TODO: Proper BCS parsing
        // Placeholder: assume last 8 bytes are timestamp in ms
        let unlock_time = 0u64; // Parse from id
        let current_time = clock::timestamp_ms(clock);

        assert!(current_time >= unlock_time, 1);
    }

    /// Owner-only access policy
    public entry fun seal_approve_owner(
        _id: vector<u8>,
        expected_owner: address,
        ctx: &TxContext
    ) {
        let sender = tx_context::sender(ctx);
        assert!(sender == expected_owner, 0);
    }

    /// Allowlist-based access policy
    public entry fun seal_approve_allowlist(
        _id: vector<u8>,
        allowlist: &Allowlist,
        ctx: &TxContext
    ) {
        let sender = tx_context::sender(ctx);
        assert!(vec_map::contains(&allowlist.members, &sender), 0);
    }

    /// Create new allowlist
    public entry fun create_allowlist(
        ctx: &mut TxContext
    ) {
        let allowlist = Allowlist {
            id: object::new(ctx),
            members: vec_map::empty(),
            owner: tx_context::sender(ctx),
        };

        transfer::share_object(allowlist);
    }

    /// Add member to allowlist
    public entry fun add_member(
        allowlist: &mut Allowlist,
        member: address,
        ctx: &TxContext
    ) {
        assert!(tx_context::sender(ctx) == allowlist.owner, 0);
        vec_map::insert(&mut allowlist.members, member, true);
    }

    /// Remove member from allowlist
    public entry fun remove_member(
        allowlist: &mut Allowlist,
        member: address,
        ctx: &TxContext
    ) {
        assert!(tx_context::sender(ctx) == allowlist.owner, 0);
        vec_map::remove(&mut allowlist.members, &member);
    }
}
```

**Key Features:**

- Multiple policy types (timelock, owner, allowlist)
- Composable access control
- Transparent on-chain logic

---

## Architecture Diagram

```
┌─────────────────────────────────────────────────────────┐
│                Frontend (Next.js + TypeScript)           │
│                                                          │
│  Components:                                            │
│  - Wallet connection (@mysten/dapp-kit)                │
│  - Stealth address generation (ECDH)                   │
│  - Seal SDK integration                                │
│  - Payment scanner                                     │
│  - Transaction history                                 │
└────────────────────┬────────────────────────────────────┘
                     │
        ┌────────────┼────────────────┐
        │            │                │
┌───────▼──────┐ ┌──▼─────────┐ ┌───▼────────┐
│  Seal KMS    │ │ Nautilus   │ │  Walrus    │
│  (Testnet)   │ │    TEE     │ │  Storage   │
│              │ │            │ │            │
│ - Server 1   │ │ AWS Nitro  │ │ - Testnet  │
│ - Server 2   │ │ Enclave    │ │ - Blob IDs │
│ - Server 3   │ │            │ │ - Erasure  │
│              │ │ - Attest   │ │   coding   │
│ Threshold:   │ │ - Compute  │ │            │
│   2-of-3     │ │ - Sign     │ │            │
└───────┬──────┘ └──┬─────────┘ └───┬────────┘
        │           │                │
        └───────────┼────────────────┘
                    │
        ┌───────────▼───────────────────┐
        │   Sui Blockchain (Testnet)    │
        │                               │
        │  Smart Contracts:             │
        │  - stealth_payment.move       │
        │  - nautilus_verifier.move     │
        │  - seal_policy.move           │
        │                               │
        │  Objects:                     │
        │  - StealthPayment (shared)    │
        │  - EnclaveConfig (shared)     │
        │  - Allowlist (shared)         │
        └───────────────────────────────┘
```

### Data Flow

**Private Payment Flow:**

```
1. Alice → Frontend: Enter Bob's scan key + amount
2. Frontend → Stealth: Generate ephemeral address
3. Frontend → Seal: Encrypt amount (2-of-3 threshold)
4. Frontend → Walrus: Store metadata → Get blob ID
5. Frontend → Sui: Call send_stealth_payment(ephemeral, encrypted, blobId)
6. Sui → Blockchain: Store StealthPayment object (shared)
7. Bob → Frontend: Click "Scan for Payments"
8. Frontend → Sui: Fetch all StealthPayment objects
9. Frontend → Stealth: Check each with Bob's scan key
10. Frontend → Seal: Decrypt matched payments
11. Bob → Frontend: Click "Claim"
12. Frontend → Sui: Call claim_payment()
```

---

## Tech Stack

### Frontend Dependencies

```json
{
  "name": "privacy-protocol-sui",
  "version": "0.1.0",
  "dependencies": {
    "next": "14.2.0",
    "react": "18.3.0",
    "react-dom": "18.3.0",
    "@mysten/sui.js": "^0.54.0",
    "@mysten/dapp-kit": "^0.14.0",
    "@mysten/seal-sdk": "latest",
    "@mysten/wallet-standard": "^0.12.0",
    "@noble/curves": "^1.3.0",
    "tailwindcss": "^3.4.0",
    "framer-motion": "^11.0.0",
    "lucide-react": "^0.344.0",
    "sonner": "^1.4.0"
  },
  "devDependencies": {
    "@types/node": "^20",
    "@types/react": "^18",
    "typescript": "^5",
    "eslint": "^8",
    "eslint-config-next": "14.2.0"
  }
}
```

### Backend (Nautilus) Dependencies

```toml
[package]
name = "nautilus-privacy"
version = "0.1.0"
edition = "2021"

[dependencies]
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
tokio = { version = "1.35", features = ["full"] }
axum = "0.7"
tower = "0.4"
tower-http = { version = "0.5", features = ["cors"] }
tracing = "0.1"
tracing-subscriber = "0.3"
```

### Move Dependencies

```toml
[package]
name = "privacy_protocol"
version = "0.1.0"
edition = "2024.beta"

[dependencies]
Sui = { git = "https://github.com/MystenLabs/sui.git", subdir = "crates/sui-framework/packages/sui-framework", rev = "framework/testnet" }

[addresses]
privacy_protocol = "0x0"
```

---

## Day-by-Day Implementation Plan

### **Day 1: Setup & Infrastructure** (8 hours)

**Morning (4 hours):**

- [ ] Set up Next.js project with TypeScript
  ```bash
  npx create-next-app@latest privacy-protocol-sui --typescript --tailwind --app
  ```
- [ ] Install Sui dependencies
  ```bash
  npm install @mysten/sui.js @mysten/dapp-kit @mysten/seal-sdk
  ```
- [ ] Set up Sui Move project
  ```bash
  sui move new privacy_protocol
  ```
- [ ] Deploy skeleton Move contracts to testnet
- [ ] Set up AWS account for Nautilus (optional)
- [ ] Configure Walrus testnet access

**Afternoon (4 hours):**

- [ ] Implement wallet connection UI component
- [ ] Test basic Sui transaction signing
- [ ] Set up environment variables
  ```env
  NEXT_PUBLIC_SUI_NETWORK=testnet
  NEXT_PUBLIC_PACKAGE_ID=0x...
  NEXT_PUBLIC_WALRUS_RPC=...
  ```
- [ ] Deploy simple "hello world" Nautilus enclave (if doing TEE)
- [ ] Verify Seal testnet servers are accessible
  ```typescript
  const servers = [
    "0x73d05d62c18d9374e3ea529e8e0ed6161da1a141...",
    "0xf5d14a81a982144ae441cd7d64b09027f116a468...",
  ];
  ```

**End of Day 1 Deliverable:**

- ✅ Project skeleton with wallet integration working
- ✅ Move contracts deployed to testnet
- ✅ Development environment configured
- ✅ Team can make test transactions

---

### **Day 2: Seal Integration** (8 hours)

**Morning (4 hours):**

- [ ] Integrate Seal SDK in frontend
  ```typescript
  import { SealClient, SessionKey } from "@mysten/seal-sdk";
  ```
- [ ] Implement encryption function
  ```typescript
  async function encryptData(data: Uint8Array) {
    const { encryptedObject, key } = await client.encrypt({
      threshold: 2,
      packageId: PACKAGE_ID,
      id: generateId(),
      data,
    });
    return { encryptedObject, backupKey: key };
  }
  ```
- [ ] Deploy `seal_policy.move` contract
- [ ] Test 2-of-3 threshold encryption with testnet servers

**Afternoon (4 hours):**

- [ ] Create session key management
  ```typescript
  const sessionKey = await SessionKey.create({
    address: userAddress,
    packageId: PACKAGE_ID,
    ttlMin: 10,
    suiClient,
  });
  ```
- [ ] Implement decryption function

  ```typescript
  async function decryptData(encrypted: Uint8Array) {
    const tx = new Transaction();
    tx.moveCall({
      target: `${PACKAGE_ID}::seal_policy::seal_approve_owner`,
      arguments: [
        /* ... */
      ],
    });

    return await client.decrypt({
      data: encrypted,
      sessionKey,
      txBytes: await tx.build({ client: suiClient }),
    });
  }
  ```

- [ ] Build UI for encrypting/decrypting data
- [ ] Add error handling and loading states
- [ ] Test end-to-end encryption/decryption

**End of Day 2 Deliverable:**

- ✅ Working threshold encryption demo
- ✅ Session key management
- ✅ UI for encryption/decryption
- ✅ Seal policies deployed and tested

---

### **Day 3: Stealth Addresses** (8 hours) ⭐ CRITICAL

**Morning (4 hours):**

- [ ] Implement stealth address cryptography

  ```typescript
  import { secp256k1 } from "@noble/curves/secp256k1";

  // Generate ephemeral key pair
  function generateEphemeralKey() {
    const privateKey = secp256k1.utils.randomPrivateKey();
    const publicKey = secp256k1.getPublicKey(privateKey);
    return { privateKey, publicKey };
  }

  // Compute stealth address
  function computeStealthAddress(
    recipientScanKey: Uint8Array,
    ephemeralPrivateKey: Uint8Array
  ) {
    const sharedSecret = secp256k1.getSharedSecret(
      ephemeralPrivateKey,
      recipientScanKey
    );
    // Derive stealth address from shared secret
    return deriveAddress(sharedSecret);
  }
  ```

- [ ] Deploy `stealth_payment.move` contract
- [ ] Build "Generate Stealth Address" UI component
- [ ] Test stealth address generation

**Afternoon (4 hours):**

- [ ] Implement payment scanning logic

  ```typescript
  async function scanForPayments(
    scanPrivateKey: Uint8Array
  ): Promise<StealthPayment[]> {
    // Fetch all StealthPayment objects
    const payments = await suiClient.getObjects({
      /* filter */
    });

    // Check each payment
    const myPayments = [];
    for (const payment of payments) {
      const sharedSecret = computeSharedSecret(
        scanPrivateKey,
        payment.ephemeralPubkey
      );

      if (isForMe(sharedSecret, payment)) {
        myPayments.push(payment);
      }
    }

    return myPayments;
  }
  ```

- [ ] Build "Scan for Payments" UI
- [ ] Integrate Seal for amount encryption in stealth payments
- [ ] Build "Send Private Payment" complete flow
- [ ] Test full stealth payment end-to-end

**End of Day 3 Deliverable:**

- ✅ Working stealth payment demo (KEY FEATURE!)
- ✅ Payment scanning works
- ✅ Amount encryption integrated
- ✅ Complete send → scan → claim flow

---

### **Day 4: Nautilus & Walrus** (8 hours)

**Morning (4 hours) - Nautilus:**

- [ ] Deploy Nautilus enclave with attestation
  ```bash
  cd nautilus/
  make ENCLAVE_APP=privacy && make run
  ```
- [ ] Implement computation endpoint
  ```rust
  #[post("/process_encrypted")]
  async fn process_encrypted(
      payload: Json<EncryptedPayload>
  ) -> Json<SignedResult> {
      // Decrypt inside TEE
      // Process computation
      // Sign result
      // Return with attestation
  }
  ```
- [ ] Deploy `nautilus_verifier.move`
- [ ] Test attestation verification

  ```typescript
  const result = await fetch(`${NAUTILUS_URL}/process_encrypted`, {
    method: "POST",
    body: JSON.stringify(encryptedData),
  });

  const { output, attestation, signature } = await result.json();

  // Verify on-chain
  await verifyAttestation(attestation, signature);
  ```

**Afternoon (4 hours) - Walrus:**

- [ ] Integrate Walrus SDK

  ```typescript
  import { WalrusClient } from "@walrus/sdk";

  const walrus = new WalrusClient({
    rpcUrl: WALRUS_TESTNET_RPC,
  });
  ```

- [ ] Store encrypted data on Walrus
  ```typescript
  async function storeOnWalrus(data: Uint8Array): Promise<string> {
    const blobId = await walrus.store(data);
    console.log(`Stored on Walrus: ${blobId}`);
    return blobId;
  }
  ```
- [ ] Reference blob IDs in Move contracts
- [ ] Build UI showing cost comparison
  ```typescript
  const onChainCost = dataSize * 100; // 100x replication
  const walrusCost = dataSize * 5; // 5x erasure coding
  const savings = ((onChainCost - walrusCost) / onChainCost) * 100;
  ```
- [ ] Fetch and decrypt from Walrus

**End of Day 4 Deliverable:**

- ✅ TEE attestation working (or mocked)
- ✅ Walrus storage working
- ✅ Cost comparison demo
- ✅ All 4 major features integrated

---

### **Day 5: Polish & Demo** (8 hours)

**Morning (4 hours):**

- [ ] Polish UI/UX
  - [ ] Consistent styling
  - [ ] Smooth animations
  - [ ] Clear labels and tooltips
  - [ ] Responsive design
- [ ] Add comprehensive error handling
  ```typescript
  try {
    await sendPayment();
  } catch (error) {
    if (error.code === "INSUFFICIENT_FUNDS") {
      toast.error("Insufficient balance");
    } else if (error.code === "USER_REJECTED") {
      toast.error("Transaction cancelled");
    } else {
      toast.error("Transaction failed");
    }
  }
  ```
- [ ] Add loading states everywhere
- [ ] Write user instructions and tooltips
- [ ] Test on multiple wallets (Sui Wallet, Suiet)

**Afternoon (4 hours):**

- [ ] Record demo video (5-10 minutes)
  - [ ] Show stealth payment flow
  - [ ] Show threshold encryption
  - [ ] Show cost comparison
  - [ ] Compare with Encifher
- [ ] Prepare presentation slides (8 slides max)
- [ ] Write README with:
  - [ ] Setup instructions
  - [ ] Demo walkthrough
  - [ ] Architecture explanation
  - [ ] Comparison table
- [ ] Deploy to production (Vercel)
  ```bash
  vercel deploy --prod
  ```
- [ ] Test production deployment
- [ ] Final testing on mobile

**End of Day 5 Deliverable:**

- ✅ Complete hackathon submission
- ✅ Demo video recorded
- ✅ Presentation ready
- ✅ Production deployment live
- ✅ Documentation complete

---

## User Flows (Demo Script)

### **Flow 1: Private Payment with Stealth Address** (5 min demo)

**Setup:**

- Alice: Sender wallet
- Bob: Recipient with scan key

**Step-by-step:**

```
1. [Alice] Open app → Connect wallet (Sui Wallet)
   Screen: "Connected: 0xAlice..."

2. [Alice] Click "Send Private Payment"
   Screen: Send payment form

3. [Alice] Enter:
   - Bob's scan key: "0xBobScanKey..."
   - Amount: 100 USDC

4. [Alice] Click "Generate Stealth Address"
   Screen shows:
   - "Stealth address generated: 0xEphemeral123..."
   - "This address is one-time use only"
   - "Only Bob can discover this payment"

5. [Alice] Click "Encrypt Amount"
   Loading: "Encrypting with Seal (2-of-3 threshold)..."
   Screen shows:
   - "✓ Encrypted by Server 1"
   - "✓ Encrypted by Server 2"
   - "✓ Threshold reached (2/3)"

6. [Alice] Click "Send Payment"
   Loading: "Sending transaction..."
   Success: "✓ Payment sent!"

   On-chain observers see:
   - StealthPayment object created
   - ephemeral_pubkey: 0xEphemeral123...
   - encrypted_amount: 0xCiphertext...
   - recipient: UNKNOWN (hidden!)

7. [Bob] Open app → Connect wallet
   Screen: "Connected: 0xBob..."

8. [Bob] Click "Scan for Payments"
   Loading: "Scanning blockchain..."
   Screen shows:
   - "Checking 47 stealth payments..."
   - "Found 1 payment for you!"

9. [Bob] Click on payment
   Screen shows:
   - "From: Stealth Address 0xEph..."
   - "Encrypted amount: 0xCipher..."
   - "Click to decrypt"

10. [Bob] Click "Decrypt Amount"
    Wallet prompt: "Approve session key for decryption"
    [Bob] Approves
    Loading: "Fetching keys from Seal servers..."
    Screen shows:
    - "✓ Key from Server 1"
    - "✓ Key from Server 2"
    - "Decrypted: 100 USDC"

11. [Bob] Click "Claim Payment"
    Loading: "Claiming funds..."
    Success: "✓ 100 USDC claimed!"

12. [Demo complete]
```

**What makes this impressive:**

- ✅ No one except Bob knows he received payment
- ✅ Amount is encrypted (Seal threshold)
- ✅ Stealth address is one-time use
- ✅ **Encifher CANNOT do this** ← Emphasize!

**Talking points:**

- "Notice how the recipient address is completely hidden"
- "Even the sender (Alice) doesn't know the final address"
- "Bob can scan privately without revealing his identity"
- "This is impossible with Encifher's architecture"

---

### **Flow 2: Threshold Encryption Demo** (3 min demo)

**Step-by-step:**

```
1. User clicks "Encrypt Data" tab

2. User enters secret message:
   "My strategy is to buy 1000 SOL at $200"

3. Screen shows key server selection:
   [ ] Server 1 (US-East)
   [ ] Server 2 (EU-West)
   [ ] Server 3 (Asia-Pacific)

   User selects all 3

4. User selects threshold: "2 of 3"
   Explanation: "Need 2 servers to decrypt"

5. User clicks "Encrypt"
   Loading animation shows:
   - "Encrypting with Server 1..." ✓
   - "Encrypting with Server 2..." ✓
   - "Encrypting with Server 3..." ✓

6. Screen shows result:
   Encrypted data: 0xDEADBEEF1234567890...
   Backup key: 0xBACKUP9876...

   Status:
   - "✓ Encrypted with 2-of-3 threshold"
   - "✓ No single server can decrypt"
   - "✓ Data safe even if 1 server compromised"

7. User clicks "Decrypt"
   Wallet prompt: "Approve session key?"
   User approves

8. Loading shows:
   - "Requesting key from Server 1..." ✓
   - "Requesting key from Server 2..." ✓
   - "Threshold reached! Decrypting..."

9. Screen shows decrypted:
   "My strategy is to buy 1000 SOL at $200"

10. [Demo complete]
```

**Comparison slide shown:**

```
Encifher:
❌ Single gateway (trust one party)
❌ encrypt.rpc.encifher.io
❌ Single point of failure

Our Solution:
✅ Multiple key servers (your choice)
✅ 2-of-3 threshold (no single point of failure)
✅ Decentralized and verifiable
```

**Talking points:**

- "You choose which servers to trust"
- "No single entity can decrypt your data"
- "True decentralization, not just marketing"

---

### **Flow 3: TEE Attestation Verification** (3 min demo)

**Step-by-step:**

```
1. User clicks "Verify TEE" tab

2. Screen shows:
   "Submit encrypted computation to Nautilus TEE"

3. User enters: "Compute: sum([1, 2, 3, 4, 5])"

4. User clicks "Submit to TEE"
   Loading: "Sending to secure enclave..."

5. Screen shows TEE processing:
   - "TEE received request"
   - "Processing in isolated environment..."
   - "Generating attestation..."

6. Result received:
   Output: 15
   Attestation Document: [Show hex]
   Signature: [Show hex]
   PCR Values:
   - PCR0: 911c87d0abc8c984...
   - PCR1: 911c87d0abc8c984...
   - PCR2: 21b9efbc18480766...

7. User clicks "Verify On-Chain"
   Loading: "Verifying attestation..."

8. Move contract verification:
   ✓ Attestation signature valid
   ✓ PCR values match registered enclave
   ✓ Enclave is active and trusted
   ✓ Computation verified!

9. Screen shows comparison:
   "Registered PCRs (on-chain):
    PCR0: 911c87d0abc8c984...

    Attestation PCRs (from TEE):
    PCR0: 911c87d0abc8c984...

    ✓ Match! Computation is verifiable."

10. [Demo complete]
```

**Show reproducible build:**

```
Terminal demo:
$ git clone <repo>
$ cd nautilus
$ make ENCLAVE_APP=privacy
$ cat out/nitro.pcrs

PCR0=911c87d0abc8c984...
PCR1=911c87d0abc8c984...
PCR2=21b9efbc18480766...

"These match on-chain! Anyone can verify."
```

**Talking points:**

- "Self-managed TEE, not external gateway"
- "Reproducible builds prove source code"
- "On-chain verification of attestation"
- "Encifher uses black box - you can't verify"

---

### **Flow 4: Walrus Cost Comparison** (2 min demo)

**Step-by-step:**

```
1. User clicks "Storage Cost Demo"

2. Screen shows:
   "Upload encrypted order book"

3. User uploads file: "encrypted_orders.json" (1.2 MB)

4. Loading: "Analyzing storage options..."

5. Screen shows two cards side-by-side:

   ┌─────────────────────────┐  ┌─────────────────────────┐
   │   Store On-Chain        │  │    Store on Walrus      │
   ├─────────────────────────┤  ├─────────────────────────┤
   │ File size: 1.2 MB       │  │ File size: 1.2 MB       │
   │ Replication: 100x       │  │ Erasure coding: 5x      │
   │ Effective: 120 MB       │  │ Effective: 6 MB         │
   │ Cost: ~$1,200 SUI       │  │ Cost: ~$60 SUI          │
   │                         │  │                         │
   │ ❌ Very expensive       │  │ ✅ 95% cheaper!         │
   └─────────────────────────┘  └─────────────────────────┘

6. User selects "Store on Walrus"

7. Loading: "Uploading to Walrus..."
   Progress bar: [████████████] 100%

8. Success screen:
   ✓ Stored on Walrus!
   Blob ID: bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi

   On-chain storage: 32 bytes (just the blob ID!)
   Actual data: 1.2 MB (on Walrus)

   Cost breakdown:
   - Walrus storage: $60 SUI
   - On-chain pointer: $0.001 SUI
   - Total: $60.001 SUI

   Savings: $1,139.999 SUI (95% cheaper!)

9. User clicks "Fetch from Walrus"
   Loading: "Retrieving data..."
   Success: "✓ Data retrieved and decrypted"

10. [Demo complete]
```

**Visual comparison chart:**

```
Bar chart:
On-chain: ████████████████████ $1,200
Walrus:   █ $60

          20x cheaper!
```

**Talking points:**

- "Walrus uses erasure coding, not replication"
- "Still decentralized, still secure"
- "Makes privacy practical for large datasets"
- "Production-ready for real applications"

---

## Judging Criteria Alignment

### **1. Innovation** (Weight: 30%)

**Our strengths:**

- ✅ **First stealth address implementation on Sui**

  - Novel cryptographic technique
  - Recipient privacy (Encifher can't do)
  - Clear innovation

- ✅ **Novel architecture combining 3 Sui primitives**

  - Nautilus + Seal + Walrus together
  - No one has built this stack
  - Showcases Sui ecosystem

- ✅ **Verifiable privacy** (new approach)
  - Reproducible builds
  - On-chain attestation
  - Transparent vs black box

**Pitch:**
"This is the first fully verifiable, decentralized privacy protocol on Sui, combining three cutting-edge primitives in a novel way."

---

### **2. Technical Complexity** (Weight: 25%)

**Our strengths:**

- ✅ **Threshold cryptography** (Seal integration)

  - Complex cryptographic protocols
  - Multi-party coordination
  - Key management

- ✅ **TEE integration** (Nautilus)

  - Secure enclave deployment
  - Attestation verification
  - Hardware-backed security

- ✅ **Stealth address cryptography** (ECDH)

  - Elliptic curve cryptography
  - Key derivation
  - Scanning algorithms

- ✅ **Multi-system integration**
  - 4 major components working together
  - Complex state management
  - Cross-system coordination

**Pitch:**
"We're not just calling APIs - we're implementing cryptographic primitives, managing TEEs, and coordinating multiple decentralized systems."

---

### **3. Practical Utility** (Weight: 25%)

**Our strengths:**

- ✅ **Solves real DeFi problem**

  - Privacy is #1 request in DeFi
  - MEV protection
  - Strategy confidentiality

- ✅ **Clear use cases**

  - Private payments (actual utility)
  - Confidential trading
  - DAO governance
  - Enterprise payments

- ✅ **Better than existing solutions**

  - Direct comparison with Encifher
  - Measurable improvements
  - Production-ready architecture

- ✅ **Cost efficiency**
  - 20x cheaper storage (Walrus)
  - Practical for real apps
  - Scalable solution

**Pitch:**
"This isn't a toy project. It solves real problems better than existing solutions, with a clear path to production."

---

### **4. Sui Ecosystem Fit** (Weight: 20%)

**Our strengths:**

- ✅ **Uses 3 Sui native tools**

  - Nautilus (TEE)
  - Seal (KMS)
  - Walrus (Storage)
  - Deep ecosystem integration

- ✅ **Demonstrates object model advantages**

  - Shared objects for scanning
  - Parallel execution potential
  - Natural access control

- ✅ **Move smart contract innovation**

  - Novel contract patterns
  - Composable policies
  - Sui-specific optimizations

- ✅ **Showcases Sui's privacy capabilities**
  - Positions Sui as privacy leader
  - Differentiates from other chains
  - Marketing value for Sui

**Pitch:**
"This project showcases what makes Sui special - it wouldn't be possible on other chains."

---

## Presentation Slide Outline (8 Slides)

### **Slide 1: Title**

```
[PROJECT NAME]
Verifiable Privacy DeFi on Sui

Tagline: "Your keys, your servers, your privacy"

Built with: Nautilus · Seal · Walrus

[Team names]
[GitHub] [Demo] [Docs]
```

---

### **Slide 2: The Problem**

```
Why DeFi Privacy Fails Today

Current solutions like Encifher:
❌ Centralized gateways (single point of failure)
❌ Black box computation (can't verify)
❌ Recipients visible on-chain
❌ Expensive on-chain storage
❌ No user control

Users want privacy, but not at the cost of security.
```

---

### **Slide 3: Our Solution**

```
True Privacy Through Decentralization

✅ Stealth Addresses
   → Recipients hidden on-chain

✅ Threshold Encryption (Seal)
   → 2-of-3 key servers, no single point of failure

✅ Verifiable TEE (Nautilus)
   → Self-managed, reproducible builds

✅ Cost-Efficient Storage (Walrus)
   → 20x cheaper than on-chain

Built on Sui's native privacy stack
```

---

### **Slide 4: Architecture**

```
[Architecture diagram from earlier]

Frontend → Seal (2-of-3) → Sui Blockchain
       ↘ Nautilus TEE ↗  ↘ Walrus Storage

Key Innovation: All components verifiable
- Seal: User chooses key servers
- Nautilus: Reproducible builds
- Walrus: Cryptographic proofs
```

---

### **Slide 5: Live Demo**

```
[Screen recording or live demo]

Demo: Private Payment with Stealth Address

1. Alice sends 100 USDC to Bob
2. Amount encrypted (Seal 2-of-3)
3. Recipient hidden (stealth address)
4. Bob scans and discovers payment
5. Only Bob can decrypt and claim

On-chain: Observers see nothing!
```

---

### **Slide 6: Comparison**

```
Us vs. Encifher

│ Feature             │ Encifher │ Our Solution │
├────────────────────┼──────────┼──────────────┤
│ Amount Privacy     │    ✅    │      ✅      │
│ Recipient Privacy  │    ❌    │      ✅      │
│ Threshold Crypto   │    ❌    │      ✅      │
│ Self-Managed TEE   │    ❌    │      ✅      │
│ Verifiable         │    ❌    │      ✅      │
│ Cost Efficient     │    ❌    │      ✅      │
│ Decentralized      │    ❌    │      ✅      │

We provide better privacy, more control,
and lower costs.
```

---

### **Slide 7: Impact & Use Cases**

```
Real-World Applications

🔒 Private Payments
   Enterprise payroll, vendor payments

📊 Confidential Trading
   Protect strategies from MEV bots

🗳️ DAO Governance
   Secret ballots, sealed-bid auctions

💼 Institutional DeFi
   Compliance-friendly privacy

Market Opportunity:
- $100B+ DeFi TVL needs privacy
- Privacy is #1 user request
- Sui positioned as privacy leader
```

---

### **Slide 8: Next Steps & Call to Action**

```
What's Next?

Phase 1 (Complete): Core MVP
✅ Stealth addresses
✅ Threshold encryption
✅ TEE verification
✅ Walrus integration

Phase 2 (Next): Production
🚀 Security audit
🚀 Mainnet deployment
🚀 DEX integrations
🚀 Mobile app

Try it now: [demo.link]
GitHub: [repo.link]
Docs: [docs.link]

Join us in building the future of private DeFi on Sui!
```

---

## Minimum Viable Demo (If Time Is Tight)

### **Priority Ranking:**

**MUST HAVE (Core Demo):**

1. ✅ Stealth addresses (KEY DIFFERENTIATOR)
2. ✅ Basic Seal encryption (2-of-3)
3. ✅ Working UI (send + receive)
4. ✅ One complete user flow
5. ✅ Move contracts deployed

**SHOULD HAVE (Impressive Demo):** 6. 🎯 Nautilus TEE integration 7. 🎯 Walrus storage integration 8. 🎯 Attestation verification 9. 🎯 Cost comparison UI

**NICE TO HAVE (Polish):** 10. ⭐ Advanced Seal policies 11. ⭐ Multiple payment flows 12. ⭐ Transaction history 13. ⭐ Mobile responsive

### **If Only 3 Days:**

**Day 1:** Setup + Seal integration
**Day 2:** Stealth addresses (all day)
**Day 3:** UI + Demo video

**Cut:** Nautilus, Walrus (mention in slides only)

### **If Only 2 Days:**

**Day 1:** Setup + Basic encryption
**Day 2:** Simplified stealth + Demo

**Cut:** Full threshold crypto, just show concept

**Fallback Demo:**
Focus entirely on stealth payments with basic encryption. This alone is:

- Novel on Sui
- Better than Encifher
- Technically impressive
- Clearly useful

---

## Risks & Mitigations

### **Risk 1: Nautilus Setup Too Complex** ⚠️ HIGH

**Indicators:**

- AWS Nitro requires specific instance types
- Enclave configuration is tricky
- Attestation document parsing is complex

**Mitigation:**

- Start early (Day 1 afternoon)
- Use pre-existing Nautilus weather example as template
- Have fallback: Mock attestation with disclaimer
  ```typescript
  // For demo only - production would use real TEE
  function mockAttestation() {
    return {
      pcr0: "mock_pcr_value",
      verified: true,
      note: "Demo mode - production uses real AWS Nitro",
    };
  }
  ```
- Focus on concept, not perfect implementation
- Judges care more about understanding than perfection

**Backup Plan:**
Show slides explaining how Nautilus works, demonstrate mocked version, emphasize "production would use real TEE"

---

### **Risk 2: Seal Testnet Servers Down** ⚠️ MEDIUM

**Indicators:**

- Testnet servers might be unreliable
- Rate limiting on free tier
- Network issues

**Mitigation:**

- Test connectivity immediately (Day 1)
- Have backup: Run own Seal key server locally
  ```bash
  cargo run --bin key-server -- --config local.yaml
  ```
- Cache successful encryptions for demo
- Have pre-recorded video backup

**Backup Plan:**
Run 2-3 local key servers, show they're independent processes, explain production would use distributed servers

---

### **Risk 3: Stealth Address Crypto Is Hard** ⚠️ HIGH

**Indicators:**

- ECDH key exchange is complex
- Scanning algorithm needs optimization
- Multiple cryptographic operations

**Mitigation:**

- Use battle-tested library: @noble/curves
  ```typescript
  import { secp256k1 } from "@noble/curves/secp256k1";
  // Well-tested, used in Bitcoin/Ethereum
  ```
- Study prior implementations:
  - Monero stealth addresses
  - Umbra protocol (Ethereum)
  - StealthPay (reference code)
- Start early (entire Day 3)
- Have team member dedicated to crypto
- Simplify for MVP:
  - Skip complex optimizations
  - Linear scan acceptable for demo
  - Focus on correctness over performance

**Backup Plan:**
If full stealth addresses fail, implement simpler "encrypted recipient" where recipient address is Seal-encrypted but not truly stealth. Still better than Encifher.

---

### **Risk 4: Move Contract Bugs** ⚠️ MEDIUM

**Indicators:**

- Move compiler errors
- Logic bugs in contracts
- Test failures

**Mitigation:**

- Keep contracts simple (MVP scope)
- Use shared objects (easier than owned)
- Test on testnet early and often
- Have team member review all Move code
- Use Move Prover for critical logic (optional)

**Testing checklist:**

```bash
# Local testing
sui move test

# Testnet deployment
sui client publish --gas-budget 100000000

# Integration testing
# Test each function with sui client call

# Verify objects created correctly
sui client objects
```

**Backup Plan:**
If contracts fail, use simplified versions or even off-chain state management with Seal for demo

---

### **Risk 5: Integration Hell** ⚠️ HIGH

**Indicators:**

- 4 major components (Frontend, Seal, Nautilus, Walrus)
- Multiple APIs to coordinate
- Complex state management
- Version mismatches

**Mitigation:**

- Build incrementally (one component per day)
- Test integration points early
- Use TypeScript for type safety
- Document APIs clearly
- Have integration testing on Day 4

**Integration checklist:**

- [ ] Frontend ↔ Sui (wallet, transactions)
- [ ] Frontend ↔ Seal (encrypt/decrypt)
- [ ] Frontend ↔ Nautilus (TEE calls)
- [ ] Frontend ↔ Walrus (storage)
- [ ] Move ↔ Seal (policies)
- [ ] Move ↔ Nautilus (attestation)

**Backup Plan:**
If integration fails, demonstrate components separately and explain how they would integrate in production

---

## Success Metrics

### **Minimum Success (Still Win):**

- [ ] Working stealth payment demo

  - Generate stealth address ✓
  - Send encrypted payment ✓
  - Scan and discover ✓
  - Claim payment ✓

- [ ] Seal threshold encryption working

  - Encrypt with 2-of-3 ✓
  - Decrypt with session key ✓

- [ ] Clean UI with one complete flow

  - Professional design ✓
  - Clear instructions ✓
  - No major bugs ✓

- [ ] 5-minute demo video
  - Shows stealth payment ✓
  - Explains advantages ✓
  - Compares with Encifher ✓

**Why this wins:**
Stealth addresses alone are novel on Sui and better than Encifher. Combined with threshold crypto, it's a strong hackathon project.

---

### **Good Success (Likely Win):**

Everything above, PLUS:

- [ ] All 4 features working

  - Stealth addresses ✓
  - Seal encryption ✓
  - Nautilus TEE ✓
  - Walrus storage ✓

- [ ] Multiple user flows demonstrated

  - Send payment ✓
  - Receive payment ✓
  - Encrypt/decrypt data ✓
  - Store on Walrus ✓

- [ ] Comparative analysis vs Encifher

  - Feature comparison table ✓
  - Cost comparison ✓
  - Architecture comparison ✓

- [ ] Production-ready deployment
  - Deployed on Vercel ✓
  - Working on testnet ✓
  - Mobile responsive ✓

**Why this wins:**
Complete demo of all Sui privacy primitives, clear advantages over existing solution, production-quality implementation.

---

### **Great Success (Definitely Win):**

Everything above, PLUS:

- [ ] Advanced features

  - Multiple Seal policies ✓
  - Time-locked payments ✓
  - Allowlist access control ✓

- [ ] Performance benchmarks

  - Encryption speed ✓
  - Transaction costs ✓
  - Storage costs ✓

- [ ] Open-source documentation

  - Architecture docs ✓
  - Integration guide ✓
  - API documentation ✓
  - Video tutorials ✓

- [ ] Community engagement
  - Twitter/X posts ✓
  - Discord discussions ✓
  - Developer interest ✓

**Why this wins:**
Goes beyond hackathon demo, shows production readiness, creates ecosystem value, generates buzz.

---

## Repository Structure

```
privacy-protocol-sui/
│
├── frontend/                          # Next.js application
│   ├── app/
│   │   ├── page.tsx                  # Home page
│   │   ├── send/
│   │   │   └── page.tsx              # Send private payment
│   │   ├── receive/
│   │   │   └── page.tsx              # Receive/scan payments
│   │   ├── history/
│   │   │   └── page.tsx              # Transaction history
│   │   ├── demo/
│   │   │   └── page.tsx              # Encryption demo
│   │   └── layout.tsx                # Root layout
│   │
│   ├── components/
│   │   ├── WalletConnect.tsx         # Wallet connection
│   │   ├── StealthAddressInput.tsx   # Generate stealth address
│   │   ├── EncryptionWidget.tsx      # Seal encryption UI
│   │   ├── PaymentScanner.tsx        # Scan for payments
│   │   ├── TransactionList.tsx       # Show tx history
│   │   ├── CostComparison.tsx        # Walrus cost demo
│   │   └── LoadingState.tsx          # Loading component
│   │
│   ├── lib/
│   │   ├── stealth.ts                # Stealth address logic
│   │   ├── seal.ts                   # Seal SDK wrapper
│   │   ├── sui.ts                    # Sui client setup
│   │   ├── walrus.ts                 # Walrus integration
│   │   ├── nautilus.ts               # Nautilus TEE client
│   │   └── utils.ts                  # Helper functions
│   │
│   ├── hooks/
│   │   ├── useStealth.ts             # Stealth address hook
│   │   ├── useSeal.ts                # Seal encryption hook
│   │   ├── usePaymentScanner.ts      # Payment scanning hook
│   │   └── useWalrus.ts              # Walrus storage hook
│   │
│   ├── types/
│   │   ├── stealth.ts                # Stealth address types
│   │   ├── seal.ts                   # Seal types
│   │   └── contracts.ts              # Move contract types
│   │
│   ├── public/
│   │   ├── logo.svg
│   │   └── demo-video.mp4
│   │
│   ├── package.json
│   ├── tsconfig.json
│   ├── tailwind.config.ts
│   └── next.config.js
│
├── contracts/                         # Sui Move contracts
│   ├── sources/
│   │   ├── stealth_payment.move      # Stealth payment contract
│   │   ├── nautilus_verifier.move    # TEE attestation verifier
│   │   └── seal_policy.move          # Seal access policies
│   │
│   ├── tests/
│   │   ├── stealth_payment_tests.move
│   │   ├── nautilus_tests.move
│   │   └── seal_policy_tests.move
│   │
│   ├── Move.toml
│   └── Move.lock
│
├── nautilus/                          # Nautilus TEE server
│   ├── src/
│   │   ├── main.rs                   # Server entry point
│   │   ├── attestation.rs            # Attestation handling
│   │   ├── computation.rs            # Encrypted computation
│   │   └── routes.rs                 # API routes
│   │
│   ├── Dockerfile.enclave            # Enclave container
│   ├── Cargo.toml
│   ├── run.sh                        # Run script
│   └── config.yaml                   # Configuration
│
├── scripts/                           # Deployment scripts
│   ├── deploy-contracts.sh           # Deploy Move contracts
│   ├── setup-nautilus.sh             # Set up TEE
│   ├── test-integration.sh           # Integration tests
│   └── demo-reset.sh                 # Reset demo state
│
├── docs/                              # Documentation
│   ├── ARCHITECTURE.md               # System architecture
│   ├── USER_GUIDE.md                 # How to use
│   ├── DEVELOPER_GUIDE.md            # Developer docs
│   ├── DEMO_SCRIPT.md                # Demo walkthrough
│   ├── COMPARISON.md                 # vs Encifher
│   └── API.md                        # API documentation
│
├── .env.example                       # Environment template
├── .gitignore
├── README.md                          # Project overview
├── LICENSE
└── package.json                       # Root package.json
```

---

## Environment Variables

Create `.env.local` in frontend:

```bash
# Sui Network
NEXT_PUBLIC_SUI_NETWORK=testnet
NEXT_PUBLIC_SUI_RPC_URL=https://fullnode.testnet.sui.io

# Deployed Contract Addresses
NEXT_PUBLIC_PACKAGE_ID=0x...
NEXT_PUBLIC_STEALTH_PAYMENT_CONFIG=0x...
NEXT_PUBLIC_NAUTILUS_VERIFIER_CONFIG=0x...

# Seal Configuration (Testnet)
NEXT_PUBLIC_SEAL_SERVER_1=0x73d05d62c18d9374e3ea529e8e0ed6161da1a141a94d3f76ae3fe4e99356db75
NEXT_PUBLIC_SEAL_SERVER_2=0xf5d14a81a982144ae441cd7d64b09027f116a468bd36e7eca494f750591623c8
NEXT_PUBLIC_SEAL_SERVER_3=0x...
NEXT_PUBLIC_SEAL_THRESHOLD=2

# Walrus Configuration
NEXT_PUBLIC_WALRUS_RPC=https://walrus-testnet.mystenlabs.com
NEXT_PUBLIC_WALRUS_AGGREGATOR=https://aggregator.walrus-testnet.mystenlabs.com

# Nautilus TEE
NEXT_PUBLIC_NAUTILUS_URL=https://your-enclave.compute.amazonaws.com:3000

# Optional: Analytics
NEXT_PUBLIC_ANALYTICS_ID=...
```

---

## Git Strategy

### **Branch Structure:**

```
main                    # Production-ready code
├── develop            # Integration branch
├── feature/stealth    # Stealth addresses
├── feature/seal       # Seal integration
├── feature/nautilus   # TEE implementation
├── feature/walrus     # Walrus storage
└── feature/ui         # Frontend UI
```

### **Commit Strategy:**

```bash
# Day 1
git commit -m "feat: initial project setup"
git commit -m "feat: wallet connection integration"
git commit -m "feat: deploy skeleton Move contracts"

# Day 2
git commit -m "feat: Seal SDK integration"
git commit -m "feat: threshold encryption working"
git commit -m "feat: session key management"

# Day 3
git commit -m "feat: stealth address generation"
git commit -m "feat: payment scanning logic"
git commit -m "feat: complete stealth payment flow"

# Day 4
git commit -m "feat: Nautilus TEE integration"
git commit -m "feat: Walrus storage integration"
git commit -m "feat: attestation verification"

# Day 5
git commit -m "feat: UI polish and animations"
git commit -m "docs: add comprehensive README"
git commit -m "chore: prepare demo deployment"
```

---

## Resources & Links

### **Documentation:**

- **Seal:** https://docs.mystenlabs.com/seal
- **Nautilus:** https://docs.mystenlabs.com/nautilus
- **Walrus:** https://docs.walrus.site
- **Sui Move:** https://docs.sui.io/guides/developer/first-app
- **@mysten/dapp-kit:** https://sdk.mystenlabs.com/dapp-kit

### **Code Examples:**

- **Seal integration tests:** Check Seal GitHub repo
- **Nautilus weather example:** In Nautilus repo
- **Walrus sites:** https://github.com/MystenLabs/walrus-sites

### **Libraries:**

```json
{
  "@mysten/seal-sdk": "latest",
  "@mysten/sui.js": "^0.54.0",
  "@mysten/dapp-kit": "^0.14.0",
  "@noble/curves": "^1.3.0",
  "@noble/hashes": "^1.3.3"
}
```

### **Tools:**

- **Sui CLI:** https://docs.sui.io/references/cli
- **Move Analyzer:** VS Code extension
- **Sui Explorer:** https://suiexplorer.com/?network=testnet

### **Community:**

- **Sui Discord:** https://discord.gg/sui
- **Nautilus Channel:** #nautilus in Sui Discord
- **Seal Channel:** #seal in Sui Discord

---

## Go/No-Go Checklist (Before Demo)

### **Technical Requirements:**

**GO if you have:**

- [ ] Stealth payment works end-to-end

  - [ ] Generate stealth address
  - [ ] Send encrypted payment
  - [ ] Scan and discover payment
  - [ ] Decrypt and claim

- [ ] Seal encryption/decryption works

  - [ ] Connect to 2+ key servers
  - [ ] Encrypt with threshold
  - [ ] Session key approval
  - [ ] Successful decryption

- [ ] UI is presentable

  - [ ] No visual bugs
  - [ ] Loading states work
  - [ ] Error handling present
  - [ ] Mobile responsive (basic)

- [ ] One complete user flow works reliably

  - [ ] Tested 3+ times
  - [ ] No critical bugs
  - [ ] Acceptable performance

- [ ] Move contracts deployed and working
  - [ ] Published to testnet
  - [ ] Functions callable
  - [ ] Objects created correctly

**NO-GO if:**

- [ ] No features work end-to-end
- [ ] Critical bugs in demo flow
- [ ] Can't connect to Seal servers
- [ ] Move contracts won't deploy
- [ ] UI completely broken

### **Demo Requirements:**

**GO if you have:**

- [ ] Demo script prepared (5-10 min)
- [ ] Demo video recorded (backup)
- [ ] Presentation slides ready (8 slides)
- [ ] Code deployed (Vercel/similar)
- [ ] Team knows their parts

**NO-GO if:**

- [ ] No demo prepared
- [ ] No backup plan
- [ ] Code only on localhost
- [ ] Team unprepared

### **Documentation Requirements:**

**GO if you have:**

- [ ] README with setup instructions
- [ ] Architecture explanation
- [ ] Comparison with Encifher
- [ ] Demo walkthrough

**Nice to have:**

- [ ] Video tutorial
- [ ] API documentation
- [ ] Developer guide

---

## Post-Hackathon Roadmap

### **Week 1-2: Polish & Fixes**

- [ ] Fix bugs discovered during demo
- [ ] Improve UI/UX based on feedback
- [ ] Add comprehensive error handling
- [ ] Write unit tests
- [ ] Add integration tests
- [ ] Security review of crypto code

### **Month 1: Production Preparation**

- [ ] Security audit (smart contracts)
- [ ] Professional code review
- [ ] Performance optimization
- [ ] Deploy to mainnet
- [ ] Set up monitoring (Sentry, etc.)
- [ ] Write comprehensive documentation
- [ ] Create video tutorials

### **Month 2-3: Feature Expansion**

- [ ] Advanced privacy features

  - [ ] Multi-hop stealth addresses
  - [ ] Ring signatures (optional)
  - [ ] Mixing protocols

- [ ] DEX integrations

  - [ ] Cetus integration
  - [ ] Turbos integration
  - [ ] FlowX integration

- [ ] Compliance modules

  - [ ] Optional KYC integration
  - [ ] Transaction limits
  - [ ] Audit logging

- [ ] Mobile app
  - [ ] React Native version
  - [ ] Mobile wallet support

### **Month 4+: Ecosystem Growth**

- [ ] Partnerships

  - [ ] DEX partnerships
  - [ ] Wallet integrations
  - [ ] Infrastructure providers

- [ ] Liquidity incentives

  - [ ] Token launch (if applicable)
  - [ ] Liquidity mining
  - [ ] User rewards

- [ ] Community building

  - [ ] Discord server
  - [ ] Twitter presence
  - [ ] Educational content

- [ ] Grants and funding
  - [ ] Sui Foundation grant
  - [ ] Mysten Labs grant
  - [ ] VC fundraising

---

## Team Roles (Suggested)

### **2-Person Team:**

- **Person 1:** Frontend + Smart Contracts

  - Next.js app
  - Move contracts
  - UI/UX
  - Demo

- **Person 2:** Cryptography + Infrastructure
  - Stealth address implementation
  - Seal integration
  - Nautilus setup
  - Walrus integration

### **3-Person Team:**

- **Person 1:** Frontend

  - Next.js app
  - UI components
  - User flows

- **Person 2:** Smart Contracts + Backend

  - Move contracts
  - Nautilus TEE
  - API integration

- **Person 3:** Cryptography
  - Stealth addresses
  - Seal integration
  - Security review

### **4-Person Team:**

- **Person 1:** Frontend Lead
- **Person 2:** Smart Contracts
- **Person 3:** Cryptography/Security
- **Person 4:** Infrastructure (Nautilus + Walrus)

---

## Final Checklist

### **Before Starting:**

- [ ] Team assembled
- [ ] Roles assigned
- [ ] GitHub repo created
- [ ] Development environment set up
- [ ] AWS account ready (for Nautilus)
- [ ] Sui testnet tokens acquired
- [ ] Project name decided

### **Day 1 Evening:**

- [ ] Project structure created
- [ ] Wallet connection working
- [ ] Move contracts deployed
- [ ] Team aligned on scope

### **Day 3 Evening (Critical Checkpoint):**

- [ ] Stealth payments working (MUST HAVE)
- [ ] Seal encryption working
- [ ] Basic UI functional
- [ ] Demo flow identified

### **Day 5 Morning (Final Checkpoint):**

- [ ] All chosen features working
- [ ] Demo script finalized
- [ ] Presentation slides ready
- [ ] Code deployed
- [ ] Video recorded (backup)

### **Submission:**

- [ ] Code pushed to GitHub
- [ ] Demo video uploaded
- [ ] Presentation submitted
- [ ] Documentation complete
- [ ] Links working

---

## Motivation & Inspiration

### **Why This Matters:**

**For Users:**

- Privacy is a fundamental right
- MEV protection saves millions
- Confidential trading enables new strategies
- Enterprise adoption requires privacy

**For Sui:**

- Showcases Sui's unique capabilities
- Positions Sui as privacy leader
- Attracts privacy-conscious developers
- Differentiates from other L1s

**For Blockchain:**

- Makes DeFi accessible to institutions
- Enables compliant privacy solutions
- Proves privacy and transparency can coexist
- Advances the entire ecosystem

### **Success Stories:**

- Tornado Cash: $1B+ in deposits (before sanctions)
- Zcash: $2B+ market cap
- Monero: $3B+ market cap
- **Opportunity:** Privacy DeFi on modern infrastructure

---

## Let's Build! 🚀

**Next Immediate Steps:**

1. ✅ Choose project name
2. ✅ Create GitHub repository
3. ✅ Set up Next.js project
4. ✅ Deploy basic Move contracts
5. ✅ Start Day 1 implementation

**Remember:**

- Start simple, iterate
- Test early, test often
- Document as you go
- Focus on core differentiators
- Have fun building!

**You've got this! The Sui privacy stack is powerful, and you're about to showcase it in the best way possible.**

---

_This PRD will be refined as we build. Don't be afraid to adapt and adjust based on what you learn during development._

**Good luck! 🎉**
