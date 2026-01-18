"use client";

import { useState } from 'react';
import {
  useCurrentAccount,
  useSignAndExecuteTransaction,
} from '@mysten/dapp-kit';
import { Transaction } from '@mysten/sui/transactions';

const BACKEND_URL = process.env.NEXT_PUBLIC_BACKEND_URL || 'http://localhost:3001';

const TOKENS = {
  SUI: {
    type: '0x2::sui::SUI',
    symbol: 'SUI',
    decimals: 9,
  },
  MIST: {
    type: '0x1071c10ef6fa032cd54f51948b5193579e6596ffaecd173df2dac6f73e31a468::mist_token::MIST_TOKEN',
    symbol: 'MIST',
    decimals: 9,
  },
};

interface SwapQuote {
  tx_bytes: string;
  swap_info: {
    amount_in: number;
    min_amount_out: number;
    direction: string;
  };
  estimated_gas: number;
}

export default function FlowXSwapCard() {
  const account = useCurrentAccount();
  const { mutate: signAndExecute } = useSignAndExecuteTransaction();

  const [amountIn, setAmountIn] = useState<string>('');
  const [estimatedOut, setEstimatedOut] = useState<string>('0');
  const [slippage, setSlippage] = useState<string>('0.5');
  const [loading, setLoading] = useState(false);
  const [txStatus, setTxStatus] = useState<string>('');
  const [swapDirection, setSwapDirection] = useState<'sui-to-mist' | 'mist-to-sui'>('sui-to-mist');

  const inputToken = swapDirection === 'sui-to-mist' ? TOKENS.SUI : TOKENS.MIST;
  const outputToken = swapDirection === 'sui-to-mist' ? TOKENS.MIST : TOKENS.SUI;

  // Simple 1:1 estimation (in production, fetch from pool state)
  const estimateOutput = (input: string) => {
    if (!input || isNaN(parseFloat(input))) {
      setEstimatedOut('0');
      return;
    }

    const inputAmount = parseFloat(input);
    const feeRate = 0.003; // 0.3%
    const outputAmount = inputAmount * (1 - feeRate);
    setEstimatedOut(outputAmount.toFixed(6));
  };

  // Build swap transaction using backend API
  const buildSwapTransaction = async (): Promise<SwapQuote | null> => {
    if (!account) return null;

    try {
      const inputAmount = parseFloat(amountIn);
      if (isNaN(inputAmount) || inputAmount <= 0) {
        throw new Error('Invalid amount');
      }

      // Convert to smallest units
      const amountInUnits = Math.floor(inputAmount * Math.pow(10, inputToken.decimals));

      // Calculate minimum output with slippage tolerance
      const minAmountOut = 1; // For testing; in production, calculate based on slippage

      const requestBody = {
        user_address: account.address,
        amount: amountInUnits,
        min_amount_out: minAmountOut,
        is_sui_to_token: swapDirection === 'sui-to-mist',
      };

      console.log('Sending build-swap request to:', `${BACKEND_URL}/api/flowx/build-swap`);
      console.log('Request:', requestBody);

      const response = await fetch(`${BACKEND_URL}/api/flowx/build-swap`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(requestBody),
      });

      if (!response.ok) {
        const errorData = await response.json();
        throw new Error(errorData.error || 'Failed to build transaction');
      }

      return await response.json();
    } catch (error: any) {
      console.error('Failed to build transaction:', error);
      setTxStatus(`Error: ${error.message}`);
      return null;
    }
  };

  const handleSwap = async () => {
    if (!account) {
      setTxStatus('Please connect your wallet');
      return;
    }

    if (loading) return;

    setLoading(true);
    setTxStatus('Building transaction...');

    try {
      const quote = await buildSwapTransaction();

      if (!quote) {
        setTxStatus('Failed to build transaction');
        return;
      }

      setTxStatus('Waiting for wallet signature...');

      // Decode tx_bytes from base64
      const txBytes = Uint8Array.from(atob(quote.tx_bytes), c => c.charCodeAt(0));
      const transaction = Transaction.from(txBytes);

      signAndExecute(
        {
          transaction: transaction as any,
        },
        {
          onSuccess: (result) => {
            console.log('Swap successful!', result);
            setTxStatus(`✅ Swap successful! Digest: ${result.digest}`);
            setAmountIn('');
            setEstimatedOut('0');
          },
          onError: (error) => {
            console.error('Swap failed:', error);
            setTxStatus(`❌ Swap failed: ${error.message}`);
          },
        }
      );
    } catch (error: any) {
      console.error('Error:', error);
      setTxStatus(`❌ Error: ${error.message}`);
    } finally {
      setLoading(false);
    }
  };

  const switchDirection = () => {
    setSwapDirection(prev =>
      prev === 'sui-to-mist' ? 'mist-to-sui' : 'sui-to-mist'
    );
    setAmountIn('');
    setEstimatedOut('0');
    setTxStatus('');
  };

  return (
    <div className="glass-card p-6 max-w-md w-full">
      <h2 className="text-2xl font-bold mb-6 text-white">FlowX Swap</h2>

      {/* Pool Info */}
      <div className="mb-4 p-3 bg-white/5 rounded-lg">
        <div className="text-sm text-gray-400 mb-1">Pool</div>
        <div className="font-semibold text-white">MIST / SUI</div>
        <div className="text-xs text-gray-500 mt-1">Fee: 0.3% • Liquidity: 2 SUI + 2 MIST</div>
      </div>

      {/* Input Token */}
      <div className="mb-4">
        <label className="block text-sm font-medium text-gray-300 mb-2">
          You Pay
        </label>
        <div className="flex gap-2">
          <input
            type="number"
            value={amountIn}
            onChange={(e) => {
              setAmountIn(e.target.value);
              estimateOutput(e.target.value);
            }}
            placeholder="0.0"
            className="flex-1 bg-white/10 border border-white/20 rounded-lg px-4 py-3 text-white focus:outline-none focus:border-blue-500"
          />
          <div className="bg-white/10 border border-white/20 rounded-lg px-4 py-3 min-w-[100px] flex items-center justify-center font-semibold text-white">
            {inputToken.symbol}
          </div>
        </div>
      </div>

      {/* Swap Direction Button */}
      <div className="flex justify-center mb-4">
        <button
          onClick={switchDirection}
          className="bg-white/10 hover:bg-white/20 border border-white/20 rounded-full p-2 transition-all"
        >
          <svg className="w-6 h-6 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M7 16V4m0 0L3 8m4-4l4 4m6 0v12m0 0l4-4m-4 4l-4-4" />
          </svg>
        </button>
      </div>

      {/* Output Token */}
      <div className="mb-4">
        <label className="block text-sm font-medium text-gray-300 mb-2">
          You Receive (estimated)
        </label>
        <div className="flex gap-2">
          <input
            type="text"
            value={estimatedOut}
            readOnly
            placeholder="0.0"
            className="flex-1 bg-white/5 border border-white/20 rounded-lg px-4 py-3 text-white"
          />
          <div className="bg-white/10 border border-white/20 rounded-lg px-4 py-3 min-w-[100px] flex items-center justify-center font-semibold text-white">
            {outputToken.symbol}
          </div>
        </div>
      </div>

      {/* Slippage */}
      <div className="mb-6">
        <label className="block text-sm font-medium text-gray-300 mb-2">
          Slippage Tolerance (%)
        </label>
        <div className="flex gap-2">
          {['0.5', '1', '2'].map((value) => (
            <button
              key={value}
              onClick={() => setSlippage(value)}
              className={`flex-1 py-2 px-4 rounded-lg border transition-all ${
                slippage === value
                  ? 'bg-blue-600 border-blue-500 text-white'
                  : 'bg-white/10 border-white/20 text-gray-300 hover:bg-white/20'
              }`}
            >
              {value}%
            </button>
          ))}
          <input
            type="number"
            value={slippage}
            onChange={(e) => setSlippage(e.target.value)}
            className="w-20 bg-white/10 border border-white/20 rounded-lg px-3 py-2 text-white text-center focus:outline-none focus:border-blue-500"
            step="0.1"
            min="0.1"
            max="50"
          />
        </div>
      </div>

      {/* Swap Button */}
      <button
        onClick={handleSwap}
        disabled={loading || !account || !amountIn || parseFloat(amountIn) <= 0}
        className="w-full glass-button disabled:opacity-50 disabled:cursor-not-allowed"
      >
        {loading ? 'Swapping...' : account ? 'Swap' : 'Connect Wallet'}
      </button>

      {/* Status */}
      {txStatus && (
        <div className={`mt-4 p-3 rounded-lg ${
          txStatus.includes('✅')
            ? 'bg-green-500/20 text-green-300'
            : txStatus.includes('❌')
            ? 'bg-red-500/20 text-red-300'
            : 'bg-blue-500/20 text-blue-300'
        }`}>
          <p className="text-sm break-all">{txStatus}</p>
        </div>
      )}

      {/* Info */}
      <div className="mt-6 p-4 bg-white/5 rounded-lg">
        <div className="text-xs text-gray-400 space-y-1">
          <div className="flex justify-between">
            <span>Rate:</span>
            <span className="text-white">1 {inputToken.symbol} ≈ 1 {outputToken.symbol}</span>
          </div>
          <div className="flex justify-between">
            <span>Fee:</span>
            <span className="text-white">0.3%</span>
          </div>
          <div className="flex justify-between">
            <span>Min. received:</span>
            <span className="text-white">
              {(parseFloat(estimatedOut || '0') * (1 - parseFloat(slippage) / 100)).toFixed(6)} {outputToken.symbol}
            </span>
          </div>
        </div>
      </div>
    </div>
  );
}
