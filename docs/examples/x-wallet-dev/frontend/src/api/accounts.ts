import { API_BASE_URL } from '../utils/constants';

// Types
export interface TokenBalance {
  symbol: string;
  coin_type: string;
  balance_raw: number;
  balance_formatted: string;
  decimals: number;
}

export interface BalanceResponse {
  balance_mist: number;
  balance_sui: string;
  balances: TokenBalance[];
  x_user_id: string;
  sui_object_id: string;
}

export interface AccountResponse {
  x_user_id: string;
  x_handle: string;
  sui_object_id: string;
  owner_address: string | null;
}

// API Functions

/**
 * Get account by wallet address
 */
export async function getAccountByWallet(walletAddress: string): Promise<AccountResponse | null> {
  try {
    const response = await fetch(`${API_BASE_URL}/api/account/by-wallet/${walletAddress}`);
    if (response.status === 404) {
      return null;
    }
    if (!response.ok) {
      throw new Error(`HTTP error! status: ${response.status}`);
    }
    return await response.json();
  } catch (error) {
    throw error;
  }
}

/**
 * Get account balance by sui_object_id
 */
export async function getAccountBalance(suiObjectId: string): Promise<BalanceResponse> {
  const response = await fetch(`${API_BASE_URL}/api/account/${suiObjectId}/balance`);
  if (!response.ok) {
    throw new Error(`HTTP error! status: ${response.status}`);
  }
  return await response.json();
}
