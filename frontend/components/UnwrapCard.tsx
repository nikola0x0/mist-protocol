"use client";

import { useState } from "react";

export function UnwrapCard() {
  const [token, setToken] = useState<"eSUI" | "eUSDC">("eSUI");
  const [amount, setAmount] = useState("");
  const [loading, setLoading] = useState(false);

  const realToken = token === "eSUI" ? "SUI" : "USDC";

  const handleUnwrap = async () => {
    setLoading(true);
    await new Promise((resolve) => setTimeout(resolve, 2000));
    alert(`Unwrapped ${amount} ${token} → ${realToken}`);
    setAmount("");
    setLoading(false);
  };

  return (
    <div className="card p-6 animate-slide-up">
      <h3 className="text-xl font-bold mb-6">Unwrap Tokens</h3>

      {/* Token Selector */}
      <div className="grid grid-cols-2 gap-3 mb-6">
        <button
          onClick={() => setToken("eSUI")}
          className={`p-4 rounded-lg font-medium transition border ${
            token === "eSUI"
              ? "bg-blue-600 text-white border-blue-600"
              : "bg-[#141414] text-gray-400 border-[#262626] hover:border-[#333] hover:text-white"
          }`}
        >
          <div className="flex items-center justify-center gap-2">
            <div className="w-6 h-6 bg-blue-600 rounded-full flex items-center justify-center text-xs">
              🔒
            </div>
            <span>eSUI</span>
          </div>
        </button>
        <button
          onClick={() => setToken("eUSDC")}
          className={`p-4 rounded-lg font-medium transition border ${
            token === "eUSDC"
              ? "bg-blue-600 text-white border-blue-600"
              : "bg-[#141414] text-gray-400 border-[#262626] hover:border-[#333] hover:text-white"
          }`}
        >
          <div className="flex items-center justify-center gap-2">
            <div className="w-6 h-6 bg-green-600 rounded-full flex items-center justify-center text-xs">
              🔒
            </div>
            <span>eUSDC</span>
          </div>
        </button>
      </div>

      {/* Amount Input */}
      <div className="mb-6">
        <div className="flex justify-between mb-2">
          <label className="text-sm text-gray-400">Amount</label>
          <span className="text-xs text-gray-500">Encrypted Balance: 50.00 {token}</span>
        </div>
        <input
          type="number"
          value={amount}
          onChange={(e) => setAmount(e.target.value)}
          placeholder="0.00"
          className="w-full px-4 py-4 bg-[#0a0a0a] border border-[#262626] rounded-lg text-2xl font-medium focus:outline-none focus:border-blue-600 transition"
        />
      </div>

      {/* Unwrap Button */}
      <button
        onClick={handleUnwrap}
        disabled={!amount || loading}
        className="w-full bg-green-600 hover:bg-green-700 disabled:bg-gray-800 disabled:text-gray-600 text-white font-medium py-4 rounded-lg transition"
      >
        {loading ? "Unwrapping..." : `Unwrap ${token} → ${realToken}`}
      </button>

      <div className="mt-4 text-xs text-gray-500 text-center">
        ℹ️ {token} token will be burned, you'll receive {realToken}
      </div>
    </div>
  );
}
