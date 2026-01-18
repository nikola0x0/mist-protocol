"use client";

import { useState, useEffect } from "react";
import { useCurrentAccount } from "@mysten/dapp-kit";
import { useDepositNotes } from "../hooks/useDepositNotes";
import { formatAmount, DepositNote } from "../lib/deposit-notes";
import { checkRelayerStatus } from "../lib/relayer";
import Image from "next/image";
import dynamic from "next/dynamic";
import { ArrowDown, ChevronDown, X, HelpCircle, TrendingUp } from "lucide-react";

// TradingView chart - loaded client-side only
const AdvancedChart = dynamic(
  () => import("react-tradingview-embed").then((mod) => mod.AdvancedChart),
  { ssr: false, loading: () => <div className="h-full w-full bg-white/5 rounded-xl animate-pulse" /> }
);

// Token types
const MIST_TOKEN_TYPE = "0x1071c10ef6fa032cd54f51948b5193579e6596ffaecd173df2dac6f73e31a468::mist_token::MIST_TOKEN";

// DEX options with output token configuration
const DEX_OPTIONS = [
  { id: "flowx", name: "FlowX", logo: "/assets/dex/flowX.svg", outputToken: "MIST", outputIcon: "/assets/token-icons/mist-token.png", outputType: MIST_TOKEN_TYPE },
  { id: "walrus", name: "Walrus Swap", logo: "/assets/dex/walrus-swap.svg", outputToken: "WAL", outputIcon: "/assets/token-icons/wal.svg", outputType: "WAL" },
];

export function SwapCard() {
  const [selectedNote, setSelectedNote] = useState<DepositNote | null>(null);
  const [swapAmount, setSwapAmount] = useState("");
  const [showConfirm, setShowConfirm] = useState(false);
  const [showNoteSelector, setShowNoteSelector] = useState(false);
  const [selectedDex, setSelectedDex] = useState(DEX_OPTIONS[0]);

  // Privacy settings
  const [useRelayer, setUseRelayer] = useState(false);
  const [relayerAvailable, setRelayerAvailable] = useState(false);

  // Slippage settings
  const [slippage, setSlippage] = useState("0.5");
  const [customSlippage, setCustomSlippage] = useState("");
  const SLIPPAGE_PRESETS = ["0.1", "0.5", "1.0"];

  // Price oracle settings
  const [showPriceOracle, setShowPriceOracle] = useState(false);

  const currentAccount = useCurrentAccount();
  const { unspentNotes, createSwapIntent, createSwapIntentViaRelayer, loading, error } = useDepositNotes();

  // Check if relayer is available on mount
  useEffect(() => {
    checkRelayerStatus().then((status) => {
      setRelayerAvailable(status.status === "ready");
    });
  }, []);

  // Set default note if available and none selected
  useEffect(() => {
    if (unspentNotes.length > 0 && !selectedNote) {
      // Don't auto-select to force user choice
    }
  }, [unspentNotes, selectedNote]);

  const handleSelectNote = (note: DepositNote) => {
    setSelectedNote(note);
    setSwapAmount(formatAmount(note.amount)); // Default to max
    setShowNoteSelector(false);
  };

  const handleSwap = async () => {
    if (!selectedNote || !swapAmount) return;

    // Get output token type from selected DEX
    const tokenOut = selectedDex.outputType;

    // Use relayer for extra privacy if enabled
    const result = useRelayer
      ? await createSwapIntentViaRelayer(selectedNote, swapAmount, tokenOut)
      : await createSwapIntent(selectedNote, swapAmount, tokenOut);

    if (result.success) {
      setShowConfirm(true);
      setSelectedNote(null);
      setSwapAmount("");
    }
  };

  const maxAmount = selectedNote ? formatAmount(selectedNote.amount) : "0";
  const parsedSwapAmount = parseFloat(swapAmount) || 0;
  const parsedMaxAmount = parseFloat(maxAmount) || 0;
  const isValidAmount =
    parsedSwapAmount > 0 && parsedSwapAmount <= parsedMaxAmount;

  if (!currentAccount) {
    return (
      <div className="max-w-[480px] mx-auto animate-slide-up">
        <div className="glass-card rounded-2xl p-4">
          <div className="text-center py-12 text-gray-400">
            Connect wallet to swap
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className="animate-slide-up relative flex justify-center">
      <div className="flex gap-6">
        {/* Swap Panel */}
        <div className="w-[480px] flex-shrink-0">
          {/* Header */}
          <div className="flex justify-between items-center mb-4 px-2">
            <h2 className="text-xl font-bold font-tektur text-white">Swap</h2>
          </div>

          {/* Main Swap UI */}
          <div className="relative">
        
        {/* Input Card (Pay) */}
        <div className="glass-card rounded-2xl p-4 mb-2 border border-white/5 hover:border-white/10 transition-colors">
          <div className="flex justify-between text-sm text-gray-400 mb-2 font-medium">
            <span>Pay</span>
            {selectedNote && (
              <span className="text-xs">
                Balance: {formatAmount(selectedNote.amount)}
              </span>
            )}
          </div>
          
          <div className="flex items-center gap-2">
            <input
              type="number"
              value={swapAmount}
              onChange={(e) => setSwapAmount(e.target.value)}
              placeholder="0"
              className="bg-transparent text-4xl font-medium flex-1 min-w-0 outline-none text-white placeholder-gray-600"
              disabled={!selectedNote}
            />

            <button
              onClick={() => setShowNoteSelector(true)}
              className="flex items-center gap-2 bg-white/5 hover:bg-white/10 transition-colors rounded-full pl-2 pr-3 py-1.5 flex-shrink-0 shadow-lg border border-white/5"
            >
              <div className="w-6 h-6 relative rounded-full overflow-hidden">
                <Image src="/assets/token-icons/sui.png" alt="SUI" fill className="object-cover" />
              </div>
              <span className="font-bold text-lg text-white">SUI</span>
              <ChevronDown size={16} />
            </button>
          </div>
          
          {selectedNote && (
            <div className="mt-2 text-xs text-gray-500 font-mono flex justify-between items-center">
              <span>Note: ...{selectedNote.nullifier.slice(-6)}</span>
              <button onClick={() => setSwapAmount(maxAmount)} className="text-blue-400 hover:text-blue-300">MAX</button>
            </div>
          )}
        </div>

        {/* Swap Arrow */}
        <div className="flex justify-center -my-6 relative z-10">
          <div className="w-12 h-12 rounded-full bg-[#0a0a0a] border border-white/10 flex items-center justify-center text-gray-400 hover:text-white hover:bg-white/15 transition-colors cursor-pointer">
            <ArrowDown size={18} strokeWidth={2.5} />
          </div>
        </div>

        {/* Output Card (Receive) */}
        <div className="glass-card rounded-2xl p-4 mb-4 border border-white/5 hover:border-white/10 transition-colors">
          <div className="flex justify-between text-sm text-gray-400 mb-2 font-medium">
            <span>Receive</span>
            <span className="text-xs">via {selectedDex.name}</span>
          </div>

          <div className="flex items-center gap-2">
            <input
              type="text"
              value={swapAmount}
              placeholder="0"
              readOnly
              className="bg-transparent text-4xl font-medium flex-1 min-w-0 outline-none text-gray-400 cursor-default"
            />

            <button className="flex items-center gap-2 bg-white/5 hover:bg-white/10 transition-colors rounded-full pl-2 pr-3 py-1.5 flex-shrink-0 shadow-lg cursor-default border border-white/5">
              <div className="w-6 h-6 relative rounded-full overflow-hidden">
                <Image
                  src={selectedDex.outputIcon}
                  alt={selectedDex.outputToken}
                  fill
                  className="object-cover"
                />
              </div>
              <span className="font-bold text-lg text-white">
                {selectedDex.outputToken}
              </span>
            </button>
          </div>
        </div>

        {/* DEX Selection */}
        <div className="mb-4">
          <div className="text-xs text-gray-500 mb-2 px-1">Route via</div>
          <div className="flex gap-2">
            {DEX_OPTIONS.map((dex) => (
              <button
                key={dex.id}
                onClick={() => setSelectedDex(dex)}
                className={`flex items-center gap-2 px-3 py-2 rounded-xl transition-all ${
                  selectedDex.id === dex.id
                    ? "bg-white/10 border border-white/20"
                    : "bg-white/5 border border-transparent hover:bg-white/10"
                }`}
              >
                <Image
                  src={dex.logo}
                  alt={dex.name}
                  width={20}
                  height={20}
                  className="rounded-full"
                />
                <span className="text-sm text-white">{dex.name}</span>
              </button>
            ))}
          </div>
        </div>

        {/* Slippage & Price Oracle Settings */}
        <div className="mb-4 flex items-center justify-between gap-3">
          <div className="flex items-center gap-1.5">
            <span className="text-xs text-gray-500">Slippage</span>
            {SLIPPAGE_PRESETS.map((preset) => (
              <button
                key={preset}
                onClick={() => {
                  setSlippage(preset);
                  setCustomSlippage("");
                }}
                className={`px-2.5 py-1 rounded-lg text-xs font-medium transition-all ${
                  slippage === preset && !customSlippage
                    ? "bg-blue-600 text-white"
                    : "bg-white/5 text-gray-400 hover:bg-white/10 hover:text-white"
                }`}
              >
                {preset}%
              </button>
            ))}
          </div>
          <div className="flex items-center gap-2">
            <button
              onClick={() => setShowPriceOracle(!showPriceOracle)}
              className={`p-1.5 rounded-lg transition-all ${
                showPriceOracle
                  ? "bg-blue-600 text-white"
                  : "bg-white/5 text-gray-400 hover:bg-white/10 hover:text-white"
              }`}
              title="Price Oracle"
            >
              <TrendingUp size={14} />
            </button>
            <div className="relative w-20">
              <input
                type="number"
                value={customSlippage || slippage}
                onChange={(e) => {
                  const val = e.target.value;
                  if (val === "" || (parseFloat(val) >= 0 && parseFloat(val) <= 50)) {
                    setCustomSlippage(val);
                  }
                }}
                className={`w-full bg-white/5 border rounded-lg py-1 px-2 pr-6 text-xs text-right text-white outline-none ${
                  customSlippage ? "border-blue-500/50" : "border-white/10"
                }`}
              />
              <span className="absolute right-2 top-1/2 -translate-y-1/2 text-gray-500 text-xs">%</span>
            </div>
          </div>
        </div>

        {/* Privacy Relayer Toggle */}
        <div className="flex justify-between items-center py-3 px-1 mb-2">
          <div className="flex items-center gap-2">
            <span className="text-sm text-gray-400">Privacy Relayer</span>
            <div className="group relative">
              <HelpCircle size={14} className="text-gray-500 cursor-help" />
              <div className="absolute bottom-full left-1/2 -translate-x-1/2 mb-2 px-3 py-2 bg-gray-900 text-xs text-gray-300 rounded-lg opacity-0 group-hover:opacity-100 transition-opacity pointer-events-none w-48 text-center border border-white/10">
                Swap without revealing your wallet address on-chain.
              </div>
            </div>
          </div>
          <button
            onClick={() => relayerAvailable && setUseRelayer(!useRelayer)}
            disabled={!relayerAvailable}
            className={`w-11 h-6 rounded-full p-1 transition-colors ${
              useRelayer ? "bg-green-600" : "bg-white/10"
            } ${!relayerAvailable ? "opacity-50 cursor-not-allowed" : "cursor-pointer"}`}
          >
            <div
              className={`w-4 h-4 rounded-full bg-white transition-transform ${
                useRelayer ? "translate-x-5" : "translate-x-0"
              }`}
            />
          </button>
        </div>

        {/* Error Message */}
        {error && (
          <div className="mb-4 p-3 bg-red-500/10 border border-red-500/20 rounded-lg text-red-400 text-sm">
            {error}
          </div>
        )}

        {/* Action Button */}
        <button
          onClick={selectedNote ? handleSwap : () => setShowNoteSelector(true)}
          disabled={!!(selectedNote && (!isValidAmount || loading))}
          className={`w-full py-4 rounded-2xl font-bold text-xl transition-all shadow-lg font-tektur ${
            !selectedNote
              ? "bg-blue-600 hover:bg-blue-500 text-white shadow-blue-500/20"
              : loading
              ? "bg-white/10 text-gray-400 cursor-not-allowed"
              : isValidAmount
              ? useRelayer 
                ? "bg-green-600 hover:bg-green-500 text-white shadow-green-500/20" 
                : "bg-blue-600 hover:bg-blue-500 text-white shadow-blue-500/20"
              : "bg-white/10 text-gray-500 cursor-not-allowed"
          }`}
        >
          {loading
            ? (useRelayer ? "Submitting..." : "Swapping...")
            : !selectedNote
            ? "Select Deposit Note"
            : !isValidAmount
            ? "Enter Amount"
            : useRelayer
            ? "Private Swap (Relayer)"
            : "Swap"}
        </button>
          </div>
        </div>

        {/* Price Oracle Panel */}
        {showPriceOracle && (
          <div className="w-[520px] h-[580px] flex-shrink-0 rounded-2xl border border-white/10 bg-[#0F0F0F] animate-fade-in-right p-3">
            <div className="w-full h-full rounded-xl overflow-hidden">
              <AdvancedChart
                widgetProps={{
                  symbol: selectedDex.id === "walrus" ? "WALSUI_F4238F.USD" : "SUIUSD",
                  width: 496,
                  height: 556,
                interval: "15",
                timezone: "Etc/UTC",
                theme: "dark",
                style: "1",
                locale: "en",
                toolbar_bg: "#0a0a0a",
                enable_publishing: false,
                hide_top_toolbar: false,
                save_image: false,
                allow_symbol_change: false,
                }}
              />
            </div>
          </div>
        )}
      </div>

      {/* Deposit Selection Modal */}
      {showNoteSelector && (
        <div className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/60 backdrop-blur-md animate-fade-in">
          <div className="glass-card w-full max-w-md max-h-[80vh] flex flex-col shadow-2xl animate-slide-up rounded-2xl">
            <div className="flex justify-between items-center p-4 border-b border-white/10">
              <h3 className="font-bold text-lg text-white">Select Deposit Note</h3>
              <button 
                onClick={() => setShowNoteSelector(false)}
                className="text-gray-400 hover:text-white p-1 rounded-lg hover:bg-white/5"
              >
                <X size={24} />
              </button>
            </div>
            
            <div className="p-2 overflow-y-auto flex-1 custom-scrollbar">
              {unspentNotes.length === 0 ? (
                <div className="text-center py-12 text-gray-500">
                  No active deposits found.
                  <br />
                  <a href="/app" className="text-blue-400 hover:underline mt-2 inline-block">Go to Deposit</a>
                </div>
              ) : (
                <div className="space-y-2">
                  {unspentNotes.map((note) => (
                    <button
                      key={note.nullifier}
                      onClick={() => handleSelectNote(note)}
                      className={`w-full flex justify-between items-center p-4 rounded-xl transition-all ${
                        selectedNote?.nullifier === note.nullifier
                          ? "bg-blue-500/10 border border-blue-500/50"
                          : "bg-white/5 border border-transparent hover:bg-white/10"
                      }`}
                    >
                      <div className="flex items-center gap-3">
                        <div className="w-10 h-10 rounded-full bg-blue-500/20 flex items-center justify-center">
                          <Image src="/assets/token-icons/sui.png" alt="SUI" width={24} height={24} />
                        </div>
                        <div className="text-left">
                          <div className="font-bold text-white text-lg">
                            {formatAmount(note.amount)} SUI
                          </div>
                          <div className="text-xs text-gray-500 font-mono">
                            {new Date(note.timestamp).toLocaleDateString()}
                          </div>
                        </div>
                      </div>
                      {selectedNote?.nullifier === note.nullifier && (
                        <div className="text-blue-400">
                          <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="3" strokeLinecap="round" strokeLinejoin="round">
                            <polyline points="20 6 9 17 4 12" />
                          </svg>
                        </div>
                      )}
                    </button>
                  ))}
                </div>
              )}
            </div>
          </div>
        </div>
      )}

      {/* Success Modal */}
      {showConfirm && (
        <div className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/80 backdrop-blur-md">
          <div className="glass-card rounded-2xl p-6 max-w-sm w-full text-center animate-slide-up shadow-2xl">
            <div className="w-16 h-16 bg-green-500/20 rounded-full flex items-center justify-center mx-auto mb-4 text-green-500">
              <svg width="32" height="32" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="3" strokeLinecap="round" strokeLinejoin="round">
                <polyline points="20 6 9 17 4 12" />
              </svg>
            </div>
            <h3 className="text-2xl font-bold mb-2 text-white">Swap Submitted</h3>
            <p className="text-gray-400 mb-6">
              Your private swap has been {useRelayer ? "relayed" : "submitted"}. 
              Funds will arrive at your stealth address shortly.
            </p>
            <button
              onClick={() => setShowConfirm(false)}
              className="w-full glass-button text-white font-bold py-3 rounded-xl transition-colors hover:bg-white/10"
            >
              Close
            </button>
          </div>
        </div>
      )}
    </div>
  );
}
