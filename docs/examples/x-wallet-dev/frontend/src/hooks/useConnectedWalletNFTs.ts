import { useCurrentAccount, useSuiClient } from '@mysten/dapp-kit';
import { useQuery } from '@tanstack/react-query';
import type { NFTObject } from './useXWalletNFTTransactions';

/**
 * Check if an object is likely an NFT that can be deposited.
 * Uses Sui's built-in `hasPublicTransfer` flag instead of hardcoded exclusions.
 */
function isNFTObject(obj: any): boolean {
  const type = obj.data?.type || '';
  const content = obj.data?.content;

  // Skip coins - they have their own deposit flow
  if (type.includes('::coin::Coin') || type.includes('::sui::SUI')) {
    return false;
  }

  // Skip kiosk-related objects - they require special handling
  if (type.toLowerCase().includes('kiosk')) {
    return false;
  }

  // Key check: hasPublicTransfer indicates the object has 'store' ability
  // This is required for deposit_nft<T: key + store>
  if (content?.dataType === 'moveObject' && !content?.hasPublicTransfer) {
    return false;
  }

  // Accept objects with display data (typical NFT indicator) or custom types
  const hasDisplay = obj.data?.display?.data;
  const hasFields = content?.fields;

  return !!(hasDisplay || hasFields);
}

function extractNFTMetadata(obj: any): Partial<NFTObject> {
  const content = obj.data?.content;
  const display = obj.data?.display?.data;
  const fields = content?.fields;

  const name = display?.name || fields?.name || 'Unknown NFT';
  const description = display?.description || fields?.description || '';
  let imageUrl = display?.image_url || fields?.image_url || fields?.url || '';

  // Handle IPFS URLs
  if (imageUrl?.startsWith('ipfs://')) {
    imageUrl = `https://ipfs.io/ipfs/${imageUrl.slice(7)}`;
  }

  return { name, description, imageUrl };
}

/**
 * Hook to fetch NFTs owned by the connected wallet
 */
export function useConnectedWalletNFTs() {
  const currentAccount = useCurrentAccount();
  const suiClient = useSuiClient();

  return useQuery({
    queryKey: ['wallet-nfts', currentAccount?.address],
    queryFn: async (): Promise<NFTObject[]> => {
      if (!currentAccount?.address) {
        return [];
      }

      // Fetch all objects owned by the wallet
      const { data: objects } = await suiClient.getOwnedObjects({
        owner: currentAccount.address,
        options: {
          showType: true,
          showContent: true,
          showDisplay: true,
        },
      });

      // Filter to NFTs only and map to our format
      const nfts: NFTObject[] = [];

      for (const obj of objects) {
        if (!obj.data) continue;

        if (!isNFTObject(obj)) continue;

        const type = obj.data.type || '';
        const metadata = extractNFTMetadata(obj);

        nfts.push({
          objectId: obj.data.objectId,
          type: type,
          name: metadata.name,
          description: metadata.description,
          imageUrl: metadata.imageUrl,
        });
      }

      return nfts;
    },
    enabled: !!currentAccount?.address,
    staleTime: 2000,
    // refetchInterval: 3000,
  });
}
