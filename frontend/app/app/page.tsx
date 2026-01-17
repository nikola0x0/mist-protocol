"use client";

import { useState } from "react";
import Image from "next/image";
import { ConnectButton } from "@/components/ConnectButton";
import { DepositCard } from "@/components/DepositCard";
import { SwapCard } from "@/components/SwapCard";
import { StealthOutputsCard } from "@/components/StealthOutputsCard";
import { useCurrentAccount } from "@mysten/dapp-kit";
import { motion } from "framer-motion";
import Link from "next/link";
import { Wallet } from "lucide-react";

export default function AppPage() {
  const account = useCurrentAccount();
  const [activeTab, setActiveTab] = useState<"deposit" | "swap" | "claim">(
    "deposit"
  );

  return (
    <div className="min-h-screen bg-[#050505] text-white overflow-hidden relative">
      {/* Animated Light Beams Background */}
      <div className="light-beams-container">
        {Array.from({ length: 4 }, (_, i) => (
          <div key={i} className="light-beam" />
        ))}
        <div className="light-edge-horizontal" />
        <div className="light-edge-vertical" />
      </div>

      {/* Blur overlay to soften beams */}
      <div className="fixed inset-0 backdrop-blur-xl z-[2] pointer-events-none" />

      {/* Header */}
      <header className="border-none backdrop-blur-xl sticky top-0 z-50">
        <div className="px-6 py-4 flex justify-between items-center">
          <div className="flex items-center gap-3">
            <Image
              src="/assets/logo.svg"
              alt="MistTx"
              width={36}
              height={36}
              className="opacity-90"
            />
            <h1 className="text-xl font-bold font-tektur gradient-text leading-none">
              MistTx
            </h1>
          </div>
          <ConnectButton />
        </div>
      </header>

      {/* Main Content Area */}
      <div className="relative mx-auto px-4 py-12 z-10 max-w-[1050px]">
        {/* Navigation Tabs */}
        <div className="flex p-1 glass-panel rounded-xl mb-8 w-[480px] mx-auto">
          {[
            { key: "deposit", label: "Deposit" },
            { key: "swap", label: "Swap" },
            { key: "claim", label: "Claim" },
          ].map((tab) => (
            <button
              key={tab.key}
              onClick={() =>
                setActiveTab(tab.key as "deposit" | "swap" | "claim")
              }
              className={`flex-1 py-2 rounded-lg text-sm font-medium transition-all ${
                activeTab === tab.key
                  ? "bg-white/10 text-white shadow-sm backdrop-blur-md"
                  : "text-gray-400 hover:text-white"
              }`}
            >
              {tab.label}
            </button>
          ))}
        </div>

        {/* Content Cards */}
        <div className="relative min-h-[400px]">
          {!account ? (
            <div className="w-[480px] mx-auto animate-slide-up">
              <div className="glass-card rounded-2xl p-8 text-center">
                <Wallet size={32} className="mx-auto mb-4 text-blue-500" />
                <h3 className="text-lg font-bold text-white mb-2">
                  Connect Wallet
                </h3>
                <p className="text-gray-400 text-sm mb-6">
                  Connect your Sui wallet to access private deposits and swaps.
                </p>
              </div>
            </div>
          ) : (
            <>
              {activeTab === "deposit" && <DepositCard />}
              {activeTab === "swap" && <SwapCard />}
              {activeTab === "claim" && <StealthOutputsCard />}
            </>
          )}
        </div>
      </div>

      {/* Powered By Logo - Peek from bottom */}
      <div className="fixed bottom-0 left-0 right-0 flex justify-center z-50 pointer-events-none">
        <Link href="/how-it-works" className="pointer-events-auto group">
          <motion.div
            initial={{
              opacity: 0,
              y: 60,
              filter: "drop-shadow(0 0 0px transparent)",
            }}
            animate={{
              opacity: 0.5,
              y: 50,
              filter: "drop-shadow(0 0 0px transparent)",
            }}
            whileHover={{
              opacity: 1,
              y: 15,
              scale: 1.15,
              filter: "drop-shadow(0 0 20px rgba(255,255,255,0.5))",
            }}
            transition={{ type: "spring", stiffness: 300, damping: 25 }}
            className="cursor-pointer relative"
          >
            <div className="absolute -top-8 left-1/2 -translate-x-1/2 px-3 py-1 bg-white/10 backdrop-blur-md text-xs text-white rounded-full opacity-0 group-hover:opacity-100 transition-opacity whitespace-nowrap border border-white/10">
              How it works?
            </div>
            <Image
              src="/assets/logos/seal-nautilus.jpg"
              alt="Powered by SEAL & Nautilus"
              width={220}
              height={80}
              className="rounded-t-xl"
            />
          </motion.div>
        </Link>
      </div>
    </div>
  );
}
