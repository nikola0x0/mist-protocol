/**
 * X (Twitter) OAuth 2.0 Authentication Hook
 *
 * Handles the complete OAuth flow:
 * 1. Initiate login (redirect to X)
 * 2. Handle callback (exchange code for token via backend)
 * 3. Get user info and XWallet account
 */

import { useState, useCallback } from 'react';
import {
  generateCodeVerifier,
  generateCodeChallenge,
  generateState,
  storePKCE,
  retrievePKCE,
  clearPKCE,
} from '../utils/pkce';
import { API_BASE_URL } from '../utils/constants';

// OAuth configuration
const TWITTER_AUTH_URL = 'https://twitter.com/i/oauth2/authorize';
const CLIENT_ID = import.meta.env.VITE_TWITTER_CLIENT_ID;
const REDIRECT_URI = import.meta.env.VITE_TWITTER_REDIRECT_URI;
const SCOPES = ['tweet.read', 'users.read', 'offline.access'];

export interface XUser {
  id: string;
  username: string;
  name: string;
  profile_image_url?: string;
}

export interface XAuthResult {
  user: XUser;
  accessToken: string;
  refreshToken?: string | null;
  xwalletAccount?: {
    sui_object_id: string;
    x_user_id: string;
    x_handle: string;
    owner_address?: string;
  };
}

export interface UseXAuthReturn {
  isLoading: boolean;
  error: string | null;
  initiateLogin: () => Promise<void>;
  handleCallback: (code: string, state: string) => Promise<XAuthResult>;
  logout: () => void;
}

export function useXAuth(): UseXAuthReturn {
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  /**
   * Step 1: Initiate OAuth flow - redirect user to X login
   */
  const initiateLogin = useCallback(async () => {
    setIsLoading(true);
    setError(null);

    try {
      // Generate PKCE values
      const codeVerifier = generateCodeVerifier();
      const codeChallenge = await generateCodeChallenge(codeVerifier);
      const state = generateState();

      // Store for later verification
      storePKCE(codeVerifier, state);

      // Build authorization URL
      const params = new URLSearchParams({
        response_type: 'code',
        client_id: CLIENT_ID,
        redirect_uri: REDIRECT_URI,
        scope: SCOPES.join(' '),
        state: state,
        code_challenge: codeChallenge,
        code_challenge_method: 'S256',
      });

      // Redirect to X
      window.location.href = `${TWITTER_AUTH_URL}?${params.toString()}`;
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to initiate login');
      setIsLoading(false);
    }
  }, []);

  /**
   * Step 2: Handle callback - exchange code for token via backend
   */
  const handleCallback = useCallback(async (code: string, returnedState: string): Promise<XAuthResult> => {
    setIsLoading(true);
    setError(null);

    try {
      // Retrieve stored PKCE values
      const { codeVerifier, state } = retrievePKCE();

      // Verify state to prevent CSRF
      if (!state || state !== returnedState) {
        throw new Error('Invalid state parameter - possible CSRF attack');
      }

      if (!codeVerifier) {
        throw new Error('Missing code verifier - please try logging in again');
      }

      // Exchange code for token via backend
      const response = await fetch(`${API_BASE_URL}/api/auth/twitter/token`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({
          code,
          code_verifier: codeVerifier,
          redirect_uri: REDIRECT_URI,
        }),
      });

      if (!response.ok) {
        const errorData = await response.json().catch(() => ({}));
        throw new Error(errorData.error || `Token exchange failed: ${response.status}`);
      }

      const result: XAuthResult = await response.json();

      // Clear PKCE values
      clearPKCE();

      setIsLoading(false);
      return result;
    } catch (err) {
      clearPKCE();
      const message = err instanceof Error ? err.message : 'Authentication failed';
      setError(message);
      setIsLoading(false);
      throw err;
    }
  }, []);

  /**
   * Logout - clear all auth state
   */
  const logout = useCallback(() => {
    clearPKCE();
    // Additional logout logic can be added here
    // e.g., clear tokens from localStorage, call backend logout endpoint
  }, []);

  return {
    isLoading,
    error,
    initiateLogin,
    handleCallback,
    logout,
  };
}
