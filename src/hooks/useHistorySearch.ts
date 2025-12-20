import { useMemo } from "react";

interface UseHistorySearchOptions {
  history: readonly string[];
  query: string;
}

export interface HistoryMatch {
  command: string;
  index: number;
}

/**
 * Hook for filtering command history by substring match (case-insensitive).
 * Returns matches sorted by recency (most recent first).
 *
 * @param history - Command history array (oldest to newest)
 * @param query - Search query string
 * @returns Array of matching commands with their indices
 *
 * @example
 * ```tsx
 * const { matches } = useHistorySearch({ history, query: "git" });
 * // Returns commands containing "git", most recent first
 * ```
 */
export function useHistorySearch({ history, query }: UseHistorySearchOptions): {
  matches: HistoryMatch[];
} {
  const matches = useMemo(() => {
    if (!query) {
      return [];
    }

    const lowerQuery = query.toLowerCase();
    const results: HistoryMatch[] = [];

    // Iterate from newest to oldest
    for (let i = history.length - 1; i >= 0; i--) {
      const command = history[i];
      if (command.toLowerCase().includes(lowerQuery)) {
        results.push({ command, index: i });
      }
    }

    return results;
  }, [history, query]);

  return { matches };
}
