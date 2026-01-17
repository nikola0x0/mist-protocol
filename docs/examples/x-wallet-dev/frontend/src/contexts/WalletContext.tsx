/**
 * Wallet Context
 *
 * Provides transaction execution with optional sponsorship.
 * This context wraps the Sui wallet and provides methods to:
 * - Sign and execute transactions with gas sponsorship via Enoki
 * - Execute transactions without sponsorship
 * - Auto-toggle between sponsored/non-sponsored based on config
 */

import React, { createContext, useContext, useCallback, useMemo } from 'react';
import { Transaction } from '@mysten/sui/transactions';
import { toB64, fromB64 } from '@mysten/sui/utils';
import {
  useCurrentAccount,
  useCurrentWallet,
  useSignTransaction,
  useSuiClient,
} from '@mysten/dapp-kit';
import type {
  SuiTransactionBlockResponse,
  SuiTransactionBlockResponseOptions,
} from '@mysten/sui/client';
import {
  createSponsoredTransaction,
  executeSponsoredTransaction,
} from '../api/sponsor';
import { SUI_NETWORK } from '../utils/constants';
import { useAppConfig } from './AppConfigContext';
import type {
  SponsorTxRequestBody,
  CreateSponsoredTransactionApiResponse,
} from '../types';

// ====== Types ======

interface SponsorAndExecuteTransactionBlockProps {
  tx: Transaction;
  network?: 'mainnet' | 'testnet';
  options?: SuiTransactionBlockResponseOptions;
  includesTransferTx?: boolean;
  allowedAddresses?: string[];
}

interface ExecuteTransactionBlockWithoutSponsorshipProps {
  tx: Transaction;
  options?: SuiTransactionBlockResponseOptions;
}

interface ExecuteTransactionProps {
  tx: Transaction;
  options?: SuiTransactionBlockResponseOptions;
  allowedAddresses?: string[];
}

interface WalletContextProps {
  isConnected: boolean;
  address?: string;
  sponsorEnabled: boolean;
  sponsorAndExecuteTransactionBlock: (
    props: SponsorAndExecuteTransactionBlockProps
  ) => Promise<SuiTransactionBlockResponse>;
  executeTransactionBlockWithoutSponsorship: (
    props: ExecuteTransactionBlockWithoutSponsorshipProps
  ) => Promise<SuiTransactionBlockResponse | void>;
  executeTransaction: (
    props: ExecuteTransactionProps
  ) => Promise<SuiTransactionBlockResponse>;
}

// ====== Context ======

const WalletContext = createContext<WalletContextProps>({
  isConnected: false,
  address: undefined,
  sponsorEnabled: true,
  sponsorAndExecuteTransactionBlock: async () => {
    throw new Error('WalletProvider not initialized');
  },
  executeTransactionBlockWithoutSponsorship: async () => {
    throw new Error('WalletProvider not initialized');
  },
  executeTransaction: async () => {
    throw new Error('WalletProvider not initialized');
  },
});

export const useWallet = () => {
  const context = useContext(WalletContext);
  if (!context) {
    throw new Error('useWallet must be used within WalletProvider');
  }
  return context;
};

// ====== Provider ======

interface WalletProviderProps {
  children: React.ReactNode;
}

export function WalletProvider({ children }: WalletProviderProps) {
  const suiClient = useSuiClient();
  const currentAccount = useCurrentAccount();
  const { isConnected: isWalletConnected } = useCurrentWallet();
  const { mutateAsync: signTransactionBlock } = useSignTransaction();
  const { sponsorEnabled } = useAppConfig();

  // Derive connection state and address
  const { isConnected, address } = useMemo(() => {
    return {
      isConnected: isWalletConnected && !!currentAccount?.address,
      address: currentAccount?.address,
    };
  }, [isWalletConnected, currentAccount?.address]);

  // Sign transaction with connected wallet
  const signTransaction = useCallback(
    async (bytes: Uint8Array): Promise<string> => {
      const txBlock = Transaction.from(bytes);
      const result = await signTransactionBlock({
        transaction: txBlock,
        chain: `sui:${SUI_NETWORK}`,
      });
      return result.signature;
    },
    [signTransactionBlock]
  );

  /**
   * Sponsor and execute a transaction block
   * 
   * Flow:
   * 1. Build transaction kind bytes
   * 2. Request sponsorship from backend (which uses Enoki)
   * 3. Sign the sponsored transaction
   * 4. Execute via backend
   * 5. Wait for confirmation and return result
   */
  const sponsorAndExecuteTransactionBlock = useCallback(
    async ({
      tx,
      network = SUI_NETWORK as 'mainnet' | 'testnet',
      options = { showEffects: true, showEvents: true },
      allowedAddresses = [],
    }: SponsorAndExecuteTransactionBlockProps): Promise<SuiTransactionBlockResponse> => {
      if (!isConnected || !address) {
        throw new Error('Wallet is not connected');
      }

      try {
        // Step 1: Build transaction kind bytes (without gas info)
        const txBytes = await tx.build({
          client: suiClient,
          onlyTransactionKind: true,
        });

        // Step 2: Request sponsorship from backend
        const sponsorTxBody: SponsorTxRequestBody = {
          network,
          txBytes: toB64(txBytes),
          sender: address,
          allowedAddresses,
        };

        const sponsorResponse: CreateSponsoredTransactionApiResponse =
          await createSponsoredTransaction(sponsorTxBody);

        // Step 3: Sign the sponsored transaction bytes
        const signature = await signTransaction(fromB64(sponsorResponse.bytes));

        // Step 4: Execute via backend
        const executeResponse = await executeSponsoredTransaction({
          signature,
          digest: sponsorResponse.digest,
        });

        const finalDigest = executeResponse.digest;

        // Step 5: Wait for confirmation and get full transaction details
        await suiClient.waitForTransaction({
          digest: finalDigest,
          timeout: 10_000,
        });

        return suiClient.getTransactionBlock({
          digest: finalDigest,
          options,
        });
      } catch (err) {
        throw new Error(
          err instanceof Error
            ? err.message
            : 'Failed to sponsor and execute transaction'
        );
      }
    },
    [isConnected, address, suiClient, signTransaction]
  );

  /**
   * Execute a transaction without sponsorship
   * 
   * Some transactions cannot be sponsored (e.g., when using gas coin as argument).
   * This method executes the transaction directly using the user's gas.
   */
  const executeTransactionBlockWithoutSponsorship = useCallback(
    async ({
      tx,
      options = { showEffects: true, showEvents: true },
    }: ExecuteTransactionBlockWithoutSponsorshipProps): Promise<SuiTransactionBlockResponse | void> => {
      if (!isConnected || !address) {
        return;
      }

      try {
        tx.setSender(address);
        const txBytes = await tx.build({ client: suiClient });
        const signature = await signTransaction(txBytes);

        return suiClient.executeTransactionBlock({
          transactionBlock: txBytes,
          signature,
          requestType: 'WaitForLocalExecution',
          options,
        });
      } catch (err) {
        throw new Error(
          err instanceof Error
            ? err.message
            : 'Failed to execute transaction'
        );
      }
    },
    [isConnected, address, suiClient, signTransaction]
  );

  /**
   * Execute a transaction - automatically uses sponsor or user gas based on config
   */
  const executeTransaction = useCallback(
    async ({
      tx,
      options = { showEffects: true, showEvents: true },
      allowedAddresses = [],
    }: ExecuteTransactionProps): Promise<SuiTransactionBlockResponse> => {
      if (!isConnected || !address) {
        throw new Error('Wallet is not connected');
      }

      if (sponsorEnabled) {
        // Use sponsored execution
        return sponsorAndExecuteTransactionBlock({
          tx,
          options,
          allowedAddresses,
        });
      } else {
        // User pays gas
        const result = await executeTransactionBlockWithoutSponsorship({
          tx,
          options,
        });
        if (!result) {
          throw new Error('Transaction execution failed');
        }
        return result;
      }
    },
    [isConnected, address, sponsorEnabled, sponsorAndExecuteTransactionBlock, executeTransactionBlockWithoutSponsorship]
  );

  const contextValue = useMemo(
    () => ({
      isConnected,
      address,
      sponsorEnabled,
      sponsorAndExecuteTransactionBlock,
      executeTransactionBlockWithoutSponsorship,
      executeTransaction,
    }),
    [
      isConnected,
      address,
      sponsorEnabled,
      sponsorAndExecuteTransactionBlock,
      executeTransactionBlockWithoutSponsorship,
      executeTransaction,
    ]
  );

  return (
    <WalletContext.Provider value={contextValue}>
      {children}
    </WalletContext.Provider>
  );
}

export default WalletProvider;
