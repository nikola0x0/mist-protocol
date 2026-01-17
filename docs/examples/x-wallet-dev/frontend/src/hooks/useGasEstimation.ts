import { useState, useEffect } from 'react';
import { useSuiClient } from '@mysten/dapp-kit';
import { Transaction } from '@mysten/sui/transactions';
import { useWallet } from '../contexts/WalletContext';
import { XWALLET_PACKAGE_ID, COIN_TYPES, MIST_PER_SUI } from '../utils/constants';

interface UseGasEstimationParams {
  coinType: string | null;
  amount: string;
  decimals: number;
  suiObjectId: string;
  enabled: boolean;
}

interface UseGasEstimationResult {
  estimatedGas: string | null;
  isEstimating: boolean;
  hasError: boolean;
}

/**
 * Hook for estimating gas fees for deposit transactions
 * Only estimates when `enabled` is true (i.e., when not sponsored)
 */
export function useGasEstimation({
  coinType,
  amount,
  decimals,
  suiObjectId,
  enabled,
}: UseGasEstimationParams): UseGasEstimationResult {
  const { address } = useWallet();
  const suiClient = useSuiClient();
  const [estimatedGas, setEstimatedGas] = useState<string | null>(null);
  const [isEstimating, setIsEstimating] = useState(false);
  const [hasError, setHasError] = useState(false);

  useEffect(() => {
    // Skip estimation if disabled or missing required params
    if (!enabled || !coinType || !amount || !address || !suiObjectId) {
      setEstimatedGas(null);
      setHasError(false);
      return;
    }

    let cancelled = false;

    const estimateGas = async () => {
      setIsEstimating(true);
      setHasError(false);

      try {
        // Convert amount to smallest unit
        const parts = amount.split('.');
        const multiplier = BigInt(10 ** decimals);
        const whole = BigInt(parts[0] || '0') * multiplier;
        const amountSmallest = parts[1]
          ? whole + BigInt(parts[1].padEnd(decimals, '0').slice(0, decimals))
          : whole;

        if (amountSmallest <= 0n) {
          if (!cancelled) setEstimatedGas(null);
          return;
        }

        // Build transaction for estimation
        const tx = new Transaction();
        tx.setSender(address);
        const isSuiCoin = coinType === COIN_TYPES.SUI;

        if (isSuiCoin) {
          // For SUI deposits (non-sponsored): use tx.gas directly
          const [depositCoin] = tx.splitCoins(tx.gas, [tx.pure.u64(amountSmallest)]);
          tx.moveCall({
            target: `${XWALLET_PACKAGE_ID}::xwallet::deposit_coin`,
            typeArguments: [coinType],
            arguments: [tx.object(suiObjectId), depositCoin],
          });
        } else {
          // For other coins: fetch and use specific coin objects
          const coins = await suiClient.getCoins({
            owner: address,
            coinType: coinType,
          });

          if (coins.data.length === 0) {
            if (!cancelled) setEstimatedGas(null);
            return;
          }

          const [depositCoin] = tx.splitCoins(
            tx.object(coins.data[0].coinObjectId),
            [tx.pure.u64(amountSmallest)]
          );
          tx.moveCall({
            target: `${XWALLET_PACKAGE_ID}::xwallet::deposit_coin`,
            typeArguments: [coinType],
            arguments: [tx.object(suiObjectId), depositCoin],
          });
        }

        // Dry run to estimate gas
        const txBytes = await tx.build({ client: suiClient });
        const dryRunResult = await suiClient.dryRunTransactionBlock({
          transactionBlock: txBytes,
        });

        if (cancelled) return;

        const gasUsed = dryRunResult.effects.gasUsed;
        const totalGas =
          BigInt(gasUsed.computationCost) +
          BigInt(gasUsed.storageCost) -
          BigInt(gasUsed.storageRebate);
        const gasSui = (Number(totalGas) / MIST_PER_SUI).toFixed(6);
        setEstimatedGas(gasSui);
      } catch (err) {
        if (cancelled) return;
        const errorMsg = err instanceof Error ? err.message : '';
        if (errorMsg.includes('No valid gas coins')) {
          setEstimatedGas('Need SUI for gas');
        } else {
          setEstimatedGas(null);
          setHasError(true);
        }
      } finally {
        if (!cancelled) setIsEstimating(false);
      }
    };

    // Debounce 500ms
    const debounce = setTimeout(estimateGas, 500);
    return () => {
      cancelled = true;
      clearTimeout(debounce);
    };
  }, [enabled, coinType, amount, decimals, address, suiObjectId, suiClient]);

  return { estimatedGas, isEstimating, hasError };
}
