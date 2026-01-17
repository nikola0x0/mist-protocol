"use client";

import { useState } from "react";
import { useCurrentAccount } from "@mysten/dapp-kit";
import { useDepositNotes } from "../hooks/useDepositNotes";
import { formatAmount, exportNotesForBackup } from "../lib/deposit-notes";
import Image from "next/image";

export function DepositCard() {
  const [amount, setAmount] = useState("");
  const [showBackupWarning, setShowBackupWarning] = useState(false);
  const [lastNote, setLastNote] = useState<string | null>(null);

  const currentAccount = useCurrentAccount();
  const { deposit, loading, error, unspentNotes, notes } = useDepositNotes();

  const handleDeposit = async () => {
    if (!amount || parseFloat(amount) <= 0) return;

    const result = await deposit(amount);

    if (result.success && result.note) {
      // Show backup warning with nullifier
      setLastNote(result.note.nullifier);
      setShowBackupWarning(true);
      setAmount("");
    }
  };

  const handleBackup = () => {
    if (!currentAccount?.address) return;
    const backup = exportNotesForBackup(currentAccount.address);
    const blob = new Blob([backup], { type: "application/json" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = `mist-protocol-backup-${currentAccount.address.substring(0, 8)}-${Date.now()}.json`;
    a.click();
    URL.revokeObjectURL(url);
    setShowBackupWarning(false);
  };

  if (!currentAccount) {
    return (
      <div className="card p-6 animate-slide-up">
        <h3 className="text-xl font-bold mb-6">Deposit SUI</h3>
        <p className="text-gray-400 text-center py-8">
          Connect wallet to deposit
        </p>
      </div>
    );
  }

  return (
    <div className="card p-6 animate-slide-up">
      <h3 className="text-xl font-bold mb-6">Deposit SUI</h3>

      {/* Backup Warning Modal */}
      {showBackupWarning && (
        <div className="fixed inset-0 bg-black/80 flex items-center justify-center z-50 p-4">
          <div className="bg-[#1a1a1a] border border-[#333] rounded-xl p-6 max-w-md w-full">
            <div className="text-yellow-500 text-4xl mb-4 text-center">
              &#9888;
            </div>
            <h4 className="text-xl font-bold mb-4 text-center">
              Backup Your Deposit Note!
            </h4>
            <p className="text-gray-400 mb-4 text-sm">
              Your deposit note contains a secret nullifier. If you lose it,{" "}
              <span className="text-red-500 font-bold">
                your funds are UNRECOVERABLE
              </span>
              .
            </p>
            <p className="text-gray-400 mb-4 text-sm">
              This is how privacy works - like Tornado Cash, only you can access
              your funds.
            </p>
            <div className="bg-[#0a0a0a] p-3 rounded-lg mb-4 font-mono text-xs break-all">
              Nullifier: {lastNote?.substring(0, 20)}...
            </div>
            <div className="flex gap-3">
              <button
                onClick={handleBackup}
                className="flex-1 bg-green-600 hover:bg-green-700 text-white font-medium py-3 rounded-lg transition"
              >
                Download Backup
              </button>
              <button
                onClick={() => setShowBackupWarning(false)}
                className="flex-1 bg-gray-700 hover:bg-gray-600 text-white font-medium py-3 rounded-lg transition"
              >
                I&apos;ve Saved It
              </button>
            </div>
          </div>
        </div>
      )}

      {/* Token Display */}
      <div className="bg-[#141414] border border-[#262626] rounded-lg p-4 mb-6">
        <div className="flex items-center gap-3">
          <div className="w-10 h-10 rounded-full overflow-hidden flex items-center justify-center bg-blue-500/20">
            <Image
              src="/assets/token-icons/sui.png"
              alt="SUI"
              width={40}
              height={40}
            />
          </div>
          <div>
            <div className="font-medium">SUI</div>
            <div className="text-xs text-gray-500">Sui Network</div>
          </div>
        </div>
      </div>

      {/* Amount Input */}
      <div className="mb-6">
        <div className="flex justify-between mb-2">
          <label className="text-sm text-gray-400">Amount</label>
          <span className="text-xs text-gray-500">
            Privacy pool: deposits are unlinkable
          </span>
        </div>
        <input
          type="number"
          value={amount}
          onChange={(e) => setAmount(e.target.value)}
          placeholder="0.00"
          min="0"
          step="0.001"
          className="w-full px-4 py-4 bg-[#0a0a0a] border border-[#262626] rounded-lg text-2xl font-medium focus:outline-none focus:border-blue-600 transition"
        />
      </div>

      {/* Error Display */}
      {error && (
        <div className="mb-4 p-3 bg-red-900/30 border border-red-700 rounded-lg text-red-400 text-sm">
          {error}
        </div>
      )}

      {/* Deposit Button */}
      <button
        onClick={handleDeposit}
        disabled={!amount || loading || parseFloat(amount) <= 0}
        className="w-full bg-blue-600 hover:bg-blue-700 disabled:bg-gray-800 disabled:text-gray-600 text-white font-medium py-4 rounded-lg transition"
      >
        {loading ? "Depositing..." : "Deposit SUI"}
      </button>

      {/* Privacy Info */}
      <div className="mt-4 text-xs text-gray-500 text-center">
        Your deposit will be encrypted. Only you (with your nullifier) can spend
        it.
      </div>

      {/* Active Deposits */}
      {unspentNotes.length > 0 && (
        <div className="mt-6 pt-6 border-t border-[#262626]">
          <h4 className="text-sm font-medium text-gray-400 mb-3">
            Your Active Deposits
          </h4>
          <div className="space-y-2">
            {unspentNotes.map((note) => (
              <div
                key={note.nullifier}
                className="flex justify-between items-center bg-[#0a0a0a] p-3 rounded-lg"
              >
                <div>
                  <div className="font-medium">
                    {formatAmount(note.amount)} SUI
                  </div>
                  <div className="text-xs text-gray-500">
                    {new Date(note.timestamp).toLocaleDateString()}
                  </div>
                </div>
                <div className="text-xs text-green-500">Active</div>
              </div>
            ))}
          </div>
        </div>
      )}

      {/* Total Stats */}
      {notes.length > 0 && (
        <div className="mt-4 flex justify-between text-xs text-gray-500">
          <span>Total deposits: {notes.length}</span>
          <span>
            Active: {unspentNotes.length} | Spent: {notes.length - unspentNotes.length}
          </span>
        </div>
      )}
    </div>
  );
}
