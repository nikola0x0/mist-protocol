import { useSuiClient } from '@mysten/dapp-kit';
import { useQuery } from '@tanstack/react-query';
import { useEffect } from 'react';
import { useAuth } from '../contexts/AuthContext';

// ===================
// Types
// ===================

export interface TokenBalance {
  symbol: string;
  coin_type: string;
  balance_raw: string;
  balance_formatted: string;
  decimals: number;
}

export interface NFTObject {
  objectId: string;
  type: string;
  name: string;
  description: string;
  imageUrl: string;
}

interface AccountObjectFields {
  owner_address: string | null;
  handle: string;
  xid: string;
  balancesBagId: string | null;
  nftsBagId: string | null;
}

// ===================
// Coin metadata lookup
// ===================

const COIN_METADATA: Record<string, { symbol: string; decimals: number }> = {
  '0x2::sui::SUI': { symbol: 'SUI', decimals: 9 },
  '0x8270feb7375eee355e64fdb69c50abb6b5f9393a722883c1cf45f8e26048810a::wal::WAL': { symbol: 'WAL', decimals: 9 },
  '0xa1ec7fc00a6f40db9693ad1415d0c193ad3906494428cf252621037bd7117e29::usdc::USDC': { symbol: 'USDC', decimals: 6 },
};

// ===================
// Helpers
// ===================

// "0x2::coin::Balance<0x2::sui::SUI>" → "0x2::sui::SUI"
function extractCoinType(balanceType: string): string | null {
  const match = balanceType.match(/Balance<(.+?)>/);
  return match ? match[1] : null;
}

// Get human-readable symbol (SUI, WAL, USDC) or fallback to last part of type
function getSymbolFromType(coinType: string): string {
  if (COIN_METADATA[coinType]) return COIN_METADATA[coinType].symbol;
  const parts = coinType.split('::');
  return parts[parts.length - 1] || 'UNKNOWN';
}

// Get decimals for a coin type (default 9 for unknown coins)
function getDecimalsFromType(coinType: string): number {
  return COIN_METADATA[coinType]?.decimals ?? 9;
}

// Convert raw balance (e.g. "1000000000") to formatted (e.g. "1.0")
function formatBalance(rawBalance: string, decimals: number): string {
  const balance = BigInt(rawBalance);
  const divisor = BigInt(10 ** decimals);
  const whole = balance / divisor;
  const fraction = balance % divisor;

  if (fraction === 0n) return whole.toString();

  const fractionStr = fraction.toString().padStart(decimals, '0').replace(/0+$/, '');
  return `${whole}.${fractionStr}`;
}

// ===================
// BASE HOOK: Account Object
// ===================
// This is the foundation which fetches the XWallet account object from chain.
// Other hooks depend on this and React Query caches it automatically.

export function useXWalletAccountObject(suiObjectId: string | null | undefined) {
  const suiClient = useSuiClient();

  return useQuery({
    queryKey: ['xwallet-account-object', suiObjectId],
    queryFn: async (): Promise<AccountObjectFields> => {
      if (!suiObjectId) throw new Error('No suiObjectId provided');

      const accountObj = await suiClient.getObject({
        id: suiObjectId,
        options: { showContent: true, showType: true },
      });

      if (!accountObj.data?.content || accountObj.data.content.dataType !== 'moveObject') {
        throw new Error('Invalid account object');
      }

      const fields = accountObj.data.content.fields as Record<string, unknown>;

      // Extract the Bag IDs for balances and NFTs (nested structure in Move)
      const balancesBag = fields.balances as Record<string, unknown> | undefined;
      const balancesBagId = ((balancesBag?.fields as Record<string, unknown>)?.id as Record<string, unknown>)?.id as string | null;

      const nftsBag = fields.nfts as Record<string, unknown> | undefined;
      const nftsBagId = ((nftsBag?.fields as Record<string, unknown>)?.id as Record<string, unknown>)?.id as string | null;

      return {
        owner_address: (fields.owner_address as string) || null,
        handle: (fields.handle as string) || '',
        xid: (fields.xid as string) || '',
        balancesBagId,
        nftsBagId,
      };
    },
    enabled: !!suiObjectId,
    staleTime: 5000,
    refetchOnWindowFocus: true,
  });
}

// ===================
// BALANCES HOOK
// ===================
// Fetches all token balances from the account's Bag.
// Waits for base hook to get the balancesBagId first.

export function useXWalletBalances(suiObjectId: string | null | undefined) {
  const suiClient = useSuiClient();
  const { data: accountObj } = useXWalletAccountObject(suiObjectId);

  return useQuery({
    queryKey: ['xwallet-balances', suiObjectId],
    queryFn: async (): Promise<TokenBalance[]> => {
      const bagId = accountObj?.balancesBagId;
      if (!bagId) return [];

      const balances: TokenBalance[] = [];
      const { data: dynamicFields } = await suiClient.getDynamicFields({ parentId: bagId });

      // Loop through each balance in the Bag
      for (const field of dynamicFields || []) {
        try {
          const balanceObj = await suiClient.getObject({
            id: field.objectId,
            options: { showContent: true, showType: true },
          });

          if (!balanceObj.data?.content || balanceObj.data.content.dataType !== 'moveObject') continue;

          const balanceFields = balanceObj.data.content.fields as Record<string, unknown>;
          const balanceType = balanceObj.data.content.type;

          const coinType = extractCoinType(balanceType);
          if (!coinType) continue;

          // Get raw balance value (handles different Move struct layouts)
          const valueField = balanceFields.value as Record<string, unknown> | undefined;
          const rawBalance = (valueField?.fields as Record<string, unknown>)?.value as string
            || balanceFields.value as string
            || '0';

          const decimals = getDecimalsFromType(coinType);
          const symbol = getSymbolFromType(coinType);

          balances.push({
            symbol,
            coin_type: coinType,
            balance_raw: rawBalance,
            balance_formatted: formatBalance(rawBalance, decimals),
            decimals,
          });
        } catch {
          // Skip failed balance fetch
        }
      }

      // SUI first, then alphabetically
      balances.sort((a, b) => {
        if (a.symbol === 'SUI') return -1;
        if (b.symbol === 'SUI') return 1;
        return a.symbol.localeCompare(b.symbol);
      });

      return balances;
    },
    enabled: !!suiObjectId && !!accountObj?.balancesBagId,
    staleTime: 5000,
    refetchOnWindowFocus: true,
  });
}

// ===================
// NFTs HOOK
// ===================
// Fetches all NFTs from the account's ObjectBag.
// Waits for base hook to get the nftsBagId first.

export function useXWalletNFTs(suiObjectId: string | null | undefined) {
  const suiClient = useSuiClient();
  const { data: accountObj } = useXWalletAccountObject(suiObjectId);

  return useQuery({
    queryKey: ['xwallet-nfts', suiObjectId],
    queryFn: async (): Promise<NFTObject[]> => {
      const nftsBagId = accountObj?.nftsBagId;
      if (!nftsBagId) return [];

      const nfts: NFTObject[] = [];
      const { data: dynamicFields } = await suiClient.getDynamicFields({ parentId: nftsBagId });

      for (const field of dynamicFields || []) {
        try {
          const nftObj = await suiClient.getObject({
            id: field.objectId,
            options: { showContent: true, showType: true, showDisplay: true },
          });

          if (!nftObj.data) continue;

          const content = nftObj.data.content;
          if (content?.dataType !== 'moveObject') continue;

          const nftFields = content.fields as Record<string, unknown>;
          const display = nftObj.data.display?.data;

          // Try display data first, then fall back to object fields
          const name = (display?.name || nftFields?.name || 'Unknown NFT') as string;
          const description = (display?.description || nftFields?.description || '') as string;
          const imageUrl = (display?.image_url || nftFields?.url || nftFields?.image_url || '') as string;

          nfts.push({
            objectId: field.objectId,
            type: nftObj.data.type || content.type || '',
            name,
            description,
            imageUrl,
          });
        } catch {
          // Skip failed NFT fetch
        }
      }

      return nfts;
    },
    enabled: !!suiObjectId && !!accountObj?.nftsBagId,
    staleTime: 5000,
    refetchOnWindowFocus: true,
  });
}

// ===================
// OWNER SYNC HOOK
// ===================
// Syncs the on-chain owner_address to local AuthContext.
// Only use for logged-in user's own account!

export function useXWalletOwnerSync(suiObjectId: string | null | undefined) {
  const { data: accountObj } = useXWalletAccountObject(suiObjectId);
  const { user, linkWallet } = useAuth();

  useEffect(() => {
    const onChainOwner = accountObj?.owner_address;
    const localOwner = user?.linkedWalletAddress;

    // If chain has different owner than local, sync it
    if (onChainOwner && onChainOwner !== localOwner) {
      linkWallet(onChainOwner);
    }
  }, [accountObj?.owner_address, user?.linkedWalletAddress, linkWallet]);

  return {
    ownerAddress: accountObj?.owner_address ?? null,
    handle: accountObj?.handle ?? '',
    xid: accountObj?.xid ?? '',
  };
}

// ===================
// CONVENIENCE HOOKS
// ===================

// For YOUR OWN account - includes owner sync
export function useXWalletAccount(suiObjectId: string | null | undefined) {
  const { data: accountObj, isLoading: isLoadingAccount, isError, error, refetch } = useXWalletAccountObject(suiObjectId);
  const { data: balances = [], isLoading: isLoadingBalances } = useXWalletBalances(suiObjectId);
  const { data: nfts = [], isLoading: isLoadingNFTs } = useXWalletNFTs(suiObjectId);

  // Sync owner to AuthContext
  useXWalletOwnerSync(suiObjectId);

  return {
    ownerAddress: accountObj?.owner_address ?? null,
    handle: accountObj?.handle ?? '',
    xid: accountObj?.xid ?? '',
    balances,
    nfts,
    isLoading: isLoadingAccount || isLoadingBalances || isLoadingNFTs,
    isLoadingAccount,
    isLoadingBalances,
    isLoadingNFTs,
    isError,
    error,
    refetch,
  };
}

// For viewing OTHER users' accounts - no owner sync
export function useXWalletAccountView(suiObjectId: string | null | undefined) {
  const { data: accountObj, isLoading: isLoadingAccount, isError, error, refetch } = useXWalletAccountObject(suiObjectId);
  const { data: balances = [], isLoading: isLoadingBalances } = useXWalletBalances(suiObjectId);
  const { data: nfts = [], isLoading: isLoadingNFTs } = useXWalletNFTs(suiObjectId);

  return {
    ownerAddress: accountObj?.owner_address ?? null,
    handle: accountObj?.handle ?? '',
    xid: accountObj?.xid ?? '',
    balances,
    nfts,
    isLoading: isLoadingAccount || isLoadingBalances || isLoadingNFTs,
    isLoadingAccount,
    isLoadingBalances,
    isLoadingNFTs,
    isError,
    error,
    refetch,
  };
}
