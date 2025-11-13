"use client";

import { ConnectButton } from "@/components/ConnectButton";
import { useCurrentAccount } from "@mysten/dapp-kit";
import { BalanceCard } from "@/components/BalanceCard";
import { WrapCard } from "@/components/WrapCard";
import { SwapCard } from "@/components/SwapCard";
import { UnwrapCard } from "@/components/UnwrapCard";
import { TransactionHistory } from "@/components/TransactionHistory";
import { useState } from "react";

export default function Home() {
  const account = useCurrentAccount();
  const [activeTab, setActiveTab] = useState<"wrap" | "swap" | "unwrap">("swap");

  if (!account) {
    return (
      <div className="min-h-screen flex flex-col bg-[#0a0a0a]">
        <header className="border-b border-[#262626] bg-[#0a0a0a]/80 backdrop-blur-sm sticky top-0 z-50">
          <div className="container mx-auto px-6 py-4 flex justify-between items-center">
            <h1 className="text-2xl font-bold gradient-text">Mist Protocol</h1>
            <ConnectButton />
          </div>
        </header>

        <main className="flex-1 container mx-auto px-6 py-20">
          <div className="max-w-4xl mx-auto text-center space-y-8 animate-slide-up">
            <h2 className="text-5xl md:text-6xl font-bold leading-tight">
              Private DeFi Trading on{" "}
              <span className="gradient-text">Sui</span>
            </h2>
            <p className="text-xl text-gray-400 max-w-2xl mx-auto">
              Trade with encrypted amounts using TEE-powered execution. Your transactions, your privacy.
            </p>

            <div className="pt-4">
              <ConnectButton />
            </div>

            <div className="pt-16 grid grid-cols-1 md:grid-cols-3 gap-6">
              <div className="card p-8 hover:glow">
                <div className="text-4xl mb-4">🔐</div>
                <h3 className="font-bold text-lg mb-2">Encrypted Trading</h3>
                <p className="text-sm text-gray-400">
                  Trade without exposing amounts on-chain
                </p>
              </div>
              <div className="card p-8 hover:glow">
                <div className="text-4xl mb-4">⚡</div>
                <h3 className="font-bold text-lg mb-2">TEE Verified</h3>
                <p className="text-sm text-gray-400">
                  Nautilus-powered trusted execution
                </p>
              </div>
              <div className="card p-8 hover:glow">
                <div className="text-4xl mb-4">🌊</div>
                <h3 className="font-bold text-lg mb-2">Sui Native</h3>
                <p className="text-sm text-gray-400">
                  Built on Sui with Seal & Walrus
                </p>
              </div>
            </div>
          </div>
        </main>

        <footer className="border-t border-[#262626] py-8">
          <div className="container mx-auto px-6 text-center text-sm text-gray-500">
            Powered by Nautilus • Seal • Walrus • Cetus
          </div>
        </footer>
      </div>
    );
  }

  return (
    <div className="min-h-screen flex flex-col bg-[#0a0a0a]">
      <header className="border-b border-[#262626] bg-[#0a0a0a]/80 backdrop-blur-sm sticky top-0 z-50">
        <div className="container mx-auto px-6 py-4 flex justify-between items-center">
          <h1 className="text-2xl font-bold gradient-text">Mist Protocol</h1>
          <ConnectButton />
        </div>
      </header>

      <main className="flex-1 container mx-auto px-6 py-8">
        <div className="max-w-6xl mx-auto">
          {/* Navigation Tabs */}
          <div className="flex gap-3 mb-8">
            <button
              onClick={() => setActiveTab("wrap")}
              className={`px-6 py-2.5 rounded-lg font-medium transition ${
                activeTab === "wrap"
                  ? "bg-blue-600 text-white"
                  : "bg-[#141414] text-gray-400 hover:text-white hover:bg-[#1a1a1a]"
              }`}
            >
              Wrap
            </button>
            <button
              onClick={() => setActiveTab("swap")}
              className={`px-6 py-2.5 rounded-lg font-medium transition ${
                activeTab === "swap"
                  ? "bg-blue-600 text-white"
                  : "bg-[#141414] text-gray-400 hover:text-white hover:bg-[#1a1a1a]"
              }`}
            >
              Swap
            </button>
            <button
              onClick={() => setActiveTab("unwrap")}
              className={`px-6 py-2.5 rounded-lg font-medium transition ${
                activeTab === "unwrap"
                  ? "bg-blue-600 text-white"
                  : "bg-[#141414] text-gray-400 hover:text-white hover:bg-[#1a1a1a]"
              }`}
            >
              Unwrap
            </button>
          </div>

          <div className="grid grid-cols-1 lg:grid-cols-12 gap-6">
            {/* Main Action Area */}
            <div className="lg:col-span-7 space-y-6">
              {activeTab === "wrap" && <WrapCard />}
              {activeTab === "swap" && <SwapCard />}
              {activeTab === "unwrap" && <UnwrapCard />}

              {/* Account Info */}
              <div className="card p-4">
                <div className="text-xs text-gray-500 mb-1">Connected Account</div>
                <div className="text-sm font-mono text-gray-400 truncate">
                  {account.address}
                </div>
              </div>
            </div>

            {/* Sidebar */}
            <div className="lg:col-span-5 space-y-6">
              <BalanceCard />
              <TransactionHistory />
            </div>
          </div>
        </div>
      </main>

      <footer className="border-t border-[#262626] py-8 mt-auto">
        <div className="container mx-auto px-6 text-center text-sm text-gray-500">
          Powered by Nautilus • Seal • Walrus • Cetus
        </div>
      </footer>
    </div>
  );
}
