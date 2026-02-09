/**
 * Combined Selector for TabBar Component
 *
 * This module provides optimized selectors for the TabBar, avoiding
 * subscriptions to entire Record objects that would trigger re-renders
 * on ANY session change.
 *
 * Key optimizations:
 * 1. Returns only the data needed for tab display
 * 2. Per-tab memoization - changes to one tab don't invalidate others
 * 3. Computes "isRunning" state locally instead of subscribing
 */

import type { SessionMode, TabType } from "../index";
import { useStore } from "../index";

/**
 * State for a single tab item.
 * Contains all fields needed by TabItem component for rendering.
 */
export interface TabItemState {
  id: string;
  name: string;
  customName: string | null;
  isRunning: boolean;
  hasNewActivity: boolean;
  hasPendingCommand: boolean;
  // Additional fields needed for TabItem rendering
  tabType: TabType;
  workingDirectory: string;
  processName: string | null;
  mode: SessionMode;
}

/**
 * Combined state for the entire TabBar.
 */
export interface TabBarState {
  tabs: TabItemState[];
  activeSessionId: string | null;
  homeTabId: string | null;
}

// Cache for per-tab state
interface TabItemCacheEntry {
  session: ReturnType<typeof useStore.getState>["sessions"][string] | undefined;
  isAgentResponding: boolean | undefined;
  pendingCommand: ReturnType<typeof useStore.getState>["pendingCommand"][string] | undefined;
  hasNewActivity: boolean | undefined;
  result: TabItemState;
}

const tabItemCache = new Map<string, TabItemCacheEntry>();

// Cache for full tab bar state
interface TabBarCacheEntry {
  tabLayoutIds: string[];
  sessionIds: string[];
  activeSessionId: string | null;
  homeTabId: string | null;
  tabStates: TabItemState[];
  result: TabBarState;
}

let tabBarCache: TabBarCacheEntry | null = null;

/**
 * Get raw inputs for a single tab item.
 */
function getRawTabItemInputs(state: ReturnType<typeof useStore.getState>, sessionId: string) {
  return {
    session: state.sessions[sessionId],
    isAgentResponding: state.isAgentResponding[sessionId],
    pendingCommand: state.pendingCommand[sessionId],
    hasNewActivity: state.tabHasNewActivity[sessionId],
  };
}

/**
 * Check if tab item cache is valid.
 */
function isTabItemCacheValid(
  cached: TabItemCacheEntry,
  inputs: ReturnType<typeof getRawTabItemInputs>
): boolean {
  return (
    cached.session === inputs.session &&
    cached.isAgentResponding === inputs.isAgentResponding &&
    cached.pendingCommand === inputs.pendingCommand &&
    cached.hasNewActivity === inputs.hasNewActivity
  );
}

/**
 * Create TabItemState from raw inputs.
 */
function createTabItemState(
  sessionId: string,
  inputs: ReturnType<typeof getRawTabItemInputs>
): TabItemState {
  const session = inputs.session;
  const isRunning = inputs.isAgentResponding ?? false;
  const hasPendingCommand = !!inputs.pendingCommand?.command;

  return {
    id: sessionId,
    name: session?.name ?? sessionId,
    customName: session?.customName ?? null,
    isRunning,
    hasNewActivity: inputs.hasNewActivity ?? false,
    hasPendingCommand,
    // Additional fields for TabItem rendering
    tabType: session?.tabType ?? "terminal",
    workingDirectory: session?.workingDirectory ?? "",
    processName: session?.processName ?? null,
    mode: session?.mode ?? "terminal",
  };
}

/**
 * Memoized selector for a single tab item.
 */
export function selectTabItemState(
  state: ReturnType<typeof useStore.getState>,
  sessionId: string
): TabItemState {
  const inputs = getRawTabItemInputs(state, sessionId);
  const cached = tabItemCache.get(sessionId);

  if (cached && isTabItemCacheValid(cached, inputs)) {
    return cached.result;
  }

  const result = createTabItemState(sessionId, inputs);

  tabItemCache.set(sessionId, {
    ...inputs,
    result,
  });

  return result;
}

/**
 * Memoized selector for the entire TabBar state.
 */
export function selectTabBarState(state: ReturnType<typeof useStore.getState>): TabBarState {
  // Use explicit tabOrder for ordering (home tab is always at index 0)
  // Fall back to Object.keys(tabLayouts) for backward compatibility
  const tabLayoutIds =
    state.tabOrder.length > 0
      ? state.tabOrder.filter((id) => state.tabLayouts[id] != null)
      : Object.keys(state.tabLayouts);
  const sessionIds = tabLayoutIds.filter((id) => state.sessions[id] != null);

  const activeSessionId = state.activeSessionId;
  const homeTabId = state.homeTabId;

  // Collect tab states
  const tabStates = sessionIds.map((id) => selectTabItemState(state, id));

  // Check if we can reuse the cached result
  if (tabBarCache) {
    // First check if tab layouts changed (affects which sessions are tabs)
    const sameTabLayoutIds =
      tabBarCache.tabLayoutIds.length === tabLayoutIds.length &&
      tabBarCache.tabLayoutIds.every((id, i) => id === tabLayoutIds[i]);

    const sameSessionIds =
      tabBarCache.sessionIds.length === sessionIds.length &&
      tabBarCache.sessionIds.every((id, i) => id === sessionIds[i]);

    const sameActiveSession = tabBarCache.activeSessionId === activeSessionId;
    const sameHomeTab = tabBarCache.homeTabId === homeTabId;

    // Check if all tab states are the same references
    const sameTabStates =
      tabBarCache.tabStates.length === tabStates.length &&
      tabBarCache.tabStates.every((tab, i) => tab === tabStates[i]);

    if (sameTabLayoutIds && sameSessionIds && sameActiveSession && sameHomeTab && sameTabStates) {
      return tabBarCache.result;
    }
  }

  const result: TabBarState = {
    tabs: tabStates,
    activeSessionId,
    homeTabId,
  };

  tabBarCache = {
    tabLayoutIds,
    sessionIds,
    activeSessionId,
    homeTabId,
    tabStates,
    result,
  };

  return result;
}

/**
 * React hook for accessing TabBar state.
 */
export function useTabBarState(): TabBarState {
  return useStore((state) => selectTabBarState(state));
}

/**
 * React hook for accessing a single tab item state.
 */
export function useTabItemState(sessionId: string): TabItemState {
  return useStore((state) => selectTabItemState(state, sessionId));
}

/**
 * Clear cache for a specific tab.
 */
export function clearTabItemCache(sessionId: string): void {
  tabItemCache.delete(sessionId);
}

/**
 * Clear all tab bar caches.
 */
export function clearTabBarCache(): void {
  tabItemCache.clear();
  tabBarCache = null;
}
