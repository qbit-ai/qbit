import { useDeferredValue, useEffect, useState } from "react";
import { logger } from "@/lib/logger";
import { listPathCompletions, type PathCompletion } from "@/lib/tauri";

interface UsePathCompletionOptions {
  sessionId: string;
  partialPath: string;
  enabled: boolean;
}

export function usePathCompletion({ sessionId, partialPath, enabled }: UsePathCompletionOptions) {
  const [completions, setCompletions] = useState<PathCompletion[]>([]);
  const [totalCount, setTotalCount] = useState(0);
  const [isLoading, setIsLoading] = useState(false);
  const deferredPartialPath = useDeferredValue(partialPath);

  useEffect(() => {
    if (!enabled) {
      setCompletions([]);
      setTotalCount(0);
      return;
    }

    let cancelled = false;
    let debounceTimer: ReturnType<typeof setTimeout>;

    // Debounce API calls to avoid excessive requests during rapid typing
    const fetchCompletions = () => {
      setIsLoading(true);

      listPathCompletions(sessionId, deferredPartialPath, 20)
        .then((response) => {
          if (!cancelled) {
            setCompletions(response.completions);
            setTotalCount(response.total_count);
          }
        })
        .catch((error) => {
          logger.error("Path completion error:", error);
          if (!cancelled) {
            setCompletions([]);
            setTotalCount(0);
          }
        })
        .finally(() => {
          if (!cancelled) setIsLoading(false);
        });
    };

    // Debounce with 300ms delay to batch rapid changes
    debounceTimer = setTimeout(fetchCompletions, 300);

    return () => {
      cancelled = true;
      clearTimeout(debounceTimer);
    };
  }, [sessionId, deferredPartialPath, enabled]);

  return { completions, totalCount, isLoading };
}
