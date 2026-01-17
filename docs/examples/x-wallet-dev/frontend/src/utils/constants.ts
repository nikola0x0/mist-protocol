// API Configuration
export const API_BASE_URL = import.meta.env.VITE_API_BASE_URL || 'http://localhost:3001';
export const ENCLAVE_URL = import.meta.env.VITE_ENCLAVE_URL || 'http://localhost:3000';

// Sui Network Configuration
export const SUI_NETWORK = import.meta.env.VITE_SUI_NETWORK || 'testnet';

// Smart Contract Addresses
export const XWALLET_PACKAGE_ID = import.meta.env.VITE_XWALLET_PACKAGE_ID || '';

export const CONTRACT_ADDRESSES = {
  XWALLET_ACCOUNT: import.meta.env.VITE_XWALLET_ACCOUNT_ADDRESS || '',
  XWALLET_TRANSFER: import.meta.env.VITE_XWALLET_TRANSFER_ADDRESS || '',
  XWALLET_ENCLAVE: import.meta.env.VITE_XWALLET_ENCLAVE_ADDRESS || '',
  ENCLAVE_CONFIG: import.meta.env.VITE_ENCLAVE_CONFIG_ADDRESS || '',
};

// X (Twitter) OAuth Configuration
export const TWITTER_OAUTH = {
  CLIENT_ID: import.meta.env.VITE_TWITTER_CLIENT_ID || '',
  REDIRECT_URI: import.meta.env.VITE_TWITTER_REDIRECT_URI || 'http://localhost:5173/callback',
  SCOPES: ['tweet.read', 'users.read', 'offline.access'],
};

// Coin Types
export const COIN_TYPES = {
  SUI: '0x2::sui::SUI',
  WAL: '0x8270feb7375eee355e64fdb69c50abb6b5f9393a722883c1cf45f8e26048810a::wal::WAL',
  USDC: '0xa1ec7fc00a6f40db9693ad1415d0c193ad3906494428cf252621037bd7117e29::usdc::USDC',
} as const;

// Default Values
export const DEFAULT_DECIMALS = 9; // SUI decimals
export const MIST_PER_SUI = 1_000_000_000; // 10^9 MIST = 1 SUI
export const REFRESH_INTERVAL = 10000; // 10 seconds

// Route Paths
export const ROUTES = {
  HOME: '/',
  DASHBOARD: '/dashboard',
  CALLBACK: '/callback',
} as const;
