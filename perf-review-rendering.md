# React Rendering Performance Review

This document identifies rendering performance issues in the Qbit frontend codebase and provides recommendations for optimization.

## Executive Summary

The codebase demonstrates **good awareness of React performance patterns** with existing use of:
- React.memo on key components (TabItem, UnifiedTimeline, UnifiedBlock, ToolItem, etc.)
- Combined selectors to reduce Zustand subscriptions (useSessionState, useUnifiedInputState, useTabBarState)
- Stable empty reference patterns (EMPTY_TIMELINE, EMPTY_STREAMING_BLOCKS, etc.)
- useCallback for event handlers in performance-critical components

However, several issues remain that cause unnecessary re-renders, particularly during high-frequency streaming operations.

---

## High Priority Issues

### 1. Inline Object Creation in JSX Props

**File:** `/frontend/components/LiveTerminalBlock/LiveTerminalBlock.tsx:37-41`

```tsx
<code
  style={{
    fontSize: "12px",
    lineHeight: 1.4,
    fontFamily: "JetBrains Mono, Menlo, Monaco, Consolas, monospace",
  }}
>
```

**Problem:** Creates a new style object on every render, causing the code element to always re-render.

**Fix:** Extract to a constant outside the component:
```tsx
const codeStyle = {
  fontSize: "12px",
  lineHeight: 1.4,
  fontFamily: "JetBrains Mono, Menlo, Monaco, Consolas, monospace",
} as const;

// Then use in JSX
<code style={codeStyle}>
```

**Impact:** This component renders frequently during command execution.

---

### 2. Missing React.memo on Frequently Rendered List Items

**File:** `/frontend/components/HomeView/HomeView.tsx:148-244` (ProjectRow)
**File:** `/frontend/components/HomeView/HomeView.tsx:247-293` (RecentDirectoryRow)

```tsx
function ProjectRow({ project, isExpanded, onToggle, onOpenDirectory, onContextMenu, onWorktreeContextMenu }) {
  // ...
}

function RecentDirectoryRow({ directory, onOpen }) {
  // ...
}
```

**Problem:** These list item components are not memoized, causing all rows to re-render when any state in HomeView changes (e.g., expandedProjects Set updates).

**Fix:** Wrap with React.memo:
```tsx
const ProjectRow = memo(function ProjectRow({ ... }) {
  // ...
});

const RecentDirectoryRow = memo(function RecentDirectoryRow({ ... }) {
  // ...
});
```

**Impact:** Medium - HomeView is not a high-frequency render target, but memoization prevents unnecessary row re-renders when expanding/collapsing projects.

---

### 3. Inline Arrow Functions in List Rendering

**File:** `/frontend/components/HomeView/HomeView.tsx:614-625`

```tsx
projects.map((project) => (
  <ProjectRow
    key={project.path}
    project={project}
    isExpanded={expandedProjects.has(project.path)}
    onToggle={() => toggleProject(project.path)}
    onOpenDirectory={handleOpenDirectory}
    onContextMenu={(e) => handleProjectContextMenu(e, project)}
    onWorktreeContextMenu={(e, worktreePath, branchName) =>
      handleWorktreeContextMenu(e, project.path, worktreePath, branchName)
    }
  />
))
```

**Problem:** Creates new function references on every render for `onToggle`, `onContextMenu`, and `onWorktreeContextMenu`. This defeats memoization even if React.memo is applied.

**Fix:** Use stable callback patterns that accept the item identifier:
```tsx
// In parent component
const handleToggle = useCallback((projectPath: string) => {
  toggleProject(projectPath);
}, [toggleProject]);

// In ProjectRow - call with project.path
<button onClick={() => onToggle(project.path)}>
```

Or use a wrapper component pattern that memoizes per-item callbacks.

---

### 4. Unstable Style Objects in DiffView

**File:** `/frontend/components/DiffView/DiffView.tsx:80`

```tsx
<div className="overflow-auto bg-background" style={contentStyle}>
```

**Issue:** The `contentStyle` is correctly memoized with useMemo (line 63), but the component renders many diff lines. While the style is stable, consider if the entire DiffView content could benefit from virtualization for large diffs.

---

### 5. SlashCommandPopup List Items Missing Memoization

**File:** `/frontend/components/SlashCommandPopup/SlashCommandPopup.tsx:72-119`

```tsx
{commands.map((command, index) => (
  <div
    key={command.path}
    onClick={() => onSelect(command)}
    onKeyDown={(e) => {
      if (e.key === "Enter" || e.key === " ") {
        e.preventDefault();
        onSelect(command);
      }
    }}
    // ...
  >
```

**Problem:**
1. Each list item creates new inline functions for onClick and onKeyDown
2. List items are not extracted into a memoized component

**Fix:** Extract to a memoized component:
```tsx
const CommandItem = memo(function CommandItem({
  command,
  index,
  isSelected,
  onSelect
}: {
  command: SlashCommand;
  index: number;
  isSelected: boolean;
  onSelect: (command: SlashCommand) => void;
}) {
  const handleClick = useCallback(() => onSelect(command), [onSelect, command]);
  const handleKeyDown = useCallback((e: React.KeyboardEvent) => {
    if (e.key === "Enter" || e.key === " ") {
      e.preventDefault();
      onSelect(command);
    }
  }, [onSelect, command]);

  return (
    <div onClick={handleClick} onKeyDown={handleKeyDown}>
      {/* ... */}
    </div>
  );
});
```

---

### 6. FileCommandPopup Has Same Issues

**File:** `/frontend/components/FileCommandPopup/FileCommandPopup.tsx:49-75`

Same problem as SlashCommandPopup - inline functions in list items without memoized item components.

---

## Medium Priority Issues

### 7. ThinkingBlock ReactMarkdown Components Object

**File:** `/frontend/components/ThinkingBlock/ThinkingBlock.tsx:62-128`

```tsx
<ReactMarkdown
  remarkPlugins={[remarkGfm]}
  components={{
    h1: ({ children }) => (
      <p className="font-bold text-muted-foreground mt-2 mb-1 first:mt-0">{children}</p>
    ),
    // ... more inline component definitions
  }}
>
```

**Problem:** The `components` object is recreated on every render with new function references for each custom component. ReactMarkdown will re-render all elements when this changes.

**Fix:** Define components outside the component or memoize with useMemo:
```tsx
// Outside component
const MARKDOWN_COMPONENTS = {
  h1: ({ children }: { children: React.ReactNode }) => (
    <p className="font-bold text-muted-foreground mt-2 mb-1 first:mt-0">{children}</p>
  ),
  // ...
};

// Or inside with useMemo
const markdownComponents = useMemo(() => ({
  h1: ({ children }) => /* ... */,
  // ...
}), []);
```

**Impact:** ThinkingBlock renders during AI streaming, so this affects perceived performance.

---

### 8. InlineTaskPlan Store Subscription Pattern

**File:** `/frontend/components/InlineTaskPlan/InlineTaskPlan.tsx:21`

```tsx
const plan = useStore((state) => state.sessions[sessionId]?.plan);
```

**Problem:** Subscribes to the entire session object, causing re-renders when any session property changes, not just the plan.

**Fix:** Use a more targeted selector:
```tsx
const plan = useStore(
  useCallback((state) => state.sessions[sessionId]?.plan, [sessionId]),
  shallow
);
```

Or add to the existing `useSessionState` combined selector if plan access is common.

---

### 9. SessionBrowser Inline Functions in List

**File:** `/frontend/components/SessionBrowser/SessionBrowser.tsx:262-309`

```tsx
{filteredSessions.map((session) => (
  <button
    onClick={() => handleSelectSession(session)}
    // ...
  >
```

**Problem:** Creates new functions for onClick on every render.

**Fix:** Extract to memoized SessionItem component.

---

### 10. SettingsTabContent Navigation Item Rendering

**File:** `/frontend/components/Settings/SettingsTabContent.tsx:250-272`

```tsx
{NAV_ITEMS.map((item) => (
  <button
    key={item.id}
    onClick={() => setActiveSection(item.id)}
    // ...
  >
```

**Problem:** New onClick function created for each nav item on every render.

**Fix:** Since NAV_ITEMS is static, either:
1. Use `item.id` in onClick and memoize the handler
2. Extract NavItem as a memoized component

---

## Low Priority Issues

### 11. Context Menu Components Create Inline Functions

**File:** `/frontend/components/HomeView/HomeView.tsx:94-146` (WorktreeContextMenu)
**File:** `/frontend/components/HomeView/HomeView.tsx:296-348` (ProjectContextMenu)

The context menu buttons create inline functions, but since these are rendered conditionally and don't persist, the impact is minimal.

---

### 12. StatsBadge and WorktreeBadge Could Be Memoized

**File:** `/frontend/components/HomeView/HomeView.tsx:46-80`

These small components receive primitive props and could benefit from React.memo, though the impact is low since they're not in high-frequency render paths.

---

## Already Well-Optimized Patterns

The codebase already implements several excellent patterns:

1. **Combined Selectors** (`useSessionState`, `useUnifiedInputState`, `useTabBarState`)
   - Reduces subscription count significantly
   - Uses external cache with shallow equality

2. **Stable Empty References**
   ```tsx
   const EMPTY_TIMELINE: UnifiedBlock[] = [];
   const EMPTY_STREAMING_BLOCKS: StreamingBlock[] = [];
   ```

3. **Memoized Components**
   - `UnifiedTimeline` (memo)
   - `UnifiedBlock` (memo)
   - `TabItem` (memo)
   - `ToolItem` (memo)
   - `ToolGroup` (memo)
   - `DiffView` (memo)
   - `InlineTaskPlan` (memo)
   - `GhostTextHint` (memo in UnifiedInput)

4. **Throttled Updates**
   - Text delta batching in useAiEvents (16ms intervals)
   - Streaming text scroll triggers throttled to 50-char buckets

5. **Virtualized Timeline**
   - VirtualizedTimeline component for timeline blocks

6. **getState() Pattern for Actions**
   ```tsx
   const getToggleBlockCollapse = () => useStore.getState().toggleBlockCollapse;
   ```

---

## Recommended Priority Order for Fixes

1. **High Priority** (visible during streaming):
   - LiveTerminalBlock inline style object (#1)
   - ThinkingBlock ReactMarkdown components (#7)

2. **Medium Priority** (affects list interactions):
   - HomeView ProjectRow/RecentDirectoryRow memoization (#2, #3)
   - SlashCommandPopup list item memoization (#5)
   - FileCommandPopup list item memoization (#6)

3. **Low Priority** (minimal user impact):
   - InlineTaskPlan selector refinement (#8)
   - SessionBrowser list items (#9)
   - SettingsTabContent nav items (#10)
   - Context menu inline functions (#11)
   - Small badge components (#12)

---

## Testing Recommendations

1. **React DevTools Profiler**: Record interactions during:
   - AI streaming responses
   - Typing in UnifiedInput
   - Expanding/collapsing items in HomeView
   - Scrolling through long timelines

2. **Highlight Updates**: Enable "Highlight updates when components render" in React DevTools to visualize unnecessary re-renders.

3. **Performance Metrics**: Monitor:
   - Frame rate during streaming
   - Time to first meaningful paint
   - Interaction to next paint (INP)

---

## Summary

The Qbit frontend already follows many React performance best practices. The most impactful improvements would be:

1. Extracting inline style objects to constants in high-frequency components
2. Memoizing list item components and their callbacks
3. Moving ReactMarkdown component definitions outside render functions

These changes would reduce unnecessary React reconciliation work, especially during AI streaming operations where performance is most visible to users.
