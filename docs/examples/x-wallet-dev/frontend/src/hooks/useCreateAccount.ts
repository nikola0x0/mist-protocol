/**
 * Hook for creating an XWallet account
 *
 * Flow:
 * 1. User clicks "Create Account" button
 * 2. Frontend sends XID to backend
 * 3. Backend calls enclave to sign init account payload
 * 4. Backend submits transaction on-chain
 * 5. Account is created
 */

import { useState, useCallback } from 'react';
import { useAuth } from '../contexts/AuthContext';
import { API_BASE_URL } from '../utils/constants';

interface CreateAccountResponse {
  success: boolean;
  tx_digest?: string;
  error?: string;
}

export interface UseCreateAccountReturn {
  createAccount: () => Promise<CreateAccountResponse>;
  isCreating: boolean;
  error: string | null;
}

export function useCreateAccount(): UseCreateAccountReturn {
  const [isCreating, setIsCreating] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const { user } = useAuth();

  const createAccount = useCallback(async (): Promise<CreateAccountResponse> => {
    if (!user?.twitterUserId) {
      throw new Error('User not authenticated');
    }

    setIsCreating(true);
    setError(null);

    try {
      const response = await fetch(`${API_BASE_URL}/api/account/create`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({
          xid: user.twitterUserId,
        }),
      });

      if (!response.ok) {
        const errorData = await response.json().catch(() => ({}));
        throw new Error(errorData.error || 'Failed to create account');
      }

      const result: CreateAccountResponse = await response.json();

      if (!result.success) {
        throw new Error(result.error || 'Failed to create account');
      }

      setIsCreating(false);
      return result;
    } catch (err) {
      const message = err instanceof Error ? err.message : 'Failed to create account';
      setError(message);
      setIsCreating(false);
      throw err;
    }
  }, [user]);

  return {
    createAccount,
    isCreating,
    error,
  };
}
