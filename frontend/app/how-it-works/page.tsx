"use client";

import Image from "next/image";
import Link from "next/link";
import { ArrowLeft, ArrowRight, Wallet, Eye, Lock, Box, Ghost, Download } from "lucide-react";
import { motion } from "framer-motion";

export default function HowItWorksPage() {
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
            <h1 className="text-xl font-bold font-tektur gradient-text">MistTx</h1>
          </div>
          <Link href="/app" className="flex items-center gap-2 text-gray-400 hover:text-white transition-colors">
            <ArrowLeft size={16} />
            <span className="text-sm">Back to App</span>
          </Link>
        </div>
      </header>

      {/* Content */}
      <main className="relative max-w-3xl mx-auto px-6 py-12 z-10">
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.5 }}
        >
          {/* Title */}
          <div className="text-center mb-10">
            <h1 className="text-3xl font-bold font-tektur mb-3">How MistTx Works</h1>
            <p className="text-gray-400 text-sm">
              A step-by-step look at private swaps on Sui
            </p>
          </div>

          {/* Story Steps */}
          <div className="space-y-8">
            {/* Step 1 */}
            <motion.section
              initial={{ opacity: 0, y: 15 }}
              animate={{ opacity: 1, y: 0 }}
              transition={{ delay: 0 }}
            >
              <div className="flex items-center gap-3 mb-3">
                <span className="text-red-400/60 font-mono text-sm">01</span>
                <h2 className="text-base font-bold font-tektur text-white">The problem with DeFi today</h2>
              </div>
              <div className="glass-card rounded-xl overflow-hidden ml-10">
                <div className="w-full py-8 px-6 bg-gradient-to-br from-red-950/30 to-transparent flex items-center justify-center gap-3">
                  <div className="flex flex-col items-center gap-2">
                    <div className="w-12 h-12 rounded-xl bg-white/10 flex items-center justify-center">
                      <Wallet className="text-gray-400" size={24} />
                    </div>
                    <span className="text-[10px] text-gray-500">Your Wallet</span>
                  </div>
                  <div className="flex-1 flex items-center relative -mt-5">
                    <div className="w-full border-t border-dashed border-red-500/40" />
                    <div className="absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 text-[10px] text-red-400/80 bg-[#0a0505] px-2">visible</div>
                  </div>
                  <div className="flex flex-col items-center gap-2">
                    <div className="w-12 h-12 rounded-xl bg-red-500/10 flex items-center justify-center">
                      <Eye className="text-red-400" size={24} />
                    </div>
                    <span className="text-[10px] text-gray-500">Observers</span>
                  </div>
                  <div className="flex-1 flex items-center -mt-5">
                    <div className="w-full border-t border-dashed border-red-500/40" />
                  </div>
                  <div className="flex flex-col items-center gap-2">
                    <div className="w-12 h-12 rounded-xl bg-red-500/10 flex items-center justify-center">
                      <Eye className="text-red-400" size={24} />
                    </div>
                    <span className="text-[10px] text-gray-500">Trackers</span>
                  </div>
                </div>
                <div className="p-5">
                  <p className="text-gray-300 text-sm leading-relaxed">
                    When you swap on any DEX, everything is public. Your wallet address, the tokens, the amounts -
                    all visible to anyone. Blockchain explorers, analytics firms, and bad actors can track your every move.
                  </p>
                </div>
              </div>
            </motion.section>

            {/* Step 2 */}
            <motion.section
              initial={{ opacity: 0, y: 15 }}
              animate={{ opacity: 1, y: 0 }}
              transition={{ delay: 0.1 }}
            >
              <div className="flex items-center gap-3 mb-3">
                <span className="text-blue-400/60 font-mono text-sm">02</span>
                <h2 className="text-base font-bold font-tektur text-white">You deposit SUI into MistTx</h2>
              </div>
              <div className="glass-card rounded-xl overflow-hidden ml-10">
                <div className="w-full py-8 px-6 bg-gradient-to-br from-blue-950/30 to-transparent flex items-center justify-center gap-3">
                  <div className="flex flex-col items-center gap-2">
                    <div className="w-12 h-12 rounded-xl bg-blue-500/10 flex items-center justify-center">
                      <Wallet className="text-blue-400" size={24} />
                    </div>
                    <span className="text-[10px] text-gray-500">Your Wallet</span>
                  </div>
                  <ArrowRight className="text-blue-500/40 -mt-5" size={20} />
                  <div className="flex flex-col items-center gap-2">
                    <div className="w-14 h-14 rounded-xl bg-blue-500/20 flex items-center justify-center border border-blue-500/30">
                      <Box className="text-blue-400" size={28} />
                    </div>
                    <span className="text-[10px] text-gray-500">MistTx Pool</span>
                  </div>
                  <ArrowRight className="text-blue-500/40 -mt-5" size={20} />
                  <div className="flex flex-col items-center gap-2">
                    <div className="w-12 h-12 rounded-xl bg-green-500/10 flex items-center justify-center border border-green-500/30">
                      <Lock className="text-green-400" size={24} />
                    </div>
                    <span className="text-[10px] text-gray-500">Secret Note</span>
                  </div>
                </div>
                <div className="p-5">
                  <p className="text-gray-300 text-sm leading-relaxed">
                    Deposit SUI into MistTx&apos;s pool. You get a <strong className="text-white">secret deposit note</strong> stored only on your device.
                    This note is your key to accessing your funds later - no one else has it.
                  </p>
                </div>
              </div>
            </motion.section>

            {/* Step 3 */}
            <motion.section
              initial={{ opacity: 0, y: 15 }}
              animate={{ opacity: 1, y: 0 }}
              transition={{ delay: 0.2 }}
            >
              <div className="flex items-center gap-3 mb-3">
                <span className="text-purple-400/60 font-mono text-sm">03</span>
                <h2 className="text-base font-bold font-tektur text-white">Your swap amount gets encrypted</h2>
              </div>
              <div className="glass-card rounded-xl overflow-hidden ml-10">
                <div className="w-full py-8 px-6 bg-gradient-to-br from-purple-950/30 to-transparent flex items-center justify-center gap-4">
                  <div className="flex flex-col items-center gap-2">
                    <div className="px-4 py-3 rounded-lg bg-white/10 font-mono text-lg text-white">
                      100 SUI
                    </div>
                    <span className="text-[10px] text-gray-500">Your Amount</span>
                  </div>
                  <ArrowRight className="text-purple-500/40 -mt-5" size={20} />
                  <div className="flex flex-col items-center gap-2">
                    <div className="w-12 h-12 rounded-xl bg-purple-500/20 flex items-center justify-center border border-purple-500/30">
                      <Lock className="text-purple-400" size={24} />
                    </div>
                    <span className="text-[10px] text-gray-500">SEAL</span>
                  </div>
                  <ArrowRight className="text-purple-500/40 -mt-5" size={20} />
                  <div className="flex flex-col items-center gap-2">
                    <div className="px-4 py-3 rounded-lg bg-purple-500/10 font-mono text-lg text-purple-400 border border-purple-500/20">
                      0x7f3...
                    </div>
                    <span className="text-[10px] text-gray-500">Encrypted</span>
                  </div>
                </div>
                <div className="p-5">
                  <p className="text-gray-300 text-sm leading-relaxed">
                    We use <strong className="text-white">SEAL threshold encryption</strong> to hide the amount.
                    On-chain, it appears as random bytes - no one can tell what you&apos;re swapping.
                  </p>
                </div>
              </div>
            </motion.section>

            {/* Step 4 */}
            <motion.section
              initial={{ opacity: 0, y: 15 }}
              animate={{ opacity: 1, y: 0 }}
              transition={{ delay: 0.3 }}
            >
              <div className="flex items-center gap-3 mb-3">
                <span className="text-cyan-400/60 font-mono text-sm">04</span>
                <h2 className="text-base font-bold font-tektur text-white">The swap executes in a secure enclave</h2>
              </div>
              <div className="glass-card rounded-xl overflow-hidden ml-10">
                <div className="w-full py-8 px-6 bg-gradient-to-br from-cyan-950/30 to-transparent flex items-center justify-center gap-3">
                  <div className="flex flex-col items-center gap-2">
                    <div className="px-3 py-2 rounded-lg bg-purple-500/10 font-mono text-sm text-purple-400">
                      0x7f3...
                    </div>
                    <span className="text-[10px] text-gray-500">Encrypted</span>
                  </div>
                  <ArrowRight className="text-cyan-500/40 -mt-5" size={20} />
                  <div className="flex flex-col items-center gap-2">
                    <div className="w-20 h-16 rounded-xl bg-cyan-500/10 flex items-center justify-center border-2 border-cyan-500/40 relative">
                      <Box className="text-cyan-400" size={28} />
                      <div className="absolute -bottom-1 -right-1 w-4 h-4 rounded-full bg-green-500 flex items-center justify-center">
                        <Lock className="text-white" size={10} />
                      </div>
                    </div>
                    <span className="text-[10px] text-gray-500">TEE Enclave</span>
                  </div>
                  <ArrowRight className="text-cyan-500/40 -mt-5" size={20} />
                  <div className="flex flex-col items-center gap-2">
                    <div className="px-3 py-2 rounded-lg bg-green-500/10 font-mono text-sm text-green-400">
                      Swapped
                    </div>
                    <span className="text-[10px] text-gray-500">Output</span>
                  </div>
                </div>
                <div className="p-5">
                  <p className="text-gray-300 text-sm leading-relaxed">
                    Our backend runs inside <strong className="text-white">Nautilus TEE</strong> - a hardware-isolated environment.
                    Even we cannot see what happens inside. It&apos;s like a sealed black box.
                  </p>
                </div>
              </div>
            </motion.section>

            {/* Step 5 */}
            <motion.section
              initial={{ opacity: 0, y: 15 }}
              animate={{ opacity: 1, y: 0 }}
              transition={{ delay: 0.4 }}
            >
              <div className="flex items-center gap-3 mb-3">
                <span className="text-indigo-400/60 font-mono text-sm">05</span>
                <h2 className="text-base font-bold font-tektur text-white">Tokens arrive at a stealth address</h2>
              </div>
              <div className="glass-card rounded-xl overflow-hidden ml-10">
                <div className="w-full py-8 px-6 bg-gradient-to-br from-indigo-950/30 to-transparent flex items-center justify-center gap-4">
                  <div className="flex flex-col items-center gap-2 opacity-40">
                    <div className="w-12 h-12 rounded-xl bg-white/10 flex items-center justify-center">
                      <Wallet className="text-gray-500" size={24} />
                    </div>
                    <span className="text-[10px] text-gray-600">Original</span>
                  </div>
                  <div className="flex-1 flex items-center relative -mt-5">
                    <div className="w-full border-t border-dashed border-red-500/30" />
                    <div className="absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 text-red-500/70 text-lg">✕</div>
                  </div>
                  <div className="flex flex-col items-center gap-2">
                    <div className="w-12 h-12 rounded-xl bg-indigo-500/20 flex items-center justify-center border border-indigo-500/40">
                      <Ghost className="text-indigo-400" size={24} />
                    </div>
                    <span className="text-[10px] text-gray-500">Stealth</span>
                  </div>
                  <ArrowRight className="text-green-500/40 -mt-5" size={20} />
                  <div className="flex flex-col items-center gap-2">
                    <div className="px-3 py-2 rounded-lg bg-green-500/10 font-mono text-sm text-green-400 border border-green-500/20">
                      USDC
                    </div>
                    <span className="text-[10px] text-gray-500">Received</span>
                  </div>
                </div>
                <div className="p-5">
                  <p className="text-gray-300 text-sm leading-relaxed">
                    Swapped tokens go to a <strong className="text-white">stealth address</strong> - a fresh address just for this transaction.
                    On-chain, there&apos;s no visible connection to your original wallet.
                  </p>
                </div>
              </div>
            </motion.section>

            {/* Step 6 */}
            <motion.section
              initial={{ opacity: 0, y: 15 }}
              animate={{ opacity: 1, y: 0 }}
              transition={{ delay: 0.5 }}
            >
              <div className="flex items-center gap-3 mb-3">
                <span className="text-green-400/60 font-mono text-sm">06</span>
                <h2 className="text-base font-bold font-tektur text-white">Claim your tokens anytime</h2>
              </div>
              <div className="glass-card rounded-xl overflow-hidden ml-10">
                <div className="w-full py-8 px-6 bg-gradient-to-br from-green-950/30 to-transparent flex items-center justify-center gap-3">
                  <div className="flex flex-col items-center gap-2">
                    <div className="w-12 h-12 rounded-xl bg-indigo-500/20 flex items-center justify-center">
                      <Ghost className="text-indigo-400" size={24} />
                    </div>
                    <span className="text-[10px] text-gray-500">Stealth</span>
                  </div>
                  <ArrowRight className="text-green-500/40 -mt-5" size={20} />
                  <div className="flex flex-col items-center gap-2">
                    <div className="w-12 h-12 rounded-xl bg-green-500/20 flex items-center justify-center border border-green-500/40">
                      <Lock className="text-green-400" size={24} />
                    </div>
                    <span className="text-[10px] text-gray-500">Your Key</span>
                  </div>
                  <ArrowRight className="text-green-500/40 -mt-5" size={20} />
                  <div className="flex flex-col items-center gap-2">
                    <div className="w-14 h-14 rounded-xl bg-green-500/20 flex items-center justify-center border-2 border-green-500/50">
                      <Download className="text-green-400" size={28} />
                    </div>
                    <span className="text-[10px] text-gray-500">Any Wallet</span>
                  </div>
                </div>
                <div className="p-5">
                  <p className="text-gray-300 text-sm leading-relaxed">
                    Using the keys on your device, claim tokens from your stealth address to any wallet.
                    The link is broken - <strong className="text-white">your swap is private</strong>.
                  </p>
                </div>
              </div>
            </motion.section>
          </div>

          {/* Security Note */}
          <motion.div
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            transition={{ delay: 0.6 }}
            className="mt-10 mb-8 p-5 border border-white/10 rounded-xl bg-white/[0.02]"
          >
            <div className="text-xs text-gray-500 uppercase tracking-wider mb-2">Security guarantees</div>
            <p className="text-gray-400 text-sm leading-relaxed">
              SEAL uses <strong className="text-gray-300">2-of-3 threshold encryption</strong> - no single server can decrypt your data.
              Nautilus runs in <strong className="text-gray-300">AWS Nitro Enclaves</strong> with cryptographic attestation.
              Your keys <strong className="text-gray-300">never leave your device</strong>.
            </p>
          </motion.div>

          {/* Powered By */}
          <div className="text-center mb-8">
            <div className="text-xs text-gray-500 uppercase tracking-widest mb-4">Powered By</div>
            <Image
              src="/assets/logos/seal-nautilus.jpg"
              alt="SEAL & Nautilus"
              width={200}
              height={70}
              className="mx-auto rounded-xl opacity-80"
            />
          </div>

          {/* Back to App */}
          <div className="text-center">
            <Link href="/app">
              <button className="bg-blue-600 hover:bg-blue-500 px-8 py-3 rounded-xl font-tektur text-white transition-colors shadow-lg shadow-blue-500/20">
                Start Using MistTx
              </button>
            </Link>
          </div>
        </motion.div>
      </main>
    </div>
  );
}
