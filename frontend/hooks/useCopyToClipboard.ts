import { useCallback, useEffect, useState } from "react";

interface UseCopyToClipboardReturn {
  /** Whether the content was recently copied */
  copied: boolean;
  /** Copy text to clipboard */
  copy: (text: string) => Promise<boolean>;
}

/**
 * Hook for copying text to clipboard with temporary "copied" state.
 *
 * @param resetTimeout - Time in ms before resetting copied state (default: 2000)
 * @returns Copied state and copy function
 *
 * @example
 * ```tsx
 * const { copied, copy } = useCopyToClipboard();
 *
 * const handleCopy = async () => {
 *   const success = await copy("text to copy");
 *   if (success) {
 *     // Handle success
 *   }
 * };
 * ```
 */
export function useCopyToClipboard(resetTimeout = 2000): UseCopyToClipboardReturn {
  const [copied, setCopied] = useState(false);

  const copy = useCallback(async (text: string): Promise<boolean> => {
    try {
      await navigator.clipboard.writeText(text);
      setCopied(true);
      return true;
    } catch (error) {
      console.error("Failed to copy to clipboard:", error);
      return false;
    }
  }, []);

  useEffect(() => {
    if (!copied) return;

    const timer = setTimeout(() => {
      setCopied(false);
    }, resetTimeout);

    return () => clearTimeout(timer);
  }, [copied, resetTimeout]);

  return {
    copied,
    copy,
  };
}
