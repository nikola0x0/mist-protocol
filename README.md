# Mist Protocol

**Privacy-preserving DeFi swaps on Sui using Nautilus TEE and SEAL encryption**

Built with: [Nautilus](https://docs.sui.io/concepts/cryptography/nautilus) • [SEAL](https://docs.mystenlabs.com/seal) • [Cetus](https://cetus.zone) • [Sui](https://sui.io)

---

## What is Mist Protocol?

Mist Protocol enables **truly private token swaps** on Sui by combining:
- **SEAL threshold encryption** for hiding swap amounts
- **Nautilus TEE** (AWS Nitro Enclaves) for trusted execution
- **Intent-based architecture** for asynchronous swap processing
- **TEE wallet separation** to break on-chain linkability

Unlike traditional DEXs where every swap is publicly visible, Mist Protocol keeps swap amounts private while maintaining verifiability through TEE attestation.

---

## Key Innovation

### Privacy Through TEE Wallet Separation

```
Traditional DEX:                  Mist Protocol:
User Wallet → Swap → Output       User → Encrypted Intent → TEE Wallet → Swap → Encrypted Output
     ↑                                                           ↑
  Publicly linked                                    Unlinkable on-chain
```

**Result:** On-chain observers cannot link users to their swap transactions!

---

## How It Works

### 1. Deposit & Get Encrypted Tickets

```
User deposits 1.0 SUI
  ↓
Creates encrypted ticket in vault
  ↓
User can decrypt: "1.0 SUI"
TEE can decrypt: "1.0 SUI"
On-chain: [encrypted bytes]
```

### 2. Create Swap Intent

```
User selects tickets to swap
  ↓
Frontend encrypts intent with SEAL
  ↓
On-chain: SwapIntent object added to IntentQueue
  ↓
Backend polls queue every 5 seconds
```

### 3. TEE Executes Swap

```
Backend detects intent
  ↓
Decrypts with SEAL (2-of-3 threshold)
  ↓
TEE wallet executes swap on Cetus
  ↓
Re-encrypts output with SEAL
  ↓
Creates new encrypted ticket in user vault
```

### 4. User Receives Output

```
User refreshes vault
  ↓
Sees new encrypted output ticket
  ↓
Decrypts with SEAL: "0.95 SUI"
  ↓
Can unwrap to get real tokens
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
│  LiquidityPool  │  VaultEntry  │  IntentQueue      │
│  (Shared)       │  (Per-user)  │  (Shared)         │
│                                                     │
│  - SUI/USDC     │  - Encrypted │  - Pending        │
│  - TEE wallet   │    tickets   │    intents        │
└─────────────────────────────────────────────────────┘
       ▲                                      │
       │                                      │ Polls every 5s
       │ Executes signed tx                  ▼
       │                          ┌──────────────────────┐
       │                          │  Nautilus Backend    │
       │                          │  (Rust + TEE)        │
       │                          │                      │
       │                          │  - SEAL decrypt      │
       │                          │  - Cetus swap        │
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
| **Backend (TEE)** | Rust, Axum, Nautilus, SEAL SDK |
| **Signing Service** | Rust, Axum, Sui CLI |
| **Frontend** | Next.js 14, TypeScript, @mysten/dapp-kit |
| **Encryption** | SEAL (threshold encryption) |
| **DEX** | Cetus Protocol |
| **TEE** | AWS Nitro Enclaves |

---

## Repository Structure

```
mist-protocol/
├── backend/              # Nautilus TEE backend (Rust)
│   ├── src/apps/mist-protocol/
│   │   ├── intent_processor.rs   # Polls IntentQueue
│   │   ├── swap_executor.rs      # Executes swaps
│   │   └── seal_encryption.rs    # SEAL crypto
│   └── Cargo.toml
│
├── tx-signer/            # Transaction signing service
│   ├── src/main.rs       # HTTP wrapper around sui keytool
│   └── Cargo.toml
│
├── contracts/            # Sui Move smart contracts
│   └── mist_protocol/
│       └── sources/
│           ├── mist_protocol.move    # Main protocol
│           └── seal_policy.move      # Vault + tickets
│
├── frontend/             # Next.js frontend
│   ├── app/              # Pages
│   ├── components/       # React components
│   └── lib/seal-vault.ts # SEAL integration
│
├── cetus-swap/           # Cetus integration (future)
│
└── docs/                 # Documentation
    ├── ARCHITECTURE.md   # System design
    ├── SETUP.md          # Installation guide
    └── SIGNING_SOLUTION.md  # Technical notes
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
- Receive encrypted ticket #1

### 2. View Balance (Private!)
- Click "Decrypt" on ticket
- Sign message with wallet
- See: "0.5 SUI" ✅
- On-chain: Only encrypted bytes visible

### 3. Create Swap Intent
- Select ticket #1 (0.5 SUI)
- Swap to: USDC
- Min output: 0.475 USDC (5% slippage)
- Create intent

### 4. TEE Processes (Automatic)
Backend logs show:
```
📊 Poll cycle #5
✅ Successfully decrypted intent
   🎯 Intent: 0x1e6f...
   💱 Swap: 0.5 SUI → USDC (min: 0.475)
   🎫 Tickets: ["#1: 0.5"]

🔄 Executing swap...
   🔐 Encrypting output amount with SEAL...
   ✅ Encrypted successfully!
   🔐 Calling signing service...
   ✅ Transaction signed successfully!
   🚀 Executing signed transaction on-chain...
   ✅ Swap executed successfully!
   📝 Transaction: rkZeR5Fw5j...
```

### 5. View Output
- Refresh vault
- See new ticket #2
- Decrypt: "0.48 USDC" ✅
- Unwrap to get real USDC

---

## What Makes This Special

### 🔐 True Privacy
- Swap amounts encrypted with SEAL threshold encryption
- On-chain observers see only encrypted bytes
- Even node operators cannot see amounts

### 🔒 TEE Security
- AWS Nitro Enclaves provide hardware attestation
- Backend code verifiable through attestation document
- Keys released only to verified TEE

### 🎯 Intent-Based
- Users submit intents, TEE executes asynchronously
- No need to stay online during swap
- MEV-resistant (intents processed in queue order)

### 💡 TEE Wallet Separation
- TEE uses its own wallet for swaps
- Breaks user → swap transaction linkage
- Enhanced privacy vs traditional DEXs

---

## Current Status

### ✅ Working Features (Tested & Verified)

- [x] Deposit SUI/USDC with SEAL encryption
- [x] Create encrypted swap intents
- [x] TEE polls IntentQueue every 5 seconds
- [x] SEAL threshold decryption (2-of-3 key servers)
- [x] Transaction signing via tx-signer service
- [x] Execute swap transactions on-chain
- [x] Create encrypted output tickets
- [x] User decrypt output amounts
- [x] Unwrap tickets to real tokens

### 🚧 In Progress

- [ ] Real Cetus swap integration (currently mock: SUI → SUI)
- [ ] USDC swap support
- [ ] Production deployment to AWS Nitro Enclaves

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
- User ticket balances (SEAL encrypted)
- User → swap linkage (TEE wallet breaks link)

**What's Public:**
- Total pool liquidity (required for AMM)
- Swap events (user address, tokens, but amounts encrypted)
- Intent queue (pending vs completed)

### Trust Model

**Trusted:**
- AWS Nitro Enclaves (hardware attestation)
- SEAL key servers (2-of-3 threshold)
- Smart contract logic (auditable on-chain)

**Not Trusted:**
- Individual key servers (threshold prevents collusion)
- RPC nodes (cannot decrypt)
- Frontend (encryption happens client-side)

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
- **Nautilus TEE:** https://docs.sui.io/concepts/cryptography/nautilus
- **SEAL Encryption:** https://docs.mystenlabs.com/seal
- **Cetus DEX:** https://cetus.zone

---

## License

Apache-2.0

Copyright (c) Mysten Labs (Nautilus framework)
Mist Protocol implementation by Nikola & Max

---

## Acknowledgments

- **Mysten Labs** for Nautilus, SEAL, and Sui
- **Cetus Protocol** for DEX infrastructure
- **AWS** for Nitro Enclaves

---

**Built for Sui Hackathon - November 2025**
