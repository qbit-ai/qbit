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
 * 2. Removes matched tool calls from the content blocks
 * 3. Creates sub_agent blocks for rendering SubAgentCard components
 * 4. Falls back to index-based matching for legacy data without parentRequestId
 * 5. Appends any unmatched sub-agents at the end (handles state race conditions)
 *
 * @param groupedBlocks - Grouped streaming blocks (text, tool, tool_group, udiff_result)
 * @param subAgents - Active sub-agents to match against tool calls
 * @returns Object with subAgentBlocks for rendering and contentBlocks with sub-agents removed
 */
export function extractSubAgentBlocks(
  groupedBlocks: GroupedStreamingBlock[],
  subAgents: ActiveSubAgent[]
): ExtractedBlocks {
  const matchedParentIds = new Set<string>();
  const subAgentBlocks: RenderBlock[] = [];
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
          subAgentBlocks.push({ type: "sub_agent", subAgent: matchingSubAgent });
          if (!hasParentRequestIds) subAgentIndex++;
        }
        // Skip adding to content blocks - sub-agent replaces it
        continue;
      }
      // Regular tool - pass through
      contentBlocks.push(block);
    } else if (block.type === "tool_group") {
      // Tool group - filter out sub_agent tools
      const { filteredTools, extractedSubAgents, newSubAgentIndex } = filterToolGroup(
        block.tools,
        subAgents,
        matchedParentIds,
        hasParentRequestIds,
        subAgentIndex
      );

      subAgentIndex = newSubAgentIndex;

      // Add extracted sub-agents
      for (const subAgent of extractedSubAgents) {
        subAgentBlocks.push({ type: "sub_agent", subAgent });
      }

      // Handle remaining tools
      if (filteredTools.length > 0) {
        if (filteredTools.length === 1) {
          // Convert to single tool
          contentBlocks.push({ type: "tool", toolCall: filteredTools[0] });
        } else {
          // Keep as group with remaining tools (preserve toolName if present)
          const newGroup: ToolGroup = {
            type: "tool_group",
            tools: filteredTools,
          };
          if (block.toolName) {
            newGroup.toolName = block.toolName;
          }
          contentBlocks.push(newGroup);
        }
      }
    } else {
      // Text or udiff_result - pass through unchanged
      contentBlocks.push(block);
    }
  }

  // Fallback: Add any remaining sub-agents that weren't matched to tool calls
  // This can happen if activeSubAgents state updates before streamingBlocks
  for (const subAgent of subAgents) {
    if (!matchedParentIds.has(subAgent.parentRequestId)) {
      subAgentBlocks.push({ type: "sub_agent", subAgent });
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
 * Filter sub-agent tools from a tool group.
 */
function filterToolGroup(
  tools: AnyToolCall[],
  subAgents: ActiveSubAgent[],
  matchedParentIds: Set<string>,
  hasParentRequestIds: boolean,
  subAgentIndex: number
): {
  filteredTools: AnyToolCall[];
  extractedSubAgents: ActiveSubAgent[];
  newSubAgentIndex: number;
} {
  const filteredTools: AnyToolCall[] = [];
  const extractedSubAgents: ActiveSubAgent[] = [];
  let currentIndex = subAgentIndex;

  for (const tool of tools) {
    if (tool.name.startsWith("sub_agent_")) {
      const matchingSubAgent = findMatchingSubAgent(
        tool,
        subAgents,
        matchedParentIds,
        hasParentRequestIds,
        currentIndex
      );

      if (matchingSubAgent) {
        matchedParentIds.add(matchingSubAgent.parentRequestId);
        extractedSubAgents.push(matchingSubAgent);
        if (!hasParentRequestIds) currentIndex++;
      }
      // Don't add to filtered tools
    } else {
      filteredTools.push(tool);
    }
  }

  return { filteredTools, extractedSubAgents, newSubAgentIndex: currentIndex };
}
