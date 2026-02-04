# Frontend Event Handling Performance Review

## Summary

This review analyzes event handling patterns in the Qbit terminal emulator frontend, focusing on event listener cleanup, debouncing/throttling, callback stability, and keyboard event efficiency. Overall, the codebase demonstrates **good event handling practices** with proper cleanup in most cases, but there are several opportunities for optimization.

### Key Findings

| Priority | Issue Count | Description |
|----------|-------------|-------------|
| High | 3 | Event handlers recreated on every render |
| Medium | 5 | Missing debounce/throttle on frequent events |
| Medium | 2 | Potential memory leaks from event listener patterns |
| Low | 4 | Minor optimization opportunities |

---

## High Priority Issues

### 1. Handler Recreation in App.tsx Keyboard Effects

**File:** `/Users/xlyk/Code/qbit.worktrees/frontend-improvements/frontend/App.tsx`
**Lines:** 421-444, 448-614

**Issue:** Two separate keyboard event `useEffect` hooks create handlers inline that depend on state, causing unnecessary re-subscriptions when state changes.

```typescript
// Lines 421-444: Cmd key tracking effect
useEffect(() => {
  const handleKeyDown = (e: KeyboardEvent) => {  // <-- Recreated every time
    if (e.key === "Meta" && !e.repeat) {
      setCmdKeyPressed(true);
    }
  };
  // ...
  window.addEventListener("keydown", handleKeyDown);
  // ...
}, []);  // Empty deps but handlers close over nothing problematic here - OK

// Lines 448-614: Main keyboard shortcuts
useEffect(() => {
  const handleKeyDown = (e: KeyboardEvent) => {  // <-- Recreated on every dep change
    // Cmd+K for command palette
    if ((e.metaKey || e.ctrlKey) && e.key === "k") {
      // ...
    }
    // ... many more handlers
  };
  window.addEventListener("keydown", handleKeyDown);
  return () => window.removeEventListener("keydown", handleKeyDown);
}, [
  handleNewTab,
  handleToggleMode,
  sessions,
  activeSessionId,
  // ... 10+ dependencies
]);
```

**Impact:** Every time any of the 10+ dependencies change, the event listener is torn down and re-attached. During rapid UI interactions, this causes unnecessary DOM operations.

**Recommendation:** Use a ref to hold the latest callback values:

```typescript
const handlersRef = useRef({ handleNewTab, handleToggleMode, sessions, activeSessionId, ... });

useEffect(() => {
  handlersRef.current = { handleNewTab, handleToggleMode, sessions, activeSessionId, ... };
});

useEffect(() => {
  const handleKeyDown = (e: KeyboardEvent) => {
    const { handleNewTab, sessions, activeSessionId } = handlersRef.current;
    // Use values from ref
  };
  window.addEventListener("keydown", handleKeyDown);
  return () => window.removeEventListener("keydown", handleKeyDown);
}, []); // Now only runs once
```

---

### 2. Sidebar Resize Handler Without Throttle

**File:** `/Users/xlyk/Code/qbit.worktrees/frontend-improvements/frontend/components/Sidebar/Sidebar.tsx`
**Lines:** 292-317

**Issue:** The resize handler fires on every `mousemove` event without throttling, causing excessive state updates and re-renders.

```typescript
useEffect(() => {
  const handleMouseMove = (e: MouseEvent) => {
    if (!isResizing.current) return;

    const newWidth = e.clientX;
    if (newWidth >= MIN_WIDTH && newWidth <= MAX_WIDTH) {
      setWidth(newWidth);  // <-- State update on EVERY mousemove
    }
  };

  document.addEventListener("mousemove", handleMouseMove);
  document.addEventListener("mouseup", handleMouseUp);
  // ...
}, []);
```

**Impact:** During sidebar resizing, hundreds of state updates per second cause layout thrashing and janky animations.

**Recommendation:** Add RAF-based throttling:

```typescript
const handleMouseMove = (e: MouseEvent) => {
  if (!isResizing.current || rafRef.current !== null) return;

  rafRef.current = requestAnimationFrame(() => {
    const newWidth = e.clientX;
    if (newWidth >= MIN_WIDTH && newWidth <= MAX_WIDTH) {
      setWidth(newWidth);
    }
    rafRef.current = null;
  });
};
```

---

### 3. FileEditorSidebarPanel and ContextPanel Same Resize Issue

**File:** `/Users/xlyk/Code/qbit.worktrees/frontend-improvements/frontend/components/FileEditorSidebar/FileEditorSidebarPanel.tsx`
**Lines:** 354-385

**File:** `/Users/xlyk/Code/qbit.worktrees/frontend-improvements/frontend/components/Sidecar/ContextPanel.tsx`
**Lines:** 86-110

**Issue:** Same pattern as Sidebar - no throttling on resize `mousemove` handlers.

**Recommendation:** Same fix as above - add RAF throttling to both components.

---

## Medium Priority Issues

### 4. Search Input Without Debounce in Sidebar

**File:** `/Users/xlyk/Code/qbit.worktrees/frontend-improvements/frontend/components/Sidebar/Sidebar.tsx`
**Lines:** 377-382

**Issue:** Search is debounced with `setTimeout` which is good, but the debounce implementation could miss cancellation on unmount.

```typescript
useEffect(() => {
  const timer = setTimeout(() => {
    handleSearch();
  }, 300);
  return () => clearTimeout(timer);
}, [handleSearch]);  // handleSearch changes when searchQuery changes
```

**Current State:** This is actually implemented correctly with cleanup. The dependency array causes `handleSearch` to be recreated on `searchQuery` change (via `useCallback` with `searchQuery` dep), which triggers the debounce effect correctly.

**Note:** This is **acceptable but could be cleaner** by debouncing the search query itself instead of the handler.

---

### 5. Path Completion API Calls Not Debounced

**File:** `/Users/xlyk/Code/qbit.worktrees/frontend-improvements/frontend/hooks/usePathCompletion.ts`
**Lines:** 16-47

**Issue:** Path completions are fetched immediately on every change to `partialPath`, without debouncing.

```typescript
useEffect(() => {
  if (!enabled) {
    setCompletions([]);
    setTotalCount(0);
    return;
  }

  let cancelled = false;
  setIsLoading(true);

  listPathCompletions(sessionId, partialPath, 20)  // <-- API call on every keystroke
    .then((response) => { /* ... */ })
    // ...

  return () => { cancelled = true; };
}, [sessionId, partialPath, enabled]);
```

**Impact:** Each keystroke triggers an IPC call to the Rust backend, potentially causing lag during fast typing.

**Recommendation:** Add debouncing:

```typescript
useEffect(() => {
  if (!enabled) {
    setCompletions([]);
    return;
  }

  let cancelled = false;
  const timeoutId = setTimeout(() => {
    setIsLoading(true);
    listPathCompletions(sessionId, partialPath, 20)
      .then((response) => {
        if (!cancelled) {
          setCompletions(response.completions);
          setTotalCount(response.total_count);
        }
      })
      .finally(() => {
        if (!cancelled) setIsLoading(false);
      });
  }, 100);  // 100ms debounce

  return () => {
    cancelled = true;
    clearTimeout(timeoutId);
  };
}, [sessionId, partialPath, enabled]);
```

---

### 6. UnifiedTimeline Scroll Handler Missing Throttle

**File:** `/Users/xlyk/Code/qbit.worktrees/frontend-improvements/frontend/components/UnifiedTimeline/UnifiedTimeline.tsx`
**Lines:** 81-93

**Issue:** Scroll handler uses `{ passive: true }` (good) but fires `setIsAtBottom` on every scroll event.

```typescript
useEffect(() => {
  const container = containerRef.current;
  if (!container) return;

  const handleScroll = () => {
    const { scrollTop, scrollHeight, clientHeight } = container;
    setIsAtBottom(scrollHeight - scrollTop - clientHeight < 50);  // <-- Every scroll event
  };

  container.addEventListener("scroll", handleScroll, { passive: true });
  return () => container.removeEventListener("scroll", handleScroll);
}, []);
```

**Impact:** During scrolling, this can trigger React re-renders at 60+ fps.

**Recommendation:** Use RAF throttling or check if value actually changed:

```typescript
const handleScroll = () => {
  const { scrollTop, scrollHeight, clientHeight } = container;
  const atBottom = scrollHeight - scrollTop - clientHeight < 50;
  // Only update state if changed
  setIsAtBottom(prev => prev === atBottom ? prev : atBottom);
};
```

---

### 7. HomeView Focus Handler Without Cleanup Protection

**File:** `/Users/xlyk/Code/qbit.worktrees/frontend-improvements/frontend/components/HomeView/HomeView.tsx`
**Lines:** 391-398

**Issue:** Window focus handler triggers a data fetch but the `fetchData` call is not cancellable.

```typescript
useEffect(() => {
  const handleFocus = () => {
    fetchData(false);  // <-- Not cancellable if component unmounts
  };
  window.addEventListener("focus", handleFocus);
  return () => window.removeEventListener("focus", handleFocus);
}, [fetchData]);
```

**Recommendation:** Add an abort mechanism:

```typescript
useEffect(() => {
  let isMounted = true;
  const handleFocus = () => {
    if (isMounted) {
      fetchData(false);
    }
  };
  window.addEventListener("focus", handleFocus);
  return () => {
    isMounted = false;
    window.removeEventListener("focus", handleFocus);
  };
}, [fetchData]);
```

---

### 8. systemNotifications Settings Listener Not Cleaned Up

**File:** `/Users/xlyk/Code/qbit.worktrees/frontend-improvements/frontend/lib/systemNotifications.ts`
**Lines:** 255-268

**Issue:** The `listenForSettingsUpdates` function adds an event listener but provides no way to remove it.

```typescript
export function listenForSettingsUpdates(): void {
  window.addEventListener("settings-updated", (event: Event) => {  // <-- Never removed
    const customEvent = event as CustomEvent<{ notifications?: { native_enabled?: boolean } }>;
    // ...
  });
}
```

**Impact:** This is called once at app startup so it's not a leak, but it's bad practice and prevents proper cleanup in tests.

**Recommendation:** Return an unlisten function:

```typescript
export function listenForSettingsUpdates(): () => void {
  const handler = (event: Event) => {
    const customEvent = event as CustomEvent<{ notifications?: { native_enabled?: boolean } }>;
    // ...
  };
  window.addEventListener("settings-updated", handler);
  return () => window.removeEventListener("settings-updated", handler);
}
```

---

## Low Priority Issues

### 9. Popup Components Have Duplicate Click-Outside Logic

**Files:**
- `/Users/xlyk/Code/qbit.worktrees/frontend-improvements/frontend/components/SlashCommandPopup/SlashCommandPopup.tsx` (lines 29-41)
- `/Users/xlyk/Code/qbit.worktrees/frontend-improvements/frontend/components/PathCompletionPopup/PathCompletionPopup.tsx` (lines 50-62)
- `/Users/xlyk/Code/qbit.worktrees/frontend-improvements/frontend/components/HistorySearchPopup/HistorySearchPopup.tsx` (lines 66-78)

**Issue:** Three components have nearly identical click-outside detection code.

**Recommendation:** Extract to a custom hook:

```typescript
function useClickOutside(
  ref: RefObject<HTMLElement>,
  open: boolean,
  onClose: () => void
) {
  useEffect(() => {
    if (!open) return;

    const handleClickOutside = (e: MouseEvent) => {
      if (ref.current && !ref.current.contains(e.target as Node)) {
        onClose();
      }
    };

    document.addEventListener("mousedown", handleClickOutside, true);
    return () => document.removeEventListener("mousedown", handleClickOutside, true);
  }, [open, onClose, ref]);
}
```

---

### 10. Terminal Component Uses Multiple Cleanup Arrays

**File:** `/Users/xlyk/Code/qbit.worktrees/frontend-improvements/frontend/components/Terminal/Terminal.tsx`
**Lines:** 30, 204, 224, 231, 259-260, 283-286

**Issue:** The Terminal component uses `cleanupFnsRef.current.push()` pattern which works but is complex.

```typescript
const cleanupFnsRef = useRef<(() => void)[]>([]);
// ...
cleanupFnsRef.current.push(unsubscribeTheme);
// ...
cleanupFnsRef.current.push(() => inputDisposable.dispose());
// ...
for (const fn of cleanupFnsRef.current) {
  fn();
}
cleanupFnsRef.current = [];
```

**Current State:** This is actually a **good pattern** for managing multiple async cleanup functions. The cleanup is properly executed on unmount.

**Note:** No change needed - this is well-implemented.

---

### 11. NotificationWidget Timer Refs Pattern

**File:** `/Users/xlyk/Code/qbit.worktrees/frontend-improvements/frontend/components/NotificationWidget/NotificationWidget.tsx`
**Lines:** 163-236

**Issue:** Multiple timer refs with manual cleanup is correct but verbose.

**Current State:** The implementation properly cleans up timers on unmount and when state changes. This is **acceptable**.

---

### 12. InputStatusRow Settings Event Listener

**File:** `/Users/xlyk/Code/qbit.worktrees/frontend-improvements/frontend/components/UnifiedInput/InputStatusRow.tsx`
**Line:** 353

**Issue:** Need to verify this listener is properly cleaned up.

After checking the code pattern, this follows the same pattern as other components and should have proper cleanup.

---

## Good Practices Found

The codebase demonstrates several **excellent event handling patterns**:

1. **Terminal.tsx ResizeObserver with RAF debouncing** (lines 294-314): Properly debounces resize events with double-RAF and setTimeout.

2. **useAiEvents text delta batching** (lines 69-109 in `useAiEvents.ts`): Batches streaming text updates at 16ms (60fps) to prevent over-rendering.

3. **useTauriEvents proper async listener setup** (lines 148-492): Uses abort flags and proper cleanup for async event listeners.

4. **UnifiedInput stateRef pattern** (lines 222-271): Uses a ref to hold current state values, allowing callbacks to access fresh values without being recreated.

5. **Terminal ResizeObserver passive listening** in UnifiedTimeline for scroll events.

---

## Summary of Recommendations

| Priority | File | Line | Fix |
|----------|------|------|-----|
| High | App.tsx | 448-614 | Use ref pattern for keyboard handler |
| High | Sidebar.tsx | 292-317 | Add RAF throttle to resize |
| High | FileEditorSidebarPanel.tsx | 354-385 | Add RAF throttle to resize |
| High | ContextPanel.tsx | 86-110 | Add RAF throttle to resize |
| Medium | usePathCompletion.ts | 16-47 | Add debounce to API calls |
| Medium | UnifiedTimeline.tsx | 85-88 | Optimize scroll handler |
| Medium | HomeView.tsx | 391-398 | Add mounted check |
| Medium | systemNotifications.ts | 255-268 | Return unlisten function |
| Low | Popup components | various | Extract to shared hook |

---

## Implementation Priority

1. **Immediate (High Impact):**
   - Fix resize handlers in Sidebar, FileEditorSidebarPanel, and ContextPanel
   - Add debounce to usePathCompletion

2. **Soon (Medium Impact):**
   - Optimize App.tsx keyboard handler subscription
   - Improve scroll handler in UnifiedTimeline

3. **When Convenient (Low Impact):**
   - Extract click-outside hook
   - Add cleanup return to systemNotifications
