# SEAL Encryption Setup

## Architecture: Vault + Tickets

**Design**: Each user has a vault with encrypted tickets representing token balances.

**Privacy**: TEE swaps with own wallet → unlinkable to users!

See: `docs/development/VAULT_ARCHITECTURE.md`

## Quick Start

```bash
./test-seal.sh
```

Opens: http://localhost:3000/seal-test

## Contract

**Package**: `0xb55a45cb2100b347c68528c4f05ee378c6bdd89df281935f7ee042b159ccad74`

**Functions**:
- `seal_approve_user` - User decrypts their tickets
- `seal_approve_tee` - TEE decrypts for swaps
- `create_vault_entry` - Creates user vault

## What SEAL Encrypts

**Ticket amounts** (the "xxx" in your flow):
```
ticket_Y: xxx eUSDC  ← "xxx" is SEAL encrypted!
ticket_Z: xxx eSUI   ← "xxx" is SEAL encrypted!
```

## Test Flow

1. Connect wallet
2. Create vault
3. Encrypt ticket amount (e.g., "100000000")
4. Decrypt with user wallet
5. Verify match

## Privacy: Per-User Vault vs Single Vault

**Answer**: Same privacy, but per-user vaults perform better!

Both designs:
- ✅ Hide amounts (SEAL encrypted)
- ⚠️ Show user has tickets
- ✅ TEE swaps unlinkable to users

## Ports

- Frontend: 3000
- Backend-seal: 3001
