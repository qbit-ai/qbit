/**
 * PaneLeaf Selectors
 *
 * This module provides optimized selectors for PaneLeaf component state.
 * The goal is to reduce unnecessary re-renders by:
 * 1. Only subscribing to relevant state (not full TabLayout or Session objects)
 * 2. Using shallow comparison for memoization
 * 3. Returning stable references when state hasn't changed
 */

import { useShallow } from "zustand/react/shallow";
import type { RenderMode, TabType } from "../index";
import { useStore } from "../index";

/**
 * State needed by PaneLeaf component.
 * Only includes the specific properties that affect rendering.
 */
export interface PaneLeafState {
  focusedPaneId: string | null;
  renderMode: RenderMode;
  workingDirectory: string | undefined;
  tabType: TabType;
  sessionExists: boolean;
  sessionName: string | undefined;
}

/**
 * Cache entry for memoized pane leaf state.
 */
interface CacheEntry {
  // Input values for comparison
  focusedPaneId: string | null;
  renderMode: RenderMode | undefined;
  workingDirectory: string | undefined;
  tabType: TabType | undefined;
  sessionExists: boolean;
  sessionName: string | undefined;
  // Computed result
  result: PaneLeafState;
}

// Cache keyed by "tabId:sessionId"
const cache = new Map<string, CacheEntry>();

/**
 * Generate cache key for tab+session combination.
 */
function getCacheKey(tabId: string, sessionId: string): string {
  return `${tabId}:${sessionId}`;
}

/**
 * Get raw inputs from store for a pane leaf.
 */
function getRawPaneLeafInputs(
  state: ReturnType<typeof useStore.getState>,
  tabId: string,
  sessionId: string
) {
  const tabLayout = state.tabLayouts[tabId];
  const session = state.sessions[sessionId];

  return {
    focusedPaneId: tabLayout?.focusedPaneId ?? null,
    renderMode: session?.renderMode,
    workingDirectory: session?.workingDirectory,
    tabType: session?.tabType,
    sessionExists: !!session,
    sessionName: session?.name,
  };
}

/**
 * Check if cache entry is still valid.
 */
function isCacheValid(
  cached: CacheEntry,
  inputs: ReturnType<typeof getRawPaneLeafInputs>
): boolean {
  return (
    cached.focusedPaneId === inputs.focusedPaneId &&
    cached.renderMode === inputs.renderMode &&
    cached.workingDirectory === inputs.workingDirectory &&
    cached.tabType === inputs.tabType &&
    cached.sessionExists === inputs.sessionExists &&
    cached.sessionName === inputs.sessionName
  );
}

/**
 * Create PaneLeafState from raw inputs.
 */
function createPaneLeafState(inputs: ReturnType<typeof getRawPaneLeafInputs>): PaneLeafState {
  return {
    focusedPaneId: inputs.focusedPaneId,
    renderMode: inputs.renderMode ?? "timeline",
    workingDirectory: inputs.workingDirectory,
    tabType: inputs.tabType ?? "terminal",
    sessionExists: inputs.sessionExists,
    sessionName: inputs.sessionName,
  };
}

/**
 * Memoized selector for pane leaf state.
 *
 * Returns a stable reference if none of the relevant inputs have changed.
 * This allows PaneLeaf components to skip re-renders when unrelated state changes.
 *
 * @param state - The full store state
 * @param tabId - The tab ID
 * @param sessionId - The session ID for this pane
 * @returns PaneLeafState with only the properties PaneLeaf needs
 */
export function selectPaneLeafState(
  state: ReturnType<typeof useStore.getState>,
  tabId: string,
  sessionId: string
): PaneLeafState {
  const cacheKey = getCacheKey(tabId, sessionId);
  const inputs = getRawPaneLeafInputs(state, tabId, sessionId);
  const cached = cache.get(cacheKey);

  // Return cached result if inputs haven't changed
  if (cached && isCacheValid(cached, inputs)) {
    return cached.result;
  }

  // Compute new result
  const result = createPaneLeafState(inputs);

  // Update cache
  cache.set(cacheKey, {
    ...inputs,
    result,
  });

  return result;
}

/**
 * React hook for accessing pane leaf state with automatic shallow comparison.
 *
 * This replaces multiple individual useStore calls with a single subscription
 * that only triggers re-renders when the relevant state actually changes.
 *
 * @param tabId - The tab ID
 * @param sessionId - The session ID for this pane
 * @returns PaneLeafState
 */
export function usePaneLeafState(tabId: string, sessionId: string): PaneLeafState {
  return useStore(useShallow((state) => selectPaneLeafState(state, tabId, sessionId)));
}

/**
 * Clear the cache for a specific tab+session combination.
 */
export function clearPaneLeafCache(tabId: string, sessionId: string): void {
  cache.delete(getCacheKey(tabId, sessionId));
}

/**
 * Clear all pane leaf caches.
 * Useful for testing.
 */
export function clearAllPaneLeafCaches(): void {
  cache.clear();
}
