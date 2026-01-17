import { useSuiClient } from '@mysten/dapp-kit';
import { Transaction } from '@mysten/sui/transactions';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import { XWALLET_PACKAGE_ID, COIN_TYPES } from '../utils/constants';
import { useWallet } from '../contexts/WalletContext';

interface DepositParams {
  suiObjectId: string; // XWallet account object ID
  amount: string; // Amount in human readable format
  coinType?: string; // Coin type (default: SUI)
  decimals?: number; // Decimals for the coin (default: 9)
}

interface WithdrawParams {
  suiObjectId: string; // XWallet account object ID
  amount: string; // Amount in human readable format
  coinType?: string; // Coin type (default: SUI)
  decimals?: number; // Decimals for the coin (default: 9)
}

// Convert human readable amount to smallest unit based on decimals
function toSmallestUnit(amount: string, decimals: number = 9): bigint {
  const parts = amount.split('.');
  const multiplier = BigInt(10 ** decimals);
  const whole = BigInt(parts[0] || '0') * multiplier;
  if (parts[1]) {
    const decimal = parts[1].padEnd(decimals, '0').slice(0, decimals);
    return whole + BigInt(decimal);
  }
  return whole;
}

/**
 * Hook for depositing coins into XWallet account
 *
 * Supports ANY coin type on Sui network.
 *
 * This transaction CAN be sponsored by:
 * 1. Fetching user's coins of the specified type
 * 2. Merging them if needed
 * 3. Splitting the exact amount
 * 4. Depositing the split coin
 *
 * Gas is paid by Enoki sponsor!
 */
export function useDeposit() {
  const { executeTransaction, address, sponsorEnabled } = useWallet();
  const suiClient = useSuiClient();
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: async ({ suiObjectId, amount, coinType = COIN_TYPES.SUI, decimals = 9 }: DepositParams) => {
      if (!XWALLET_PACKAGE_ID) {
        throw new Error('XWALLET_PACKAGE_ID not configured');
      }

      if (!address) {
        throw new Error('Wallet not connected');
      }

      const amountSmallest = toSmallestUnit(amount, decimals);
      if (amountSmallest <= 0n) {
        throw new Error('Amount must be greater than 0');
      }

      // Fetch user's coins of the specified type
      const coins = await suiClient.getCoins({
        owner: address,
        coinType: coinType,
      });

      if (coins.data.length === 0) {
        throw new Error(`No ${coinType.split('::').pop()} coins available`);
      }

      // Calculate total balance
      const totalBalance = coins.data.reduce(
        (sum, coin) => sum + BigInt(coin.balance),
        0n
      );

      if (totalBalance < amountSmallest) {
        throw new Error(`Insufficient balance. Have ${totalBalance}, need ${amountSmallest}`);
      }

      const tx = new Transaction();
      const isSuiCoin = coinType === COIN_TYPES.SUI;

      // When NOT sponsored and depositing SUI: use tx.gas (the gas coin)
      // When sponsored OR depositing other coins: use specific coin objects
      if (!sponsorEnabled && isSuiCoin) {
        // Use gas coin directly - works for non-sponsored SUI deposits
        const [depositCoin] = tx.splitCoins(tx.gas, [tx.pure.u64(amountSmallest)]);

        tx.moveCall({
          target: `${XWALLET_PACKAGE_ID}::xwallet::deposit_coin`,
          typeArguments: [coinType],
          arguments: [
            tx.object(suiObjectId),
            depositCoin,
          ],
        });
      } else if (coins.data.length === 1) {
        // Single coin - split from it (sponsored or non-SUI)
        const [depositCoin] = tx.splitCoins(
          tx.object(coins.data[0].coinObjectId),
          [tx.pure.u64(amountSmallest)]
        );

        tx.moveCall({
          target: `${XWALLET_PACKAGE_ID}::xwallet::deposit_coin`,
          typeArguments: [coinType],
          arguments: [
            tx.object(suiObjectId),
            depositCoin,
          ],
        });
      } else {
        // Multiple coins - merge first, then split
        const primaryCoin = tx.object(coins.data[0].coinObjectId);
        const otherCoins = coins.data.slice(1).map(c => tx.object(c.coinObjectId));

        // Merge all coins into the first one
        tx.mergeCoins(primaryCoin, otherCoins);

        // Split the deposit amount
        const [depositCoin] = tx.splitCoins(primaryCoin, [tx.pure.u64(amountSmallest)]);

        tx.moveCall({
          target: `${XWALLET_PACKAGE_ID}::xwallet::deposit_coin`,
          typeArguments: [coinType],
          arguments: [
            tx.object(suiObjectId),
            depositCoin,
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
      // Invalidate balance, transactions and wallet coins queries to refresh UI
      queryClient.invalidateQueries({ queryKey: ['xwallet-balance', variables.suiObjectId] });
      queryClient.invalidateQueries({ queryKey: ['xwallet-transactions', variables.suiObjectId] });
      queryClient.invalidateQueries({ queryKey: ['wallet-coins'] });
    },
  });
}

/**
 * Hook for withdrawing coins from XWallet account
 *
 * Supports ANY coin type on Sui network.
 *
 * This transaction CAN be sponsored via Enoki since it doesn't
 * use the gas coin as an input argument.
 */
export function useWithdraw() {
  const { executeTransaction, address } = useWallet();
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: async ({ suiObjectId, amount, coinType = COIN_TYPES.SUI, decimals = 9 }: WithdrawParams) => {
      if (!XWALLET_PACKAGE_ID) {
        throw new Error('XWALLET_PACKAGE_ID not configured');
      }

      if (!address) {
        throw new Error('Wallet not connected');
      }

      const amountSmallest = toSmallestUnit(amount, decimals);
      if (amountSmallest <= 0n) {
        throw new Error('Amount must be greater than 0');
      }

      const tx = new Transaction();

      // Call xwallet::xwallet::withdraw_coin<T>
      // This returns a Coin<T> that we need to transfer to the sender
      const [withdrawnCoin] = tx.moveCall({
        target: `${XWALLET_PACKAGE_ID}::xwallet::withdraw_coin`,
        typeArguments: [coinType],
        arguments: [
          tx.object(suiObjectId), // XWallet account
          tx.pure.u64(amountSmallest), // Amount to withdraw
        ],
      });

      // Transfer the withdrawn coin to the connected wallet address
      tx.transferObjects([withdrawnCoin], address);

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
      // Invalidate balance, transactions and wallet coins queries to refresh UI
      queryClient.invalidateQueries({ queryKey: ['xwallet-balance', variables.suiObjectId] });
      queryClient.invalidateQueries({ queryKey: ['xwallet-transactions', variables.suiObjectId] });
      queryClient.invalidateQueries({ queryKey: ['wallet-coins'] });
    },
  });
}
