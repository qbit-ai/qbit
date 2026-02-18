/**
 * Combined Selector for UnifiedInput Component
 *
 * This module provides an optimized selector for accessing all state needed
 * by the UnifiedInput component, reducing ~15 individual subscriptions to 1.
 *
 * Key optimizations:
 * 1. Single selector returns all input-related state
 * 2. Memoization per session - prevents unnecessary recalculations
 * 3. Cross-session isolation - changes to one session don't invalidate others
 */

import { useStore } from "../index";

/**
 * Combined state for UnifiedInput component.
 */
export interface UnifiedInputState {
  // Session info
  inputMode: "terminal" | "agent" | "auto";
  workingDirectory: string;
  virtualEnv: string | null;

  // Agent state
  isAgentResponding: boolean;
  isCompacting: boolean;
  isSessionDead: boolean;
  streamingBlocksLength: number;

  // Git state
  gitBranch: string | null;
  gitStatus: {
    insertions: number;
    deletions: number;
    ahead: number;
    behind: number;
  } | null;
}

// Stable defaults
const EMPTY_GIT_STATUS = null;

/**
 * Cache for memoized input state.
 */
interface CacheEntry {
  // Input references for shallow comparison
  session: ReturnType<typeof useStore.getState>["sessions"][string] | undefined;
  isAgentResponding: boolean | undefined;
  isCompacting: boolean | undefined;
  isSessionDead: boolean | undefined;
  streamingBlocks: unknown[] | undefined;
  gitStatus: ReturnType<typeof useStore.getState>["gitStatus"][string] | undefined;
  // Computed result
  result: UnifiedInputState;
}

const cache = new Map<string, CacheEntry>();

/**
 * Get raw state values for a session's input.
 */
function getRawInputs(state: ReturnType<typeof useStore.getState>, sessionId: string) {
  return {
    session: state.sessions[sessionId],
    isAgentResponding: state.isAgentResponding[sessionId],
    isCompacting: state.isCompacting[sessionId],
    isSessionDead: state.isSessionDead[sessionId],
    streamingBlocks: state.streamingBlocks[sessionId],
    gitStatus: state.gitStatus[sessionId],
  };
}

/**
 * Check if cache entry is still valid.
 */
function isCacheValid(cached: CacheEntry, inputs: ReturnType<typeof getRawInputs>): boolean {
  return (
    cached.session === inputs.session &&
    cached.isAgentResponding === inputs.isAgentResponding &&
    cached.isCompacting === inputs.isCompacting &&
    cached.isSessionDead === inputs.isSessionDead &&
    cached.streamingBlocks === inputs.streamingBlocks &&
    cached.gitStatus === inputs.gitStatus
  );
}

/**
 * Create UnifiedInputState from raw inputs.
 */
function createInputState(inputs: ReturnType<typeof getRawInputs>): UnifiedInputState {
  const session = inputs.session;
  const gitStatus = inputs.gitStatus;

  return {
    inputMode: session?.inputMode ?? "terminal",
    workingDirectory: session?.workingDirectory ?? "",
    virtualEnv: session?.virtualEnv ?? null,
    isAgentResponding: inputs.isAgentResponding ?? false,
    isCompacting: inputs.isCompacting ?? false,
    isSessionDead: inputs.isSessionDead ?? false,
    streamingBlocksLength: inputs.streamingBlocks?.length ?? 0,
    gitBranch: gitStatus?.branch ?? null,
    gitStatus: gitStatus
      ? {
          insertions: gitStatus.insertions ?? 0,
          deletions: gitStatus.deletions ?? 0,
          ahead: gitStatus.ahead ?? 0,
          behind: gitStatus.behind ?? 0,
        }
      : EMPTY_GIT_STATUS,
  };
}

/**
 * Memoized selector for UnifiedInput state.
 */
export function selectUnifiedInputState(
  state: ReturnType<typeof useStore.getState>,
  sessionId: string
): UnifiedInputState {
  const inputs = getRawInputs(state, sessionId);
  const cached = cache.get(sessionId);

  if (cached && isCacheValid(cached, inputs)) {
    return cached.result;
  }

  const result = createInputState(inputs);

  cache.set(sessionId, {
    ...inputs,
    result,
  });

  return result;
}

/**
 * React hook for accessing combined UnifiedInput state.
 */
export function useUnifiedInputState(sessionId: string): UnifiedInputState {
  return useStore((state) => selectUnifiedInputState(state, sessionId));
}

/**
 * Clear cache for a session.
 */
export function clearUnifiedInputCache(sessionId: string): void {
  cache.delete(sessionId);
}

/**
 * Clear all caches.
 */
export function clearAllUnifiedInputCaches(): void {
  cache.clear();
}
