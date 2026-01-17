import { API_BASE_URL } from '../utils/constants';

// Types
export interface NftResponse {
  nft_object_id: string;
  nft_type: string;
  name: string | null;
  image_url: string | null;
  created_at: string;
}

export interface AccountNftsResponse {
  nfts: NftResponse[];
  count: number;
}

// API Functions

/**
 * Get NFTs for an account by sui_object_id
 */
export async function getAccountNfts(suiObjectId: string): Promise<AccountNftsResponse> {
  const response = await fetch(`${API_BASE_URL}/api/account/${suiObjectId}/nfts`);
  if (!response.ok) {
    throw new Error(`HTTP error! status: ${response.status}`);
  }
  return await response.json();
}

// Note: NFT activities are now part of the unified /transactions endpoint
// Use useActivitiesStream hook to get all activities including NFTs
