import { useSuiClient } from '@mysten/dapp-kit';
import { useQuery } from '@tanstack/react-query';
import { useWallet } from '../contexts/WalletContext';
import { COIN_TYPES } from '../utils/constants';

// Token icons mapping (for known tokens)
import suiIcon from '../assets/tokens/sui.png';
import walIcon from '../assets/tokens/wal.png';
import usdcIcon from '../assets/tokens/usdc.png';
import unknownIcon from '../assets/tokens/unknown.png';

export interface WalletCoin {
  coinType: string;
  symbol: string;
  name: string;
  decimals: number;
  iconUrl: string | null;
  balance: bigint;
  balanceFormatted: string;
  isVerified: boolean;
  hasUnknownDecimals: boolean; // True if decimals defaulted to 9 due to missing metadata
  coins: Array<{
    coinObjectId: string;
    balance: string;
  }>;
}

// Verified tokens with their metadata (exact address match only)
const VERIFIED_TOKENS: Record<string, { symbol: string; decimals: number; icon: string }> = {
  [COIN_TYPES.SUI]: { symbol: 'SUI', decimals: 9, icon: suiIcon },
  [COIN_TYPES.WAL]: { symbol: 'WAL', decimals: 9, icon: walIcon },
  [COIN_TYPES.USDC]: { symbol: 'USDC', decimals: 6, icon: usdcIcon },
};

// Check if coin type is a verified token (exact match only)
function isVerifiedToken(coinType: string): boolean {
  return coinType in VERIFIED_TOKENS;
}

// Get verified token info (exact match only - no suffix matching)
function getVerifiedTokenInfo(coinType: string): { symbol: string; decimals: number; icon: string } | null {
  return VERIFIED_TOKENS[coinType] || null;
}

// Format balance based on decimals
function formatBalance(balance: bigint, decimals: number): string {
  const divisor = BigInt(10 ** decimals);
  const whole = balance / divisor;
  const remainder = balance % divisor;

  if (remainder === 0n) {
    return whole.toString();
  }

  const decimalStr = remainder.toString().padStart(decimals, '0');
  const trimmed = decimalStr.replace(/0+$/, '');
  return `${whole}.${trimmed}`;
}

/**
 * Hook to fetch all coins in the connected wallet with metadata
 */
export function useConnectedWalletCoins() {
  const { address } = useWallet();
  const suiClient = useSuiClient();

  return useQuery({
    queryKey: ['wallet-coins', address],
    queryFn: async (): Promise<WalletCoin[]> => {
      if (!address) return [];

      // Fetch all coins owned by the wallet
      const allCoins = await suiClient.getAllCoins({ owner: address });

      // Group coins by type
      const coinsByType = new Map<string, typeof allCoins.data>();
      for (const coin of allCoins.data) {
        const existing = coinsByType.get(coin.coinType) || [];
        existing.push(coin);
        coinsByType.set(coin.coinType, existing);
      }

      // Fetch metadata for each unique coin type
      const walletCoins: WalletCoin[] = [];

      for (const [coinType, coins] of coinsByType) {
        // Calculate total balance
        const totalBalance = coins.reduce(
          (sum, coin) => sum + BigInt(coin.balance),
          0n
        );

        // Skip if zero balance
        if (totalBalance === 0n) continue;

        // Check if this is a verified token first
        const verifiedInfo = getVerifiedTokenInfo(coinType);
        const verified = isVerifiedToken(coinType);

        let symbol: string;
        let name: string;
        let decimals: number;
        let iconUrl: string | null = null;
        let hasUnknownDecimals = false;

        if (verifiedInfo) {
          // Use verified token info (exact match)
          symbol = verifiedInfo.symbol;
          name = verifiedInfo.symbol;
          decimals = verifiedInfo.decimals;
          iconUrl = verifiedInfo.icon;
        } else {
          // Try to fetch coin metadata from chain
          try {
            const coinMetadata = await suiClient.getCoinMetadata({ coinType });
            if (coinMetadata) {
              symbol = coinMetadata.symbol;
              name = coinMetadata.name;
              decimals = coinMetadata.decimals;
            } else {
              // Fallback: extract symbol from coin type
              const parts = coinType.split('::');
              symbol = parts[parts.length - 1] || 'UNKNOWN';
              name = symbol;
              decimals = 9; // Default to 9 decimals
              hasUnknownDecimals = true;
            }
          } catch {
            // Metadata not available, use fallback
            const parts = coinType.split('::');
            symbol = parts[parts.length - 1] || 'UNKNOWN';
            name = symbol;
            decimals = 9; // Default to 9 decimals
            hasUnknownDecimals = true;
          }
          // Use unknown icon for unverified tokens
          iconUrl = unknownIcon;
        }

        walletCoins.push({
          coinType,
          symbol,
          name,
          decimals,
          iconUrl,
          balance: totalBalance,
          balanceFormatted: formatBalance(totalBalance, decimals),
          isVerified: verified,
          hasUnknownDecimals,
          coins: coins.map(c => ({
            coinObjectId: c.coinObjectId,
            balance: c.balance,
          })),
        });
      }

      // Sort: SUI first, then verified tokens, then rest alphabetically
      walletCoins.sort((a, b) => {
        // SUI always first
        if (a.coinType === COIN_TYPES.SUI) return -1;
        if (b.coinType === COIN_TYPES.SUI) return 1;
        // Verified tokens before unverified
        if (a.isVerified && !b.isVerified) return -1;
        if (!a.isVerified && b.isVerified) return 1;
        // Alphabetically by symbol
        return a.symbol.localeCompare(b.symbol);
      });

      return walletCoins;
    },
    enabled: !!address,
    // refetchInterval: 10000, // Refetch every 10 seconds
  });
}
