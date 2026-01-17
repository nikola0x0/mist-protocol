"use client";

import { useState, useEffect } from "react";
import { SuiClient } from "@mysten/sui/client";
import { useCurrentAccount } from "@mysten/dapp-kit";
import Image from "next/image";

export function BalanceCard() {
  const currentAccount = useCurrentAccount();
  const [poolBalances, setPoolBalances] = useState<{ SUI: number; USDC: number }>({
    SUI: 0,
    USDC: 0,
  });
  const [loading, setLoading] = useState(true);

  const client = new SuiClient({
    url: process.env.NEXT_PUBLIC_NETWORK === "mainnet"
      ? "https://fullnode.mainnet.sui.io"
      : "https://fullnode.testnet.sui.io"
  });

  // User wallet balances will be loaded from actual Sui network
  const [userBalances, setUserBalances] = useState<{ SUI: number; USDC: number }>({
    SUI: 0,
    USDC: 0,
  });

  // Load pool liquidity from smart contract using robust object loading
  const loadPoolBalances = async () => {
    try {
      const poolId = process.env.NEXT_PUBLIC_POOL_ID;
      if (!poolId) {
        console.log("Pool ID not configured");
        return;
      }

      console.log(`💰 Loading pool balances from: ${poolId.substring(0, 20)}...`);

      const poolObject = await client.getObject({
        id: poolId,
        options: {
          showContent: true,
          showOwner: true,
        },
      });

      if (!poolObject.data?.content) {
        console.log("❌ Pool not found");
        return;
      }

      const content = poolObject.data.content as any;

      // Handle different field structures (like we do with tickets)
      let suiBalance = 0;
      let usdcBalance = 0;

      // Try direct fields first
      if (content.fields?.sui_balance?.fields?.value) {
        suiBalance = content.fields.sui_balance.fields.value;
      } else if (content.fields?.sui_balance) {
        suiBalance = content.fields.sui_balance;
      }

      if (content.fields?.usdc_balance?.fields?.value) {
        usdcBalance = content.fields.usdc_balance.fields.value;
      } else if (content.fields?.usdc_balance) {
        usdcBalance = content.fields.usdc_balance;
      }

      console.log(`📊 Raw balances - SUI: ${suiBalance}, USDC: ${usdcBalance}`);

      // Convert to human-readable format
      const humanSui = suiBalance / 1e9; // Convert from MIST to SUI
      const humanUsdc = usdcBalance / 1e6; // Convert from smallest USDC unit

      setPoolBalances({
        SUI: humanSui,
        USDC: humanUsdc,
      });

      console.log(`✅ Pool balances loaded - SUI: ${humanSui.toFixed(3)}, USDC: ${humanUsdc.toFixed(2)}`);
    } catch (error) {
      console.error("❌ Failed to load pool balances:", error);
      // Set to zero if pool cannot be loaded - no mock data
      setPoolBalances({
        SUI: 0,
        USDC: 0,
      });
    } finally {
      setLoading(false);
    }
  };

  // Load user wallet balances from Sui network
  const loadUserBalances = async () => {
    if (!currentAccount) return;

    try {
      console.log(`👛 Loading user balances for: ${currentAccount.address.substring(0, 20)}...`);

      // Get all coins owned by the user
      const suiCoins = await client.getCoins({
        owner: currentAccount.address,
        coinType: "0x2::sui::SUI"
      });

      const usdcCoins = await client.getCoins({
        owner: currentAccount.address,
        coinType: process.env.NEXT_PUBLIC_USDC_COIN_TYPE || "0x5d4b3025b5f0c4f6b3b1f9c9b2c2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b::usdc::USDC"
      });

      const totalSui = suiCoins.data.reduce((sum, coin) => sum + parseFloat(coin.balance), 0) / 1e9;
      const totalUsdc = usdcCoins.data.reduce((sum, coin) => sum + parseFloat(coin.balance), 0) / 1e6;

      setUserBalances({
        SUI: totalSui,
        USDC: totalUsdc,
      });

      console.log(`✅ User balances loaded - SUI: ${totalSui.toFixed(3)}, USDC: ${totalUsdc.toFixed(2)}`);
    } catch (error) {
      console.error("❌ Failed to load user balances:", error);
      setUserBalances({
        SUI: 0,
        USDC: 0,
      });
    }
  };

  useEffect(() => {
    loadPoolBalances();
    loadUserBalances();
  }, [currentAccount]);

  return (
    <div className="card p-6">
      <h3 className="text-lg font-bold mb-4">Protocol & Pool Status</h3>

      {/* Pool Liquidity (Available for Unwrap) */}
      <div className="mb-6">
        <div className="flex items-center justify-between mb-3">
          <div className="text-sm font-medium text-gray-300">Pool Liquidity</div>
          {loading ? (
            <div className="text-xs text-gray-500">Loading...</div>
          ) : (
            <div className="text-xs text-green-500">Available for unwrap</div>
          )}
        </div>

        <div className="space-y-3">
          <div className="flex items-center justify-between p-3 bg-green-950/10 rounded-lg border border-green-900/20">
            <div className="flex items-center gap-3">
              <div className="w-8 h-8 rounded-full overflow-hidden flex items-center justify-center bg-blue-500/20">
                <Image
                  src="/assets/token-icons/sui.png"
                  alt="SUI"
                  width={32}
                  height={32}
                />
              </div>
              <div>
                <div className="font-medium">SUI Pool</div>
                <div className="text-xs text-gray-500">Available to unwrap</div>
              </div>
            </div>
            <div className="text-right">
              <div className="font-bold text-green-400">
                {loading ? "..." : poolBalances.SUI.toLocaleString()}
              </div>
              <div className="text-xs text-green-600">
                {!loading && `≈ $${(poolBalances.SUI * 2.5).toLocaleString()}`}
              </div>
            </div>
          </div>

          <div className="flex items-center justify-between p-3 bg-green-950/10 rounded-lg border border-green-900/20">
            <div className="flex items-center gap-3">
              <div className="w-8 h-8 bg-green-500 rounded-full flex items-center justify-center text-sm font-bold">
                U
              </div>
              <div>
                <div className="font-medium">USDC Pool</div>
                <div className="text-xs text-gray-500">Available to unwrap</div>
              </div>
            </div>
            <div className="text-right">
              <div className="font-bold text-green-400">
                ${!loading ? poolBalances.USDC.toLocaleString() : "..."}
              </div>
              <div className="text-xs text-green-600">Stable value</div>
            </div>
          </div>
        </div>
      </div>

      {/* User Wallet Balances */}
      <div className="mb-6">
        <div className="text-sm font-medium text-gray-300 mb-3">Your Wallet Balances</div>
        <div className="space-y-3">
          <div className="flex items-center justify-between p-3 bg-[#0a0a0a] rounded-lg border border-[#262626]">
            <div className="flex items-center gap-3">
              <div className="w-8 h-8 rounded-full overflow-hidden flex items-center justify-center bg-blue-500/20">
                <Image
                  src="/assets/token-icons/sui.png"
                  alt="SUI"
                  width={32}
                  height={32}
                />
              </div>
              <div>
                <div className="font-medium">SUI</div>
                <div className="text-xs text-gray-500">In wallet</div>
              </div>
            </div>
            <div className="text-right">
              <div className="font-bold">{userBalances.SUI.toFixed(3)}</div>
              <div className="text-xs text-gray-500">{userBalances.SUI > 0 ? `≈ $${(userBalances.SUI * 2.5).toFixed(0)}` : 'No balance'}</div>
            </div>
          </div>

          <div className="flex items-center justify-between p-3 bg-[#0a0a0a] rounded-lg border border-[#262626]">
            <div className="flex items-center gap-3">
              <div className="w-8 h-8 bg-green-500 rounded-full flex items-center justify-center text-sm font-bold">
                U
              </div>
              <div>
                <div className="font-medium">USDC</div>
                <div className="text-xs text-gray-500">In wallet</div>
              </div>
            </div>
            <div className="text-right">
              <div className="font-bold">{userBalances.USDC.toFixed(2)}</div>
              <div className="text-xs text-gray-500">{userBalances.USDC > 0 ? `$${userBalances.USDC.toFixed(0)}` : 'No balance'}</div>
            </div>
          </div>
        </div>
      </div>

      {/* Important Note for Unwrap */}
      <div className="p-3 bg-blue-950/10 rounded-lg border border-blue-900/20">
        <div className="flex items-center gap-2 mb-2">
          <div className="w-4 h-4 bg-blue-500 rounded-full flex items-center justify-center text-xs">
            !
          </div>
          <div className="text-sm font-medium text-blue-400">Unwrap Information</div>
        </div>
        <div className="text-xs text-blue-300 space-y-1">
          <p>• You can unwrap your tickets for real tokens from the pool</p>
          <p>• Pool liquidity shows available tokens for immediate unwrap</p>
          <p>• Large unwraps may be limited by pool reserves</p>
        </div>
      </div>
    </div>
  );
}
