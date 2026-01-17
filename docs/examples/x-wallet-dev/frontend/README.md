# X-Wallet Frontend

X-enabled Sui Wallet - Frontend Application

## Tech Stack

- **Framework**: React 18 + TypeScript
- **Build Tool**: Vite
- **Styling**: Tailwind CSS
- **Routing**: React Router v6
- **Blockchain**: Sui Blockchain
  - `@mysten/dapp-kit` - Sui dApp development kit
  - `@mysten/sui` - Sui SDK
- **State Management**: React Query (@tanstack/react-query)

## Features

### Implemented (UI Placeholder)
- Onboarding page with X login UI
- Dashboard with:
  - Balance overview
  - Quick actions (Deposit, Withdraw, Link Wallet)
  - Tabs: Overview, Transactions, NFTs
- Routing setup
- Sui Wallet Provider configuration
- Dark/Light mode (follows system preference)

### To Be Implemented
- X (Twitter) OAuth integration
- Sui Wallet connection (using dApp Kit)
- XWalletAccount on-chain interaction
- Transfer functionality
- NFT management
- Transaction history
- Backend API integration
- Enclave signature verification

## Project Structure

```
src/
├── components/       # Reusable UI components
├── contexts/         # React Context providers
│   └── AuthContext.tsx
├── hooks/           # Custom React hooks
├── pages/           # Page components
│   ├── Onboarding.tsx
│   └── Dashboard.tsx
├── types/           # TypeScript type definitions
│   └── index.ts
├── utils/           # Utility functions and constants
│   └── constants.ts
├── App.tsx          # Main app with routing
└── main.tsx         # Entry point with providers
```

## Getting Started

### Prerequisites

- Node.js 18+ and npm
- Git

### Installation

1. Clone the repository and navigate to frontend:
```bash
cd frontend
```

2. Install dependencies:
```bash
npm install
```

3. Create environment file:
```bash
cp .env.example .env
```

4. Update `.env` with your configuration:
```env
VITE_API_BASE_URL=http://localhost:3001
VITE_ENCLAVE_URL=http://localhost:3000
VITE_SUI_NETWORK=testnet
VITE_TWITTER_CLIENT_ID=your_x_client_id
```

### Development

Start the development server:
```bash
npm run dev
```

The app will be available at `http://localhost:5173`

### Build

Build for production:
```bash
npm run build
```

Preview production build:
```bash
npm run preview
```

## Configuration

### Sui Network

The app is configured to use Sui Testnet by default. You can change this in:
- `src/main.tsx` - Update `defaultNetwork` prop
- `.env` - Set `VITE_SUI_NETWORK`

### Contract Addresses

Update contract addresses in `.env` after deploying smart contracts:
```env
VITE_XWALLET_ACCOUNT_ADDRESS=0x...
VITE_XWALLET_TRANSFER_ADDRESS=0x...
VITE_XWALLET_ENCLAVE_ADDRESS=0x...
```

## Routing

- `/` - Onboarding page (X login)
- `/dashboard` - User dashboard (protected route)

## Next Steps

1. **Implement X (Twitter) OAuth Flow**
   - Set up X Developer account
   - Configure OAuth 2.0
   - Implement callback handler

2. **Integrate Sui Wallet**
   - Use `@mysten/dapp-kit` hooks
   - Implement wallet connection UI
   - Handle wallet events

3. **Connect to Backend**
   - Create API client utility
   - Implement data fetching hooks
   - Handle authentication tokens

4. **Implement Smart Contract Interactions**
   - Create transaction builders
   - Implement signing flow
   - Handle enclave signatures

5. **Add Transaction Management**
   - Implement transaction submission
   - Add transaction tracking
   - Show transaction history

## Available Scripts

- `npm run dev` - Start development server
- `npm run build` - Build for production
- `npm run preview` - Preview production build
- `npm run lint` - Run ESLint

## Contributing

This is part of the X-Wallet project. Please refer to the main project README for contribution guidelines.
