import { useEffect, useState } from "react";
import { listPathCompletions, type PathCompletion } from "@/lib/tauri";

interface UsePathCompletionOptions {
  sessionId: string;
  partialPath: string;
  enabled: boolean;
}

export function usePathCompletion({ sessionId, partialPath, enabled }: UsePathCompletionOptions) {
  const [completions, setCompletions] = useState<PathCompletion[]>([]);
  const [isLoading, setIsLoading] = useState(false);

  useEffect(() => {
    if (!enabled) {
      setCompletions([]);
      return;
    }

    let cancelled = false;
    setIsLoading(true);

    listPathCompletions(sessionId, partialPath, 20)
      .then((results) => {
        if (!cancelled) setCompletions(results);
      })
      .catch((error) => {
        console.error("Path completion error:", error);
        if (!cancelled) setCompletions([]);
      })
      .finally(() => {
        if (!cancelled) setIsLoading(false);
      });

    return () => {
      cancelled = true;
    };
  }, [sessionId, partialPath, enabled]);

  return { completions, isLoading };
}
