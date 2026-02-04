# Terminal/Canvas Rendering Performance Review

## Summary

This review examines the terminal rendering architecture in Qbit's frontend, focusing on xterm.js performance, canvas optimization, and the React portal system used for terminal persistence. The codebase demonstrates several sophisticated patterns for terminal management, but there are opportunities for optimization.

**Overall Assessment**: The architecture is well-designed with good separation of concerns. The portal-based persistence system and instance manager pattern are solid foundations. However, there are several performance issues related to addon loading, resize handling, theme application, and multiple terminal instance management.

---

## Issues Found

### HIGH Priority

#### 1. WebGL Addon Loading Without Error Recovery Strategy
**File**: `/Users/xlyk/Code/qbit.worktrees/frontend-improvements/frontend/components/Terminal/Terminal.tsx`
**Lines**: 173-179

**Issue**: The WebGL addon is loaded after the terminal opens, and if it fails, the fallback to canvas renderer happens silently. However, the `WebglAddon` is created without any configuration options that could improve compatibility or performance.

```typescript
// Current code
try {
  const webglAddon = new WebglAddon();
  terminal.loadAddon(webglAddon);
} catch (e) {
  logger.warn("WebGL not available, falling back to canvas", e);
}
```

**Problem**:
- No attempt to check WebGL availability before instantiation
- No tracking of which terminals are using WebGL vs canvas (useful for debugging)
- The addon is loaded AFTER `terminal.open()`, which means the initial render uses canvas and then switches to WebGL, causing a potential visual flicker

**Recommended Fix**:
```typescript
// Check WebGL support before creating addon
const canvas = document.createElement('canvas');
const gl = canvas.getContext('webgl2') || canvas.getContext('webgl');

if (gl) {
  try {
    const webglAddon = new WebglAddon();
    // Handle context loss gracefully
    webglAddon.onContextLoss(() => {
      logger.warn("[Terminal] WebGL context lost, attempting recovery");
      webglAddon.dispose();
      // Could retry or fallback to canvas here
    });
    terminal.loadAddon(webglAddon);
    logger.debug("[Terminal] WebGL renderer active");
  } catch (e) {
    logger.warn("WebGL addon failed, using canvas renderer", e);
  }
} else {
  logger.debug("[Terminal] WebGL not available, using canvas renderer");
}
```

---

#### 2. Resize Handling Over-Debouncing and Multiple RAF Calls
**File**: `/Users/xlyk/Code/qbit.worktrees/frontend-improvements/frontend/components/Terminal/Terminal.tsx`
**Lines**: 294-314

**Issue**: The resize handling uses a combination of double `requestAnimationFrame` AND a 50ms `setTimeout`, creating excessive delay for resize events.

```typescript
// Current code
const resizeObserver = new ResizeObserver(() => {
  if (resizeRafRef.current !== null) {
    cancelAnimationFrame(resizeRafRef.current);
  }
  if (resizeTimeoutRef !== null) {
    clearTimeout(resizeTimeoutRef);
  }
  resizeRafRef.current = requestAnimationFrame(() => {
    resizeRafRef.current = requestAnimationFrame(() => {
      resizeRafRef.current = null;
      resizeTimeoutRef = setTimeout(() => {
        resizeTimeoutRef = null;
        if (!aborted) {
          handleResize();
        }
      }, 50);
    });
  });
});
```

**Problem**:
- Double RAF + 50ms timeout = ~83ms minimum delay (at 60fps, 2 frames = 33ms + 50ms)
- This creates noticeably laggy resize behavior during pane splits
- The double RAF was intended to wait for layout, but `ResizeObserver` already fires after layout

**Recommended Fix**:
```typescript
// Simplified debounce - single RAF with immediate timeout clear
const resizeObserver = new ResizeObserver(() => {
  if (resizeRafRef.current !== null) {
    cancelAnimationFrame(resizeRafRef.current);
  }
  resizeRafRef.current = requestAnimationFrame(() => {
    resizeRafRef.current = null;
    if (!aborted) {
      handleResize();
    }
  });
});
```

If debouncing is truly needed for rapid pane restructuring, use a shorter timeout:
```typescript
const RESIZE_DEBOUNCE_MS = 16; // One frame at 60fps

const resizeObserver = new ResizeObserver(() => {
  if (resizeTimeoutRef !== null) {
    clearTimeout(resizeTimeoutRef);
  }
  resizeTimeoutRef = setTimeout(() => {
    resizeTimeoutRef = null;
    requestAnimationFrame(() => {
      if (!aborted) {
        handleResize();
      }
    });
  }, RESIZE_DEBOUNCE_MS);
});
```

---

#### 3. Theme Changes Trigger Full Terminal Re-render
**File**: `/Users/xlyk/Code/qbit.worktrees/frontend-improvements/frontend/lib/theme/ThemeManager.ts`
**Lines**: 160-203

**Issue**: `applyToTerminal()` sets multiple individual options on the terminal, each of which can trigger internal xterm.js re-renders.

```typescript
// Current code
applyToTerminal(term: XTerm) {
  if (!this.currentTheme) return;
  // ... build xtermTheme object ...

  // Each of these can trigger a re-render
  term.options.theme = xtermTheme;
  if (t.typography?.terminal?.fontFamily) {
    term.options.fontFamily = t.typography.terminal.fontFamily;
  }
  if (t.typography?.terminal?.fontSize) {
    term.options.fontSize = t.typography.terminal.fontSize;
  }
  // ... more options ...
}
```

**Problem**: Setting options individually (theme, fontFamily, fontSize, cursorBlink, cursorStyle) can cause up to 5 separate re-renders.

**Recommended Fix**:
```typescript
applyToTerminal(term: XTerm) {
  if (!this.currentTheme) return;
  const t = this.currentTheme;
  const ansi = t.colors.ansi;
  const hasBgImage = !!t.background?.image;

  // Batch all options into a single object update
  const options: Partial<ITerminalOptions> = {
    theme: {
      background: hasBgImage ? "rgba(0,0,0,0)" : t.colors.ui.background,
      foreground: ansi.defaultFg ?? t.colors.ui.foreground,
      cursor: ansi.defaultFg ?? t.colors.ui.foreground,
      cursorAccent: t.colors.ui.background,
      selectionBackground: t.terminal?.selectionBackground ?? ansi.blue,
      black: ansi.black,
      // ... all ANSI colors ...
    },
  };

  // Add optional properties only if defined
  if (t.typography?.terminal?.fontFamily) {
    options.fontFamily = t.typography.terminal.fontFamily;
  }
  if (t.typography?.terminal?.fontSize) {
    options.fontSize = t.typography.terminal.fontSize;
  }
  if (t.terminal?.cursorBlink !== undefined) {
    options.cursorBlink = t.terminal.cursorBlink;
  }
  if (t.terminal?.cursorStyle) {
    options.cursorStyle = t.terminal.cursorStyle;
  }

  // Single options assignment triggers one re-render
  Object.assign(term.options, options);
}
```

---

### MEDIUM Priority

#### 4. StaticTerminalOutput Creates New Terminal Instance Per Command Block
**File**: `/Users/xlyk/Code/qbit.worktrees/frontend-improvements/frontend/components/CommandBlock/StaticTerminalOutput.tsx`
**Lines**: 79-124

**Issue**: Each `StaticTerminalOutput` component creates its own xterm.js Terminal instance. In a timeline with many command blocks, this creates many DOM elements and canvas contexts.

```typescript
// Current code - creates new terminal on every mount
useEffect(() => {
  if (!containerRef.current) return;
  if (!terminalRef.current) {
    const terminal = new Terminal({
      // ... options ...
    });
    terminal.open(containerRef.current);
    terminalRef.current = terminal;
  }
  return () => {
    if (terminalRef.current) {
      terminalRef.current.dispose();
      terminalRef.current = null;
    }
  };
}, []);
```

**Problem**:
- Each terminal creates its own canvas element
- Browser has limits on WebGL contexts (typically 8-16)
- Memory overhead accumulates with many command blocks
- No virtualization - all terminals render even when off-screen

**Recommended Fix**:
Consider a pooling strategy or use a simpler ANSI renderer for static output:

```typescript
// Option 1: Pool of static terminals
class StaticTerminalPool {
  private pool: Terminal[] = [];
  private inUse = new Map<string, Terminal>();
  private maxSize = 10;

  acquire(id: string): Terminal {
    if (this.inUse.has(id)) return this.inUse.get(id)!;

    let terminal = this.pool.pop();
    if (!terminal) {
      terminal = new Terminal({ /* static options */ });
    }
    this.inUse.set(id, terminal);
    return terminal;
  }

  release(id: string): void {
    const terminal = this.inUse.get(id);
    if (terminal) {
      this.inUse.delete(id);
      terminal.clear();
      if (this.pool.length < this.maxSize) {
        this.pool.push(terminal);
      } else {
        terminal.dispose();
      }
    }
  }
}

// Option 2: Use react-ansi or ansi-to-react for static output
// and only use xterm.js for live/interactive content
```

---

#### 5. LiveTerminalManager Creates Terminal Without Container
**File**: `/Users/xlyk/Code/qbit.worktrees/frontend-improvements/frontend/lib/terminal/LiveTerminalManager.ts`
**Lines**: 39-110

**Issue**: `getOrCreate()` creates a Terminal instance but doesn't open it (no container). The terminal is only opened later when `attachToContainer()` is called. This means addons are loaded to a non-rendered terminal.

```typescript
getOrCreate(sessionId: string): Terminal {
  // ... creates terminal with options ...
  const fitAddon = new FitAddon();
  const serializeAddon = new SerializeAddon();
  terminal.loadAddon(fitAddon);
  terminal.loadAddon(serializeAddon);
  // ... but terminal.open() is NOT called here ...
  this.instances.set(sessionId, instance);
  return terminal;
}
```

**Problem**:
- `FitAddon.fit()` will fail if called before `terminal.open()`
- Theme is applied before terminal has a DOM element
- Pending writes buffer but can't be displayed

**Recommended Fix**:
The current buffering approach in `pendingWrites` is correct, but consider documenting this more clearly and ensuring `fit()` is never called before open:

```typescript
attachToContainer(sessionId: string, container: HTMLElement): boolean {
  // ... existing code ...

  if (!terminal.element) {
    // First time opening
    terminal.open(container);
    instance.isOpened = true;

    // Now safe to fit - terminal has dimensions
    try {
      fitAddon.fit();
    } catch (e) {
      logger.debug("[LiveTerminalManager] Initial fit failed:", e);
    }

    // Flush pending writes AFTER fit to ensure proper dimensions
    if (instance.pendingWrites.length > 0) {
      for (const data of instance.pendingWrites) {
        terminal.write(data);
      }
      instance.pendingWrites = [];
    }
  }
  // ...
}
```

---

#### 6. Terminal Portal Target Registration Causes Re-renders
**File**: `/Users/xlyk/Code/qbit.worktrees/frontend-improvements/frontend/hooks/useTerminalPortal.tsx`
**Lines**: 53-60

**Issue**: Every time a portal target is registered or unregistered, a new `Map` is created and all listeners are notified.

```typescript
const notifyListeners = useCallback(() => {
  // Create a new immutable snapshot when targets change
  snapshotRef.current = new Map(targetsRef.current);
  for (const listener of listenersRef.current) {
    listener();
  }
}, []);
```

**Problem**: Creating a new `Map` on every registration triggers `useSyncExternalStore` to re-render `TerminalLayer`, which then re-checks all portal entries.

**Recommended Fix**:
Use a version counter instead of full Map copy for simple changes:

```typescript
const versionRef = useRef(0);

const notifyListeners = useCallback(() => {
  versionRef.current++;
  // Only create new Map snapshot when needed for consumers
  snapshotRef.current = new Map(targetsRef.current);
  for (const listener of listenersRef.current) {
    listener();
  }
}, []);

// In TerminalLayer, compare by session IDs instead of full Map
export function TerminalLayer() {
  const targets = useTerminalPortalTargets();
  const prevSessionIds = useRef<Set<string>>(new Set());

  // Only re-render portals if session list actually changed
  const sessionIds = new Set(targets.keys());
  const sessionIdsChanged = !setsEqual(sessionIds, prevSessionIds.current);

  useEffect(() => {
    prevSessionIds.current = sessionIds;
  });

  // ... render logic ...
}
```

---

#### 7. VirtualTerminal Creates pendingWrites Promise Array
**File**: `/Users/xlyk/Code/qbit.worktrees/frontend-improvements/frontend/lib/terminal/VirtualTerminal.ts`
**Lines**: 42-48

**Issue**: Every `write()` call creates a new Promise and pushes it to an array that grows unbounded until `flush()` is called.

```typescript
write(data: string): void {
  const promise = new Promise<void>((resolve) => {
    this.terminal.write(data, resolve);
  });
  this.pendingWrites.push(promise);  // Array grows with each write
  this.contentDirty = true;
}
```

**Problem**: For commands with lots of output (like `npm install`), this can create thousands of Promises.

**Recommended Fix**:
Use a single Promise for batched writes:

```typescript
private pendingWriteCount = 0;
private flushPromise: Promise<void> | null = null;
private flushResolve: (() => void) | null = null;

write(data: string): void {
  this.pendingWriteCount++;
  this.contentDirty = true;

  if (!this.flushPromise) {
    this.flushPromise = new Promise((resolve) => {
      this.flushResolve = resolve;
    });
  }

  this.terminal.write(data, () => {
    this.pendingWriteCount--;
    if (this.pendingWriteCount === 0 && this.flushResolve) {
      this.flushResolve();
      this.flushPromise = null;
      this.flushResolve = null;
    }
  });
}

async flush(): Promise<void> {
  if (this.flushPromise) {
    await this.flushPromise;
  }
}
```

---

### LOW Priority

#### 8. SyncOutputBuffer Timeout Not Configurable
**File**: `/Users/xlyk/Code/qbit.worktrees/frontend-improvements/frontend/lib/terminal/SyncOutputBuffer.ts`
**Lines**: 20-21

**Issue**: The sync timeout is hardcoded to 1000ms.

```typescript
private static readonly SYNC_TIMEOUT_MS = 1000;
```

**Recommendation**: Consider making this configurable via settings for users with slower connections or specific needs.

---

#### 9. TerminalInstanceManager Parking Lot Fixed Positioning
**File**: `/Users/xlyk/Code/qbit.worktrees/frontend-improvements/frontend/lib/terminal/TerminalInstanceManager.ts`
**Lines**: 31-51

**Issue**: The parking lot uses `position: fixed` with extreme offsets.

```typescript
el.style.position = "fixed";
el.style.left = "-10000px";
el.style.top = "-10000px";
```

**Recommendation**: Use `visibility: hidden` instead for better semantics:

```typescript
el.style.position = "absolute";
el.style.width = "1px";
el.style.height = "1px";
el.style.overflow = "hidden";
el.style.visibility = "hidden";
el.setAttribute("aria-hidden", "true");
```

---

#### 10. Missing Terminal Disposal on Tab Close
**File**: `/Users/xlyk/Code/qbit.worktrees/frontend-improvements/frontend/store/index.ts`
**Lines**: 1851

**Issue**: The comment mentions terminal cleanup happens outside Immer, but the actual disposal relies on `TerminalInstanceManager.dispose()` being called elsewhere.

```typescript
// Clean up outside Immer (terminal instances and AI event sequence tracking)
```

**Recommendation**: Ensure `TerminalInstanceManager.dispose(sessionId)` is always called when:
1. A tab is closed
2. A pane is closed (if it's the last reference to that session)
3. The app unmounts

The current code in `App.tsx` handles pane closes but tab close may miss cleanup in some edge cases.

---

## Performance Recommendations Summary

| Priority | Issue | Impact | Effort |
|----------|-------|--------|--------|
| HIGH | WebGL addon loading | Medium | Low |
| HIGH | Resize over-debouncing | High | Low |
| HIGH | Theme changes multiple re-renders | Medium | Medium |
| MEDIUM | Static terminal per command block | High | High |
| MEDIUM | LiveTerminal pre-open state | Low | Low |
| MEDIUM | Portal registration re-renders | Medium | Medium |
| MEDIUM | VirtualTerminal Promise accumulation | Low | Medium |
| LOW | Sync timeout not configurable | Low | Low |
| LOW | Parking lot positioning | Low | Low |
| LOW | Terminal disposal edge cases | Low | Medium |

---

## Architecture Strengths

1. **Portal-based Persistence**: The `TerminalPortalProvider` + `TerminalLayer` pattern elegantly solves the React remount problem during pane splits.

2. **Instance Manager Pattern**: `TerminalInstanceManager` provides good lifecycle control and the parking lot concept prevents xterm.js renderer crashes.

3. **Synchronized Output Buffer**: `SyncOutputBuffer` correctly implements DEC 2026 for flicker-free rendering with a safety timeout.

4. **Separation of Concerns**: Clear separation between:
   - `Terminal.tsx` (React component)
   - `TerminalInstanceManager` (lifecycle)
   - `SyncOutputBuffer` (output synchronization)
   - `VirtualTerminal` (headless processing)
   - `LiveTerminalManager` (live command output)

5. **Theme Integration**: Clean theme application through `ThemeManager` with proper listener cleanup.
