"use client";

import FlowXSwapCard from '@/components/FlowXSwapCard';
import Link from 'next/link';

export default function FlowXSwapPage() {
  return (
    <div className="min-h-screen bg-gradient-to-br from-gray-900 via-blue-900 to-purple-900">
      {/* Header */}
      <nav className="p-6 flex justify-between items-center border-b border-white/10">
        <Link href="/" className="text-2xl font-bold text-white hover:text-blue-400 transition-colors">
          Mist Protocol
        </Link>
        <div className="flex gap-4">
          <Link href="/app" className="text-gray-300 hover:text-white transition-colors">
            App
          </Link>
          <Link href="/cetus-swap" className="text-gray-300 hover:text-white transition-colors">
            Cetus Swap
          </Link>
          <Link href="/flowx-swap" className="text-white font-semibold">
            FlowX Swap
          </Link>
        </div>
      </nav>

      {/* Main Content */}
      <div className="container mx-auto px-4 py-12">
        <div className="flex flex-col lg:flex-row gap-8 max-w-6xl mx-auto">
          {/* Swap Card */}
          <div className="flex-1 flex justify-center">
            <FlowXSwapCard />
          </div>

          {/* Info Sidebar */}
          <div className="flex-1 space-y-6">
            <div className="glass-card p-6">
              <h3 className="text-xl font-bold text-white mb-4">About FlowX CLMM</h3>
              <p className="text-gray-300 text-sm mb-4">
                FlowX is a concentrated liquidity market maker (CLMM) on Sui Network.
                This swap interface allows you to trade MIST tokens directly with SUI using
                our dedicated liquidity pool.
              </p>
              <div className="space-y-2 text-sm">
                <div className="flex justify-between">
                  <span className="text-gray-400">Protocol:</span>
                  <span className="text-white font-semibold">FlowX CLMM</span>
                </div>
                <div className="flex justify-between">
                  <span className="text-gray-400">Network:</span>
                  <span className="text-white font-semibold">Sui Testnet</span>
                </div>
                <div className="flex justify-between">
                  <span className="text-gray-400">Pool Fee:</span>
                  <span className="text-white font-semibold">0.3%</span>
                </div>
              </div>
            </div>

            <div className="glass-card p-6">
              <h3 className="text-xl font-bold text-white mb-4">Pool Information</h3>
              <div className="space-y-3 text-sm">
                <div>
                  <div className="text-gray-400 mb-1">Pool Address</div>
                  <div className="text-white font-mono text-xs break-all bg-white/5 p-2 rounded">
                    0xcacee1...6a64d27
                  </div>
                </div>
                <div>
                  <div className="text-gray-400 mb-1">Current Liquidity</div>
                  <div className="text-white font-semibold">2 SUI + 2 MIST</div>
                </div>
                <div>
                  <div className="text-gray-400 mb-1">Price Range</div>
                  <div className="text-white font-semibold">0.5 - 2.0 SUI per MIST</div>
                </div>
                <div>
                  <div className="text-gray-400 mb-1">Current Price</div>
                  <div className="text-white font-semibold">1 MIST ≈ 1 SUI</div>
                </div>
              </div>
            </div>

            <div className="glass-card p-6">
              <h3 className="text-xl font-bold text-white mb-4">How It Works</h3>
              <ol className="space-y-3 text-sm text-gray-300">
                <li className="flex gap-3">
                  <span className="flex-shrink-0 w-6 h-6 bg-blue-600 rounded-full flex items-center justify-center text-white font-bold text-xs">
                    1
                  </span>
                  <span>Connect your Sui wallet (Sui Wallet, Suiet, or Ethos)</span>
                </li>
                <li className="flex gap-3">
                  <span className="flex-shrink-0 w-6 h-6 bg-blue-600 rounded-full flex items-center justify-center text-white font-bold text-xs">
                    2
                  </span>
                  <span>Enter the amount you want to swap</span>
                </li>
                <li className="flex gap-3">
                  <span className="flex-shrink-0 w-6 h-6 bg-blue-600 rounded-full flex items-center justify-center text-white font-bold text-xs">
                    3
                  </span>
                  <span>Review the estimated output and slippage settings</span>
                </li>
                <li className="flex gap-3">
                  <span className="flex-shrink-0 w-6 h-6 bg-blue-600 rounded-full flex items-center justify-center text-white font-bold text-xs">
                    4
                  </span>
                  <span>Click &quot;Swap&quot; and approve the transaction in your wallet</span>
                </li>
              </ol>
            </div>

            <div className="glass-card p-6 bg-yellow-500/10 border-yellow-500/20">
              <h3 className="text-lg font-bold text-yellow-300 mb-2 flex items-center gap-2">
                <svg className="w-5 h-5" fill="currentColor" viewBox="0 0 20 20">
                  <path fillRule="evenodd" d="M8.257 3.099c.765-1.36 2.722-1.36 3.486 0l5.58 9.92c.75 1.334-.213 2.98-1.742 2.98H4.42c-1.53 0-2.493-1.646-1.743-2.98l5.58-9.92zM11 13a1 1 0 11-2 0 1 1 0 012 0zm-1-8a1 1 0 00-1 1v3a1 1 0 002 0V6a1 1 0 00-1-1z" clipRule="evenodd" />
                </svg>
                Testnet Notice
              </h3>
              <p className="text-yellow-200 text-sm">
                This is running on Sui Testnet. Tokens have no real value.
                Get testnet SUI from the{' '}
                <a
                  href="https://discord.com/channels/916379725201563759/971488439931392130"
                  target="_blank"
                  rel="noopener noreferrer"
                  className="underline hover:text-white"
                >
                  Sui Discord faucet
                </a>
                .
              </p>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
