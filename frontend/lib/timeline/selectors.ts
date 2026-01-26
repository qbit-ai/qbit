import type { AgentMessage, CommandBlock, UnifiedBlock } from "@/store";

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
 * Creates a memoized version of the command blocks selector.
 *
 * This uses a simple cache that stores the last result for each session.
 * The cache is invalidated when the timeline reference changes.
 */
export function createMemoizedCommandBlocksSelector() {
  const cache = new Map<string, { timeline: UnifiedBlock[] | undefined; result: CommandBlock[] }>();

  return (sessionId: string, timeline: UnifiedBlock[] | undefined): CommandBlock[] => {
    const cached = cache.get(sessionId);

    // Return cached result if timeline reference hasn't changed
    if (cached && cached.timeline === timeline) {
      return cached.result;
    }

    // Compute new result
    const result = selectCommandBlocksFromTimeline(timeline);

    // Update cache
    cache.set(sessionId, { timeline, result });

    return result;
  };
}

/**
 * Creates a memoized version of the agent messages selector.
 *
 * This uses a simple cache that stores the last result for each session.
 * The cache is invalidated when the timeline reference changes.
 */
export function createMemoizedAgentMessagesSelector() {
  const cache = new Map<string, { timeline: UnifiedBlock[] | undefined; result: AgentMessage[] }>();

  return (sessionId: string, timeline: UnifiedBlock[] | undefined): AgentMessage[] => {
    const cached = cache.get(sessionId);

    // Return cached result if timeline reference hasn't changed
    if (cached && cached.timeline === timeline) {
      return cached.result;
    }

    // Compute new result
    const result = selectAgentMessagesFromTimeline(timeline);

    // Update cache
    cache.set(sessionId, { timeline, result });

    return result;
  };
}

// Singleton instances of memoized selectors
export const memoizedSelectCommandBlocks = createMemoizedCommandBlocksSelector();
export const memoizedSelectAgentMessages = createMemoizedAgentMessagesSelector();
