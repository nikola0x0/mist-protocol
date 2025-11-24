import { useState } from 'react';
import {
  ConnectButton,
  useCurrentAccount,
  useSignAndExecuteTransactionBlock,
  useSuiClient,
} from '@mysten/dapp-kit';
import { TransactionBlock } from '@mysten/sui.js/transactions';

const BACKEND_URL = 'http://localhost:3000';

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

export default function CetusSwap() {
  const account = useCurrentAccount();
  const { mutate: signAndExecute } = useSignAndExecuteTransactionBlock();
  const suiClient = useSuiClient();

  const [pools, setPools] = useState<Pool[]>([]);
  const [selectedPool, setSelectedPool] = useState<Pool | null>(null);
  const [amountIn, setAmountIn] = useState<string>('');
  const [slippage, setSlippage] = useState<string>('2');
  const [loading, setLoading] = useState(false);
  const [txStatus, setTxStatus] = useState<string>('');
  const [swapDirection, setSwapDirection] = useState<'a2b' | 'b2a'>('a2b'); // a2b = USDC->SUI, b2a = SUI->USDC

  // Initialize available pools
  // These pools are verified to work with the backend integration
  const fetchPools = async () => {
    setLoading(true);
    try {
      const availablePools = [
        {
          swap_account: '0x06d8af9e6afd27262db436f0d37b304a041f710c3ea1fa4c3a9bab36b3569ad3',
          symbol: 'USDT-SUI (Wormhole)',
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

      setPools(availablePools);
      setSelectedPool(availablePools[0]); // Default to USDT-SUI pool
    } catch (error) {
      console.error('Failed to fetch pools:', error);
    } finally {
      setLoading(false);
    }
  };

  // Build swap transaction via backend
  // The backend constructs the unsigned transaction which we then sign with the wallet
  const buildSwapTransaction = async (): Promise<SwapQuote | null> => {
    if (!account || !selectedPool) return null;

    try {
      // Determine token decimals for amount conversion
      // SUI and CETUS have 9 decimals, USDC/USDT have 6 decimals
      const getTokenDecimals = (tokenAddress: string) => {
        if (tokenAddress.includes('::sui::SUI')) return 1_000_000_000;    // 9 decimals
        if (tokenAddress.includes('::cetus::CETUS')) return 1_000_000_000; // 9 decimals
        return 1_000_000; // USDC/USDT have 6 decimals
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

      console.log('Sending build-swap request:', requestBody);

      const response = await fetch(`${BACKEND_URL}/api/build-swap`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(requestBody),
      });

      if (!response.ok) {
        const errorData = await response.json();
        console.error('Backend error:', errorData);
        throw new Error(errorData.error || 'Failed to build transaction');
      }

      return await response.json();
    } catch (error: any) {
      console.error('Failed to build transaction:', error);
      console.error('Error message:', error.message);
      return null;
    }
  };

  // Execute swap
  const executeSwap = async () => {
    if (!account) {
      setTxStatus('Please connect your wallet');
      return;
    }

    // Prevent double-clicking - if already processing, ignore the call
    if (loading) {
      console.log('Swap already in progress, ignoring duplicate call');
      return;
    }

    setLoading(true);
    setTxStatus('Building transaction...');

    try {
      // Get unsigned transaction from backend
      const quote = await buildSwapTransaction();
      
      if (!quote) {
        setTxStatus('Failed to build transaction');
        return;
      }

      setTxStatus(`Expected output: ${quote.pool_info.expected_output / 1_000_000} tokens`);
      setTxStatus('Waiting for wallet signature...');

      // Decode and deserialize the transaction
      const txBytes = Uint8Array.from(atob(quote.tx_bytes), c => c.charCodeAt(0));
      const txBlock = TransactionBlock.from(txBytes);

      // Sign and execute with user's wallet
      signAndExecute(
        {
          transactionBlock: txBlock,
        },
        {
          onSuccess: (result) => {
            console.log('Swap successful!', result);
            setTxStatus(`Swap successful! Digest: ${result.digest}`);
          },
          onError: (error) => {
            console.error('Swap failed:', error);
            setTxStatus(`Swap failed: ${error.message}`);
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
    <div className="min-h-screen bg-gradient-to-br from-blue-50 to-indigo-100 p-8">
      <div className="max-w-2xl mx-auto">
        <div className="bg-white rounded-2xl shadow-xl p-8">
          <h1 className="text-3xl font-bold text-gray-800 mb-6">
            🌊 Cetus Swap
          </h1>

          {/* Wallet Connection */}
          <div className="mb-6">
            <ConnectButton />
            {account && (
              <p className="mt-2 text-sm text-gray-600">
                Connected: {account.address.slice(0, 6)}...{account.address.slice(-4)}
              </p>
            )}
          </div>

          {/* Pool Selection */}
          <div className="mb-6">
            <label className="block text-sm font-medium text-gray-700 mb-2">
              Select Pool
            </label>
            {pools.length === 0 ? (
              <button
                onClick={fetchPools}
                className="w-full bg-blue-500 text-white py-2 px-4 rounded-lg hover:bg-blue-600 transition"
                disabled={loading}
              >
                Load Pools
              </button>
            ) : (
              <select
                className="w-full border border-gray-300 rounded-lg p-3 text-gray-900 bg-white"
                value={selectedPool?.swap_account || ''}
                onChange={(e) => {
                  const pool = pools.find(p => p.swap_account === e.target.value);
                  setSelectedPool(pool || null);
                }}
              >
                <option value="">Choose a pool</option>
                {pools.map((pool) => {
                  // Detect if this pool uses native USDC or wUSDC
                  const isNativeUSDC = pool.token_a_address.includes('dba34672') ||
                                      pool.token_b_address.includes('dba34672');
                  const usdcType = isNativeUSDC ? 'Native USDC' : 'wUSDC';

                  return (
                    <option key={pool.swap_account} value={pool.swap_account}>
                      {pool.symbol} ({usdcType}) - Fee: {(parseFloat(pool.fee) * 100).toFixed(2)}%
                    </option>
                  );
                })}
              </select>
            )}
          </div>

          {/* Swap Direction */}
          {selectedPool && (
            <div className="mb-6">
              <label className="block text-sm font-medium text-gray-700 mb-2">
                Swap Direction
              </label>
              <div className="flex gap-2">
                <button
                  onClick={() => setSwapDirection('a2b')}
                  className={`flex-1 py-2 px-4 rounded-lg font-medium transition ${
                    swapDirection === 'a2b'
                      ? 'bg-blue-600 text-white'
                      : 'bg-gray-200 text-gray-800'
                  }`}
                >
                  {selectedPool.symbol.split('-')[0]} → {selectedPool.symbol.split('-')[1]}
                </button>
                <button
                  onClick={() => setSwapDirection('b2a')}
                  className={`flex-1 py-2 px-4 rounded-lg font-medium transition ${
                    swapDirection === 'b2a'
                      ? 'bg-blue-600 text-white'
                      : 'bg-gray-200 text-gray-800'
                  }`}
                >
                  {selectedPool.symbol.split('-')[1]} → {selectedPool.symbol.split('-')[0]}
                </button>
              </div>
            </div>
          )}

          {/* Amount Input */}
          <div className="mb-6">
            <label className="block text-sm font-medium text-gray-700 mb-2">
              Amount to Swap {selectedPool && `(${swapDirection === 'a2b' ? 'USDC' : 'SUI'})`}
            </label>
            <input
              type="number"
              value={amountIn}
              onChange={(e) => setAmountIn(e.target.value)}
              className="w-full border border-gray-300 rounded-lg p-3 text-gray-900 bg-white"
              placeholder="0.0"
            />
          </div>

          {/* Slippage Input */}
          <div className="mb-6">
            <label className="block text-sm font-medium text-gray-700 mb-2">
              Slippage Tolerance (%)
            </label>
            <input
              type="number"
              value={slippage}
              onChange={(e) => setSlippage(e.target.value)}
              className="w-full border border-gray-300 rounded-lg p-3 text-gray-900 bg-white"
              placeholder="0.5"
              step="0.1"
            />
          </div>

          {/* Swap Button */}
          <button
            onClick={executeSwap}
            disabled={!account || !selectedPool || !amountIn || loading}
            className="w-full bg-gradient-to-r from-blue-500 to-indigo-600 text-white py-4 px-6 rounded-lg font-semibold hover:from-blue-600 hover:to-indigo-700 transition disabled:opacity-50 disabled:cursor-not-allowed"
          >
            {loading ? 'Processing...' : 'Swap'}
          </button>

          {/* Status Message */}
          {txStatus && (
            <div className="mt-4 p-4 bg-blue-50 border border-blue-200 rounded-lg">
              <p className="text-sm text-blue-800">{txStatus}</p>
            </div>
          )}
        </div>

        {/* Info Card */}
        <div className="mt-6 bg-white rounded-xl shadow p-6">
          <h3 className="font-semibold text-gray-800 mb-3">How it works:</h3>
          <ol className="space-y-2 text-sm text-gray-600">
            <li>1. Connect your Sui wallet (Sui Wallet, Suiet, etc.)</li>
            <li>2. Select a liquidity pool</li>
            <li>3. Enter the amount you want to swap</li>
            <li>4. Backend builds the transaction</li>
            <li>5. Sign the transaction in your wallet</li>
            <li>6. Transaction is submitted to the blockchain</li>
          </ol>
          <p className="mt-4 text-xs text-gray-500">
            🔒 Your private keys never leave your browser
          </p>
        </div>
      </div>
    </div>
  );
}
