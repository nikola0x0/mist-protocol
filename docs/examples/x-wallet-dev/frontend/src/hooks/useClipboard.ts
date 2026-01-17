import { useState, useCallback } from 'react';

interface UseClipboardOptions {
  timeout?: number;
}

interface UseClipboardReturn {
  copied: boolean;
  copiedField: string | null;
  copy: (text: string, field?: string) => Promise<boolean>;
  reset: () => void;
}

/**
 * Hook for copying text to clipboard with feedback state
 * @param options - Configuration options
 * @param options.timeout - Time in ms before resetting copied state (default: 2000)
 */
export function useClipboard(options: UseClipboardOptions = {}): UseClipboardReturn {
  const { timeout = 2000 } = options;
  const [copied, setCopied] = useState(false);
  const [copiedField, setCopiedField] = useState<string | null>(null);

  const copy = useCallback(async (text: string, field?: string): Promise<boolean> => {
    if (!text) return false;

    try {
      await navigator.clipboard.writeText(text);
      setCopied(true);
      setCopiedField(field || null);

      setTimeout(() => {
        setCopied(false);
        setCopiedField(null);
      }, timeout);

      return true;
    } catch {
      return false;
    }
  }, [timeout]);

  const reset = useCallback(() => {
    setCopied(false);
    setCopiedField(null);
  }, []);

  return { copied, copiedField, copy, reset };
}
