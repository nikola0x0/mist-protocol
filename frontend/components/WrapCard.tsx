"use client";

import { useState } from "react";

export function WrapCard() {
  const [token, setToken] = useState<"SUI" | "USDC">("SUI");
  const [amount, setAmount] = useState("");
  const [loading, setLoading] = useState(false);

  const handleWrap = async () => {
    setLoading(true);
    await new Promise((resolve) => setTimeout(resolve, 2000));
    alert(`Wrapped ${amount} ${token} → e${token}`);
    setAmount("");
    setLoading(false);
  };

  return (
    <div className="card p-6 animate-slide-up">
      <h3 className="text-xl font-bold mb-6">Wrap Tokens</h3>

      {/* Token Selector */}
      <div className="grid grid-cols-2 gap-3 mb-6">
        <button
          onClick={() => setToken("SUI")}
          className={`p-4 rounded-lg font-medium transition border ${
            token === "SUI"
              ? "bg-blue-600 text-white border-blue-600"
              : "bg-[#141414] text-gray-400 border-[#262626] hover:border-[#333] hover:text-white"
          }`}
        >
          <div className="flex items-center justify-center gap-2">
            <div className="w-6 h-6 bg-blue-500 rounded-full flex items-center justify-center text-xs font-bold">
              S
            </div>
            <span>SUI</span>
          </div>
        </button>
        <button
          onClick={() => setToken("USDC")}
          className={`p-4 rounded-lg font-medium transition border ${
            token === "USDC"
              ? "bg-blue-600 text-white border-blue-600"
              : "bg-[#141414] text-gray-400 border-[#262626] hover:border-[#333] hover:text-white"
          }`}
        >
          <div className="flex items-center justify-center gap-2">
            <div className="w-6 h-6 bg-green-500 rounded-full flex items-center justify-center text-xs font-bold">
              U
            </div>
            <span>USDC</span>
          </div>
        </button>
      </div>

      {/* Amount Input */}
      <div className="mb-6">
        <div className="flex justify-between mb-2">
          <label className="text-sm text-gray-400">Amount</label>
          <span className="text-xs text-gray-500">Balance: 100.00 {token}</span>
        </div>
        <input
          type="number"
          value={amount}
          onChange={(e) => setAmount(e.target.value)}
          placeholder="0.00"
          className="w-full px-4 py-4 bg-[#0a0a0a] border border-[#262626] rounded-lg text-2xl font-medium focus:outline-none focus:border-blue-600 transition"
        />
      </div>

      {/* Wrap Button */}
      <button
        onClick={handleWrap}
        disabled={!amount || loading}
        className="w-full bg-blue-600 hover:bg-blue-700 disabled:bg-gray-800 disabled:text-gray-600 text-white font-medium py-4 rounded-lg transition"
      >
        {loading ? "Wrapping..." : `Wrap ${token} → e${token}`}
      </button>

      <div className="mt-4 text-xs text-gray-500 text-center">
        ℹ️ You'll receive e{token} tokens with encrypted balance
      </div>
    </div>
  );
}
