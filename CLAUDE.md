# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Mist Protocol is a privacy-preserving DeFi swap protocol on Sui blockchain that uses SEAL threshold encryption for hiding swap amounts, Nautilus TEE (AWS Nitro Enclaves) for trusted execution, and an intent-based architecture for asynchronous swap processing.

## Architecture

```
Frontend (Next.js) → Sui Blockchain → Nautilus Backend (Rust/TEE) → tx-signer
```

**Key Components:**
- **Frontend** (`frontend/`): Next.js 14 with @mysten/dapp-kit for wallet connection and SEAL SDK for encryption
- **Smart Contracts** (`contracts/mist_protocol/`): Sui Move contracts for LiquidityPool, VaultEntry, and IntentQueue
- **Backend** (`backend/`): Rust/Axum server running in TEE that polls IntentQueue every 5 seconds, decrypts with SEAL, executes swaps
- **Signing Service** (`tx-signer/`): Separate Rust service that wraps `sui keytool sign` to solve fastcrypto version conflicts

**Why tx-signer exists:** SEAL SDK uses fastcrypto v1, sui-types uses fastcrypto v2. These are incompatible in the same binary, so signing is delegated to a separate service.

## Build & Run Commands

### Frontend
```bash
cd frontend
pnpm install
pnpm dev          # Development server on :3000
pnpm build        # Production build
pnpm lint         # ESLint
```

### Backend
```bash
cd backend
cargo build --features mist-protocol
cargo run --features mist-protocol    # Runs on :3001
RUST_LOG=info cargo run --features mist-protocol  # With logging
```

### Signing Service
```bash
cd tx-signer
cargo build
cargo run         # Runs on :4000 (localhost only)
```

### Contract Deployment
```bash
cd contracts/mist_protocol
sui client publish --gas-budget 500000000
# Save package_id, pool_id, queue_id from output
```

## Running All Services (3 terminals)

```bash
# Terminal 1: Signing service
cd tx-signer && cargo run

# Terminal 2: Backend
cd backend && cargo run --features mist-protocol

# Terminal 3: Frontend
cd frontend && pnpm dev
```

## Configuration Files

- `backend/.env`: `BACKEND_PRIVATE_KEY`, `SUI_RPC_URL`
- `backend/src/apps/mist-protocol/seal_config.yaml`: Contract IDs, SEAL key servers
- `frontend/.env.local`: `NEXT_PUBLIC_PACKAGE_ID`, `NEXT_PUBLIC_POOL_ID`, `NEXT_PUBLIC_INTENT_QUEUE_ID`, `NEXT_PUBLIC_NETWORK`

## Key Implementation Details

### Backend Authorization
The backend address is hardcoded in `contracts/mist_protocol/sources/seal_policy.move`:
```move
const BACKEND_ADDRESS: address = @0x...;
```
The imported key's address must match this constant.

### SEAL Encryption ID Format
`vault_id (32 bytes) + random_nonce (5 bytes)` - embedded in encrypted objects, allows both user and TEE to decrypt.

### Intent Processing Loop
Backend polls `IntentQueue` every 5 seconds → SEAL decrypt (2-of-3 threshold) → execute swap → SEAL re-encrypt output → sign via tx-signer → submit transaction.

## Directory Structure

```
backend/src/apps/mist-protocol/
├── intent_processor.rs   # Polls IntentQueue
├── swap_executor.rs      # Executes swaps (mock: SUI→SUI)
├── seal_encryption.rs    # SEAL encrypt/decrypt
└── seal_config.yaml      # Configuration

frontend/
├── app/page.tsx          # Main UI
├── components/           # SwapCard, WrapCard, UnwrapCard, etc.
├── hooks/useVault.ts     # Vault management hook
└── lib/seal-vault.ts     # SEAL encryption helpers

contracts/mist_protocol/sources/
├── mist_protocol.move    # Main protocol logic
└── seal_policy.move      # Vault + encrypted tickets
```

## Tech Stack

- **Contracts**: Sui Move (2024.beta)
- **Backend**: Rust nightly, Axum 0.7, SEAL SDK, Nautilus
- **Signing Service**: Rust stable, Axum 0.7, sui keytool
- **Frontend**: Next.js 14, TypeScript, @mysten/dapp-kit ^0.14.0, @mysten/seal ^0.9.4, Tailwind CSS

## Current Status

**Working:** Deposit/wrap tokens, create swap intents, TEE processing, SEAL encryption/decryption, tx signing, output tickets, unwrap tokens.

**Mock:** Cetus swap integration (currently SUI → SUI pass-through in `swap_executor.rs`).
