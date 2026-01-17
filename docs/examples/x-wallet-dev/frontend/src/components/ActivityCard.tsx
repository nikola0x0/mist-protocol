import React from 'react';
import { ArrowDownLeft, ArrowUpRight, ArrowLeftRight, Image, Link } from 'lucide-react';
import { formatTimestamp, shortenAddress } from '../utils/format';
import type { Activity, TransactionData } from '../hooks/useActivitiesStream';

interface ActivityCardProps {
  activity: Activity;
  currentXid?: string;
  linkedWallet?: string | null;
  onClick?: () => void;
}

const getTxIcon = (type: string) => {
  switch (type) {
    case 'coin_deposit':
    case 'nft_deposit':
      return <ArrowDownLeft className="w-5 h-5" />;
    case 'coin_withdraw':
    case 'nft_withdraw':
      return <ArrowUpRight className="w-5 h-5" />;
    case 'nft_transfer':
      return <Image className="w-5 h-5" />;
    case 'link_wallet':
      return <Link className="w-5 h-5" />;
    default:
      return <ArrowLeftRight className="w-5 h-5" />;
  }
};

const getTxColor = (type: string) => {
  switch (type) {
    case 'coin_deposit':
    case 'nft_deposit':
      return 'text-cyber-green bg-cyber-green/20';
    case 'coin_withdraw':
    case 'nft_withdraw':
      return 'text-red-400 bg-red-500/20';
    case 'link_wallet':
      return 'text-purple-400 bg-purple-500/20';
    default:
      return 'text-sui-400 bg-sui-500/20';
  }
};

const getActivityLabel = (type: string) => {
  switch (type) {
    case 'coin_deposit':
      return 'Deposit';
    case 'coin_withdraw':
      return 'Withdraw';
    case 'coin_transfer':
      return 'Transfer';
    case 'nft_deposit':
      return 'NFT Deposit';
    case 'nft_withdraw':
      return 'NFT Withdraw';
    case 'nft_transfer':
      return 'NFT Transfer';
    case 'link_wallet':
      return 'Link Wallet';
    default:
      return type;
  }
};

const getPartyInfo = (
  tx: TransactionData,
  currentXid?: string
): { partyInfo: string; isIncoming: boolean; isOutgoing: boolean } => {
  const isIncoming = tx.tx_type === 'coin_deposit' || tx.tx_type === 'nft_deposit' ||
    ((tx.tx_type === 'coin_transfer' || tx.tx_type === 'nft_transfer') && tx.to_id === currentXid);
  const isOutgoing = tx.tx_type === 'coin_withdraw' || tx.tx_type === 'nft_withdraw' ||
    ((tx.tx_type === 'coin_transfer' || tx.tx_type === 'nft_transfer') && tx.from_id === currentXid);

  let partyInfo = '';

  if (tx.tx_type === 'link_wallet') {
    // Show wallet address change for link wallet
    if (tx.from_address && tx.to_address) {
      partyInfo = `${shortenAddress(tx.from_address)} → ${shortenAddress(tx.to_address)}`;
    } else if (tx.to_address) {
      partyInfo = shortenAddress(tx.to_address);
    }
  } else if (tx.tx_type === 'coin_deposit' || tx.tx_type === 'nft_deposit') {
    // Deposit: from_id contains external address (starts with 0x)
    const depositFrom = tx.from_id?.startsWith('0x') ? tx.from_id : null;
    partyInfo = depositFrom ? `from ${shortenAddress(depositFrom)}` : 'from wallet';
  } else if (tx.tx_type === 'coin_withdraw' || tx.tx_type === 'nft_withdraw') {
    // Withdraw: to_id contains external address (starts with 0x)
    const withdrawTo = tx.to_id?.startsWith('0x') ? tx.to_id : null;
    partyInfo = withdrawTo ? `to ${shortenAddress(withdrawTo)}` : 'to wallet';
  } else if (tx.tx_type === 'coin_transfer' || tx.tx_type === 'nft_transfer') {
    if (isIncoming) {
      partyInfo = tx.from_handle ? `from @${tx.from_handle}` : tx.from_id ? 'from X user' : '';
    } else if (isOutgoing) {
      if (tx.to_address) {
        partyInfo = `to ${shortenAddress(tx.to_address)}`;
      } else {
        partyInfo = tx.to_handle ? `to @${tx.to_handle}` : tx.to_id ? 'to X user' : '';
      }
    }
  }

  return { partyInfo, isIncoming, isOutgoing };
};

export const ActivityCard: React.FC<ActivityCardProps> = ({
  activity,
  currentXid,
  onClick,
}) => {
  const tx = activity.data;
  const { partyInfo, isIncoming, isOutgoing } = getPartyInfo(tx, currentXid);

  // Determine display values based on activity type
  const isLinkWallet = activity.type === 'link_wallet';
  const isNft = activity.type === 'nft';
  const isCoin = activity.type === 'coin';

  return (
    <button
      onClick={onClick}
      className="w-full flex items-center justify-between p-4 bg-gray-50 dark:bg-white/5 rounded-xl hover:bg-gray-100 dark:hover:bg-white/10 transition-all cursor-pointer text-left"
    >
      <div className="flex items-center gap-4">
        <div
          className={`w-10 h-10 rounded-xl flex items-center justify-center ${
            isLinkWallet
              ? getTxColor('link_wallet')
              : isIncoming
              ? 'text-cyber-green bg-cyber-green/20'
              : isOutgoing
              ? 'text-red-400 bg-red-500/20'
              : getTxColor(tx.tx_type)
          }`}
        >
          {getTxIcon(tx.tx_type)}
        </div>
        <div>
          <p className="font-medium text-gray-900 dark:text-white">{getActivityLabel(tx.tx_type)}</p>
          <p className="text-sm text-gray-500">{partyInfo}</p>
        </div>
      </div>
      <div className="text-right">
        {isLinkWallet ? (
          <p className="font-semibold text-purple-400">Wallet Linked</p>
        ) : isNft ? (
          <p className={`font-semibold ${isIncoming ? 'text-cyber-green' : 'text-red-400'}`}>
            {isIncoming ? '+' : '-'} {tx.nft_name || 'NFT'}
          </p>
        ) : isCoin && tx.amount ? (
          <p
            className={`font-semibold ${
              isIncoming ? 'text-cyber-green' : isOutgoing ? 'text-red-400' : 'text-gray-900 dark:text-white'
            }`}
          >
            {isIncoming ? '+' : isOutgoing ? '-' : ''}
            {tx.amount} {tx.coin_type?.split('::').pop() || 'SUI'}
          </p>
        ) : null}
        <p className="text-sm text-gray-500">{formatTimestamp(tx.timestamp)}</p>
      </div>
    </button>
  );
};
