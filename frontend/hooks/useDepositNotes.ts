"use client";

import { useState, useEffect, useCallback } from "react";
import { useCurrentAccount, useSignAndExecuteTransaction } from "@mysten/dapp-kit";
import { SuiClient } from "@mysten/sui/client";
import { Transaction } from "@mysten/sui/transactions";
import {
  DepositNote,
  loadDepositNotes,
  addDepositNote,
  markNoteSpent,
  getUnspentNotes,
  generateNullifier,
  generateStealthAddress,
  encryptDepositData,
  encryptSwapIntent,
  saveStealthKeys,
  formatAmount,
  parseAmount,
  SwapIntentDetails,
} from "../lib/deposit-notes";

// ============ CONFIG ============

const PACKAGE_ID = process.env.NEXT_PUBLIC_PACKAGE_ID || "";
const POOL_ID = process.env.NEXT_PUBLIC_POOL_ID || "";
const NETWORK = (process.env.NEXT_PUBLIC_NETWORK as "testnet" | "mainnet") || "testnet";

const RPC_URL =
  NETWORK === "mainnet"
    ? "https://fullnode.mainnet.sui.io"
    : "https://fullnode.testnet.sui.io";

// ============ TYPES ============

export interface UseDepositNotesReturn {
  /** All deposit notes (including spent) */
  notes: DepositNote[];
  /** Only unspent notes */
  unspentNotes: DepositNote[];
  /** Loading state */
  loading: boolean;
  /** Error message if any */
  error: string | null;
  /** Refresh notes from localStorage */
  refresh: () => void;
  /** Create a new deposit */
  deposit: (amountSui: string) => Promise<{ success: boolean; note?: DepositNote; error?: string }>;
  /** Create a swap intent */
  createSwapIntent: (
    note: DepositNote,
    swapAmountSui: string
  ) => Promise<{ success: boolean; error?: string }>;
}

// ============ HOOK ============

export function useDepositNotes(): UseDepositNotesReturn {
  const [notes, setNotes] = useState<DepositNote[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const currentAccount = useCurrentAccount();
  const walletAddress = currentAccount?.address || "";
  const { mutateAsync: signAndExecute } = useSignAndExecuteTransaction();

  const suiClient = new SuiClient({ url: RPC_URL });

  // Load notes when wallet changes
  useEffect(() => {
    if (walletAddress) {
      setNotes(loadDepositNotes(walletAddress));
    } else {
      setNotes([]);
    }
  }, [walletAddress]);

  const refresh = useCallback(() => {
    if (walletAddress) {
      setNotes(loadDepositNotes(walletAddress));
    }
  }, [walletAddress]);

  const unspentNotes = notes.filter((n) => !n.spent);

  /**
   * Create a new deposit
   * 1. Generate random nullifier
   * 2. SEAL encrypt (amount, nullifier)
   * 3. Call deposit_sui on contract
   * 4. Save deposit note locally
   */
  const deposit = useCallback(
    async (
      amountSui: string
    ): Promise<{ success: boolean; note?: DepositNote; error?: string }> => {
      if (!currentAccount) {
        return { success: false, error: "Wallet not connected" };
      }

      setLoading(true);
      setError(null);

      try {
        // 1. Generate nullifier
        const nullifier = generateNullifier();
        const amountMist = parseAmount(amountSui, 9);

        console.log("Creating deposit:", {
          amountSui,
          amountMist,
          nullifierPrefix: nullifier.substring(0, 10) + "...",
        });

        // 2. SEAL encrypt
        const encryptedData = await encryptDepositData(
          amountMist,
          nullifier,
          { packageId: PACKAGE_ID, network: NETWORK },
          suiClient
        );

        // 3. Build transaction
        const tx = new Transaction();

        // Split coins for deposit
        const [coin] = tx.splitCoins(tx.gas, [tx.pure.u64(BigInt(amountMist))]);

        // Call deposit_sui
        tx.moveCall({
          target: `${PACKAGE_ID}::mist_protocol::deposit_sui`,
          arguments: [
            tx.object(POOL_ID),
            coin,
            tx.pure.vector("u8", Array.from(new TextEncoder().encode(encryptedData))),
          ],
        });

        // 4. Execute transaction
        const result = await signAndExecute({
          transaction: tx,
        });

        console.log("Deposit transaction:", result);

        // 5. Save deposit note locally (wallet-scoped)
        const note: DepositNote = {
          nullifier,
          amount: amountMist,
          tokenType: "SUI",
          timestamp: Date.now(),
          depositId: result.digest, // Use tx digest as reference
          spent: false,
        };

        addDepositNote(walletAddress, note);
        refresh();

        return { success: true, note };
      } catch (err) {
        const errorMsg = err instanceof Error ? err.message : "Deposit failed";
        setError(errorMsg);
        console.error("Deposit error:", err);
        return { success: false, error: errorMsg };
      } finally {
        setLoading(false);
      }
    },
    [currentAccount, walletAddress, signAndExecute, suiClient, refresh]
  );

  /**
   * Create a swap intent
   * 1. Generate stealth addresses
   * 2. SEAL encrypt (nullifier, amounts, stealth addresses)
   * 3. Call create_swap_intent on contract
   * 4. Save stealth keys locally
   */
  const createSwapIntent = useCallback(
    async (
      note: DepositNote,
      swapAmountSui: string
    ): Promise<{ success: boolean; error?: string }> => {
      if (!currentAccount) {
        return { success: false, error: "Wallet not connected" };
      }

      if (note.spent) {
        return { success: false, error: "Note already spent" };
      }

      setLoading(true);
      setError(null);

      try {
        const swapAmountMist = parseAmount(swapAmountSui, 9);
        const depositAmount = BigInt(note.amount);
        const swapAmount = BigInt(swapAmountMist);

        if (swapAmount > depositAmount) {
          return { success: false, error: "Swap amount exceeds deposit" };
        }

        // 1. Generate stealth addresses
        const outputStealth = generateStealthAddress();
        const remainderStealth = generateStealthAddress();

        console.log("Creating swap intent:", {
          nullifierPrefix: note.nullifier.substring(0, 10) + "...",
          swapAmount: swapAmountMist,
          outputStealth: outputStealth.address.substring(0, 10) + "...",
        });

        // 2. SEAL encrypt swap details
        const swapDetails: SwapIntentDetails = {
          nullifier: note.nullifier,
          inputAmount: swapAmountMist,
          outputStealth: outputStealth.address,
          remainderStealth: remainderStealth.address,
        };

        const encryptedDetails = await encryptSwapIntent(
          swapDetails,
          { packageId: PACKAGE_ID, network: NETWORK },
          suiClient
        );

        // 3. Build transaction
        const tx = new Transaction();

        // Deadline: 1 hour from now (in milliseconds)
        const deadline = Date.now() + 60 * 60 * 1000;

        tx.moveCall({
          target: `${PACKAGE_ID}::mist_protocol::create_swap_intent`,
          arguments: [
            tx.pure.vector("u8", Array.from(new TextEncoder().encode(encryptedDetails))),
            tx.pure.vector("u8", Array.from(new TextEncoder().encode("SUI"))),
            tx.pure.vector("u8", Array.from(new TextEncoder().encode("SUI"))), // SUI→SUI for now
            tx.pure.u64(deadline),
          ],
        });

        // 4. Execute transaction
        const result = await signAndExecute({
          transaction: tx,
        });

        console.log("Swap intent transaction:", result);

        // 5. Save stealth keys for scanning later (wallet-scoped)
        saveStealthKeys(walletAddress, outputStealth, remainderStealth);

        // Note: We mark spent when TEE confirms, but for now mark optimistically
        // In production, listen for SwapExecutedEvent with matching nullifier
        markNoteSpent(walletAddress, note.nullifier);
        refresh();

        return { success: true };
      } catch (err) {
        const errorMsg = err instanceof Error ? err.message : "Swap intent failed";
        setError(errorMsg);
        console.error("Swap intent error:", err);
        return { success: false, error: errorMsg };
      } finally {
        setLoading(false);
      }
    },
    [currentAccount, walletAddress, signAndExecute, suiClient, refresh]
  );

  return {
    notes,
    unspentNotes,
    loading,
    error,
    refresh,
    deposit,
    createSwapIntent,
  };
}
