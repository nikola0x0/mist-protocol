import React, { useState, useEffect } from 'react';
import { useDocumentTitle } from '../hooks/useDocumentTitle';
import { useAuth } from '../contexts/AuthContext';
import { ConnectButton, useCurrentAccount, useSignPersonalMessage } from '@mysten/dapp-kit';

export const Onboarding: React.FC = () => {
  useDocumentTitle('Login');
  const { loginWithWallet, isLoading } = useAuth();
  const currentAccount = useCurrentAccount();
  const { mutateAsync: signPersonalMessage } = useSignPersonalMessage();
  const [error, setError] = useState<string>('');
  const [isWaitingForSignature, setIsWaitingForSignature] = useState(false);

  // Request signature when wallet is connected
  useEffect(() => {
    if (currentAccount?.address && !isLoading && !isWaitingForSignature) {
      handleWalletLogin();
    }
  }, [currentAccount?.address]);

  const handleWalletLogin = async () => {
    if (!currentAccount?.address) return;

    try {
      setError('');
      setIsWaitingForSignature(true);

      // Create a message to sign
      const message = `Sign this message to login to X-Wallet\n\nAddress: ${currentAccount.address}\nTimestamp: ${Date.now()}`;

      // Request signature from wallet
      const { signature } = await signPersonalMessage({
        message: new TextEncoder().encode(message),
      });

      // If signature successful, login
      if (signature) {
        await loginWithWallet(currentAccount.address);
      }
    } catch {
      setError('Failed to sign message. Please try again.');
      setIsWaitingForSignature(false);
    }
  };

  return (
    <div className="min-h-screen flex items-center justify-center bg-gray-50 dark:bg-gray-900">
      <div className="bg-white dark:bg-gray-800 p-8 rounded-lg shadow-xl max-w-md w-full">
        <div className="text-center mb-8">
          <h1 className="text-3xl font-bold text-gray-900 dark:text-white mb-2">
            X-Wallet
          </h1>
          <p className="text-gray-600 dark:text-gray-300">
            X-enabled Sui Wallet
          </p>
        </div>

        {error && (
          <div className="mb-4 p-3 bg-red-100 dark:bg-red-900 text-red-700 dark:text-red-200 rounded-lg text-sm">
            {error}
          </div>
        )}

        <div className="space-y-4">
          {/* Sui Wallet Connect Button */}
          <div className="w-full">
            {!currentAccount?.address ? (
              <ConnectButton
                className="w-full"
                connectText="Connect Sui Wallet"
              />
            ) : (
              <div className="text-center py-3">
                {isWaitingForSignature && (
                  <p className="text-sm text-blue-600 dark:text-blue-400">
                    Please sign the message in your wallet...
                  </p>
                )}
                {isLoading && (
                  <p className="text-sm text-gray-500 dark:text-gray-400">
                    Logging in...
                  </p>
                )}
              </div>
            )}
          </div>
        </div>

        <p className="text-xs text-gray-500 dark:text-gray-400 text-center mt-6">
          By connecting, you agree to our Terms of Service and Privacy Policy
        </p>
      </div>
    </div>
  );
};
