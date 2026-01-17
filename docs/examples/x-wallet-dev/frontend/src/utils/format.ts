/**
 * Convert MIST to SUI
 */
export function mistToSui(mist: number | string): string {
  const mistNum = typeof mist === 'string' ? parseInt(mist, 10) : mist;
  const sui = mistNum / 1_000_000_000;
  return sui.toFixed(9).replace(/\.?0+$/, '') || '0';
}

/**
 * Format timestamp to readable date (simple format)
 */
export function formatTimestamp(timestamp: number): string {
  if (!timestamp) return 'Unknown';
  const date = new Date(timestamp);
  return date.toLocaleString();
}

/**
 * Format timestamp to detailed readable date
 */
export function formatDate(timestamp: number): string {
  if (!timestamp) return 'Unknown';
  const date = new Date(timestamp);
  return date.toLocaleDateString('en-US', {
    year: 'numeric',
    month: 'short',
    day: 'numeric',
    hour: '2-digit',
    minute: '2-digit',
  });
}

/**
 * Shorten transaction digest for display
 */
export function shortenDigest(digest: string): string {
  if (!digest || digest.length < 12) return digest;
  return `${digest.slice(0, 6)}...${digest.slice(-6)}`;
}

/**
 * Shorten address/hash for display
 * @param address - The address or hash to shorten
 * @param startLen - Number of characters to show at start (default: 6)
 * @param endLen - Number of characters to show at end (default: 4)
 */
export function shortenAddress(address: string, startLen: number = 6, endLen: number = 4): string {
  if (!address) return '';
  if (address.length <= startLen + endLen + 3) return address;
  return `${address.slice(0, startLen)}...${address.slice(-endLen)}`;
}

/**
 * Get Sui explorer URL for transaction
 */
export function getExplorerUrl(txDigest: string, network: string = 'testnet'): string {
  return `https://suiscan.xyz/${network}/tx/${txDigest}`;
}

/**
 * Get Sui explorer URL for object
 */
export function getExplorerObjectUrl(objectId: string, network: string = 'testnet'): string {
  return `https://suiscan.xyz/${network}/object/${objectId}`;
}
