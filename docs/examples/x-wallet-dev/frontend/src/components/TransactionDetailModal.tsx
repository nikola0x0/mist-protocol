import React from 'react';
import {
  X,
  ArrowDownLeft,
  ArrowUpRight,
  ArrowRight,
  CheckCircle,
  Check,
  Link2,
  Link,
  Copy,
} from 'lucide-react';
import { useSuiClient } from '@mysten/dapp-kit';
import { useQuery } from '@tanstack/react-query';
import { TokenIcon } from './TokenIcon';
import { useClipboard } from '../hooks/useClipboard';
import { shortenAddress, formatDate, getExplorerUrl } from '../utils/format';
import type { Activity } from '../hooks/useActivitiesStream';

interface TransactionDetailModalProps {
  activity: Activity;
  currentXid?: string;
  shareableUrl?: string;
  onClose: () => void;
}

// Helper to get token symbol from coin_type
const getTokenSymbol = (coinType: string | null): string => {
  if (!coinType) return 'TOKEN';
  const parts = coinType.split('::');
  return parts[parts.length - 1] || 'TOKEN';
};

export const TransactionDetailModal: React.FC<TransactionDetailModalProps> = ({
  activity,
  currentXid,
  shareableUrl,
  onClose,
}) => {
  const { copied, copy, copiedField } = useClipboard();
  const suiClient = useSuiClient();

  const tx = activity.data;
  const txDigest = tx.tx_digest;

  // Fetch and cache gas info from blockchain
  const { data: gasInfo } = useQuery({
    queryKey: ['tx-gas-info', txDigest],
    queryFn: async () => {
      const txBlock = await suiClient.getTransactionBlock({
        digest: txDigest,
        options: { showEffects: true, showInput: true },
      });

      // Check if sponsored: gas payer != sender
      const sender = txBlock.transaction?.data.sender;
      const gasOwner = txBlock.transaction?.data.gasData.owner;
      const isSponsored = sender !== gasOwner;

      if (txBlock.effects?.gasUsed) {
        const { computationCost, storageCost, storageRebate } = txBlock.effects.gasUsed;
        const totalGas = BigInt(computationCost) + BigInt(storageCost) - BigInt(storageRebate);
        const gasSui = (Number(totalGas) / 1_000_000_000).toFixed(6);
        return { sponsored: isSponsored, amount: gasSui };
      }
      return null;
    },
    enabled: !!txDigest,
    staleTime: Infinity, // Never refetch - gas info doesn't change
    gcTime: 1000 * 60 * 30, // Keep in cache for 30 minutes
  });

  // Handle click outside to close
  const handleBackdropClick = (e: React.MouseEvent<HTMLDivElement>) => {
    if (e.target === e.currentTarget) {
      onClose();
    }
  };

  // Determine transaction type and direction
  const isLinkWallet = activity.type === 'link_wallet';
  const isNft = activity.type === 'nft';
  const isCoin = activity.type === 'coin';

  const isIncoming = tx.tx_type === 'coin_deposit' || tx.tx_type === 'nft_deposit' ||
    ((tx.tx_type === 'coin_transfer' || tx.tx_type === 'nft_transfer') && tx.to_id === currentXid);
  const isOutgoing = tx.tx_type === 'coin_withdraw' || tx.tx_type === 'nft_withdraw' ||
    ((tx.tx_type === 'coin_transfer' || tx.tx_type === 'nft_transfer') && tx.from_id === currentXid);

  let icon: React.ReactNode;
  let iconBgClass: string;
  let amountColorClass: string;
  let amountPrefix: string;
  let typeLabel: string;

  if (isLinkWallet) {
    icon = <Link className="w-6 h-6" />;
    iconBgClass = 'bg-purple-500/20 text-purple-400';
    amountColorClass = 'text-purple-400';
    amountPrefix = '';
    typeLabel = 'Link Wallet';
  } else if (tx.tx_type === 'coin_deposit' || tx.tx_type === 'nft_deposit') {
    icon = <ArrowDownLeft className="w-6 h-6" />;
    iconBgClass = 'bg-green-500/20 text-green-400';
    amountColorClass = 'text-green-400';
    amountPrefix = '+';
    typeLabel = 'Deposited';
  } else if (tx.tx_type === 'coin_withdraw' || tx.tx_type === 'nft_withdraw') {
    icon = <ArrowUpRight className="w-6 h-6" />;
    iconBgClass = 'bg-red-500/20 text-red-400';
    amountColorClass = 'text-red-400';
    amountPrefix = '-';
    typeLabel = 'Withdrawn';
  } else if (isIncoming) {
    icon = <ArrowDownLeft className="w-6 h-6" />;
    iconBgClass = 'bg-green-500/20 text-green-400';
    amountColorClass = 'text-green-400';
    amountPrefix = '+';
    typeLabel = 'Received';
  } else if (isOutgoing) {
    icon = <ArrowUpRight className="w-6 h-6" />;
    iconBgClass = 'bg-red-500/20 text-red-400';
    amountColorClass = 'text-red-400';
    amountPrefix = '-';
    typeLabel = 'Sent';
  } else {
    icon = <ArrowRight className="w-6 h-6" />;
    iconBgClass = 'bg-sui-500/20 text-sui-400';
    amountColorClass = 'text-white';
    amountPrefix = '';
    typeLabel = 'Transfer';
  }

  // Helper to format user display (handle or XID)
  const formatUser = (xid: string | null, handle: string | null, isCurrentUser: boolean): string => {
    if (!xid) return 'Unknown';
    if (handle) {
      return isCurrentUser ? `@${handle} (you)` : `@${handle}`;
    }
    // Fallback: if no handle available, show shortened XID
    return `X user (${xid.slice(-6)})`;
  };

  return (
    <div
      className="fixed inset-0 bg-black/70 backdrop-blur-sm flex items-center justify-center z-[100]"
      onClick={handleBackdropClick}
    >
      <div className="glass-strong rounded-2xl w-full max-w-md mx-4 overflow-hidden max-h-[90vh] overflow-y-auto">
        {/* Header */}
        <div className="flex items-center justify-between p-4 border-b border-white/10">
          <h3 className="text-base font-semibold text-white">Transaction Details</h3>
          <div className="flex items-center gap-1">
            {shareableUrl && (
              <button
                onClick={() => copy(shareableUrl)}
                className="p-1.5 rounded-lg hover:bg-white/10 transition-colors"
                title="Copy shareable link"
              >
                {copied ? (
                  <Check className="w-5 h-5 text-cyber-green" />
                ) : (
                  <Link2 className="w-5 h-5 text-gray-400" />
                )}
              </button>
            )}
            <button
              onClick={onClose}
              className="p-1.5 rounded-lg hover:bg-white/10 transition-colors"
            >
              <X className="w-5 h-5 text-gray-400" />
            </button>
          </div>
        </div>

        {/* Content */}
        <div className="p-4 space-y-4">
          {/* Status */}
          <div className="text-center">
            <div className="flex items-center justify-center gap-2 text-green-400">
              <CheckCircle className="w-4 h-4" />
              <span className="text-sm font-medium">Successful</span>
            </div>
          </div>

          {/* Amount / NFT / Wallet Display */}
          <div className="text-center py-3">
            <div className={`inline-flex items-center justify-center w-14 h-14 rounded-full ${iconBgClass} mb-3`}>
              {icon}
            </div>
            {isLinkWallet ? (
              <p className={`text-xl font-bold ${amountColorClass} mb-1`}>
                Wallet Linked
              </p>
            ) : isNft ? (
              <>
                <p className={`text-xl font-bold ${amountColorClass} mb-1`}>
                  {amountPrefix} NFT
                </p>
                <p className="text-gray-400 text-sm">{tx.nft_name || 'NFT'}</p>
              </>
            ) : isCoin && tx.amount ? (
              <>
                <div className="flex items-center justify-center gap-2 mb-1">
                  <TokenIcon symbol={getTokenSymbol(tx.coin_type)} size="md" />
                  <span className={`text-2xl font-bold ${amountColorClass}`}>
                    {amountPrefix}{tx.amount}
                  </span>
                </div>
                <p className="text-gray-400 text-sm">{getTokenSymbol(tx.coin_type)}</p>
              </>
            ) : null}
          </div>

          {/* Details */}
          <div className="space-y-0">
            {/* Type */}
            <div className="flex items-center justify-between py-2.5 border-b border-white/5">
              <span className="text-gray-400 text-sm">Type</span>
              <span className="text-white text-sm font-medium">{typeLabel}</span>
            </div>

            {/* Date */}
            <div className="flex items-center justify-between py-2.5 border-b border-white/5">
              <span className="text-gray-400 text-sm">Date</span>
              <span className="text-white text-sm">{formatDate(tx.timestamp)}</span>
            </div>

            {/* From (for link wallet) */}
            {isLinkWallet && tx.from_address && (
              <div className="flex items-center justify-between py-2.5 border-b border-white/5">
                <span className="text-gray-400 text-sm">From</span>
                <button
                  onClick={() => copy(tx.from_address!, 'link-from')}
                  className="flex items-center gap-1.5 text-white text-xs hover:text-sui-400 transition-colors"
                  title="Click to copy"
                >
                  {shortenAddress(tx.from_address)}
                  {copied && copiedField === 'link-from' ? (
                    <Check className="w-3 h-3 text-green-400" />
                  ) : (
                    <Copy className="w-3 h-3 text-gray-500" />
                  )}
                </button>
              </div>
            )}

            {/* To (for link wallet) */}
            {isLinkWallet && tx.to_address && (
              <div className="flex items-center justify-between py-2.5 border-b border-white/5">
                <span className="text-gray-400 text-sm">To</span>
                <button
                  onClick={() => copy(tx.to_address!, 'link-to')}
                  className="flex items-center gap-1.5 text-white text-xs hover:text-sui-400 transition-colors"
                  title="Click to copy"
                >
                  {shortenAddress(tx.to_address)}
                  {copied && copiedField === 'link-to' ? (
                    <Check className="w-3 h-3 text-green-400" />
                  ) : (
                    <Copy className="w-3 h-3 text-gray-500" />
                  )}
                </button>
              </div>
            )}

            {/* From (for deposits - show wallet address) */}
            {(tx.tx_type === 'coin_deposit' || tx.tx_type === 'nft_deposit') && tx.from_address && (
              <div className="flex items-center justify-between py-2.5 border-b border-white/5">
                <span className="text-gray-400 text-sm">From</span>
                <span className="text-white text-xs">
                  {shortenAddress(tx.from_address)}
                </span>
              </div>
            )}

            {/* To (for withdrawals - show wallet address) */}
            {(tx.tx_type === 'coin_withdraw' || tx.tx_type === 'nft_withdraw') && tx.to_address && (
              <div className="flex items-center justify-between py-2.5 border-b border-white/5">
                <span className="text-gray-400 text-sm">To</span>
                <span className="text-white text-xs">
                  {shortenAddress(tx.to_address)}
                </span>
              </div>
            )}

            {/* From (for transfers) */}
            {(tx.tx_type === 'coin_transfer' || tx.tx_type === 'nft_transfer') && tx.from_id && (
              <div className="flex items-center justify-between py-2.5 border-b border-white/5">
                <span className="text-gray-400 text-sm">From</span>
                <span className="text-white text-sm">
                  {formatUser(tx.from_id, tx.from_handle, tx.from_id === currentXid)}
                </span>
              </div>
            )}

            {/* To (for transfers) */}
            {(tx.tx_type === 'coin_transfer' || tx.tx_type === 'nft_transfer') && (tx.to_id || tx.to_address) && (
              <div className="flex items-center justify-between py-2.5 border-b border-white/5">
                <span className="text-gray-400 text-sm">To</span>
                <span className="text-white text-sm">
                  {tx.to_address
                    ? shortenAddress(tx.to_address)
                    : formatUser(tx.to_id, tx.to_handle, tx.to_id === currentXid)}
                </span>
              </div>
            )}

            {/* NFT Object ID */}
            {isNft && tx.nft_object_id && (
              <div className="flex items-center justify-between py-2.5 border-b border-white/5">
                <span className="text-gray-400 text-sm">NFT ID</span>
                <span className="text-white text-xs">
                  {shortenAddress(tx.nft_object_id)}
                </span>
              </div>
            )}

            {/* Gas Fee */}
            <div className="flex items-center justify-between py-2.5">
              <span className="text-gray-400 text-sm">Gas Fee</span>
              {!gasInfo ? (
                <span className="text-gray-400 text-sm">Loading...</span>
              ) : gasInfo.sponsored ? (
                <span className="text-green-400 text-sm">Sponsored</span>
              ) : (
                <span className="text-gray-300 text-sm">{gasInfo.amount} SUI</span>
              )}
            </div>
          </div>

          {/* Action Buttons */}
          <div className="flex gap-2 pt-2">
            {/* Tweet Link (if available) */}
            {tx.tweet_id && (
              <a
                href={`https://x.com/i/status/${tx.tweet_id}`}
                target="_blank"
                rel="noopener noreferrer"
                className="flex-1 btn-glass flex items-center justify-center gap-2 text-sm py-2.5"
              >
                View Tweet
              </a>
            )}

            {/* Explorer Link */}
            <a
              href={getExplorerUrl(tx.tx_digest)}
              target="_blank"
              rel="noopener noreferrer"
              className="flex-1 btn-sui flex items-center justify-center gap-2 text-sm py-2.5"
            >
              View on Explorer
            </a>
          </div>
        </div>
      </div>
    </div>
  );
};
