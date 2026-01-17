import React, { useState, useEffect } from 'react';
import { useNavigate, useParams } from 'react-router-dom';
import { useDocumentTitle } from '../hooks/useDocumentTitle';
import { Header } from '../components/Header';
import { TokenIcon } from '../components/TokenIcon';
import { TransactionDetailModal } from '../components/TransactionDetailModal';
import { ActivityCard } from '../components/ActivityCard';
import { PageSizeDropdown } from '../components/PageSizeDropdown';
import { PaginationControls } from '../components/PaginationControls';
import { NFTGrid } from '../components/NFTCard';
import { DepositModal } from '../components/DepositModal';
import { ConnectWallet } from '../components/ConnectWallet';
import { useActivitiesStream } from '../hooks/useActivitiesStream';
import { useXWalletAccountView } from '../hooks/useXWalletAccount';
import type { PageSize } from '../hooks/usePagination';
import { useCurrentAccount } from '@mysten/dapp-kit';
import { API_BASE_URL } from '../utils/constants';
import {
  ExternalLink,
  Copy,
  Check,
  AlertCircle,
  ArrowLeftRight,
  ArrowDownLeft,
} from 'lucide-react';
import type { Activity } from '../hooks/useActivitiesStream';

// X icon component (the new X logo)
const XIcon: React.FC<{ className?: string }> = ({ className }) => (
  <svg viewBox="0 0 24 24" className={className} fill="currentColor">
    <path d="M18.244 2.25h3.308l-7.227 8.26 8.502 11.24H16.17l-5.214-6.817L4.99 21.75H1.68l7.73-8.835L1.254 2.25H8.08l4.713 6.231zm-1.161 17.52h1.833L7.084 4.126H5.117z" />
  </svg>
);


interface AccountData {
  account: {
    x_user_id: string;
    x_handle: string;
    sui_object_id: string;
    owner_address: string | null;
  };
}

// Activity type for modal
type SelectedActivity = Activity | null;

// Tab types for AccountView (public view - no tweets tab)
type TabType = 'overview' | 'activities' | 'nfts';

const isValidTab = (tab: string | undefined): tab is Exclude<TabType, 'overview'> => {
  return tab === 'activities' || tab === 'nfts';
};

export const AccountView: React.FC = () => {
  const { twitter_id, digest, tab } = useParams<{ twitter_id: string; digest?: string; tab?: string }>();
  const navigate = useNavigate();
  const currentAccount = useCurrentAccount();

  // Determine initial tab from URL
  const getInitialTab = (): TabType => {
    if (digest) return 'activities';
    if (isValidTab(tab)) return tab;
    return 'overview';
  };

  const [account, setAccount] = useState<AccountData | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string>('');
  const [activeTab, setActiveTab] = useState<TabType>(getInitialTab());
  const [copiedField, setCopiedField] = useState<string | null>(null);
  const [selectedActivity, setSelectedActivity] = useState<SelectedActivity>(null);
  const [showDepositModal, setShowDepositModal] = useState(false);

  const isWalletConnected = !!currentAccount?.address;

  useDocumentTitle(account ? `@${account.account.x_handle} - XWallet` : 'XWallet Account');

  // Fetch account info from backend (to get sui_object_id from twitter_id)
  useEffect(() => {
    const fetchAccount = async () => {
      if (!twitter_id) return;

      setIsLoading(true);
      setError('');

      try {
        const accountRes = await fetch(`${API_BASE_URL}/api/accounts/${twitter_id}`);

        if (!accountRes.ok) {
          if (accountRes.status === 404) {
            setError('Account not found');
          } else {
            setError('Failed to load account');
          }
          return;
        }
        const accountData = await accountRes.json();
        setAccount(accountData);
      } catch {
        setError('Failed to load account');
      } finally {
        setIsLoading(false);
      }
    };

    fetchAccount();
  }, [twitter_id]);

  // Pagination state (server-side)
  const [currentPage, setCurrentPage] = useState(1);
  const [pageSize, setPageSizeState] = useState<PageSize>(10);

  const setPageSize = (size: PageSize) => {
    setPageSizeState(size);
    setCurrentPage(1); // Reset to first page when changing page size
  };

  const goToPage = (page: number) => {
    if (page >= 1) setCurrentPage(page);
  };

  // Use activities stream hook with server-side pagination
  const suiObjectId = account?.account?.sui_object_id;
  const {
    combinedActivities,
    isLoading: isLoadingActivities,
    totalItems: totalActivities,
    totalPages,
  } = useActivitiesStream(suiObjectId, { page: currentPage, pageSize });

  // Fetch balances and NFTs from on-chain
  const {
    balances: onChainBalances,
    nfts: accountNFTs,
    ownerAddress,
    isLoading: isLoadingOnChain,
  } = useXWalletAccountView(suiObjectId);

  // Auto-open modal when digest is in URL (only on initial load or direct navigation)
  const [hasOpenedFromUrl, setHasOpenedFromUrl] = useState(false);

  useEffect(() => {
    // Reset flag when digest changes
    if (!digest) {
      setHasOpenedFromUrl(false);
      return;
    }

    // Skip if already opened from this URL or still loading
    if (hasOpenedFromUrl || isLoadingActivities) return;

    // Find transaction in loaded activities
    const activity = combinedActivities.find((a) => a.data.tx_digest === digest);
    if (activity) {
      setSelectedActivity(activity);
      setHasOpenedFromUrl(true);
      return;
    }

    // If not found in loaded activities, fetch from API
    const fetchTransaction = async () => {
      try {
        const response = await fetch(`${API_BASE_URL}/api/tx/${digest}`);
        if (!response.ok) return;

        const data = await response.json();
        // Construct Activity type based on transaction data
        if (data.tx_type === 'link_wallet') {
          setSelectedActivity({ type: 'link_wallet', data: data });
        } else if (data.nft_object_id) {
          setSelectedActivity({ type: 'nft', data: data });
        } else {
          setSelectedActivity({ type: 'coin', data: data });
        }
        setHasOpenedFromUrl(true);
      } catch {
        // Transaction not found
      }
    };

    fetchTransaction();
  }, [digest, combinedActivities, isLoadingActivities, hasOpenedFromUrl]);

  // Handle opening activity modal - update URL for shareability
  const handleOpenActivity = (activity: SelectedActivity) => {
    if (!activity) return;
    setSelectedActivity(activity);
    setHasOpenedFromUrl(true);
    navigate(`/account/${twitter_id}/activities/tx/${activity.data.tx_digest}`, { replace: true });
  };

  // Handle modal close - navigate back to current tab
  const handleModalClose = () => {
    setSelectedActivity(null);
    navigate(`/account/${twitter_id}/activities`, { replace: true });
  };

  // Handle tab change - update URL
  const handleTabChange = (newTab: TabType) => {
    setActiveTab(newTab);
    if (newTab === 'overview') {
      navigate(`/account/${twitter_id}`, { replace: true });
    } else {
      navigate(`/account/${twitter_id}/${newTab}`, { replace: true });
    }
  };


  const copyToClipboard = async (text: string, field: string) => {
    await navigator.clipboard.writeText(text);
    setCopiedField(field);
    setTimeout(() => setCopiedField(null), 2000);
  };

  // Loading state
  if (isLoading) {
    return (
      <div className="min-h-screen">
        <Header />
        <div className="flex items-center justify-center py-32">
          <div className="w-12 h-12 border-4 border-sui-500/30 border-t-sui-500 rounded-full animate-spin" />
        </div>
      </div>
    );
  }

  // Error state
  if (error) {
    return (
      <div className="min-h-screen">
        <Header />
        <div className="max-w-2xl mx-auto px-4 py-20">
          <div className="glass rounded-2xl p-8 text-center">
            <div className="w-16 h-16 mx-auto mb-6 rounded-full bg-red-500/20 flex items-center justify-center">
              <AlertCircle className="w-8 h-8 text-red-400" />
            </div>
            <h2 className="text-2xl font-bold text-white mb-3">{error}</h2>
            <p className="text-gray-400 mb-6">
              The account you're looking for may not exist or hasn't been created yet.
            </p>
            <button
              onClick={() => navigate('/')}
              className="btn-sui"
            >
              Back to Search
            </button>
          </div>
        </div>
      </div>
    );
  }

  if (!account) {
    return null;
  }

  // Avatar: use unavatar.io proxy for other users' accounts
  const avatarUrl = `https://unavatar.io/twitter/${account.account.x_handle}`;

  return (
    <div className="min-h-screen">
      <Header />

      <main className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
        {/* Balance Card */}
        <div className="glass rounded-2xl p-8 mb-8 relative overflow-hidden">
          <div className="absolute inset-0 bg-sui-gradient opacity-10" />
          <div className="absolute top-0 right-0 w-64 h-64 bg-cyber-cyan/20 rounded-full blur-3xl -translate-y-1/2 translate-x-1/2" />

          {/* Header Row - Profile & Links */}
          <div className="relative flex flex-col md:flex-row justify-between items-start gap-4 mb-12">
            <div className="flex items-center gap-3">
              <img
                src={avatarUrl}
                alt={`@${account.account.x_handle}`}
                className="w-12 h-12 rounded-full object-cover bg-sui-500/20"
              />
              <div>
                <h2 className="text-xl font-bold text-white">@{account.account.x_handle}</h2>
                <p className="text-sm text-gray-400">ID: {account.account.x_user_id}</p>
              </div>
            </div>
            {/* Links */}
            <div className="flex flex-wrap gap-2">
              <a
                href={`https://x.com/${account.account.x_handle}`}
                target="_blank"
                rel="noopener noreferrer"
                className="btn-glass text-sm flex items-center gap-1"
              >
                <XIcon className="w-4 h-4" />
                Profile
              </a>
              <a
                href={`https://suiscan.xyz/testnet/object/${account.account.sui_object_id}`}
                target="_blank"
                rel="noopener noreferrer"
                className="btn-glass text-sm flex items-center gap-1"
              >
                <ExternalLink className="w-4 h-4" />
                Explorer
              </a>
            </div>
          </div>

          {/* Balances Row with Action Buttons */}
          <div className="relative">
            <p className="text-sm text-gray-400 mb-2 tracking-wide">Balances</p>
            <div className="flex flex-wrap items-end justify-between gap-4">
              {/* Token Balances (from on-chain) */}
              <div className="flex flex-wrap gap-4">
                {isLoadingOnChain ? (
                  <div className="animate-pulse text-gray-400 text-2xl">Loading...</div>
                ) : onChainBalances.length > 0 ? (
                  onChainBalances.map((token) => (
                    <div key={token.coin_type} className="flex items-center gap-3 glass rounded-xl px-4 py-3">
                      <TokenIcon symbol={token.symbol} size="lg" />
                      <div>
                        <p className="text-2xl font-bold text-white">{token.balance_formatted}</p>
                        <p className="text-sm text-gray-400">{token.symbol}</p>
                      </div>
                    </div>
                  ))
                ) : (
                  <div className="flex items-center gap-3 glass rounded-xl px-4 py-3">
                    <TokenIcon symbol="SUI" size="lg" />
                    <div>
                      <p className="text-2xl font-bold text-white">0</p>
                      <p className="text-sm text-gray-400">SUI</p>
                    </div>
                  </div>
                )}
              </div>

              {/* Action Button */}
              <div className="flex gap-2">
                {isWalletConnected ? (
                  <button
                    onClick={() => setShowDepositModal(true)}
                    className="btn-sui flex items-center justify-center gap-2 min-w-[150px]"
                  >
                    <ArrowDownLeft className="w-4 h-4" />
                    Deposit
                  </button>
                ) : (
                  <ConnectWallet />
                )}
              </div>
            </div>
          </div>
        </div>

        {/* Tabs */}
        <div className="glass rounded-2xl overflow-hidden mb-8">
          <div className="border-b border-white/5">
            <nav className="flex gap-1 p-2">
              {(['overview', 'activities', 'nfts'] as const).map((tab) => (
                <button
                  key={tab}
                  className={`px-5 py-2.5 rounded-lg font-medium text-sm transition-all ${
                    activeTab === tab ? 'bg-gray-200 dark:bg-white/10 text-gray-900 dark:text-white' : 'text-gray-600 dark:text-gray-400 hover:text-gray-900 dark:hover:text-white hover:bg-gray-100 dark:hover:bg-white/5'
                  }`}
                  onClick={() => handleTabChange(tab)}
                >
                  {tab === 'overview' && 'Overview'}
                  {tab === 'activities' && 'Activities'}
                  {tab === 'nfts' && 'NFTs'}
                </button>
              ))}
            </nav>
          </div>

          <div className="p-6">
            {activeTab === 'overview' && (
              <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                {[
                  { label: 'X User ID', value: account.account.x_user_id, copyable: true },
                  { label: 'X Handle', value: `@${account.account.x_handle}`, copyable: false },
                  { label: 'XWallet Account ID', value: account.account.sui_object_id, copyable: true, mono: true, link: `https://suiscan.xyz/testnet/object/${account.account.sui_object_id}` },
                  { label: 'Linked Wallet', value: ownerAddress || 'Not linked', copyable: !!ownerAddress, mono: true },
                ].map((item) => (
                  <div key={item.label} className="glass-subtle rounded-xl p-4">
                    <p className="text-sm text-gray-500 mb-1">{item.label}</p>
                    <div className="flex items-center justify-between">
                      <p className={`text-white ${item.mono ? 'text-sm break-all' : ''}`}>
                        {item.mono && item.value && item.value.length > 20
                          ? `${item.value.slice(0, 16)}...${item.value.slice(-8)}`
                          : item.value}
                      </p>
                      <div className="flex items-center gap-1">
                        {item.copyable && item.value && (
                          <button onClick={() => copyToClipboard(item.value, item.label)} className="p-1.5 rounded-lg hover:bg-white/10">
                            {copiedField === item.label ? <Check className="w-4 h-4 text-cyber-green" /> : <Copy className="w-4 h-4 text-gray-400" />}
                          </button>
                        )}
                        {item.link && (
                          <a href={item.link} target="_blank" rel="noopener noreferrer" className="p-1.5 rounded-lg hover:bg-white/10">
                            <ExternalLink className="w-4 h-4 text-sui-400" />
                          </a>
                        )}
                      </div>
                    </div>
                  </div>
                ))}
              </div>
            )}

            {activeTab === 'activities' && (
              <div>
                {/* Top header with page size dropdown */}
                {totalActivities > 0 && (
                  <div className="flex items-center justify-between mb-4">
                    <p className="text-sm text-gray-400">
                      {totalActivities} activities
                    </p>
                    <PageSizeDropdown value={pageSize} onChange={setPageSize} />
                  </div>
                )}
                {isLoadingActivities && combinedActivities.length === 0 ? (
                  <div className="text-center py-12">
                    <div className="w-8 h-8 border-2 border-sui-500/30 border-t-sui-500 rounded-full animate-spin mx-auto mb-4" />
                    <p className="text-gray-400">Loading activities...</p>
                  </div>
                ) : combinedActivities.length === 0 ? (
                  <div className="text-center py-12">
                    <div className="w-16 h-16 mx-auto mb-4 rounded-full bg-white/5 flex items-center justify-center">
                      <ArrowLeftRight className="w-8 h-8 text-gray-500" />
                    </div>
                    <p className="text-gray-400">No activities yet</p>
                  </div>
                ) : (
                  <>
                    <div className="space-y-3">
                      {combinedActivities.map((activity) => (
                        <ActivityCard
                          key={activity.data.tx_digest}
                          activity={activity}
                          currentXid={account?.account?.x_user_id}
                          linkedWallet={ownerAddress}
                          onClick={() => handleOpenActivity(activity)}
                        />
                      ))}
                    </div>
                    {(totalPages > 1 || totalActivities > 0) && (
                      <div className="mt-6 pt-4 border-t border-white/5">
                        <div className="flex items-center justify-between">
                          <div className="flex items-center gap-3">
                            <PageSizeDropdown value={pageSize} onChange={setPageSize} position="top" />
                            <p className="text-sm text-gray-500">
                              {totalActivities} activities
                            </p>
                          </div>
                          <PaginationControls
                            currentPage={currentPage}
                            totalPages={totalPages}
                            onPageChange={goToPage}
                          />
                        </div>
                      </div>
                    )}
                  </>
                )}
              </div>
            )}

            {activeTab === 'nfts' && (
              <div>
                <NFTGrid
                  nfts={accountNFTs}
                  loading={isLoadingOnChain}
                  emptyMessage="No NFTs in this account"
                />
              </div>
            )}
          </div>
        </div>

      </main>

      {/* Transaction Detail Modal */}
      {selectedActivity && (
        <TransactionDetailModal
          activity={selectedActivity}
          currentXid={account?.account?.x_user_id}
          shareableUrl={`${window.location.origin}/account/${twitter_id}/activities/tx/${selectedActivity.data.tx_digest}`}
          onClose={handleModalClose}
        />
      )}

      <DepositModal
        isOpen={showDepositModal}
        onClose={() => setShowDepositModal(false)}
        targetHandle={account?.account?.x_handle || ''}
        suiObjectId={account?.account?.sui_object_id || ''}
      />
    </div>
  );
};
