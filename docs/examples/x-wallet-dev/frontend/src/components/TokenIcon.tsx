import React from 'react';

// Token icons
import suiIcon from '../assets/tokens/sui.png';
import walIcon from '../assets/tokens/wal.png';
import usdcIcon from '../assets/tokens/usdc.png';

interface TokenIconProps {
  symbol: string;
  iconUrl?: string | null;
  size?: 'sm' | 'md' | 'lg';
  className?: string;
}

// Known token icons by symbol
const KNOWN_ICONS: Record<string, string> = {
  SUI: suiIcon,
  WAL: walIcon,
  USDC: usdcIcon,
};

// Generate a consistent color from a string
function stringToColor(str: string): string {
  let hash = 0;
  for (let i = 0; i < str.length; i++) {
    hash = str.charCodeAt(i) + ((hash << 5) - hash);
  }
  const hue = Math.abs(hash % 360);
  return `hsl(${hue}, 70%, 50%)`;
}

export const TokenIcon: React.FC<TokenIconProps> = ({
  symbol,
  iconUrl,
  size = 'md',
  className = '',
}) => {
  const sizeClasses = {
    sm: 'w-6 h-6 text-xs',
    md: 'w-8 h-8 text-sm',
    lg: 'w-12 h-12 text-lg',
  };

  // Try to get icon: custom iconUrl -> known icon -> fallback letter
  const icon = iconUrl || KNOWN_ICONS[symbol.toUpperCase()];

  if (icon) {
    return (
      <img
        src={icon}
        alt={symbol}
        className={`${sizeClasses[size]} rounded-full object-cover ${className}`}
      />
    );
  }

  // Fallback: first letter with colored background
  const bgColor = stringToColor(symbol);

  return (
    <div
      className={`${sizeClasses[size]} rounded-full flex items-center justify-center font-bold text-white ${className}`}
      style={{ backgroundColor: bgColor }}
    >
      {symbol.charAt(0).toUpperCase()}
    </div>
  );
};

export default TokenIcon;
