import { useSuiClient } from '@mysten/dapp-kit';
import { Transaction } from '@mysten/sui/transactions';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import { XWALLET_PACKAGE_ID } from '../utils/constants';
import { useWallet } from '../contexts/WalletContext';

export interface NFTObject {
  objectId: string;
  type: string;
  name?: string;
  description?: string;
  imageUrl?: string;
}

interface DepositNFTParams {
  suiObjectId: string; // XWallet account object ID
  nfts: NFTObject[]; // NFTs to deposit (supports single or multiple)
}

interface WithdrawNFTParams {
  suiObjectId: string; // XWallet account object ID
  nfts: NFTObject[]; // NFTs to withdraw (supports single or multiple)
}

/**
 * Hook for depositing NFTs into XWallet account
 *
 * Gas is paid by Enoki sponsor!
 */
export function useDepositNFT() {
  const { executeTransaction } = useWallet();
  const queryClient = useQueryClient();
  const suiClient = useSuiClient();

  return useMutation({
    mutationFn: async ({ suiObjectId, nfts }: DepositNFTParams) => {
      if (!XWALLET_PACKAGE_ID) {
        throw new Error('XWALLET_PACKAGE_ID not configured');
      }

      if (nfts.length === 0) {
        throw new Error('No NFTs selected for deposit');
      }

      // Fetch account object to get shared object version
      const accountObj = await suiClient.getObject({
        id: suiObjectId,
        options: { showOwner: true },
      });

      if (!accountObj.data) {
        throw new Error('XWallet account not found');
      }

      const owner = accountObj.data.owner;
      const isShared = owner && typeof owner === 'object' && 'Shared' in owner;

      const tx = new Transaction();

      // Deposit each NFT
      for (const nft of nfts) {
        // Validate type doesn't have issues
        if (!nft.type || nft.type.includes('::coin::') || nft.type.includes('::Coin<')) {
          throw new Error(`Invalid NFT type: ${nft.type}. Coins cannot be deposited as NFTs.`);
        }

        // Use sharedObjectRef for shared objects
        const accountArg = isShared
          ? tx.sharedObjectRef({
              objectId: suiObjectId,
              initialSharedVersion: Number((owner as { Shared: { initial_shared_version: string | number } }).Shared.initial_shared_version),
              mutable: true,
            })
          : tx.object(suiObjectId);

        tx.moveCall({
          target: `${XWALLET_PACKAGE_ID}::xwallet::deposit_nft`,
          typeArguments: [nft.type],
          arguments: [
            accountArg, // XWallet account (shared object)
            tx.object(nft.objectId), // The NFT to deposit
          ],
        });
      }

      // Execute transaction (sponsored or user-paid based on config)
      const result = await executeTransaction({
        tx,
        options: {
          showEffects: true,
          showEvents: true,
          showObjectChanges: true,
        },
      });
      return result;
    },
    onSuccess: (_, variables) => {
      // Invalidate NFT queries to refresh UI
      queryClient.invalidateQueries({ queryKey: ['xwallet-nfts', variables.suiObjectId] });
      queryClient.invalidateQueries({ queryKey: ['wallet-nfts'] });
    },
  });
}

/**
 * Hook for withdrawing NFTs from XWallet account
 */
export function useWithdrawNFT() {
  const { executeTransaction, address } = useWallet();
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: async ({ suiObjectId, nfts }: WithdrawNFTParams) => {
      if (!XWALLET_PACKAGE_ID) {
        throw new Error('XWALLET_PACKAGE_ID not configured');
      }

      if (!address) {
        throw new Error('Wallet not connected');
      }

      if (nfts.length === 0) {
        throw new Error('No NFTs selected for withdrawal');
      }

      const tx = new Transaction();

      // Withdraw each NFT and transfer to connected wallet
      for (const nft of nfts) {
        const [withdrawnNft] = tx.moveCall({
          target: `${XWALLET_PACKAGE_ID}::xwallet::withdraw_nft`,
          typeArguments: [nft.type],
          arguments: [
            tx.object(suiObjectId), // XWallet account
            tx.pure.address(nft.objectId), // NFT ID to withdraw
          ],
        });

        // Transfer the withdrawn NFT to the connected wallet
        tx.transferObjects([withdrawnNft], address);
      }

      // Execute transaction (sponsored or user-paid based on config)
      const result = await executeTransaction({
        tx,
        options: {
          showEffects: true,
          showEvents: true,
          showObjectChanges: true,
        },
      });

      return result;
    },
    onSuccess: (_, variables) => {
      // Invalidate NFT queries to refresh UI
      queryClient.invalidateQueries({ queryKey: ['xwallet-nfts', variables.suiObjectId] });
      queryClient.invalidateQueries({ queryKey: ['wallet-nfts'] });
    },
  });
}
