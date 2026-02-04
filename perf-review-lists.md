# List Rendering Performance Review

## Executive Summary

The frontend codebase shows a mix of well-optimized and potentially problematic list rendering patterns. The main timeline (`UnifiedTimeline`) correctly implements virtualization with `@tanstack/react-virtual`, which is excellent for handling large message histories. However, several secondary lists lack virtualization and have other performance concerns that could impact responsiveness as datasets grow.

**Key findings:**
- UnifiedTimeline: Good virtualization implementation with threshold-based activation
- SessionBrowser: Filter operations on every render without memoization
- HistorySearchPopup: Missing virtualization for potentially large command history
- AgentMessage: Multiple array operations in render that could be expensive
- FileBrowser: No virtualization for directory listings
- HomeView: Filter operations without memoization

---

## Issues Found

### HIGH PRIORITY

#### 1. SessionBrowser - Filter operation on every render

**File:** `/frontend/components/SessionBrowser/SessionBrowser.tsx:129-137`

**Issue:** The search filter runs in a `useEffect` that depends on `searchQuery` and `sessions`, meaning it re-filters the entire session list on every keystroke without debouncing or memoization.

```tsx
// Current implementation (lines 129-137)
useEffect(() => {
  if (!searchQuery.trim()) {
    setFilteredSessions(sessions);
    return;
  }

  const query = searchQuery.toLowerCase();
  const filtered = sessions.filter(
    (session) =>
      session.workspace_label.toLowerCase().includes(query) ||
      session.model.toLowerCase().includes(query) ||
      session.first_prompt_preview?.toLowerCase().includes(query) ||
      session.first_reply_preview?.toLowerCase().includes(query)
  );
  setFilteredSessions(filtered);
}, [searchQuery, sessions]);
```

**Recommendation:** Use `useMemo` instead of `useEffect` + `useState`, and add debouncing for search input:

```tsx
import { useDeferredValue, useMemo } from "react";

// Debounce the search query
const deferredSearchQuery = useDeferredValue(searchQuery);

// Memoize filtered results
const filteredSessions = useMemo(() => {
  if (!deferredSearchQuery.trim()) {
    return sessions;
  }
  const query = deferredSearchQuery.toLowerCase();
  return sessions.filter(
    (session) =>
      session.workspace_label.toLowerCase().includes(query) ||
      session.model.toLowerCase().includes(query) ||
      session.first_prompt_preview?.toLowerCase().includes(query) ||
      session.first_reply_preview?.toLowerCase().includes(query)
  );
}, [deferredSearchQuery, sessions]);
```

---

#### 2. SessionBrowser - Messages list without virtualization

**File:** `/frontend/components/SessionBrowser/SessionBrowser.tsx:390-427`

**Issue:** Session message previews are rendered without virtualization. A session could have hundreds of messages, all rendered to the DOM.

```tsx
// Current implementation (lines 390-427)
{sessionDetail.messages.map((msg, index) => (
  <div
    key={`${msg.role}-${index}-${msg.content.slice(0, 20)}`}
    className={...}
  >
    {/* Message content */}
  </div>
))}
```

**Additional Issues:**
- Using index in key combined with unstable content slice
- No virtualization for potentially large message lists

**Recommendation:** Add virtualization using `@tanstack/react-virtual`:

```tsx
import { useVirtualizer } from "@tanstack/react-virtual";

const parentRef = useRef<HTMLDivElement>(null);

const rowVirtualizer = useVirtualizer({
  count: sessionDetail.messages.length,
  getScrollElement: () => parentRef.current,
  estimateSize: () => 80, // Estimated row height
  overscan: 5,
});

return (
  <div ref={parentRef} className="flex-1 overflow-auto">
    <div style={{ height: rowVirtualizer.getTotalSize(), position: "relative" }}>
      {rowVirtualizer.getVirtualItems().map((virtualRow) => {
        const msg = sessionDetail.messages[virtualRow.index];
        return (
          <div
            key={msg.id || `${msg.role}-${virtualRow.index}`}
            style={{
              position: "absolute",
              top: virtualRow.start,
              width: "100%",
            }}
            ref={rowVirtualizer.measureElement}
            data-index={virtualRow.index}
          >
            {/* Message content */}
          </div>
        );
      })}
    </div>
  </div>
);
```

---

#### 3. AgentMessage - Multiple array operations in render

**File:** `/frontend/components/AgentChat/AgentMessage.tsx:165-198`

**Issue:** Multiple `useMemo` calls with array operations that cascade. While individually memoized, the chain `filteredHistory -> groupedHistory -> contentBlocks` creates multiple array transformations per message.

```tsx
const filteredHistory = useMemo(() => {
  if (!message.streamingHistory) return [];
  return message.streamingHistory.filter((block) => {
    if (block.type !== "tool") return true;
    return true;
  });
}, [message.streamingHistory]);

const groupedHistory = useMemo(
  () => groupConsecutiveToolsByAny(filteredHistory),
  [filteredHistory]
);

const { contentBlocks } = useMemo(() => {
  if (!hasStreamingHistory) return { contentBlocks: [] as RenderBlock[] };
  return extractSubAgentBlocks(groupedHistory, message.subAgents || []);
}, [groupedHistory, message.subAgents, hasStreamingHistory]);
```

**Recommendation:** Consolidate into a single `useMemo` to avoid cascade:

```tsx
const contentBlocks = useMemo(() => {
  if (!message.streamingHistory?.length) return [];

  // Filter (currently a no-op but kept for future filtering)
  const filtered = message.streamingHistory.filter((block) => {
    if (block.type !== "tool") return true;
    return true;
  });

  // Group consecutive tools
  const grouped = groupConsecutiveToolsByAny(filtered);

  // Extract sub-agent blocks
  const { contentBlocks } = extractSubAgentBlocks(grouped, message.subAgents || []);

  return contentBlocks;
}, [message.streamingHistory, message.subAgents]);
```

---

### MEDIUM PRIORITY

#### 4. HistorySearchPopup - Missing virtualization

**File:** `/frontend/components/HistorySearchPopup/HistorySearchPopup.tsx:120-146`

**Issue:** Command history list is not virtualized. Shell power users can have thousands of commands in history.

```tsx
// Current implementation (lines 121-145)
{matches.map((match, index) => (
  <div
    key={`${match.index}-${match.command}`}
    role="option"
    aria-selected={index === selectedIndex}
    {...}
  >
    {highlightMatch(match.command, searchQuery)}
  </div>
))}
```

**Recommendation:** For lists that can grow unbounded, consider windowing. Since this popup has `max-h-[300px]`, a simple approach is acceptable:

```tsx
// Option 1: Simple slice for performance (quick fix)
const displayMatches = matches.slice(0, 100); // Limit visible items

// Option 2: Full virtualization (better UX)
import { useVirtualizer } from "@tanstack/react-virtual";

const listRef = useRef<HTMLDivElement>(null);
const virtualizer = useVirtualizer({
  count: matches.length,
  getScrollElement: () => listRef.current,
  estimateSize: () => 36,
  overscan: 5,
});
```

---

#### 5. highlightMatch function creates elements on every call

**File:** `/frontend/components/HistorySearchPopup/HistorySearchPopup.tsx:19-51`

**Issue:** The `highlightMatch` function creates new React elements on every render. While memoized at the component level, each item calls this during render.

```tsx
function highlightMatch(command: string, query: string): React.ReactNode {
  // Creates new span elements on every call
  parts.push(
    <span key={matchIndex} className="bg-yellow-500/30 text-yellow-600 dark:text-yellow-400">
      {command.slice(matchIndex, matchIndex + query.length)}
    </span>
  );
}
```

**Recommendation:** Memoize per-item or use CSS-based highlighting:

```tsx
// Option 1: Memoize component
const HighlightedCommand = memo(function HighlightedCommand({
  command,
  query
}: {
  command: string;
  query: string;
}) {
  return <>{highlightMatch(command, query)}</>;
});

// Option 2: Use mark element (browser-native highlighting)
<span dangerouslySetInnerHTML={{
  __html: escapeHtml(command).replace(
    new RegExp(`(${escapeRegExp(query)})`, 'gi'),
    '<mark class="bg-yellow-500/30">$1</mark>'
  )
}} />
```

---

#### 6. FileBrowser - No virtualization for directories

**File:** `/frontend/components/FileEditorSidebar/FileBrowser.tsx:175-198`

**Issue:** Directory listings are rendered without virtualization. Large directories (node_modules, .git) could have thousands of entries.

```tsx
{entries.map((entry) => (
  <button
    key={entry.path}
    type="button"
    {...}
  >
    {/* Entry content */}
  </button>
))}
```

**Recommendation:** Add virtualization or limit visible entries:

```tsx
// Quick fix: Limit entries with "Show more" button
const MAX_VISIBLE = 200;
const visibleEntries = entries.slice(0, MAX_VISIBLE);

// Better: Full virtualization for large directories
import { useVirtualizer } from "@tanstack/react-virtual";
```

---

#### 7. SlashCommandPopup - Filter function recreated on import

**File:** `/frontend/components/SlashCommandPopup/SlashCommandPopup.tsx:129-136`

**Issue:** The `filterCommands` function is exported but called during render, causing filter operation on every keystroke.

```tsx
export function filterCommands(commands: SlashCommand[], query: string): SlashCommand[] {
  const lowerQuery = query.toLowerCase();
  return commands.filter(
    (command) =>
      command.name.toLowerCase().includes(lowerQuery) ||
      command.description?.toLowerCase().includes(lowerQuery)
  );
}
```

**Recommendation:** Ensure filtering is memoized at the call site:

```tsx
// In parent component
const filteredCommands = useMemo(
  () => filterCommands(commands, query),
  [commands, query]
);
```

---

### LOW PRIORITY

#### 8. HomeView - Project and worktree lists without memoization

**File:** `/frontend/components/HomeView/HomeView.tsx:614-626, 640-647`

**Issue:** Project and directory lists are small but could benefit from `memo()` on row components for stricter optimization.

```tsx
{projects.map((project) => (
  <ProjectRow
    key={project.path}
    project={project}
    {...}
  />
))}
```

**Recommendation:** The `ProjectRow` and `RecentDirectoryRow` components should use `memo()`:

```tsx
const ProjectRow = memo(function ProjectRow({ ... }) {
  // Component implementation
});

const RecentDirectoryRow = memo(function RecentDirectoryRow({ ... }) {
  // Component implementation
});
```

---

#### 9. WorkflowTree - Nested loops in tool grouping

**File:** `/frontend/components/WorkflowTree/WorkflowTree.tsx:71-91, 217-225`

**Issue:** Tool calls are grouped by step index, then regrouped by tool name inside `StepNode`. Double grouping on every render.

```tsx
// First grouping (lines 71-91)
function groupToolCallsByStepIndex(...) {
  const groups = new Map<number, ActiveToolCall[]>();
  for (const tool of toolCalls) { ... }
  return groups;
}

// Second grouping inside StepNode (lines 217-225)
const toolGroups = useMemo(() => {
  const groups = new Map<string, ActiveToolCall[]>();
  for (const tool of toolCalls) { ... }
  return Array.from(groups.entries());
}, [toolCalls]);
```

**Recommendation:** Consider pre-computing the nested structure:

```tsx
interface GroupedStepTools {
  stepIndex: number;
  toolGroups: Map<string, ActiveToolCall[]>;
}

const groupToolCallsByStep = (toolCalls: ActiveToolCall[], workflowId: string) => {
  // Return pre-grouped structure
};
```

---

#### 10. UnifiedTimeline - Streaming blocks filter on every frame

**File:** `/frontend/components/UnifiedTimeline/UnifiedTimeline.tsx:103-202`

**Issue:** The `renderBlocks` useMemo is well-implemented but runs complex logic including Set operations and nested loops. During active streaming, this runs frequently.

```tsx
const renderBlocks = useMemo((): RenderBlock[] => {
  // Stage 1: Filter out workflow tool calls
  const filteredBlocks = streamingBlocks.filter((block) => { ... });

  // Stage 2: Group consecutive tool calls
  const groupedBlocks = groupConsecutiveToolsByAny(filteredBlocks);

  // Stage 3: Transform with Set operations and nested loops
  const matchedParentIds = new Set<string>();
  for (const block of groupedBlocks) {
    if (block.type === "tool_group") {
      for (const tool of block.tools) { ... }
    }
  }
}, [streamingBlocks, activeWorkflow, activeSubAgents]);
```

**Recommendation:** This is already well-structured. Consider moving sub-agent matching to the store level if profiling shows it as a bottleneck.

---

## Good Practices Found

1. **VirtualizedTimeline**: Properly implements `@tanstack/react-virtual` with:
   - Threshold-based activation (50 items)
   - Height estimation for initial layout
   - Measurement-based sizing for accuracy
   - Reasonable overscan (5 items)

2. **Memoized Selectors**: The `selectSessionState` pattern in `/frontend/store/selectors/session.ts` effectively batches multiple store subscriptions into one.

3. **memo() Usage**: Many list item components (`ToolItem`, `ToolGroup`, `MainToolGroup`, `ToolPreviewRow`, `UnifiedBlock`) are properly memoized.

4. **Stable Empty References**: The store correctly uses stable empty arrays (`EMPTY_TIMELINE`, `EMPTY_STREAMING_BLOCKS`) to prevent unnecessary re-renders.

5. **useCallback for Handlers**: Event handlers in list contexts generally use `useCallback` to maintain stable references.

---

## Priority Summary

| Priority | Issue | Impact | Effort |
|----------|-------|--------|--------|
| HIGH | SessionBrowser filter on every render | Laggy search UX | Low |
| HIGH | SessionBrowser messages without virtualization | Potential freeze on large sessions | Medium |
| HIGH | AgentMessage cascade memoization | Unnecessary re-computation | Low |
| MEDIUM | HistorySearchPopup missing virtualization | Slow with large history | Medium |
| MEDIUM | highlightMatch creates elements | Minor GC pressure | Low |
| MEDIUM | FileBrowser missing virtualization | Slow on large directories | Medium |
| MEDIUM | SlashCommandPopup filter memoization | Minor perf impact | Low |
| LOW | HomeView row memoization | Minimal impact | Low |
| LOW | WorkflowTree double grouping | Minor during workflows | Low |
| LOW | UnifiedTimeline streaming filter | Already well-optimized | N/A |

---

## Recommended Actions

1. **Immediate (High Priority):**
   - Add `useDeferredValue` and consolidate filter logic in SessionBrowser
   - Add virtualization to SessionBrowser messages list
   - Consolidate AgentMessage memoization chain

2. **Short-term (Medium Priority):**
   - Add virtualization or limits to HistorySearchPopup
   - Add virtualization to FileBrowser for large directories
   - Ensure filter functions are called within memoized contexts

3. **As Needed (Low Priority):**
   - Memo-wrap HomeView row components
   - Refactor WorkflowTree grouping if profiling shows impact
