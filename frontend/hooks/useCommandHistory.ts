import { useCallback, useEffect, useState } from "react";
import { loadHistory } from "@/lib/history";

interface UseCommandHistoryReturn {
  /** Current history array (readonly) */
  history: readonly string[];
  /** Add a command to history */
  add: (command: string) => void;
  /** Navigate up in history, returns the command or null if at end */
  navigateUp: () => string | null;
  /** Navigate down in history, returns the command or empty string if at beginning */
  navigateDown: () => string;
  /** Reset navigation index (call when user edits input manually) */
  reset: () => void;
  /** Current history index (-1 means not navigating) */
  index: number;
  /** Whether persisted history has been loaded */
  isLoaded: boolean;
}

interface UseCommandHistoryOptions {
  initialHistory?: string[];
  entryType?: "cmd" | "prompt";
  limit?: number;
}

const EMPTY_HISTORY: string[] = [];

/**
 * Hook for managing command history with up/down navigation.
 *
 * Loads persisted history (best-effort) and keeps an in-memory list for navigation.
 */
export function useCommandHistory(options: UseCommandHistoryOptions = {}): UseCommandHistoryReturn {
  const { initialHistory = EMPTY_HISTORY, entryType, limit = 500 } = options;

  const [history, setHistory] = useState<string[]>(initialHistory);
  const [index, setIndex] = useState(-1);
  const [isLoaded, setIsLoaded] = useState(false);

  // Load persisted history on mount (and when the type changes).
  useEffect(() => {
    let cancelled = false;
    setIsLoaded(false);

    loadHistory(limit, entryType)
      .then((entries) => {
        if (cancelled) return;
        setHistory(entries.map((e) => e.c));
        setIndex(-1);
        setIsLoaded(true);
      })
      .catch(() => {
        if (cancelled) return;
        // History is best-effort; ignore failures.
        setHistory(initialHistory);
        setIndex(-1);
        setIsLoaded(true);
      });

    return () => {
      cancelled = true;
    };
  }, [entryType, limit, initialHistory]);

  const add = useCallback((command: string) => {
    if (!command.trim()) return;
    setHistory((prev) => [...prev, command]);
    setIndex(-1);
  }, []);

  const navigateUp = useCallback((): string | null => {
    if (history.length === 0) return null;

    const newIndex = index < history.length - 1 ? index + 1 : index;
    setIndex(newIndex);
    return history[history.length - 1 - newIndex] ?? null;
  }, [history, index]);

  const navigateDown = useCallback((): string => {
    if (index > 0) {
      const newIndex = index - 1;
      setIndex(newIndex);
      return history[history.length - 1 - newIndex] ?? "";
    }
    setIndex(-1);
    return "";
  }, [history, index]);

  const reset = useCallback(() => {
    setIndex(-1);
  }, []);

  return {
    history,
    add,
    navigateUp,
    navigateDown,
    reset,
    index,
    isLoaded,
  };
}
