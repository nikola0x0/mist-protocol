import { useState, useEffect, useCallback } from 'react';
import { API_BASE_URL } from '../utils/constants';

export type TweetStatus = 'pending' | 'processing' | 'submitting' | 'replying' | 'completed' | 'failed';

export interface TweetStatusData {
  event_id: string;
  tweet_id: string | null;
  status: TweetStatus;
  tx_digest: string | null;
  error_message: string | null;
  created_at: string; // RFC3339 string
  updated_at: string; // RFC3339 string
  text: string | null; // Tweet text
  screen_name: string | null; // Author handle
}

interface UseTweetsOptions {
  enabled?: boolean;
}

/**
 * Hook to fetch tweet status data
 * Fetches on mount and when refetch is called
 */
export function useTweetStream(xUserId: string | null | undefined, options: UseTweetsOptions = {}) {
  const { enabled = true } = options;
  const [tweets, setTweets] = useState<TweetStatusData[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Fetch data
  const fetchData = useCallback(async () => {
    if (!xUserId) return;

    setIsLoading(true);
    setError(null);

    try {
      const response = await fetch(`${API_BASE_URL}/api/account/${xUserId}/tweets`);
      if (response.ok) {
        const data = await response.json();
        setTweets(data.tweets || []);
      } else {
        setError('Failed to fetch tweets');
      }
    } catch {
      setError('Failed to fetch tweets');
    } finally {
      setIsLoading(false);
    }
  }, [xUserId]);

  // Fetch on mount and when xUserId changes
  useEffect(() => {
    if (!xUserId || !enabled) {
      setTweets([]);
      return;
    }

    fetchData();
  }, [xUserId, enabled, fetchData]);

  // Get tweets by status
  const pendingTweets = tweets.filter(t => t.status === 'pending' || t.status === 'processing' || t.status === 'submitting' || t.status === 'replying');
  const completedTweets = tweets.filter(t => t.status === 'completed');
  const failedTweets = tweets.filter(t => t.status === 'failed');

  return {
    tweets,
    pendingTweets,
    completedTweets,
    failedTweets,
    isLoading,
    error,
    refetch: fetchData,
  };
}

/**
 * Get display info for tweet status
 */
export function getTweetStatusInfo(status: TweetStatus): { label: string; color: string; bgColor: string } {
  switch (status) {
    case 'pending':
      return { label: 'Pending', color: 'text-yellow-400', bgColor: 'bg-yellow-500/20' };
    case 'processing':
      return { label: 'Processing', color: 'text-blue-400', bgColor: 'bg-blue-500/20' };
    case 'submitting':
      return { label: 'Submitting', color: 'text-purple-400', bgColor: 'bg-purple-500/20' };
    case 'replying':
      return { label: 'Replying', color: 'text-cyan-400', bgColor: 'bg-cyan-500/20' };
    case 'completed':
      return { label: 'Completed', color: 'text-green-400', bgColor: 'bg-green-500/20' };
    case 'failed':
      return { label: 'Failed', color: 'text-red-400', bgColor: 'bg-red-500/20' };
    default:
      return { label: status, color: 'text-gray-400', bgColor: 'bg-gray-500/20' };
  }
}
