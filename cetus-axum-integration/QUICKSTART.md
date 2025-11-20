# Quick Start Guide

Get up and running with Cetus swap integration in 5 minutes!

## Prerequisites

- **Rust** 1.70+ ([Install](https://rustup.rs/))
- **Node.js** 18+ ([Install](https://nodejs.org/))
- **Sui Wallet** browser extension ([Chrome](https://chrome.google.com/webstore/detail/sui-wallet))

## Step 1: Backend Setup (2 minutes)

```bash
# Navigate to backend
cd backend

# Build the project
cargo build

# Run the server (mainnet)
cargo run

# Or for testnet
NETWORK=testnet cargo run
```

You should see:
```
🚀 Server running on http://0.0.0.0:3000
```

Test it:
```bash
curl http://localhost:3000/health
# Should return: "Cetus Integration Service is running!"
```

## Step 2: Frontend Setup (2 minutes)

```bash
# Navigate to frontend (in a new terminal)
cd frontend

# Install dependencies
npm install

# Run development server
npm run dev
```

Visit http://localhost:3000

## Step 3: Execute Your First Swap (1 minute)

1. **Connect Wallet**
   - Click "Connect Wallet"
   - Select Sui Wallet
   - Approve connection

2. **Load Pools**
   - Click "Load Pools"
   - Select a pool (e.g., SUI-USDC)

3. **Swap**
   - Enter amount (try 0.1 SUI first)
   - Set slippage (default 0.5% is fine)
   - Click "Swap"
   - Approve in wallet popup

4. **Done!**
   - Transaction digest will appear
   - Check on [Sui Explorer](https://suiexplorer.com)

## Architecture Overview

```
Browser Wallet (Signs) → React App → Axum Backend → Sui Blockchain
                           ↓
                     (Unsigned TX)
```

## Testing on Testnet

### Backend
```bash
cd backend
NETWORK=testnet cargo run
```

### Frontend
Update in `components/CetusSwap.tsx`:
```typescript
// Change wallet provider network
<SuiClientProvider defaultNetwork="testnet">
```

### Get Test Tokens
Visit [Sui Faucet](https://faucet.sui.io/) for testnet SUI

## Common Issues

### "Connection refused"
- Make sure backend is running on port 3000
- Check with: `curl http://localhost:3000/health`

### "Failed to fetch pools"
- Check internet connection
- Verify backend is running
- Check logs: `RUST_LOG=debug cargo run`

### "Wallet not detected"
- Install Sui Wallet extension
- Refresh browser
- Try in incognito mode (disable other extensions)

### "Transaction failed"
- Check if you have enough SUI for gas
- Verify you're on correct network (mainnet vs testnet)
- Try smaller amount

## Next Steps

- Read [Backend Guide](./docs/BACKEND.md) for detailed backend info
- Read [Frontend Guide](./docs/FRONTEND.md) for wallet integration
- Read [Cetus Protocol](./docs/CETUS.md) for protocol details
- Check [API Reference](./docs/API.md) for endpoint documentation

## Production Deployment

### Backend

```bash
# Build release
cargo build --release

# Run
./target/release/cetus-axum-backend
```

### Frontend

```bash
# Build
npm run build

# Serve
npm start
```

### Environment Variables

```bash
# Backend
export NETWORK=mainnet
export PORT=3000

# Frontend
export NEXT_PUBLIC_BACKEND_URL=https://api.yourdomain.com
```

## Security Reminders

✅ **Private keys stay in browser wallet**
✅ **Backend never sees private keys**
✅ **User approves each transaction**
✅ **Always use HTTPS in production**

## Support

- **Documentation**: Check `/docs` folder
- **Issues**: GitHub Issues
- **Cetus Dev**: https://t.me/CetusDevNews
- **Sui Discord**: https://discord.gg/sui

## Example Commands

```bash
# Check backend health
curl http://localhost:3000/health

# Get all pools
curl http://localhost:3000/api/pools

# Get specific pool info
curl -X POST http://localhost:3000/api/pool/info \
  -H "Content-Type: application/json" \
  -d '{
    "token_a": "0x2::sui::SUI",
    "token_b": "0x..."
  }'
```

## Congratulations! 🎉

You now have a working Cetus swap integration!

Try swapping different tokens and explore the code to customize it for your needs.
