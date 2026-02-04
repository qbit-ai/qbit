# Data Fetching Performance Review

This document identifies performance issues related to Tauri invoke patterns, caching strategies, request deduplication, and async state management in the Qbit frontend.

## Summary

The codebase generally has decent async patterns, with some notable existing mitigations (e.g., indexer deduplication). However, several areas could benefit from improvement:

1. **Missing caching** for static/rarely-changing data
2. **Missing request deduplication** in several components
3. **Waterfall requests** that could be parallelized
4. **Potential race conditions** in async state management
5. **Event listener cleanup patterns** that could fail in edge cases

---

## High Priority Issues

### 1. Settings Loaded Multiple Times Without Caching

**Files:**
- `frontend/hooks/useTauriEvents.ts:169-176`
- `frontend/hooks/useCreateTerminalTab.ts:39`
- `frontend/components/Settings/index.tsx:117-124`

**Problem:**
`getSettings()` is called on every terminal tab creation, every event hook initialization, and every settings dialog open without any caching layer. Settings rarely change during a session.

```typescript
// useTauriEvents.ts:169-176
getSettings()
  .then((settings) => {
    const userCommands = settings.terminal.fullterm_commands ?? [];
    fulltermCommands = new Set([...BUILTIN_FULLTERM_COMMANDS, ...userCommands]);
  })
```

```typescript
// useCreateTerminalTab.ts:39
const settings = await getSettings();
```

**Impact:** Redundant IPC calls on every tab creation. Settings typically only change when user explicitly modifies them.

**Recommended Fix:**
Implement a simple cache with TTL or invalidation on settings-updated event:

```typescript
// frontend/lib/settings.ts
let settingsCache: QbitSettings | null = null;
let settingsCacheTime = 0;
const CACHE_TTL_MS = 5000; // 5 seconds

export async function getSettingsCached(): Promise<QbitSettings> {
  const now = Date.now();
  if (settingsCache && now - settingsCacheTime < CACHE_TTL_MS) {
    return settingsCache;
  }
  settingsCache = await getSettings();
  settingsCacheTime = now;
  return settingsCache;
}

// Invalidate on update
export async function updateSettings(settings: QbitSettings): Promise<void> {
  settingsCache = null; // Clear cache
  return invoke("update_settings", { settings });
}
```

---

### 2. Git Status Polling Without Deduplication

**File:** `frontend/hooks/useTauriEvents.ts:463-474`

**Problem:**
Git status is polled every 5 seconds for ALL active sessions, regardless of whether the previous request has completed. This can lead to multiple concurrent requests for the same session.

```typescript
// Poll interval without tracking active requests
const gitStatusPollInterval = setInterval(() => {
  const state = store.getState();
  const sessions = state.sessions;
  for (const sessionId of Object.keys(sessions)) {
    const session = sessions[sessionId];
    if (session?.workingDirectory) {
      refreshGitInfo(sessionId, session.workingDirectory);
    }
  }
}, GIT_STATUS_POLL_INTERVAL_MS);
```

While `refreshGitInfo` uses a sequence number to ignore stale results, it still fires the requests.

**Impact:** Wasted IPC calls when git operations are slow. Backend may be processing redundant status checks.

**Recommended Fix:**
Track in-flight requests per session:

```typescript
const gitRefreshInFlight = new Map<string, boolean>();

function refreshGitInfo(sessionId: string, cwd: string) {
  // Skip if request already in flight for this session
  if (gitRefreshInFlight.get(sessionId)) return;

  gitRefreshInFlight.set(sessionId, true);
  const state = store.getState();
  // ... existing logic
  void (async () => {
    try {
      // ... existing fetch logic
    } finally {
      gitRefreshInFlight.delete(sessionId);
      // ... existing finally logic
    }
  })();
}
```

---

### 3. Waterfall Requests in useCreateTerminalTab

**File:** `frontend/hooks/useCreateTerminalTab.ts:38-111`

**Problem:**
PTY creation, settings fetch, and project settings fetch are done sequentially when they could be partially parallelized.

```typescript
// Current sequential pattern:
const session = await ptyCreate(workingDirectory);  // Wait
const settings = await getSettings();               // Wait
projectSettings = await getProjectSettings(session.working_directory); // Wait
// Then git operations...
```

**Impact:** Slower tab creation time, especially on first launch.

**Recommended Fix:**
Load settings in parallel with PTY creation:

```typescript
const createTerminalTab = useCallback(async (workingDirectory?: string): Promise<string | null> => {
  try {
    // Start settings load immediately - don't wait for PTY
    const settingsPromise = getSettings();

    // Create PTY (this is what actually creates the session)
    const session = await ptyCreate(workingDirectory);

    // Wait for settings (likely already resolved)
    const settings = await settingsPromise;

    // Project settings must wait for session.working_directory
    const projectSettings = await getProjectSettings(session.working_directory);

    // ... rest of the logic
  }
});
```

---

### 4. HomeView Data Fetched on Every Window Focus

**File:** `frontend/components/HomeView/HomeView.tsx:392-398`

**Problem:**
Data is re-fetched on every window focus without debounce or cache:

```typescript
useEffect(() => {
  const handleFocus = () => {
    fetchData(false);  // Called on EVERY focus event
  };
  window.addEventListener("focus", handleFocus);
  return () => window.removeEventListener("focus", handleFocus);
}, [fetchData]);
```

**Impact:** Rapid focus/unfocus (e.g., clicking between windows) triggers multiple redundant fetches.

**Recommended Fix:**
Add debounce or minimum interval:

```typescript
useEffect(() => {
  let lastFetchTime = 0;
  const MIN_FETCH_INTERVAL_MS = 5000;

  const handleFocus = () => {
    const now = Date.now();
    if (now - lastFetchTime >= MIN_FETCH_INTERVAL_MS) {
      lastFetchTime = now;
      fetchData(false);
    }
  };
  window.addEventListener("focus", handleFocus);
  return () => window.removeEventListener("focus", handleFocus);
}, [fetchData]);
```

---

## Medium Priority Issues

### 5. Slash Commands Reloaded on Every Working Directory Change

**File:** `frontend/hooks/useSlashCommands.ts:77-79`

**Problem:**
Skills and prompts are re-fetched whenever `workingDirectory` changes. While local skills can differ per workspace, the global skills are the same.

```typescript
useEffect(() => {
  loadCommands();
}, [loadCommands]); // loadCommands depends on workingDirectory
```

**Impact:** Redundant IPC calls when switching between directories in the same session.

**Recommended Fix:**
Cache global skills separately and only refresh local skills on directory change:

```typescript
// Cache global skills at module level
let globalSkillsCache: SkillInfo[] | null = null;

const loadCommands = useCallback(async () => {
  setIsLoading(true);
  try {
    // Load global skills from cache or fetch once
    if (!globalSkillsCache) {
      globalSkillsCache = await listSkills(); // no workingDirectory = global only
    }

    // Always fetch local skills for current directory
    const [localSkills, prompts] = await Promise.all([
      listSkills(workingDirectory),
      listPrompts(workingDirectory),
    ]);

    // Merge global + local (local overrides)
    // ...
  }
});
```

---

### 6. usePathCompletion Has No Debounce

**File:** `frontend/hooks/usePathCompletion.ts:16-47`

**Problem:**
Path completions are fetched on every keystroke change to `partialPath`:

```typescript
useEffect(() => {
  // Fires immediately on every partialPath change
  listPathCompletions(sessionId, partialPath, 20)
    .then(...)
}, [sessionId, partialPath, enabled]);
```

**Impact:** High-frequency IPC calls during typing. Backend may be overwhelmed with completion requests.

**Recommended Fix:**
Add debounce (150-200ms is typical for autocomplete):

```typescript
useEffect(() => {
  if (!enabled) {
    setCompletions([]);
    setTotalCount(0);
    return;
  }

  let cancelled = false;
  setIsLoading(true);

  // Debounce the request
  const timeoutId = setTimeout(() => {
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
  }, 150);

  return () => {
    cancelled = true;
    clearTimeout(timeoutId);
  };
}, [sessionId, partialPath, enabled]);
```

---

### 7. CodebasesSettings Auto-Detects Memory Files Sequentially

**File:** `frontend/components/Settings/CodebasesSettings.tsx:44-60`

**Problem:**
Memory file detection happens sequentially for each codebase:

```typescript
const updatedList = await Promise.all(
  list.map(async (codebase) => {
    if (codebase.memory_file === undefined || codebase.memory_file === null) {
      try {
        const detected = await detectMemoryFiles(codebase.path);  // Each one waits
        if (detected) {
          await updateCodebaseMemoryFile(codebase.path, detected);  // Another wait
          // ...
        }
      }
    }
    return codebase;
  })
);
```

**Impact:** While Promise.all parallelizes the outer loop, each iteration still has a serial await chain.

**Recommended Fix:**
This is actually fine since each codebase's detection is independent and runs in parallel. However, the update could be batched:

```typescript
// After detection, batch all updates
const detectedUpdates: Array<[string, string]> = [];
const updatedList = await Promise.all(
  list.map(async (codebase) => {
    if (!codebase.memory_file) {
      const detected = await detectMemoryFiles(codebase.path);
      if (detected) {
        detectedUpdates.push([codebase.path, detected]);
        return { ...codebase, memory_file: detected };
      }
    }
    return codebase;
  })
);

// Batch updates (could be parallelized or made into single IPC call)
await Promise.all(
  detectedUpdates.map(([path, file]) => updateCodebaseMemoryFile(path, file))
);
```

---

### 8. SessionBrowser Loads All Sessions Without Pagination

**File:** `frontend/components/SessionBrowser/SessionBrowser.tsx:99`

**Problem:**
Loads up to 50 sessions at once without pagination:

```typescript
const result = await listAiSessions(50);
```

**Impact:** For users with many sessions, this could be slow. Also loads session details on every click without caching.

**Recommended Fix:**
Consider virtual scrolling with on-demand loading, or at minimum cache session details:

```typescript
// Simple detail cache
const sessionDetailCache = useRef(new Map<string, SessionSnapshot>());

const handleSelectSession = useCallback(async (session: SessionListingInfo) => {
  setSelectedSession(session);

  // Check cache first
  const cached = sessionDetailCache.current.get(session.identifier);
  if (cached) {
    setSessionDetail(cached);
    return;
  }

  setIsLoadingDetail(true);
  try {
    const detail = await loadAiSession(session.identifier);
    if (detail) {
      sessionDetailCache.current.set(session.identifier, detail);
    }
    setSessionDetail(detail);
  } finally {
    setIsLoadingDetail(false);
  }
}, []);
```

---

## Low Priority Issues

### 9. Stale Closure Risk in useAiEvents Timeout

**File:** `frontend/hooks/useAiEvents.ts:105-108`

**Problem:**
The flush timeout captures `pendingDeltas` in its closure. If the component unmounts and remounts quickly, the old timeout might try to access stale state.

```typescript
if (!flushTimeout) {
  flushTimeout = setTimeout(flushPendingDeltas, FLUSH_INTERVAL_MS - (now - lastFlushTime));
}
```

**Impact:** Low - the cleanup function does clear the timeout, but there's a potential race in strict mode double-mount.

**Recommended Fix:**
Already handled well by the `isMounted` flag and cleanup. No action needed.

---

### 10. Event Listener Cleanup Uses Async Unlisten

**File:** `frontend/hooks/useTauriEvents.ts:487-490`

**Problem:**
Cleanup iterates over promises and calls unlisten after they resolve:

```typescript
for (const p of unlisteners) {
  p.then((unlisten) => unlisten());
}
```

**Impact:** If the component unmounts before all listeners are set up, cleanup might not properly unlisten from those that complete after unmount.

**Recommended Fix:**
Track which listeners have been set up and only unlisten those:

```typescript
const unlistenFns: Array<() => void> = [];

// When setting up:
listen<CommandBlockEvent>("command_block", handler).then((unlisten) => {
  if (isMounted) {
    unlistenFns.push(unlisten);
  } else {
    unlisten(); // Already unmounted, cleanup immediately
  }
});

// In cleanup:
return () => {
  isMounted = false;
  for (const unlisten of unlistenFns) {
    unlisten();
  }
};
```

---

### 11. useFileIndex Triggers Background Indexing Without Tracking

**File:** `frontend/hooks/useFileIndex.ts:41-44`

**Problem:**
Background indexing is fire-and-forget:

```typescript
// Index the directory in background (don't await to avoid blocking UI)
indexDirectory(root).catch((err) => {
  console.warn("Background indexing failed:", err);
});
```

**Impact:** If the workspace changes rapidly, multiple indexing operations might run concurrently. The indexer.ts does have deduplication for this, so actual impact is low.

**Recommended Fix:**
Already handled by `indexingPromises` Map in `frontend/lib/indexer.ts`. No action needed.

---

### 12. GitPanel Refreshes on Dialog Open Without Cache

**File:** `frontend/components/GitPanel/GitPanel.tsx:671-675`

**Problem:**
Git status is refreshed every time the dialog opens, even if it was just refreshed:

```typescript
useEffect(() => {
  if (open) {
    void refreshStatus();
  }
}, [open, refreshStatus]);
```

**Impact:** Minor - git status should be fresh when viewing changes. However, rapid open/close would cause redundant calls.

**Recommended Fix:**
Add minimum interval between refreshes (similar to HomeView fix).

---

## Existing Good Patterns

The codebase already has several well-implemented patterns:

1. **Indexer Deduplication** (`frontend/lib/indexer.ts:17-21, 74-100`)
   - Uses module-level promise tracking to prevent thundering herd
   - `initIndexer` and `indexDirectory` both deduplicate concurrent calls

2. **AI Event Sequence Deduplication** (`frontend/hooks/useAiEvents.ts:7-26`)
   - Tracks last seen sequence number per session
   - Skips duplicate/out-of-order events

3. **Git Refresh Sequence Numbers** (`frontend/hooks/useTauriEvents.ts:157, 178-205`)
   - Prevents out-of-order git refreshes using incrementing sequence numbers

4. **Text Delta Batching** (`frontend/hooks/useAiEvents.ts:69-108`)
   - Batches streaming text deltas at ~60fps to avoid excessive renders

5. **Proper Cleanup with isMounted Flags** (`frontend/hooks/useSidecarEvents.ts:39-41`)
   - Uses `isMounted` flag to prevent state updates after unmount

---

## Implementation Priority

| Priority | Issue | Estimated Impact | Effort |
|----------|-------|------------------|--------|
| High | Settings caching | Significant IPC reduction | Low |
| High | Git status polling deduplication | Prevent backend overload | Low |
| High | Tab creation waterfall | Faster UX | Low |
| High | HomeView focus debounce | Prevent rapid fetches | Low |
| Medium | Slash commands caching | Moderate IPC reduction | Medium |
| Medium | Path completion debounce | Reduce typing lag | Low |
| Medium | SessionBrowser detail caching | Better UX on repeat views | Low |
| Low | Event listener cleanup | Edge case fix | Medium |

---

## Conclusion

The most impactful improvements would be:
1. Adding a settings cache with invalidation
2. Debouncing/deduplicating git status polling
3. Parallelizing tab creation requests
4. Debouncing HomeView focus refreshes

These changes would reduce IPC overhead without significant architectural changes.
