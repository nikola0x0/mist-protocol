"use client";

import { useState, useEffect, useCallback } from "react";
import { useCurrentAccount, useSuiClient, useSignTransaction } from "@mysten/dapp-kit";
import { Transaction } from "@mysten/sui/transactions";
import { fromBase64 } from "@mysten/sui/utils";
import {
  loadStealthKeys,
  getStealthKeypair,
  formatAmount,
  StealthAddress,
  removeStealthKeyPair,
} from "../lib/deposit-notes";
import { RefreshCw, Inbox } from "lucide-react";

// ============ TYPES ============

interface StealthOutput {
  outputStealth: StealthAddress;
  remainderStealth: StealthAddress;
  outputBalance: string;
  remainderBalance: string;
  timestamp: number;
}

// ============ COMPONENT ============

export function StealthOutputsCard() {
  const [outputs, setOutputs] = useState<StealthOutput[]>([]);
  const [loading, setLoading] = useState(false);
  const [claiming, setClaiming] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const currentAccount = useCurrentAccount();
  const walletAddress = currentAccount?.address || "";
  const suiClient = useSuiClient();
  const { mutateAsync: signTransaction } = useSignTransaction();

  // Load stealth outputs and check balances
  const loadOutputs = useCallback(async () => {
    if (!walletAddress) {
      setOutputs([]);
      return;
    }

    setLoading(true);
    setError(null);

    try {
      const stealthKeys = loadStealthKeys(walletAddress);

      // Fetch balances for each stealth address
      const outputsWithBalances: StealthOutput[] = await Promise.all(
        stealthKeys.map(async (keys) => {
          // Query balance for output stealth address
          let outputBalance = "0";
          try {
            const outputCoins = await suiClient.getCoins({
              owner: keys.output.address,
              coinType: "0x2::sui::SUI",
            });
            outputBalance = outputCoins.data
              .reduce((sum, coin) => sum + BigInt(coin.balance), BigInt(0))
              .toString();
          } catch {
            // Address may not exist yet
          }

          // Query balance for remainder stealth address
          let remainderBalance = "0";
          try {
            const remainderCoins = await suiClient.getCoins({
              owner: keys.remainder.address,
              coinType: "0x2::sui::SUI",
            });
            remainderBalance = remainderCoins.data
              .reduce((sum, coin) => sum + BigInt(coin.balance), BigInt(0))
              .toString();
          } catch {
            // Address may not exist yet
          }

          return {
            outputStealth: keys.output,
            remainderStealth: keys.remainder,
            outputBalance,
            remainderBalance,
            timestamp: keys.timestamp,
          };
        })
      );

      // Filter to only show outputs with balances or recent ones (last 24h)
      const oneDayAgo = Date.now() - 24 * 60 * 60 * 1000;
      const filtered = outputsWithBalances.filter(
        (o) =>
          BigInt(o.outputBalance) > 0 ||
          BigInt(o.remainderBalance) > 0 ||
          o.timestamp > oneDayAgo
      );

      setOutputs(filtered.reverse()); // Show newest first
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to load outputs");
    } finally {
      setLoading(false);
    }
  }, [walletAddress, suiClient]);

  // Load on mount and when wallet changes
  useEffect(() => {
    loadOutputs();
  }, [loadOutputs]);

  // Claim coins from a stealth address using sponsored transaction
  // User's main wallet pays for gas, stealth address is the sender
  const claimCoins = async (stealth: StealthAddress) => {
    if (!walletAddress) return;

    setClaiming(stealth.address);
    setError(null);

    try {
      // Get the keypair for the stealth address
      const stealthKeypair = getStealthKeypair(stealth);

      // Query all coins at the stealth address
      const coins = await suiClient.getCoins({
        owner: stealth.address,
        coinType: "0x2::sui::SUI",
      });

      if (coins.data.length === 0) {
        throw new Error("No coins to claim");
      }

      // Build transaction:
      // - Sender: stealth address (owns the coins)
      // - Gas sponsor: user's main wallet (pays for gas)
      const tx = new Transaction();
      tx.setSender(stealth.address);
      tx.setGasOwner(walletAddress); // User's wallet sponsors gas

      // Transfer all coins to the main wallet
      const coinRefs = coins.data.map((coin) => tx.object(coin.coinObjectId));
      tx.transferObjects(coinRefs, walletAddress);

      // Step 1: Get sponsor (user wallet) signature via dapp-kit
      // This prompts the user to sign
      const { signature: sponsorSignature, bytes: txBytesBase64 } = await signTransaction({
        transaction: tx as any, // Type cast due to version mismatch
      });

      // Step 2: Sign with stealth keypair (convert base64 to Uint8Array)
      const txBytes = fromBase64(txBytesBase64);
      const stealthSignature = await stealthKeypair.signTransaction(txBytes);

      // Step 3: Execute with both signatures
      const result = await suiClient.executeTransactionBlock({
        transactionBlock: txBytesBase64,
        signature: [stealthSignature.signature, sponsorSignature],
        options: {
          showEffects: true,
        },
      });

      console.log("Claim transaction:", result);

      // Check if both balances are now 0 for this entry, if so remove it
      const entry = outputs.find(
        (o) => o.outputStealth.address === stealth.address || o.remainderStealth.address === stealth.address
      );
      if (entry) {
        // Re-fetch balances for this entry
        let newOutputBalance = "0";
        let newRemainderBalance = "0";
        try {
          const outputCoins = await suiClient.getCoins({
            owner: entry.outputStealth.address,
            coinType: "0x2::sui::SUI",
          });
          newOutputBalance = outputCoins.data
            .reduce((sum, coin) => sum + BigInt(coin.balance), BigInt(0))
            .toString();
        } catch { /* ignore */ }
        try {
          const remainderCoins = await suiClient.getCoins({
            owner: entry.remainderStealth.address,
            coinType: "0x2::sui::SUI",
          });
          newRemainderBalance = remainderCoins.data
            .reduce((sum, coin) => sum + BigInt(coin.balance), BigInt(0))
            .toString();
        } catch { /* ignore */ }

        // If both are 0, remove from storage
        if (BigInt(newOutputBalance) === BigInt(0) && BigInt(newRemainderBalance) === BigInt(0)) {
          removeStealthKeyPair(walletAddress, entry.outputStealth.address);
        }
      }

      // Refresh outputs
      await loadOutputs();
    } catch (err) {
      console.error("Claim error:", err);
      setError(err instanceof Error ? err.message : "Failed to claim");
    } finally {
      setClaiming(null);
    }
  };

  if (!currentAccount) {
    return (
      <div className="w-[480px] mx-auto animate-slide-up">
        <div className="glass-card rounded-2xl p-4">
          <h3 className="text-xl font-bold mb-6 text-white text-center">Stealth Outputs</h3>
          <p className="text-gray-400 text-center py-8">
            Connect wallet to view outputs
          </p>
        </div>
      </div>
    );
  }

  return (
    <div className="w-[480px] mx-auto animate-slide-up relative">
      
      {/* Header */}
      <div className="flex justify-between items-center mb-4 px-2">
        <h2 className="text-xl font-bold font-tektur text-white">Claim</h2>
        <button 
          onClick={loadOutputs}
          disabled={loading}
          className="p-2 rounded-full text-gray-400 hover:text-white transition-colors disabled:opacity-50"
        >
          <RefreshCw size={18} />
        </button>
      </div>

      {error && (
        <div className="mb-4 p-3 bg-red-500/10 border border-red-500/20 rounded-xl text-red-400 text-sm">
          {error}
        </div>
      )}

      {outputs.length === 0 ? (
        <div className="glass-card rounded-2xl p-12 text-center">
          <Inbox size={48} className="mx-auto mb-4 text-gray-600 opacity-30" />
          <p className="text-gray-400 mb-2 font-medium">No stealth outputs yet</p>
          <p className="text-gray-500 text-xs font-inter">
            Complete a swap to receive unlinkable tokens at your private stealth addresses.
          </p>
        </div>
      ) : (
        <div className="space-y-4">
          {outputs.map((output, index) => (
            <div
              key={`${output.outputStealth.address}-${index}`}
              className="glass-card rounded-2xl p-5 border border-white/5 hover:border-white/10 transition-all"
            >
              {/* Output Stealth */}
              {BigInt(output.outputBalance) > 0 && (
                <div className="mb-4 pb-4 border-b border-white/5">
                  <div className="flex justify-between items-center mb-2">
                    <span className="text-xs font-bold text-gray-500 uppercase tracking-wider">Swap Output</span>
                    <span className="text-xl font-bold text-green-400">
                      {formatAmount(output.outputBalance)} SUI
                    </span>
                  </div>
                  <div className="text-[10px] text-gray-500 font-mono truncate mb-4 opacity-60">
                    {output.outputStealth.address}
                  </div>
                  <button
                    onClick={() => claimCoins(output.outputStealth)}
                    disabled={claiming === output.outputStealth.address}
                    className="w-full bg-blue-600 hover:bg-blue-500 disabled:bg-white/5 disabled:text-gray-500 text-white font-bold py-3 rounded-xl transition shadow-lg shadow-blue-500/10 font-tektur"
                  >
                    {claiming === output.outputStealth.address
                      ? "Claiming..."
                      : "Claim to Main Wallet"}
                  </button>
                </div>
              )}

              {/* Remainder Stealth */}
              {BigInt(output.remainderBalance) > 0 && (
                <div>
                  <div className="flex justify-between items-center mb-2">
                    <span className="text-xs font-bold text-gray-500 uppercase tracking-wider">Remainder</span>
                    <span className="text-xl font-bold text-blue-400">
                      {formatAmount(output.remainderBalance)} SUI
                    </span>
                  </div>
                  <div className="text-[10px] text-gray-500 font-mono truncate mb-4 opacity-60">
                    {output.remainderStealth.address}
                  </div>
                  <button
                    onClick={() => claimCoins(output.remainderStealth)}
                    disabled={claiming === output.remainderStealth.address}
                    className="w-full bg-white/10 hover:bg-white/20 disabled:bg-white/5 disabled:text-gray-500 text-white font-bold py-3 rounded-xl transition font-tektur"
                  >
                    {claiming === output.remainderStealth.address
                      ? "Claiming..."
                      : "Claim Remainder"}
                  </button>
                </div>
              )}

              {/* Empty state (pending) */}
              {BigInt(output.outputBalance) === BigInt(0) &&
                BigInt(output.remainderBalance) === BigInt(0) && (
                  <div className="text-center py-4">
                    <div className="flex items-center justify-center gap-2 mb-2">
                      <div className="w-2 h-2 bg-orange-500 rounded-full animate-pulse" />
                      <p className="text-orange-400 text-sm font-medium">
                        Pending execution...
                      </p>
                    </div>
                    <p className="text-gray-600 text-[10px] font-mono mb-3">
                      {new Date(output.timestamp).toLocaleString()}
                    </p>
                    <button
                      onClick={() => {
                        removeStealthKeyPair(walletAddress, output.outputStealth.address);
                        loadOutputs();
                      }}
                      className="text-xs text-gray-500 hover:text-gray-300 underline transition-colors"
                    >
                      Dismiss
                    </button>
                  </div>
                )}
            </div>
          ))}
        </div>
      )}

      {/* Info */}
      <div className="mt-8 text-[10px] text-gray-600 text-center uppercase tracking-tighter">
        Stealth addresses are unlinkable to your main wallet on-chain.
      </div>
    </div>
  );
}
