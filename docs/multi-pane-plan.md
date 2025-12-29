# Multi-Pane Support Plan

This document outlines the implementation plan for adding multi-pane (split view) support to Qbit terminal.

## Overview

Multi-pane support allows users to split the terminal view into multiple panes within a single tab, each running its own terminal session. This is similar to functionality in iTerm2, tmux, and VS Code's terminal.

### Design Principle: Pane as Primary Unit

**Each pane is a fully independent workspace.** All features that were previously attributed to tabs or windows are now per-pane:

- **Session**: Each pane has its own terminal/agent session
- **Render mode**: Each pane independently switches between timeline/fullterm
- **Agent mode**: Each pane can run its own AI agent
- **Working directory**: Each pane tracks its own cwd
- **Command history**: Each pane maintains its own history

Tabs become purely a layout organization mechanism, grouping related panes together.

### Key Shortcuts
- **Cmd+D** - Split pane vertically (new pane to the right)
- **Cmd+Shift+D** - Split pane horizontally (new pane below)
- **Cmd+W** - Close current pane (closes tab if last pane)
- **Cmd+Option+Arrow** - Navigate between panes

## Current Architecture

### What Exists
- **Sessions**: Flat `Record<string, Session>` in Zustand store
- **Single active session**: `activeSessionId` pointer
- **PTY management**: One PTY per session with resize support
- **Resizable components**: `react-resizable-panels` already in dependencies (unused)
- **Terminal component**: Supports multiple instances with session filtering

### What's Missing
- Pane layout model (spatial hierarchy)
- Per-tab pane structure
- Pane focus management
- Keyboard navigation between panes
- Layout persistence

## Data Model

### New Types

```typescript
// Unique identifier for panes
type PaneId = string;

// Direction of split
type SplitDirection = "horizontal" | "vertical";

// A pane can be either a leaf (terminal) or a container (split)
type PaneNode =
  | { type: "leaf"; id: PaneId; sessionId: string }
  | { type: "split"; id: PaneId; direction: SplitDirection; children: [PaneNode, PaneNode]; ratio: number };

// Layout for a single tab
interface TabLayout {
  root: PaneNode;
  focusedPaneId: PaneId;
}

// Extended session to include pane info
interface Session {
  // ... existing fields
  paneId?: PaneId;  // Which pane this session is displayed in (if any)
}
```

### Store Additions

```typescript
interface QbitState {
  // Existing
  sessions: Record<string, Session>;
  activeSessionId: string | null;

  // New: pane layouts per tab (tab = root session that owns the layout)
  tabLayouts: Record<string, TabLayout>;  // tabId -> layout

  // Actions
  splitPane: (tabId: string, paneId: PaneId, direction: SplitDirection) => void;
  closePane: (tabId: string, paneId: PaneId) => void;
  focusPane: (tabId: string, paneId: PaneId) => void;
  resizePane: (tabId: string, paneId: PaneId, ratio: number) => void;
  navigatePane: (tabId: string, direction: "up" | "down" | "left" | "right") => void;
}
```

## Implementation Phases

### Phase 1: Foundation (Core Data Model)

**Goal**: Establish pane layout data structures and basic state management.

#### Tasks

1. **Add pane types to store**
   - File: `frontend/store/index.ts`
   - Add `PaneId`, `SplitDirection`, `PaneNode`, `TabLayout` types
   - Add `tabLayouts` state field
   - Initialize with single-pane layout when creating new tabs

2. **Create pane utility functions**
   - File: `frontend/lib/pane-utils.ts` (new)
   - `createLeafPane(sessionId)` - Create leaf pane node
   - `splitPaneNode(node, paneId, direction, newSessionId)` - Split a pane
   - `removePaneNode(node, paneId)` - Remove pane, collapse parent
   - `findPaneById(node, paneId)` - Find pane in tree
   - `findPaneBySessionId(node, sessionId)` - Find pane by session
   - `getPaneNeighbor(node, paneId, direction)` - Get adjacent pane
   - `updatePaneRatio(node, paneId, ratio)` - Update split ratio
   - `getAllLeafPanes(node)` - Get all leaf panes

3. **Add store actions**
   - `splitPane(tabId, paneId, direction)` - Split pane, create new session
   - `closePane(tabId, paneId)` - Close pane, destroy session if orphaned
   - `focusPane(tabId, paneId)` - Update focused pane
   - `resizePane(tabId, paneId, ratio)` - Update split ratio
   - `navigatePane(tabId, direction)` - Move focus to adjacent pane

4. **Migrate existing sessions**
   - When tab is created, initialize with single-pane layout
   - Existing sessions auto-wrapped in leaf pane

### Phase 2: Rendering (Pane Components)

**Goal**: Render pane layouts with proper terminal instances.

#### Tasks

1. **Create PaneContainer component**
   - File: `frontend/components/PaneContainer/PaneContainer.tsx` (new)
   - Recursive component that renders `PaneNode` tree
   - Uses `ResizablePanelGroup` for splits
   - Uses `ResizablePanel` for each child
   - Uses `ResizableHandle` for drag resize

```typescript
interface PaneContainerProps {
  node: PaneNode;
  tabId: string;
  onResize: (paneId: PaneId, ratio: number) => void;
}

function PaneContainer({ node, tabId, onResize }: PaneContainerProps) {
  if (node.type === "leaf") {
    return <PaneLeaf paneId={node.id} sessionId={node.sessionId} tabId={tabId} />;
  }

  const direction = node.direction === "horizontal" ? "vertical" : "horizontal";

  return (
    <ResizablePanelGroup direction={direction}>
      <ResizablePanel defaultSize={node.ratio * 100}>
        <PaneContainer node={node.children[0]} tabId={tabId} onResize={onResize} />
      </ResizablePanel>
      <ResizableHandle />
      <ResizablePanel defaultSize={(1 - node.ratio) * 100}>
        <PaneContainer node={node.children[1]} tabId={tabId} onResize={onResize} />
      </ResizablePanel>
    </ResizablePanelGroup>
  );
}
```

2. **Create PaneLeaf component**
   - File: `frontend/components/PaneContainer/PaneLeaf.tsx` (new)
   - Renders single terminal/timeline for a session
   - Shows focus indicator (border highlight)
   - Handles click-to-focus
   - Broadcasts resize events to PTY

```typescript
interface PaneLeafProps {
  paneId: PaneId;
  sessionId: string;
  tabId: string;
}

function PaneLeaf({ paneId, sessionId, tabId }: PaneLeafProps) {
  const isFocused = useStore(s => s.tabLayouts[tabId]?.focusedPaneId === paneId);
  const session = useStore(s => s.sessions[sessionId]);
  const focusPane = useStore(s => s.focusPane);

  return (
    <div
      className={cn("h-full w-full flex", isFocused && "ring-1 ring-accent")}
      onClick={() => focusPane(tabId, paneId)}
    >
      <div className="flex-1">
        {session.renderMode === "fullterm" ? (
          <Terminal sessionId={sessionId} />
        ) : (
          <UnifiedTimeline sessionId={sessionId} />
        )}
      </div>
    </div>
  );
}
```

3. **Integrate into main layout**
   - File: `frontend/App.tsx`
   - Replace single terminal/timeline render with `PaneContainer`
   - Pass current tab's layout to container

4. **Handle pane resize → PTY resize**
   - Use `ResizeObserver` on each `PaneLeaf`
   - Debounce resize events (100ms)
   - Call `ptyResize(sessionId, rows, cols)` on resize

### Phase 3: Keyboard Shortcuts

**Goal**: Implement pane management shortcuts.

#### Tasks

1. **Add split shortcuts**
   - File: `frontend/App.tsx` (keyboard handler)
   - **Cmd+D**: Split vertically (pane to right)
   - **Cmd+Shift+D**: Split horizontally (pane below)

```typescript
// In keyboard event handler
if (e.metaKey && e.key === "d") {
  e.preventDefault();
  if (e.shiftKey) {
    // Horizontal split (below)
    splitPane(activeTabId, focusedPaneId, "horizontal");
  } else {
    // Vertical split (right)
    splitPane(activeTabId, focusedPaneId, "vertical");
  }
}
```

2. **Add navigation shortcuts**
   - **Cmd+Option+←** - Focus pane to left
   - **Cmd+Option+→** - Focus pane to right
   - **Cmd+Option+↑** - Focus pane above
   - **Cmd+Option+↓** - Focus pane below

```typescript
if (e.metaKey && e.altKey && ["ArrowLeft", "ArrowRight", "ArrowUp", "ArrowDown"].includes(e.key)) {
  e.preventDefault();
  const direction = {
    ArrowLeft: "left",
    ArrowRight: "right",
    ArrowUp: "up",
    ArrowDown: "down",
  }[e.key];
  navigatePane(activeTabId, direction);
}
```

3. **Update close pane behavior**
   - **Cmd+W**: Close focused pane
   - If last pane in tab, close the tab
   - Focus moves to sibling pane after close

4. **Add to Command Palette**
   - "Split Pane Right" (Cmd+D)
   - "Split Pane Down" (Cmd+Shift+D)
   - "Close Pane" (Cmd+W)
   - "Focus Pane Left/Right/Up/Down"

### Phase 4: Focus Management

**Goal**: Proper focus tracking and visual indicators.

#### Tasks

1. **Track focused pane per tab**
   - Each tab layout has `focusedPaneId`
   - Switching tabs restores focus to that tab's focused pane
   - `activeSessionId` derived from focused pane's session

2. **Visual focus indicator**
   - Active pane has a thin, highlighted border (e.g., 1-2px accent color) to clearly indicate it's the currently focused pane
   - Unfocused panes have no border or a very subtle muted border
   - Border should be visually distinct but not distracting
   - Optional: dim unfocused panes slightly

3. **Focus on click**
   - Clicking anywhere in pane focuses it
   - Also focuses the terminal/input within

4. **Auto-focus on split**
   - When splitting, focus moves to new pane
   - New pane gets new session (or option to clone?)

5. **Focus on close**
   - When closing pane, focus sibling
   - Prefer left/up sibling, then right/down

### Phase 5: Input Routing

**Goal**: Ensure keyboard input routes to correct pane.

#### Tasks

1. **Route terminal input to focused pane**
   - `UnifiedInput` targets focused pane's session
   - Terminal mode writes to focused session's PTY
   - Agent mode sends to focused session's agent

2. **Handle focus during typing**
   - Input field click focuses containing pane
   - Typing in terminal focuses that pane

3. **Global vs pane-local shortcuts**
   - Pane navigation: global (Cmd+Option+Arrow)
   - Tab switching: global (Ctrl+[ / Ctrl+])
   - Terminal input: routed to focused pane
   - Agent commands: routed to focused pane

### Phase 6: Persistence & Polish

**Goal**: Save layouts and add visual polish.

#### Tasks

1. **Persist pane layouts**
   - Save layouts to session storage or settings
   - Restore on app restart

2. **Maximize pane (optional)**
   - **Cmd+Shift+Enter**: Temporarily maximize pane
   - Press again to restore layout
   - Or click outside to restore

3. **Pane zoom (optional)**
   - Double-click resize handle to equalize sizes
   - Or: double-click to maximize one side

4. **Drag to reorder (optional, future)**
   - Drag pane header to rearrange
   - Or drag to different tab

5. **Pane title bar (optional)**
   - Show session name/directory in pane header
   - Show process name if running
   - Close button on pane header

## File Changes Summary

### New Files
| File | Purpose |
|------|---------|
| `frontend/lib/pane-utils.ts` | Pane tree manipulation utilities |
| `frontend/components/PaneContainer/index.ts` | Barrel export |
| `frontend/components/PaneContainer/PaneContainer.tsx` | Recursive pane layout renderer |
| `frontend/components/PaneContainer/PaneLeaf.tsx` | Single pane terminal/timeline |

### Modified Files
| File | Changes |
|------|---------|
| `frontend/store/index.ts` | Add pane types, `tabLayouts`, pane actions |
| `frontend/App.tsx` | Replace single view with `PaneContainer`, add shortcuts |
| `frontend/components/UnifiedInput/UnifiedInput.tsx` | Route input to focused pane's session |
| `frontend/components/Terminal/Terminal.tsx` | Handle pane-level resize |
| `frontend/components/StatusBar/StatusBar.tsx` | Show focused pane info |

## Edge Cases

### Multi-Pane Considerations

1. **Fullterm mode per pane**
   - Each pane can independently be in fullterm mode
   - One pane running vim, another showing timeline

2. **Agent mode per pane**
   - Each pane's session can be in terminal or agent mode
   - Agent in one pane, terminal in another

3. **Resize behavior**
   - PTY resize must be called when pane resizes
   - Debounce to avoid excessive calls
   - Terminal must re-fit on resize

4. **Memory considerations**
   - Each pane = one xterm.js instance
   - Consider lazy initialization for hidden panes

5. **Tab close with multiple panes**
   - Close all sessions in the tab's layout
   - Clean up all PTY instances

## Testing Strategy

### Unit Tests
- Pane utility functions (split, remove, find, navigate)
- Store actions (splitPane, closePane, focusPane)
- Layout tree manipulation

### Integration Tests
- Keyboard shortcuts trigger correct actions
- Focus moves correctly on navigation
- PTY resize called on pane resize
- Session cleanup on pane close

### E2E Tests
- Split pane with Cmd+D
- Navigate between panes
- Close pane and verify focus
- Type in correct pane after focus switch
- Persist and restore layouts

## Open Questions

1. **Clone vs new session on split?**
   - Option A: New session starts fresh shell
   - Option B: Clone current directory from source pane
   - Recommendation: Option B (better UX)

2. **Maximum pane depth?**
   - Unlimited splits could get unwieldy
   - Consider limit of 4-6 panes per tab?
   - Or let users decide

3. **Pane headers?**
   - Show mini title bar per pane?
   - Or rely on focus indicator only?
   - Headers add visual noise but improve UX

4. **Drag and drop reorder?**
   - Complex to implement properly
   - Defer to future phase?

5. **Sync scroll between panes?**
   - Useful for comparing files
   - Probably out of scope for v1

## Timeline Estimate

| Phase | Complexity | Dependencies |
|-------|------------|--------------|
| Phase 1: Foundation | Medium | None |
| Phase 2: Rendering | High | Phase 1 |
| Phase 3: Shortcuts | Low | Phase 1, 2 |
| Phase 4: Focus | Medium | Phase 2 |
| Phase 5: Input | Medium | Phase 4 |
| Phase 6: Persistence | Low | Phase 1-5 |

Recommended order: Phase 1 → 2 → 4 → 5 → 3 → 6

## Success Criteria

- [ ] Cmd+D splits pane vertically (new pane right)
- [ ] Cmd+Shift+D splits pane horizontally (new pane below)
- [ ] Cmd+Option+Arrow navigates between panes
- [ ] Cmd+W closes focused pane
- [ ] Active pane has thin highlighted border to indicate focus
- [ ] Typing routes to correct session
- [ ] PTY resizes correctly when pane resizes
- [ ] Layout persists across app restarts
- [ ] No memory leaks when closing panes
- [ ] Multiple agents can run in parallel (one per pane)
