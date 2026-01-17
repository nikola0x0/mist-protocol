import { useState, useEffect, useCallback, useMemo } from 'react';
import { API_BASE_URL } from '../utils/constants';

// Unified transaction type - includes coins, NFTs, and link wallet
export interface TransactionData {
  tx_digest: string;
  tx_type: 'coin_transfer' | 'coin_deposit' | 'coin_withdraw' | 'nft_transfer' | 'nft_deposit' | 'nft_withdraw' | 'link_wallet';
  from_id: string | null;
  to_id: string | null;
  from_handle: string | null;
  to_handle: string | null;
  from_address: string | null;
  to_address: string | null;
  // Coin fields (null for NFT/link_wallet transactions)
  coin_type: string | null;
  amount: string | null;
  amount_mist: number | null;
  // NFT fields (null for coin/link_wallet transactions)
  nft_object_id: string | null;
  nft_type: string | null;
  nft_name: string | null;
  tweet_id: string | null;
  timestamp: number;
  created_at: string;
}

// Keep for backward compatibility
export interface NftActivityData {
  tx_digest: string;
  activity_type: 'nft_transfer' | 'nft_deposit' | 'nft_withdraw';
  from_id: string | null;
  to_id: string | null;
  from_handle: string | null;
  to_handle: string | null;
  nft_object_id: string;
  nft_type: string | null;
  nft_name: string | null;
  timestamp: number;
  created_at: string;
}

export type Activity =
  | { type: 'coin'; data: TransactionData }
  | { type: 'nft'; data: TransactionData }
  | { type: 'link_wallet'; data: TransactionData };

interface UseActivitiesOptions {
  enabled?: boolean;
  page?: number;
  pageSize?: number;
}

/**
 * Hook to fetch unified activities data (transactions, NFT activities, link wallet)
 * Uses server-side pagination - fetches only what's needed per page
 */
export function useActivitiesStream(
  suiObjectId: string | null | undefined,
  options: UseActivitiesOptions = {}
) {
  const { enabled = true, page = 1, pageSize = 10 } = options;
  const [transactions, setTransactions] = useState<TransactionData[]>([]);
  const [totalTransactions, setTotalTransactions] = useState(0);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Fetch data with server-side pagination
  const fetchData = useCallback(async () => {
    if (!suiObjectId) return;

    setIsLoading(true);
    setError(null);

    try {
      // Fetch unified transactions (includes coins, NFTs, and link_wallet)
      const response = await fetch(
        `${API_BASE_URL}/api/account/${suiObjectId}/transactions?page=${page}&limit=${pageSize}`
      );

      const data = response.ok ? await response.json() : { data: [], total: 0 };

      setTransactions(data.data || []);
      setTotalTransactions(data.total || 0);
    } catch {
      setError('Failed to fetch activities');
    } finally {
      setIsLoading(false);
    }
  }, [suiObjectId, page, pageSize]);

  // Fetch when dependencies change
  useEffect(() => {
    if (!suiObjectId || !enabled) {
      setTransactions([]);
      setTotalTransactions(0);
      return;
    }

    fetchData();
  }, [suiObjectId, enabled, fetchData]);

  // Combined activities with type classification (memoized)
  const combinedActivities = useMemo<Activity[]>(() =>
    transactions.map((tx): Activity => {
      if (tx.tx_type === 'link_wallet') {
        return { type: 'link_wallet', data: tx };
      } else if (tx.nft_object_id) {
        return { type: 'nft', data: tx };
      } else {
        return { type: 'coin', data: tx };
      }
    }).sort((a, b) => b.data.timestamp - a.data.timestamp),
    [transactions]
  );

  // For backward compatibility - filter NFT activities
  const nftActivities = useMemo<NftActivityData[]>(() =>
    transactions
      .filter(tx => tx.nft_object_id)
      .map(tx => ({
        tx_digest: tx.tx_digest,
        activity_type: tx.tx_type as 'nft_transfer' | 'nft_deposit' | 'nft_withdraw',
        from_id: tx.from_id,
        to_id: tx.to_id,
        from_handle: tx.from_handle,
        to_handle: tx.to_handle,
        nft_object_id: tx.nft_object_id!,
        nft_type: tx.nft_type,
        nft_name: tx.nft_name,
        timestamp: tx.timestamp,
        created_at: tx.created_at,
      })),
    [transactions]
  );

  // Total count for pagination (memoized)
  const totalItems = totalTransactions;
  const totalPages = useMemo(() => Math.max(1, Math.ceil(totalItems / pageSize)), [totalItems, pageSize]);

  return {
    transactions,
    nftActivities,
    combinedActivities,
    isLoading,
    error,
    totalItems,
    totalPages,
    totalTransactions,
    totalNftActivities: nftActivities.length,
    refetch: fetchData,
  };
}
