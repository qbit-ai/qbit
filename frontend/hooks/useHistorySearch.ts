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
    const lowerQuery = query.toLowerCase();
    const results: HistoryMatch[] = [];
    const seen = new Set<string>();

    // Iterate from newest to oldest, skipping duplicates
    for (let i = history.length - 1; i >= 0; i--) {
      const command = history[i];
      // Skip if we've already seen this exact command
      if (seen.has(command)) {
        continue;
      }
      seen.add(command);
      // If no query, include all; otherwise filter by substring match
      if (!query || command.toLowerCase().includes(lowerQuery)) {
        results.push({ command, index: i });
      }
    }

    // Reverse so oldest appears at top, most recent at bottom (near input)
    return results.reverse();
  }, [history, query]);

  return { matches };
}
