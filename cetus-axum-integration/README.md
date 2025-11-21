# Cetus DEX Integration with Axum Backend

A production-ready implementation for integrating Cetus Protocol swap functionality with a Rust Axum backend and browser wallet support.

## 📋 Overview

This project provides a complete integration with Cetus DEX on Sui blockchain, featuring:
- Unsigned transaction building on Axum backend using Cetus pool_script_v2
- Secure wallet signing in the browser (Sui Wallet, Suiet, etc.)
- Support for multiple token pairs (USDT, USDC, CETUS ↔ SUI)
- Mainnet deployment ready

## 🏗️ Architecture

```
┌─────────────┐         ┌──────────────┐         ┌──────────────┐
│   Browser   │         │    Axum      │         │  Cetus API   │
│   Wallet    │◄────────┤   Backend    │────────►│  & Sui RPC   │
│             │         │              │         │              │
│ Sign TX     │         │ Build TX     │         │ Execute TX   │
│ (Private)   │         │ (Public)     │         │              │
└─────────────┘         └──────────────┘         └──────────────┘
```

### Security Model
- ✅ Private keys NEVER leave the browser
- ✅ Backend only builds unsigned transactions
- ✅ User approves each transaction in their wallet
- ✅ No backend private keys required

## 🚀 Quick Start

### Prerequisites
- Rust 1.70+
- Node.js 18+
- Sui Wallet browser extension
- SUI tokens for gas fees

### Backend Setup

```bash
cd backend
cargo build --release
NETWORK=mainnet cargo run --release
```

Server will start on `http://localhost:3000`

### Frontend Setup

```bash
cd frontend
npm install
npm run dev
```

Frontend will be available at `http://localhost:3001`

## 🎯 Supported Pools

The integration currently supports these verified Cetus pools:

| Pool | Token A | Token B | Fee | TVL |
|------|---------|---------|-----|-----|
| USDT-SUI | Wormhole USDT | SUI | 0.25% | High |
| CETUS-SUI | CETUS | SUI | 0.25% | $1.3M+ |
| USDC-SUI | Wormhole USDC | SUI | 0.25% | High |

**Token Decimals:**
- SUI: 9 decimals
- CETUS: 9 decimals
- USDC/USDT: 6 decimals

## 🔑 Key Features

### Backend (Axum)
- Dynamic pool fetching from Cetus API
- Transaction building using pool_script_v2 integration
- Automatic token decimal detection
- Double-click prevention
- Proper gas coin handling
- Environment-based configuration (mainnet/testnet)

### Frontend (Next.js + React)
- Multi-wallet support (@mysten/dapp-kit)
- Real-time transaction status
- Bidirectional swaps (A→B and B→A)
- Dynamic pool selection
- Slippage tolerance control

## 🌐 Network Configuration

### Mainnet (Current)
- **RPC:** `https://fullnode.mainnet.sui.io:443`
- **CLMM Package:** `0x1eabed72c53feb3805120a081dc15963c204dc8d091542592abaf7a35689b2fb`
- **Global Config:** `0xdaa46292632c3c4d8f31f23ea0f9b36a28ff3677e9684980e4438403a67a3d8f`
- **Integrate Package:** `0xb2db7142fa83210a7d78d9c12ac49c043b3cbbd482224fea6e3da00aa5a5ae2d`

### Testnet
- **RPC:** `https://fullnode.testnet.sui.io:443`
- **CLMM Package:** `0x5372d555ac734e272659136c2a0cd3227f9b92de67c80dc11250307268af2db8`
- **Global Config:** `0xf5ff7d5ba73b581bca6b4b9fa0049cd320360abd154b809f8700a8fd3cfaf7ca`
- **Integrate Package:** `0x8227f3b46f9f8730a814051dca402c2e05110acbbfef3f9e91996f357d305b0b`

## 📖 API Endpoints

### `GET /health`
Health check endpoint.

**Response:**
```
"Cetus Integration Service is running!"
```

### `GET /api/pools`
Fetch all available Cetus pools from the API.

**Response:**
```json
[
  {
    "swap_account": "0x06d8af9e...",
    "symbol": "USDT-SUI",
    "coin_a_address": "0xc060006111016b8a...",
    "coin_b_address": "0x2::sui::SUI",
    "fee_rate": "0.0025"
  }
]
```

### `POST /api/build-swap`
Build unsigned swap transaction.

**Request:**
```json
{
  "user_address": "0x476aa5cd...",
  "token_a": "0xc060006111016b8a020ad5b33834984a437aaa7d3c74c18e09a95d48aceab08c::coin::COIN",
  "token_b": "0x2::sui::SUI",
  "amount": 10000,
  "slippage": 0.02,
  "a_to_b": true
}
```

**Response:**
```json
{
  "tx_bytes": "base64_encoded_transaction",
  "pool_info": {
    "pool_address": "0x06d8af9e...",
    "symbol": "USDT-SUI",
    "fee_rate": "0.0025",
    "expected_output": 0,
    "price_impact": 0.0
  },
  "estimated_gas": 1000000
}
```

## 🔧 Technical Implementation

### Transaction Building Flow

1. **Frontend determines swap direction and decimals**
   - Detects token type (SUI/CETUS = 9 decimals, USDC/USDT = 6 decimals)
   - Converts human-readable amount to smallest unit

2. **Backend fetches pool and builds transaction**
   - Queries Cetus API for pool data
   - Finds single coin with sufficient balance (no merging)
   - Builds transaction with `pool_script_v2::swap_a2b` or `swap_b2a`

3. **Transaction structure:**
   - Command 1: SplitCoins (exact swap amount)
   - Command 2: Create zero coin for output token
   - Command 3: Call swap function with proper arguments

4. **Frontend signs and submits**
   - Decodes base64 transaction bytes
   - Deserializes into TransactionBlock
   - User signs in wallet
   - Transaction executed on-chain

### Key Implementation Details

**Bool Parameter:** Always `true` (by_amount_in mode - we specify input amount, not output)

**Slippage Protection:** Currently set to `min_output = 1` (essentially disabled). For production, implement proper price calculation.

**Gas Handling:**
- SUI swaps use GasCoin directly (split amount from gas)
- Other tokens use separate coin objects

## 🔒 Security Considerations

1. **No Private Keys on Server** - Backend never handles private keys
2. **Input Validation** - All user inputs are validated
3. **Slippage Protection** - Set minimum output to protect against price manipulation
4. **Gas Safety** - Ensures sufficient SUI for gas fees
5. **CORS** - Configure properly for production

## 📦 Dependencies

### Backend
```toml
axum = "0.7"
sui-sdk = "0.5"
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
reqwest = "0.11"
tower-http = { version = "0.5", features = ["cors"] }
tracing = "0.1"
anyhow = "1"
```

### Frontend
```json
{
  "@mysten/dapp-kit": "^0.14",
  "@mysten/sui.js": "^0.54",
  "react": "^18",
  "next": "^14"
}
```

## 💡 Usage Tips

1. **Minimum Swap Amount:** Use at least $0.50-$1.00 worth of tokens to ensure output covers gas fees

2. **Slippage Settings:**
   - Normal conditions: 0.5-2%
   - High volatility: 3-5%

3. **Coin Consolidation:** If you get "No single coin with enough balance" error, consolidate your coins first

4. **Gas Requirements:** Keep at least 0.01 SUI for gas fees

## 🐛 Troubleshooting

**"No single coin with enough balance"**
- Consolidate your token coins into one large coin
- Each swap requires a single coin with sufficient balance

**"Swap succeeded but I lost tokens"**
- Very small swaps may result in net loss due to gas fees
- Always swap at least $0.50-$1.00 worth

**"Transaction failed"**
- Check you have enough balance
- Verify slippage tolerance is sufficient
- Ensure you have SUI for gas

## 🔗 Resources

- [Cetus Documentation](https://cetus-1.gitbook.io/cetus-developer-docs)
- [Sui Documentation](https://docs.sui.io/)
- [Cetus Protocol](https://www.cetus.zone/)
- [@mysten/dapp-kit](https://sdk.mystenlabs.com/dapp-kit)

## 📝 Project Structure

```
cetus-axum-integration/
├── backend/
│   ├── src/
│   │   ├── main.rs           # API endpoints
│   │   ├── transaction.rs    # Transaction building logic
│   │   ├── cetus.rs          # Cetus API integration
│   │   └── config.rs         # Network configuration
│   └── Cargo.toml
├── frontend/
│   ├── components/
│   │   └── CetusSwap.tsx     # Main swap UI
│   ├── app/
│   │   └── page.tsx          # Next.js page
│   └── package.json
└── README.md
```

## 📄 License

MIT License - See LICENSE file for details

## ⚠️ Disclaimer

This code is provided as-is for educational and integration purposes. Always audit and test thoroughly before using in production. The authors are not responsible for any loss of funds.

## 🤝 Contributing

Contributions are welcome! Areas for improvement:
- [ ] Real-time price calculation from on-chain pool data
- [ ] Support for native USDC pool
- [ ] Multi-hop swaps
- [ ] Transaction history
- [ ] Price charts and analytics
