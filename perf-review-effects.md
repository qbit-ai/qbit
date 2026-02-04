# React Effects & Hooks Performance Review

## Summary

This review analyzes the frontend codebase for React effect-related performance issues, focusing on `useEffect` patterns, dependency arrays, cleanup functions, and opportunities to use more appropriate hooks.

### Key Findings

| Priority | Category | Issues Found |
|----------|----------|--------------|
| High | Missing Dependencies / Stale Closures | 3 |
| High | Effects That Could Be Event Handlers | 2 |
| Medium | Redundant Effects / Synchronization Chains | 4 |
| Medium | Missing Cleanup Functions | 2 |
| Low | Optimization Opportunities | 5 |

---

## High Priority Issues

### 1. InputStatusRow: Duplicated Settings Update Logic

**File:** `/frontend/components/UnifiedInput/InputStatusRow.tsx`
**Lines:** 290-357

**Problem:** The component has two nearly identical effects that fetch and set provider settings:
1. `useEffect` at line 290 that calls `refreshProviderSettings()`
2. `useEffect` at line 295 that listens for `settings-updated` events and duplicates all the same state updates

The second effect contains ~60 lines of duplicated code from `refreshProviderSettings`.

**Impact:** Code duplication increases maintenance burden and risk of bugs when one is updated but not the other.

**Recommended Fix:**
```tsx
// Remove the duplicated code in the second effect and reuse refreshProviderSettings
useEffect(() => {
  refreshProviderSettings();
}, [refreshProviderSettings]);

useEffect(() => {
  const handleSettingsUpdated = () => {
    refreshProviderSettings();
  };

  window.addEventListener("settings-updated", handleSettingsUpdated);
  return () => {
    window.removeEventListener("settings-updated", handleSettingsUpdated);
  };
}, [refreshProviderSettings]);
```

---

### 2. UnifiedInput: State Ref Pattern Without Dependency Management

**File:** `/frontend/components/UnifiedInput/UnifiedInput.tsx`
**Lines:** 247-271

**Problem:** The component maintains a `stateRef` that is updated every render to hold current state values. While this is a valid pattern for stable callbacks, the effect updating the ref has no dependency array, meaning it runs on every single render.

```tsx
// Current code - runs on EVERY render
useEffect(() => {
  stateRef.current = {
    input,
    inputMode,
    isAgentBusy,
    // ... 20+ more fields
  };
});
```

**Impact:** This effect runs synchronously after every render, creating an object with 20+ fields each time. While the effect itself is fast, it contributes to render overhead.

**Recommended Fix:**
```tsx
// Option 1: Use useRef with direct assignment (no effect needed)
// Update stateRef.current directly in render, not via effect
stateRef.current = {
  input,
  inputMode,
  isAgentBusy,
  // ...
};

// Option 2: If effect is truly needed, consider if all fields need to be in the ref
// or if individual refs for frequently-accessed values would be more efficient
```

---

### 3. useSlashCommands: Unnecessary Effect Dependency on useCallback

**File:** `/frontend/hooks/useSlashCommands.ts`
**Lines:** 77-79

**Problem:** The effect depends on `loadCommands` which is a `useCallback` that depends on `workingDirectory`. When `workingDirectory` changes, both the callback and the effect re-run.

```tsx
const loadCommands = useCallback(async () => {
  // ...
}, [workingDirectory]);

useEffect(() => {
  loadCommands();
}, [loadCommands]); // This re-runs when workingDirectory changes
```

**Impact:** Minor - the pattern works correctly but is less clear than directly depending on `workingDirectory`.

**Recommended Fix:**
```tsx
// Option 1: Depend directly on workingDirectory
useEffect(() => {
  loadCommands();
}, [workingDirectory]); // eslint-disable-line react-hooks/exhaustive-deps

// Option 2: If loadCommands needs to be stable, ensure it's properly memoized
// and add a comment explaining the dependency relationship
```

---

## Medium Priority Issues

### 4. useCommandHistory: Potential Over-Fetching on entryType Change

**File:** `/frontend/hooks/useCommandHistory.ts`
**Lines:** 42-64

**Problem:** The effect re-fetches history whenever `initialHistory` changes. Since `initialHistory` defaults to `EMPTY_HISTORY` (a constant), this shouldn't cause issues, but if a caller passes a new array reference on each render, it would cause unnecessary fetches.

```tsx
useEffect(() => {
  loadHistory(limit, entryType)
    .then((entries) => {
      // ...
    });
  // ...
}, [entryType, limit, initialHistory]); // initialHistory in deps could be problematic
```

**Recommended Fix:**
```tsx
// Use useMemo for initialHistory in the calling component, or
// remove initialHistory from deps if it's only used as fallback
useEffect(() => {
  loadHistory(limit, entryType)
    .then((entries) => {
      if (cancelled) return;
      setHistory(entries.map((e) => e.c));
      // ...
    })
    .catch(() => {
      if (cancelled) return;
      // Use ref for initialHistory fallback
      setHistory(initialHistoryRef.current);
      // ...
    });
}, [entryType, limit]); // Remove initialHistory
```

---

### 5. SessionBrowser: Cascading State Updates in Effects

**File:** `/frontend/components/SessionBrowser/SessionBrowser.tsx`
**Lines:** 110-137

**Problem:** Two separate effects handle related state:
1. First effect loads sessions when dialog opens
2. Second effect filters sessions when `searchQuery` or `sessions` change

This creates a cascade: open dialog -> load sessions -> sessions state updates -> filter effect runs.

```tsx
useEffect(() => {
  if (open) {
    loadSessions();
  } else {
    setSelectedSession(null);
    setSessionDetail(null);
    setSearchQuery("");
  }
}, [open, loadSessions]);

useEffect(() => {
  if (!searchQuery.trim()) {
    setFilteredSessions(sessions);
    return;
  }
  const filtered = sessions.filter(/* ... */);
  setFilteredSessions(filtered);
}, [searchQuery, sessions]);
```

**Impact:** Extra render cycle due to cascade. First render: sessions loaded. Second render: filtering applied.

**Recommended Fix:**
```tsx
// Combine into derived state using useMemo instead of effect
const filteredSessions = useMemo(() => {
  if (!searchQuery.trim()) return sessions;
  const query = searchQuery.toLowerCase();
  return sessions.filter(/* ... */);
}, [searchQuery, sessions]);
```

---

### 6. NotificationWidget: Multiple Independent Effects for Related Logic

**File:** `/frontend/components/NotificationWidget/NotificationWidget.tsx`
**Lines:** 168-266

**Problem:** The component has 5 separate `useEffect` hooks that could potentially be consolidated:
1. Watch for new notifications and show preview
2. Clear preview when panel is expanded
3. Cleanup timers on unmount
4. Close on click outside
5. Close on escape

While each effect has a clear purpose, effects 4 and 5 (click outside and escape handlers) are closely related and could share event listener setup.

**Recommended Fix:**
```tsx
// Combine click outside and escape into a single effect
useEffect(() => {
  if (!isExpanded) return;

  function handleClickOutside(event: MouseEvent) {
    if (
      panelRef.current &&
      triggerRef.current &&
      !panelRef.current.contains(event.target as Node) &&
      !triggerRef.current.contains(event.target as Node)
    ) {
      setExpanded(false);
    }
  }

  function handleEscape(event: KeyboardEvent) {
    if (event.key === "Escape") {
      setExpanded(false);
    }
  }

  document.addEventListener("mousedown", handleClickOutside);
  document.addEventListener("keydown", handleEscape);

  return () => {
    document.removeEventListener("mousedown", handleClickOutside);
    document.removeEventListener("keydown", handleEscape);
  };
}, [isExpanded, setExpanded]);
```

---

### 7. ContextPanel: Missing Cancellation for Async Operations

**File:** `/frontend/components/Sidecar/ContextPanel.tsx`
**Lines:** 203-213

**Problem:** The artifact preview effect fetches data but doesn't cancel the operation if the component unmounts or the selection changes.

```tsx
useEffect(() => {
  if (!selectedArtifact || !resolvedSessionId) {
    setArtifactPreview(null);
    return;
  }

  setArtifactPreview(null);
  previewArtifact(resolvedSessionId, selectedArtifact)
    .then(setArtifactPreview) // Could set state on unmounted component
    .catch(() => setArtifactPreview("Failed to load preview"));
}, [selectedArtifact, resolvedSessionId]);
```

**Recommended Fix:**
```tsx
useEffect(() => {
  if (!selectedArtifact || !resolvedSessionId) {
    setArtifactPreview(null);
    return;
  }

  let cancelled = false;
  setArtifactPreview(null);

  previewArtifact(resolvedSessionId, selectedArtifact)
    .then((preview) => {
      if (!cancelled) setArtifactPreview(preview);
    })
    .catch(() => {
      if (!cancelled) setArtifactPreview("Failed to load preview");
    });

  return () => {
    cancelled = true;
  };
}, [selectedArtifact, resolvedSessionId]);
```

---

### 8. SlashCommandPopup: Three Effects for Popup Behavior

**File:** `/frontend/components/SlashCommandPopup/SlashCommandPopup.tsx`
**Lines:** 29-58

**Problem:** Three separate effects handle popup behavior (click outside, window blur, scroll into view). These could be consolidated.

```tsx
// Effect 1: Click outside
useEffect(() => {
  if (!open) return;
  const handleClickOutside = /* ... */;
  document.addEventListener("mousedown", handleClickOutside, true);
  return () => document.removeEventListener("mousedown", handleClickOutside, true);
}, [open, onOpenChange]);

// Effect 2: Window blur
useEffect(() => {
  if (!open) return;
  const handleBlur = () => onOpenChange(false);
  window.addEventListener("blur", handleBlur);
  return () => window.removeEventListener("blur", handleBlur);
}, [open, onOpenChange]);

// Effect 3: Scroll into view
useEffect(() => {
  if (open && listRef.current) {
    const selectedElement = listRef.current.querySelector(`[data-index="${selectedIndex}"]`);
    selectedElement?.scrollIntoView({ block: "nearest" });
  }
}, [selectedIndex, open]);
```

**Recommended Fix:**
```tsx
// Combine click outside and blur into one effect
useEffect(() => {
  if (!open) return;

  const handleClickOutside = (e: MouseEvent) => {
    if (containerRef.current && !containerRef.current.contains(e.target as Node)) {
      onOpenChange(false);
    }
  };

  const handleBlur = () => onOpenChange(false);

  document.addEventListener("mousedown", handleClickOutside, true);
  window.addEventListener("blur", handleBlur);

  return () => {
    document.removeEventListener("mousedown", handleClickOutside, true);
    window.removeEventListener("blur", handleBlur);
  };
}, [open, onOpenChange]);
```

---

## Low Priority Issues

### 9. usePathCompletion: Could Use Debouncing

**File:** `/frontend/hooks/usePathCompletion.ts`
**Lines:** 16-47

**Problem:** Path completion requests are made on every change to `partialPath`. For fast typists, this could result in many unnecessary API calls.

**Recommended Fix:**
```tsx
// Add debouncing to reduce API calls
useEffect(() => {
  if (!enabled) {
    setCompletions([]);
    setTotalCount(0);
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
      .catch(/* ... */)
      .finally(() => {
        if (!cancelled) setIsLoading(false);
      });
  }, 100); // 100ms debounce

  return () => {
    cancelled = true;
    clearTimeout(timeoutId);
  };
}, [sessionId, partialPath, enabled]);
```

---

### 10. useTauriEvents: Large Effect with Many Responsibilities

**File:** `/frontend/hooks/useTauriEvents.ts`
**Lines:** 148-493

**Problem:** This single effect handles multiple event types with complex logic. While the biome-ignore comment suppresses the exhaustive-deps warning, the effect is doing a lot:
- Terminal output handling
- Command block lifecycle
- Directory change handling
- Virtual environment changes
- Session ended handling
- Alternate screen detection
- Git status polling

**Impact:** Difficult to maintain and reason about. Changes to one event handler could inadvertently affect others.

**Recommended Fix:** Consider splitting into smaller, focused effects or extracting event handlers into separate files:
```tsx
// Option 1: Use separate hooks for each event category
useCommandBlockEvents();
useTerminalOutputEvents();
useDirectoryChangeEvents();
useGitStatusPolling();

// Option 2: Extract handlers to a separate file and keep one orchestrating effect
import { createEventHandlers } from './tauri-event-handlers';
```

---

### 11. useTheme: Three Separate Subscription Effects

**File:** `/frontend/hooks/useTheme.tsx`
**Lines:** 34-80

**Problem:** Three separate effects handle theme initialization and subscription:
1. Initialize theme on mount
2. Subscribe to theme changes
3. Subscribe to registry changes

These are all related to theme management and could potentially be consolidated.

**Impact:** Minor - adds complexity but doesn't cause performance issues.

---

### 12. App.tsx: Keyboard Shortcuts Effect with Many Dependencies

**File:** `/frontend/App.tsx`
**Lines:** 447-629

**Problem:** The keyboard shortcuts effect has 12 dependencies in its array, making it re-create the handler frequently.

```tsx
useEffect(() => {
  const handleKeyDown = (e: KeyboardEvent) => {
    // ~180 lines of keyboard handling
  };

  window.addEventListener("keydown", handleKeyDown);
  return () => window.removeEventListener("keydown", handleKeyDown);
}, [
  handleNewTab,
  handleToggleMode,
  sessions,
  activeSessionId,
  openContextPanel,
  toggleFileEditorPanel,
  openGitPanel,
  gitPanelOpen,
  setRenderMode,
  handleSplitPane,
  handleClosePane,
  handleNavigatePane,
  openSettingsTab,
]);
```

**Recommended Fix:**
```tsx
// Use refs for values that don't need to trigger re-subscription
const sessionsRef = useRef(sessions);
const activeSessionIdRef = useRef(activeSessionId);

useEffect(() => {
  sessionsRef.current = sessions;
  activeSessionIdRef.current = activeSessionId;
});

useEffect(() => {
  const handleKeyDown = (e: KeyboardEvent) => {
    // Use refs instead of direct values
    const currentSessions = sessionsRef.current;
    const currentActiveId = activeSessionIdRef.current;
    // ...
  };

  window.addEventListener("keydown", handleKeyDown);
  return () => window.removeEventListener("keydown", handleKeyDown);
}, [/* only stable callbacks */]);
```

---

### 13. Terminal.tsx: Complex Initialization Effect

**File:** `/frontend/components/Terminal/Terminal.tsx`
**Lines:** 109-352

**Problem:** The main terminal effect is ~240 lines and handles:
- Terminal instance creation/reattachment
- Theme subscription
- PTY resize
- Input handling
- Resize observer setup
- Event listener setup (async)
- Focus event handling

**Impact:** While functional, the complexity makes it hard to understand and maintain.

**Recommended Fix:** Extract logical sections into helper functions:
```tsx
function setupTerminalInstance(sessionId, containerRef, terminalRef, fitAddonRef) {
  // ~80 lines of terminal creation logic
}

function setupEventListeners(sessionId, terminal, syncBufferRef, cleanupFns) {
  // ~50 lines of event listener setup
}

function setupResizeObserver(containerRef, handleResize) {
  // ~30 lines of resize observer logic
}
```

---

## General Recommendations

### 1. Consider Using Custom Hooks for Repeated Patterns

Several components implement similar patterns for:
- Click outside detection
- Escape key handling
- Async data fetching with cancellation

Create reusable hooks:
```tsx
function useClickOutside(ref, handler) { /* ... */ }
function useEscapeKey(handler) { /* ... */ }
function useFetch(asyncFn, deps) { /* ... */ }
```

### 2. Replace State + Effect with useMemo for Derived State

Multiple components use this pattern:
```tsx
const [derived, setDerived] = useState(initialValue);
useEffect(() => {
  setDerived(computeFromDeps(deps));
}, [deps]);
```

Use `useMemo` instead:
```tsx
const derived = useMemo(() => computeFromDeps(deps), [deps]);
```

### 3. Document Complex Effects

Add comments explaining:
- Why the effect exists
- What triggers it to re-run
- Any intentional dependency omissions

### 4. Consider Effect Cleanup Audit

Run through all async effects to ensure:
- Cancellation flags are set in cleanup
- Timers are cleared
- Subscriptions are unsubscribed

---

## Files Reviewed

- `/frontend/hooks/useAiEvents.ts` - Well structured with proper cleanup
- `/frontend/hooks/useTauriEvents.ts` - Large but functional, could be split
- `/frontend/hooks/usePathCompletion.ts` - Good, could add debouncing
- `/frontend/hooks/useCommandHistory.ts` - Minor dependency issue
- `/frontend/hooks/useSidecarEvents.ts` - Well structured
- `/frontend/hooks/useSlashCommands.ts` - Minor optimization possible
- `/frontend/hooks/useCreateTerminalTab.ts` - No effects, uses callbacks correctly
- `/frontend/hooks/useTheme.tsx` - Three related effects could be combined
- `/frontend/hooks/useTerminalPortal.tsx` - Uses useSyncExternalStore correctly
- `/frontend/hooks/useFileIndex.ts` - Good async handling
- `/frontend/hooks/useHistorySearch.ts` - Uses useMemo correctly (no effects)
- `/frontend/hooks/useFileEditorSidebar.ts` - Uses useMemo correctly (no effects)
- `/frontend/components/Terminal/Terminal.tsx` - Complex but functional
- `/frontend/components/UnifiedInput/InputStatusRow.tsx` - Duplicated logic
- `/frontend/components/UnifiedInput/UnifiedInput.tsx` - State ref pattern review
- `/frontend/components/SlashCommandPopup/SlashCommandPopup.tsx` - Could consolidate
- `/frontend/components/SessionBrowser/SessionBrowser.tsx` - Cascading state
- `/frontend/components/Sidecar/SidecarPanel.tsx` - Good structure
- `/frontend/components/Sidecar/ContextPanel.tsx` - Missing cancellation
- `/frontend/components/NotificationWidget/NotificationWidget.tsx` - Many effects
- `/frontend/components/CommandPalette/CommandPalette.tsx` - No effects (callbacks only)
- `/frontend/App.tsx` - Complex keyboard effect
