"use client";

import { ConnectButton } from "@/components/ConnectButton";

export default function Home() {
  return (
    <div className="min-h-screen flex flex-col">
      <header className="border-b border-gray-200 dark:border-gray-800">
        <div className="container mx-auto px-4 py-4 flex justify-between items-center">
          <h1 className="text-2xl font-bold">Mist Protocol</h1>
          <ConnectButton />
        </div>
      </header>

      <main className="flex-1 container mx-auto px-4 py-12">
        <div className="max-w-2xl mx-auto text-center space-y-6">
          <h2 className="text-4xl font-bold">
            Privacy-Preserving DeFi on Sui
          </h2>
          <p className="text-xl text-gray-600 dark:text-gray-400">
            Trade with encrypted amounts using intent-based execution and
            verifiable TEE computation.
          </p>

          <div className="pt-8 grid grid-cols-1 md:grid-cols-3 gap-6">
            <div className="p-6 border border-gray-200 dark:border-gray-800 rounded-lg">
              <h3 className="font-bold mb-2">Intent-Based Trading</h3>
              <p className="text-sm text-gray-600 dark:text-gray-400">
                Express trading intents without exposing amounts
              </p>
            </div>
            <div className="p-6 border border-gray-200 dark:border-gray-800 rounded-lg">
              <h3 className="font-bold mb-2">TEE Verification</h3>
              <p className="text-sm text-gray-600 dark:text-gray-400">
                Nautilus-powered trusted execution with on-chain attestation
              </p>
            </div>
            <div className="p-6 border border-gray-200 dark:border-gray-800 rounded-lg">
              <h3 className="font-bold mb-2">Encrypted Escrow</h3>
              <p className="text-sm text-gray-600 dark:text-gray-400">
                Sui Move contracts with encrypted amount storage
              </p>
            </div>
          </div>
        </div>
      </main>

      <footer className="border-t border-gray-200 dark:border-gray-800 py-6">
        <div className="container mx-auto px-4 text-center text-sm text-gray-600 dark:text-gray-400">
          Built with Nautilus • Seal • Walrus • Cetus • Sui
        </div>
      </footer>
    </div>
  );
}
