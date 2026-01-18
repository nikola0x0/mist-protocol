"use client";

import { ConnectButton } from "@mysten/dapp-kit";
import CetusSwapCard from "@/components/CetusSwapCard";
import Link from "next/link";

export default function CetusSwapPage() {
  return (
    <div className="min-h-screen bg-gradient-to-br from-blue-50 via-purple-50 to-pink-50">
      {/* Header */}
      <div className="bg-white/80 backdrop-blur-sm border-b border-purple-100 sticky top-0 z-10">
        <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
          <div className="flex justify-between items-center py-4">
            <div className="flex items-center gap-4">
              <Link
                href="/"
                className="text-gray-600 hover:text-gray-900 transition flex items-center gap-2"
              >
                <svg
                  className="w-5 h-5"
                  fill="none"
                  stroke="currentColor"
                  viewBox="0 0 24 24"
                >
                  <path
                    strokeLinecap="round"
                    strokeLinejoin="round"
                    strokeWidth={2}
                    d="M10 19l-7-7m0 0l7-7m-7 7h18"
                  />
                </svg>
                Back to Home
              </Link>
              <div className="h-6 w-px bg-gray-300" />
              <h1 className="text-2xl font-bold bg-gradient-to-r from-blue-600 to-cyan-600 bg-clip-text text-transparent">
                Cetus DEX Integration
              </h1>
            </div>
            <ConnectButton />
          </div>
        </div>
      </div>

      {/* Main Content */}
      <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-12">
        <div className="grid lg:grid-cols-3 gap-8">
          {/* Main Swap Card */}
          <div className="lg:col-span-2">
            <CetusSwapCard />
          </div>

          {/* Info Sidebar */}
          <div className="space-y-6">
            {/* About Card */}
            <div className="bg-white rounded-3xl shadow-xl p-6 border border-purple-100">
              <h3 className="text-lg font-bold text-gray-900 mb-4">About Cetus</h3>
              <div className="space-y-3 text-sm text-gray-600">
                <p>
                  Cetus is a decentralized exchange (DEX) on the Sui blockchain featuring concentrated liquidity pools.
                </p>
                <p>
                  This integration allows you to swap tokens directly through Cetus pools using our backend infrastructure.
                </p>
              </div>
            </div>

            {/* Features Card */}
            <div className="bg-white rounded-3xl shadow-xl p-6 border border-purple-100">
              <h3 className="text-lg font-bold text-gray-900 mb-4">Features</h3>
              <ul className="space-y-2 text-sm text-gray-600">
                <li className="flex items-start gap-2">
                  <span className="text-green-500 mt-0.5">✓</span>
                  <span>Multiple token pairs (USDC, USDT, CETUS, SUI)</span>
                </li>
                <li className="flex items-start gap-2">
                  <span className="text-green-500 mt-0.5">✓</span>
                  <span>Configurable slippage tolerance</span>
                </li>
                <li className="flex items-start gap-2">
                  <span className="text-green-500 mt-0.5">✓</span>
                  <span>Bidirectional swaps (A→B and B→A)</span>
                </li>
                <li className="flex items-start gap-2">
                  <span className="text-green-500 mt-0.5">✓</span>
                  <span>Low fees (0.25% pool fee)</span>
                </li>
                <li className="flex items-start gap-2">
                  <span className="text-green-500 mt-0.5">✓</span>
                  <span>Wallet-signed transactions (keys stay in your browser)</span>
                </li>
              </ul>
            </div>

            {/* How It Works Card */}
            <div className="bg-white rounded-3xl shadow-xl p-6 border border-purple-100">
              <h3 className="text-lg font-bold text-gray-900 mb-4">How It Works</h3>
              <ol className="space-y-3 text-sm text-gray-600">
                <li className="flex gap-3">
                  <span className="flex-shrink-0 w-6 h-6 bg-blue-100 text-blue-600 rounded-full flex items-center justify-center font-semibold text-xs">
                    1
                  </span>
                  <span>Connect your Sui wallet</span>
                </li>
                <li className="flex gap-3">
                  <span className="flex-shrink-0 w-6 h-6 bg-blue-100 text-blue-600 rounded-full flex items-center justify-center font-semibold text-xs">
                    2
                  </span>
                  <span>Select a liquidity pool and swap direction</span>
                </li>
                <li className="flex gap-3">
                  <span className="flex-shrink-0 w-6 h-6 bg-blue-100 text-blue-600 rounded-full flex items-center justify-center font-semibold text-xs">
                    3
                  </span>
                  <span>Enter the amount you want to swap</span>
                </li>
                <li className="flex gap-3">
                  <span className="flex-shrink-0 w-6 h-6 bg-blue-100 text-blue-600 rounded-full flex items-center justify-center font-semibold text-xs">
                    4
                  </span>
                  <span>Backend builds an unsigned transaction</span>
                </li>
                <li className="flex gap-3">
                  <span className="flex-shrink-0 w-6 h-6 bg-blue-100 text-blue-600 rounded-full flex items-center justify-center font-semibold text-xs">
                    5
                  </span>
                  <span>Sign the transaction in your wallet</span>
                </li>
                <li className="flex gap-3">
                  <span className="flex-shrink-0 w-6 h-6 bg-blue-100 text-blue-600 rounded-full flex items-center justify-center font-semibold text-xs">
                    6
                  </span>
                  <span>Transaction is submitted to Sui blockchain</span>
                </li>
              </ol>
            </div>

            {/* Security Notice */}
            <div className="bg-gradient-to-br from-green-50 to-emerald-50 rounded-3xl p-6 border border-green-200">
              <div className="flex items-start gap-3">
                <span className="text-2xl">🔒</span>
                <div>
                  <h4 className="font-semibold text-gray-900 mb-1">Security</h4>
                  <p className="text-sm text-gray-600">
                    Your private keys never leave your browser. All transactions are signed locally in your wallet.
                  </p>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
