# Encifher Vaults - Comprehensive Architecture Analysis

## Executive Summary

This is a **Next.js 14 frontend application** for an encrypted DeFi protocol. It integrates with both **Solana** (primary blockchain) and **EVM chains** (Monad/Ethereum). The application provides user-facing widgets for encrypted token operations with decryption services powered by external gateways.

**Key Finding:** This is primarily a **frontend/client application**, not a complete system. Core infrastructure (TEE, threshold cryptography, symbolic execution) exists as external services, not implemented in this repository.

---

## Project Structure Overview

```
encifher-vaults-main/
├── app/                          # Next.js application root
│   ├── api/                      # Backend API routes
│   ├── components/               # React UI components
│   ├── hooks/                    # Custom React hooks
│   ├── idls/                     # Solana program IDLs (interfaces)
│   ├── layouts/                  # Layout components
│   ├── providers.tsx             # Wallet providers setup
│   └── [pages]/                  # Page routes (faucet, swap, payment, etc.)
├── lib/                          # Configuration & constants
├── utils/                        # Utility functions
├── scripts/                      # Node.js scripts
└── public/                       # Static assets
```

---

## Feature Implementation Status

### 1. TEE (Trusted Execution Environment) Co-Processor Functionality

**Status:** INTEGRATED (External Service) ✓ Partially

#### Implementation Details:
- **Primary Integration:** `@encifher-js/core` package (v1.1.7)
- **TEE Gateway:** Configurable via environment variables
  - `NEXT_PUBLIC_TEE_GATEWAY_URL` - Default: `https://monad.encrypt.rpc.encifher.io`
  - `COPROCESSOR_URL` - Default: `https://monad.decrypt.rpc.encifher.io`

#### Code Location:
```typescript
// utils/fhevm.ts
const client = new TEEClient({ 
  teeGatewayUrl: process.env.TEE_GATEWAY_URL || 'https://monad.encrypt.rpc.encifher.io' 
});
await client.init();
const handle = await client.encrypt(amount, PlaintextType.uint32);
```

#### Capabilities Implemented:
- **Encryption:** Supports uint32 and uint64 plaintext types
- **Handle-based Ciphertext:** Returns encrypted value as a "handle" (numeric identifier)
- **Proof Generation:** Creates zero-knowledge proofs for encrypted operations
- **Decryption:** Server-side decryption via `/api/decrypt` endpoint

#### What's NOT Implemented:
- No actual TEE/enclave code (SGX, SEV, TrustZone, Nitro Enclaves)
- No local TEE attestation
- Client only calls external TEE gateway services
- TEE implementation is entirely external (not in this repo)

---

### 2. Threshold Cryptography/KMS System

**Status:** NOT IMPLEMENTED ✗

#### Findings:
- **No threshold cryptography code** found in the repository
- **No secret sharing mechanisms** implemented
- **No multi-signature schemes** present
- **No KMS (Key Management System)** implementation

#### What Exists Instead:
- Single wallet-based key management (Solana/Ethereum wallets)
- Environment variable based secrets storage
- NextAuth for session management (Twitter OAuth)

```typescript
// Authentication via NextAuth with Twitter provider
const authOptions: NextAuthOptions = {
  providers: [
    TwitterProvider({
      clientId: process.env.TWITTER_CLIENT_ID!,
      clientSecret: process.env.TWITTER_CLIENT_SECRET!,
      version: '2.0',
    }),
  ],
};
```

---

### 3. Symbolic Execution Capabilities

**Status:** NOT IMPLEMENTED ✗

#### Findings:
- **No symbolic execution engine** in the codebase
- **No constraint solvers** used
- **No formal verification** of operations
- **No symbolic state tracking**

#### Grep Results:
- Search for "symbolic", "execution", "constraint" found only build artifacts

#### What's Used Instead:
- Direct transaction construction and execution
- Simple request/response patterns
- No formal analysis or constraint-based computation

---

### 4. Ciphertext Management with Handle-based Approach

**Status:** IMPLEMENTED ✓ Fully

#### Implementation:

**Handle Structure:**
```typescript
// From etoken.json IDL
type Einput = {
  handle: u128;        // Encrypted value reference
  proof: bytes;        // Zero-knowledge proof
}

type Euint64 = {
  handle: u128;        // Ciphertext handle
}
```

**Encryption/Decryption Flow:**

1. **Client-side Encryption** (utils/fhevm.ts):
```typescript
export const encryptAmount = async (address: string, amount: bigint, contractAddress: string) => {
  const client = new TEEClient({ teeGatewayUrl: process.env.TEE_GATEWAY_URL });
  await client.init();
  const handle = await client.encrypt(amount, PlaintextType.uint32);
  return {
    handles: [handle],
    inputProof: (new Uint8Array(1)).fill(1),
  }
};
```

2. **Server-side Decryption** (app/api/decrypt/route.ts):
```typescript
export async function POST(req: Request) {
  const { handle } = await req.json();
  const coprocessorUrl = process.env.COPROCESSOR_URL || 'https://monad.decrypt.rpc.encifher.io';
  const response = await fetch(`${coprocessorUrl}/decrypt`, {
    method: 'POST',
    body: JSON.stringify({ handle }),
  });
  return Response.json(await response.json());
}
```

3. **Handle Usage in Transactions** (hooks/usePlaceOrder.ts):
```typescript
const encAmount = await client.encrypt(parsedAmount, PlaintextType.uint64);
const ix = await orderManagerProgram.methods.placeOrder(deadline, {
  handle: new anchor.BN(encAmount),
  proof: Buffer.from([0])
}).accounts({...}).instruction();
```

**Key Features:**
- Handles are u128 numeric identifiers
- Each handle represents a ciphertext
- Proofs validate the encryption
- Handles used in smart contracts for privacy-preserving operations

---

### 5. Integration with Solana/EVM Chains

**Status:** PARTIALLY IMPLEMENTED ✓ Partially

#### Solana Integration (Primary)

**Primary Chain:** Solana Devnet
```typescript
// providers.tsx
const network = WalletAdapterNetwork.Devnet;
const wallets = [
  new PhantomWalletAdapter(),
  new SolflareWalletAdapter()
];
```

**Program Interactions:**
1. **Order Manager Program** (ID: `Acnjuc5m2iASg5wtPkSMzwA48P9iT6ggMm6AmB4GYBJU`)
   - Place orders with encrypted amounts
   - Aggregate and process orders
   - Solver-based order execution

2. **EToken Program** (ID: `EgvZcun7jwa2rXUYGyeTYp9BVRTmf4n3TAcDC2C2FfmP`)
   - Initialize token accounts
   - Mint encrypted tokens
   - Transfer encrypted amounts (`etransfer`)
   - Burn operations

3. **PET Executor Program** (ID: `65veyWr4M57qBv83ZFD79LXT5DyovxMgdDZLzApR3oCK`)
   - Program execution environment
   - Compute unit coordination

**Solana IDLs Present:**
- etoken.json (489 lines) - Token program interface
- order_manager.json (553 lines) - Order management interface
- pet_executor.json (397 lines) - Executor interface
- solver.json (223 lines) - Solver interface

#### EVM Integration (Secondary)

**Chains Supported:** Monad Testnet
```typescript
// lib/config.ts (commented out but shows intended config)
const monadTestnet: Chain = {
  id: 0x279f,
  name: 'Monad Testnet',
  nativeCurrency: { name: 'MON', symbol: 'MON', decimals: 18 },
  rpcUrls: { default: { http: [process.env.NEXT_PUBLIC_MONAD_RPC_URL] } },
};
```

**EVM Smart Contracts:**
1. **eERC20 Token** - Encrypted ERC20 implementation
   - Functions: mint, burn, transfer, approve
   - Encrypted balance storage (euint32)
   - Supports encrypted approvals

2. **eERC20Wrapper** - Wrapper contract for token conversion
   - depositAndWrap - Convert plaintext to encrypted tokens
   - withdrawToken - Encrypted withdrawal
   - claimWrappedToken - Claim wrapped tokens
   - Callback support for async operations

3. **OrderManager** - Order routing and matching
   - placeOrder with encrypted amounts
   - aggregateOrders functionality
   - Process solver requests
   - Multiple order manager instances per pair

4. **AnonTransferContract** - Anonymous transfers
   - anonymousTransfer with encrypted target/amount
   - Transfer completion mechanics

**EVM ABIs Available:**
- eERC20Abi - 27 functions/events
- encifherERC20Abi - Standard ERC20 with ECDSA
- eerc20WrapperAbi - 9 wrapper functions
- orderManagerAbi - 19 functions
- anonTransferAbi - 6 functions

#### Multi-chain Tokens Supported:

| Token | Solana Address | EVM Address | Wrapper |
|-------|---|---|---|
| USDC | (eUSDC) | 0x32b998... | 0x2DdcacdB... |
| ENC | (eENC) | 0x45BfbF0D... | 0xeENC_WRAPPER |
| SHMON | (eSHMON) | 0xE5E9d55A... | - |

---

### 6. Privacy-preserving DeFi Operations

**Status:** IMPLEMENTED ✓ Fully (Limited Scope)

#### Operations Implemented:

##### A. Private Payments (PaymentWidget)
```typescript
// components/PaymentWidget/PaymentWidget.tsx
const handlePay = async () => {
  const parsedAmount = Number(amount) * 10 ** 6;
  const client = new TEEClient({ teeGatewayUrl: process.env.NEXT_PUBLIC_TEE_GATEWAY_URL });
  await client.init();
  
  // Encrypt amount
  const encryptedAmount = await client.encrypt(parsedAmount, PlaintextType.uint64);
  
  // Create encrypted transfer transaction
  const ix = await etokenProgram.methods.etransfer({
    handle: new BN(encryptedAmount),
    proof: Buffer.from([0])
  }).accounts({...}).instruction();
};
```
**Privacy Level:** Amount is encrypted, receiver address remains visible

##### B. Private Swaps (SwapWidget)
- Order placement with encrypted amounts (currently disabled/commented)
- Order aggregation before execution
- Solver-based matching (maintains privacy during aggregation)

```typescript
// Current state: Swap functionality is mostly commented out
const swap = async (amountIn: string, amountOut: string, onSuccess: () => void) => {
  // Code is currently commented/disabled
  // Full implementation available but not active
};
```

##### C. Private Deposits/Wrapping (WrapperWidget)
- Convert plaintext ERC20 to encrypted eERC20
- Encrypted deposit mechanism
- Private recipient support (optional)

##### D. Balance Decryption On-Demand
- Balances displayed as encrypted until user chooses to decrypt
- Decryption happens client-side via TEE gateway
- Balance shown after decryption

```typescript
// hooks/useAsync.ts
const fetchBalance = async (account: PublicKey) => {
  const accountData = etokenProgram.account.tokenAccount.coder.accounts.decode("tokenAccount", accountInfo.data);
  const decryptedBalance = await decrypt32(accountData?.amount?.handle?.toString());
  return Number(decryptedBalance) / 10 ** 6;
};
```

#### Privacy Guarantees:
- **Amount Privacy:** Encrypted in transit and storage
- **Limited Recipient Privacy:** Recipient address on-chain visible (can be encrypted separately)
- **Balance Privacy:** Encrypted until decryption requested
- **Transaction Privacy:** Solana transaction visible but amount encrypted

#### What's NOT Private:
- Wallet addresses (visible on-chain)
- Transaction existence
- Transaction timing
- Relayer/solver involvement patterns

---

## Architecture Deep Dive

### Core Technology Stack

**Frontend Framework:**
- Next.js 14.2.6 (React 18)
- TypeScript 5
- Tailwind CSS 3.4.1
- Framer Motion 11.5.4 (animations)

**Blockchain Integration:**
- Solana Web3.js
- Anchor Framework 0.31.1
- Coral XYZ Wallet Adapter
- Ethers.js (EVM)

**Privacy/Cryptography:**
- @encifher-js/core 1.1.7 (TEE client)
- petcrypt-js-lite 1.0.1 (PET cryptography)

**Infrastructure:**
- MongoDB 6.12.0 (transaction history)
- DynamoDB (AWS SDK)
- NextAuth 4.24.11 (authentication)

**DevOps:**
- Stack.so integration (points tracking)
- Pointy API (blockchain tracking)
- The Graph (subgraph queries)

### Data Flow

#### 1. Encryption Pipeline
```
User Input 
  ↓
encryptAmount() [fhevm.ts]
  ↓
TEEClient.init() + TEEClient.encrypt()
  ↓
TEE Gateway (https://monad.encrypt.rpc.encifher.io)
  ↓
Handle + Proof
  ↓
Send to Smart Contract
```

#### 2. Decryption Pipeline
```
Smart Contract/Account with Handle
  ↓
decrypt32(handle) [fhevm.ts]
  ↓
POST /api/decrypt
  ↓
Coprocessor Gateway (https://monad.decrypt.rpc.encifher.io)
  ↓
Plaintext Value
  ↓
Display/Use in Client
```

#### 3. Order Processing
```
User Places Encrypted Order
  ↓
OrderManager.placeOrder(deadline, encryptedAmount, proof)
  ↓
Orders Aggregated (via aggregateOrders)
  ↓
Solver Processes Aggregated Orders
  ↓
Relayer Executes on Wrapper Contracts
  ↓
User Receives Swapped Token
```

### API Routes

| Route | Method | Purpose |
|-------|--------|---------|
| `/api/decrypt` | POST | Decrypt encrypted values via coprocessor |
| `/api/mint` | POST | Mint plaintext tokens (Monad faucet) |
| `/api/mint-erc20` | POST | Mint SHMON tokens |
| `/api/wrap-shmon` | POST | Wrap SHMON token on Solana |
| `/api/transactions` | POST | Fetch and cache user transactions |
| `/api/fetchTransaction` | GET | Retrieve transaction from cache |
| `/api/users` | POST | Create/update user record |
| `/api/auth/[...nextauth]` | GET/POST | Authentication endpoints |

### Smart Contract Integration

#### Order Manager Flow
```solana
1. User calls: placeOrder(deadline, encryptedAmount, proof)
   ├─ Validates user authority
   ├─ Stores order with encrypted amount
   └─ Emits OrderPlaced event

2. Relayer calls: aggregateOrders()
   ├─ Collects all pending orders
   ├─ Calls PET Executor program
   └─ Emits OrdersProcessed event

3. Solver calls: processSolverRequest(usdcAmount)
   ├─ Executes matched orders
   ├─ Releases funds to users
   └─ Emits OrdersSolved event
```

### Token Account Structure

```rust
// From IDL: TokenAccount
struct TokenAccount {
    mint: PublicKey,
    owner: PublicKey,
    amount: Euint64,           // Encrypted balance
    delegate: Option<PublicKey>,
    is_initialized: bool,
    is_frozen: bool
}

// Euint64 wrapper for encrypted value
struct Euint64 {
    handle: u128               // Numeric identifier for ciphertext
}
```

---

## Key Components & Widgets

### UI Components

1. **SwapWidget** - Encrypted token swaps
   - Asset selection
   - Amount input
   - Encrypted swap execution
   - Status tracking

2. **PaymentWidget** - Private payments
   - Recipient address input
   - Encrypted amount
   - Recipient token account initialization
   - Transaction confirmation

3. **WrapperWidget** - Token wrapping
   - Plaintext to encrypted conversion
   - Optional private recipient
   - Deposit confirmation

4. **Faucet** - Token minting
   - ENC token distribution
   - SHMON token wrapping
   - Rate-limited claims

5. **Leaderboard** - User statistics
   - Wallet enumeration
   - Activity tracking
   - Searchable interface

6. **Vault** - Portfolio view
   - Balance display (encrypted/decrypted)
   - Asset allocation
   - Transaction history

### Hooks

- `useSwap` - Swap operation management
- `usePlaceOrder` - Solana order placement
- `useAsync` - Async balance fetching
- `useStake` - Staking operations
- `useSyncProvider` - Provider synchronization

---

## External Dependencies & Services

### Gateway Services

1. **TEE Encryption Gateway**
   - URL: `https://monad.encrypt.rpc.encifher.io`
   - Function: Client-side encryption of amounts
   - Returns: Handle + Proof

2. **Coprocessor Decryption Gateway**
   - URL: `https://monad.decrypt.rpc.encifher.io`
   - Function: Server-side decryption of handles
   - Returns: Plaintext values

3. **Solana RPC**
   - Devnet endpoint (configurable)
   - Used for: Transaction submission, account data

4. **The Graph Subgraph**
   - Purpose: Query pool prices for USDC/USDT
   - Function: Real-time price feeds

### Third-party Services

1. **Stack.so** - Points tracking
   - Tracks swap and payment activities
   - Awards points to users

2. **Pointy** - Blockchain leaderboard
   - User activity indexing
   - Leaderboard data

3. **MongoDB Atlas** - Transaction storage
   - Persistent transaction history
   - User-specific data

4. **Twitter OAuth** - Authentication
   - User login via Twitter
   - Session management

---

## Configuration & Environment Variables

### Required Variables (Inferred)

```bash
# Blockchain RPC Endpoints
NEXT_PUBLIC_RPC_URL=https://api.devnet.solana.com

# TEE Gateway Services
TEE_GATEWAY_URL=https://monad.encrypt.rpc.encifher.io
COPROCESSOR_URL=https://monad.decrypt.rpc.encifher.io

# Smart Contract Addresses
NEXT_PUBLIC_USDC_ADDRESS=...
NEXT_PUBLIC_EUSDC_ADDRESS=...
NEXT_PUBLIC_ENC_ADDRESS=...
NEXT_PUBLIC_EENC_ADDRESS=...
NEXT_PUBLIC_ORDER_MANAGER=...
NEXT_PUBLIC_EXECUTOR=...
NEXT_PUBLIC_EMINT=...

# Wrapper Contract Addresses
NEXT_PUBLIC_EUSDC_WRAPPER_ADDRESS=...
NEXT_PUBLIC_EENC_WRAPPER_ADDRESS=...

# Faucet Configuration
FAUCET_KEY=<private-key>
REFILL_PRIVATE_KEY=<private-key>

# Database
MONGODB_URI=mongodb+srv://...
DB_NAME=database

# Authentication
NEXTAUTH_SECRET=...
TWITTER_CLIENT_ID=...
TWITTER_CLIENT_SECRET=...

# Third-party APIs
NEXT_PUBLIC_STACKSO_API_KEY=...
NEXT_PUBLIC_POINTY_API_KEY=...
NEXT_PUBLIC_POINTY_PRIVATE_KEY=...
NEXT_PUBLIC_GRAPH_APIKEY=...

# Solana Program Authority
AUTHORITY=<base64-encoded-keypair>
```

---

## What's Actually Implemented vs. Technical Spec Claims

### Implemented Features

| Feature | Implementation Status | Location |
|---------|---|---|
| TEE Integration | ✓ External service client only | utils/fhevm.ts |
| Handle-based Ciphertext | ✓ Full | Everywhere (eToken, OrderManager) |
| Solana Integration | ✓ Complete | All hooks, components |
| EVM Integration | ✓ Partial (Config present, code commented) | lib/config.ts |
| Private Payments | ✓ Functional | PaymentWidget |
| Private Balance Viewing | ✓ Functional | useAsync hook |
| Order Management | ✓ Functional (Solana) | usePlaceOrder, hooks |
| Token Wrapping | ✓ Partial (UI present, logic commented) | WrapperWidget |
| Authentication | ✓ Twitter OAuth | NextAuth integration |
| Transaction History | ✓ MongoDB cached | api/transactions |

### NOT Implemented

| Feature | Status | Notes |
|---------|--------|-------|
| Threshold Cryptography | ✗ None | No multi-sig, no secret sharing |
| KMS System | ✗ None | Wallet-based only |
| Symbolic Execution | ✗ None | Not relevant to frontend |
| Local TEE/Enclave | ✗ None | Only external service calls |
| Private Swaps | ✗ Mostly disabled | Code present but commented out |
| Full Private Transfers | ✗ Partial | Recipient address visible |
| Formal Verification | ✗ None | No constraint solving |

### Disabled/Commented Features

1. **EVM-based Swaps**
   - Code written but commented in useSwap.ts
   - Rainbow Kit wallet integration disabled
   - Viem-based transactions disabled

2. **Token Wrapping on EVM**
   - Wrapper contract ABIs defined
   - User interface present
   - Logic commented out in WrapperWidget.tsx

3. **Pool Price Queries**
   - Uniswap integration available
   - Quote logic disabled
   - Graph API query skeleton present

---

## Security Observations

### Strengths
1. **Encryption at Rest:** All amounts encrypted using TEE gateway
2. **Proof Validation:** Zero-knowledge proofs included with encrypted values
3. **Rate Limiting:** Faucet implements IP-based rate limiting
4. **Transaction Verification:** Transactions cached in MongoDB with deduplication
5. **Authority Validation:** Solana transactions require wallet signatures

### Potential Concerns
1. **External TEE Trust:** Complete reliance on external TEE gateway URLs
2. **Proof Validation:** Proofs are minimal (single byte `Buffer.from([0])`)
3. **Decryption Transparency:** Decryption happens server-side with no verification
4. **Address Privacy:** Recipient addresses and wallet owners remain visible
5. **No Local Encryption:** Encryption/decryption entirely dependent on external services
6. **Environment Variables:** Sensitive keys in .env (not committed but critical)

---

## Development & Build

### Build Configuration
- Next.js 14 with experimental features
- TypeScript strict mode
- ESLint enabled
- Tailwind CSS JIT compilation

### Development Commands
```bash
npm run dev        # Start development server
npm run build      # Build for production
npm start          # Run production build
npm run lint       # Run ESLint
npm run monitor    # Run transaction service
```

### Notable: Transaction Monitor Service
```typescript
// scripts/transactionService.ts
- Monitors blockchain for new transactions
- Fetches transaction details
- Updates MongoDB cache
- Runs continuously in background
```

---

## Conclusion

### What This Actually Is
**A Next.js frontend application** that provides a user interface for encrypted DeFi operations on Solana and EVM chains. It's a **client for existing services**, not a complete privacy infrastructure.

### Architecture Type
- **Three-tier application:**
  1. **Frontend:** React/Next.js components (this repo)
  2. **Backend API:** Next.js API routes + MongoDB
  3. **External Services:** TEE gateways, blockchain nodes, third-party APIs

### Key Limitations
1. TEE implementation is external (not in this repo)
2. No local cryptographic operations
3. Limited threshold cryptography
4. No symbolic execution engine
5. Server-side decryption introduces trust requirements

### Intended Use Case
Privacy-preserving token swaps and payments on Solana, with experimental EVM support. Suitable for:
- Private token transfers
- Confidential trading
- Anonymous payments
- Privacy-conscious DeFi users

### This Is NOT
- A standalone cryptographic library
- A TEE implementation
- A full blockchain node
- A complete KMS system
- A formal verification engine

