import React, { useState } from 'react';
import { Check, Image as ImageIcon, Copy } from 'lucide-react';
import type { NFTObject } from '../hooks/useXWalletNFTTransactions';

interface NFTCardProps {
  nft: NFTObject;
  selected?: boolean;
  onSelect?: (nft: NFTObject) => void;
  selectable?: boolean;
  showActions?: boolean;
  onAction?: (nft: NFTObject) => void;
  actionLabel?: string;
  compact?: boolean;
}

export const NFTCard: React.FC<NFTCardProps> = ({
  nft,
  selected = false,
  onSelect,
  selectable = false,
  showActions = false,
  onAction,
  actionLabel = 'Select',
  compact = false,
}) => {
  const [copied, setCopied] = useState(false);

  const handleClick = () => {
    if (selectable && onSelect) {
      onSelect(nft);
    }
  };

  const handleCopyId = (e: React.MouseEvent) => {
    e.stopPropagation();
    navigator.clipboard.writeText(nft.objectId);
    setCopied(true);
    setTimeout(() => setCopied(false), 1500);
  };

  // Truncate type for display
  const shortType = nft.type
    ? nft.type.split('::').slice(-1)[0] || 'NFT'
    : 'NFT';

  return (
    <div
      onClick={handleClick}
      className={`
        relative glass rounded-lg transition-all duration-200 group
        ${selectable ? 'cursor-pointer hover:border-sui-400/50' : ''}
        ${selected ? 'border-sui-400 ring-2 ring-sui-400/30' : ''}
      `}
    >
      {/* Selection indicator */}
      {selected && (
        <div className="absolute top-1.5 right-1.5 z-10 w-5 h-5 rounded-full bg-sui-500 flex items-center justify-center">
          <Check className="w-3 h-3 text-white" />
        </div>
      )}

      {/* NFT Image */}
      <div className="aspect-square bg-dark-800 relative overflow-hidden rounded-t-lg">
        {nft.imageUrl ? (
          <img
            src={nft.imageUrl}
            alt={nft.name || 'NFT'}
            className="w-full h-full object-cover"
            onError={(e) => {
              (e.target as HTMLImageElement).style.display = 'none';
              (e.target as HTMLImageElement).nextElementSibling?.classList.remove('hidden');
            }}
          />
        ) : null}
        <div className={`absolute inset-0 flex items-center justify-center ${nft.imageUrl ? 'hidden' : ''}`}>
          <ImageIcon className={compact ? "w-8 h-8" : "w-10 h-10"} />
        </div>

        {/* Hover overlay with NFT info */}
        <div className="absolute inset-0 bg-black/80 opacity-0 group-hover:opacity-100 transition-opacity flex flex-col justify-end p-2.5 shadow-lg">
          <p className="text-white font-medium text-sm truncate">{nft.name || 'Unknown NFT'}</p>
          <p className="text-gray-300 text-xs truncate mt-0.5">{shortType}</p>
          <div className="flex items-center gap-1.5 mt-1.5">
            <p className="text-gray-400 text-[10px] truncate flex-1">
              {nft.objectId.slice(0, 8)}...{nft.objectId.slice(-6)}
            </p>
            <button
              onClick={handleCopyId}
              className="p-1 rounded hover:bg-white/20 transition-colors flex-shrink-0"
              title="Copy NFT ID"
            >
              {copied ? (
                <Check className="w-3.5 h-3.5 text-green-400" />
              ) : (
                <Copy className="w-3.5 h-3.5 text-gray-400" />
              )}
            </button>
          </div>
          {nft.description && (
            <p className="text-gray-400 text-[10px] mt-1 line-clamp-2">{nft.description}</p>
          )}
        </div>
      </div>

      {/* NFT Info */}
      <div className={compact ? "p-2.5" : "p-3"}>
        <h4 className={`text-white font-medium truncate ${compact ? "text-sm" : "text-sm"}`}>
          {nft.name || 'Unknown NFT'}
        </h4>
        <p className={`text-gray-500 truncate ${compact ? "text-xs" : "text-xs"}`}>
          {shortType}
        </p>

        {/* Action button */}
        {showActions && onAction && (
          <button
            onClick={(e) => {
              e.stopPropagation();
              onAction(nft);
            }}
            className="mt-2 w-full bg-sui-gradient text-white text-xs py-1.5 rounded-lg font-medium hover:shadow-glow-sm hover:scale-[1.02] active:scale-[0.98] transition-all duration-200"
          >
            {actionLabel}
          </button>
        )}
      </div>
    </div>
  );
};

interface NFTGridProps {
  nfts: NFTObject[];
  selectedNfts?: NFTObject[];
  onSelectNft?: (nft: NFTObject) => void;
  selectable?: boolean;
  showActions?: boolean;
  onAction?: (nft: NFTObject) => void;
  actionLabel?: string;
  emptyMessage?: string;
  loading?: boolean;
  compact?: boolean; // Use smaller cards (for modals)
}

export const NFTGrid: React.FC<NFTGridProps> = ({
  nfts,
  selectedNfts = [],
  onSelectNft,
  selectable = false,
  showActions = false,
  onAction,
  actionLabel,
  emptyMessage = 'No NFTs found',
  loading = false,
  compact = false,
}) => {
  const gridClass = compact
    ? "grid grid-cols-3 gap-2"
    : "grid grid-cols-3 sm:grid-cols-4 lg:grid-cols-5 gap-3";

  if (loading) {
    return (
      <div className={gridClass}>
        {[...Array(compact ? 3 : 5)].map((_, i) => (
          <div key={i} className="glass rounded-lg overflow-hidden animate-pulse">
            <div className="aspect-square bg-dark-700" />
            <div className={compact ? "p-2.5" : "p-3"}>
              <div className="h-4 bg-dark-700 rounded w-3/4 mb-1.5" />
              <div className="h-3 bg-dark-700 rounded w-1/2" />
            </div>
          </div>
        ))}
      </div>
    );
  }

  if (nfts.length === 0) {
    return (
      <div className="text-center py-8 glass rounded-xl">
        <ImageIcon className="w-10 h-10 text-gray-600 mx-auto mb-2" />
        <p className="text-gray-400 text-sm">{emptyMessage}</p>
      </div>
    );
  }

  return (
    <div className={gridClass}>
      {nfts.map((nft) => (
        <NFTCard
          key={nft.objectId}
          nft={nft}
          selected={selectedNfts.some((n) => n.objectId === nft.objectId)}
          onSelect={onSelectNft}
          selectable={selectable}
          showActions={showActions}
          onAction={onAction}
          actionLabel={actionLabel}
          compact={compact}
        />
      ))}
    </div>
  );
};
