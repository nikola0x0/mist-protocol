import React, { useState, useEffect } from 'react';
import { useSearchParams, useParams, useNavigate } from 'react-router-dom';
import { useDocumentTitle } from '../hooks/useDocumentTitle';
import { Header } from '../components/Header';
import { useAuth } from '../contexts/AuthContext';
import { useCurrentAccount, useWallets, useConnectWallet, useSuiClient } from '@mysten/dapp-kit';
import { Transaction } from '@mysten/sui/transactions';
import { useAppConfig } from '../contexts/AppConfigContext';
import { XWALLET_PACKAGE_ID, COIN_TYPES } from '../utils/constants';
import { useWithdraw } from '../hooks/useXWalletCoinTransactions';
import { useWithdrawNFT, type NFTObject } from '../hooks/useXWalletNFTTransactions';
import { useTweetStream, getTweetStatusInfo, type TweetStatusData } from '../hooks/useTweetStream';
import { useXWalletAccount } from '../hooks/useXWalletAccount';
import { useActivitiesStream } from '../hooks/useActivitiesStream';
import { useLinkWallet } from '../hooks/useLinkWallet';
import { useCreateAccount } from '../hooks/useCreateAccount';
import type { PageSize } from '../hooks/usePagination';
import { NFTGrid } from '../components/NFTCard';
import { TokenIcon } from '../components/TokenIcon';
import { TransactionDetailModal } from '../components/TransactionDetailModal';
import { ActivityCard } from '../components/ActivityCard';
import { PageSizeDropdown } from '../components/PageSizeDropdown';
import { PaginationControls } from '../components/PaginationControls';
import { DepositModal } from '../components/DepositModal';
import type { Activity } from '../hooks/useActivitiesStream';
import {
  Copy,
  Check,
  ExternalLink,
  RefreshCw,
  ArrowLeftRight,
  ArrowDownLeft,
  ArrowUpRight,
  X,
  Coins,
  MessageSquare,
  ChevronDown,
  Image,
  Wallet,
  AlertTriangle,
} from 'lucide-react';
import { getExplorerUrl } from '../utils/format';

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
  balances: Array<{
    coin_type: string;
    balance: string;
  }>;
}

const ONBOARDING_DISMISSED_KEY = 'xwallet_onboarding_dismissed';

// Tab types for MyAccount (includes tweets tab)
type TabType = 'overview' | 'activities' | 'nfts' | 'tweets';

const isValidTab = (tab: string | undefined): tab is Exclude<TabType, 'overview'> => {
  return tab === 'activities' || tab === 'nfts' || tab === 'tweets';
};

export const MyAccount: React.FC = () => {
  useDocumentTitle('My Account - XWallet');
  const [searchParams, setSearchParams] = useSearchParams();
  const { tab } = useParams<{ tab?: string }>();
  const navigate = useNavigate();
  const { user, refreshAccount } = useAuth();
  const currentAccount = useCurrentAccount();
  const wallets = useWallets();
  const { mutate: connectWallet } = useConnectWallet();

  // Determine initial tab from URL
  const getInitialTab = (): TabType => {
    if (isValidTab(tab)) return tab;
    return 'overview';
  };

  const [activeTab, setActiveTab] = useState<TabType>(getInitialTab());

  // Handle tab change - update URL
  const handleTabChange = (newTab: TabType) => {
    setActiveTab(newTab);
    if (newTab === 'overview') {
      navigate('/profile', { replace: true });
    } else {
      navigate(`/profile/${newTab}`, { replace: true });
    }
  };
  const [copiedField, setCopiedField] = useState<string | null>(null);

  // Link wallet state
  const { linkWallet, isLinking, error: linkError } = useLinkWallet();
  const [showLinkWalletModal, setShowLinkWalletModal] = useState(false);
  const [linkWalletSuccess, setLinkWalletSuccess] = useState<string | null>(null);

  // Create account state
  const { createAccount, isCreating, error: createError } = useCreateAccount();
  const [createSuccess, setCreateSuccess] = useState<string | null>(null);

  // Onboarding state
  const [onboardingDismissed, setOnboardingDismissed] = useState(() => {
    return localStorage.getItem(ONBOARDING_DISMISSED_KEY) === 'true';
  });
  const [showWalletDropdown, setShowWalletDropdown] = useState(false);

  // Delay showing wallet warning to prevent flash on initial load
  const [walletStateReady, setWalletStateReady] = useState(false);
  useEffect(() => {
    const timer = setTimeout(() => setWalletStateReady(true), 500);
    return () => clearTimeout(timer);
  }, []);

  // Modal state
  const [showDepositModal, setShowDepositModal] = useState(false);
  const [showWithdrawModal, setShowWithdrawModal] = useState(false);
  const [withdrawType, setWithdrawType] = useState<'select' | 'tokens' | 'nfts'>('select');
  const [withdrawAmount, setWithdrawAmount] = useState('');

  // Token selection state (for withdraw)
  const [selectedWithdrawToken, setSelectedWithdrawToken] = useState<{ symbol: string; coinType: string; balance: string; decimals: number } | null>(null);
  const [showWithdrawTokenDropdown, setShowWithdrawTokenDropdown] = useState(false);

  // NFT selection state (for withdraw)
  const [selectedNFTsForWithdraw, setSelectedNFTsForWithdraw] = useState<NFTObject[]>([]);

  // Withdraw gas estimation state
  const [estimatedWithdrawGas, setEstimatedWithdrawGas] = useState<string | null>(null);
  const [isEstimatingWithdrawGas, setIsEstimatingWithdrawGas] = useState(false);
  const [withdrawGasEstimateError, setWithdrawGasEstimateError] = useState(false);

  // Transaction detail modal state
  const [selectedActivity, setSelectedActivity] = useState<Activity | null>(null);

  // Transaction hooks (withdraw only - deposit handled by DepositModal)
  const withdrawMutation = useWithdraw();
  const withdrawNFTMutation = useWithdrawNFT();

  // Hooks for gas estimation
  const suiClient = useSuiClient();
  const { sponsorEnabled } = useAppConfig();

  // Use suiObjectId from X OAuth context (set during login)
  const suiObjectId = user?.suiObjectId;

  // Auto-refresh account data if user is logged in but has no XWallet account yet
  useEffect(() => {
    if (user?.twitterUserId && !user.suiObjectId) {
      refreshAccount();
    }
  }, [user?.twitterUserId, user?.suiObjectId, refreshAccount]);

  // Handle relink query param - open link wallet modal
  useEffect(() => {
    if (searchParams.get('relink') === 'true' && currentAccount) {
      setShowLinkWalletModal(true);
      // Clear the query param
      setSearchParams({});
    }
  }, [searchParams, currentAccount, setSearchParams]);

  // Account data from auth context
  const accountData: AccountData | null = user ? {
    account: {
      x_user_id: user.twitterUserId,
      x_handle: user.twitterHandle,
      sui_object_id: user.suiObjectId || '',
      owner_address: user.linkedWalletAddress,
    },
    balances: [],
  } : null;

  // XWallet account data from on-chain (source of truth)
  // This hook also syncs owner_address to AuthContext if different
  const { balances: onChainBalances, nfts: accountNFTs, isLoading: isLoadingAccount } = useXWalletAccount(suiObjectId);

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

  // Activities stream with server-side pagination
  const {
    combinedActivities,
    isLoading: isLoadingActivitiesData,
    totalItems: totalActivities,
    totalPages,
  } = useActivitiesStream(suiObjectId, { page: currentPage, pageSize });

  const isLoadingBalance = isLoadingAccount;
  const isLoadingActivities = isLoadingActivitiesData && combinedActivities.length === 0;

  // Tweet stream for real-time updates
  const { tweets, pendingTweets } = useTweetStream(accountData?.account?.x_user_id);

  // Wallet connection and linking status
  const isWalletConnected = !!currentAccount?.address;
  const linkedWalletAddress = accountData?.account?.owner_address;
  const isWalletLinked = !!linkedWalletAddress;
  const isWalletMatched = isWalletConnected && isWalletLinked &&
    currentAccount?.address?.toLowerCase() === linkedWalletAddress?.toLowerCase();

  // Check if user has deposited any funds (tokens or NFTs)
  const hasDeposited = (onChainBalances.length > 0 && onChainBalances.some(b => parseFloat(b.balance_formatted) > 0)) ||
    (accountNFTs && accountNFTs.length > 0);

  // Deposit: requires wallet connected and linked (mismatch OK - deposit goes to XWallet)
  const canDeposit = isWalletConnected && isWalletLinked;
  // Withdraw: requires wallet connected, linked, AND matched (funds go to connected wallet)
  const canWithdraw = isWalletConnected && isWalletLinked && isWalletMatched;

  // Get warning message for wallet status
  const getWalletWarning = (): string | null => {
    if (!isWalletConnected) return 'Connect your wallet to deposit or withdraw';
    if (!isWalletLinked) return 'Link your wallet to enable transactions';
    if (!isWalletMatched) return 'Connected wallet does not match linked wallet. Withdraw is disabled.';
    return null;
  };
  const walletWarning = getWalletWarning();
  const [walletWarningDismissed, setWalletWarningDismissed] = useState(false);

  // Estimate gas for withdraw
  useEffect(() => {
    if (sponsorEnabled || !selectedWithdrawToken || !withdrawAmount || !currentAccount?.address || !suiObjectId) {
      setEstimatedWithdrawGas(null);
      setWithdrawGasEstimateError(false);
      return;
    }

    const estimateGas = async () => {
      setIsEstimatingWithdrawGas(true);
      setWithdrawGasEstimateError(false);
      try {
        const decimals = selectedWithdrawToken.decimals;
        const parts = withdrawAmount.split('.');
        const multiplier = BigInt(10 ** decimals);
        const whole = BigInt(parts[0] || '0') * multiplier;
        const amountSmallest = parts[1]
          ? whole + BigInt(parts[1].padEnd(decimals, '0').slice(0, decimals))
          : whole;

        if (amountSmallest <= 0n) {
          setEstimatedWithdrawGas(null);
          return;
        }

        const tx = new Transaction();
        tx.setSender(currentAccount.address);
        const isSuiCoin = selectedWithdrawToken.coinType === COIN_TYPES.SUI;

        // For SUI withdrawals: use tx.gas
        if (isSuiCoin) {
          const [withdrawnCoin] = tx.moveCall({
            target: `${XWALLET_PACKAGE_ID}::xwallet::withdraw_coin`,
            typeArguments: [selectedWithdrawToken.coinType],
            arguments: [
              tx.object(suiObjectId),
              tx.pure.u64(amountSmallest),
            ],
          });
          tx.transferObjects([withdrawnCoin], currentAccount.address);
        } else {
          const [withdrawnCoin] = tx.moveCall({
            target: `${XWALLET_PACKAGE_ID}::xwallet::withdraw_coin`,
            typeArguments: [selectedWithdrawToken.coinType],
            arguments: [
              tx.object(suiObjectId),
              tx.pure.u64(amountSmallest),
            ],
          });
          tx.transferObjects([withdrawnCoin], currentAccount.address);
        }

        const txBytes = await tx.build({ client: suiClient });
        const dryRunResult = await suiClient.dryRunTransactionBlock({
          transactionBlock: txBytes,
        });

        const gasUsed = dryRunResult.effects.gasUsed;
        const totalGas =
          BigInt(gasUsed.computationCost) +
          BigInt(gasUsed.storageCost) -
          BigInt(gasUsed.storageRebate);
        const gasSui = (Number(totalGas) / 1_000_000_000).toFixed(6);
        setEstimatedWithdrawGas(gasSui);
      } catch (err) {
        const errorMsg = err instanceof Error ? err.message : '';
        if (errorMsg.includes('No valid gas coins')) {
          setEstimatedWithdrawGas('Need SUI for gas');
        } else {
          setEstimatedWithdrawGas(null);
          setWithdrawGasEstimateError(true);
        }
      } finally {
        setIsEstimatingWithdrawGas(false);
      }
    };

    const debounce = setTimeout(estimateGas, 500);
    return () => clearTimeout(debounce);
  }, [sponsorEnabled, selectedWithdrawToken, withdrawAmount, currentAccount?.address, suiObjectId, suiClient]);

  // Copy to clipboard
  const copyToClipboard = async (text: string, field: string) => {
    await navigator.clipboard.writeText(text);
    setCopiedField(field);
    setTimeout(() => setCopiedField(null), 2000);
  };

  // Handlers
  const handleWithdraw = async () => {
    if (!suiObjectId || !withdrawAmount || !selectedWithdrawToken) return;
    try {
      await withdrawMutation.mutateAsync({
        suiObjectId,
        amount: withdrawAmount,
        coinType: selectedWithdrawToken.coinType,
        decimals: selectedWithdrawToken.decimals,
      });
      setShowWithdrawModal(false);
      setWithdrawAmount('');
      setSelectedWithdrawToken(null);
      setWithdrawType('select');
    } catch {
      // Error is handled by mutation state
    }
  };

  const toggleNFTSelectionForWithdraw = (nft: NFTObject) => {
    setSelectedNFTsForWithdraw((prev) => {
      const isSelected = prev.some((n) => n.objectId === nft.objectId);
      return isSelected ? prev.filter((n) => n.objectId !== nft.objectId) : [...prev, nft];
    });
  };

  const handleWithdrawNFT = async (nft: NFTObject) => {
    if (!suiObjectId) return;
    try {
      await withdrawNFTMutation.mutateAsync({ suiObjectId, nfts: [nft] });
    } catch {
      // Error is handled by mutation state
    }
  };

  const handleWithdrawNFTs = async () => {
    if (!suiObjectId || selectedNFTsForWithdraw.length === 0) return;
    try {
      await withdrawNFTMutation.mutateAsync({ suiObjectId, nfts: selectedNFTsForWithdraw });
      setShowWithdrawModal(false);
      setWithdrawType('select');
      setSelectedNFTsForWithdraw([]);
    } catch {
      // Error is handled by mutation state
    }
  };

  // Link wallet handler
  const handleLinkWallet = async () => {
    if (!currentAccount?.address) return;
    try {
      setLinkWalletSuccess(null);
      const result = await linkWallet(currentAccount.address);
      if (result.success) {
        setLinkWalletSuccess(result.tx_digest || 'Wallet linked successfully!');
      }
    } catch {
      // Error is handled by hook state
    }
  };

  // No XWallet account yet - show initialization flow
  if (!suiObjectId) {
    const tweetCommand = `@NautilusXWallet init`;

    const handleCreateAccount = async () => {
      try {
        setCreateSuccess(null);
        const result = await createAccount();
        if (result.success) {
          setCreateSuccess('Account created successfully!');
          // Reload page after brief delay to show success message
          setTimeout(() => window.location.reload(), 1500);
        }
      } catch {
        // Error is handled by hook state
      }
    };

    return (
      <div className="min-h-screen">
        <Header />
        <div className="max-w-2xl mx-auto px-4 py-16">
          <div className="text-center mb-8">
            <h1 className="text-3xl font-bold text-gray-900 dark:text-white mb-3">Create Your XWallet</h1>
            <p className="text-gray-600 dark:text-gray-400">Welcome @{user?.twitterHandle}! Create your XWallet account to get started.</p>
          </div>
          <div className="glass rounded-2xl p-8">
            {/* Success message */}
            {createSuccess && (
              <div className="mb-6 p-4 rounded-xl bg-cyber-green/10 border border-cyber-green/30">
                <p className="text-cyber-green font-medium">{createSuccess}</p>
              </div>
            )}

            {/* Error message */}
            {createError && (
              <div className="mb-6 p-4 rounded-xl bg-red-500/10 border border-red-500/30">
                <p className="text-red-400">{createError}</p>
              </div>
            )}

            {/* One-click create button */}
            <div className="text-center mb-8">
              <p className="text-gray-600 dark:text-gray-400 mb-4">Click the button below to create your account instantly.</p>
              <button
                onClick={handleCreateAccount}
                disabled={isCreating || !!createSuccess}
                className="btn-sui w-full flex items-center justify-center gap-2 py-4 text-lg disabled:opacity-50"
              >
                {isCreating ? (
                  <>
                    <RefreshCw className="w-5 h-5 animate-spin" />
                    Creating Account...
                  </>
                ) : createSuccess ? (
                  <>
                    <Check className="w-5 h-5" />
                    Account Created
                  </>
                ) : (
                  'Create Account'
                )}
              </button>
            </div>

            {/* Divider */}
            <div className="relative my-6">
              <div className="absolute inset-0 flex items-center">
                <div className="w-full border-t border-gray-300 dark:border-white/10"></div>
              </div>
              <div className="relative flex justify-center text-sm">
                <span className="px-2 bg-white dark:bg-dark-900 text-gray-500">or create via X</span>
              </div>
            </div>

            {/* Alternative: Tweet option */}
            <div className="bg-gray-100 dark:bg-dark-800/50 rounded-xl p-5">
              <div className="flex flex-col md:flex-row md:items-center justify-between gap-3">
                <div className="flex-1">
                  <code className="text-sui-600 dark:text-sui-400 font-medium">{tweetCommand}</code>
                  <p className="text-sm text-gray-600 dark:text-gray-500 mt-1">Create your XWallet account via X</p>
                </div>
                <div className="flex items-center gap-2">
                  <button
                    onClick={() => copyToClipboard(tweetCommand, 'tweet')}
                    className="p-2 rounded-lg hover:bg-gray-200 dark:hover:bg-white/10 transition-colors"
                    title="Copy command"
                  >
                    {copiedField === 'tweet' ? (
                      <Check className="w-4 h-4 text-green-500" />
                    ) : (
                      <Copy className="w-4 h-4 text-gray-500 dark:text-gray-400" />
                    )}
                  </button>
                  <a
                    href={`https://twitter.com/intent/tweet?text=${encodeURIComponent(tweetCommand)}`}
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
          </div>
        </div>
      </div>
    );
  }

  // Account found - show dashboard
  const account = accountData!.account;

  // Avatar: API value with unavatar.io fallback
  const avatarUrl = user?.avatarUrl || `https://unavatar.io/twitter/${account.x_handle}`;

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
                alt={`@${account.x_handle}`}
                className="w-12 h-12 rounded-full object-cover bg-sui-500/20"
              />
              <div>
                <h2 className="text-xl font-bold text-white">@{account.x_handle}</h2>
                <p className="text-sm text-gray-600 dark:text-gray-400">ID: {account.x_user_id}</p>
              </div>
            </div>
            {/* Links */}
            <div className="flex flex-wrap gap-2">
              <a
                href={`https://x.com/${account.x_handle}`}
                target="_blank"
                rel="noopener noreferrer"
                className="btn-glass text-sm flex items-center gap-1"
              >
                <XIcon className="w-4 h-4" />
                Profile
              </a>
              <a
                href={`https://suiscan.xyz/testnet/object/${account.sui_object_id}`}
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
            <p className="text-sm text-gray-600 dark:text-gray-400 mb-2 tracking-wide">Your Balances</p>
            <div className="flex flex-wrap items-end justify-between gap-4">
              {/* Token Balances */}
              <div className="flex flex-wrap gap-4">
                {isLoadingBalance ? (
                  <div className="animate-pulse text-gray-600 dark:text-gray-400 text-2xl">Loading...</div>
                ) : onChainBalances.length > 0 ? (
                  onChainBalances.map((token) => (
                    <div key={token.coin_type} className="flex items-center gap-3 glass rounded-xl px-4 py-3">
                      <TokenIcon symbol={token.symbol} size="lg" />
                      <div>
                        <p className="text-2xl font-bold text-white">{token.balance_formatted}</p>
                        <p className="text-sm text-gray-600 dark:text-gray-400">{token.symbol}</p>
                      </div>
                    </div>
                  ))
                ) : (
                  <div className="flex items-center gap-3 glass rounded-xl px-4 py-3">
                    <TokenIcon symbol="SUI" size="lg" />
                    <div>
                      <p className="text-2xl font-bold text-white">0</p>
                      <p className="text-sm text-gray-600 dark:text-gray-400">SUI</p>
                    </div>
                  </div>
                )}
              </div>

              {/* Action Buttons */}
              <div className="flex gap-2">
                <button
                  onClick={() => setShowDepositModal(true)}
                  disabled={!canDeposit}
                  className="btn-sui flex items-center justify-center gap-2 min-w-[150px] disabled:opacity-50 disabled:cursor-not-allowed"
                >
                  <ArrowDownLeft className="w-4 h-4" />
                  Deposit
                </button>
                <button
                  onClick={() => setShowWithdrawModal(true)}
                  disabled={!canWithdraw}
                  className="btn-glass flex items-center justify-center gap-2 min-w-[150px] disabled:opacity-50 disabled:cursor-not-allowed"
                  title={!canWithdraw ? (!isWalletConnected ? 'Connect your wallet first' : !isWalletLinked ? 'Link your wallet first' : 'Connected wallet does not match linked wallet') : undefined}
                >
                  <ArrowUpRight className="w-4 h-4" />
                  Withdraw
                </button>
              </div>
            </div>
          </div>
        </div>

        {/* Wallet Warning - show when there's a warning*/}
        {walletStateReady && walletWarning && !walletWarningDismissed && (
          <div className="glass rounded-2xl p-4 mb-4 flex items-center gap-3 border-yellow-500/30 bg-yellow-500/5">
            <AlertTriangle className="w-5 h-5 shrink-0 text-yellow-400" />
            <span className="text-sm text-yellow-400 flex-1">{walletWarning}</span>
            <button
              onClick={() => setWalletWarningDismissed(true)}
              className="p-1 hover:bg-white/10 rounded-lg transition-colors"
            >
              <X className="w-4 h-4 text-yellow-400" />
            </button>
          </div>
        )}

        {/* Onboarding Steps - show if not dismissed */}
        {!onboardingDismissed && (
          <div className="glass rounded-2xl p-6 mb-8 border-sui-400/30 bg-sui-400/5">
            <div className="flex items-center justify-between mb-4">
              <h3 className="font-semibold text-gray-900 dark:text-white">Get started with XWallet</h3>
              <button
                onClick={() => {
                  setOnboardingDismissed(true);
                  localStorage.setItem(ONBOARDING_DISMISSED_KEY, 'true');
                }}
                className="p-1 hover:bg-white/10 rounded-lg transition-colors"
                title="Dismiss"
              >
                <X className="w-4 h-4 text-gray-400" />
              </button>
            </div>
            <div className="space-y-3">
              {/* Step 1: Connect Wallet */}
              <div className={`flex items-center gap-3 p-3 rounded-xl transition-colors ${currentAccount ? 'bg-cyber-green/10' : 'bg-white/5'}`}>
                <div className={`w-8 h-8 rounded-full flex items-center justify-center shrink-0 ${currentAccount ? 'bg-cyber-green' : 'bg-gray-200 dark:bg-white/10 text-gray-500'}`}>
                  {currentAccount ? <Check className="w-4 h-4 text-white dark:text-gray-900" strokeWidth={3} /> : <span className="text-sm font-medium">1</span>}
                </div>
                <div className="flex-1">
                  <p className={`font-medium ${currentAccount ? 'text-cyber-green' : 'text-gray-900 dark:text-white'}`}>
                    Connect Wallet
                  </p>
                  <p className="text-sm text-gray-500">Connect your Sui wallet to interact with XWallet</p>
                </div>
                {!currentAccount && (
                  <div className="relative">
                    <button
                      onClick={() => setShowWalletDropdown(!showWalletDropdown)}
                      className="px-4 py-2 bg-sui-400 text-white font-medium rounded-xl hover:bg-sui-500 transition-colors flex items-center gap-2"
                    >
                      <Wallet className="w-4 h-4" />
                      Connect
                      <ChevronDown className={`w-4 h-4 transition-transform ${showWalletDropdown ? 'rotate-180' : ''}`} />
                    </button>
                    {showWalletDropdown && (
                      <>
                        <div
                          className="fixed inset-0 z-40"
                          onClick={() => setShowWalletDropdown(false)}
                        />
                        <div className="absolute right-0 mt-2 w-48 glass rounded-xl p-2 z-50 shadow-xl">
                          {wallets.length === 0 ? (
                            <p className="px-3 py-2 text-sm text-gray-400">No wallets found</p>
                          ) : (
                            wallets.map((wallet) => (
                              <button
                                key={wallet.name}
                                onClick={() => {
                                  connectWallet({ wallet });
                                  setShowWalletDropdown(false);
                                }}
                                className="w-full px-3 py-2 flex items-center gap-2 rounded-lg hover:bg-white/10 transition-colors"
                              >
                                {wallet.icon && <img src={wallet.icon} alt={wallet.name} className="w-5 h-5 rounded" />}
                                <span className="text-gray-900 dark:text-white text-sm">{wallet.name}</span>
                              </button>
                            ))
                          )}
                        </div>
                      </>
                    )}
                  </div>
                )}
              </div>

              {/* Step 2: Link Wallet */}
              <div className={`flex items-center gap-3 p-3 rounded-xl transition-colors ${isWalletLinked ? 'bg-cyber-green/10' : 'bg-white/5'}`}>
                <div className={`w-8 h-8 rounded-full flex items-center justify-center shrink-0 ${isWalletLinked ? 'bg-cyber-green' : 'bg-gray-200 dark:bg-white/10 text-gray-500'}`}>
                  {isWalletLinked ? <Check className="w-4 h-4 text-white dark:text-gray-900" strokeWidth={3} /> : <span className="text-sm font-medium">2</span>}
                </div>
                <div className="flex-1">
                  <p className={`font-medium ${isWalletLinked ? 'text-cyber-green' : 'text-gray-900 dark:text-white'}`}>
                    Link Wallet
                  </p>
                  <p className="text-sm text-gray-500">Link your wallet to enable withdrawals</p>
                </div>
                {!isWalletLinked && currentAccount && (
                  <button
                    onClick={() => setShowLinkWalletModal(true)}
                    className="px-4 py-2 bg-sui-400 text-white font-medium rounded-xl hover:bg-sui-500 transition-colors"
                  >
                    Link
                  </button>
                )}
              </div>

              {/* Step 3: Deposit (Optional) */}
              <div className={`flex items-center gap-3 p-3 rounded-xl transition-colors ${hasDeposited ? 'bg-cyber-green/10' : 'bg-white/5'}`}>
                <div className={`w-8 h-8 rounded-full flex items-center justify-center shrink-0 ${hasDeposited ? 'bg-cyber-green' : 'bg-gray-200 dark:bg-white/10 text-gray-500'}`}>
                  {hasDeposited ? <Check className="w-4 h-4 text-white dark:text-gray-900" strokeWidth={3} /> : <span className="text-sm font-medium">3</span>}
                </div>
                <div className="flex-1">
                  <p className={`font-medium ${hasDeposited ? 'text-cyber-green' : 'text-gray-900 dark:text-white'}`}>
                    Deposit Funds <span className={`text-xs font-normal ${hasDeposited ? 'text-cyber-green/70' : 'text-gray-400'}`}>(Optional)</span>
                  </p>
                  <p className="text-sm text-gray-500">
                    {hasDeposited ? 'You have funds in your XWallet' : 'Deposit tokens or NFTs to your XWallet'}
                  </p>
                </div>
                {currentAccount && isWalletLinked && (
                  <button
                    onClick={() => setShowDepositModal(true)}
                    className={`px-4 py-2 font-medium rounded-xl transition-colors ${hasDeposited ? 'bg-cyber-green/20 text-cyber-green hover:bg-cyber-green/30' : 'bg-white/10 text-white hover:bg-white/20'}`}
                  >
                    {hasDeposited ? 'Deposit More' : 'Deposit'}
                  </button>
                )}
              </div>
            </div>
          </div>
        )}

        {/* Tabs */}
        <div className="glass rounded-2xl overflow-hidden">
          <div className="border-b border-white/5">
            <nav className="flex gap-1 p-2">
              {(['overview', 'activities', 'nfts', 'tweets'] as const).map((tab) => (
                <button
                  key={tab}
                  className={`px-5 py-2.5 rounded-lg font-medium text-sm transition-all flex items-center gap-2 ${
                    activeTab === tab ? 'bg-gray-200 dark:bg-white/10 text-gray-900 dark:text-white' : 'text-gray-600 dark:text-gray-400 hover:text-gray-900 dark:hover:text-white hover:bg-gray-100 dark:hover:bg-white/5'
                  }`}
                  onClick={() => handleTabChange(tab)}
                >
                  {tab === 'overview' && 'Overview'}
                  {tab === 'activities' && 'Activities'}
                  {tab === 'nfts' && 'NFTs'}
                  {tab === 'tweets' && (
                    <>
                      Tweets
                      {pendingTweets.length > 0 && (
                        <span className="px-1.5 py-0.5 text-xs rounded-full bg-yellow-100 dark:bg-yellow-500/20 text-yellow-700 dark:text-yellow-400">
                          {pendingTweets.length}
                        </span>
                      )}
                    </>
                  )}
                </button>
              ))}
            </nav>
          </div>

          <div className="p-6">
            {activeTab === 'overview' && (
              <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                {[
                  { label: 'X User ID', value: account.x_user_id, copyable: true },
                  { label: 'X Handle', value: `@${account.x_handle}`, copyable: false },
                  { label: 'XWallet Account ID', value: account.sui_object_id, copyable: true, mono: true, link: `https://suiscan.xyz/testnet/object/${account.sui_object_id}` },
                  { label: 'Linked Wallet', value: account.owner_address || 'Not linked', copyable: !!account.owner_address, mono: true },
                ].map((item) => (
                  <div key={item.label} className="glass-subtle rounded-xl p-4">
                    <p className="text-sm text-gray-600 dark:text-gray-500 mb-1">{item.label}</p>
                    <div className="flex items-center justify-between">
                      <p className={`text-gray-900 dark:text-white ${item.mono ? 'text-sm break-all' : ''}`}>
                        {item.mono && item.value && item.value.length > 20
                          ? `${item.value.slice(0, 16)}...${item.value.slice(-8)}`
                          : item.value}
                      </p>
                      <div className="flex items-center gap-1">
                        {item.copyable && item.value && (
                          <button onClick={() => copyToClipboard(item.value, item.label)} className="p-1.5 rounded-lg hover:bg-white/10">
                            {copiedField === item.label ? <Check className="w-4 h-4 text-cyber-green" /> : <Copy className="w-4 h-4 text-gray-600 dark:text-gray-400" />}
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
                {isLoadingActivities ? (
                  <div className="text-center py-12">
                    <div className="w-8 h-8 border-2 border-sui-500/30 border-t-sui-500 rounded-full animate-spin mx-auto mb-4" />
                    <p className="text-gray-600 dark:text-gray-400">Loading activities...</p>
                  </div>
                ) : combinedActivities.length === 0 ? (
                  <div className="text-center py-12">
                    <div className="w-16 h-16 mx-auto mb-4 rounded-full bg-gray-100 dark:bg-white/5 flex items-center justify-center">
                      <ArrowLeftRight className="w-8 h-8 text-gray-500 dark:text-gray-500" />
                    </div>
                    <p className="text-gray-600 dark:text-gray-400">No activities yet</p>
                  </div>
                ) : (
                  <>
                    <div className="space-y-3">
                      {combinedActivities.map((activity) => (
                        <ActivityCard
                          key={activity.data.tx_digest}
                          activity={activity}
                          currentXid={accountData?.account?.x_user_id}
                          linkedWallet={accountData?.account?.owner_address}
                          onClick={() => setSelectedActivity(activity)}
                        />
                      ))}
                    </div>
                    {(totalPages > 1 || totalActivities > 0) && (
                      <div className="mt-6 pt-4 border-t border-gray-200 dark:border-white/5">
                        <div className="flex items-center justify-between">
                          <div className="flex items-center gap-3">
                            <PageSizeDropdown value={pageSize} onChange={setPageSize} position="top" />
                            <p className="text-sm text-gray-600 dark:text-gray-500">
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
                {withdrawNFTMutation.isPending && (
                  <div className="mb-4 p-3 glass rounded-lg flex items-center gap-2">
                    <div className="w-4 h-4 border-2 border-sui-400/30 border-t-sui-400 rounded-full animate-spin" />
                    <span className="text-sm text-gray-600 dark:text-gray-400">Withdrawing NFT...</span>
                  </div>
                )}
                {withdrawNFTMutation.error && (
                  <div className="mb-4 p-3 glass rounded-lg border-red-500/30 bg-red-500/10">
                    <p className="text-red-400 text-sm">{withdrawNFTMutation.error instanceof Error ? withdrawNFTMutation.error.message : 'Withdraw failed'}</p>
                  </div>
                )}
                <NFTGrid
                  nfts={accountNFTs}
                  loading={isLoadingAccount}
                  showActions
                  onAction={handleWithdrawNFT}
                  actionLabel="Withdraw"
                  emptyMessage="No NFTs in your X-Wallet"
                />
              </div>
            )}

            {activeTab === 'tweets' && (
              <div>
                {/* Tweet List */}
                {tweets.length === 0 ? (
                  <div className="text-center py-12">
                    <div className="w-16 h-16 mx-auto mb-4 rounded-full bg-gray-100 dark:bg-white/5 flex items-center justify-center">
                      <MessageSquare className="w-8 h-8 text-gray-500 dark:text-gray-500" />
                    </div>
                    <p className="text-gray-600 dark:text-gray-400">No tweet commands yet</p>
                    <p className="text-sm text-gray-600 dark:text-gray-500 mt-2">
                      Mention @NautilusXWallet on X to execute commands
                    </p>
                  </div>
                ) : (
                  <div className="space-y-3">
                    {tweets.map((tweet: TweetStatusData) => {
                      const statusInfo = getTweetStatusInfo(tweet.status);
                      const isProcessing = ['pending', 'processing', 'submitting', 'replying'].includes(tweet.status);
                      return (
                        <div
                          key={tweet.event_id}
                          className={`glass-subtle rounded-xl p-4 transition-all ${isProcessing ? 'ring-1 ring-yellow-500/30' : ''}`}
                        >
                          <div className="flex items-start justify-between gap-4">
                            <div className="flex-1 min-w-0">
                              {/* Status Badge */}
                              <div className="flex items-center gap-2 mb-2">
                                <span className={`inline-flex items-center gap-1.5 px-2.5 py-1 rounded-full text-xs font-medium ${statusInfo.bgColor} ${statusInfo.color}`}>
                                  {isProcessing && (
                                    <div className="w-2 h-2 rounded-full bg-current animate-pulse" />
                                  )}
                                  {statusInfo.label}
                                </span>
                              </div>

                              {/* Tweet Text */}
                              {tweet.text && (
                                <p className="text-gray-900 dark:text-white text-sm mb-2 break-words">
                                  {tweet.text.length > 200
                                    ? `${tweet.text.slice(0, 200)}...`
                                    : tweet.text}
                                </p>
                              )}

                              {/* Author & Link */}
                              <div className="flex items-center gap-3 text-xs text-gray-600 dark:text-gray-500">
                                {tweet.screen_name && (
                                  <span>@{tweet.screen_name}</span>
                                )}
                                {tweet.tweet_id && (
                                  <a
                                    href={`https://x.com/i/web/status/${tweet.tweet_id}`}
                                    target="_blank"
                                    rel="noopener noreferrer"
                                    className="text-sui-400 hover:text-sui-300 flex items-center gap-1"
                                  >
                                    View tweet
                                    <ExternalLink className="w-3 h-3" />
                                  </a>
                                )}
                                {tweet.tx_digest && (
                                  <a
                                    href={`https://suiscan.xyz/testnet/tx/${tweet.tx_digest}`}
                                    target="_blank"
                                    rel="noopener noreferrer"
                                    className="text-sui-400 hover:text-sui-300 flex items-center gap-1"
                                  >
                                    View tx
                                    <ExternalLink className="w-3 h-3" />
                                  </a>
                                )}
                              </div>

                              {/* Error Message */}
                              {tweet.error_message && (
                                <div className="mt-2 p-2 rounded bg-red-500/10 border border-red-500/20">
                                  <p className="text-red-400 text-xs">{tweet.error_message}</p>
                                </div>
                              )}
                            </div>

                            {/* Timestamp */}
                            <div className="text-right text-xs text-gray-600 dark:text-gray-500 whitespace-nowrap">
                              {new Date(tweet.created_at).toLocaleString()}
                            </div>
                          </div>
                        </div>
                      );
                    })}
                  </div>
                )}
              </div>
            )}
          </div>
        </div>
      </main>

      <DepositModal
        isOpen={showDepositModal}
        onClose={() => setShowDepositModal(false)}
        targetHandle={user?.twitterHandle || ''}
        suiObjectId={suiObjectId || ''}
      />

      {/* Withdraw Modal */}
      {showWithdrawModal && (
        <div className="fixed inset-0 bg-black/70 backdrop-blur-sm flex items-center justify-center z-[100]">
          <div className="glass-strong rounded-2xl p-6 w-full max-w-2xl mx-4">
            <div className="flex items-center justify-between mb-6">
              <div className="flex items-center gap-2">
                {withdrawType !== 'select' && (
                  <button onClick={() => setWithdrawType('select')} className="p-1.5 rounded-lg hover:bg-white/10">
                    <ArrowUpRight className="w-4 h-4 text-gray-400 rotate-45" />
                  </button>
                )}
                <h3 className="text-lg font-semibold text-white">
                  {withdrawType === 'select' && 'Withdraw'}
                  {withdrawType === 'tokens' && 'Withdraw Tokens'}
                  {withdrawType === 'nfts' && 'Withdraw NFTs'}
                </h3>
              </div>
              <button onClick={() => { setShowWithdrawModal(false); setWithdrawAmount(''); setWithdrawType('select'); setSelectedNFTsForWithdraw([]); }} className="p-2 rounded-lg hover:bg-white/10">
                <X className="w-5 h-5 text-gray-400" />
              </button>
            </div>

            {withdrawType === 'select' && (
              <div className="space-y-3">
                <p className="text-sm text-gray-600 dark:text-gray-400 mb-4">What would you like to withdraw?</p>
                <button onClick={() => setWithdrawType('tokens')} className="w-full glass glass-hover rounded-xl p-4 text-left group">
                  <div className="flex items-center gap-4">
                    <div className="w-12 h-12 rounded-xl bg-sui-500/20 flex items-center justify-center"><Coins className="w-6 h-6 text-sui-400" /></div>
                    <div><h4 className="font-semibold text-gray-900 dark:text-white">Tokens</h4><p className="text-sm text-gray-600 dark:text-gray-400">Withdraw SUI or other tokens</p></div>
                  </div>
                </button>
                <button onClick={() => setWithdrawType('nfts')} className="w-full glass glass-hover rounded-xl p-4 text-left group">
                  <div className="flex items-center gap-4">
                    <div className="w-12 h-12 rounded-xl bg-cyber-cyan/20 flex items-center justify-center"><Image className="w-6 h-6 text-cyber-cyan" /></div>
                    <div><h4 className="font-semibold text-gray-900 dark:text-white">NFTs</h4><p className="text-sm text-gray-600 dark:text-gray-400">Withdraw NFTs to your wallet</p></div>
                  </div>
                </button>
              </div>
            )}

            {withdrawType === 'tokens' && (
              <div className="space-y-4">
                <div>
                  <label className="block text-sm font-medium text-gray-600 dark:text-gray-400 mb-2">Select Token</label>
                  <div className="relative">
                    <button
                      onClick={() => setShowWithdrawTokenDropdown(!showWithdrawTokenDropdown)}
                      className="w-full bg-gray-100 dark:bg-neutral-900 border border-gray-300 dark:border-neutral-700 rounded-xl px-4 py-3 flex items-center justify-between hover:border-gray-400 dark:hover:border-neutral-600"
                    >
                      {selectedWithdrawToken ? (
                        <div className="flex items-center gap-3">
                          <TokenIcon symbol={selectedWithdrawToken.symbol} size="sm" />
                          <span className="text-gray-900 dark:text-white">{selectedWithdrawToken.symbol}</span>
                          <span className="text-gray-600 dark:text-gray-400 text-sm">Balance: {selectedWithdrawToken.balance}</span>
                        </div>
                      ) : (
                        <span className="text-gray-500 dark:text-gray-400">Select a token</span>
                      )}
                      <ChevronDown className={`w-4 h-4 text-gray-400 transition-transform ${showWithdrawTokenDropdown ? 'rotate-180' : ''}`} />
                    </button>
                    {showWithdrawTokenDropdown && (
                      <div className="absolute z-50 w-full mt-2 bg-white dark:bg-neutral-900 border border-gray-300 dark:border-neutral-700 rounded-xl overflow-hidden max-h-60 overflow-y-auto shadow-lg"
                      >
                        {onChainBalances.length === 0 ? (
                          <div className="p-4 text-center text-gray-600 dark:text-gray-400">No tokens in XWallet</div>
                        ) : (
                          onChainBalances.map((token) => (
                            <button key={token.coin_type} onClick={() => { setSelectedWithdrawToken({ symbol: token.symbol, coinType: token.coin_type, balance: token.balance_formatted, decimals: token.decimals }); setShowWithdrawTokenDropdown(false); setWithdrawAmount(''); }} className="w-full p-3 flex items-center gap-3 hover:bg-gray-100 dark:hover:bg-white/10">
                              <TokenIcon symbol={token.symbol} size="md" />
                              <div className="flex-1 text-left"><p className="text-gray-900 dark:text-white font-medium">{token.symbol}</p></div>
                              <p className="text-gray-700 dark:text-gray-300">{token.balance_formatted}</p>
                            </button>
                          ))
                        )}
                      </div>
                    )}
                  </div>
                </div>
                {selectedWithdrawToken && (
                  <div>
                    <label className="block text-sm font-medium text-gray-600 dark:text-gray-400 mb-2">Amount ({selectedWithdrawToken.symbol})</label>
                    <div className="relative">
                      <input type="number" value={withdrawAmount} onChange={(e) => setWithdrawAmount(e.target.value)} placeholder="0.0" className="input-glass pr-16" />
                      <button onClick={() => setWithdrawAmount(selectedWithdrawToken.balance)} className="absolute right-2 top-1/2 -translate-y-1/2 px-2 py-1 text-xs text-sui-400 hover:text-sui-300">MAX</button>
                    </div>
                    <p className="text-sm text-gray-600 dark:text-gray-500 mt-2">
                      Estimated Gas Fees:{' '}
                      {sponsorEnabled ? (
                        <span className="text-green-400">Sponsored</span>
                      ) : isEstimatingWithdrawGas ? (
                        <span className="text-gray-400">Estimating...</span>
                      ) : withdrawGasEstimateError ? (
                        <span className="text-red-400">Failed to estimate</span>
                      ) : estimatedWithdrawGas === 'Need SUI for gas' ? (
                        <span className="text-yellow-400">{estimatedWithdrawGas}</span>
                      ) : (
                        <span className="text-gray-300">{estimatedWithdrawGas || '0.00'} SUI</span>
                      )}
                    </p>
                  </div>
                )}
                {withdrawMutation.error && <p className="text-red-400 text-sm">{withdrawMutation.error instanceof Error ? withdrawMutation.error.message : 'Withdraw failed'}</p>}
                <div className="flex gap-3 pt-2">
                  <button onClick={() => { setWithdrawType('select'); setSelectedWithdrawToken(null); setWithdrawAmount(''); }} className="flex-1 btn-glass">Back</button>
                  <button onClick={handleWithdraw} disabled={!withdrawAmount || !selectedWithdrawToken || withdrawMutation.isPending} className="flex-1 btn-sui disabled:opacity-40">
                    {withdrawMutation.isPending ? <span className="flex items-center justify-center gap-2"><div className="w-4 h-4 border-2 border-white/30 border-t-white rounded-full animate-spin" />Processing</span> : 'Withdraw'}
                  </button>
                </div>
              </div>
            )}

            {withdrawType === 'nfts' && (
              <div className="space-y-4">
                <p className="text-sm text-gray-600 dark:text-gray-400">Select NFTs from your X-Wallet to withdraw ({selectedNFTsForWithdraw.length} selected)</p>
                <div className="max-h-80 overflow-y-auto">
                  <NFTGrid nfts={accountNFTs} selectedNfts={selectedNFTsForWithdraw} onSelectNft={toggleNFTSelectionForWithdraw} selectable loading={isLoadingAccount} emptyMessage="No NFTs in your X-Wallet" compact />
                </div>
                {selectedNFTsForWithdraw.length > 0 && (
                  <div className="mt-4 pt-4 border-t border-white/10">
                    <p className="text-xs text-gray-600 dark:text-gray-500 mb-2">Selected NFTs:</p>
                    <div className="flex flex-wrap gap-2">
                      {selectedNFTsForWithdraw.map((nft) => (
                        <div key={nft.objectId} className="relative group">
                          <div className="w-12 h-12 rounded-lg overflow-hidden ring-2 ring-sui-500/50">
                            {nft.imageUrl ? <img src={nft.imageUrl} alt={nft.name || 'NFT'} className="w-full h-full object-cover" /> : <div className="w-full h-full bg-white/10 flex items-center justify-center"><Image className="w-5 h-5 text-gray-500" /></div>}
                          </div>
                          <button onClick={() => toggleNFTSelectionForWithdraw(nft)} className="absolute -top-1 -right-1 w-4 h-4 rounded-full bg-black/80 hover:bg-black flex items-center justify-center"><X className="w-2.5 h-2.5 text-white" /></button>
                        </div>
                      ))}
                    </div>
                  </div>
                )}
                {withdrawNFTMutation.error && <p className="text-red-400 text-sm">{withdrawNFTMutation.error instanceof Error ? withdrawNFTMutation.error.message : 'Withdraw failed'}</p>}
                <div className="flex gap-3 pt-2">
                  <button onClick={() => { setWithdrawType('select'); setSelectedNFTsForWithdraw([]); }} className="flex-1 btn-glass">Back</button>
                  <button onClick={handleWithdrawNFTs} disabled={selectedNFTsForWithdraw.length === 0 || withdrawNFTMutation.isPending} className="flex-1 btn-sui disabled:opacity-40">
                    {withdrawNFTMutation.isPending ? <span className="flex items-center justify-center gap-2"><div className="w-4 h-4 border-2 border-white/30 border-t-white rounded-full animate-spin" />Withdrawing</span> : `Withdraw ${selectedNFTsForWithdraw.length > 0 ? `(${selectedNFTsForWithdraw.length})` : ''}`}
                  </button>
                </div>
              </div>
            )}
          </div>
        </div>
      )}

      {/* Transaction Detail Modal */}
      {selectedActivity && (
        <TransactionDetailModal
          activity={selectedActivity}
          currentXid={accountData?.account?.x_user_id}
          shareableUrl={`${window.location.origin}/account/${accountData?.account?.x_user_id}/activities/tx/${selectedActivity.data.tx_digest}`}
          onClose={() => setSelectedActivity(null)}
        />
      )}

      {/* Link Wallet Modal */}
      {showLinkWalletModal && (
        <div className="fixed inset-0 bg-black/70 backdrop-blur-sm flex items-center justify-center z-[100]">
          <div className="glass-strong rounded-2xl p-6 w-full max-w-md mx-4">
            <div className="flex items-center justify-between mb-6">
              <h3 className="text-lg font-semibold text-white">Link Wallet</h3>
              <button
                onClick={() => {
                  setShowLinkWalletModal(false);
                  setLinkWalletSuccess(null);
                }}
                className="p-2 rounded-lg hover:bg-white/10 transition-colors"
              >
                <X className="w-5 h-5 text-gray-400" />
              </button>
            </div>
            <div className="space-y-4">
              <div className="glass-subtle rounded-xl p-4">
                <p className="text-sm text-gray-500 mb-1">X Account</p>
                <p className="font-medium text-white">
                  @{user?.twitterHandle || 'Unknown'}
                </p>
              </div>
              <div className="glass-subtle rounded-xl p-4">
                <p className="text-sm text-gray-500 mb-1">Wallet Address</p>
                <p className="text-sm text-white break-all">
                  {currentAccount?.address || 'Not connected'}
                </p>
              </div>
              <p className="text-sm text-gray-400">
                By linking your wallet, you'll be able to withdraw funds directly to this address.
                You'll need to sign a message to verify ownership.
              </p>
              <div className="glass-subtle rounded-xl p-3 border-gray-500/20">
                <p className="text-xs text-gray-500 mb-2">Alternatively, link via X by posting:</p>
                <div className="flex items-center gap-2">
                  <span className="flex-1 text-xs text-white bg-white/10 px-2 py-1.5 rounded-lg truncate">
                    @NautilusXWallet link wallet {currentAccount?.address ? `${currentAccount.address.slice(0, 6)}...${currentAccount.address.slice(-4)}` : '0x...'}
                  </span>
                  <button
                    onClick={() => copyToClipboard(`@NautilusXWallet link ${currentAccount?.address || ''}`, 'linkCmd')}
                    className="p-1.5 rounded-lg hover:bg-white/10 transition-colors"
                    title="Copy command"
                  >
                    {copiedField === 'linkCmd' ? (
                      <Check className="w-4 h-4 text-cyber-green" />
                    ) : (
                      <Copy className="w-4 h-4 text-gray-400" />
                    )}
                  </button>
                  <a
                    href={`https://twitter.com/intent/post?text=${encodeURIComponent(`@NautilusXWallet link wallet ${currentAccount?.address || ''}`)}`}
                    target="_blank"
                    rel="noopener noreferrer"
                    className="p-1.5 rounded-lg hover:bg-white/10 transition-colors"
                    title="Post on X"
                  >
                    <ExternalLink className="w-4 h-4 text-gray-400" />
                  </a>
                </div>
              </div>
              {linkError && (
                <p className="text-red-400 text-sm">{linkError}</p>
              )}
              {linkWalletSuccess && (
                <div className="glass-subtle rounded-xl p-4 border-cyber-green/30 bg-cyber-green/10">
                  <div className="flex items-center justify-between">
                    <p className="text-cyber-green text-sm font-medium">
                      Wallet linked successfully!
                    </p>
                    {linkWalletSuccess !== 'Wallet linked successfully!' && (
                      <a
                        href={getExplorerUrl(linkWalletSuccess)}
                        target="_blank"
                        rel="noopener noreferrer"
                        className="text-cyber-green/80 hover:text-cyber-green p-1 rounded-lg hover:bg-white/10"
                        title="View transaction"
                      >
                        <ExternalLink className="w-4 h-4" />
                      </a>
                    )}
                  </div>
                </div>
              )}
              <div className="flex gap-3 pt-2">
                <button
                  onClick={() => {
                    setShowLinkWalletModal(false);
                    setLinkWalletSuccess(null);
                  }}
                  className="flex-1 btn-glass"
                >
                  {linkWalletSuccess ? 'Close' : 'Cancel'}
                </button>
                {!linkWalletSuccess && (
                  <button
                    onClick={handleLinkWallet}
                    disabled={isLinking || !currentAccount?.address}
                    className="flex-1 btn-sui disabled:opacity-40 disabled:cursor-not-allowed"
                  >
                    {isLinking ? (
                      <span className="flex items-center justify-center gap-2">
                        <div className="w-4 h-4 border-2 border-white/30 border-t-white rounded-full animate-spin" />
                        Linking
                      </span>
                    ) : (
                      'Link Wallet'
                    )}
                  </button>
                )}
              </div>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};
