import { useEffect, useMemo, useState } from "react";
import { virtualTerminalManager } from "../lib/terminal";

/**
 * Hook to get processed output from VirtualTerminal for a session.
 *
 * This hook gets processed output from VirtualTerminalManager
 * and returns it. The output is properly processed to handle
 * terminal animations like spinners and progress bars.
 *
 * @param sessionId - The session ID to get output for
 * @param rawOutput - The raw output string (used as dependency for updates)
 * @param fallbackFn - Optional fallback function for when VirtualTerminal isn't available
 * @returns The processed output string
 */
export function useProcessedOutput(
  sessionId: string,
  rawOutput: string | undefined,
  fallbackFn?: (raw: string) => string
): string {
  // Compute fallback synchronously for immediate display
  const fallbackOutput = useMemo(() => {
    if (!rawOutput) return "";
    return fallbackFn ? fallbackFn(rawOutput) : rawOutput;
  }, [rawOutput, fallbackFn]);

  // Store the VirtualTerminal processed output (if available)
  const [vtOutput, setVtOutput] = useState<string | null>(null);

  useEffect(() => {
    if (!rawOutput) {
      setVtOutput(null);
      return;
    }

    let cancelled = false;

    // Try to get processed output from VirtualTerminal
    virtualTerminalManager
      .getProcessedOutput(sessionId)
      .then((content) => {
        if (!cancelled && content) {
          // Only update if VirtualTerminal returned content
          setVtOutput(content);
        }
      })
      .catch(() => {
        // On error, don't update - will use fallback
      });

    return () => {
      cancelled = true;
    };
  }, [sessionId, rawOutput]);

  // Prefer VirtualTerminal output, fall back to processed fallback
  return vtOutput ?? fallbackOutput;
}
