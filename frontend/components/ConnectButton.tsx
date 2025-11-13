"use client";

import { ConnectButton as DappKitConnectButton, useCurrentAccount } from "@mysten/dapp-kit";

export function ConnectButton() {
  const account = useCurrentAccount();

  return (
    <div className="flex items-center gap-4">
      {account && (
        <div className="text-sm text-gray-600 dark:text-gray-400">
          {account.address.slice(0, 6)}...{account.address.slice(-4)}
        </div>
      )}
      <DappKitConnectButton />
    </div>
  );
}
