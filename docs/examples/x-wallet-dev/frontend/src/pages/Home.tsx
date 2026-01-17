import React, { useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { useDocumentTitle } from '../hooks/useDocumentTitle';
import { Header } from '../components/Header';
import { API_BASE_URL } from '../utils/constants';
import {
  Search,
  Send,
  Shield,
  ExternalLink,
  Copy,
  Check,
} from 'lucide-react';

interface AccountSearchResult {
  x_user_id: string;
  x_handle: string;
  sui_object_id: string;
  owner_address?: string;
}

export const Home: React.FC = () => {
  useDocumentTitle('XWallet Explorer - Social Wallet on Sui');
  const navigate = useNavigate();

  const [searchQuery, setSearchQuery] = useState('');
  const [searchResults, setSearchResults] = useState<AccountSearchResult[]>([]);
  const [isSearching, setIsSearching] = useState(false);
  const [searchError, setSearchError] = useState<string>('');
  const [hasSearched, setHasSearched] = useState(false);
  const [copiedField, setCopiedField] = useState<string | null>(null);

  // Copy to clipboard
  const copyToClipboard = async (text: string, field: string) => {
    await navigator.clipboard.writeText(text);
    setCopiedField(field);
    setTimeout(() => setCopiedField(null), 2000);
  };

  // Handle search
  const handleSearch = async (e: React.FormEvent) => {
    e.preventDefault();

    if (!searchQuery.trim()) {
      return;
    }

    setIsSearching(true);
    setSearchError('');
    setSearchResults([]);
    setHasSearched(true);

    try {
      const response = await fetch(`${API_BASE_URL}/api/accounts/search?q=${encodeURIComponent(searchQuery)}`);

      if (!response.ok) {
        throw new Error('Search failed');
      }

      const data = await response.json();
      setSearchResults(data.accounts || []);
    } catch {
      setSearchError('Failed to search accounts. Please try again.');
    } finally {
      setIsSearching(false);
    }
  };

  return (
    <div className="min-h-screen">
      <Header />

      {/* Hero Section with Search - Full viewport height */}
      <section className="relative overflow-hidden min-h-[calc(100vh-64px)] flex flex-col justify-center">
        {/* Background gradient */}
        <div className="absolute inset-0 bg-gradient-to-b from-sui-500/10 via-transparent to-transparent" />

        <div className="max-w-4xl w-full mx-auto px-4 sm:px-6 lg:px-8 pt-16 pb-8 relative">
          <div className="text-center mb-8">
            <h1 className="text-4xl md:text-5xl font-bold text-gray-900 dark:text-white mb-4">
              XWallet <span className="text-gradient">Explorer</span>
            </h1>
            <p className="text-lg text-gray-600 dark:text-gray-400 max-w-2xl mx-auto">
              Search any account by @handle, X User ID, or Sui address
            </p>
          </div>

          {/* Search Form - Hero */}
          <form onSubmit={handleSearch} className="mb-8">
            <div className="relative">
              <input
                type="text"
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
                placeholder="@alice, 123456789, or 0x..."
                className="input-glass pl-6 pr-16 py-5 text-lg rounded-2xl w-full"
                autoFocus
              />
              <button
                type="submit"
                disabled={isSearching || !searchQuery.trim()}
                className="absolute inset-y-0 right-0 px-5 m-2 btn-sui rounded-xl disabled:opacity-40 disabled:cursor-not-allowed"
              >
                {isSearching ? (
                  <div className="w-5 h-5 border-2 border-white/30 border-t-white rounded-full animate-spin" />
                ) : (
                  <Search className="w-5 h-5" />
                )}
              </button>
            </div>
          </form>

          {/* Search Error */}
          {searchError && (
            <div className="mb-6 p-4 glass rounded-xl border-red-500/30 bg-red-500/10">
              <p className="text-red-400">{searchError}</p>
            </div>
          )}

          {/* Search Results */}
          {searchResults.length > 0 && (
            <div className="glass rounded-2xl overflow-hidden mb-8">
              <div className="p-5 border-b border-gray-200 dark:border-white/10">
                <h3 className="text-lg font-semibold text-gray-900 dark:text-white">
                  Results ({searchResults.length})
                </h3>
              </div>
              <div className="divide-y divide-gray-200 dark:divide-white/5">
                {searchResults.map((account) => (
                  <div
                    key={account.x_user_id}
                    className="p-6 hover:bg-gray-50 dark:hover:bg-white/5 transition-colors cursor-pointer"
                    onClick={() => navigate(`/account/${account.x_user_id}`)}
                  >
                    <div className="flex items-start justify-between">
                      <div className="flex-1">
                        <div className="flex items-center gap-3 mb-3">
                          <img
                            src={`https://unavatar.io/twitter/${account.x_handle}`}
                            alt={`@${account.x_handle}`}
                            className="w-10 h-10 rounded-full object-cover bg-sui-500/20"
                            onError={(e) => {
                              // Fallback to initial letter if avatar fails to load
                              const target = e.target as HTMLImageElement;
                              target.style.display = 'none';
                              target.nextElementSibling?.classList.remove('hidden');
                            }}
                          />
                          <div className="w-10 h-10 rounded-full bg-sui-gradient flex items-center justify-center text-white font-bold hidden">
                            {account.x_handle[0]?.toUpperCase()}
                          </div>
                          <div>
                            <span className="text-xl font-bold text-gray-900 dark:text-white">
                              @{account.x_handle}
                            </span>
                            <p className="text-sm text-gray-600 dark:text-gray-500">
                              ID: {account.x_user_id}
                            </p>
                          </div>
                        </div>

                        <div className="space-y-2 ml-13">
                          <div className="flex items-center gap-2">
                            <span className="text-xs font-medium text-gray-600 dark:text-gray-500 uppercase tracking-wide">
                              Account
                            </span>
                            <span className="text-xs text-sui-600 dark:text-sui-400 bg-sui-500/10 px-2 py-1 rounded-lg">
                              {account.sui_object_id.slice(0, 20)}...{account.sui_object_id.slice(-8)}
                            </span>
                          </div>

                          {account.owner_address && (
                            <div className="flex items-center gap-2">
                              <span className="text-xs font-medium text-gray-600 dark:text-gray-500 uppercase tracking-wide">
                                Owner
                              </span>
                              <span className="text-xs text-gray-600 dark:text-gray-400 bg-gray-100 dark:bg-white/5 px-2 py-1 rounded-lg">
                                {account.owner_address.slice(0, 20)}...{account.owner_address.slice(-8)}
                              </span>
                            </div>
                          )}
                        </div>
                      </div>
                    </div>
                  </div>
                ))}
              </div>
            </div>
          )}

          {/* Empty State */}
          {!isSearching && searchResults.length === 0 && hasSearched && !searchError && (
            <div className="text-center py-12 glass rounded-2xl">
              <div className="w-16 h-16 mx-auto mb-4 rounded-full bg-gray-100 dark:bg-white/5 flex items-center justify-center">
                <Search className="w-8 h-8 text-gray-500 dark:text-gray-500" />
              </div>
              <p className="text-gray-600 dark:text-gray-400 mb-2">
                No accounts found for "<span className="text-gray-900 dark:text-white">{searchQuery}</span>"
              </p>
            </div>
          )}
        </div>

        {/* Feature Cards - Inside hero section */}
        {!hasSearched && (
          <div className="max-w-4xl mx-auto px-4 sm:px-6 lg:px-8 pb-8 relative">
            <div className="grid md:grid-cols-3 gap-6">
              <div className="glass glass-hover rounded-2xl p-6 group text-center">
                <div className="w-12 h-12 rounded-xl bg-sui-500/20 flex items-center justify-center mb-4 group-hover:shadow-glow-sm transition-shadow mx-auto">
                  <Search className="w-6 h-6 text-sui-400" />
                </div>
                <h3 className="text-lg font-semibold text-gray-900 dark:text-white mb-2">
                  Search Accounts
                </h3>
                <p className="text-gray-600 dark:text-gray-400 min-h-[48px]">
                  Find any XWallet account by X handle, user ID, or Sui address
                </p>
              </div>

              <div className="glass glass-hover rounded-2xl p-6 group text-center">
                <div className="w-12 h-12 rounded-xl bg-cyber-cyan/20 flex items-center justify-center mb-4 group-hover:shadow-glow-cyan transition-shadow mx-auto">
                  <Send className="w-6 h-6 text-cyber-cyan" />
                </div>
                <h3 className="text-lg font-semibold text-gray-900 dark:text-white mb-2">
                  Tweet to Transact
                </h3>
                <p className="text-gray-600 dark:text-gray-400 min-h-[48px]">
                  Send tokens to anyone on X by tweeting @NautilusXWallet
                </p>
              </div>

              <div className="glass glass-hover rounded-2xl p-6 group text-center">
                <div className="w-12 h-12 rounded-xl bg-sui-500/20 flex items-center justify-center mb-4 group-hover:shadow-glow-sm transition-shadow mx-auto">
                  <Shield className="w-6 h-6 text-sui-400" />
                </div>
                <h3 className="text-lg font-semibold text-gray-900 dark:text-white mb-2">
                  Secure & Verifiable
                </h3>
                <p className="text-gray-600 dark:text-gray-400 min-h-[48px]">
                  Powered by Sui blockchain and Nautilus Enclave for trusted execution
                </p>
              </div>
            </div>
          </div>
        )}
      </section>

      {/* Bottom Section - Get Started & Commands */}
      {!hasSearched && (
        <section className="pb-16">
          <div className="max-w-4xl mx-auto px-4 sm:px-6 lg:px-8">
            {/* Get Started */}
            <div className="glass rounded-2xl p-8 border-sui-500/30 bg-sui-500/5">
              <div className="flex flex-col md:flex-row md:items-center justify-between gap-4">
                <div>
                  <h3 className="text-xl font-semibold text-gray-900 dark:text-white mb-2">Get Started</h3>
                  <p className="text-sm text-gray-600 dark:text-gray-400">
                    Create your XWallet account by posting the create command on X
                  </p>
                </div>
                <a
                  href={`https://twitter.com/intent/post?text=${encodeURIComponent('@NautilusXWallet create account')}`}
                  target="_blank"
                  rel="noopener noreferrer"
                  className="btn-sui whitespace-nowrap"
                >
                  Post on X
                </a>
              </div>
            </div>

            {/* Available Commands */}
            <div className="mt-8 glass rounded-2xl p-8">
              <h3 className="text-xl font-semibold text-gray-900 dark:text-white mb-6 text-center">Available Commands</h3>
              <div className="space-y-4">
                {[
                  { cmd: '@NautilusXWallet create account', desc: 'Create your XWallet account', badge: 'Getting Started', badgeColor: 'bg-green-200 text-green-700 dark:bg-green-400/30 dark:text-green-300' },
                  { cmd: '@NautilusXWallet link wallet 0x...', desc: 'Link your Sui wallet address for withdrawals', badge: 'Account', badgeColor: 'bg-amber-200 text-amber-700 dark:bg-amber-400/30 dark:text-amber-300' },
                  { cmd: '@NautilusXWallet send <amount> <token> to @handle', desc: 'Send tokens to any X user', badge: 'Transfer', badgeColor: 'bg-cyan-200 text-cyan-700 dark:bg-cyan-400/30 dark:text-cyan-300' },
                  { cmd: '@NautilusXWallet send nft <nft_id> to @handle', desc: 'Send an NFT by ID to any X user', badge: 'NFT', badgeColor: 'bg-purple-200 text-purple-700 dark:bg-purple-400/30 dark:text-purple-300' },
                  { cmd: '@NautilusXWallet send nft "<nft_name>" to @handle', desc: 'Send an NFT by name to any X user', badge: 'NFT', badgeColor: 'bg-purple-200 text-purple-700 dark:bg-purple-400/30 dark:text-purple-300' },
                ].map((item, index) => (
                  <div key={index} className="bg-gray-100 dark:bg-dark-800/50 rounded-xl p-5">
                    <div className="flex flex-col md:flex-row md:items-center justify-between gap-3">
                      <div className="flex-1">
                        <span className="text-sui-600 dark:text-sui-400 font-medium">{item.cmd}</span>
                        <p className="text-sm text-gray-600 dark:text-gray-500 mt-1">{item.desc}</p>
                      </div>
                      <div className="flex items-center gap-2">
                        <span className={`text-xs px-3 py-1.5 rounded-full font-medium ${item.badgeColor}`}>{item.badge}</span>
                        <button
                          onClick={() => copyToClipboard(item.cmd, `cmd-${index}`)}
                          className="p-2 rounded-lg hover:bg-gray-200 dark:hover:bg-white/10 transition-colors"
                          title="Copy command"
                        >
                          {copiedField === `cmd-${index}` ? (
                            <Check className="w-4 h-4 text-green-500" />
                          ) : (
                            <Copy className="w-4 h-4 text-gray-500 dark:text-gray-400" />
                          )}
                        </button>
                        <a
                          href={`https://twitter.com/intent/post?text=${encodeURIComponent(item.cmd)}`}
                          target="_blank"
                          rel="noopener noreferrer"
                          className="p-2 rounded-lg hover:bg-gray-200 dark:hover:bg-white/10 transition-colors"
                          title="Post on X"
                        >
                          <ExternalLink className="w-4 h-4 text-gray-500 dark:text-gray-400" />
                        </a>
                      </div>
                    </div>
                  </div>
                ))}
              </div>
            </div>
          </div>
        </section>
      )}

      {/* Footer */}
      <footer className="border-t border-gray-200 dark:border-white/5 py-8 mt-auto">
        <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
          <div className="flex flex-col md:flex-row justify-between items-center gap-4">
            <p className="text-sm text-gray-600 dark:text-gray-500">
              Powered by <span className="text-sui-600 dark:text-sui-400">Sui</span>
            </p>
            <div className="flex items-center gap-6">
              <a href="#" className="text-sm text-gray-600 dark:text-gray-500 hover:text-gray-900 dark:hover:text-white transition-colors">
                Documentation
              </a>
              <a href="#" className="text-sm text-gray-600 dark:text-gray-500 hover:text-gray-900 dark:hover:text-white transition-colors">
                GitHub
              </a>
              <a
                href="https://twitter.com/NautilusXWallet"
                target="_blank"
                rel="noopener noreferrer"
                className="text-sm text-gray-600 dark:text-gray-500 hover:text-gray-900 dark:hover:text-white transition-colors"
              >
                X
              </a>
            </div>
          </div>
        </div>
      </footer>
    </div>
  );
};
