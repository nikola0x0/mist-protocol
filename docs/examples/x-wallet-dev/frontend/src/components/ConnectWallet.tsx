import React, { useState } from 'react';
import { useWallets, useConnectWallet } from '@mysten/dapp-kit';
import { Wallet } from 'lucide-react';

interface ConnectWalletProps {
  className?: string;
  buttonText?: string;
}

export const ConnectWallet: React.FC<ConnectWalletProps> = ({
  className = 'btn-sui flex items-center justify-center gap-2 min-w-[150px]',
  buttonText = 'Connect Wallet',
}) => {
  const [showDropdown, setShowDropdown] = useState(false);
  const wallets = useWallets();
  const { mutate: connectWallet } = useConnectWallet();

  return (
    <div className="relative">
      <button
        onClick={() => setShowDropdown(!showDropdown)}
        className={className}
      >
        <Wallet className="w-4 h-4" />
        {buttonText}
      </button>
      {showDropdown && (
        <>
          <div
            className="fixed inset-0 z-40"
            onClick={() => setShowDropdown(false)}
          />
          <div className="absolute right-0 top-full mt-2 w-64 glass rounded-xl p-2 z-50">
            {wallets.length === 0 ? (
              <p className="text-gray-400 text-center py-4 text-sm">No wallets found</p>
            ) : (
              wallets.map((wallet) => (
                <button
                  key={wallet.name}
                  onClick={() => {
                    connectWallet({ wallet });
                    setShowDropdown(false);
                  }}
                  className="w-full flex items-center gap-3 p-3 rounded-lg hover:bg-white/10 text-left"
                >
                  {wallet.icon && (
                    <img src={wallet.icon} alt={wallet.name} className="w-6 h-6" />
                  )}
                  <span className="text-white">{wallet.name}</span>
                </button>
              ))
            )}
          </div>
        </>
      )}
    </div>
  );
};
