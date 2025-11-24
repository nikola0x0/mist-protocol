"use client";

import { useState } from "react";

export function SwapCard() {
  const [fromToken, setFromToken] = useState<"eSUI" | "eUSDC">("eSUI");
  const [toToken, setToToken] = useState<"eUSDC" | "eSUI">("eUSDC");
  const [amount, setAmount] = useState("");
  const [loading, setLoading] = useState(false);

  const handleSwap = async () => {
    setLoading(true);
    await new Promise((resolve) => setTimeout(resolve, 2000));
    alert(`Swap requested: ${amount} ${fromToken} → ${toToken}`);
    setAmount("");
    setLoading(false);
  };

  const handleFlip = () => {
    setFromToken(toToken);
    setToToken(fromToken);
  };

  return (
    <div className="card p-6 animate-slide-up">
      <h3 className="text-xl font-bold mb-6">Swap Tokens</h3>

      {/* From Token */}
      <div className="bg-[#0a0a0a] border border-[#262626] rounded-lg p-4 mb-3">
        <div className="flex justify-between mb-2">
          <label className="text-sm text-gray-400">From</label>
          <span className="text-xs text-gray-500">Balance: 50.00 {fromToken}</span>
        </div>
        <div className="flex items-center gap-3">
          <input
            type="number"
            value={amount}
            onChange={(e) => setAmount(e.target.value)}
            placeholder="0.00"
            className="flex-1 bg-transparent text-2xl font-medium focus:outline-none"
          />
          <div className="flex items-center gap-2 px-3 py-2 bg-[#141414] rounded-lg">
            <span className="font-medium">{fromToken}</span>
          </div>
        </div>
      </div>

      {/* Flip Button */}
      <div className="flex justify-center my-3">
        <button
          onClick={handleFlip}
          className="p-2 bg-[#141414] hover:bg-[#1a1a1a] border border-[#262626] rounded-lg transition"
        >
          <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M7 16V4m0 0L3 8m4-4l4 4m6 0v12m0 0l4-4m-4 4l-4-4" />
          </svg>
        </button>
      </div>

      {/* To Token */}
      <div className="bg-[#0a0a0a] border border-[#262626] rounded-lg p-4 mb-6">
        <div className="flex justify-between mb-2">
          <label className="text-sm text-gray-400">To (estimated)</label>
          <span className="text-xs text-gray-500">Rate: 1 {fromToken} ≈ 2.5 {toToken}</span>
        </div>
        <div className="flex items-center gap-3">
          <input
            type="text"
            value={amount ? (parseFloat(amount) * 2.5).toFixed(2) : "0.00"}
            readOnly
            className="flex-1 bg-transparent text-2xl font-medium text-gray-500"
          />
          <div className="flex items-center gap-2 px-3 py-2 bg-[#141414] rounded-lg">
            <span className="font-medium">{toToken}</span>
          </div>
        </div>
      </div>

      {/* Swap Button */}
      <button
        onClick={handleSwap}
        disabled={!amount || loading}
        className="w-full bg-blue-600 hover:bg-blue-700 disabled:bg-gray-800 disabled:text-gray-600 text-white font-medium py-4 rounded-lg transition"
      >
        {loading ? "Requesting Swap..." : "Request Swap"}
      </button>

      <div className="mt-4 text-xs text-gray-500 text-center">
        TEE will execute swap on Cetus DEX privately
      </div>
    </div>
  );
}
