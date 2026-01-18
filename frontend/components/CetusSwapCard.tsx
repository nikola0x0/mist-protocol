"use client";

import { useState } from 'react';
import {
  useCurrentAccount,
  useSignAndExecuteTransaction,
} from '@mysten/dapp-kit';
import { Transaction } from '@mysten/sui/transactions';

const BACKEND_URL = process.env.NEXT_PUBLIC_BACKEND_URL || 'http://localhost:3001';

interface Pool {
  swap_account: string;
  symbol: string;
  token_a_address: string;
  token_b_address: string;
  fee: string;
  current_sqrt_price?: string;
  tvl_in_usd?: string;
  vol_in_usd_24h?: string;
}

interface SwapQuote {
  tx_bytes: string;
  pool_info: {
    pool_address: string;
    symbol: string;
    fee_rate: string;
    expected_output: number;
    price_impact: number;
  };
  estimated_gas: number;
}

export default function CetusSwapCard() {
  const account = useCurrentAccount();
  const { mutate: signAndExecute } = useSignAndExecuteTransaction();

  const [pools, setPools] = useState<Pool[]>([]);
  const [selectedPool, setSelectedPool] = useState<Pool | null>(null);
  const [amountIn, setAmountIn] = useState<string>('');
  const [slippage, setSlippage] = useState<string>('2');
  const [loading, setLoading] = useState(false);
  const [txStatus, setTxStatus] = useState<string>('');
  const [swapDirection, setSwapDirection] = useState<'a2b' | 'b2a'>('a2b');

  // Hardcoded pools - Cetus API is currently unavailable
  const AVAILABLE_POOLS: Pool[] = [
    {
      swap_account: '0x06d8af9e6afd27262db436f0d37b304a041f710c3ea1fa4c3a9bab36b3569ad3',
      symbol: 'USDT-SUI',
      token_a_address: '0xc060006111016b8a020ad5b33834984a437aaa7d3c74c18e09a95d48aceab08c::coin::COIN',
      token_b_address: '0x0000000000000000000000000000000000000000000000000000000000000002::sui::SUI',
      fee: '0.0025',
    },
    {
      swap_account: '0x2e041f3fd93646dcc877f783c1f2b7fa62d30271bdef1f21ef002cebf857bded',
      symbol: 'CETUS-SUI',
      token_a_address: '0x06864a6f921804860930db6ddbe2e16acdf8504495ea7481637a1c8b9a8fe54b::cetus::CETUS',
      token_b_address: '0x0000000000000000000000000000000000000000000000000000000000000002::sui::SUI',
      fee: '0.0025',
    },
    {
      swap_account: '0xcf994611fd4c48e277ce3ffd4d4364c914af2c3cbb05f7bf6facd371de688630',
      symbol: 'USDC-SUI (Wormhole)',
      token_a_address: '0x5d4b302506645c37ff133b98c4b50a5ae14841659738d6d733d59d0d217a93bf::coin::COIN',
      token_b_address: '0x0000000000000000000000000000000000000000000000000000000000000002::sui::SUI',
      fee: '0.0025',
    },
    {
      swap_account: '0x51e883ba7c0b566a26cbc8a94cd33eb0abd418a77cc1e60ad22fd9b1f29cd2ab',
      symbol: 'USDC-SUI (Native)',
      token_a_address: '0xdba34672e30cb065b1f93e3ab55318768fd6fef66c15942c9f7cb846e2f900e7::usdc::USDC',
      token_b_address: '0x0000000000000000000000000000000000000000000000000000000000000002::sui::SUI',
      fee: '0.0025',
    },
  ];

  const fetchPools = () => {
    // Just load hardcoded pools directly
    setPools(AVAILABLE_POOLS);
    setSelectedPool(AVAILABLE_POOLS[0]);
  };

  const buildSwapTransaction = async (): Promise<SwapQuote | null> => {
    if (!account || !selectedPool) return null;

    try {
      const getTokenDecimals = (tokenAddress: string) => {
        if (tokenAddress.includes('::sui::SUI')) return 1_000_000_000;
        if (tokenAddress.includes('::cetus::CETUS')) return 1_000_000_000;
        return 1_000_000;
      };

      const inputToken = swapDirection === 'a2b' ? selectedPool.token_a_address : selectedPool.token_b_address;
      const inputDecimals = getTokenDecimals(inputToken);

      const requestBody = {
        user_address: account.address,
        token_a: selectedPool.token_a_address,
        token_b: selectedPool.token_b_address,
        amount: Math.floor(parseFloat(amountIn) * inputDecimals),
        slippage: parseFloat(slippage) / 100,
        a_to_b: swapDirection === 'a2b',
      };

      console.log('Sending build-swap request to:', `${BACKEND_URL}/api/cetus/build-swap`);

      const response = await fetch(`${BACKEND_URL}/api/cetus/build-swap`, {
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

  const executeSwap = async () => {
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

      const txBytes = Uint8Array.from(atob(quote.tx_bytes), c => c.charCodeAt(0));
      const transaction = Transaction.from(txBytes);

      signAndExecute(
        {
          transaction: transaction as any, // Type workaround for package version mismatch
        },
        {
          onSuccess: (result) => {
            console.log('Swap successful!', result);
            setTxStatus(`✅ Swap successful! Digest: ${result.digest}`);
          },
          onError: (error) => {
            console.error('Swap failed:', error);
            setTxStatus(`❌ Swap failed: ${error.message}`);
          },
        }
      );
    } catch (error: any) {
      console.error('Error:', error);
      setTxStatus(`Error: ${error.message}`);
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="bg-white rounded-3xl shadow-xl p-8 border border-purple-100">
      <div className="flex items-center gap-3 mb-6">
        <div className="w-12 h-12 bg-gradient-to-br from-blue-500 to-cyan-400 rounded-2xl flex items-center justify-center">
          <span className="text-2xl">🌊</span>
        </div>
        <div>
          <h2 className="text-2xl font-bold bg-gradient-to-r from-blue-600 to-cyan-600 bg-clip-text text-transparent">
            Cetus Swap
          </h2>
          <p className="text-sm text-gray-500">Public DEX Integration</p>
        </div>
      </div>

      {!account ? (
        <div className="text-center py-8">
          <p className="text-gray-600 mb-4">Connect your wallet to start swapping</p>
        </div>
      ) : (
        <div className="space-y-6">
          {/* Pool Selection */}
          <div>
            <label className="block text-sm font-medium text-gray-700 mb-2">
              Select Pool
            </label>
            {pools.length === 0 ? (
              <button
                onClick={fetchPools}
                className="w-full bg-gradient-to-r from-blue-500 to-cyan-500 text-white py-3 px-4 rounded-xl hover:from-blue-600 hover:to-cyan-600 transition font-medium"
                disabled={loading}
              >
                Load Available Pools
              </button>
            ) : (
              <select
                className="w-full border border-gray-300 rounded-xl p-3 text-gray-900 bg-white focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                value={selectedPool?.swap_account || ''}
                onChange={(e) => {
                  const pool = pools.find(p => p.swap_account === e.target.value);
                  setSelectedPool(pool || null);
                }}
              >
                {pools.map((pool) => (
                  <option key={pool.swap_account} value={pool.swap_account}>
                    {pool.symbol} - Fee: {(parseFloat(pool.fee) * 100).toFixed(2)}%
                  </option>
                ))}
              </select>
            )}
          </div>

          {/* Swap Direction */}
          {selectedPool && (
            <div>
              <label className="block text-sm font-medium text-gray-700 mb-2">
                Swap Direction
              </label>
              <div className="grid grid-cols-2 gap-2">
                <button
                  onClick={() => setSwapDirection('a2b')}
                  className={`py-3 px-4 rounded-xl font-medium transition ${
                    swapDirection === 'a2b'
                      ? 'bg-gradient-to-r from-blue-500 to-cyan-500 text-white'
                      : 'bg-gray-100 text-gray-700 hover:bg-gray-200'
                  }`}
                >
                  {selectedPool.symbol.split('-')[0]} → {selectedPool.symbol.split('-')[1]}
                </button>
                <button
                  onClick={() => setSwapDirection('b2a')}
                  className={`py-3 px-4 rounded-xl font-medium transition ${
                    swapDirection === 'b2a'
                      ? 'bg-gradient-to-r from-blue-500 to-cyan-500 text-white'
                      : 'bg-gray-100 text-gray-700 hover:bg-gray-200'
                  }`}
                >
                  {selectedPool.symbol.split('-')[1]} → {selectedPool.symbol.split('-')[0]}
                </button>
              </div>
            </div>
          )}

          {/* Amount Input */}
          <div>
            <label className="block text-sm font-medium text-gray-700 mb-2">
              Amount to Swap
              {selectedPool && ` (${swapDirection === 'a2b'
                ? selectedPool.symbol.split('-')[0]
                : selectedPool.symbol.split('-')[1]})`}
            </label>
            <input
              type="number"
              value={amountIn}
              onChange={(e) => setAmountIn(e.target.value)}
              className="w-full border border-gray-300 rounded-xl p-3 text-gray-900 bg-white focus:ring-2 focus:ring-blue-500 focus:border-transparent"
              placeholder="0.0"
              step="0.000001"
            />
          </div>

          {/* Slippage Input */}
          <div>
            <label className="block text-sm font-medium text-gray-700 mb-2">
              Slippage Tolerance (%)
            </label>
            <input
              type="number"
              value={slippage}
              onChange={(e) => setSlippage(e.target.value)}
              className="w-full border border-gray-300 rounded-xl p-3 text-gray-900 bg-white focus:ring-2 focus:ring-blue-500 focus:border-transparent"
              placeholder="0.5"
              step="0.1"
            />
          </div>

          {/* Swap Button */}
          <button
            onClick={executeSwap}
            disabled={!selectedPool || !amountIn || loading}
            className="w-full bg-gradient-to-r from-blue-500 to-cyan-500 text-white py-4 px-6 rounded-xl font-semibold hover:from-blue-600 hover:to-cyan-600 transition disabled:opacity-50 disabled:cursor-not-allowed shadow-lg"
          >
            {loading ? 'Processing...' : 'Execute Swap'}
          </button>

          {/* Status Message */}
          {txStatus && (
            <div className="p-4 bg-blue-50 border border-blue-200 rounded-xl">
              <p className="text-sm text-blue-800 break-all">{txStatus}</p>
            </div>
          )}
        </div>
      )}

      {/* Info */}
      <div className="mt-6 p-4 bg-gray-50 rounded-xl">
        <p className="text-xs text-gray-600">
          <span className="font-semibold">How it works:</span> Select a pool, enter amount, and click swap.
          The backend builds the transaction, you sign it in your wallet, and it&apos;s submitted to the blockchain.
        </p>
        <p className="text-xs text-gray-500 mt-2">
          🔒 Your keys never leave your browser • Powered by Cetus DEX
        </p>
      </div>
    </div>
  );
}
