"use client";

import { useState } from "react";
import { useCurrentAccount } from "@mysten/dapp-kit";
import { useDepositNotes } from "../hooks/useDepositNotes";
import { formatAmount, exportNotesForBackup } from "../lib/deposit-notes";
import Image from "next/image";
import { Info, AlertTriangle } from "lucide-react";

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
      <div className="w-[480px] mx-auto animate-slide-up">
        <div className="glass-card rounded-2xl p-4">
          <div className="text-center py-12 text-gray-400">
            Connect wallet to deposit
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className="w-[480px] mx-auto animate-slide-up relative">
      
      {/* Header */}
      <div className="flex justify-between items-center mb-4 px-2">
        <h2 className="text-xl font-bold font-tektur text-white">Deposit</h2>
        <div className="text-gray-500 hover:text-white transition-colors cursor-help" title="Deposits are encrypted and unlinkable">
          <Info size={16} />
        </div>
      </div>

      {/* Backup Warning Modal */}
      {showBackupWarning && (
        <div className="fixed inset-0 bg-black/80 flex items-center justify-center z-50 p-4 backdrop-blur-md animate-fade-in">
          <div className="glass-card border border-red-500/30 rounded-2xl p-6 max-w-md w-full shadow-2xl animate-slide-up">
            <div className="flex justify-center mb-4">
              <AlertTriangle size={48} className="text-yellow-500" />
            </div>
            <h4 className="text-xl font-bold mb-4 text-center text-white">
              Backup Your Deposit Note!
            </h4>
            <p className="text-gray-300 mb-4 text-sm text-center font-inter">
              Your deposit note contains a secret nullifier. If you lose it,{" "}
              <span className="text-red-400 font-bold">
                your funds are UNRECOVERABLE
              </span>
              .
            </p>
            <div className="glass-panel p-3 rounded-lg mb-4 font-mono text-xs break-all text-gray-300">
              Nullifier: {lastNote?.substring(0, 20)}...
            </div>
            <div className="flex gap-3">
              <button
                onClick={handleBackup}
                className="flex-1 bg-green-600 hover:bg-green-700 text-white font-bold py-3 rounded-xl transition shadow-lg"
              >
                Download Backup
              </button>
              <button
                onClick={() => setShowBackupWarning(false)}
                className="flex-1 glass-button hover:bg-white/10 text-white font-bold py-3 rounded-xl transition"
              >
                I&apos;ve Saved It
              </button>
            </div>
          </div>
        </div>
      )}

      {/* Input Section */}
      <div className="relative mb-4">
        <div className="glass-card rounded-2xl p-4 border border-white/5 hover:border-white/10 transition-colors">
          <div className="flex justify-between text-sm text-gray-400 mb-2 font-medium">
            <span>Asset to Deposit</span>
            <span className="text-xs">Unlinkable Pool</span>
          </div>
          
          <div className="flex justify-between items-center h-12">
            <input
              type="number"
              value={amount}
              onChange={(e) => setAmount(e.target.value)}
              placeholder="0.00"
              className="bg-transparent text-4xl font-medium w-full outline-none text-white placeholder-gray-600"
            />
            
            <div className="flex items-center gap-2 bg-white/5 rounded-full pl-2 pr-3 py-1.5 h-10 ml-2 border border-white/5">
              <div className="w-6 h-6 relative rounded-full overflow-hidden">
                <Image src="/assets/token-icons/sui.png" alt="SUI" fill className="object-cover" />
              </div>
              <span className="font-bold text-lg text-white">SUI</span>
            </div>
          </div>
          
          <div className="mt-2 text-[10px] text-gray-500 uppercase tracking-widest font-bold">
            Privacy tier: maximum
          </div>
        </div>
      </div>

      {/* Error Message */}
      {error && (
        <div className="mb-4 p-3 bg-red-500/10 border border-red-500/20 rounded-xl text-red-400 text-sm">
          {error}
        </div>
      )}

      {/* Action Button */}
      <button
        onClick={handleDeposit}
        disabled={!amount || loading || parseFloat(amount) <= 0}
        className={`w-full py-4 rounded-2xl font-bold text-xl transition-all shadow-lg mb-8 font-tektur ${
          !amount || loading || parseFloat(amount) <= 0
            ? "bg-white/5 text-gray-500 cursor-not-allowed"
            : "bg-blue-600 hover:bg-blue-500 text-white shadow-blue-500/20"
        }`}
      >
        {loading ? "Processing..." : "Deposit SUI"}
      </button>

      {/* Active Deposits List */}
      {unspentNotes.length > 0 && (
        <div className="animate-fade-in px-1">
          <h4 className="text-xs font-bold text-gray-500 uppercase tracking-widest mb-4">
            Active Deposit Notes ({unspentNotes.length})
          </h4>
          <div className="space-y-2">
            {unspentNotes.map((note) => (
              <div
                key={note.nullifier}
                className="flex justify-between items-center glass-card p-4 rounded-xl border border-transparent hover:border-white/5 transition-all group"
              >
                <div className="flex items-center gap-3">
                  <div className="w-8 h-8 rounded-full bg-blue-500/10 flex items-center justify-center">
                    <Image src="/assets/token-icons/sui.png" alt="SUI" width={18} height={18} />
                  </div>
                  <div>
                    <div className="font-bold text-white">
                      {formatAmount(note.amount)} SUI
                    </div>
                    <div className="text-[10px] text-gray-500 font-mono">
                      Nullifier: ...{note.nullifier.slice(-8)}
                    </div>
                  </div>
                </div>
                <div className="text-right">
                  <div className="text-[10px] text-green-400 font-bold bg-green-400/5 px-2 py-1 rounded border border-green-400/10">
                    ACTIVE
                  </div>
                  <div className="text-[10px] text-gray-600 mt-1">
                    {new Date(note.timestamp).toLocaleDateString()}
                  </div>
                </div>
              </div>
            ))}
          </div>
        </div>
      )}

      {/* Info Footer */}
      <div className="mt-8 text-[10px] text-gray-600 text-center uppercase tracking-tighter">
        Notes are stored locally. Backup to prevent fund loss.
      </div>
    </div>
  );
}
