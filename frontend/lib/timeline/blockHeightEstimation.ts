import type { UnifiedBlock } from "@/store";

// Base heights for each block type (header/collapsed state)
const BASE_HEIGHTS = {
  command: 52, // Header with command text
  agent_message: 80, // Minimal message with border
  system_hook: 44, // Collapsed hook card
} as const;

// Approximate characters per line at typical viewport width
const CHARS_PER_LINE = 80;
const LINE_HEIGHT = 20;

// Maximum heights to prevent extreme estimates
const MAX_HEIGHTS = {
  command: 500,
  agent_message: 800,
  system_hook: 200,
} as const;

/**
 * Estimates the rendered height of a timeline block based on its type and content.
 * Used by the virtualizer for initial sizing before actual measurement.
 */
export function estimateBlockHeight(block: UnifiedBlock): number {
  switch (block.type) {
    case "command": {
      const { output } = block.data;
      // Commands are collapsed by default, but estimate with some output
      const outputLines = output ? Math.ceil(output.length / CHARS_PER_LINE) : 0;
      const estimatedHeight = BASE_HEIGHTS.command + Math.min(outputLines * LINE_HEIGHT, 300);
      return Math.min(estimatedHeight, MAX_HEIGHTS.command);
    }

    case "agent_message": {
      const { content, toolCalls, thinkingContent, streamingHistory } = block.data;
      let height = BASE_HEIGHTS.agent_message;

      // Response text
      if (content) {
        const contentLines = Math.ceil(content.length / CHARS_PER_LINE);
        height += contentLines * LINE_HEIGHT;
      }

      // Streaming history blocks
      if (streamingHistory?.length) {
        for (const historyBlock of streamingHistory) {
          if (historyBlock.type === "text") {
            const textLines = Math.ceil(historyBlock.content.length / CHARS_PER_LINE);
            height += textLines * LINE_HEIGHT;
          } else if (historyBlock.type === "tool") {
            // Tool calls are collapsed, ~44px each
            height += 44;
          }
        }
      }

      // Legacy tool calls (collapsed: ~44px each)
      if (toolCalls?.length) {
        height += toolCalls.length * 44;
      }

      // Thinking content (collapsed: ~32px)
      if (thinkingContent) {
        height += 32;
      }

      return Math.min(height, MAX_HEIGHTS.agent_message);
    }

    case "system_hook": {
      const { hooks } = block.data;
      // Base height plus some for expanded state
      const expandedHeight = hooks.length * 24;
      return Math.min(BASE_HEIGHTS.system_hook + expandedHeight, MAX_HEIGHTS.system_hook);
    }

    default:
      return 100; // Fallback for unknown types
  }
}
