import React, { createContext, useContext, useState, useCallback, useEffect, useRef } from 'react';
import type { ReactNode } from 'react';
import { API_BASE_URL } from '../utils/constants';

// Storage keys
const STORAGE_KEYS = {
  USER: 'xwallet_user',
  ACCESS_TOKEN: 'xwallet_access_token',
  REFRESH_TOKEN: 'xwallet_refresh_token',
} as const;

interface User {
  twitterHandle: string;
  twitterUserId: string;
  avatarUrl: string | null;
  suiObjectId: string | null;
  linkedWalletAddress: string | null;
}

interface LoginData {
  twitterUserId: string;
  twitterHandle: string;
  avatarUrl: string | null;
  suiObjectId: string | null;
  linkedWalletAddress: string | null;
}

interface Tokens {
  accessToken: string;
  refreshToken?: string | null;
}

interface AuthContextType {
  user: User | null;
  isAuthenticated: boolean;
  isLoading: boolean;
  accessToken: string | null;
  sessionExpired: boolean;
  login: (data: LoginData, tokens?: Tokens) => void;
  loginWithWallet: (address: string) => Promise<void>;
  logout: () => void;
  linkWallet: (address: string) => Promise<void>;
  refreshAccount: () => Promise<void>;
  refreshAccessToken: () => Promise<string | null>;
  confirmSessionExpired: () => void;
}

const AuthContext = createContext<AuthContextType | undefined>(undefined);

export const AuthProvider: React.FC<{ children: ReactNode }> = ({ children }) => {
  const [user, setUser] = useState<User | null>(null);
  const [accessToken, setAccessToken] = useState<string | null>(null);
  // refreshToken state kept in sync with localStorage (used for token rotation)
  const [_refreshToken, setRefreshToken] = useState<string | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [sessionExpired, setSessionExpired] = useState(false);

  // Prevent concurrent refresh attempts
  const isRefreshing = useRef(false);

  // Load user from localStorage on mount
  useEffect(() => {
    try {
      const storedUser = localStorage.getItem(STORAGE_KEYS.USER);
      const storedAccessToken = localStorage.getItem(STORAGE_KEYS.ACCESS_TOKEN);
      const storedRefreshToken = localStorage.getItem(STORAGE_KEYS.REFRESH_TOKEN);

      if (storedUser) {
        setUser(JSON.parse(storedUser));
      }
      if (storedAccessToken) {
        setAccessToken(storedAccessToken);
      }
      if (storedRefreshToken) {
        setRefreshToken(storedRefreshToken);
      }
    } catch {
      localStorage.removeItem(STORAGE_KEYS.USER);
      localStorage.removeItem(STORAGE_KEYS.ACCESS_TOKEN);
      localStorage.removeItem(STORAGE_KEYS.REFRESH_TOKEN);
    } finally {
      setIsLoading(false);
    }
  }, []);

  // Logout - clear all auth state
  const logout = useCallback(() => {
    setUser(null);
    setAccessToken(null);
    setRefreshToken(null);
    setSessionExpired(false);
    localStorage.removeItem(STORAGE_KEYS.USER);
    localStorage.removeItem(STORAGE_KEYS.ACCESS_TOKEN);
    localStorage.removeItem(STORAGE_KEYS.REFRESH_TOKEN);
  }, []);

  // Confirm session expired - called when user clicks OK on modal
  const confirmSessionExpired = useCallback(() => {
    logout();
  }, [logout]);

  // Refresh access token using refresh token
  // Returns the new access token on success, null on failure
  const refreshAccessToken = useCallback(async (): Promise<string | null> => {
    const storedRefreshToken = localStorage.getItem(STORAGE_KEYS.REFRESH_TOKEN);

    if (!storedRefreshToken) {
      return null;
    }

    // Prevent concurrent refresh attempts
    if (isRefreshing.current) {
      return null;
    }

    isRefreshing.current = true;

    try {
      const response = await fetch(`${API_BASE_URL}/api/auth/refresh`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({ refreshToken: storedRefreshToken }),
      });

      if (!response.ok) {
        // Refresh token is invalid/expired - show session expired modal
        setSessionExpired(true);
        return null;
      }

      const data = await response.json();

      // Update access token
      setAccessToken(data.accessToken);
      localStorage.setItem(STORAGE_KEYS.ACCESS_TOKEN, data.accessToken);

      // Update refresh token if a new one was provided (token rotation)
      if (data.refreshToken) {
        setRefreshToken(data.refreshToken);
        localStorage.setItem(STORAGE_KEYS.REFRESH_TOKEN, data.refreshToken);
      }

      return data.accessToken;
    } catch {
      return null;
    } finally {
      isRefreshing.current = false;
    }
  }, []);

  // Login with X OAuth data
  const login = useCallback((data: LoginData, tokens?: Tokens) => {
    const newUser: User = {
      twitterHandle: data.twitterHandle,
      twitterUserId: data.twitterUserId,
      avatarUrl: data.avatarUrl,
      suiObjectId: data.suiObjectId,
      linkedWalletAddress: data.linkedWalletAddress,
    };

    setUser(newUser);
    localStorage.setItem(STORAGE_KEYS.USER, JSON.stringify(newUser));

    if (tokens?.accessToken) {
      setAccessToken(tokens.accessToken);
      localStorage.setItem(STORAGE_KEYS.ACCESS_TOKEN, tokens.accessToken);
    }

    if (tokens?.refreshToken) {
      setRefreshToken(tokens.refreshToken);
      localStorage.setItem(STORAGE_KEYS.REFRESH_TOKEN, tokens.refreshToken);
    }
  }, []);

  const loginWithWallet = useCallback(async (address: string) => {
    setIsLoading(true);
    try {
      const newUser: User = {
        twitterHandle: '',
        twitterUserId: '',
        avatarUrl: null,
        suiObjectId: null,
        linkedWalletAddress: address,
      };
      setUser(newUser);
      localStorage.setItem(STORAGE_KEYS.USER, JSON.stringify(newUser));
    } catch (error) {
      throw error;
    } finally {
      setIsLoading(false);
    }
  }, []);

  // Update local state after wallet is linked
  const linkWallet = useCallback(async (address: string) => {
    if (!user) {
      throw new Error('User not authenticated');
    }

    const updatedUser: User = {
      ...user,
      linkedWalletAddress: address,
    };
    setUser(updatedUser);
    localStorage.setItem(STORAGE_KEYS.USER, JSON.stringify(updatedUser));
  }, [user]);

  // Refresh account data from backend
  // On 401, tries to refresh token before showing session expired modal
  const refreshAccount = useCallback(async () => {
    const storedToken = localStorage.getItem(STORAGE_KEYS.ACCESS_TOKEN);
    if (!storedToken) {
      return;
    }

    try {
      const response = await fetch(`${API_BASE_URL}/api/my-account`, {
        headers: {
          'Authorization': `Bearer ${storedToken}`,
        },
      });

      if (response.ok) {
        const data = await response.json();
        const account = data.account || data;

        const updatedUser: User = {
          twitterUserId: account.x_user_id,
          twitterHandle: account.x_handle,
          avatarUrl: account.avatar_url || user?.avatarUrl || null,
          suiObjectId: account.sui_object_id || null,
          linkedWalletAddress: account.owner_address || null,
        };
        setUser(updatedUser);
        localStorage.setItem(STORAGE_KEYS.USER, JSON.stringify(updatedUser));
      } else if (response.status === 401) {
        // Token expired - try to refresh
        const newToken = await refreshAccessToken();

        if (newToken) {
          // Retry the request with new token
          const retryResponse = await fetch(`${API_BASE_URL}/api/my-account`, {
            headers: {
              'Authorization': `Bearer ${newToken}`,
            },
          });

          if (retryResponse.ok) {
            const data = await retryResponse.json();
            const account = data.account || data;

            const updatedUser: User = {
              twitterUserId: account.x_user_id,
              twitterHandle: account.x_handle,
              avatarUrl: account.avatar_url || user?.avatarUrl || null,
              suiObjectId: account.sui_object_id || null,
              linkedWalletAddress: account.owner_address || null,
            };
            setUser(updatedUser);
            localStorage.setItem(STORAGE_KEYS.USER, JSON.stringify(updatedUser));
          }
        }
        // If refresh failed, sessionExpired modal will be shown by refreshAccessToken
      }
    } catch {
      // Silently fail - user can retry manually
    }
  }, [user, refreshAccessToken]);

  return (
    <AuthContext.Provider
      value={{
        user,
        isAuthenticated: !!user,
        isLoading,
        accessToken,
        sessionExpired,
        login,
        loginWithWallet,
        logout,
        linkWallet,
        refreshAccount,
        refreshAccessToken,
        confirmSessionExpired,
      }}
    >
      {children}
    </AuthContext.Provider>
  );
};

export const useAuth = () => {
  const context = useContext(AuthContext);
  if (context === undefined) {
    throw new Error('useAuth must be used within an AuthProvider');
  }
  return context;
};
