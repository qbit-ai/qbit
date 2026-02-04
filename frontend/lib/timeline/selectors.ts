import type { AgentMessage, CommandBlock, UnifiedBlock } from "@/store";

/**
 * Shallow comparison for arrays.
 * Returns true if arrays have the same length and all elements are reference-equal.
 * This is used to determine if a derived array has actually changed.
 */
function arraysShallowEqual<T>(a: T[], b: T[]): boolean {
  if (a.length !== b.length) return false;
  for (let i = 0; i < a.length; i++) {
    if (a[i] !== b[i]) return false;
  }
  return true;
}

/**
 * Derives CommandBlock array from the unified timeline.
 *
 * This is the core selector function that extracts command blocks from the timeline.
 * Used by useSessionBlocks hook to provide command blocks to components.
 *
 * @param timeline - The unified timeline array (or undefined if session doesn't exist)
 * @returns Array of CommandBlock objects
 */
export function selectCommandBlocksFromTimeline(
  timeline: UnifiedBlock[] | undefined
): CommandBlock[] {
  if (!timeline) return [];

  return timeline
    .filter((block): block is UnifiedBlock & { type: "command" } => block.type === "command")
    .map((block) => block.data);
}

/**
 * Derives AgentMessage array from the unified timeline.
 *
 * This is the core selector function that extracts agent messages from the timeline.
 * Used by useAgentMessages hook to provide messages to components.
 *
 * @param timeline - The unified timeline array (or undefined if session doesn't exist)
 * @returns Array of AgentMessage objects
 */
export function selectAgentMessagesFromTimeline(
  timeline: UnifiedBlock[] | undefined
): AgentMessage[] {
  if (!timeline) return [];

  return timeline
    .filter(
      (block): block is UnifiedBlock & { type: "agent_message" } => block.type === "agent_message"
    )
    .map((block) => block.data);
}

/**
 * Cache entry for memoized command blocks selector.
 * Stores both the timeline reference and the extracted result for comparison.
 */
interface CommandBlocksCacheEntry {
  timeline: UnifiedBlock[] | undefined;
  result: CommandBlock[];
}

/**
 * Creates a memoized version of the command blocks selector.
 *
 * This uses a cache that stores the last result for each session.
 * The cache uses shallow comparison on the extracted command blocks,
 * returning the same reference if the content hasn't changed.
 * This prevents unnecessary re-renders even when the timeline array
 * reference changes but the command blocks within it are the same.
 */
export function createMemoizedCommandBlocksSelector() {
  const cache = new Map<string, CommandBlocksCacheEntry>();

  return (sessionId: string, timeline: UnifiedBlock[] | undefined): CommandBlock[] => {
    const cached = cache.get(sessionId);

    // Fast path: if timeline reference is the same, result is definitely the same
    if (cached && cached.timeline === timeline) {
      return cached.result;
    }

    // Compute new result
    const result = selectCommandBlocksFromTimeline(timeline);

    // Check if the extracted items are the same as the cached result
    // This handles the case where timeline reference changed but content didn't
    if (cached && arraysShallowEqual(cached.result, result)) {
      // Update the cached timeline reference but return the same result reference
      cache.set(sessionId, { timeline, result: cached.result });
      return cached.result;
    }

    // Update cache with new result
    cache.set(sessionId, { timeline, result });

    return result;
  };
}

/**
 * Cache entry for memoized agent messages selector.
 * Stores both the timeline reference and the extracted result for comparison.
 */
interface AgentMessagesCacheEntry {
  timeline: UnifiedBlock[] | undefined;
  result: AgentMessage[];
}

/**
 * Creates a memoized version of the agent messages selector.
 *
 * This uses a cache that stores the last result for each session.
 * The cache uses shallow comparison on the extracted agent messages,
 * returning the same reference if the content hasn't changed.
 * This prevents unnecessary re-renders even when the timeline array
 * reference changes but the agent messages within it are the same.
 */
export function createMemoizedAgentMessagesSelector() {
  const cache = new Map<string, AgentMessagesCacheEntry>();

  return (sessionId: string, timeline: UnifiedBlock[] | undefined): AgentMessage[] => {
    const cached = cache.get(sessionId);

    // Fast path: if timeline reference is the same, result is definitely the same
    if (cached && cached.timeline === timeline) {
      return cached.result;
    }

    // Compute new result
    const result = selectAgentMessagesFromTimeline(timeline);

    // Check if the extracted items are the same as the cached result
    // This handles the case where timeline reference changed but content didn't
    if (cached && arraysShallowEqual(cached.result, result)) {
      // Update the cached timeline reference but return the same result reference
      cache.set(sessionId, { timeline, result: cached.result });
      return cached.result;
    }

    // Update cache with new result
    cache.set(sessionId, { timeline, result });

    return result;
  };
}

// Singleton instances of memoized selectors
export const memoizedSelectCommandBlocks = createMemoizedCommandBlocksSelector();
export const memoizedSelectAgentMessages = createMemoizedAgentMessagesSelector();
