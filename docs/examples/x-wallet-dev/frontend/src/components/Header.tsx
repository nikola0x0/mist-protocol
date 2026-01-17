import React, { useState, useEffect, useRef } from 'react';
import { useNavigate, useLocation } from 'react-router-dom';
import { useAuth } from '../contexts/AuthContext';
import { useXAuth } from '../hooks/useXAuth';
import { useTheme } from '../contexts/ThemeContext';
import { useClipboard } from '../hooks/useClipboard';
import { shortenAddress } from '../utils/format';
import {
  useCurrentAccount,
  useDisconnectWallet,
  useConnectWallet,
  useWallets,
} from '@mysten/dapp-kit';
import { Wallet, Copy, Check, ChevronDown, LogOut, Sun, Moon, User, Link2 } from 'lucide-react';

// X icon component
const XIcon: React.FC<{ className?: string }> = ({ className }) => (
  <svg viewBox="0 0 24 24" className={className} fill="currentColor">
    <path d="M18.244 2.25h3.308l-7.227 8.26 8.502 11.24H16.17l-5.214-6.817L4.99 21.75H1.68l7.73-8.835L1.254 2.25H8.08l4.713 6.231zm-1.161 17.52h1.833L7.084 4.126H5.117z" />
  </svg>
);

// Profile Dropdown Component (for authenticated users)
const ProfileDropdown: React.FC = () => {
  const navigate = useNavigate();
  const location = useLocation();
  const { logout, user } = useAuth();
  const { toggleTheme, isDark } = useTheme();
  const currentAccount = useCurrentAccount();
  const { mutate: disconnect } = useDisconnectWallet();
  const { mutate: connect } = useConnectWallet();
  const wallets = useWallets();
  const [showDropdown, setShowDropdown] = useState(false);
  const [showWalletList, setShowWalletList] = useState(false);
  const { copied, copy } = useClipboard();
  const dropdownRef = useRef<HTMLDivElement>(null);

  const isProfilePage = location.pathname === '/profile';

  // Close dropdown when clicking outside
  useEffect(() => {
    if (!showDropdown) return;

    const handleClickOutside = (e: MouseEvent) => {
      if (dropdownRef.current && !dropdownRef.current.contains(e.target as Node)) {
        setShowDropdown(false);
        setShowWalletList(false);
      }
    };

    document.addEventListener('mousedown', handleClickOutside);
    return () => document.removeEventListener('mousedown', handleClickOutside);
  }, [showDropdown]);

  return (
    <div className="relative" ref={dropdownRef}>
      <button
        onClick={() => setShowDropdown(!showDropdown)}
        className="glass flex items-center gap-2 px-3 py-2 rounded-xl hover:bg-black/5 dark:hover:bg-white/10 transition-colors"
      >
        <img
          src={user?.avatarUrl || `https://unavatar.io/twitter/${user?.twitterHandle}`}
          alt={`@${user?.twitterHandle}`}
          className="w-7 h-7 rounded-full object-cover bg-gray-200 dark:bg-dark-700"
        />
        <span className="text-gray-900 dark:text-white text-sm font-medium">@{user?.twitterHandle}</span>
        <ChevronDown className="w-4 h-4 text-gray-600 dark:text-gray-400" />
      </button>

      {showDropdown && (
        <div className="absolute right-0 mt-2 w-64 bg-white dark:bg-dark-900 border border-gray-200 dark:border-white/10 rounded-xl py-2 z-50 shadow-xl">
            {/* Wallet Section */}
            <div className="px-4 py-3 border-b border-gray-200 dark:border-white/10">
              {currentAccount ? (
                <div className="flex items-center justify-between">
                  <div className="flex items-center gap-2">
                    <div className="w-6 h-6 rounded-full bg-sui-gradient flex items-center justify-center">
                      <Wallet className="w-3 h-3 text-white" />
                    </div>
                    <span className="text-sm text-sui-600 dark:text-sui-400 font-medium">
                      {shortenAddress(currentAccount.address)}
                    </span>
                  </div>
                  <div className="flex items-center gap-1">
                    <button
                      onClick={() => copy(currentAccount.address)}
                      className="p-1.5 rounded-lg hover:bg-gray-100 dark:hover:bg-white/10 transition-colors"
                      aria-label="Copy address"
                    >
                      {copied ? (
                        <Check className="w-3.5 h-3.5 text-green-600 dark:text-cyber-green" />
                      ) : (
                        <Copy className="w-3.5 h-3.5 text-gray-600 dark:text-gray-400" />
                      )}
                    </button>
                    <button
                      onClick={() => disconnect()}
                      className="p-1.5 rounded-lg hover:bg-red-50 dark:hover:bg-red-500/10 transition-colors"
                      title="Disconnect wallet"
                      aria-label="Disconnect wallet"
                    >
                      <LogOut className="w-3.5 h-3.5 text-red-600 dark:text-red-400" />
                    </button>
                  </div>
                </div>
              ) : (
                <div>
                  <button
                    onClick={() => setShowWalletList(!showWalletList)}
                    className="w-full flex items-center justify-between px-3 py-2 rounded-lg bg-gray-100 dark:bg-white/5 hover:bg-gray-200 dark:hover:bg-white/10 transition-colors"
                  >
                    <div className="flex items-center gap-2">
                      <Wallet className="w-4 h-4 text-gray-700 dark:text-gray-400" />
                      <span className="text-sm text-gray-900 dark:text-white font-medium">Connect Wallet</span>
                    </div>
                    <ChevronDown className={`w-4 h-4 text-gray-600 dark:text-gray-400 transition-transform duration-200 ${showWalletList ? 'rotate-180' : ''}`} />
                  </button>
                  {showWalletList && (
                    <div className="mt-2 space-y-1">
                      {wallets.length === 0 ? (
                        <p className="px-3 py-2 text-sm text-gray-600 dark:text-gray-400">No wallets found</p>
                      ) : (
                        wallets.map((wallet) => (
                          <button
                            key={wallet.name}
                            onClick={() => {
                              connect({ wallet });
                              setShowWalletList(false);
                            }}
                            className="w-full px-3 py-2 flex items-center gap-2 rounded-lg hover:bg-gray-100 dark:hover:bg-white/10 transition-colors"
                          >
                            {wallet.icon && (
                              <img src={wallet.icon} alt={wallet.name} className="w-5 h-5 rounded" />
                            )}
                            <span className="text-gray-900 dark:text-white text-sm">{wallet.name}</span>
                          </button>
                        ))
                      )}
                    </div>
                  )}
                </div>
              )}
            </div>

            {/* Profile */}
            {!isProfilePage && (
              <button
                onClick={() => {
                  navigate('/profile');
                  setShowDropdown(false);
                }}
                className="w-full px-4 py-3 flex items-center gap-3 hover:bg-gray-100 dark:hover:bg-white/10 transition-colors"
              >
                <User className="w-4 h-4 text-gray-700 dark:text-gray-400" />
                <span className="text-gray-900 dark:text-white text-sm">Profile</span>
              </button>
            )}

            {/* Link/Relink Wallet - show when wallet is connected */}
            {currentAccount && (
              <button
                onClick={() => {
                  navigate('/profile?relink=true');
                  setShowDropdown(false);
                }}
                className="w-full px-4 py-3 flex items-center gap-3 hover:bg-gray-100 dark:hover:bg-white/10 transition-colors"
              >
                <Link2 className="w-4 h-4 text-sui-500 dark:text-sui-400" />
                <span className="text-gray-900 dark:text-white text-sm">
                  {user?.linkedWalletAddress ? 'Relink Wallet' : 'Link Wallet'}
                </span>
              </button>
            )}

            {/* Theme Toggle */}
            <div className="px-4 py-3 flex items-center justify-between">
              <div className="flex items-center gap-3">
                <div className="relative w-4 h-4">
                  <Sun className={`w-4 h-4 text-amber-500 absolute inset-0 transition-all duration-300 ${isDark ? 'opacity-0 rotate-90 scale-0' : 'opacity-100 rotate-0 scale-100'}`} />
                  <Moon className={`w-4 h-4 text-sui-500 dark:text-sui-400 absolute inset-0 transition-all duration-300 ${isDark ? 'opacity-100 rotate-0 scale-100' : 'opacity-0 -rotate-90 scale-0'}`} />
                </div>
                <span className="text-gray-900 dark:text-white text-sm">{isDark ? 'Dark Mode' : 'Light Mode'}</span>
              </div>
              <button
                onClick={toggleTheme}
                className={`relative inline-flex items-center rounded-full transition-all duration-300 ${isDark ? 'bg-sui-500 h-6 w-10' : 'bg-gray-400 h-6 w-10'}`}
                aria-label={isDark ? 'Switch to light mode' : 'Switch to dark mode'}
              >
                <span
                  className={`inline-block h-4 w-4 rounded-full bg-white shadow transition-transform duration-300 ${isDark ? 'translate-x-[18px]' : 'translate-x-[2px]'}`}
                />
              </button>
            </div>

            {/* Logout */}
            <div className="border-t border-gray-200 dark:border-white/10 mt-1 pt-1">
              <button
                onClick={() => {
                  logout();
                  setShowDropdown(false);
                }}
                className="w-full px-4 py-3 flex items-center gap-3 text-red-600 dark:text-red-400 hover:bg-red-50 dark:hover:bg-red-500/10 transition-colors"
              >
                <LogOut className="w-4 h-4" />
                <span className="text-sm font-medium">Logout</span>
              </button>
            </div>
          </div>
      )}
    </div>
  );
};

export const Header: React.FC = () => {
  const navigate = useNavigate();
  const { isAuthenticated } = useAuth();
  const { initiateLogin: initiateXLogin, isLoading: xAuthLoading } = useXAuth();
  const { toggleTheme, isDark } = useTheme();

  return (
    <header className="glass-subtle border-b border-white/5 sticky top-0 z-[100]">
      <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-4">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-4">
            {/* Logo */}
            <div
              onClick={() => navigate('/')}
              className="cursor-pointer flex items-center gap-2"
            >
              <img src="/android-chrome-512x512.png" alt="Nautilus" className="h-8 w-8 object-contain" />
              <span className="text-xl font-bold text-gray-900 dark:text-white">Nautilus X Wallet</span>
            </div>
          </div>

          <div className="flex items-center gap-3">
            {isAuthenticated ? (
              <ProfileDropdown />
            ) : (
              <>
                {/* Theme Toggle (when not logged in) */}
                <button
                  onClick={toggleTheme}
                  className="p-2 rounded-lg glass hover:bg-white/10 transition-colors"
                  aria-label={isDark ? 'Switch to light mode' : 'Switch to dark mode'}
                >
                  {isDark ? (
                    <Sun className="w-5 h-5 text-yellow-400" />
                  ) : (
                    <Moon className="w-5 h-5 text-sui-500" />
                  )}
                </button>

                {/* X Login Button */}
                <button
                  onClick={initiateXLogin}
                  disabled={xAuthLoading}
                  className="btn-sui flex items-center gap-2"
                >
                  <XIcon className="w-4 h-4" />
                  {xAuthLoading ? 'Connecting...' : 'Login with X'}
                </button>
              </>
            )}
          </div>
        </div>
      </div>
    </header>
  );
};
