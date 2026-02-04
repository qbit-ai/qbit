# State Management Performance Review

## Summary

The Zustand store in `frontend/store/index.ts` has undergone significant optimization with combined selectors and memoization patterns. However, several performance issues remain that can cause unnecessary re-renders and cascading updates.

**Overall Assessment:** The codebase demonstrates awareness of performance best practices (combined selectors, stable references, memoization). However, there are still opportunities for improvement, particularly around:
1. Missing shallow comparisons in derived selectors
2. Object recreation in some selectors causing unnecessary re-renders
3. Inefficient state normalization patterns
4. Several component-level subscription patterns that could be optimized

---

## Issues Found

### HIGH Priority

#### 1. App.tsx subscribes to entire `sessions` and `tabLayouts` objects
**File:** `/Users/xlyk/Code/qbit.worktrees/frontend-improvements/frontend/App.tsx:84-97`

```typescript
const {
  addSession,
  activeSessionId,
  sessions,        // Full Record<string, Session>
  tabLayouts,      // Full Record<string, TabLayout>
  setInputMode,
  // ...
} = useStore();
```

**Problem:** Destructuring `sessions` and `tabLayouts` directly from the store subscribes the App component to ALL changes in these records. Any change to any session or layout triggers a re-render of the entire App component tree.

**Recommended Fix:**
```typescript
// Only subscribe to specific data needed
const activeSessionId = useStore((state) => state.activeSessionId);
const addSession = useStore((state) => state.addSession);

// For keyboard shortcuts, use getState() pattern since we only need values at call time
const handleKeyDown = useCallback((e: KeyboardEvent) => {
  const { sessions, tabLayouts } = useStore.getState();
  // ... use sessions/tabLayouts
}, []);
```

---

#### 2. Missing shallow comparison in `selectUnreadNotificationCount`
**File:** `/Users/xlyk/Code/qbit.worktrees/frontend-improvements/frontend/store/slices/notification.ts:109-110`

```typescript
export const selectUnreadNotificationCount = <T extends NotificationState>(state: T): number =>
  state.notifications.filter((n) => !n.read).length;
```

**Problem:** This creates a new array on every state change via `.filter()`. While it returns a primitive number, the filter operation itself is wasteful. More importantly, this pattern could encourage similar issues elsewhere.

**Recommended Fix:**
```typescript
export const selectUnreadNotificationCount = <T extends NotificationState>(state: T): number => {
  let count = 0;
  for (const n of state.notifications) {
    if (!n.read) count++;
  }
  return count;
};
```

---

#### 3. PaneLeaf subscribes to full session object and tabLayout
**File:** `/Users/xlyk/Code/qbit.worktrees/frontend-improvements/frontend/components/PaneContainer/PaneLeaf.tsx:34-38`

```typescript
const focusPane = useStore((state) => state.focusPane);
const tabLayout = useStore((state) => state.tabLayouts[tabId]);
const focusedPaneId = tabLayout?.focusedPaneId;
const session = useStore((state) => state.sessions[sessionId]);
```

**Problem:**
- `tabLayout` selector returns the entire `TabLayout` object. Any change to that tab's layout (resize, focus change) triggers re-render.
- `session` selector returns the full `Session` object. Any property change triggers re-render even if the rendered properties didn't change.

**Recommended Fix:**
```typescript
// Create a combined selector for PaneLeaf state
interface PaneLeafState {
  focusedPaneId: PaneId | null;
  renderMode: RenderMode;
  workingDirectory: string | undefined;
  tabType: TabType;
  sessionExists: boolean;
}

export function usePaneLeafState(tabId: string, sessionId: string): PaneLeafState {
  return useStore((state) => {
    const tabLayout = state.tabLayouts[tabId];
    const session = state.sessions[sessionId];
    return {
      focusedPaneId: tabLayout?.focusedPaneId ?? null,
      renderMode: session?.renderMode ?? "timeline",
      workingDirectory: session?.workingDirectory,
      tabType: session?.tabType ?? "terminal",
      sessionExists: !!session,
    };
  }, shallow); // Use shallow comparison
}
```

---

#### 4. Timeline memoized selectors don't use shallow comparison
**File:** `/Users/xlyk/Code/qbit.worktrees/frontend-improvements/frontend/lib/timeline/selectors.ts:52-67`

```typescript
return (sessionId: string, timeline: UnifiedBlock[] | undefined): CommandBlock[] => {
  const cached = cache.get(sessionId);

  // Return cached result if timeline reference hasn't changed
  if (cached && cached.timeline === timeline) {
    return cached.result;
  }
  // ... creates new array every time timeline changes
```

**Problem:** The memoization only checks if the timeline reference is the same. If any timeline mutation creates a new array (which Immer does), the entire result array is recreated even if the command blocks within it haven't changed.

**Recommended Fix:** Add shallow comparison of the derived arrays:
```typescript
function arraysShallowEqual<T>(a: T[], b: T[]): boolean {
  if (a.length !== b.length) return false;
  for (let i = 0; i < a.length; i++) {
    if (a[i] !== b[i]) return false;
  }
  return true;
}

export function createMemoizedCommandBlocksSelector() {
  const cache = new Map<string, { timeline: UnifiedBlock[] | undefined; result: CommandBlock[] }>();

  return (sessionId: string, timeline: UnifiedBlock[] | undefined): CommandBlock[] => {
    const cached = cache.get(sessionId);

    if (cached && cached.timeline === timeline) {
      return cached.result;
    }

    const result = selectCommandBlocksFromTimeline(timeline);

    // Return cached result if content is shallow-equal
    if (cached && arraysShallowEqual(cached.result, result)) {
      // Update cache with new timeline reference but keep result reference
      cache.set(sessionId, { timeline, result: cached.result });
      return cached.result;
    }

    cache.set(sessionId, { timeline, result });
    return result;
  };
}
```

---

### MEDIUM Priority

#### 5. `useActiveSession` creates inline selector function
**File:** `/Users/xlyk/Code/qbit.worktrees/frontend-improvements/frontend/store/index.ts:1954-1958`

```typescript
export const useActiveSession = () =>
  useStore((state) => {
    const id = state.activeSessionId;
    return id ? state.sessions[id] : null;
  });
```

**Problem:** The inline arrow function is recreated on every call. While Zustand handles this internally with its own equality check, it's slightly more efficient to use a stable selector reference.

**Recommended Fix:**
```typescript
const selectActiveSession = (state: QbitState) => {
  const id = state.activeSessionId;
  return id ? state.sessions[id] : null;
};

export const useActiveSession = () => useStore(selectActiveSession);
```

---

#### 6. Session state selectors missing from combined `useSessionState`
**File:** `/Users/xlyk/Code/qbit.worktrees/frontend-improvements/frontend/store/selectors/session.ts`

**Problem:** `useSessionState` is well-designed but several components still use individual selectors that aren't in the combined selector:
- `useIsAgentResponding` (used in multiple places but not in `SessionState`)
- `useAgentStreaming` (used separately from `streamingTextLength`)

Components like `UnifiedInput` use `useUnifiedInputState` which is good, but there's potential for further consolidation.

**Recommended Fix:** Audit components using multiple `use*` hooks and consider expanding combined selectors or creating new ones for specific use cases.

---

#### 7. `updateAgentStreaming` performs join on every delta
**File:** `/Users/xlyk/Code/qbit.worktrees/frontend-improvements/frontend/store/index.ts:1075-1101`

```typescript
updateAgentStreaming: (sessionId, delta) =>
  set((state) => {
    // ...
    state.agentStreamingBuffer[sessionId].push(delta);
    // Update cached joined text (for selectors that read agentStreaming directly)
    state.agentStreaming[sessionId] = state.agentStreamingBuffer[sessionId].join("");
    // ...
  }),
```

**Problem:** The comment says "Note: join() is called once per ~60fps throttle cycle, not per delta" but the code shows `join()` is called on EVERY `updateAgentStreaming` call. The throttling happens in `useAiEvents.ts` at the event batching level, but if batches contain multiple deltas, each one still triggers a join.

**Recommended Fix:** Only join when reading the value, or maintain a dirty flag:
```typescript
updateAgentStreaming: (sessionId, delta) =>
  set((state) => {
    if (!state.agentStreamingBuffer[sessionId]) {
      state.agentStreamingBuffer[sessionId] = [];
    }
    state.agentStreamingBuffer[sessionId].push(delta);
    // Mark as dirty - actual join happens in getAgentStreamingText or selectors
    // Don't join here to avoid O(n) operation on every delta
  }),

// Add a lazy join pattern
getAgentStreamingText: (sessionId) => {
  const state = get();
  const buffer = state.agentStreamingBuffer[sessionId];
  if (!buffer || buffer.length === 0) return "";

  // Check if cached value needs refresh
  const cached = state.agentStreaming[sessionId];
  if (cached !== undefined && buffer._cachedLength === buffer.length) {
    return cached;
  }

  // Trigger a sync update only when actually needed
  const joined = buffer.join("");
  // Note: This could be done outside of getState if needed
  return joined;
},
```

---

#### 8. GitPanel uses three separate selectors
**File:** `/Users/xlyk/Code/qbit.worktrees/frontend-improvements/frontend/components/GitPanel/GitPanel.tsx:453-456`

```typescript
const gitStatus = useGitStatus(sessionId ?? "");
const isLoading = useGitStatusLoading(sessionId ?? "");
const storedMessage = useGitCommitMessage(sessionId ?? "");
```

**Problem:** Three separate subscriptions when one combined selector would suffice.

**Recommended Fix:**
```typescript
// In store/selectors/git.ts
export function useGitPanelState(sessionId: string) {
  return useStore((state) => ({
    gitStatus: state.gitStatus[sessionId] ?? null,
    isLoading: state.gitStatusLoading[sessionId] ?? false,
    commitMessage: state.gitCommitMessage[sessionId] ?? "",
  }), shallow);
}
```

---

### LOW Priority

#### 9. Empty array constants are duplicated
**Files:**
- `/Users/xlyk/Code/qbit.worktrees/frontend-improvements/frontend/store/index.ts:1993` (`EMPTY_TIMELINE`)
- `/Users/xlyk/Code/qbit.worktrees/frontend-improvements/frontend/store/index.ts:2018` (`EMPTY_TOOL_CALLS`)
- `/Users/xlyk/Code/qbit.worktrees/frontend-improvements/frontend/store/index.ts:2024` (`EMPTY_STREAMING_BLOCKS`)
- `/Users/xlyk/Code/qbit.worktrees/frontend-improvements/frontend/store/selectors/session.ts:45-48` (duplicates)

**Problem:** Same empty array pattern is defined in multiple places. While not a performance issue per se, it increases maintenance burden.

**Recommended Fix:** Create a shared `constants.ts` file for stable empty references:
```typescript
// store/constants.ts
export const EMPTY_ARRAY: readonly never[] = [];
export const EMPTY_TIMELINE: UnifiedBlock[] = EMPTY_ARRAY as UnifiedBlock[];
export const EMPTY_TOOL_CALLS: ActiveToolCall[] = EMPTY_ARRAY as ActiveToolCall[];
// etc.
```

---

#### 10. `markTabNewActivityInDraft` iterates all tabLayouts
**File:** `/Users/xlyk/Code/qbit.worktrees/frontend-improvements/frontend/store/index.ts:51-65`

```typescript
function markTabNewActivityInDraft(state: QbitState, sessionId: string): void {
  for (const [tabId, layout] of Object.entries(state.tabLayouts)) {
    const leaves = getAllLeafPanes(layout.root);
    if (leaves.some((leaf) => leaf.sessionId === sessionId)) {
      // ...
```

**Problem:** This iterates through all tab layouts and all panes to find the owning tab. While acceptable for small numbers of tabs, could become slow with many tabs/panes.

**Recommended Fix:** Add a reverse lookup map:
```typescript
// In state
sessionToTabMap: Record<string, string>; // sessionId -> tabId

// Update when sessions are added/removed
// Then in markTabNewActivityInDraft:
function markTabNewActivityInDraft(state: QbitState, sessionId: string): void {
  const tabId = state.sessionToTabMap[sessionId];
  if (tabId && state.activeSessionId !== tabId) {
    const rootSession = state.sessions[tabId];
    if ((rootSession?.tabType ?? "terminal") === "terminal") {
      state.tabHasNewActivity[tabId] = true;
    }
  }
}
```

---

#### 11. Immer with `enableMapSet()` has overhead
**File:** `/Users/xlyk/Code/qbit.worktrees/frontend-improvements/frontend/store/index.ts:44-45`

```typescript
// Enable Immer support for Set and Map (needed for processedToolRequests)
enableMapSet();
```

**Problem:** `enableMapSet()` adds runtime overhead to all Immer operations. Currently only used for `processedToolRequests` which is a `Set<string>`.

**Recommended Fix:** Consider using a plain object instead:
```typescript
// Instead of: processedToolRequests: Record<string, Set<string>>
processedToolRequests: Record<string, Record<string, true>>;

// Usage:
isToolRequestProcessed: (sessionId, requestId) => {
  return get().processedToolRequests[sessionId]?.[requestId] === true;
},

markToolRequestProcessed: (sessionId, requestId) =>
  set((state) => {
    if (!state.processedToolRequests[sessionId]) {
      state.processedToolRequests[sessionId] = {};
    }
    state.processedToolRequests[sessionId][requestId] = true;
  }),
```

---

## Best Practices Already Implemented

The codebase demonstrates several good patterns:

1. **Combined Selectors:** `useSessionState`, `useUnifiedInputState`, `useTabBarState` consolidate multiple subscriptions
2. **External Memoization:** `memoizedSelectCommandBlocks` and `memoizedSelectAgentMessages` with per-session caching
3. **Stable Empty References:** `EMPTY_TIMELINE`, `EMPTY_TOOL_CALLS`, etc. prevent unnecessary re-renders
4. **Actions via getState():** `TabItem.handleSave` uses `useStore.getState()` for one-shot mutations
5. **Throttled Updates:** `useAiEvents` batches text deltas at ~60fps
6. **Memo Components:** Key components like `TabItem`, `UnifiedTimeline`, `GitPanel` use `React.memo`

---

## Recommendations Summary

| Priority | Issue | Impact | Effort |
|----------|-------|--------|--------|
| HIGH | App.tsx subscribing to full records | Major re-renders | Low |
| HIGH | PaneLeaf full object subscriptions | Component re-renders | Medium |
| HIGH | Timeline selectors missing shallow compare | Array recreation | Medium |
| MEDIUM | `updateAgentStreaming` join on every delta | CPU during streaming | Medium |
| MEDIUM | Missing combined selectors in some components | Extra subscriptions | Low |
| LOW | Duplicate empty array constants | Code maintenance | Low |
| LOW | `enableMapSet()` overhead | Minor perf cost | Low |
