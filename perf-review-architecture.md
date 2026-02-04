# Frontend Architecture Performance Review

## Summary

The Qbit frontend demonstrates **strong architectural patterns** with several performance optimizations already in place, including:

- React.lazy/Suspense for code splitting dialogs and panels
- Virtualized timeline rendering with @tanstack/react-virtual
- Memoized combined selectors to reduce store subscriptions
- Terminal portal architecture preventing unmount/remount cycles
- Stable empty array references to prevent re-renders

However, there are still opportunities for improvement in component splitting, lazy loading, error boundaries, and reducing unnecessary work in critical paths.

---

## Issues Found

### 1. UnifiedInput Component is Too Large (1479 lines)

**File:** `/Users/xlyk/Code/qbit.worktrees/frontend-improvements/frontend/components/UnifiedInput/UnifiedInput.tsx`

**Priority:** HIGH

**Problem:** The UnifiedInput component handles too many responsibilities:
- Terminal input
- Agent input
- Image attachments and drag-drop
- Path completion popup
- Slash command popup
- File command popup
- History search popup
- Vision capabilities
- Git branch display
- Multiple keyboard shortcut handlers

This causes:
1. All popup components re-render when any input state changes
2. Large bundle size loaded for every pane
3. Difficult to test and maintain

**Recommended Fix:**

Split into focused sub-components:

```tsx
// UnifiedInput/index.tsx - orchestrator only
export function UnifiedInput({ sessionId, workingDirectory }: Props) {
  const { inputMode } = useUnifiedInputState(sessionId);

  return (
    <div className="border-t border-[var(--border-subtle)]">
      <InlineTaskPlan sessionId={sessionId} />
      <PathBadgesRow
        sessionId={sessionId}
        workingDirectory={workingDirectory}
      />
      {inputMode === "terminal" ? (
        <TerminalInputField sessionId={sessionId} />
      ) : (
        <AgentInputField
          sessionId={sessionId}
          workingDirectory={workingDirectory}
        />
      )}
      <InputStatusRow sessionId={sessionId} />
    </div>
  );
}

// Separate files:
// - TerminalInputField.tsx (path completion, Ctrl+C/D/L handlers)
// - AgentInputField.tsx (slash commands, file commands, image attachments)
// - PathBadgesRow.tsx (folder, git, virtualenv badges)
// - useInputKeyboardHandlers.ts (extract keyboard logic)
```

---

### 2. Missing Lazy Loading for Heavy Components in PaneLeaf

**File:** `/Users/xlyk/Code/qbit.worktrees/frontend-improvements/frontend/components/PaneContainer/PaneLeaf.tsx:12-15`

**Priority:** HIGH

**Problem:** PaneLeaf directly imports heavy components that aren't needed on initial render:

```tsx
import { ToolApprovalDialog } from "@/components/AgentChat";
import { HomeView } from "@/components/HomeView";
import { SettingsTabContent } from "@/components/Settings/SettingsTabContent";
import { UnifiedTimeline } from "@/components/UnifiedTimeline";
```

These are loaded for ALL panes even if:
- HomeView is only needed in home tabs
- SettingsTabContent is only needed in settings tabs
- ToolApprovalDialog is only needed when a tool needs approval

**Recommended Fix:**

```tsx
import { lazy, Suspense } from "react";

// Lazy load tab-specific content
const HomeView = lazy(() =>
  import("@/components/HomeView").then(m => ({ default: m.HomeView }))
);
const SettingsTabContent = lazy(() =>
  import("@/components/Settings/SettingsTabContent").then(m => ({ default: m.SettingsTabContent }))
);
const ToolApprovalDialog = lazy(() =>
  import("@/components/AgentChat/ToolApprovalDialog").then(m => ({ default: m.ToolApprovalDialog }))
);

// In render:
const renderTabContent = () => {
  switch (tabType) {
    case "home":
      return (
        <Suspense fallback={<TabLoadingSkeleton />}>
          <HomeView />
        </Suspense>
      );
    case "settings":
      return (
        <Suspense fallback={<TabLoadingSkeleton />}>
          <SettingsTabContent />
        </Suspense>
      );
    // ...
  }
};
```

---

### 3. Markdown Component Loads Heavy Dependencies Synchronously

**File:** `/Users/xlyk/Code/qbit.worktrees/frontend-improvements/frontend/components/Markdown/Markdown.tsx:9-11`

**Priority:** HIGH

**Problem:** react-syntax-highlighter imports are synchronous and very large:

```tsx
import ReactMarkdown from "react-markdown";
import { Prism as SyntaxHighlighter } from "react-syntax-highlighter";
import { oneDark } from "react-syntax-highlighter/dist/esm/styles/prism";
```

These libraries are loaded on first render even for simple text content without code blocks.

**Recommended Fix:**

```tsx
import { lazy, Suspense } from "react";

// Lazy load syntax highlighter only when code blocks are present
const SyntaxHighlighter = lazy(() =>
  import("react-syntax-highlighter").then(m => ({
    default: m.Prism
  }))
);

// Lazy load the theme
const codeThemePromise = import("react-syntax-highlighter/dist/esm/styles/prism")
  .then(m => m.oneDark);

function CodeBlock({ language, code }: { language: string; code: string }) {
  return (
    <Suspense fallback={<pre className="font-mono p-4 bg-muted">{code}</pre>}>
      <SyntaxHighlighterWrapper language={language} code={code} />
    </Suspense>
  );
}
```

---

### 4. Missing Error Boundaries Around Critical Timeline Blocks

**File:** `/Users/xlyk/Code/qbit.worktrees/frontend-improvements/frontend/components/UnifiedTimeline/UnifiedTimeline.tsx:317-398`

**Priority:** MEDIUM

**Problem:** While VirtualizedTimeline wraps blocks in TimelineBlockErrorBoundary, the streaming blocks in UnifiedTimeline are not protected:

```tsx
{renderBlocks.map((block, blockIndex) => {
  // These can throw during render and crash the entire timeline
  if (block.type === "thinking") {
    return <StaticThinkingBlock ... />;
  }
  if (block.type === "text") {
    return <Markdown ... />;  // Can throw on malformed content
  }
  // ... etc
})}
```

If any streaming block throws, the entire agent response disappears.

**Recommended Fix:**

```tsx
// Create a lightweight streaming block boundary
function StreamingBlockBoundary({ children, blockType }: {
  children: React.ReactNode;
  blockType: string;
}) {
  return (
    <ErrorBoundary
      fallback={
        <div className="text-xs text-destructive p-2 bg-destructive/10 rounded">
          Failed to render {blockType} block
        </div>
      }
    >
      {children}
    </ErrorBoundary>
  );
}

// Wrap each block type:
{renderBlocks.map((block, blockIndex) => {
  if (block.type === "text") {
    return (
      <StreamingBlockBoundary key={`text-${blockIndex}`} blockType="text">
        <Markdown ... />
      </StreamingBlockBoundary>
    );
  }
  // ...
})}
```

---

### 5. App.tsx Has Excessive useCallback Dependencies

**File:** `/Users/xlyk/Code/qbit.worktrees/frontend-improvements/frontend/App.tsx:446-629`

**Priority:** MEDIUM

**Problem:** The keyboard handler effect has 14 dependencies, causing frequent recreation:

```tsx
useEffect(() => {
  const handleKeyDown = (e: KeyboardEvent) => { /* 150+ lines */ };
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

When any dependency changes, the handler is re-created and re-attached.

**Recommended Fix:**

Use a ref pattern to avoid dependency churn:

```tsx
// Store current values in a ref
const keyboardContextRef = useRef({
  sessions,
  activeSessionId,
  gitPanelOpen,
});

// Update ref on render (no effect dependency needed)
keyboardContextRef.current = {
  sessions,
  activeSessionId,
  gitPanelOpen,
};

// Stable effect with no value dependencies
useEffect(() => {
  const handleKeyDown = (e: KeyboardEvent) => {
    const { sessions, activeSessionId } = keyboardContextRef.current;
    // ... use current values from ref
  };

  window.addEventListener("keydown", handleKeyDown);
  return () => window.removeEventListener("keydown", handleKeyDown);
}, []); // Empty deps - handler never recreated
```

---

### 6. HomeView Loads All Data on Every Mount

**File:** `/Users/xlyk/Code/qbit.worktrees/frontend-improvements/frontend/components/HomeView/HomeView.tsx:387-389`

**Priority:** MEDIUM

**Problem:** HomeView fetches projects and recent directories on every mount, even when switching tabs rapidly:

```tsx
useEffect(() => {
  fetchData();
}, [fetchData]);
```

This causes unnecessary backend calls when:
- Switching to another tab and back
- Opening/closing modals
- Re-rendering due to parent changes

**Recommended Fix:**

Use SWR or React Query pattern with caching:

```tsx
// Option 1: Simple stale-while-revalidate
const [data, setData] = useState<{ projects: ProjectInfo[]; recent: RecentDirectory[] } | null>(
  () => homeDataCache.current // Start with cached value if available
);

useEffect(() => {
  // Show cached data immediately, fetch fresh data in background
  if (homeDataCache.current && !isStale(homeDataCache.timestamp)) {
    setData(homeDataCache.current);
    return;
  }

  fetchData().then(newData => {
    homeDataCache.current = newData;
    homeDataCache.timestamp = Date.now();
    setData(newData);
  });
}, []);

// Option 2: Use @tanstack/react-query
const { data, isLoading, refetch } = useQuery({
  queryKey: ['home-view'],
  queryFn: async () => ({
    projects: await listProjectsForHome(),
    recent: await listRecentDirectories(10),
  }),
  staleTime: 30_000, // Consider data fresh for 30 seconds
});
```

---

### 7. AgentMessage Component Creates New Arrays on Every Render

**File:** `/Users/xlyk/Code/qbit.worktrees/frontend-improvements/frontend/components/AgentChat/AgentMessage.tsx:165-186`

**Priority:** MEDIUM

**Problem:** Multiple useMemo calls with message dependencies that change frequently during streaming:

```tsx
const filteredHistory = useMemo(() => {
  if (!message.streamingHistory) return [];
  return message.streamingHistory.filter(...);
}, [message.streamingHistory]);

const groupedHistory = useMemo(
  () => groupConsecutiveToolsByAny(filteredHistory),
  [filteredHistory]
);

const { contentBlocks } = useMemo(() => {
  // Creates new object on every message.subAgents change
}, [groupedHistory, message.subAgents, hasStreamingHistory]);
```

When streaming, message.streamingHistory changes on every token, triggering all three memos.

**Recommended Fix:**

```tsx
// Combine into single memo to reduce cascade
const contentBlocks = useMemo(() => {
  if (!message.streamingHistory?.length) {
    return EMPTY_BLOCKS;
  }

  const filtered = message.streamingHistory.filter(block => {
    if (block.type !== "tool") return true;
    return true;
  });

  const grouped = groupConsecutiveToolsByAny(filtered);
  return extractSubAgentBlocks(grouped, message.subAgents || EMPTY_SUB_AGENTS).contentBlocks;
}, [message.streamingHistory, message.subAgents]);
```

---

### 8. Settings Dialog Doesn't Code Split Tab Content

**File:** `/Users/xlyk/Code/qbit.worktrees/frontend-improvements/frontend/components/Settings/index.tsx:26-33`

**Priority:** LOW

**Problem:** All settings tab components are imported synchronously:

```tsx
import { AdvancedSettings } from "./AdvancedSettings";
import { AgentSettings } from "./AgentSettings";
import { AiSettings } from "./AiSettings";
import { CodebasesSettings } from "./CodebasesSettings";
import { EditorSettings } from "./EditorSettings";
import { NotificationsSettings } from "./NotificationsSettings";
import { ProviderSettings } from "./ProviderSettings";
import { TerminalSettings } from "./TerminalSettings";
```

All 8 settings panels are loaded even though only one is visible at a time.

**Recommended Fix:**

```tsx
import { lazy, Suspense } from "react";

const settingsComponents = {
  providers: lazy(() => import("./ProviderSettings")),
  ai: lazy(() => import("./AiSettings")),
  terminal: lazy(() => import("./TerminalSettings")),
  editor: lazy(() => import("./EditorSettings")),
  agent: lazy(() => import("./AgentSettings")),
  codebases: lazy(() => import("./CodebasesSettings")),
  notifications: lazy(() => import("./NotificationsSettings")),
  advanced: lazy(() => import("./AdvancedSettings")),
};

const renderContent = () => {
  const Component = settingsComponents[activeSection];
  return (
    <Suspense fallback={<SettingsLoadingSkeleton />}>
      <Component settings={settings} onChange={updateSection} />
    </Suspense>
  );
};
```

---

### 9. TabBar Re-renders All Tabs on Any Tab Change

**File:** `/Users/xlyk/Code/qbit.worktrees/frontend-improvements/frontend/components/TabBar/TabBar.tsx:148-166`

**Priority:** LOW

**Problem:** While TabItem is memoized, the parent TabBar still iterates all tabs on every render:

```tsx
{tabs.map((tab, index) => {
  const isActive = tab.id === activeSessionId;
  const isBusy = tab.tabType === "terminal" && (tab.isRunning || tab.hasPendingCommand);
  const hasNewActivity = tab.tabType === "terminal" && !isActive && tab.hasNewActivity;

  return (
    <TabItem
      key={tab.id}
      tab={tab}
      isActive={isActive}
      isBusy={isBusy}
      // ...
    />
  );
})}
```

Computing `isBusy` and `hasNewActivity` inline prevents proper memoization.

**Recommended Fix:**

Move computed props into the selector or TabItem:

```tsx
// Option 1: Include computed values in selector
interface TabItemState {
  // ... existing fields
  isActive: boolean;  // Computed in selector
  isBusy: boolean;    // Computed in selector
  hasNewActivity: boolean;  // Already computed, just pass through
}

// Option 2: Let TabItem compute internally
const TabItem = memo(function TabItem({ tab, activeSessionId }: Props) {
  const isActive = tab.id === activeSessionId;
  const isBusy = tab.tabType === "terminal" && (tab.isRunning || tab.hasPendingCommand);
  // ...
});
```

---

### 10. Terminal Component Creates Multiple Event Listeners

**File:** `/Users/xlyk/Code/qbit.worktrees/frontend-improvements/frontend/components/Terminal/Terminal.tsx:234-287`

**Priority:** LOW

**Problem:** Multiple async listeners are set up inside useEffect with race condition handling:

```tsx
(async () => {
  const unlistenOutput = await listen<TerminalOutputEvent>("terminal_output", ...);
  const unlistenSync = await listen<...>("synchronized_output", ...);

  if (aborted) {
    unlistenSync();
    unlistenOutput();
    return;
  }
  cleanupFnsRef.current.push(unlistenSync);
  cleanupFnsRef.current.push(unlistenOutput);
})();
```

This pattern is correct but could be simplified with a custom hook.

**Recommended Fix:**

```tsx
// hooks/useTauriEvent.ts
function useTauriEvent<T>(
  eventName: string,
  handler: (payload: T) => void,
  deps: DependencyList = []
) {
  const handlerRef = useRef(handler);
  handlerRef.current = handler;

  useEffect(() => {
    let unlisten: UnlistenFn | undefined;
    let aborted = false;

    listen<T>(eventName, (event) => {
      if (!aborted) {
        handlerRef.current(event.payload);
      }
    }).then(fn => {
      if (aborted) {
        fn();
      } else {
        unlisten = fn;
      }
    });

    return () => {
      aborted = true;
      unlisten?.();
    };
  }, deps);
}

// In Terminal.tsx
useTauriEvent<TerminalOutputEvent>(
  "terminal_output",
  (payload) => {
    if (payload.session_id === sessionId && syncBufferRef.current) {
      syncBufferRef.current.write(payload.data);
    }
  },
  [sessionId]
);
```

---

## Priority Summary

| Priority | Issue | Impact |
|----------|-------|--------|
| HIGH | UnifiedInput is too large (1479 lines) | Bundle size, re-renders, maintainability |
| HIGH | Missing lazy loading in PaneLeaf | Initial load time, memory |
| HIGH | Markdown loads heavy deps synchronously | Bundle size, first paint |
| MEDIUM | Missing error boundaries on streaming blocks | Error recovery |
| MEDIUM | App.tsx keyboard handler dependency churn | Handler recreation |
| MEDIUM | HomeView fetches on every mount | Unnecessary backend calls |
| MEDIUM | AgentMessage memo cascade | Streaming performance |
| LOW | Settings dialog doesn't code split | Bundle size |
| LOW | TabBar computes props inline | Minor re-render overhead |
| LOW | Terminal listener pattern | Code clarity |

---

## Positive Patterns Already Present

The codebase already implements many excellent patterns:

1. **App.tsx lazy loading** - Dialogs and panels are properly code split
2. **VirtualizedTimeline** - Only visible items render
3. **Combined selectors** - `useSessionState`, `useUnifiedInputState`, `useTabBarState` reduce subscriptions
4. **Terminal portal architecture** - Prevents unmount cycles on pane changes
5. **Stable empty references** - `EMPTY_TIMELINE`, `EMPTY_STREAMING_BLOCKS`, etc.
6. **Memoized components** - Most timeline/chat components use `memo()`
7. **TimelineBlockErrorBoundary** - Virtualized blocks are error-protected
8. **Streaming markdown optimization** - Uses lightweight renderer during streaming

These patterns should be extended to the remaining components as outlined above.
