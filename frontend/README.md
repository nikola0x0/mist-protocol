# Mist Protocol Frontend

Privacy-preserving DeFi interface built with Next.js 14 and Sui.

## Tech Stack

- **Next.js 14** with App Router
- **TypeScript** for type safety
- **Tailwind CSS** for styling
- **@mysten/dapp-kit** for Sui wallet integration
- **@tanstack/react-query** for data fetching

## Getting Started

### Install Dependencies

```bash
pnpm install
```

### Run Development Server

```bash
pnpm dev
```

Open [http://localhost:3000](http://localhost:3000) in your browser.

### Build for Production

```bash
pnpm build
pnpm start
```

## Project Structure

```
frontend/
├── app/
│   ├── layout.tsx       # Root layout with providers
│   ├── page.tsx         # Home page
│   ├── providers.tsx    # Sui wallet providers
│   └── globals.css      # Global styles
├── components/
│   └── ConnectButton.tsx # Wallet connection button
└── lib/                 # Utilities and helpers
```

## Features

- ✅ Sui wallet integration (@mysten/dapp-kit)
- ✅ Testnet configuration (ready for mainnet)
- ✅ Responsive design with Tailwind CSS
- ✅ TypeScript strict mode
- ⏳ Intent-based trading UI (coming soon)
- ⏳ Encrypted escrow integration (coming soon)

## Environment Variables

Create a `.env.local` file:

```bash
# Add environment variables here as needed
# NEXT_PUBLIC_ESCROW_CONTRACT_ADDRESS=
# NEXT_PUBLIC_BACKEND_URL=
```

## Testing Wallet Connection

1. Install [Sui Wallet](https://chrome.google.com/webstore/detail/sui-wallet) browser extension
2. Create a testnet wallet
3. Get testnet SUI from [faucet](https://discord.com/channels/916379725201563759/971488439931392130)
4. Connect wallet using the "Connect Wallet" button

## Next Steps

- [ ] Add escrow deposit UI
- [ ] Add intent creation form
- [ ] Integrate with backend API
- [ ] Add transaction history
- [ ] Add decryption UI for users
