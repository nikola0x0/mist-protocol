# Cetus Protocol Documentation

## Overview

Cetus is a concentrated liquidity market maker (CLMM) DEX protocol built on Sui blockchain. It provides efficient token swaps with improved capital efficiency.

## Key Concepts

### Concentrated Liquidity (CLMM)

Unlike traditional AMMs where liquidity is distributed across the entire price curve (0, ∞), CLMM allows liquidity providers to concentrate their capital within specific price ranges.

**Benefits:**
- Higher capital efficiency (10x or more)
- Lower slippage for traders
- Better yields for liquidity providers

### Pool Structure

Each Cetus pool has:
- **Two tokens** (coin_a and coin_b)
- **Fee tier** (e.g., 0.3%, 0.05%, 1%)
- **Current price** (as sqrt_price)
- **Liquidity** at different price points (ticks)

## Contract Architecture

### Main Contracts

1. **CLMM Package** (`pool_script`)
   - Core swap logic
   - Functions: `swap_a2b`, `swap_b2a`
   - Handles token exchanges

2. **Global Config**
   - Protocol-wide settings
   - Fee configurations
   - Admin controls

3. **Integrate Package**
   - Integration helpers
   - Router functionality
   - Multi-hop swaps

### Contract Addresses

#### Mainnet

```
CLMM Package:    0x1eabed72c53feb3805120a081dc15963c204dc8d091542592abaf7a35689b2fb
Global Config:   0xdaa46292632c3c4d8f31f23ea0f9b36a28ff3677e9684980e4438403a67a3d8f
Integrate:       0x996c4d9480708fb8b92aa7acf819fb0497b5ec8e65ba06601cae2fb6db3312c3
Config:          0x95b8d278b876cae22206131fb9724f701c9444515813042f54f0a426c9a3bc2f
```

#### Testnet

```
CLMM Package:    0x5372d555ac734e272659136c2a0cd3227f9b92de67c80dc11250307268af2db8
Global Config:   0xf5ff7d5ba73b581bca6b4b9fa0049cd320360abd154b809f8700a8fd3cfaf7ca
Integrate:       0x19dd42e05fa6c9988a60d30686ee3feb776672b5547e328d6dab16563da65293
Config:          0xf5ff7d5ba73b581bca6b4b9fa0049cd320360abd154b809f8700a8fd3cfaf7ca
```

#### Token Addresses

```
CETUS Token:  0x6864a6f921804860930db6ddbe2e16acdf8504495ea7481637a1c8b9a8fe54b::cetus::CETUS
xCETUS Token: 0x9e69acc50ca03bc943c4f7c5304c2a6002d507b51c11913b247159c60422c606::xcetus::XCETUS
SUI Token:    0x2::sui::SUI
```

## Swap Functions

### swap_a2b

Swap from token A to token B.

```move
public entry fun swap_a2b<CoinTypeA, CoinTypeB>(
    global_config: address,      // Global config object
    pool: address,                // Pool object ID
    coins: vector<Coin<CoinTypeA>>, // Input coins
    by_amount_in: bool,          // True for exact input
    amount: u64,                 // Amount to swap
    amount_limit: u64,           // Min output (slippage)
    sqrt_price_limit: u128,      // Price limit
    clock: address,              // Clock object (0x6)
)
```

### swap_b2a

Swap from token B to token A.

```move
public entry fun swap_b2a<CoinTypeA, CoinTypeB>(
    global_config: address,
    pool: address,
    coins: vector<Coin<CoinTypeB>>,
    by_amount_in: bool,
    amount: u64,
    amount_limit: u64,
    sqrt_price_limit: u128,
    clock: address,
)
```

## Parameters Explained

### by_amount_in

- `true`: Specify exact input amount
- `false`: Specify exact output amount

Most swaps use `true` (exact input).

### amount

The amount of tokens to swap (in smallest unit).

Example: 1 SUI = 1,000,000,000 (9 decimals)

### amount_limit

Slippage protection:
- For swap_a2b: Minimum output of token B
- For swap_b2a: Minimum output of token A

Calculate as:
```
amount_limit = expected_output * (1 - slippage_tolerance)
```

Example with 1% slippage:
```
expected_output = 1000000
slippage = 0.01
amount_limit = 1000000 * (1 - 0.01) = 990000
```

### sqrt_price_limit

Price limit to prevent excessive slippage.

- For a2b: Use minimum sqrt_price (e.g., 4295048016)
- For b2a: Use maximum sqrt_price (e.g., 79226673515401279992447579055)

Default values effectively disable price limit checking.

### clock

Always use `0x6` (Sui's global clock object).

## Fee Structure

### Fee Tiers

Cetus pools have different fee tiers:

- **0.01%** (100) - Stablecoin pairs
- **0.05%** (500) - Correlated assets
- **0.3%** (3000) - Most pairs (default)
- **1%** (10000) - Exotic pairs

Fee rate is in basis points (1/10000):
- 100 = 0.01% = 0.0001
- 3000 = 0.3% = 0.003

### Protocol Fee

20% of swap fees go to protocol treasury.

Example:
- Swap fee: 0.3%
- User pays: 0.3%
- LP receives: 0.24%
- Protocol receives: 0.06%

## Price Calculation

### Sqrt Price

Cetus uses sqrt(price) for efficiency:

```
sqrt_price = sqrt(price_of_token_b / price_of_token_a)
```

To get actual price:
```
price = (sqrt_price / 2^64)^2
```

### Tick Index

Price is discretized into ticks:

```
tick_index = log(sqrt_price, 1.0001)
```

Each tick represents a 0.01% price change.

## API Reference

### Cetus REST API

Base URL: `https://api-sui.cetus.zone/v2/sui`

#### Get Pools

```bash
GET /swap/count
```

Returns all available pools with:
- Pool addresses
- Token pairs
- Current prices
- Fee rates
- Liquidity

#### Example Response

```json
{
  "data": {
    "pools": [
      {
        "swap_account": "0x...",
        "symbol": "SUI-USDC",
        "coin_a_address": "0x2::sui::SUI",
        "coin_b_address": "0x...",
        "current_sqrt_price": "1234567890",
        "fee_rate": 3000,
        "liquidity": "1000000000"
      }
    ]
  }
}
```

## SDK Integration

### TypeScript SDK

```bash
npm install @cetusprotocol/cetus-sui-clmm-sdk
```

```typescript
import { CetusClmmSDK } from '@cetusprotocol/cetus-sui-clmm-sdk';

const sdk = CetusClmmSDK.createSDK({ 
  env: 'mainnet' 
});

// Get pools
const pools = await sdk.Pool.getPools();

// Build swap
const payload = await sdk.Swap.preswap({
  pool: poolAddress,
  currentSqrtPrice: pool.current_sqrt_price,
  coinTypeA: 'SUI',
  coinTypeB: 'USDC',
  decimalsA: 9,
  decimalsB: 6,
  a2b: true,
  byAmountIn: true,
  amount: '1000000000',
});
```

### Aggregator SDK

For best prices across multiple DEXes:

```bash
npm install @cetusprotocol/aggregator-sdk
```

```typescript
import { AggregatorClient } from '@cetusprotocol/aggregator-sdk';

const client = new AggregatorClient();

// Find best route
const routes = await client.findRouters({
  from: '0x2::sui::SUI',
  target: '0x...::usdc::USDC',
  amount: new BN(1000000),
  byAmountIn: true,
});

// Execute swap
await client.fastRouterSwap({
  router: routes,
  txb,
  slippage: 0.01,
});
```

## Best Practices

### 1. Always Set Slippage Protection

```rust
let expected_output = 1000000;
let slippage = 0.01; // 1%
let amount_limit = (expected_output as f64 * (1.0 - slippage)) as u64;
```

### 2. Use Proper Decimals

```rust
// SUI has 9 decimals
let sui_amount = 1.5; // SUI
let amount_in_smallest = (sui_amount * 1_000_000_000.0) as u64;
```

### 3. Check Price Impact

High price impact = bad trade:
```rust
if price_impact > 0.05 { // 5%
    warn!("High price impact!");
}
```

### 4. Use Latest Addresses

Always check official docs for latest contract addresses:
- https://cetus-1.gitbook.io/cetus-developer-docs

### 5. Handle Errors

```rust
match swap_result {
    Ok(tx) => { /* success */ },
    Err(e) if e.to_string().contains("slippage") => {
        // Slippage exceeded
    },
    Err(e) if e.to_string().contains("insufficient") => {
        // Insufficient balance
    },
    Err(e) => { /* other error */ },
}
```

## Resources

### Official Links

- **Documentation**: https://cetus-1.gitbook.io/cetus-developer-docs
- **GitHub**: https://github.com/CetusProtocol
- **Website**: https://www.cetus.zone
- **App**: https://app.cetus.zone
- **Dev Telegram**: https://t.me/CetusDevNews

### Sui Resources

- **Sui Docs**: https://docs.sui.io
- **Sui Explorer**: https://suiexplorer.com
- **Sui TypeScript SDK**: https://sdk.mystenlabs.com

### Community

- **Discord**: [Cetus Community]
- **Twitter**: [@CetusProtocol]
- **Medium**: [Cetus Blog]

## Updates

Cetus regularly updates contracts. Always:
1. Join dev notification channel
2. Check docs before deploying
3. Test on testnet first
4. Monitor for deprecation notices

Last updated: November 2024
