// User & Authentication Types
export interface User {
  twitterHandle: string;
  twitterUserId: string;
  suiAddress?: string;
  objectId?: string; // XWalletAccount object ID
}

// Wallet Types
export interface Balance {
  coinType: string;
  amount: string;
  decimals: number;
  symbol: string;
}

export interface NFT {
  objectId: string;
  name: string;
  description?: string;
  imageUrl?: string;
  collection?: string;
}

export interface XWalletAccount {
  objectId: string;
  xid: string; // Twitter user ID
  handle: string; // Twitter handle
  balances: Balance[];
  nfts: NFT[];
  ownerAddress?: string; // Linked Sui wallet address
  lastTimestamp: number;
}

// Transaction Types
export interface Transaction {
  id: string;
  type: 'deposit' | 'withdraw' | 'transfer_in' | 'transfer_out';
  amount: string;
  coinType: string;
  from?: string;
  to?: string;
  timestamp: number;
  status: 'pending' | 'success' | 'failed';
  txHash?: string;
}

// API Response Types
export interface ApiResponse<T> {
  success: boolean;
  data?: T;
  error?: string;
}

// Enclave Types
export interface EnclaveSignature {
  response: {
    intent: number;
    timestamp_ms: number;
    data: any;
  };
  signature: string;
}

// ====== Transaction Sponsorship Types ======

// Request body for creating a sponsored transaction
export interface SponsorTxRequestBody {
  network: 'mainnet' | 'testnet';
  txBytes: string; // base64 encoded transaction kind bytes
  sender: string; // Sui address
  allowedAddresses?: string[]; // Optional: addresses allowed to execute
}

// Response from sponsor API
export interface CreateSponsoredTransactionApiResponse {
  bytes: string; // base64 encoded sponsored transaction bytes
  digest: string; // transaction digest
}

// Request body for executing a sponsored transaction
export interface ExecuteSponsoredTransactionApiInput {
  digest: string; // Transaction digest from sponsor response
  signature: string; // User's signature (base64)
}

// Response from execute API
export interface ExecuteSponsoredTransactionApiResponse {
  digest: string; // Final transaction digest
}
