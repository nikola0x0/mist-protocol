# Frontend Guide - Browser Wallet Integration

## Overview

The frontend is a Next.js/React application that:
- Connects to Sui browser wallets
- Requests unsigned transactions from backend
- Signs transactions in the user's wallet
- Submits transactions to blockchain

## Security Model

```
┌──────────────────┐
│  Browser Wallet  │  ← Private keys stay here!
│  (Sui Wallet,    │
│   Suiet, etc)    │
└────────┬─────────┘
         │ Signs locally
         │
    ┌────▼─────┐
    │  React   │
    │   App    │
    └────┬─────┘
         │ Requests unsigned TX
         │
    ┌────▼─────┐
    │  Axum    │
    │ Backend  │
    └──────────┘
```

**Key Point:** Private keys NEVER leave the browser wallet!

## Setup

### 1. Install Dependencies

```bash
cd frontend
npm install
```

### 2. Configure Backend URL

In `components/CetusSwap.tsx`:

```typescript
const BACKEND_URL = 'http://localhost:3000';
// Or use environment variable
// const BACKEND_URL = process.env.NEXT_PUBLIC_BACKEND_URL;
```

### 3. Run Development Server

```bash
npm run dev
```

Visit `http://localhost:3000`

## Wallet Integration

### Supported Wallets

The app supports all Sui wallets that implement the standard:
- Sui Wallet (official)
- Suiet Wallet
- Ethos Wallet
- Martian Wallet
- Glass Wallet

### Connection Flow

```typescript
import { ConnectButton, useCurrentAccount } from '@mysten/dapp-kit';

function MyComponent() {
  const account = useCurrentAccount();
  
  return (
    <>
      <ConnectButton />
      {account && <p>Connected: {account.address}</p>}
    </>
  );
}
```

## Transaction Signing

### How It Works

1. **Request Unsigned Transaction**
   ```typescript
   const response = await fetch('/api/build-swap', {
     method: 'POST',
     body: JSON.stringify({
       user_address: account.address,
       token_a: '0x2::sui::SUI',
       token_b: '0x...',
       amount: 1000000,
     }),
   });
   
   const { tx_bytes } = await response.json();
   ```

2. **Parse Transaction**
   ```typescript
   import { Transaction } from '@mysten/sui/transactions';
   
   const txBlock = Transaction.from(tx_bytes);
   ```

3. **Request User Signature**
   ```typescript
   import { useSignAndExecuteTransaction } from '@mysten/dapp-kit';
   
   const { mutate: signAndExecute } = useSignAndExecuteTransaction();
   
   signAndExecute(
     { transaction: txBlock },
     {
       onSuccess: (result) => {
         console.log('Success!', result.digest);
       },
       onError: (error) => {
         console.error('Failed:', error);
       },
     }
   );
   ```

### What Happens During Signing

1. Wallet popup appears
2. User sees transaction details
3. User approves or rejects
4. If approved, wallet signs with private key
5. Signed transaction returned to app
6. App submits to blockchain

## Component Structure

### CetusSwap.tsx

Main swap interface component:

```typescript
export default function CetusSwap() {
  // State management
  const [pools, setPools] = useState<Pool[]>([]);
  const [selectedPool, setSelectedPool] = useState<Pool | null>(null);
  
  // Wallet connection
  const account = useCurrentAccount();
  
  // Transaction signing
  const { mutate: signAndExecute } = useSignAndExecuteTransaction();
  
  // Swap execution
  const executeSwap = async () => {
    // 1. Build transaction on backend
    const quote = await buildSwapTransaction();
    
    // 2. Sign in wallet
    signAndExecute({ transaction: quote.tx_bytes });
  };
  
  return (/* UI */);
}
```

## User Experience

### Step-by-Step Flow

1. **Connect Wallet**
   - User clicks "Connect Wallet"
   - Wallet extension opens
   - User selects wallet and approves connection

2. **Select Pool**
   - User clicks "Load Pools"
   - Pools fetched from backend
   - User selects desired token pair

3. **Enter Amount**
   - User enters swap amount
   - Sets slippage tolerance

4. **Review & Swap**
   - User clicks "Swap"
   - Backend builds transaction
   - Expected output shown
   - Wallet popup appears

5. **Sign Transaction**
   - User reviews in wallet
   - Approves transaction
   - Transaction submitted

6. **Confirmation**
   - Transaction digest displayed
   - User can view on explorer

## Error Handling

### Common Errors

```typescript
try {
  await executeSwap();
} catch (error) {
  if (error.message.includes('User rejected')) {
    // User declined in wallet
    setStatus('Transaction cancelled');
  } else if (error.message.includes('Insufficient')) {
    // Not enough balance
    setStatus('Insufficient balance');
  } else {
    // Other errors
    setStatus(`Error: ${error.message}`);
  }
}
```

### Error Types

1. **Wallet Not Connected**
   ```typescript
   if (!account) {
     alert('Please connect wallet first');
     return;
   }
   ```

2. **User Rejection**
   ```typescript
   onError: (error) => {
     if (error.message.includes('rejected')) {
       setStatus('You rejected the transaction');
     }
   }
   ```

3. **Network Errors**
   ```typescript
   if (!response.ok) {
     throw new Error('Backend unavailable');
   }
   ```

4. **Blockchain Errors**
   ```typescript
   // Transaction failed on-chain
   if (result.effects.status.status !== 'success') {
     setStatus('Transaction failed on blockchain');
   }
   ```

## Styling

### Tailwind CSS

The app uses Tailwind for styling:

```typescript
<button
  className="w-full bg-gradient-to-r from-blue-500 to-indigo-600 
             text-white py-4 px-6 rounded-lg font-semibold 
             hover:from-blue-600 hover:to-indigo-700 transition
             disabled:opacity-50 disabled:cursor-not-allowed"
>
  Swap
</button>
```

### Responsive Design

```typescript
<div className="min-h-screen p-8">
  <div className="max-w-2xl mx-auto">
    {/* Content */}
  </div>
</div>
```

## Advanced Features

### Gas Estimation

```typescript
const estimateGas = async (txBlock: Transaction) => {
  const gasEstimate = await suiClient.dryRunTransactionBlock({
    transactionBlock: await txBlock.build(),
  });
  
  return gasEstimate.effects.gasUsed;
};
```

### Price Impact Warning

```typescript
if (quote.pool_info.price_impact > 5) {
  const confirm = window.confirm(
    `High price impact: ${quote.pool_info.price_impact}%. Continue?`
  );
  if (!confirm) return;
}
```

### Transaction History

```typescript
const [history, setHistory] = useState<string[]>([]);

const saveTransaction = (digest: string) => {
  setHistory(prev => [...prev, digest]);
  localStorage.setItem('tx_history', JSON.stringify(history));
};
```

## Testing

### Manual Testing

1. **Connect Wallet**
   - Test with different wallet providers
   - Verify correct address displayed

2. **Load Pools**
   - Check all pools load correctly
   - Verify pool details are accurate

3. **Execute Swap**
   - Test with small amounts first
   - Verify expected output calculation
   - Check slippage protection works

4. **Error Cases**
   - Try with insufficient balance
   - Test wallet rejection
   - Verify network errors handled

### Automated Testing

```typescript
// __tests__/CetusSwap.test.tsx
import { render, screen, fireEvent } from '@testing-library/react';
import CetusSwap from '@/components/CetusSwap';

test('shows connect wallet button', () => {
  render(<CetusSwap />);
  expect(screen.getByText(/connect/i)).toBeInTheDocument();
});
```

## Production Checklist

- [ ] Update BACKEND_URL to production
- [ ] Configure proper CORS
- [ ] Add error tracking (Sentry)
- [ ] Implement analytics
- [ ] Add loading states
- [ ] Optimize bundle size
- [ ] Test on mobile devices
- [ ] Security audit

## Wallet Provider Configuration

### _app.tsx Setup

```typescript
import { WalletProvider } from '@mysten/dapp-kit';

const networks = {
  mainnet: { url: getFullnodeUrl('mainnet') },
  testnet: { url: getFullnodeUrl('testnet') },
};

<WalletProvider 
  networks={networks} 
  defaultNetwork="mainnet"
  autoConnect
>
  <Component {...pageProps} />
</WalletProvider>
```

## Debugging

### Enable Debug Logging

```typescript
// In _app.tsx
import { createNetworkConfig } from '@mysten/dapp-kit';

const { networkConfig } = createNetworkConfig({
  mainnet: { url: getFullnodeUrl('mainnet') },
  testnet: { url: getFullnodeUrl('testnet') },
});

// Enable verbose logging
localStorage.setItem('dapp-kit:verbose', 'true');
```

### Common Issues

1. **Wallet Not Detected**
   - Ensure wallet extension installed
   - Refresh page
   - Check browser console

2. **Transaction Fails**
   - Check network (mainnet vs testnet)
   - Verify sufficient gas
   - Check contract addresses

3. **Signature Request Not Showing**
   - Disable popup blockers
   - Try different browser
   - Check wallet extension enabled

## Resources

- [Sui dApp Kit Docs](https://sdk.mystenlabs.com/dapp-kit)
- [Wallet Standard](https://docs.sui.io/standards/wallet-standard)
- [Transaction Building](https://docs.sui.io/guides/developer/sui-101/building-ptb)
