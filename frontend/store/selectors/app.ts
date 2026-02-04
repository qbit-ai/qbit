/**
 * App.tsx Selectors
 *
 * This module provides optimized selectors for the main App component.
 * The goal is to avoid subscribing to entire `sessions` and `tabLayouts`
 * Record objects, which cause re-renders on ANY session/layout change.
 *
 * Key optimizations:
 * 1. Only extract the specific data App.tsx needs for rendering
 * 2. Use shallow comparison for memoization
 * 3. Callbacks use getState() instead of subscribed state
 */

import { useShallow } from "zustand/react/shallow";
import type { PaneNode } from "@/lib/pane-utils";
import { useStore } from "../index";

/**
 * Minimal tab info for rendering in App.tsx.
 * Only includes what's needed for the tab iteration.
 */
export interface TabLayoutInfo {
  tabId: string;
  root: PaneNode;
}

/**
 * State needed by App.tsx for rendering.
 * This replaces the full `sessions` and `tabLayouts` subscriptions.
 */
export interface AppState {
  /** The active tab ID */
  activeSessionId: string | null;
  /** Working directory for the focused session (for sidebar/panels) */
  focusedWorkingDirectory: string | undefined;
  /** Tab layouts for rendering all tabs (keeps terminals mounted) */
  tabLayouts: TabLayoutInfo[];
}

// Cache for memoization
interface AppStateCacheEntry {
  // Raw inputs for comparison
  activeSessionId: string | null;
  focusedSessionId: string | null;
  focusedWorkingDirectory: string | undefined;
  tabLayoutKeys: string[];
  tabLayoutRoots: PaneNode[];
  // Computed result
  result: AppState;
}

let cache: AppStateCacheEntry | null = null;

/**
 * Get the focused session ID from the active tab's layout.
 */
function getFocusedSessionId(state: ReturnType<typeof useStore.getState>): string | null {
  const activeSessionId = state.activeSessionId;
  if (!activeSessionId) return null;

  const tabLayout = state.tabLayouts[activeSessionId];
  if (!tabLayout) return null;

  // Find the focused pane's session ID
  function findSessionInPane(node: PaneNode, focusedPaneId: string): string | null {
    if (node.type === "leaf") {
      return node.id === focusedPaneId ? node.sessionId : null;
    }
    for (const child of node.children) {
      const result = findSessionInPane(child, focusedPaneId);
      if (result) return result;
    }
    return null;
  }

  return findSessionInPane(tabLayout.root, tabLayout.focusedPaneId);
}

/**
 * Memoized selector for App state.
 *
 * Returns a stable reference if none of the relevant inputs have changed.
 */
export function selectAppState(state: ReturnType<typeof useStore.getState>): AppState {
  const activeSessionId = state.activeSessionId;
  const focusedSessionId = getFocusedSessionId(state);
  const focusedSession = focusedSessionId ? state.sessions[focusedSessionId] : null;
  const focusedWorkingDirectory = focusedSession?.workingDirectory;

  // Extract tab layout keys and roots
  const tabLayoutKeys = Object.keys(state.tabLayouts);
  const tabLayoutRoots = tabLayoutKeys.map((key) => state.tabLayouts[key].root);

  // Check cache validity
  if (cache) {
    const sameActiveSession = cache.activeSessionId === activeSessionId;
    const sameFocusedSession = cache.focusedSessionId === focusedSessionId;
    const sameFocusedWd = cache.focusedWorkingDirectory === focusedWorkingDirectory;
    const sameTabLayoutKeys =
      cache.tabLayoutKeys.length === tabLayoutKeys.length &&
      cache.tabLayoutKeys.every((key, i) => key === tabLayoutKeys[i]);
    const sameTabLayoutRoots =
      cache.tabLayoutRoots.length === tabLayoutRoots.length &&
      cache.tabLayoutRoots.every((root, i) => root === tabLayoutRoots[i]);

    if (
      sameActiveSession &&
      sameFocusedSession &&
      sameFocusedWd &&
      sameTabLayoutKeys &&
      sameTabLayoutRoots
    ) {
      return cache.result;
    }
  }

  // Build tab layouts array
  const tabLayouts: TabLayoutInfo[] = tabLayoutKeys.map((tabId) => ({
    tabId,
    root: state.tabLayouts[tabId].root,
  }));

  const result: AppState = {
    activeSessionId,
    focusedWorkingDirectory,
    tabLayouts,
  };

  cache = {
    activeSessionId,
    focusedSessionId,
    focusedWorkingDirectory,
    tabLayoutKeys,
    tabLayoutRoots,
    result,
  };

  return result;
}

/**
 * React hook for accessing App state with automatic shallow comparison.
 *
 * This replaces the separate subscriptions to `sessions` and `tabLayouts`
 * with a single optimized subscription that only triggers re-renders when
 * the relevant state actually changes.
 */
export function useAppState(): AppState {
  return useStore(useShallow((state) => selectAppState(state)));
}

/**
 * Clear the app state cache.
 * Useful for testing.
 */
export function clearAppStateCache(): void {
  cache = null;
}
