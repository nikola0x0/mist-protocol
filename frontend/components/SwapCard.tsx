"use client";

import { useState } from "react";
import { useCurrentAccount } from "@mysten/dapp-kit";
import { useDepositNotes } from "../hooks/useDepositNotes";
import { formatAmount, DepositNote } from "../lib/deposit-notes";

export function SwapCard() {
  const [selectedNote, setSelectedNote] = useState<DepositNote | null>(null);
  const [swapAmount, setSwapAmount] = useState("");
  const [showConfirm, setShowConfirm] = useState(false);

  const currentAccount = useCurrentAccount();
  const { unspentNotes, createSwapIntent, loading, error } = useDepositNotes();

  const handleSelectNote = (note: DepositNote) => {
    setSelectedNote(note);
    // Default to full deposit amount
    setSwapAmount(formatAmount(note.amount));
  };

  const handleSwap = async () => {
    if (!selectedNote || !swapAmount) return;

    const result = await createSwapIntent(selectedNote, swapAmount);

    if (result.success) {
      setShowConfirm(true);
      setSelectedNote(null);
      setSwapAmount("");
    }
  };

  const maxAmount = selectedNote ? formatAmount(selectedNote.amount) : "0";
  const parsedSwapAmount = parseFloat(swapAmount) || 0;
  const parsedMaxAmount = parseFloat(maxAmount) || 0;
  const isValidAmount =
    parsedSwapAmount > 0 && parsedSwapAmount <= parsedMaxAmount;

  if (!currentAccount) {
    return (
      <div className="card p-6 animate-slide-up">
        <h3 className="text-xl font-bold mb-6">Private Swap</h3>
        <p className="text-gray-400 text-center py-8">
          Connect wallet to swap
        </p>
      </div>
    );
  }

  return (
    <div className="card p-6 animate-slide-up">
      <h3 className="text-xl font-bold mb-6">Private Swap</h3>

      {/* Success Confirmation */}
      {showConfirm && (
        <div className="fixed inset-0 bg-black/80 flex items-center justify-center z-50 p-4">
          <div className="bg-[#1a1a1a] border border-[#333] rounded-xl p-6 max-w-md w-full">
            <div className="text-green-500 text-4xl mb-4 text-center">
              &#10003;
            </div>
            <h4 className="text-xl font-bold mb-4 text-center">
              Swap Intent Created!
            </h4>
            <p className="text-gray-400 mb-4 text-sm text-center">
              Your swap intent has been submitted. The TEE will process it
              privately and send funds to your stealth address.
            </p>
            <p className="text-gray-500 mb-4 text-xs text-center">
              This may take a few minutes. Check your stealth addresses for the
              output.
            </p>
            <button
              onClick={() => setShowConfirm(false)}
              className="w-full bg-blue-600 hover:bg-blue-700 text-white font-medium py-3 rounded-lg transition"
            >
              Got It
            </button>
          </div>
        </div>
      )}

      {/* No Deposits Warning */}
      {unspentNotes.length === 0 ? (
        <div className="text-center py-8">
          <div className="text-gray-500 text-4xl mb-4">&#x1F512;</div>
          <p className="text-gray-400 mb-2">No active deposits</p>
          <p className="text-gray-500 text-sm">
            Deposit SUI first to create a private swap
          </p>
        </div>
      ) : (
        <>
          {/* Select Deposit */}
          <div className="mb-6">
            <label className="text-sm text-gray-400 mb-2 block">
              Select Deposit to Swap
            </label>
            <div className="space-y-2">
              {unspentNotes.map((note) => (
                <button
                  key={note.nullifier}
                  onClick={() => handleSelectNote(note)}
                  className={`w-full flex justify-between items-center p-4 rounded-lg border transition ${
                    selectedNote?.nullifier === note.nullifier
                      ? "bg-blue-600/20 border-blue-600"
                      : "bg-[#0a0a0a] border-[#262626] hover:border-[#333]"
                  }`}
                >
                  <div className="flex items-center gap-3">
                    <div className="w-8 h-8 bg-blue-500 rounded-full flex items-center justify-center text-sm font-bold">
                      S
                    </div>
                    <div className="text-left">
                      <div className="font-medium">
                        {formatAmount(note.amount)} SUI
                      </div>
                      <div className="text-xs text-gray-500">
                        {new Date(note.timestamp).toLocaleDateString()}
                      </div>
                    </div>
                  </div>
                  {selectedNote?.nullifier === note.nullifier && (
                    <div className="text-blue-500">&#10003;</div>
                  )}
                </button>
              ))}
            </div>
          </div>

          {/* Swap Amount */}
          {selectedNote && (
            <div className="mb-6">
              <div className="flex justify-between mb-2">
                <label className="text-sm text-gray-400">Swap Amount</label>
                <button
                  onClick={() => setSwapAmount(maxAmount)}
                  className="text-xs text-blue-500 hover:text-blue-400"
                >
                  Max: {maxAmount} SUI
                </button>
              </div>
              <input
                type="number"
                value={swapAmount}
                onChange={(e) => setSwapAmount(e.target.value)}
                placeholder="0.00"
                min="0"
                max={maxAmount}
                step="0.001"
                className="w-full px-4 py-4 bg-[#0a0a0a] border border-[#262626] rounded-lg text-2xl font-medium focus:outline-none focus:border-blue-600 transition"
              />
              {parsedSwapAmount > parsedMaxAmount && (
                <p className="text-red-500 text-xs mt-1">
                  Amount exceeds deposit balance
                </p>
              )}
            </div>
          )}

          {/* Output Preview */}
          {selectedNote && isValidAmount && (
            <div className="bg-[#0a0a0a] border border-[#262626] rounded-lg p-4 mb-6">
              <div className="flex justify-between text-sm mb-2">
                <span className="text-gray-400">Swap Output</span>
                <span className="text-gray-500">via TEE + Cetus</span>
              </div>
              <div className="flex justify-between items-center">
                <div>
                  <div className="text-xl font-medium">{swapAmount} SUI</div>
                  <div className="text-xs text-gray-500">To stealth address</div>
                </div>
                <div className="text-right">
                  <div className="text-xl font-medium text-green-500">
                    &#x1F512; Private
                  </div>
                  <div className="text-xs text-gray-500">Unlinkable output</div>
                </div>
              </div>
              {parsedSwapAmount < parsedMaxAmount && (
                <div className="mt-3 pt-3 border-t border-[#262626] text-xs text-gray-500">
                  Remainder:{" "}
                  {(parsedMaxAmount - parsedSwapAmount).toFixed(4)} SUI to
                  another stealth address
                </div>
              )}
            </div>
          )}

          {/* Error Display */}
          {error && (
            <div className="mb-4 p-3 bg-red-900/30 border border-red-700 rounded-lg text-red-400 text-sm">
              {error}
            </div>
          )}

          {/* Swap Button */}
          <button
            onClick={handleSwap}
            disabled={!selectedNote || !isValidAmount || loading}
            className="w-full bg-blue-600 hover:bg-blue-700 disabled:bg-gray-800 disabled:text-gray-600 text-white font-medium py-4 rounded-lg transition"
          >
            {loading
              ? "Creating Swap Intent..."
              : selectedNote
              ? "Create Private Swap"
              : "Select a Deposit First"}
          </button>

          {/* Privacy Info */}
          <div className="mt-4 text-xs text-gray-500 text-center">
            Your swap intent is encrypted. TEE processes privately via Cetus
            DEX.
          </div>
        </>
      )}
    </div>
  );
}
