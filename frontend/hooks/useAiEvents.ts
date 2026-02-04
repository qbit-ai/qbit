import { useEffect, useRef } from "react";
import { type AiEvent, onAiEvent, signalFrontendReady, type ToolSource } from "@/lib/ai";
import { logger } from "@/lib/logger";
import { type ToolCallSource, useStore } from "@/store";
import { dispatchEvent, type EventHandlerContext } from "./ai-events";

/**
 * Track last seen sequence number per session for deduplication.
 * This is module-level to persist across hook re-renders but within the same app lifecycle.
 */
const lastSeenSeq = new Map<string, number>();

/**
 * Reset sequence tracking for a session.
 * Called when a session is removed or when the app needs to reset state.
 */
export function resetSessionSequence(sessionId: string): void {
  lastSeenSeq.delete(sessionId);
}

/**
 * Reset all sequence tracking. Useful for testing.
 */
export function resetAllSequences(): void {
  lastSeenSeq.clear();
}

/**
 * Get the number of sessions being tracked.
 * Useful for testing and debugging memory management.
 */
export function getSessionSequenceCount(): number {
  return lastSeenSeq.size;
}

/** Convert AI event source to store source (snake_case to camelCase) */
function convertToolSource(source?: ToolSource): ToolCallSource | undefined {
  if (!source) return undefined;
  if (source.type === "main") return { type: "main" };
  if (source.type === "sub_agent") {
    return {
      type: "sub_agent",
      agentId: source.agent_id,
      agentName: source.agent_name,
    };
  }
  if (source.type === "workflow") {
    return {
      type: "workflow",
      workflowId: source.workflow_id,
      workflowName: source.workflow_name,
      stepName: source.step_name,
      stepIndex: source.step_index,
    };
  }
  return undefined;
}

/**
 * Hook to subscribe to AI events from the Tauri backend
 * and update the store accordingly.
 *
 * Events are routed to the correct session using `event.session_id` from the backend.
 * This ensures proper multi-session isolation even when the user switches tabs
 * during AI streaming.
 *
 * Uses the event handler registry pattern for maintainability.
 */
export function useAiEvents() {
  const unlistenRef = useRef<(() => void) | null>(null);

  useEffect(() => {
    // Track if this effect instance is still mounted (for async cleanup)
    let isMounted = true;

    // Throttle state: batch text deltas and flush periodically
    const pendingDeltas = new Map<string, string>();
    let flushTimeout: ReturnType<typeof setTimeout> | null = null;
    let lastFlushTime = 0;
    const FLUSH_INTERVAL_MS = 16; // ~60fps

    // Flush all pending deltas to the store
    const flushPendingDeltas = () => {
      if (pendingDeltas.size === 0) return;
      const state = useStore.getState();
      for (const [sessionId, delta] of pendingDeltas) {
        state.updateAgentStreaming(sessionId, delta);
      }
      pendingDeltas.clear();
      lastFlushTime = Date.now();
      flushTimeout = null;
    };

    // Flush pending deltas for a specific session immediately
    // Called before adding non-text blocks to ensure correct ordering
    const flushSessionDeltas = (sessionId: string) => {
      const pending = pendingDeltas.get(sessionId);
      if (pending) {
        useStore.getState().updateAgentStreaming(sessionId, pending);
        pendingDeltas.delete(sessionId);
      }
    };

    // Add a text delta to the pending batch
    const batchTextDelta = (sessionId: string, delta: string) => {
      const current = pendingDeltas.get(sessionId) ?? "";
      pendingDeltas.set(sessionId, current + delta);

      // Flush immediately if enough time has passed since last flush
      const now = Date.now();
      if (now - lastFlushTime >= FLUSH_INTERVAL_MS) {
        flushPendingDeltas();
      } else if (!flushTimeout) {
        // Schedule a flush for the remaining time
        flushTimeout = setTimeout(flushPendingDeltas, FLUSH_INTERVAL_MS - (now - lastFlushTime));
      }
    };

    const handleEvent = (event: AiEvent) => {
      // Get the session ID from the event for proper routing
      const state = useStore.getState();
      let sessionId = event.session_id;

      // Fall back to activeSessionId if session_id is unknown (shouldn't happen in normal operation)
      if (!sessionId || sessionId === "unknown") {
        logger.warn("AI event received with unknown session_id, falling back to activeSessionId");
        const fallbackId = state.activeSessionId;
        if (!fallbackId) return;
        sessionId = fallbackId;
      }

      // Verify the session exists in the store
      if (!state.sessions[sessionId]) {
        // Upgrade to warn - this should not happen in normal operation and indicates
        // a session lifecycle mismatch between frontend and backend
        logger.warn("AI event dropped for unknown session:", {
          sessionId,
          eventType: event.type,
          activeSessionId: state.activeSessionId,
          knownSessions: Object.keys(state.sessions),
        });
        return;
      }

      // Deduplication: check sequence number if present
      if (event.seq !== undefined) {
        const lastSeq = lastSeenSeq.get(sessionId) ?? -1;

        // Skip duplicate or out-of-order events
        if (event.seq <= lastSeq) {
          logger.debug(
            `Skipping duplicate/out-of-order event: seq=${event.seq}, lastSeq=${lastSeq}, type=${event.type}`
          );
          return;
        }

        // Warn on sequence gaps (might indicate missed events)
        if (event.seq > lastSeq + 1) {
          logger.warn(
            `Event sequence gap: expected ${lastSeq + 1}, got ${event.seq} for session ${sessionId}`
          );
        }

        // Update last seen sequence
        lastSeenSeq.set(sessionId, event.seq);
      }

      // Create handler context
      const ctx: EventHandlerContext = {
        sessionId,
        getState: () => useStore.getState(),
        flushSessionDeltas,
        batchTextDelta,
        convertToolSource,
      };

      // Dispatch to registered handler
      const handled = dispatchEvent(event, ctx);

      if (!handled) {
        logger.warn("Unhandled AI event type:", event.type);
      }
    };

    // Only set up listener once - the handler uses getState() to access current values
    const setupListener = async () => {
      try {
        const unlisten = await onAiEvent(handleEvent);
        // Only store the unlisten function if we're still mounted
        // This handles the React Strict Mode double-mount where cleanup runs
        // before the async setup completes
        if (isMounted) {
          unlistenRef.current = unlisten;

          // Signal frontend ready for all existing sessions
          // This triggers the backend to replay any buffered events
          const sessions = Object.keys(useStore.getState().sessions);
          for (const sessionId of sessions) {
            signalFrontendReady(sessionId).catch((err) => {
              // Backend command may not exist yet during development
              logger.debug("Failed to signal frontend ready:", err);
            });
          }
        } else {
          // We were unmounted before setup completed - clean up immediately
          unlisten();
        }
      } catch {
        // AI backend not yet implemented - this is expected
        logger.debug("AI events not available - backend not implemented yet");
      }
    };

    setupListener();

    return () => {
      isMounted = false;
      if (unlistenRef.current) {
        unlistenRef.current();
        unlistenRef.current = null;
      }
      // Cancel any pending delta flush
      if (flushTimeout !== null) {
        clearTimeout(flushTimeout);
        flushTimeout = null;
      }
      // Flush any remaining deltas before unmount
      flushPendingDeltas();
    };
  }, []);
}
