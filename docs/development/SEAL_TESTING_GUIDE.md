# Real SEAL Testing Guide

## Contract Deployed

**Package ID**: `0xb55a45cb2100b347c68528c4f05ee378c6bdd89df281935f7ee042b159ccad74`

**Modules**:
- `mist_protocol` - Vault + ticket system with Mist Pool
- `seal_policy` - User + TEE decryption (seal_approve_user, seal_approve_tee)

**Explorer**: https://testnet.suivision.xyz/package/0xb55a45cb2100b347c68528c4f05ee378c6bdd89df281935f7ee042b159ccad74

**Architecture**: Per-user vaults with encrypted tickets. TEE wallet executes swaps.

## How to Test

### 1. Restart Frontend

```bash
# Kill old frontend
pkill -f "next dev"

# Start with new environment
cd frontend
npm run dev
```

### 2. Open Test Page

Navigate to: **http://localhost:3000/seal-test**

### 3. Test Flow

**Step 1: Connect Wallet**

- Click "Connect Wallet"
- Connect your Sui wallet

**Step 2: Create Vault**

- Click "Create Vault" button
- Sign transaction in wallet
- Vault ID will auto-populate
- This creates your SEAL namespace

**Step 3: Encrypt Ticket Amount**

- Enter amount: `100000000` (100 USDC)
- Click "🔐 Encrypt with SEAL"
- Check logs for success

**Step 4: Decrypt Ticket (User Flow)**

- Click "🔓 Decrypt with SEAL (User)"
- Sign personal message in wallet (creates session key)
- Session valid for 10 minutes
- Verify decrypted amount matches original

## What Each Step Does

### Create Vault

```typescript
Transaction: seal_policy::create_vault_entry()

Creates:
  Vault {
    id: 0x... (SEAL namespace)
    owner: <your address>
    tickets: {}  // Empty initially
  }

Result: Vault ready for encrypted tickets
```

### Encrypt Ticket Amount

```typescript
// Generate encryption ID (vault namespace + nonce)
vault_id + random_nonce → encryption_id

// Encrypt ticket amount with SEAL
SealClient.encrypt({
  threshold: 2,              // 2-of-3 key servers
  packageId: PACKAGE_ID,
  id: encryption_id,
  data: "100000000"          // Ticket amount (100 USDC)
})

Result: Encrypted ticket amount for vault storage
```

### Decrypt Ticket (User)

```typescript
// Create session key (sign once, valid 10 min)
SessionKey.create() → sign personal message

// Build seal_approve_user transaction
seal_policy::seal_approve_user(
  encryption_id,
  vault_id    // Proves ownership
)

// Call SEAL key servers
SealClient.decrypt(encrypted, signedTx)

Result: "100000000" (ticket amount)
```

## Expected Logs

### Successful Encryption:

```
[12:34:56] 🔐 Encrypting with SEAL...
[12:34:56]    Encryption ID: 0x19fab12db7083c9690f09da125157d1e...
[12:34:57] ✅ Encrypted successfully!
[12:34:57]    Key servers: 3
[12:34:57]    Length: 2847 chars
```

### Successful Decryption:

```
[12:35:01] 🔓 Decrypting with SEAL (user)...
[12:35:01]    Creating session key...
[12:35:02]    📝 Requesting signature...
[12:35:05]    ✅ Session key created (valid 10 min)
[12:35:05]    Encryption ID: 0x19fab12db7083c9690f09da125157d1e...
[12:35:05]    Building seal_approve transaction...
[12:35:06]    ✅ Transaction signed
[12:35:06]    Calling SEAL key servers...
[12:35:08] ✅ Decrypted: 100000000
[12:35:08]    Original: 100000000, Decrypted: 100000000
[12:35:08] 🎉 Perfect match!
```

## Environment Variables

Created in `frontend/.env.local`:

```bash
NEXT_PUBLIC_PACKAGE_ID=0x19fab12db7083c9690f09da125157d1e3f6659438cf395fcb997abb571631439
NEXT_PUBLIC_NETWORK=testnet
NEXT_PUBLIC_POOL_ID=0x02e5e3be7907ce360ae6eb9da340e968551ae62512700788e5c65d739599ae1d
```

**Add after testing**:

```bash
NEXT_PUBLIC_VAULT_ID=0x... (from vault creation)
NEXT_PUBLIC_ENCLAVE_ID=0x... (from backend deployment)
```

## What We Implemented

### Frontend (`app/seal-test/page.tsx`):

- ✅ Real SEAL client initialization
- ✅ Vault creation with transaction
- ✅ Real SEAL encryption (uses vault namespace)
- ✅ Real SEAL decryption (with session key)
- ✅ Session key management (sign once, valid 10 min)
- ✅ Error handling and logging

### Smart Contract Updates:

- ✅ VaultEntry struct (user namespace)
- ✅ seal_approve (user OR TEE access)
- ✅ Namespace validation
- ✅ create_vault_entry function

### Library (`lib/seal-vault.ts`):

- ✅ encryptVaultBalance helper
- ✅ decryptVaultBalance helper
- ✅ Format helpers

## Testing Without Enclave

You can test **encryption** right now without backend:

1. Create vault ✅
2. Encrypt amount ✅
3. See encrypted object ✅

For **decryption**, you need:

- Enclave object ID from Nautilus backend deployment
- Or use a placeholder for testing (will fail SEAL but tests the flow)

## Next Steps

1. **Test encryption now**: Create vault → encrypt → verify logs
2. **Deploy Nautilus enclave**: Get enclave object ID
3. **Test decryption**: Add enclave ID → decrypt → verify match
4. **Backend integration**: TEE decrypts with same seal_approve

---

**Your test page is ready!** Open http://localhost:3001/seal-test and create a vault! 🚀
