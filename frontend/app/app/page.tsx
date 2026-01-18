"use client";

import { useState } from "react";
import Image from "next/image";
import Link from "next/link";
import { ConnectButton } from "@/components/ConnectButton";
import { DepositCard } from "@/components/DepositCard";
import { SwapCard } from "@/components/SwapCard";
import { StealthOutputsCard } from "@/components/StealthOutputsCard";
import { useCurrentAccount } from "@mysten/dapp-kit";

export default function AppPage() {
  const account = useCurrentAccount();
  const [activeTab, setActiveTab] = useState<"deposit" | "swap" | "claim">("deposit");

  return (
    <div className="min-h-screen bg-black text-white">
      {/* Radial gradient background */}
      <div className="fixed inset-0 radial-gradient-bg pointer-events-none" />

      {/* Header */}
      <header className="relative border-b border-white/10 backdrop-blur-lg sticky top-0 z-50">
        <div className="max-w-7xl mx-auto px-8 py-4 flex justify-between items-center">
          <div className="flex items-center gap-4">
            <Image
              src="/assets/logo.svg"
              alt="Mist Protocol"
              width={48}
              height={48}
              className="opacity-90"
            />
            <div>
              <h1 className="text-3xl font-bold font-tektur gradient-text mb-1">
                Mist Protocol
              </h1>
              <p className="text-xs text-gray-500 font-anonymous-pro">
                Private DEX Swaps on Sui
              </p>
            </div>
          </div>
          <div className="flex items-center gap-4">
            <Link
              href="/cetus-swap"
              className="px-4 py-2 text-sm font-anonymous-pro text-gray-400 hover:text-white transition border border-white/10 rounded-lg hover:border-blue-500/50 hover:bg-blue-500/10"
            >
              🌊 Cetus Swap
            </Link>
            <Link
              href="/flowx-swap"
              className="px-4 py-2 text-sm font-anonymous-pro text-gray-400 hover:text-white transition border border-white/10 rounded-lg hover:border-purple-500/50 hover:bg-purple-500/10"
            >
              💧 FlowX Swap
            </Link>
            <ConnectButton />
          </div>
        </div>
      </header>

      <div className="relative max-w-2xl mx-auto px-8 py-12">
        {/* Tab Navigation */}
        <div className="flex gap-3 mb-8 justify-center">
          {[
            { key: "deposit", label: "Deposit" },
            { key: "swap", label: "Swap" },
            { key: "claim", label: "Claim" },
          ].map((tab) => (
            <button
              key={tab.key}
              onClick={() => setActiveTab(tab.key as "deposit" | "swap" | "claim")}
              className={`glass-button px-8 py-3 font-medium font-tektur transition-all ${
                activeTab === tab.key
                  ? "border-white/20 text-white bg-white/5"
                  : "text-gray-400 hover:text-white hover:border-white/15"
              }`}
            >
              {tab.label}
            </button>
          ))}
        </div>

        {/* Privacy Notice */}
        {!account && (
          <div className="mb-8 p-4 bg-blue-900/20 border border-blue-700/50 rounded-lg text-center">
            <p className="text-blue-300 text-sm">
              Connect your wallet to start using private swaps
            </p>
          </div>
        )}

        {/* Main Content */}
        <div className="space-y-6">
          {activeTab === "deposit" && <DepositCard />}
          {activeTab === "swap" && <SwapCard />}
          {activeTab === "claim" && <StealthOutputsCard />}
        </div>

        {/* Privacy Info */}
        <div className="mt-12 text-center">
          <h3 className="text-lg font-medium mb-4 font-tektur">
            How Privacy Works
          </h3>
          <div className="grid grid-cols-3 gap-4 text-sm">
            <div className="p-4 bg-white/5 border border-white/10 rounded-lg">
              <div className="text-2xl mb-2">1</div>
              <div className="font-medium mb-1">Deposit</div>
              <div className="text-gray-500 text-xs">
                Your funds enter the privacy pool with an encrypted nullifier
              </div>
            </div>
            <div className="p-4 bg-white/5 border border-white/10 rounded-lg">
              <div className="text-2xl mb-2">2</div>
              <div className="font-medium mb-1">Swap</div>
              <div className="text-gray-500 text-xs">
                TEE executes your swap privately - no one can link deposit to
                swap
              </div>
            </div>
            <div className="p-4 bg-white/5 border border-white/10 rounded-lg">
              <div className="text-2xl mb-2">3</div>
              <div className="font-medium mb-1">Receive</div>
              <div className="text-gray-500 text-xs">
                Funds arrive at your stealth address - completely unlinkable
              </div>
            </div>
          </div>
        </div>

        {/* Security Warning */}
        <div className="mt-8 p-4 bg-yellow-900/20 border border-yellow-700/50 rounded-lg">
          <div className="flex items-start gap-3">
            <span className="text-yellow-500 text-xl">&#9888;</span>
            <div className="text-sm">
              <p className="font-medium text-yellow-400 mb-1">
                Backup Your Deposit Notes!
              </p>
              <p className="text-yellow-600">
                Like Tornado Cash, your deposit notes contain secrets. If you
                lose them, your funds are UNRECOVERABLE. Always download a
                backup after depositing.
              </p>
            </div>
          </div>
        </div>
      </div>

      {/* Footer */}
      <footer className="relative border-t border-white/10 backdrop-blur-lg py-6 mt-8">
        <div className="max-w-7xl mx-auto px-8 flex justify-between items-center">
          <div className="text-sm text-gray-600 font-anonymous-pro">
            Powered by Nautilus TEE • SEAL Encryption • Sui Network
          </div>
          <div className="flex items-center gap-4">
            <a
              href="https://github.com/aspect-build/mist-protocol"
              target="_blank"
              rel="noopener noreferrer"
              className="text-gray-400 hover:text-white transition-colors"
              aria-label="GitHub"
            >
              <svg
                className="w-5 h-5"
                fill="currentColor"
                viewBox="0 0 24 24"
                aria-hidden="true"
              >
                <path
                  fillRule="evenodd"
                  d="M12 2C6.477 2 2 6.484 2 12.017c0 4.425 2.865 8.18 6.839 9.504.5.092.682-.217.682-.483 0-.237-.008-.868-.013-1.703-2.782.605-3.369-1.343-3.369-1.343-.454-1.158-1.11-1.466-1.11-1.466-.908-.62.069-.608.069-.608 1.003.07 1.531 1.032 1.531 1.032.892 1.53 2.341 1.088 2.91.832.092-.647.35-1.088.636-1.338-2.22-.253-4.555-1.113-4.555-4.951 0-1.093.39-1.988 1.029-2.688-.103-.253-.446-1.272.098-2.65 0 0 .84-.27 2.75 1.026A9.564 9.564 0 0112 6.844c.85.004 1.705.115 2.504.337 1.909-1.296 2.747-1.027 2.747-1.027.546 1.379.202 2.398.1 2.651.64.7 1.028 1.595 1.028 2.688 0 3.848-2.339 4.695-4.566 4.943.359.309.678.92.678 1.855 0 1.338-.012 2.419-.012 2.747 0 .268.18.58.688.482A10.019 10.019 0 0022 12.017C22 6.484 17.522 2 12 2z"
                  clipRule="evenodd"
                />
              </svg>
            </a>
          </div>
        </div>
      </footer>
    </div>
  );
}
