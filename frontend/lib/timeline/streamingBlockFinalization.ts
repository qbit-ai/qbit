import type { FinalizedStreamingBlock, StreamingBlock, ToolCall } from "@/store";

/**
 * Converts streaming blocks to finalized blocks for persistence.
 *
 * This function:
 * 1. Converts ActiveToolCall to ToolCall format (removes startedAt/completedAt)
 * 2. Maps running tools to "completed" status (finalization happens at turn end)
 * 3. Preserves the interleaved order of text, tool, and udiff_result blocks
 *
 * @param blocks - Streaming blocks from the active response
 * @returns Finalized blocks suitable for storage in AgentMessage.streamingHistory
 */
export function finalizeStreamingBlocks(blocks: StreamingBlock[]): FinalizedStreamingBlock[] {
  return blocks.map((block) => {
    if (block.type === "text") {
      return { type: "text" as const, content: block.content };
    }

    if (block.type === "udiff_result") {
      return {
        type: "udiff_result" as const,
        response: block.response,
        durationMs: block.durationMs,
      };
    }

    if (block.type === "system_hooks") {
      return {
        type: "system_hooks" as const,
        hooks: block.hooks,
      };
    }

    // Tool block - convert ActiveToolCall to ToolCall format
    const toolCall = block.toolCall;
    const status: ToolCall["status"] =
      toolCall.status === "completed"
        ? "completed"
        : toolCall.status === "error"
          ? "error"
          : "completed"; // Running tools are finalized as completed

    return {
      type: "tool" as const,
      toolCall: {
        id: toolCall.id,
        name: toolCall.name,
        args: toolCall.args,
        status,
        result: toolCall.result,
        executedByAgent: toolCall.executedByAgent,
      },
    };
  });
}

/**
 * Extracts tool calls from finalized streaming blocks.
 *
 * This is a convenience helper for backwards compatibility,
 * extracting the toolCalls array from streamingHistory.
 *
 * @param blocks - Finalized streaming blocks
 * @returns Array of ToolCall objects
 */
export function extractToolCalls(blocks: FinalizedStreamingBlock[]): ToolCall[] {
  return blocks
    .filter((b): b is FinalizedStreamingBlock & { type: "tool" } => b.type === "tool")
    .map((b) => b.toolCall);
}
