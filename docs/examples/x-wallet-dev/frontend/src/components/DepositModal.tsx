import React, { useState } from 'react';
import { X, Coins, Image, ChevronDown } from 'lucide-react';
import { TokenIcon } from './TokenIcon';
import { NFTGrid } from './NFTCard';
import { useDeposit } from '../hooks/useXWalletCoinTransactions';
import { useDepositNFT, type NFTObject } from '../hooks/useXWalletNFTTransactions';
import { useConnectedWalletCoins, type WalletCoin } from '../hooks/useConnectedWalletCoins';
import { useConnectedWalletNFTs } from '../hooks/useConnectedWalletNFTs';
import { useGasEstimation } from '../hooks/useGasEstimation';
import { useAppConfig } from '../contexts/AppConfigContext';

interface DepositModalProps {
  isOpen: boolean;
  onClose: () => void;
  targetHandle: string;
  suiObjectId: string;
}

export const DepositModal: React.FC<DepositModalProps> = ({
  isOpen,
  onClose,
  targetHandle,
  suiObjectId,
}) => {
  const [depositType, setDepositType] = useState<'select' | 'tokens' | 'nfts'>('select');
  const [depositAmount, setDepositAmount] = useState('');
  const [selectedDepositToken, setSelectedDepositToken] = useState<WalletCoin | null>(null);
  const [showDepositTokenDropdown, setShowDepositTokenDropdown] = useState(false);
  const [selectedNFTsForDeposit, setSelectedNFTsForDeposit] = useState<NFTObject[]>([]);

  const depositMutation = useDeposit();
  const depositNFTMutation = useDepositNFT();
  const { data: walletCoins = [], isLoading: isLoadingWalletCoins } = useConnectedWalletCoins();
  const { data: walletNFTs = [], isLoading: isLoadingWalletNFTs } = useConnectedWalletNFTs();
  const { sponsorEnabled } = useAppConfig();

  // Gas estimation (only when not sponsored)
  const {
    estimatedGas,
    isEstimating: isEstimatingGas,
    hasError: gasEstimateError,
  } = useGasEstimation({
    coinType: selectedDepositToken?.coinType ?? null,
    amount: depositAmount,
    decimals: selectedDepositToken?.decimals ?? 9,
    suiObjectId,
    enabled: !sponsorEnabled,
  });

  const handleClose = () => {
    setDepositType('select');
    setDepositAmount('');
    setSelectedDepositToken(null);
    setSelectedNFTsForDeposit([]);
    onClose();
  };

  const handleDeposit = async () => {
    if (!suiObjectId || !depositAmount || !selectedDepositToken) return;
    try {
      await depositMutation.mutateAsync({
        suiObjectId,
        amount: depositAmount,
        coinType: selectedDepositToken.coinType,
        decimals: selectedDepositToken.decimals,
      });
      handleClose();
    } catch {
      // Error is handled by mutation state
    }
  };

  const toggleNFTSelectionForDeposit = (nft: NFTObject) => {
    setSelectedNFTsForDeposit((prev) => {
      const isSelected = prev.some((n) => n.objectId === nft.objectId);
      return isSelected ? prev.filter((n) => n.objectId !== nft.objectId) : [...prev, nft];
    });
  };

  const handleDepositNFTs = async () => {
    if (!suiObjectId || selectedNFTsForDeposit.length === 0) return;
    try {
      await depositNFTMutation.mutateAsync({ suiObjectId, nfts: selectedNFTsForDeposit });
      handleClose();
    } catch {
      // Error is handled by mutation state
    }
  };

  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 bg-black/70 backdrop-blur-sm flex items-center justify-center z-[100]">
      <div className="glass-strong rounded-2xl p-6 w-full max-w-2xl mx-4">
        <div className="flex items-center justify-between mb-6">
          <div className="flex items-center gap-2">
            {depositType !== 'select' && (
              <button onClick={() => setDepositType('select')} className="p-1.5 rounded-lg hover:bg-white/10">
                <ChevronDown className="w-4 h-4 text-gray-400 rotate-90" />
              </button>
            )}
            <h3 className="text-lg font-semibold text-white">
              {depositType === 'select' && `Deposit to @${targetHandle}`}
              {depositType === 'tokens' && 'Deposit Tokens'}
              {depositType === 'nfts' && 'Deposit NFTs'}
            </h3>
          </div>
          <button onClick={handleClose} className="p-2 rounded-lg hover:bg-white/10">
            <X className="w-5 h-5 text-gray-400" />
          </button>
        </div>

        {depositType === 'select' && (
          <div className="space-y-3">
            <p className="text-sm text-gray-600 dark:text-gray-400 mb-4">What would you like to deposit?</p>
            <button onClick={() => setDepositType('tokens')} className="w-full glass glass-hover rounded-xl p-4 text-left group">
              <div className="flex items-center gap-4">
                <div className="w-12 h-12 rounded-xl bg-sui-500/20 flex items-center justify-center">
                  <Coins className="w-6 h-6 text-sui-400" />
                </div>
                <div>
                  <h4 className="font-semibold text-gray-900 dark:text-white">Tokens</h4>
                  <p className="text-sm text-gray-600 dark:text-gray-400">Deposit SUI or other tokens</p>
                </div>
              </div>
            </button>
            <button onClick={() => setDepositType('nfts')} className="w-full glass glass-hover rounded-xl p-4 text-left group">
              <div className="flex items-center gap-4">
                <div className="w-12 h-12 rounded-xl bg-cyber-cyan/20 flex items-center justify-center">
                  <Image className="w-6 h-6 text-cyber-cyan" />
                </div>
                <div>
                  <h4 className="font-semibold text-gray-900 dark:text-white">NFTs</h4>
                  <p className="text-sm text-gray-600 dark:text-gray-400">Deposit NFTs to your wallet</p>
                </div>
              </div>
            </button>
          </div>
        )}

        {depositType === 'tokens' && (
          <div className="space-y-4">
            <div>
              <label className="block text-sm font-medium text-gray-600 dark:text-gray-400 mb-2">Select Token</label>
              <div className="relative">
                <button
                  onClick={() => setShowDepositTokenDropdown(!showDepositTokenDropdown)}
                  className="w-full bg-gray-100 dark:bg-neutral-900 border border-gray-300 dark:border-neutral-700 rounded-xl px-4 py-3 flex items-center justify-between hover:border-gray-400 dark:hover:border-neutral-600"
                >
                  {selectedDepositToken ? (
                    <div className="flex items-center gap-3">
                      <TokenIcon symbol={selectedDepositToken.symbol} iconUrl={selectedDepositToken.iconUrl} size="sm" />
                      <span className="text-gray-900 dark:text-white">{selectedDepositToken.symbol}</span>
                      <span className="text-gray-600 dark:text-gray-400 text-sm">Balance: {selectedDepositToken.balanceFormatted}</span>
                    </div>
                  ) : (
                    <span className="text-gray-500 dark:text-gray-400">Select a token</span>
                  )}
                  <ChevronDown className={`w-4 h-4 text-gray-400 transition-transform ${showDepositTokenDropdown ? 'rotate-180' : ''}`} />
                </button>
                {showDepositTokenDropdown && (
                  <div className="absolute z-50 w-full mt-2 bg-white dark:bg-neutral-900 border border-gray-300 dark:border-neutral-700 rounded-xl max-h-60 overflow-y-auto shadow-lg">
                    {isLoadingWalletCoins ? (
                      <div className="p-4 text-center text-gray-600 dark:text-gray-400">Loading tokens...</div>
                    ) : walletCoins.length === 0 ? (
                      <div className="p-4 text-center text-gray-600 dark:text-gray-400">No tokens in wallet</div>
                    ) : (
                      walletCoins.map((coin) => (
                        <button
                          key={coin.coinType}
                          onClick={() => {
                            setSelectedDepositToken(coin);
                            setShowDepositTokenDropdown(false);
                            setDepositAmount('');
                          }}
                          className="w-full p-3 flex items-center gap-3 hover:bg-gray-100 dark:hover:bg-white/10"
                        >
                          <TokenIcon symbol={coin.symbol} iconUrl={coin.iconUrl} size="md" />
                          <div className="flex-1 text-left">
                            <p className="text-gray-900 dark:text-white font-medium">{coin.symbol}</p>
                            <p className="text-gray-600 dark:text-gray-400 text-sm">{coin.name}</p>
                          </div>
                          <p className="text-gray-700 dark:text-gray-300">{coin.balanceFormatted}</p>
                        </button>
                      ))
                    )}
                  </div>
                )}
              </div>
            </div>
            {selectedDepositToken && (
              <div>
                <label className="block text-sm font-medium text-gray-600 dark:text-gray-400 mb-2">
                  Amount ({selectedDepositToken.symbol})
                </label>
                <div className="relative">
                  <input
                    type="number"
                    value={depositAmount}
                    onChange={(e) => setDepositAmount(e.target.value)}
                    placeholder="0.0"
                    className="input-glass pr-16"
                  />
                  <button
                    onClick={() => setDepositAmount(selectedDepositToken.balanceFormatted)}
                    className="absolute right-2 top-1/2 -translate-y-1/2 px-2 py-1 text-xs text-sui-400 hover:text-sui-300"
                  >
                    MAX
                  </button>
                </div>
                <p className="text-sm text-gray-600 dark:text-gray-500 mt-2">
                  Estimated Gas Fees:{' '}
                  {sponsorEnabled ? (
                    <span className="text-green-400">Sponsored</span>
                  ) : isEstimatingGas ? (
                    <span className="text-gray-400">Estimating...</span>
                  ) : gasEstimateError ? (
                    <span className="text-red-400">Failed to estimate</span>
                  ) : estimatedGas === 'Need SUI for gas' ? (
                    <span className="text-yellow-400">{estimatedGas}</span>
                  ) : (
                    <span className="text-gray-300">{estimatedGas || '0.00'} SUI</span>
                  )}
                </p>
              </div>
            )}
            {depositMutation.error && (
              <p className="text-red-400 text-sm">
                {depositMutation.error instanceof Error ? depositMutation.error.message : 'Deposit failed'}
              </p>
            )}
            <div className="flex gap-3 pt-2">
              <button
                onClick={() => {
                  setDepositType('select');
                  setSelectedDepositToken(null);
                  setDepositAmount('');
                }}
                className="flex-1 btn-glass"
              >
                Back
              </button>
              <button
                onClick={handleDeposit}
                disabled={!depositAmount || !selectedDepositToken || depositMutation.isPending}
                className="flex-1 btn-sui disabled:opacity-40"
              >
                {depositMutation.isPending ? (
                  <span className="flex items-center justify-center gap-2">
                    <div className="w-4 h-4 border-2 border-white/30 border-t-white rounded-full animate-spin" />
                    Processing
                  </span>
                ) : (
                  'Deposit'
                )}
              </button>
            </div>
          </div>
        )}

        {depositType === 'nfts' && (
          <div className="space-y-4">
            <p className="text-sm text-gray-600 dark:text-gray-400">
              Select NFTs from your wallet to deposit ({selectedNFTsForDeposit.length} selected)
            </p>
            <div className="max-h-80 overflow-y-auto">
              <NFTGrid
                nfts={walletNFTs}
                selectedNfts={selectedNFTsForDeposit}
                onSelectNft={toggleNFTSelectionForDeposit}
                selectable
                loading={isLoadingWalletNFTs}
                emptyMessage="No NFTs in your wallet"
                compact
              />
            </div>
            {selectedNFTsForDeposit.length > 0 && (
              <div className="mt-4 pt-4 border-t border-white/10">
                <p className="text-xs text-gray-600 dark:text-gray-500 mb-2">Selected NFTs:</p>
                <div className="flex flex-wrap gap-2">
                  {selectedNFTsForDeposit.map((nft) => (
                    <div key={nft.objectId} className="relative group">
                      <div className="w-12 h-12 rounded-lg overflow-hidden ring-2 ring-sui-500/50">
                        {nft.imageUrl ? (
                          <img src={nft.imageUrl} alt={nft.name || 'NFT'} className="w-full h-full object-cover" />
                        ) : (
                          <div className="w-full h-full bg-white/10 flex items-center justify-center">
                            <Image className="w-5 h-5 text-gray-500" />
                          </div>
                        )}
                      </div>
                      <button
                        onClick={() => toggleNFTSelectionForDeposit(nft)}
                        className="absolute -top-1 -right-1 w-4 h-4 rounded-full bg-black/80 hover:bg-black flex items-center justify-center"
                      >
                        <X className="w-2.5 h-2.5 text-white" />
                      </button>
                    </div>
                  ))}
                </div>
              </div>
            )}
            {depositNFTMutation.error && (
              <p className="text-red-400 text-sm">
                {depositNFTMutation.error instanceof Error ? depositNFTMutation.error.message : 'Deposit failed'}
              </p>
            )}
            <div className="flex gap-3 pt-2">
              <button
                onClick={() => {
                  setDepositType('select');
                  setSelectedNFTsForDeposit([]);
                }}
                className="flex-1 btn-glass"
              >
                Back
              </button>
              <button
                onClick={handleDepositNFTs}
                disabled={selectedNFTsForDeposit.length === 0 || depositNFTMutation.isPending}
                className="flex-1 btn-sui disabled:opacity-40"
              >
                {depositNFTMutation.isPending ? (
                  <span className="flex items-center justify-center gap-2">
                    <div className="w-4 h-4 border-2 border-white/30 border-t-white rounded-full animate-spin" />
                    Depositing
                  </span>
                ) : (
                  `Deposit ${selectedNFTsForDeposit.length > 0 ? `(${selectedNFTsForDeposit.length})` : ''}`
                )}
              </button>
            </div>
          </div>
        )}
      </div>
    </div>
  );
};
