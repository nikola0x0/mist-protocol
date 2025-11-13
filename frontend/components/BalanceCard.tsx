"use client";

export function BalanceCard() {
  const balances = {
    SUI: 100.5,
    USDC: 250.0,
    eSUI: 50.25,
    eUSDC: 125.75,
  };

  return (
    <div className="card p-6">
      <h3 className="text-lg font-bold mb-4">Your Balances</h3>

      {/* Real Tokens */}
      <div className="space-y-3 mb-6">
        <div className="flex items-center justify-between p-3 bg-[#0a0a0a] rounded-lg border border-[#262626]">
          <div className="flex items-center gap-3">
            <div className="w-8 h-8 bg-blue-500 rounded-full flex items-center justify-center text-sm font-bold">
              S
            </div>
            <div>
              <div className="font-medium">SUI</div>
              <div className="text-xs text-gray-500">Sui</div>
            </div>
          </div>
          <div className="text-right">
            <div className="font-bold">{balances.SUI.toFixed(2)}</div>
            <div className="text-xs text-gray-500">≈ $250</div>
          </div>
        </div>

        <div className="flex items-center justify-between p-3 bg-[#0a0a0a] rounded-lg border border-[#262626]">
          <div className="flex items-center gap-3">
            <div className="w-8 h-8 bg-green-500 rounded-full flex items-center justify-center text-sm font-bold">
              U
            </div>
            <div>
              <div className="font-medium">USDC</div>
              <div className="text-xs text-gray-500">USD Coin</div>
            </div>
          </div>
          <div className="text-right">
            <div className="font-bold">{balances.USDC.toFixed(2)}</div>
            <div className="text-xs text-gray-500">≈ $250</div>
          </div>
        </div>
      </div>

      {/* Encrypted Tokens */}
      <div className="space-y-3">
        <div className="text-xs text-gray-500 mb-2">Encrypted Balances</div>

        <div className="flex items-center justify-between p-3 bg-blue-950/10 rounded-lg border border-blue-900/20">
          <div className="flex items-center gap-3">
            <div className="w-8 h-8 bg-blue-600 rounded-full flex items-center justify-center text-sm">
              🔒
            </div>
            <div>
              <div className="font-medium">eSUI</div>
              <div className="text-xs text-gray-500">Encrypted SUI</div>
            </div>
          </div>
          <div className="text-right">
            <div className="font-bold">{balances.eSUI.toFixed(2)}</div>
            <div className="text-xs text-blue-500">Hidden</div>
          </div>
        </div>

        <div className="flex items-center justify-between p-3 bg-green-950/10 rounded-lg border border-green-900/20">
          <div className="flex items-center gap-3">
            <div className="w-8 h-8 bg-green-600 rounded-full flex items-center justify-center text-sm">
              🔒
            </div>
            <div>
              <div className="font-medium">eUSDC</div>
              <div className="text-xs text-gray-500">Encrypted USDC</div>
            </div>
          </div>
          <div className="text-right">
            <div className="font-bold">{balances.eUSDC.toFixed(2)}</div>
            <div className="text-xs text-green-500">Hidden</div>
          </div>
        </div>
      </div>
    </div>
  );
}
