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
import Image from "next/image";

// ============ CONSTANTS ============

const MIST_TOKEN_TYPE = "0x1071c10ef6fa032cd54f51948b5193579e6596ffaecd173df2dac6f73e31a468::mist_token::MIST_TOKEN";

// Token types to check for
const TOKEN_TYPES = [
  { type: "0x2::sui::SUI", symbol: "SUI", icon: "/assets/token-icons/sui.png", decimals: 9 },
  { type: MIST_TOKEN_TYPE, symbol: "MIST", icon: "/assets/token-icons/mist-token.png", decimals: 9 },
];

// ============ TYPES ============

interface TokenBalance {
  type: string;
  symbol: string;
  icon: string;
  balance: string;
  coinIds: string[];
}

interface StealthOutput {
  outputStealth: StealthAddress;
  remainderStealth: StealthAddress;
  outputBalances: TokenBalance[];
  remainderBalances: TokenBalance[];
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

  // Helper to get balances for all token types at an address
  const getTokenBalances = async (address: string): Promise<TokenBalance[]> => {
    const balances: TokenBalance[] = [];

    for (const token of TOKEN_TYPES) {
      try {
        const coins = await suiClient.getCoins({
          owner: address,
          coinType: token.type,
        });

        const totalBalance = coins.data
          .reduce((sum, coin) => sum + BigInt(coin.balance), BigInt(0))
          .toString();

        if (BigInt(totalBalance) > 0) {
          balances.push({
            type: token.type,
            symbol: token.symbol,
            icon: token.icon,
            balance: totalBalance,
            coinIds: coins.data.map(c => c.coinObjectId),
          });
        }
      } catch {
        // Address may not exist yet or no coins
      }
    }

    return balances;
  };

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

      // Fetch balances for each stealth address (all token types)
      const outputsWithBalances: StealthOutput[] = await Promise.all(
        stealthKeys.map(async (keys) => {
          const outputBalances = await getTokenBalances(keys.output.address);
          const remainderBalances = await getTokenBalances(keys.remainder.address);

          return {
            outputStealth: keys.output,
            remainderStealth: keys.remainder,
            outputBalances,
            remainderBalances,
            timestamp: keys.timestamp,
          };
        })
      );

      // Filter to only show outputs with balances or recent ones (last 24h)
      const oneDayAgo = Date.now() - 24 * 60 * 60 * 1000;
      const filtered = outputsWithBalances.filter(
        (o) =>
          o.outputBalances.length > 0 ||
          o.remainderBalances.length > 0 ||
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

  // Claim all coins from a stealth address using sponsored transaction
  // User's main wallet pays for gas, stealth address is the sender
  const claimCoins = async (stealth: StealthAddress, balances: TokenBalance[]) => {
    if (!walletAddress || balances.length === 0) return;

    setClaiming(stealth.address);
    setError(null);

    try {
      // Get the keypair for the stealth address
      const stealthKeypair = getStealthKeypair(stealth);

      // Build transaction:
      // - Sender: stealth address (owns the coins)
      // - Gas sponsor: user's main wallet (pays for gas)
      const tx = new Transaction();
      tx.setSender(stealth.address);
      tx.setGasOwner(walletAddress); // User's wallet sponsors gas

      // Transfer all coins of all types to the main wallet
      for (const tokenBalance of balances) {
        const coinRefs = tokenBalance.coinIds.map((id) => tx.object(id));
        tx.transferObjects(coinRefs, walletAddress);
      }

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

      // Check if both addresses have 0 balance for this entry, if so remove it
      const entry = outputs.find(
        (o) => o.outputStealth.address === stealth.address || o.remainderStealth.address === stealth.address
      );
      if (entry) {
        // Re-fetch balances for this entry
        const newOutputBalances = await getTokenBalances(entry.outputStealth.address);
        const newRemainderBalances = await getTokenBalances(entry.remainderStealth.address);

        // If both are empty, remove from storage
        if (newOutputBalances.length === 0 && newRemainderBalances.length === 0) {
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
              {output.outputBalances.length > 0 && (
                <div className="mb-4 pb-4 border-b border-white/5">
                  <div className="flex justify-between items-center mb-3">
                    <span className="text-xs font-bold text-gray-500 uppercase tracking-wider">Swap Output</span>
                  </div>
                  {/* Token balances */}
                  <div className="space-y-2 mb-3">
                    {output.outputBalances.map((token) => (
                      <div key={token.type} className="flex items-center gap-2">
                        <div className="w-6 h-6 relative rounded-full overflow-hidden">
                          <Image src={token.icon} alt={token.symbol} fill className="object-cover" />
                        </div>
                        <span className="text-xl font-bold text-green-400">
                          {formatAmount(token.balance)} {token.symbol}
                        </span>
                      </div>
                    ))}
                  </div>
                  <div className="text-[10px] text-gray-500 font-mono truncate mb-4 opacity-60">
                    {output.outputStealth.address}
                  </div>
                  <button
                    onClick={() => claimCoins(output.outputStealth, output.outputBalances)}
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
              {output.remainderBalances.length > 0 && (
                <div>
                  <div className="flex justify-between items-center mb-3">
                    <span className="text-xs font-bold text-gray-500 uppercase tracking-wider">Remainder</span>
                  </div>
                  {/* Token balances */}
                  <div className="space-y-2 mb-3">
                    {output.remainderBalances.map((token) => (
                      <div key={token.type} className="flex items-center gap-2">
                        <div className="w-6 h-6 relative rounded-full overflow-hidden">
                          <Image src={token.icon} alt={token.symbol} fill className="object-cover" />
                        </div>
                        <span className="text-xl font-bold text-blue-400">
                          {formatAmount(token.balance)} {token.symbol}
                        </span>
                      </div>
                    ))}
                  </div>
                  <div className="text-[10px] text-gray-500 font-mono truncate mb-4 opacity-60">
                    {output.remainderStealth.address}
                  </div>
                  <button
                    onClick={() => claimCoins(output.remainderStealth, output.remainderBalances)}
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
              {output.outputBalances.length === 0 &&
                output.remainderBalances.length === 0 && (
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
