# Mist Protocol

[First Mover Sprint Submission](https://www.deepsurge.xyz/projects/37682138-79c9-48ca-86b9-10f693a37fa4) 

**Privacy-preserving DeFi swaps on Sui using TEE (Trusted Execution Environment) and SEAL encryption**

Built with: [AWS Nitro Enclaves](https://aws.amazon.com/ec2/nitro/nitro-enclaves/) • [SEAL](https://docs.mystenlabs.com/seal) • [Cetus](https://cetus.zone) • [Sui](https://sui.io)

---

## What is Mist Protocol?

Mist Protocol enables **truly private token swaps** on Sui by combining:
- **SEAL threshold encryption** for hiding swap amounts
- **TEE** (AWS Nitro Enclaves) for trusted execution with hardware attestation
- **Privacy relayer** to break on-chain linkability
- **Stealth addresses** for unlinkable swap outputs

Unlike traditional DEXs where every swap is publicly visible, Mist Protocol keeps swap amounts private while maintaining verifiability through TEE attestation.

---

## Key Innovation

### Privacy Through Stealth Addresses

```
Traditional DEX:                  Mist Protocol:
User Wallet → Swap → Output       User → Deposit → Intent → TEE → Stealth → Claim
     ↑                                     ↑                        ↑
  Publicly linked              No owner field              Unlinkable address
```

**Result:** On-chain observers cannot link deposits to swap outputs!

---

## How It Works

### 1. Deposit & Get Deposit Note

```
User deposits 1.0 SUI
  ↓
Creates deposit with secret nullifier
  ↓
Deposit note stored locally (encrypted)
On-chain: Deposit object (no owner field - privacy!)
```

### 2. Create Swap Intent

```
User generates stealth addresses (output + remainder)
  ↓
Signs intent with wallet (proves ownership)
  ↓
Frontend SEAL-encrypts: nullifier, amounts, stealth addresses
  ↓
Submit on-chain (direct or via optional relayer)
  ↓
On-chain: SwapIntent added to IntentQueue
```

### 3. TEE Executes Swap

```
Backend polls queue every 5 seconds
  ↓
Decrypts with SEAL (2-of-3 threshold)
  ↓
Verifies wallet signature (prevents nullifier theft)
  ↓
TEE wallet executes swap on FlowX
  ↓
Sends output to stealth address (unlinkable!)
```

### 4. Claim from Stealth Address

```
User sees tokens at stealth address
  ↓
Claims to main wallet (sponsored tx)
  ↓
On-chain: No link between deposit and output!
```

---

## Architecture

```
┌──────────────┐
│   Frontend   │  Next.js + @mysten/dapp-kit + SEAL SDK
└──────┬───────┘
       │ Creates encrypted intents
       ▼
┌─────────────────────────────────────────────────────┐
│          Sui Blockchain (Move Contracts)            │
│                                                     │
│  Deposits       │  NullifierReg │  IntentQueue      │
│  (Shared)       │  (Shared)     │  (Shared)         │
│                                                     │
│  - No owner     │  - Spent      │  - Pending        │
│    field!       │    nullifiers │    intents        │
└─────────────────────────────────────────────────────┘
       ▲                                      │
       │                                      │ Polls every 5s
       │ Executes signed tx                  ▼
       │                          ┌──────────────────────┐
       │                          │  TEE Backend         │
       │                          │  (Rust + AWS Nitro)  │
       │                          │                      │
       │                          │  - SEAL decrypt      │
       │                          │  - DEX swap          │
       │                          │  - SEAL encrypt      │
       │                          │  - Build tx          │
       │                          └──────────┬───────────┘
       │                                     │
       │                                     │ POST /sign
       │                                     ▼
       │                          ┌──────────────────────┐
       └──────────────────────────│    tx-signer        │
                                  │  (HTTP Service)      │
                                  │                      │
                                  │  Wraps:              │
                                  │  sui keytool sign    │
                                  └──────────────────────┘
```

---

## Tech Stack

| Component | Technology |
|-----------|------------|
| **Smart Contracts** | Sui Move |
| **Backend (TEE)** | Rust, Axum, SEAL SDK |
| **Signing Service** | Rust, Axum, Sui CLI |
| **Frontend** | Next.js 14, TypeScript, @mysten/dapp-kit |
| **Encryption** | SEAL (2-of-3 threshold encryption) |
| **DEX** | FlowX (testnet) → MIST_TOKEN |
| **TEE** | AWS Nitro Enclaves (hardware attestation) |

---

## Repository Structure

```
mist-protocol/
├── backend/              # TEE backend (Rust + AWS Nitro)
│   └── src/apps/mist-protocol/
│       ├── intent_processor.rs   # Polls IntentQueue
│       ├── swap_executor.rs      # Executes swaps via FlowX
│       └── seal_encryption.rs    # SEAL crypto
│
├── enclave/              # AWS Nitro Enclave deployment
│   ├── Makefile          # Build enclave image
│   ├── deploy.sh         # Deployment scripts
│   └── AWS_QUICKSTART.md # Deployment guide
│
├── tx-signer/            # Transaction signing service
│   └── src/main.rs       # HTTP wrapper around sui keytool
│
├── contracts/            # Sui Move smart contracts
│   └── mist_protocol/
│       └── sources/
│           ├── mist_protocol.move    # Main protocol
│           └── seal_policy.move      # TEE + user decryption
│
├── frontend/             # Next.js frontend
│   ├── app/              # Pages + privacy relayer API
│   ├── components/       # React components
│   └── lib/deposit-notes.ts # Stealth address + note management
│
└── docs/                 # Documentation
```

---

## Quick Start

### Prerequisites

```bash
sui --version    # 1.60.0+
node --version   # v20.0.0+
pnpm --version   # 8.0.0+
cargo --version  # 1.70.0+
```

### 1. Install Dependencies

```bash
# Frontend
cd frontend && pnpm install

# Backend (will download and build dependencies)
cd ../backend && cargo build --release

# Signing service
cd ../tx-signer && cargo build --release
```

### 2. Deploy Contracts

```bash
cd contracts/mist_protocol
sui client publish --gas-budget 500000000

# Save package_id, pool_id, queue_id from output
```

### 3. Configure

**Backend:** Update `backend/src/apps/mist-protocol/seal_config.yaml`
**Frontend:** Update `frontend/.env.local`
**Backend key:** Set in `backend/.env`

See [docs/SETUP.md](docs/SETUP.md) for detailed configuration.

### 4. Run

```bash
# Terminal 1: Signing service
cd tx-signer && cargo run

# Terminal 2: Backend
cd backend && cargo run

# Terminal 3: Frontend
cd frontend && pnpm dev

# Open http://localhost:3000
```

---

## Live Demo Flow

### 1. Deposit SUI
- Connect wallet
- Deposit 0.5 SUI
- Receive deposit note with secret nullifier

### 2. Create Swap Intent
- Select deposit note (0.5 SUI)
- Swap to: MIST_TOKEN
- Sign intent with wallet
- Generates stealth addresses automatically

### 3. TEE Processes (Automatic)
Backend logs show:
```
📊 Poll cycle #5
✅ Successfully decrypted intent
   🎯 Intent: 0x1e6f...
   💱 Swap: 0.5 SUI → MIST_TOKEN
   ✅ Signature verified!

🔄 Executing swap on FlowX...
   🔐 Calling signing service...
   ✅ Transaction signed successfully!
   🚀 Executing signed transaction on-chain...
   ✅ Swap executed successfully!
   📝 Transaction: rkZeR5Fw5j...
```

### 4. Claim Output
- Check "Claim" tab
- See MIST_TOKEN at stealth address
- Click "Claim to Main Wallet"
- Tokens transferred (unlinkable on-chain!)

---

## What Makes This Special

### 🔐 True Privacy
- Swap amounts encrypted with SEAL threshold encryption
- On-chain observers see only encrypted bytes
- Even node operators cannot see amounts

### 🔒 TEE Security
- AWS Nitro Enclaves provide hardware-based trusted execution
- Cryptographic attestation proves code integrity
- SEAL keys released only to verified TEE
- Ephemeral keypairs generated inside enclave (never written to disk)

### 🎯 Intent-Based
- Users submit intents, TEE executes asynchronously
- No need to stay online during swap
- MEV-resistant (intents processed in queue order)

### 🔄 Privacy Relayer (Optional)
- Relayer can submit swap intents on behalf of users
- User's wallet never touches the swap transaction
- Extra privacy layer for those who want it

### 💡 Stealth Addresses
- Swap outputs sent to unlinkable stealth addresses
- User generates keypair locally, only they can claim
- On-chain: No link between deposit and output

---

## Current Status

### ✅ Working Features (Tested & Verified)

- [x] Deposit tokens with secret nullifier
- [x] Create encrypted swap intents with stealth addresses
- [x] Optional privacy relayer (submits intents on behalf of users)
- [x] Wallet signature verification (prevents nullifier theft)
- [x] TEE polls IntentQueue every 5 seconds
- [x] SEAL threshold decryption (2-of-3 key servers)
- [x] Transaction signing via tx-signer service
- [x] Execute swap on FlowX DEX
- [x] Send output to unlinkable stealth addresses
- [x] Claim tokens from stealth addresses

### 🚧 In Progress

- [ ] Cetus mainnet integration (implemented, pending deployment)
- [ ] Production deployment to AWS Nitro Enclaves (c5.xlarge)

**Note:** Testnet uses FlowX → MIST_TOKEN. Cetus mainnet swap is implemented but not yet deployed.

### 🎯 Future Enhancements

- [ ] Batch swap execution (multiple intents in one tx)
- [ ] Cross-pool swaps
- [ ] Additional token support
- [ ] Zero-knowledge proofs for enhanced privacy

---

## Documentation

- **[Architecture](docs/ARCHITECTURE.md)** - Complete system design and data flow
- **[Setup Guide](docs/SETUP.md)** - Installation, configuration, and deployment

---

## Technical Highlights

### Transaction Signing Solution

Due to fastcrypto version conflicts between SEAL SDK and sui-types, we built a novel signing architecture:

- **Backend:** Handles all SEAL encryption (fastcrypto v1)
- **tx-signer:** Signs transactions only (fastcrypto v2)
- **Result:** Clean separation, no version conflicts

This HTTP wrapper pattern is production-ready and commonly used in Sui projects. See [tx-signer/README.md](tx-signer/README.md).

### SEAL Integration

First DeFi protocol on Sui to use SEAL threshold encryption for:
- User balance privacy
- TEE-verifiable decryption
- Dual-party access (user + TEE)

### Wallet Signature Verification

Every swap intent requires a wallet signature to prevent nullifier theft attacks:
- Message format: `mist_intent_v2:{nullifier}:{inputAmount}:{outputStealth}:{remainderStealth}`
- TEE verifies signature before executing (Ed25519, Secp256k1, Secp256r1 supported)
- Attackers cannot steal nullifiers without the user's wallet private key

### Intent Queue Architecture

100% on-chain intent tracking:
- No database required
- Survives backend restarts
- Transparent and auditable
- Efficient RPC queries

---

## Security

### Privacy Model

**What's Private:**
- Individual swap amounts (SEAL encrypted)
- Deposit ownership (no owner field on-chain)
- User → swap linkage (stealth addresses break link)

**What's Public:**
- Deposit events (no amounts, no owner)
- Intent queue state (encrypted contents)
- Stealth address balances (unlinkable to user)

### Trust Model

**Trusted:**
- AWS Nitro Enclaves (hardware attestation via NSM API)
- SEAL key servers (2-of-3 threshold - no single point of failure)
- Smart contract logic (auditable on-chain)
- TEE backend address (hardcoded in contract for authorization)

**Not Trusted:**
- Individual key servers (threshold prevents collusion)
- RPC nodes (cannot decrypt - see only encrypted bytes)
- Frontend (encryption happens client-side with SEAL SDK)

---

## Demo

**Testnet Deployment:**
- Contracts: `0x584b4dd0e047e8cca64f82f5945a0f75cfd1c1e06d3757831a82369de976f89a`
- Network: Sui Testnet

**Try it yourself:**
1. Get testnet SUI from [faucet](https://docs.sui.io/guides/developer/getting-started/get-coins)
2. Follow setup guide: [docs/SETUP.md](docs/SETUP.md)
3. Create a swap and watch the TEE process it!

---

## Resources

- **Sui Documentation:** https://docs.sui.io
- **AWS Nitro Enclaves:** https://aws.amazon.com/ec2/nitro/nitro-enclaves/
- **SEAL Encryption:** https://docs.mystenlabs.com/seal
- **FlowX DEX:** https://flowx.finance

---

## License

Apache-2.0

Mist Protocol implementation by Nikola & Max

---

## Acknowledgments

- **Mysten Labs** for SEAL and Sui
- **Cetus Protocol** for DEX infrastructure
- **AWS** for Nitro Enclaves

---

**Built by Misty Labs**
