# Mist Protocol

**Privacy-preserving intent-based DeFi on Sui**

Built with: Nautilus • Seal • Walrus • Cetus • Sui

---

## Overview

Mist Protocol enables private DeFi transactions through intent-based execution with verifiable TEE computation. Users express trading intents that are processed securely through Nautilus enclaves, executed via escrow contracts, and settled on Cetus DEX—all while keeping transaction amounts encrypted.

### Key Features

- **Intent-Based Trading:** Express trading intents without exposing amounts
- **TEE Verification:** Nautilus-powered trusted execution with on-chain attestation
- **Encrypted Escrow:** Sui Move contracts with encrypted amount storage (eUSDC)
- **DEX Integration:** Automated swap execution on Cetus (USDC/SUI, CETUS/WMNT/FlowX)
- **Decentralized Storage:** Walrus for cost-efficient metadata storage

---

## Architecture

```
User Intent → Nautilus TEE → Wallet → Cetus DEX
                                ↓
                          Escrow Contract (eUSDC)
                                ↓
                          Intent Creation → Backend
                                ↓
                          Execute Tx + Walrus Storage
```

### Components

**Nautilus TEE:** Self-managed AWS Nitro enclaves for verifiable computation
**Escrow Contracts:** Sui Move contracts handling encrypted deposits
**Intent System:** Backend processing layer for DEX interaction
**Walrus Storage:** Decentralized data access layer for transaction metadata

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
