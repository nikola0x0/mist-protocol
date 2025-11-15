# Mist Protocol - Contract Testing Guide

## Contract Deployment Info

**Package ID:**

```
0xbfea609e22c96e824ec0d9fa32d21c825823117283ab9a418e6059b46da10fce
```

**LiquidityPool (Shared Object):**

```
0x473abdf90f498c59d1bad048e7426c6e9ca8c1c53e55fb5ea8b8d620a3d94617
```

**AdminCap:**

```
0x7dde20a1d8d69352df58a0cc623b73bc47322ebc72ff2e67c98d0b3681d81dcf
```

**Deployer Address:**

```
0x476aa5cda4a10276eb02d9b38e148c5186915cd47c5dffbf1ef14d4af3083263
```

**Transaction:**

```
https://testnet.suivision.xyz/txblock/M51cLnJCQbhYKqFnK7ZmA8iCU5aPc1kXZmxdEYfoxTS
```

**Explorer:**

```
https://testnet.suivision.xyz/package/0xbfea609e22c96e824ec0d9fa32d21c825823117283ab9a418e6059b46da10fce
```

---

## Test 1: Wrap SUI → eSUI

### Command:

```bash
sui client call \
  --package 0xbfea609e22c96e824ec0d9fa32d21c825823117283ab9a418e6059b46da10fce \
  --module mist_protocol \
  --function wrap_sui \
  --args \
    0x473abdf90f498c59d1bad048e7426c6e9ca8c1c53e55fb5ea8b8d620a3d94617 \
    <SUI_COIN_OBJECT_ID> \
    "[0x01, 0x02, 0x03]" \
  --gas-budget 10000000
```

### What you need:

- **Pool ID**: `0x473abdf90f498c59d1bad048e7426c6e9ca8c1c53e55fb5ea8b8d620a3d94617`
- **SUI Coin**: Get one using: `sui client gas`
- **Encrypted pointer**: Mock bytes `[0x01, 0x02, 0x03]`

### Expected Result:

- ✅ eSUI token created
- ✅ SUI locked in pool
- ✅ WrapEvent emitted

### Get your SUI coins:

```bash
sui client gas
```

---

## Test 2: Check Your Objects

### Command:

```bash
sui client objects
```

### What to look for:

- **eSUI token**: Type `...::EncryptedSUI`
- **AdminCap**: `0x7dde20a1d8d69352df58a0cc623b73bc47322ebc72ff2e67c98d0b3681d81dcf`

---

## Test 3: Request Swap (eSUI → eUSDC)

**Prerequisites:** You need both eSUI and eUSDC tokens first

### Command:

```bash
sui client call \
  --package 0xbfea609e22c96e824ec0d9fa32d21c825823117283ab9a418e6059b46da10fce \
  --module mist_protocol \
  --function request_swap_sui_to_usdc \
  --args \
    <ESUI_OBJECT_ID> \
    <EUSDC_OBJECT_ID> \
    "[0x05, 0x06, 0x07]" \
  --gas-budget 10000000
```

### Expected Result:

- ✅ SwapRequestEvent emitted
- ✅ TEE can listen for this event
- ✅ No state changes yet (waiting for TEE)

---

## Test 4: Unwrap eSUI → SUI

### Command:

```bash
sui client call \
  --package 0xbfea609e22c96e824ec0d9fa32d21c825823117283ab9a418e6059b46da10fce \
  --module mist_protocol \
  --function unwrap_sui \
  --args \
    0x473abdf90f498c59d1bad048e7426c6e9ca8c1c53e55fb5ea8b8d620a3d94617 \
    <ESUI_OBJECT_ID> \
    1000000000 \
    0x476aa5cda4a10276eb02d9b38e148c5186915cd47c5dffbf1ef14d4af3083263 \
  --gas-budget 10000000
```

### Parameters:

- **pool**: LiquidityPool ID
- **esui**: Your eSUI token ID
- **amount**: 1000000000 (1 SUI in MIST)
- **recipient**: Your address

### Expected Result:

- ✅ eSUI token burned
- ✅ SUI sent to recipient
- ✅ UnwrapEvent emitted

---

## Test 5: Transfer eSUI

### Command:

```bash
sui client call \
  --package 0xbfea609e22c96e824ec0d9fa32d21c825823117283ab9a418e6059b46da10fce \
  --module mist_protocol \
  --function transfer_esui \
  --args \
    <ESUI_OBJECT_ID> \
    <RECIPIENT_ADDRESS> \
  --gas-budget 10000000
```

### Expected Result:

- ✅ eSUI transferred to recipient
- ✅ Ownership changed

---

## Test 6: Admin Functions

### Update TEE Authority:

```bash
sui client call \
  --package 0xbfea609e22c96e824ec0d9fa32d21c825823117283ab9a418e6059b46da10fce \
  --module mist_protocol \
  --function update_tee_authority \
  --args \
    0x473abdf90f498c59d1bad048e7426c6e9ca8c1c53e55fb5ea8b8d620a3d94617 \
    0x7dde20a1d8d69352df58a0cc623b73bc47322ebc72ff2e67c98d0b3681d81dcf \
    <NEW_TEE_ADDRESS> \
  --gas-budget 10000000
```

### Pause/Unpause:

```bash
sui client call \
  --package 0xbfea609e22c96e824ec0d9fa32d21c825823117283ab9a418e6059b46da10fce \
  --module mist_protocol \
  --function set_pause \
  --args \
    0x473abdf90f498c59d1bad048e7426c6e9ca8c1c53e55fb5ea8b8d620a3d94617 \
    0x7dde20a1d8d69352df58a0cc623b73bc47322ebc72ff2e67c98d0b3681d81dcf \
    true \
  --gas-budget 10000000
```

---

## Helper Commands

### Check Pool Balance:

```bash
sui client object 0x473abdf90f498c59d1bad048e7426c6e9ca8c1c53e55fb5ea8b8d620a3d94617
```

### Get All Your Objects:

```bash
sui client objects
```

### Get Transaction Details:

```bash
sui client tx-block <TX_DIGEST>
```

### Watch Events:

```bash
sui client events --package 0xbfea609e22c96e824ec0d9fa32d21c825823117283ab9a418e6059b46da10fce
```

---

## Testing Workflow

### Basic Flow Test:

1. ✅ **Deploy** - Done!
2. **Wrap** - Lock 1 SUI → get eSUI
3. **Check** - Verify eSUI created
4. **Request Swap** - Emit event (TEE will process)
5. **Unwrap** - Burn eSUI → get SUI back

### Integration Test (with Nikola):

1. User wraps SUI → eSUI
2. User requests swap (eSUI → eUSDC)
3. **Nikola's TEE**:
   - Listens for SwapRequestEvent
   - Decrypts amount
   - Calls Cetus DEX
   - Calls `update_after_swap_sui_to_usdc()`
4. User unwraps eUSDC → USDC

---

## Known Limitations (MVP)

1. **Mock Encryption**: Pointers are just bytes `[0x01, 0x02, 0x03]`

   - Real: Should be Seal encrypted ciphertext
   - TODO: Integrate Seal

2. **No USDC Faucet**: Testing USDC harder on testnet

   - Alternative: Test with SUI only
   - TODO: Get testnet USDC

3. **TEE Integration**: Need Nikola's Nautilus
   - Manual testing: Use your address as TEE authority
   - TODO: Deploy Nautilus enclave

---

## Troubleshooting

### Error: "Insufficient gas"

```bash
# Increase gas budget
--gas-budget 50000000
```

### Error: "Object not found"

```bash
# Check object still exists
sui client object <OBJECT_ID>
```

### Error: "Shared object version mismatch"

```bash
# Wait a few seconds and retry
# Pool is shared, might be busy
```

### Get more testnet SUI:

```bash
# Discord faucet
# https://discord.com/channels/916379725201563759/971488439931392130
```

---

## Next Steps

1. ✅ Contract deployed
2. ⏳ Test wrap/unwrap manually
3. ⏳ Share contract addresses with Nikola
4. ⏳ Integrate with frontend
5. ⏳ Add real Seal encryption
6. ⏳ Connect Nikola's TEE

---

## Contract Addresses (Save These!)

```bash
# Add to .env or config file
PACKAGE_ID=0xbfea609e22c96e824ec0d9fa32d21c825823117283ab9a418e6059b46da10fce
LIQUIDITY_POOL=0x473abdf90f498c59d1bad048e7426c6e9ca8c1c53e55fb5ea8b8d620a3d94617
ADMIN_CAP=0x7dde20a1d8d69352df58a0cc623b73bc47322ebc72ff2e67c98d0b3681d81dcf
```
