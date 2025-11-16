# Mist Protocol

**Privacy-preserving intent-based DeFi on Sui**

Built with: Nautilus • Seal • Walrus • Cetus • Sui

---

## Overview

Mist Protocol enables private DeFi swaps through a vault + ticket system with Nautilus TEE execution. Users deposit tokens and receive encrypted tickets representing balances. The Nautilus TEE decrypts intents and executes swaps with its own wallet, making individual user swaps unlinkable on-chain.

### Key Features

- **Nautilus TEE:** AWS Nitro Enclaves for trusted execution with attestation
- **Encrypted Tickets:** Vault balances encrypted with SEAL threshold encryption
- **TEE Wallet:** TEE executes swaps with own wallet (unlinks users from swaps)
- **User + TEE Decryption:** Both users and TEE can decrypt ticket amounts
- **Flexible Deposits:** Users split deposits into multiple tickets
- **Private Swaps:** Swap amounts and ticket balances remain encrypted

---

## Architecture

```
User Deposits → Mist Pool → Vault (Tickets) → SEAL Encrypted Intent
                   ↓                                    ↓
              Lock Tokens                    Nautilus TEE Decrypts
                                                       ↓
                                            TEE Wallet Swaps on Cetus
                                                       ↓
                                            Vault Tickets Updated (encrypted)
```

### Components

**Nautilus TEE:** AWS Nitro Enclaves for verifiable computation
**Vault System:** Per-user vaults with encrypted tickets (SEAL)
**Mist Pool:** Shared liquidity pool holding all user deposits
**TEE Wallet:** Separate wallet for executing swaps (breaks user→swap linkage)
**SEAL Encryption:** Threshold encryption for ticket amounts
**Cetus Integration:** DEX execution with TEE wallet

---

## Tech Stack

**Frontend:** Next.js 14, TypeScript, @mysten/dapp-kit, Tailwind CSS
**Smart Contracts:** Sui Move
**TEE:** Nautilus, Rust + Axum, AWS Nitro Enclaves
**Backend:** Node.js, Express, TypeScript
**Storage:** Walrus
**DEX:** Cetus Protocol

---

## Getting Started

### Prerequisites

```bash
node --version  # v20+
pnpm --version
sui --version
docker --version
cargo --version
```

### Installation

```bash
# Clone repository
git clone https://github.com/nikola0x0/mist-protocol.git
cd mist-protocol

# Install dependencies
cd frontend && pnpm install
cd ../backend && pnpm install
cd ../nautilus && cargo build
cd ../contracts && sui move build
```

### Development

```bash
# Run frontend
cd frontend && pnpm dev

# Run backend
cd backend && pnpm dev

# Deploy contracts
cd contracts && sui client publish

# Run Nautilus enclave (AWS Nitro)
cd nautilus && ./scripts/deploy_enclave.sh
```

---

## Repository Structure

```
mist-protocol/
├── frontend/           # Next.js + wallet integration
├── contracts/          # Sui Move escrow contracts
├── nautilus/           # TEE server (Rust + AWS Nitro)
├── backend/            # Intent processing backend
├── scripts/            # Deployment automation
├── docs/              # Architecture & guides
└── ref/               # Reference materials
```

---

## Documentation

- **Architecture:** [docs/architecture/](docs/architecture/)
- **PRD:** [ref/hackathon-prd.md](ref/hackathon-prd.md)
- **Nautilus Feasibility:** [NAUTILUS_FEASIBILITY.md](NAUTILUS_FEASIBILITY.md)
- **Contributing:** [CONTRIBUTING.md](CONTRIBUTING.md)

---

## Resources

- **Sui:** https://docs.sui.io
- **Nautilus:** https://docs.sui.io/concepts/cryptography/nautilus
- **Seal:** https://docs.mystenlabs.com/seal
- **Walrus:** https://docs.walrus.site
- **Cetus:** https://cetus.zone

---

## Status

🚧 **Hackathon Project** - November 2025

---

## License

Apache-2.0

---

## Team

Built by Nikola & Max
