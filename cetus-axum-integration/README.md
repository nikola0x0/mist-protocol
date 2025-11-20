# Cetus DEX Integration with Axum Backend

A complete guide and implementation for integrating Cetus Protocol swap functionality into a Rust Axum backend with browser wallet support.

## 📋 Overview

This project demonstrates how to:
- Build unsigned swap transactions on an Axum backend
- Let users sign transactions with their browser wallets (Sui Wallet, Suiet, etc.)
- Submit signed transactions to the Sui blockchain
- Support both Mainnet and Testnet

## 🏗️ Architecture

```
┌─────────────┐         ┌──────────────┐         ┌──────────────┐
│   Browser   │         │    Axum      │         │     Sui      │
│   Wallet    │◄────────┤   Backend    │────────►│  Blockchain  │
│             │         │              │         │              │
│ Sign TX     │         │ Build TX     │         │ Execute TX   │
│ (Private)   │         │ (Public)     │         │              │
└─────────────┘         └──────────────┘         └──────────────┘
```

### Security Model
- ✅ Private keys NEVER leave the browser
- ✅ Backend only builds unsigned transactions
- ✅ User approves each transaction in their wallet
- ✅ Backend can submit or frontend submits directly

## 🚀 Quick Start

### Prerequisites
- Rust 1.70+
- Node.js 18+ (for frontend)
- Sui Wallet browser extension

### Backend Setup

```bash
cd backend
cargo build
NETWORK=mainnet cargo run
```

### Frontend Setup

```bash
cd frontend
npm install
npm run dev
```

## 📚 Documentation

- [Backend Guide](./docs/BACKEND.md) - Axum server implementation
- [Frontend Guide](./docs/FRONTEND.md) - Browser wallet integration
- [API Reference](./docs/API.md) - Endpoint documentation
- [Cetus Protocol](./docs/CETUS.md) - Protocol details and addresses

## 🔑 Key Features

### Backend (Axum)
- Fetch Cetus pools from API
- Build unsigned swap transactions
- Calculate slippage and price impact
- Environment-based configuration (mainnet/testnet)
- Transaction submission endpoint

### Frontend
- Connect browser wallets (Sui Wallet, Suiet)
- Request unsigned transactions from backend
- Sign transactions securely in wallet
- Submit to blockchain

## 🌐 Networks

### Mainnet
- RPC: `https://fullnode.mainnet.sui.io:443`
- CLMM Package: `0x1eabed72c53feb3805120a081dc15963c204dc8d091542592abaf7a35689b2fb`
- Global Config: `0xdaa46292632c3c4d8f31f23ea0f9b36a28ff3677e9684980e4438403a67a3d8f`

### Testnet
- RPC: `https://fullnode.testnet.sui.io:443`
- CLMM Package: `0x5372d555ac734e272659136c2a0cd3227f9b92de67c80dc11250307268af2db8`
- Global Config: `0xf5ff7d5ba73b581bca6b4b9fa0049cd320360abd154b809f8700a8fd3cfaf7ca`

## 📖 API Endpoints

### `GET /health`
Health check endpoint.

### `GET /api/pools`
Fetch all available Cetus pools.

### `POST /api/pool/info`
Get specific pool information by token pair.

### `POST /api/build-swap`
Build unsigned swap transaction.

### `POST /api/submit-signed`
Submit signed transaction to blockchain.

## 🔒 Security Notes

1. **Never store private keys on the server**
2. **Always validate user inputs**
3. **Implement rate limiting**
4. **Use HTTPS in production**
5. **Validate signed transactions before submission**

## 📦 Dependencies

### Backend
- `axum` - Web framework
- `sui-sdk` - Sui blockchain SDK
- `tokio` - Async runtime
- `serde` - Serialization
- `reqwest` - HTTP client

### Frontend
- `@mysten/dapp-kit` - Sui wallet integration
- `@mysten/sui.js` - Sui TypeScript SDK
- `react` - UI framework

## 🔗 Resources

- [Cetus Documentation](https://cetus-1.gitbook.io/cetus-developer-docs)
- [Sui Documentation](https://docs.sui.io/)
- [Cetus GitHub](https://github.com/CetusProtocol)
- [Sui Rust SDK](https://docs.sui.io/references/rust-sdk)

## 📄 License

MIT License - See LICENSE file for details

## 🤝 Contributing

Contributions welcome! Please read CONTRIBUTING.md first.

## ⚠️ Disclaimer

This is educational code. Always audit and test thoroughly before using in production.
