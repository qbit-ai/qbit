# Memory Leak Review - Qbit Frontend

## Summary

This review identifies memory leak risks in the Qbit frontend codebase, focusing on long-running session scenarios where small leaks can accumulate into significant memory pressure.

**Overall Assessment**: The codebase demonstrates good awareness of cleanup patterns with most critical paths properly handled. However, several medium-priority issues were identified that could cause memory accumulation over extended use.

---

## Critical Issues

### 1. Module-Level Map Without Cleanup - `notificationToTabMap` (HIGH)

**File**: `/Users/xlyk/Code/qbit.worktrees/frontend-improvements/frontend/lib/systemNotifications.ts`
**Lines**: 45-47

```typescript
/** In-memory map of notification identifiers to tab IDs for click routing */
const notificationToTabMap = new Map<number, string>();
```

**Problem**: This module-level Map stores notification-to-tab mappings indefinitely. While entries are cleaned up on successful click handling (line 147), if a notification is never clicked, the entry persists forever. With `notificationIdCounter` incrementing indefinitely, this can accumulate many stale entries.

**Impact**: Each entry is small (~20-50 bytes), but over thousands of notifications across long sessions, this could grow to several megabytes.

**Recommended Fix**:
```typescript
// Option 1: Use TTL-based cleanup
const NOTIFICATION_TTL_MS = 5 * 60 * 1000; // 5 minutes
const notificationToTabMap = new Map<number, { tabId: string; createdAt: number }>();

// Add periodic cleanup
setInterval(() => {
  const now = Date.now();
  for (const [id, entry] of notificationToTabMap) {
    if (now - entry.createdAt > NOTIFICATION_TTL_MS) {
      notificationToTabMap.delete(id);
    }
  }
}, 60_000); // Clean every minute

// Option 2: Limit map size
const MAX_PENDING_NOTIFICATIONS = 100;
function addNotificationMapping(id: number, tabId: string) {
  if (notificationToTabMap.size >= MAX_PENDING_NOTIFICATIONS) {
    const oldestKey = notificationToTabMap.keys().next().value;
    notificationToTabMap.delete(oldestKey);
  }
  notificationToTabMap.set(id, tabId);
}
```

---

### 2. Module-Level Sequence Tracking Map - `lastSeenSeq` (HIGH)

**File**: `/Users/xlyk/Code/qbit.worktrees/frontend-improvements/frontend/hooks/useAiEvents.ts`
**Lines**: 11

```typescript
const lastSeenSeq = new Map<string, number>();
```

**Problem**: While there is a `resetSessionSequence()` function (line 17-19), it relies on being called when sessions are removed. The store's `removeSession` and `closeTab`/`closePane` functions do call this via dynamic import, but the async nature means cleanup could be delayed or fail silently.

**Current mitigation**: The store does attempt cleanup at lines 731-733, 1665-1667, 1857-1860.

**Risk**: If the dynamic import fails or is slow, stale session IDs remain in the map indefinitely.

**Recommended Fix**: Make cleanup synchronous by exporting a sync function or use a WeakMap with session objects if possible:
```typescript
// Move cleanup call to be awaited properly
removeSession: async (sessionId) => {
  TerminalInstanceManager.dispose(sessionId);
  const { resetSessionSequence } = await import("@/hooks/useAiEvents");
  resetSessionSequence(sessionId); // Now sync and guaranteed
  // ... rest of cleanup
}
```

---

## Medium Issues

### 3. Missing Event Listener Cleanup - `listenForSettingsUpdates` (MEDIUM)

**File**: `/Users/xlyk/Code/qbit.worktrees/frontend-improvements/frontend/lib/systemNotifications.ts`
**Lines**: 255-268

```typescript
export function listenForSettingsUpdates(): void {
  window.addEventListener("settings-updated", (event: Event) => {
    // ... handler
  });
}
```

**Problem**: This function adds an event listener but never removes it. While it's called once at app startup (App.tsx:383), if the function were ever called multiple times (e.g., during hot reload in development), listeners would accumulate.

**Recommended Fix**:
```typescript
let settingsListenerAdded = false;

export function listenForSettingsUpdates(): void {
  if (settingsListenerAdded) return;
  settingsListenerAdded = true;

  window.addEventListener("settings-updated", handleSettingsUpdated);
}

function handleSettingsUpdated(event: Event) {
  const customEvent = event as CustomEvent<{ notifications?: { native_enabled?: boolean } }>;
  // ... handler logic
}
```

---

### 4. Terminal Parking Lot DOM Accumulation (MEDIUM)

**File**: `/Users/xlyk/Code/qbit.worktrees/frontend-improvements/frontend/lib/terminal/TerminalInstanceManager.ts`
**Lines**: 29-51, 148-159

```typescript
private getParkingLot(): HTMLElement {
  // Creates a hidden div to hold detached terminals
}

detach(sessionId: string): void {
  // ...
  if (instance.terminal.element) {
    this.getParkingLot().appendChild(instance.terminal.element);
  }
}
```

**Problem**: When terminals are detached (not disposed), their DOM elements are moved to a "parking lot" div. If terminals are repeatedly created/detached without disposal, the parking lot accumulates DOM nodes. While `dispose()` properly removes them, the pattern relies on callers always disposing when truly done.

**Current mitigation**: The store's `removeSession`, `closePane`, and `closeTab` properly call `TerminalInstanceManager.dispose()`.

**Risk**: If session cleanup code has bugs or is bypassed, terminals can leak.

**Recommended Fix**: Add a safeguard maximum:
```typescript
private readonly MAX_PARKED_TERMINALS = 10;

detach(sessionId: string): void {
  // ... existing code

  // Safeguard: if too many terminals are parked, warn
  const parked = this.parkingLotEl?.children.length ?? 0;
  if (parked > this.MAX_PARKED_TERMINALS) {
    logger.warn(`[TerminalInstanceManager] ${parked} parked terminals - possible leak`);
  }
}
```

---

### 5. Async Listener Setup Race Condition (MEDIUM)

**File**: `/Users/xlyk/Code/qbit.worktrees/frontend-improvements/frontend/components/Terminal/Terminal.tsx`
**Lines**: 234-287

```typescript
(async () => {
  const unlistenOutput = await listen<TerminalOutputEvent>("terminal_output", ...);
  const unlistenSync = await listen<{ session_id: string; enabled: boolean }>(
    "synchronized_output", ...
  );

  if (aborted) {
    unlistenSync();
    unlistenOutput();
    return;
  }
  cleanupFnsRef.current.push(unlistenSync);
  cleanupFnsRef.current.push(unlistenOutput);
})();
```

**Problem**: The async IIFE creates a race condition. If the component unmounts while the `listen()` promises are pending but before they resolve, the unlisten functions are properly called (good!). However, if the component unmounts AFTER the listeners are added to `cleanupFnsRef` but BEFORE the cleanup runs, there's a brief window where events could be processed on an unmounted component.

**Current mitigation**: The `aborted` flag helps, and xterm.js handles late writes gracefully.

**Risk**: Low - could cause console warnings or visual glitches but unlikely to cause memory leaks.

---

### 6. Tauri Event Listener Promises Not Awaited on Cleanup (MEDIUM)

**File**: `/Users/xlyk/Code/qbit.worktrees/frontend-improvements/frontend/hooks/useTauriEvents.ts`
**Lines**: 488-491

```typescript
// Unlisten from events
for (const p of unlisteners) {
  p.then((unlisten) => unlisten());
}
```

**Problem**: The cleanup function doesn't await the unlisten promises. This means:
1. If the component remounts quickly, old listeners may briefly overlap with new ones
2. If the app is closing, listeners may not be cleaned up before process exit

**Recommended Fix**:
```typescript
return () => {
  // ... existing timer cleanup

  // Unlisten from events (fire-and-forget is acceptable here,
  // but we should at least catch errors)
  Promise.all(unlisteners.map(p => p.then(unlisten => unlisten())))
    .catch(err => logger.debug("Error cleaning up Tauri listeners:", err));
};
```

---

### 7. VirtualTerminal Instance Retention (MEDIUM)

**File**: `/Users/xlyk/Code/qbit.worktrees/frontend-improvements/frontend/lib/terminal/VirtualTerminalManager.ts`
**Lines**: 9-11

```typescript
class VirtualTerminalManager {
  private terminals = new Map<string, VirtualTerminal>();
```

**Problem**: Similar to other managers, this Map relies on `dispose(sessionId)` being called. The `useTauriEvents` hook calls `virtualTerminalManager.dispose(session_id)` on `prompt_start` (line 228), which is correct. However, if a command is interrupted (e.g., Ctrl+C) before `prompt_start`, the VirtualTerminal may not be disposed.

**Current mitigation**: The next command will call `create()` which disposes any existing terminal for that session (line 17).

**Risk**: Low - only one VirtualTerminal per session can exist, so leaks are bounded.

---

## Low Priority Issues

### 8. LiveTerminalBlock Missing Cleanup (LOW)

**File**: `/Users/xlyk/Code/qbit.worktrees/frontend-improvements/frontend/components/LiveTerminalBlock/LiveTerminalBlock.tsx`
**Lines**: 15-28

```typescript
useEffect(() => {
  if (!containerRef.current) {
    return;
  }

  liveTerminalManager.getOrCreate(sessionId);
  liveTerminalManager.attachToContainer(sessionId, containerRef.current);

  // Cleanup: detach but don't dispose (might be reattaching)
  // Disposal happens in useTauriEvents when command completes
}, [sessionId]);
```

**Problem**: The effect has no cleanup function. Comment says disposal happens elsewhere, but if the component unmounts unexpectedly (e.g., navigation), the terminal remains attached to a now-removed container.

**Mitigated by**: LiveTerminalManager handles this gracefully - the terminal can be reattached later.

---

### 9. RequestAnimationFrame Potential Orphan (LOW)

**File**: `/Users/xlyk/Code/qbit.worktrees/frontend-improvements/frontend/lib/terminal/TerminalInstanceManager.ts`
**Lines**: 92-103

```typescript
private safeFit(fitAddon: FitAddon): void {
  requestAnimationFrame(() => {
    try {
      fitAddon.fit();
    } catch (error) {
      // ...
    }
  });
}
```

**Problem**: The RAF callback isn't tracked, so if the terminal is disposed before the callback fires, it could attempt to fit a disposed terminal.

**Risk**: Very low - caught by try/catch and xterm.js handles this gracefully.

---

### 10. Theme Manager Listener Array Growth (LOW)

**File**: `/Users/xlyk/Code/qbit.worktrees/frontend-improvements/frontend/lib/theme/ThemeManager.ts`
**Lines**: 35-39

```typescript
onChange(listener: (t: QbitTheme | null) => void) {
  this.listeners.push(listener);
  return () => {
    this.listeners = this.listeners.filter((l) => l !== listener);
  };
}
```

**Problem**: The filter creates a new array on every unsubscribe, which is fine but could cause brief memory spikes with many listeners.

**Current usage**: Only 2-3 components subscribe (ThemeProvider, Terminal components), so this is not a practical concern.

---

## Positive Patterns (Well-Handled)

1. **useTauriEvents**: Properly clears timers, intervals, and tracks abort state
2. **useAiEvents**: Uses `isMounted` flag and clears pending timeouts
3. **useSidecarEvents**: Uses `isMounted` flag for async safety
4. **Terminal component**: Comprehensive cleanup with abort flag, ResizeObserver disconnect, and listener disposal
5. **App.tsx**: All window/document event listeners are properly removed in cleanup
6. **Store session cleanup**: Properly cleans up all session-related state maps
7. **TerminalInstanceManager**: Has `disposeAll()` for app shutdown

---

## Recommendations Summary

| Priority | Issue | Location | Effort |
|----------|-------|----------|--------|
| HIGH | notificationToTabMap unbounded growth | systemNotifications.ts:45 | Low |
| HIGH | lastSeenSeq async cleanup risk | useAiEvents.ts:11 | Medium |
| MEDIUM | listenForSettingsUpdates no cleanup | systemNotifications.ts:255 | Low |
| MEDIUM | Terminal parking lot accumulation | TerminalInstanceManager.ts:29 | Low |
| MEDIUM | Async listener race condition | Terminal.tsx:234 | Medium |
| MEDIUM | Tauri listener cleanup not awaited | useTauriEvents.ts:488 | Low |
| MEDIUM | VirtualTerminal interrupt case | VirtualTerminalManager.ts:9 | Low |
| LOW | LiveTerminalBlock no cleanup return | LiveTerminalBlock.tsx:15 | Low |
| LOW | RAF orphan in safeFit | TerminalInstanceManager.ts:92 | Low |
| LOW | Theme listener array churn | ThemeManager.ts:35 | Low |

---

## Testing Recommendations

1. **Long Session Test**: Run the app for 8+ hours with active terminal usage and monitor memory via Chrome DevTools
2. **Tab Churn Test**: Create and close 100+ tabs rapidly, check for retained Terminal instances
3. **Notification Spam Test**: Trigger many notifications without clicking, verify map size doesn't grow unbounded
4. **Hot Reload Test**: In development, trigger multiple hot reloads and check for duplicate event listeners
5. **Interrupt Test**: Start commands and interrupt them (Ctrl+C) repeatedly, verify VirtualTerminal cleanup

---

*Review conducted on: 2026-02-03*
*Files analyzed: 25+ frontend source files*
