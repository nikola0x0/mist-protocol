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
} from "../lib/deposit-notes";

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
      <div className="card p-6 animate-slide-up">
        <h3 className="text-xl font-bold mb-6">Stealth Outputs</h3>
        <p className="text-gray-400 text-center py-8">
          Connect wallet to view outputs
        </p>
      </div>
    );
  }

  return (
    <div className="card p-6 animate-slide-up">
      <div className="flex justify-between items-center mb-6">
        <h3 className="text-xl font-bold">Stealth Outputs</h3>
        <button
          onClick={loadOutputs}
          disabled={loading}
          className="text-sm text-blue-500 hover:text-blue-400 disabled:text-gray-600"
        >
          {loading ? "Loading..." : "Refresh"}
        </button>
      </div>

      {error && (
        <div className="mb-4 p-3 bg-red-900/30 border border-red-700 rounded-lg text-red-400 text-sm">
          {error}
        </div>
      )}

      {outputs.length === 0 ? (
        <div className="text-center py-8">
          <div className="text-gray-500 text-4xl mb-4">&#x1F4E5;</div>
          <p className="text-gray-400 mb-2">No stealth outputs yet</p>
          <p className="text-gray-500 text-sm">
            Complete a swap to receive outputs at stealth addresses
          </p>
        </div>
      ) : (
        <div className="space-y-4">
          {outputs.map((output, index) => (
            <div
              key={`${output.outputStealth.address}-${index}`}
              className="bg-[#0a0a0a] border border-[#262626] rounded-lg p-4"
            >
              {/* Output Stealth */}
              {BigInt(output.outputBalance) > 0 && (
                <div className="mb-3 pb-3 border-b border-[#262626]">
                  <div className="flex justify-between items-center mb-2">
                    <span className="text-sm text-gray-400">Swap Output</span>
                    <span className="text-green-500 font-medium">
                      {formatAmount(output.outputBalance)} SUI
                    </span>
                  </div>
                  <div className="text-xs text-gray-600 font-mono truncate mb-2">
                    {output.outputStealth.address}
                  </div>
                  <button
                    onClick={() => claimCoins(output.outputStealth)}
                    disabled={claiming === output.outputStealth.address}
                    className="w-full bg-green-600 hover:bg-green-700 disabled:bg-gray-800 disabled:text-gray-600 text-white text-sm font-medium py-2 rounded-lg transition"
                  >
                    {claiming === output.outputStealth.address
                      ? "Claiming..."
                      : "Claim to Wallet"}
                  </button>
                </div>
              )}

              {/* Remainder Stealth */}
              {BigInt(output.remainderBalance) > 0 && (
                <div>
                  <div className="flex justify-between items-center mb-2">
                    <span className="text-sm text-gray-400">Remainder</span>
                    <span className="text-green-500 font-medium">
                      {formatAmount(output.remainderBalance)} SUI
                    </span>
                  </div>
                  <div className="text-xs text-gray-600 font-mono truncate mb-2">
                    {output.remainderStealth.address}
                  </div>
                  <button
                    onClick={() => claimCoins(output.remainderStealth)}
                    disabled={claiming === output.remainderStealth.address}
                    className="w-full bg-green-600 hover:bg-green-700 disabled:bg-gray-800 disabled:text-gray-600 text-white text-sm font-medium py-2 rounded-lg transition"
                  >
                    {claiming === output.remainderStealth.address
                      ? "Claiming..."
                      : "Claim to Wallet"}
                  </button>
                </div>
              )}

              {/* Empty state */}
              {BigInt(output.outputBalance) === BigInt(0) &&
                BigInt(output.remainderBalance) === BigInt(0) && (
                  <div className="text-center py-2">
                    <p className="text-gray-500 text-sm">
                      Pending swap execution...
                    </p>
                    <p className="text-gray-600 text-xs mt-1">
                      {new Date(output.timestamp).toLocaleString()}
                    </p>
                  </div>
                )}
            </div>
          ))}
        </div>
      )}

      {/* Info */}
      <div className="mt-4 text-xs text-gray-500 text-center">
        Stealth addresses provide unlinkable outputs. Claim transfers coins to
        your main wallet.
      </div>
    </div>
  );
}
