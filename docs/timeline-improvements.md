# Unified Timeline Improvements

This document outlines 26 improvements identified for the unified timeline system in the Qbit frontend. The timeline is responsible for rendering commands, agent messages, tool calls, and streaming content in an interleaved view.

## Overview

The unified timeline consists of several key components:

- **`UnifiedTimeline.tsx`** - Main container that renders sorted timeline blocks and streaming content
- **`UnifiedBlock.tsx`** - Dispatcher component that renders the appropriate component for each block type
- **`AgentMessage.tsx`** - Renders finalized agent messages with tool calls, sub-agents, and workflows
- **`useAiEvents.ts`** - Hook that handles AI events from the backend and updates store state
- **`store/index.ts`** - Zustand store with timeline state management

## Improvement Categories

1. [State Management](#state-management) (#1)
2. [Performance](#performance) (#2-4, #17, #23, #25)
3. [Code Organization](#code-organization) (#5, #7-8, #11-13)
4. [Reliability](#reliability) (#14-15)
5. [React Best Practices](#react-best-practices) (#6, #10, #16, #18-20)
6. [Accessibility](#accessibility) (#21)
7. [User Experience](#user-experience) (#22)
8. [Code Quality](#code-quality) (#9, #24, #26)

---

## Detailed Improvements

### #1: Eliminate Duplicate State

**Status:** ❌ Not Implemented
**Priority:** High
**Effort:** Medium
**Category:** State Management

#### Problem

The store maintains three parallel data structures for timeline content:

```typescript
// store/index.ts
timelines: Record<string, UnifiedBlock[]>;      // Master timeline
commandBlocks: Record<string, CommandBlock[]>;  // Duplicated
agentMessages: Record<string, AgentMessage[]>;  // Duplicated
```

Every mutation must update multiple arrays, risking inconsistency:

```typescript
// Example from addAgentMessage
state.agentMessages[sessionId].push(message);
state.timelines[sessionId].push({ type: "agent_message", ... });
```

#### Solution

Make `timelines` the single source of truth. Create derived selectors:

```typescript
export const useCommandBlocks = (sessionId: string) =>
  useStore((state) =>
    state.timelines[sessionId]
      ?.filter((b): b is UnifiedBlock & { type: "command" } => b.type === "command")
      .map(b => b.data) ?? []
  );

export const useAgentMessages = (sessionId: string) =>
  useStore((state) =>
    state.timelines[sessionId]
      ?.filter((b): b is UnifiedBlock & { type: "agent_message" } => b.type === "agent_message")
      .map(b => b.data) ?? []
  );
```

#### Benefits

- Eliminates duplicate state and potential inconsistencies
- Reduces memory usage
- Simplifies mutation logic
- Single place to update for new block types

---

### #2: O(n²) Filter in sortedTimeline

**Status:** ❌ Not Implemented
**Priority:** Medium
**Effort:** Low
**Category:** Performance

#### Problem

The system hook filtering logic in `UnifiedTimeline.tsx` has O(n²) complexity:

```typescript
// UnifiedTimeline.tsx:63-67
return sorted.filter((block, index) => {
  if (block.type !== "system_hook") return true;
  const hasSubsequentMessage = sorted.slice(index + 1).some((b) => b.type === "agent_message");
  return !hasSubsequentMessage;
});
```

For each `system_hook` block, it scans all subsequent blocks. With many blocks, this becomes slow.

#### Solution

Use a single reverse pass:

```typescript
const sortedTimeline = useMemo(() => {
  const sorted = /* existing sort */;

  // Single reverse pass to identify which system_hooks to keep
  let hasSeenAgentMessage = false;
  const systemHooksToKeep = new Set<string>();

  for (let i = sorted.length - 1; i >= 0; i--) {
    if (sorted[i].type === "agent_message") {
      hasSeenAgentMessage = true;
    } else if (sorted[i].type === "system_hook" && !hasSeenAgentMessage) {
      systemHooksToKeep.add(sorted[i].id);
    }
  }

  return sorted.filter(b =>
    b.type !== "system_hook" || systemHooksToKeep.has(b.id)
  );
}, [timeline]);
```

#### Benefits

- Reduces complexity from O(n²) to O(n)
- Noticeable improvement for long sessions

---

### #3: Avoid Sorting Already-Ordered Data

**Status:** ❌ Not Implemented
**Priority:** Low
**Effort:** Low
**Category:** Performance

#### Problem

Blocks are appended chronologically, but the timeline is re-sorted on every change:

```typescript
const sorted = [...timeline]
  .map((block, index) => ({ block, index }))
  .sort((a, b) => {
    const ta = new Date(a.block.timestamp).getTime();
    const tb = new Date(b.block.timestamp).getTime();
    return ta - tb || a.index - b.index;
  });
```

#### Solution

Track whether sorting is needed:

```typescript
// In store state
timelines: Record<string, { blocks: UnifiedBlock[]; needsSort: boolean }>;

// When adding a block
const last = state.timelines[sessionId].blocks.at(-1);
if (last && new Date(newBlock.timestamp) < new Date(last.timestamp)) {
  state.timelines[sessionId].needsSort = true;
}

// In useMemo
const sortedTimeline = useMemo(() => {
  if (!timeline.needsSort) return timeline.blocks;
  return [...timeline.blocks].sort(/* ... */);
}, [timeline]);
```

#### Benefits

- Avoids unnecessary sorting for the common case
- Reduces CPU usage during rapid updates

---

### #4: Add Virtualization for Long Sessions

**Status:** ❌ Not Implemented
**Priority:** High
**Effort:** Medium
**Category:** Performance

#### Problem

All timeline blocks are rendered regardless of viewport visibility. Long sessions with hundreds of blocks cause performance issues.

#### Solution

Use `@tanstack/react-virtual`:

```typescript
import { useVirtualizer } from "@tanstack/react-virtual";

const virtualizer = useVirtualizer({
  count: sortedTimeline.length,
  getScrollElement: () => containerRef.current,
  estimateSize: (index) => estimateBlockHeight(sortedTimeline[index]),
  overscan: 5,
});

return (
  <div ref={containerRef} style={{ height: "100%", overflow: "auto" }}>
    <div style={{ height: virtualizer.getTotalSize() }}>
      {virtualizer.getVirtualItems().map((virtualRow) => (
        <div
          key={virtualRow.key}
          style={{
            position: "absolute",
            top: virtualRow.start,
            width: "100%",
          }}
        >
          <UnifiedBlock block={sortedTimeline[virtualRow.index]} />
        </div>
      ))}
    </div>
  </div>
);
```

#### Benefits

- Only visible blocks are rendered
- Constant memory usage regardless of timeline length
- Smooth scrolling for long sessions

---

### #5: Split Store into Slices

**Status:** ❌ Not Implemented
**Priority:** Medium
**Effort:** High
**Category:** Code Organization

#### Problem

The store is a 2000+ line monolithic file mixing many concerns:
- Session management
- Timeline state
- Pane/tab layouts
- Notifications
- Workflows
- Context metrics
- Git status

#### Solution

Use Zustand's slice pattern:

```typescript
// store/slices/timeline.ts
export const createTimelineSlice = (set, get) => ({
  timelines: {},
  streamingBlocks: {},
  addToTimeline: (sessionId, block) => set(state => { ... }),
  clearTimeline: (sessionId) => set(state => { ... }),
});

// store/slices/session.ts
export const createSessionSlice = (set, get) => ({
  sessions: {},
  activeSessionId: null,
  addSession: (session) => set(state => { ... }),
});

// store/index.ts
export const useStore = create<QbitState>()(
  devtools(
    immer((...a) => ({
      ...createTimelineSlice(...a),
      ...createSessionSlice(...a),
      ...createWorkflowSlice(...a),
      ...createPaneSlice(...a),
    }))
  )
);
```

#### Benefits

- Better code organization
- Easier to test individual slices
- Clearer ownership of state
- Smaller files, easier navigation

---

### #6: Simplify renderBlocks Computation

**Status:** ❌ Not Implemented (Intentionally)
**Priority:** Low
**Effort:** Medium
**Category:** React Best Practices

#### Problem

The `renderBlocks` computation in `UnifiedTimeline.tsx` does multiple passes and has complex sub-agent replacement logic.

#### Current State

The streaming view (UnifiedTimeline) and finalized view (AgentMessage) have different rendering patterns:
- Streaming: Sub-agents appear inline where their tool call occurred
- Finalized: Sub-agents appear at the top before content

The shared `extractSubAgentBlocks` utility was created for the finalized pattern. The streaming pattern intentionally keeps inline logic for correct positioning.

#### Future Consideration

Could create a variant utility `extractSubAgentBlocksInline()` that preserves positions, but the current duplication is acceptable given the different semantics.

---

### #7: Type-Safe Block Rendering

**Status:** ❌ Not Implemented
**Priority:** Low
**Effort:** Low
**Category:** Code Organization

#### Problem

`UnifiedBlock` uses a switch statement requiring manual type narrowing:

```typescript
switch (block.type) {
  case "command":
    return <CommandBlock block={block.data} />; // block.data is any here
  // ...
}
```

#### Solution

Add exhaustive type guards:

```typescript
// lib/timeline/typeGuards.ts
export function isCommandBlock(
  block: UnifiedBlock
): block is UnifiedBlock & { type: "command"; data: CommandBlock } {
  return block.type === "command";
}

export function assertNever(x: never): never {
  throw new Error(`Unexpected block type: ${x}`);
}

// Usage
function renderBlock(block: UnifiedBlock): ReactNode {
  if (isCommandBlock(block)) return <CommandBlock block={block.data} />;
  if (isAgentMessageBlock(block)) return <AgentMessage message={block.data} />;
  if (isSystemHookBlock(block)) return <SystemHooksCard hooks={block.data.hooks} />;
  return assertNever(block); // Compile-time exhaustiveness check
}
```

#### Benefits

- Compile-time exhaustiveness checking
- Clearer type narrowing
- Catches missing cases when adding new block types

---

### #8: Decouple Event Handling

**Status:** ❌ Not Implemented
**Priority:** Medium
**Effort:** Medium
**Category:** Code Organization

#### Problem

`useAiEvents.ts` is a 700+ line hook with a massive switch statement handling 30+ event types. This is hard to test and maintain.

#### Solution

Use an event handler registry:

```typescript
// hooks/aiEventHandlers/types.ts
export type EventHandler<T extends AiEvent["type"]> = (
  event: Extract<AiEvent, { type: T }>,
  state: QbitState,
  sessionId: string
) => void;

// hooks/aiEventHandlers/textDelta.ts
export const handleTextDelta: EventHandler<"text_delta"> = (event, state, sessionId) => {
  state.setAgentThinking(sessionId, false);
  state.updateAgentStreaming(sessionId, event.delta);
};

// hooks/aiEventHandlers/index.ts
export const handlers: Partial<Record<AiEvent["type"], EventHandler<any>>> = {
  text_delta: handleTextDelta,
  tool_approval_request: handleToolApprovalRequest,
  completed: handleCompleted,
  // ...
};

// useAiEvents.ts
const handleEvent = (event: AiEvent) => {
  const handler = handlers[event.type];
  if (handler) {
    handler(event, state, sessionId);
  }
};
```

#### Benefits

- Each handler is independently testable
- Easier to add new event types
- Clearer separation of concerns
- Smaller, focused files

---

### #9: Use Stable Block IDs

**Status:** ❌ Not Implemented
**Priority:** Low
**Effort:** Low
**Category:** Code Quality

#### Problem

Block IDs are generated with `crypto.randomUUID()`. This makes it impossible to resume sessions or deduplicate events based on ID.

#### Solution

Use compound IDs based on session, turn, and sequence:

```typescript
const blockId = `${sessionId}-turn${turnId}-seq${eventSeq}`;
// Example: "abc123-turn5-seq42"
```

#### Benefits

- Enables session resumption
- Facilitates event deduplication
- Easier debugging (IDs are meaningful)

---

### #10: Optimize UnifiedBlock Memoization

**Status:** ❌ Not Implemented
**Priority:** Low
**Effort:** Low
**Category:** React Best Practices

#### Problem

`UnifiedBlock` is wrapped in `memo()`, but store selectors may trigger unnecessary re-renders.

#### Solution

Use stable selector references:

```typescript
export const UnifiedBlock = memo(function UnifiedBlock({ block, sessionId }) {
  // Use useCallback for stable selector
  const toggleBlockCollapse = useStore(
    useCallback((state) => state.toggleBlockCollapse, [])
  );
  // ...
});
```

Or use Zustand's `useShallow` for object selectors:

```typescript
import { useShallow } from "zustand/react/shallow";

const { toggleBlockCollapse, isCompacting } = useStore(
  useShallow((state) => ({
    toggleBlockCollapse: state.toggleBlockCollapse,
    isCompacting: state.isCompacting[sessionId],
  }))
);
```

---

### #11: Deduplicate Sub-Agent Extraction Logic

**Status:** ✅ Implemented
**Priority:** High
**Effort:** Low
**Category:** Code Organization

#### Problem

The sub-agent extraction logic was duplicated in:
- `UnifiedTimeline.tsx` (lines 120-177)
- `AgentMessage.tsx` (lines 156-237)

Both did the same matching by `parentRequestId` with legacy fallback.

#### Solution

Created `lib/timeline/subAgentExtraction.ts`:

```typescript
export function extractSubAgentBlocks(
  groupedBlocks: GroupedStreamingBlock[],
  subAgents: ActiveSubAgent[]
): { subAgentBlocks: RenderBlock[]; contentBlocks: RenderBlock[] }
```

Now `AgentMessage` uses the shared utility. `UnifiedTimeline` keeps inline logic due to different rendering semantics (inline vs. top-of-message).

#### Files Changed

- Created: `frontend/lib/timeline/subAgentExtraction.ts`
- Created: `frontend/lib/timeline/subAgentExtraction.test.ts` (13 tests)
- Modified: `frontend/components/AgentChat/AgentMessage.tsx`

---

### #12: Deduplicate System Hooks UI

**Status:** ✅ Implemented
**Priority:** Medium
**Effort:** Low
**Category:** Code Organization

#### Problem

Nearly identical system hooks rendering existed in:
- `UnifiedBlock.tsx` (lines 29-57)
- `AgentMessage.tsx` (lines 92-121, `SystemHooksCard` function)

#### Solution

Created shared `components/SystemHooksCard`:

```typescript
export const SystemHooksCard = memo(function SystemHooksCard({ hooks }: { hooks: string[] }) {
  // Collapsible display with hook count
});
```

#### Files Changed

- Created: `frontend/components/SystemHooksCard/SystemHooksCard.tsx`
- Created: `frontend/components/SystemHooksCard/SystemHooksCard.test.tsx` (12 tests)
- Modified: `frontend/components/AgentChat/AgentMessage.tsx` (uses shared component)

---

### #13: Deduplicate Streaming Block Finalization

**Status:** ✅ Implemented
**Priority:** Medium
**Effort:** Low
**Category:** Code Organization

#### Problem

The `StreamingBlock[]` → `FinalizedStreamingBlock[]` conversion appeared twice in `useAiEvents.ts`:
- Lines 270-300 (in `completed` handler)
- Lines 548-577 (in `compaction_failed` handler)

#### Solution

Created `lib/timeline/streamingBlockFinalization.ts`:

```typescript
export function finalizeStreamingBlocks(blocks: StreamingBlock[]): FinalizedStreamingBlock[] {
  return blocks.map((block) => {
    if (block.type === "text") return { type: "text", content: block.content };
    if (block.type === "udiff_result") return { ... };
    // Convert ActiveToolCall to ToolCall
    return { type: "tool", toolCall: { ... } };
  });
}

export function extractToolCalls(blocks: FinalizedStreamingBlock[]): ToolCall[] {
  return blocks.filter(b => b.type === "tool").map(b => b.toolCall);
}
```

#### Files Changed

- Created: `frontend/lib/timeline/streamingBlockFinalization.ts`
- Created: `frontend/lib/timeline/streamingBlockFinalization.test.ts` (11 tests)

**Note:** The utility is exported but not yet integrated into `useAiEvents.ts` to minimize risk. Integration can be done in a follow-up.

---

### #14: Fix Memory Leak in Sequence Tracking

**Status:** ✅ Implemented
**Priority:** High
**Effort:** Low
**Category:** Reliability

#### Problem

`lastSeenSeq` Map in `useAiEvents.ts` is module-level and never cleaned up when sessions are removed:

```typescript
// useAiEvents.ts:16
const lastSeenSeq = new Map<string, number>();
```

Over time, this grows indefinitely as sessions are created and closed.

#### Solution

Added `resetSessionSequence()` calls when sessions are removed:

```typescript
// store/index.ts - removeSession
import("@/hooks/useAiEvents").then(({ resetSessionSequence }) => {
  resetSessionSequence(sessionId);
});

// Also added to closePane and closeTab
```

#### Files Changed

- Modified: `frontend/store/index.ts` (added cleanup in `removeSession`, `closePane`, `closeTab`)

---

### #15: Add Error Boundaries for Timeline Blocks

**Status:** ✅ Implemented
**Priority:** High
**Effort:** Low
**Category:** Reliability

#### Problem

No error boundaries around timeline blocks. One malformed block could crash the entire timeline:

```typescript
// Before: If AgentMessage throws, entire timeline crashes
{sortedTimeline.map((block) => (
  <UnifiedBlock key={block.id} block={block} />
))}
```

#### Solution

Created `TimelineBlockErrorBoundary`:

```typescript
export class TimelineBlockErrorBoundary extends Component<Props, State> {
  static getDerivedStateFromError(error: Error) {
    return { hasError: true, error };
  }

  render() {
    if (this.state.hasError) {
      return (
        <div className="border border-red-500 p-3">
          <p>Failed to render block: {this.props.blockId}</p>
          <p>{this.state.error?.message}</p>
        </div>
      );
    }
    return this.props.children;
  }
}
```

Usage in `UnifiedTimeline.tsx`:

```typescript
{sortedTimeline.map((block) => (
  <TimelineBlockErrorBoundary key={block.id} blockId={block.id}>
    <UnifiedBlock block={block} sessionId={sessionId} />
  </TimelineBlockErrorBoundary>
))}
```

#### Files Changed

- Created: `frontend/components/TimelineBlockErrorBoundary/TimelineBlockErrorBoundary.tsx`
- Created: `frontend/components/TimelineBlockErrorBoundary/TimelineBlockErrorBoundary.test.tsx` (7 tests)
- Modified: `frontend/components/UnifiedTimeline/UnifiedTimeline.tsx`

---

### #16: Lift Tool Modal State

**Status:** ❌ Not Implemented
**Priority:** Low
**Effort:** Medium
**Category:** React Best Practices

#### Problem

`selectedTool` and `selectedToolGroup` state exists in both:
- `UnifiedTimeline.tsx` (lines 83-86)
- `AgentMessage.tsx` (lines 136-137)

This causes separate modal instances and inconsistent behavior.

#### Solution

Create a `ToolModalProvider` context or lift state to store:

```typescript
// contexts/ToolModalContext.tsx
export const ToolModalContext = createContext<{
  selectedTool: AnyToolCall | null;
  setSelectedTool: (tool: AnyToolCall | null) => void;
  selectedToolGroup: AnyToolCall[] | null;
  setSelectedToolGroup: (tools: AnyToolCall[] | null) => void;
}>(...);

// App.tsx
<ToolModalProvider>
  <UnifiedTimeline />
  <ToolDetailsModal /> {/* Single instance */}
  <ToolGroupDetailsModal />
</ToolModalProvider>
```

---

### #17: Simplify Auto-Scroll Effect

**Status:** ❌ Not Implemented
**Priority:** Medium
**Effort:** Medium
**Category:** Performance

#### Problem

The auto-scroll effect has 12 dependencies and complex workarounds:

```typescript
// UnifiedTimeline.tsx:207-223
const streamingTextBucket = Math.floor(streamingTextLength / 50); // Throttle

useEffect(() => {
  // Complex scroll logic with many dependencies
}, [
  sortedTimeline.length,
  streamingBlocks.length,
  streamingTextBucket,
  isThinking,
  pendingCommand,
  // ... more deps
]);
```

#### Solution

Use IntersectionObserver to only scroll when user is at bottom:

```typescript
function useIsAtBottom(ref: RefObject<HTMLElement>, threshold = 50) {
  const [isAtBottom, setIsAtBottom] = useState(true);

  useEffect(() => {
    const el = ref.current;
    if (!el) return;

    const handleScroll = () => {
      const { scrollTop, scrollHeight, clientHeight } = el;
      setIsAtBottom(scrollHeight - scrollTop - clientHeight < threshold);
    };

    el.addEventListener("scroll", handleScroll, { passive: true });
    return () => el.removeEventListener("scroll", handleScroll);
  }, [ref, threshold]);

  return isAtBottom;
}

// Usage
const isAtBottom = useIsAtBottom(containerRef);
useEffect(() => {
  if (isAtBottom) scrollToBottom();
}, [contentLength, isAtBottom]);
```

---

### #18: Replace Index-Based React Keys

**Status:** ❌ Not Implemented
**Priority:** Low
**Effort:** Low
**Category:** React Best Practices

#### Problem

Multiple places use array index as key with biome-ignore comments:

```typescript
// Multiple files
{blocks.map((block, index) => (
  // biome-ignore lint/suspicious/noArrayIndexKey: blocks have no stable id
  <Component key={index} />
))}
```

This can cause issues if blocks are reordered.

#### Solution

Generate stable keys from content:

```typescript
function hashString(str: string): string {
  let hash = 0;
  for (let i = 0; i < str.length; i++) {
    hash = ((hash << 5) - hash) + str.charCodeAt(i);
    hash |= 0;
  }
  return hash.toString(36);
}

// Usage
{blocks.map((block, index) => (
  <Component key={`${block.type}-${hashString(block.content?.slice(0, 50) ?? index.toString())}`} />
))}
```

---

### #19: Combine Store Selectors

**Status:** ❌ Not Implemented
**Priority:** Medium
**Effort:** Low
**Category:** React Best Practices

#### Problem

`UnifiedTimeline.tsx` has 8+ separate `useStore` selectors:

```typescript
const streamingBlocks = useStreamingBlocks(sessionId);
const streamingTextLength = useStreamingTextLength(sessionId);
const pendingCommand = usePendingCommand(sessionId);
const isThinking = useIsAgentThinking(sessionId);
// ... more
```

Each selector subscription can trigger re-renders independently.

#### Solution

Combine related selectors with `useShallow`:

```typescript
import { useShallow } from "zustand/react/shallow";

const {
  streamingBlocks,
  streamingTextLength,
  pendingCommand,
  isThinking,
  thinkingContent,
  activeSubAgents,
  activeWorkflow,
  isCompacting,
} = useStore(
  useShallow((state) => ({
    streamingBlocks: state.streamingBlocks[sessionId] ?? [],
    streamingTextLength: (state.streamingBlocks[sessionId] ?? [])
      .filter(b => b.type === "text")
      .reduce((acc, b) => acc + b.content.length, 0),
    pendingCommand: state.pendingCommand[sessionId],
    isThinking: state.isAgentThinking[sessionId] ?? false,
    thinkingContent: state.thinkingContent[sessionId] ?? "",
    activeSubAgents: state.activeSubAgents[sessionId] ?? [],
    activeWorkflow: state.activeWorkflows[sessionId],
    isCompacting: state.isCompacting[sessionId] ?? false,
  }))
);
```

---

### #20: Add Cancellation Token Pattern

**Status:** ❌ Not Implemented
**Priority:** Low
**Effort:** Medium
**Category:** React Best Practices

#### Problem

When switching sessions mid-stream, events may continue processing for the old session. There's no way to cancel in-flight operations.

#### Solution

Use AbortController:

```typescript
export function useAiEvents() {
  const abortControllerRef = useRef<AbortController>();

  useEffect(() => {
    abortControllerRef.current = new AbortController();
    const signal = abortControllerRef.current.signal;

    const handleEvent = (event: AiEvent) => {
      if (signal.aborted) return;
      // ... handle event
    };

    const unlisten = onAiEvent(handleEvent);

    return () => {
      abortControllerRef.current?.abort();
      unlisten();
    };
  }, []);
}
```

---

### #21: Add Accessibility Features

**Status:** ❌ Not Implemented
**Priority:** Medium
**Effort:** Medium
**Category:** Accessibility

#### Problem

- No ARIA labels on timeline blocks
- No keyboard navigation for tool groups
- No screen reader announcements for streaming content

#### Solution

```typescript
<div
  role="log"
  aria-live="polite"
  aria-label="AI conversation timeline"
>
  {sortedTimeline.map((block) => (
    <article
      role="article"
      aria-label={getBlockAriaLabel(block)}
      tabIndex={0}
    >
      <UnifiedBlock block={block} />
    </article>
  ))}
</div>

// For streaming content
<div aria-live="polite" aria-atomic="false">
  {streamingText}
</div>
```

---

### #22: Add Loading/Skeleton States

**Status:** ❌ Not Implemented
**Priority:** Low
**Effort:** Medium
**Category:** User Experience

#### Problem

No loading UI for:
- Session restoration
- Timeline loading after compaction
- Initial timeline render

#### Solution

```typescript
function TimelineSkeleton({ count = 3 }) {
  return (
    <div className="space-y-4 animate-pulse">
      {Array.from({ length: count }).map((_, i) => (
        <div key={i} className="h-20 bg-muted rounded-lg" />
      ))}
    </div>
  );
}

// Usage
{isLoading ? (
  <TimelineSkeleton count={3} />
) : (
  sortedTimeline.map(...)
)}
```

---

### #23: Optimize Timestamp Parsing

**Status:** ❌ Not Implemented
**Priority:** Low
**Effort:** Low
**Category:** Performance

#### Problem

`new Date(timestamp).getTime()` is called repeatedly in the sort comparator:

```typescript
const ta = new Date(a.block.timestamp).getTime();
const tb = new Date(b.block.timestamp).getTime();
```

#### Solution

Pre-compute timestamps:

```typescript
const sortedTimeline = useMemo(() => {
  const withTimestamps = timeline.map((block, index) => ({
    block,
    index,
    ts: new Date(block.timestamp).getTime(),
  }));

  withTimestamps.sort((a, b) => a.ts - b.ts || a.index - b.index);

  return withTimestamps.map(({ block }) => block);
}, [timeline]);
```

---

### #24: Extract Magic Values to Constants

**Status:** ❌ Not Implemented
**Priority:** Low
**Effort:** Low
**Category:** Code Quality

#### Problem

Hard-coded values scattered throughout:
- `50` for scroll throttle bucket
- `30` for max primary arg length
- `#484f58` border color

#### Solution

```typescript
// lib/constants.ts
export const TIMELINE_CONSTANTS = {
  SCROLL_THROTTLE_CHARS: 50,
  MAX_PRIMARY_ARG_LENGTH: 30,
  ANIMATION_DURATION_MS: 200,
} as const;

// theme constants
export const TIMELINE_COLORS = {
  USER_MESSAGE_BORDER: "var(--user-message-border)",
  // Use CSS variables for colors
} as const;
```

---

### #25: Add Streaming Update Debouncing

**Status:** ❌ Not Implemented
**Priority:** Medium
**Effort:** Medium
**Category:** Performance

#### Problem

`updateAgentStreaming` is called for every `text_delta` event - potentially hundreds per second during fast streaming.

#### Solution

Batch updates:

```typescript
// In useAiEvents
const pendingDeltaRef = useRef("");
const flushTimeoutRef = useRef<number>();

const handleTextDelta = (event: TextDeltaEvent) => {
  pendingDeltaRef.current += event.delta;

  // Debounce: flush every 16ms (one frame)
  if (!flushTimeoutRef.current) {
    flushTimeoutRef.current = requestAnimationFrame(() => {
      state.updateAgentStreaming(sessionId, pendingDeltaRef.current);
      pendingDeltaRef.current = "";
      flushTimeoutRef.current = undefined;
    });
  }
};
```

---

### #26: Standardize Styling Approach

**Status:** ❌ Not Implemented
**Priority:** Low
**Effort:** Low
**Category:** Code Quality

#### Problem

Mix of styling approaches:
- Inline styles: `style={{ color: "#..." }}`
- Tailwind classes: `className="text-red-500"`
- CSS variables: `var(--ansi-red)`
- Hardcoded hex values: `#484f58`

#### Solution

Standardize on Tailwind with CSS variables:

```css
/* In globals.css */
:root {
  --user-message-border: #484f58;
  --ansi-red: /* existing */;
}

.dark {
  --user-message-border: #484f58;
}
```

```typescript
// Replace hardcoded values
className="border-l-[var(--user-message-border)]"
// or define in tailwind.config
className="border-l-user-message"
```

---

## Implementation Roadmap

### Phase 1: Quick Wins (Completed)
- [x] #11: Sub-agent extraction utility
- [x] #12: SystemHooksCard component
- [x] #13: Streaming block finalization
- [x] #14: Memory leak fix
- [x] #15: Error boundaries

### Phase 2: Performance (Recommended Next)
- [ ] #2: O(n²) filter fix
- [ ] #4: Virtualization
- [ ] #25: Streaming debouncing

### Phase 3: Architecture
- [ ] #1: Single source of truth
- [ ] #5: Store slices
- [ ] #8: Event handler registry

### Phase 4: Polish
- [ ] #21: Accessibility
- [ ] #22: Loading states
- [ ] #24: Constants extraction
