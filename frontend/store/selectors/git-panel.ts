/**
 * Combined Git Panel Selectors
 *
 * This module provides optimized selectors for accessing git status state.
 * The goal is to reduce the number of store subscriptions by combining related
 * state into a single selector call.
 *
 * Key optimizations:
 * 1. Single selector returns all git state - reduces subscription count from 3 to 1
 * 2. Memoization per session - prevents unnecessary recalculations
 * 3. Stable empty references - avoids creating new objects
 * 4. Cross-session isolation - changes to one session don't invalidate others
 */

import type { GitStatusSummary } from "@/lib/tauri";
import { useStore } from "../index";

/**
 * Combined git panel state returned by the selector.
 * Contains all the state needed by GitPanel in one object.
 */
export interface GitPanelState {
  gitStatus: GitStatusSummary | null;
  isLoading: boolean;
  commitMessage: string;
}

/**
 * Cache for memoized git panel state.
 * Key: sessionId
 * Value: { inputs, result } where inputs are the raw state values used to compute result
 */
interface CacheEntry {
  // Input references for shallow comparison
  gitStatus: GitStatusSummary | null | undefined;
  isLoading: boolean | undefined;
  commitMessage: string | undefined;
  // Computed result
  result: GitPanelState;
}

const cache = new Map<string, CacheEntry>();

/**
 * Get the raw state values for git panel.
 */
function getRawGitPanelInputs(state: ReturnType<typeof useStore.getState>, sessionId: string) {
  return {
    gitStatus: state.gitStatus[sessionId],
    isLoading: state.gitStatusLoading[sessionId],
    commitMessage: state.gitCommitMessage[sessionId],
  };
}

/**
 * Check if cache entry is still valid (shallow equality on all inputs).
 */
function isCacheValid(
  cached: CacheEntry,
  inputs: ReturnType<typeof getRawGitPanelInputs>
): boolean {
  return (
    cached.gitStatus === inputs.gitStatus &&
    cached.isLoading === inputs.isLoading &&
    cached.commitMessage === inputs.commitMessage
  );
}

/**
 * Create a new GitPanelState from raw inputs.
 */
function createGitPanelState(inputs: ReturnType<typeof getRawGitPanelInputs>): GitPanelState {
  return {
    gitStatus: inputs.gitStatus ?? null,
    isLoading: inputs.isLoading ?? false,
    commitMessage: inputs.commitMessage ?? "",
  };
}

/**
 * Memoized selector for git panel state.
 *
 * Returns a stable reference if none of the inputs have changed.
 * This allows React components to skip re-renders when the git
 * state hasn't actually changed.
 *
 * Reduces subscription count from 3 (useGitStatus, useGitStatusLoading, useGitCommitMessage)
 * to 1 (useGitPanelState).
 *
 * @param state - The full store state
 * @param sessionId - The session to get git state for
 * @returns Combined git panel state
 */
export function selectGitPanelState(
  state: ReturnType<typeof useStore.getState>,
  sessionId: string
): GitPanelState {
  const inputs = getRawGitPanelInputs(state, sessionId);

  const cached = cache.get(sessionId);
  if (cached && isCacheValid(cached, inputs)) {
    return cached.result;
  }

  const result = createGitPanelState(inputs);
  cache.set(sessionId, {
    gitStatus: inputs.gitStatus,
    isLoading: inputs.isLoading,
    commitMessage: inputs.commitMessage,
    result,
  });

  return result;
}

/**
 * Hook version of selectGitPanelState.
 * Automatically subscribes to git panel state changes.
 *
 * Usage:
 *   const gitPanelState = useGitPanelState(sessionId);
 *   // Access: gitPanelState.gitStatus, gitPanelState.isLoading, gitPanelState.commitMessage
 */
export function useGitPanelState(sessionId: string): GitPanelState {
  return useStore((state) => selectGitPanelState(state, sessionId));
}
