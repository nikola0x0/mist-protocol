import { API_BASE_URL } from '../utils/constants';

// Types
export interface TransactionResponse {
  tx_digest: string;
  tx_type: 'coin_transfer' | 'coin_deposit' | 'coin_withdraw' | 'nft_transfer' | 'nft_deposit' | 'nft_withdraw' | 'link_wallet';
  from_id: string | null;
  to_id: string | null;
  from_handle: string | null;
  to_handle: string | null;
  // Coin fields
  coin_type: string | null;
  amount: string | null;
  amount_mist: number | null;
  // NFT fields
  nft_object_id: string | null;
  nft_type: string | null;
  nft_name: string | null;
  // Link wallet fields
  from_address: string | null;
  to_address: string | null;
  tweet_id: string | null;
  timestamp: number;
  created_at: string;
}

export interface PaginatedTransactionsResponse {
  data: TransactionResponse[];
  total: number;
  page: number;
  limit: number;
  total_pages: number;
}

// API Functions

/**
 * Get transaction history by sui_object_id with pagination
 */
export async function getTransactionHistory(
  suiObjectId: string,
  page: number = 1,
  limit: number = 5
): Promise<PaginatedTransactionsResponse> {
  const response = await fetch(
    `${API_BASE_URL}/api/account/${suiObjectId}/transactions?page=${page}&limit=${limit}`
  );
  if (!response.ok) {
    throw new Error(`HTTP error! status: ${response.status}`);
  }
  return await response.json();
}
