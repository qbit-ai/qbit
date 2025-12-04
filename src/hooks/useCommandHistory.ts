import { useCallback, useState } from "react";
import type { InputMode } from "@/store";

interface HistoryEntry {
  command: string;
  mode: InputMode;
}

interface UseCommandHistoryReturn {
  /** Current history array (readonly) */
  history: readonly HistoryEntry[];
  /** Add a command to history with its mode */
  add: (command: string, mode: InputMode) => void;
  /** Navigate up in history, returns the entry or null if at end */
  navigateUp: () => HistoryEntry | null;
  /** Navigate down in history, returns the entry or null if at beginning */
  navigateDown: () => HistoryEntry | null;
  /** Reset navigation index (call when user edits input manually) */
  reset: () => void;
  /** Current history index (-1 means not navigating) */
  index: number;
}

/**
 * Hook for managing command history with up/down navigation.
 *
 * @param initialHistory - Optional initial history array
 * @returns History management functions
 *
 * @example
 * ```tsx
 * const { add, navigateUp, navigateDown, reset } = useCommandHistory();
 *
 * // On submit
 * add(input, inputMode);
 *
 * // On ArrowUp
 * const entry = navigateUp();
 * if (entry !== null) {
 *   setInput(entry.command);
 *   setMode(entry.mode);
 * }
 *
 * // On ArrowDown
 * const entry = navigateDown();
 * if (entry !== null) {
 *   setInput(entry.command);
 *   setMode(entry.mode);
 * }
 *
 * // On manual input change
 * reset();
 * ```
 */
export function useCommandHistory(initialHistory: HistoryEntry[] = []): UseCommandHistoryReturn {
  const [history, setHistory] = useState<HistoryEntry[]>(initialHistory);
  const [index, setIndex] = useState(-1);

  const add = useCallback((command: string, mode: InputMode) => {
    if (!command.trim()) return;
    setHistory((prev) => [...prev, { command, mode }]);
    setIndex(-1);
  }, []);

  const navigateUp = useCallback((): HistoryEntry | null => {
    if (history.length === 0) return null;

    const newIndex = index < history.length - 1 ? index + 1 : index;
    setIndex(newIndex);
    return history[history.length - 1 - newIndex] ?? null;
  }, [history, index]);

  const navigateDown = useCallback((): HistoryEntry | null => {
    if (index > 0) {
      const newIndex = index - 1;
      setIndex(newIndex);
      return history[history.length - 1 - newIndex] ?? null;
    }
    setIndex(-1);
    return null;
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
  };
}
