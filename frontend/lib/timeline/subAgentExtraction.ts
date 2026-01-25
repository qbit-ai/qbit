import type { AnyToolCall, GroupedStreamingBlock, ToolGroup } from "@/lib/toolGrouping";
import type { ActiveSubAgent } from "@/store";

/** Block type for rendering - includes sub-agent blocks */
export type RenderBlock = GroupedStreamingBlock | { type: "sub_agent"; subAgent: ActiveSubAgent };

export interface ExtractedBlocks {
  /** Sub-agent blocks extracted from the input */
  subAgentBlocks: RenderBlock[];
  /** Content blocks with sub-agent tool calls removed */
  contentBlocks: RenderBlock[];
}

/**
 * Extracts sub-agent tool calls from grouped blocks and replaces them with SubAgentCard blocks.
 *
 * This function:
 * 1. Matches sub_agent_* tool calls with their corresponding ActiveSubAgent by parentRequestId
 * 2. Replaces sub_agent tool calls with sub_agent blocks INLINE (preserving order)
 * 3. Falls back to index-based matching for legacy data without parentRequestId
 * 4. Appends any unmatched sub-agents at the end (handles state race conditions)
 *
 * Note: subAgentBlocks is kept for backward compatibility but will be empty for new code.
 * Sub-agents are now inlined in contentBlocks at their correct position.
 *
 * @param groupedBlocks - Grouped streaming blocks (text, tool, tool_group, udiff_result)
 * @param subAgents - Active sub-agents to match against tool calls
 * @returns Object with subAgentBlocks (empty) and contentBlocks with sub-agents inlined
 */
export function extractSubAgentBlocks(
  groupedBlocks: GroupedStreamingBlock[],
  subAgents: ActiveSubAgent[]
): ExtractedBlocks {
  const matchedParentIds = new Set<string>();
  const subAgentBlocks: RenderBlock[] = []; // Kept empty for backward compatibility
  const contentBlocks: RenderBlock[] = [];

  // Check if we have parentRequestId for ID-based matching (newer data)
  const hasParentRequestIds = subAgents.length > 0 && !!subAgents[0].parentRequestId;

  let subAgentIndex = 0; // Fallback for legacy data

  for (const block of groupedBlocks) {
    if (block.type === "tool") {
      // Single tool - check if it's a sub-agent spawn
      if (block.toolCall.name.startsWith("sub_agent_")) {
        const matchingSubAgent = findMatchingSubAgent(
          block.toolCall,
          subAgents,
          matchedParentIds,
          hasParentRequestIds,
          subAgentIndex
        );

        if (matchingSubAgent) {
          matchedParentIds.add(matchingSubAgent.parentRequestId);
          // Add sub-agent inline at this position (not to separate array)
          contentBlocks.push({ type: "sub_agent", subAgent: matchingSubAgent });
          if (!hasParentRequestIds) subAgentIndex++;
        }
        // Skip the tool call itself - sub-agent replaces it
        continue;
      }
      // Regular tool - pass through
      contentBlocks.push(block);
    } else if (block.type === "tool_group") {
      // Tool group - process tools in order, replacing sub_agent tools inline
      const { processedBlocks, newSubAgentIndex } = processToolGroupInline(
        block,
        subAgents,
        matchedParentIds,
        hasParentRequestIds,
        subAgentIndex
      );

      subAgentIndex = newSubAgentIndex;
      contentBlocks.push(...processedBlocks);
    } else {
      // Text, udiff_result, system_hooks - pass through unchanged
      contentBlocks.push(block);
    }
  }

  // Fallback: Add any remaining sub-agents that weren't matched to tool calls
  // This can happen if activeSubAgents state updates before streamingBlocks
  for (const subAgent of subAgents) {
    if (!matchedParentIds.has(subAgent.parentRequestId)) {
      contentBlocks.push({ type: "sub_agent", subAgent });
    }
  }

  return { subAgentBlocks, contentBlocks };
}

/**
 * Find a matching sub-agent for a tool call.
 */
function findMatchingSubAgent(
  toolCall: AnyToolCall,
  subAgents: ActiveSubAgent[],
  matchedParentIds: Set<string>,
  hasParentRequestIds: boolean,
  subAgentIndex: number
): ActiveSubAgent | undefined {
  if (hasParentRequestIds) {
    // Match by tool call ID (which equals parentRequestId)
    return subAgents.find(
      (a) => a.parentRequestId === toolCall.id && !matchedParentIds.has(a.parentRequestId)
    );
  }
  // Fallback to index-based matching for legacy data
  if (subAgentIndex < subAgents.length) {
    return subAgents[subAgentIndex];
  }
  return undefined;
}

/**
 * Process a tool group, inlining sub-agents at their correct position.
 * This preserves the original order: if tool A comes before sub_agent B,
 * they appear in that order in the result.
 */
function processToolGroupInline(
  block: ToolGroup,
  subAgents: ActiveSubAgent[],
  matchedParentIds: Set<string>,
  hasParentRequestIds: boolean,
  subAgentIndex: number
): {
  processedBlocks: RenderBlock[];
  newSubAgentIndex: number;
} {
  const processedBlocks: RenderBlock[] = [];
  const regularTools: AnyToolCall[] = [];
  let currentIndex = subAgentIndex;

  for (const tool of block.tools) {
    if (tool.name.startsWith("sub_agent_")) {
      // First, flush any accumulated regular tools as a group/single tool
      if (regularTools.length > 0) {
        if (regularTools.length === 1) {
          processedBlocks.push({ type: "tool", toolCall: regularTools[0] });
        } else {
          const newGroup: ToolGroup = { type: "tool_group", tools: [...regularTools] };
          if (block.toolName) newGroup.toolName = block.toolName;
          processedBlocks.push(newGroup);
        }
        regularTools.length = 0;
      }

      // Then add the sub-agent at this position
      const matchingSubAgent = findMatchingSubAgent(
        tool,
        subAgents,
        matchedParentIds,
        hasParentRequestIds,
        currentIndex
      );

      if (matchingSubAgent) {
        matchedParentIds.add(matchingSubAgent.parentRequestId);
        processedBlocks.push({ type: "sub_agent", subAgent: matchingSubAgent });
        if (!hasParentRequestIds) currentIndex++;
      }
    } else {
      regularTools.push(tool);
    }
  }

  // Flush any remaining regular tools
  if (regularTools.length > 0) {
    if (regularTools.length === 1) {
      processedBlocks.push({ type: "tool", toolCall: regularTools[0] });
    } else {
      const newGroup: ToolGroup = { type: "tool_group", tools: [...regularTools] };
      if (block.toolName) newGroup.toolName = block.toolName;
      processedBlocks.push(newGroup);
    }
  }

  return { processedBlocks, newSubAgentIndex: currentIndex };
}
