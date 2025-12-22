/**
 * Utilities for extracting copyable text from agent messages.
 */

import type { AgentMessage, FinalizedStreamingBlock, ToolCall } from "@/store";
import { formatToolName, formatToolResult } from "./tools";

/**
 * Format a tool call for plain text output.
 */
function formatToolCallText(tool: ToolCall): string {
  const parts: string[] = [];

  // Tool name and primary argument
  const name = formatToolName(tool.name);
  parts.push(`[${name}]`);

  // Add key arguments based on tool type
  if (tool.args) {
    const args = tool.args;
    if (args.file_path || args.path) {
      parts.push(`Path: ${String(args.file_path || args.path)}`);
    }
    if (args.command) {
      parts.push(`Command: ${String(args.command)}`);
    }
    if (args.query) {
      parts.push(`Query: ${String(args.query)}`);
    }
  }

  // Add result if available
  if (tool.result !== undefined) {
    const resultText = formatToolResult(tool.result);
    // Truncate very long results
    const maxLength = 500;
    if (resultText.length > maxLength) {
      parts.push(`Result: ${resultText.slice(0, maxLength)}...`);
    } else {
      parts.push(`Result: ${resultText}`);
    }
  }

  return parts.join("\n");
}

/**
 * Extract plain text content from a streaming block.
 */
function extractBlockText(block: FinalizedStreamingBlock): string {
  switch (block.type) {
    case "text":
      return block.content;
    case "tool":
      return formatToolCallText(block.toolCall);
    case "udiff_result":
      return `[Diff]\n${block.response}`;
    default:
      return "";
  }
}

/**
 * Extract all text content from an agent message for copying.
 * Includes thinking content, text blocks, and formatted tool calls.
 */
export function extractMessageText(message: AgentMessage): string {
  const parts: string[] = [];

  // Include thinking content if present
  if (message.thinkingContent) {
    parts.push(`[Thinking]\n${message.thinkingContent}`);
  }

  // Include workflow summary if present
  if (message.workflow) {
    const steps = message.workflow.steps
      .map((step, i) => `${i + 1}. ${step.name}: ${step.status}`)
      .join("\n");
    parts.push(`[Workflow: ${message.workflow.workflowName}]\n${steps}`);
  }

  // Use streaming history if available (interleaved text + tool calls)
  if (message.streamingHistory && message.streamingHistory.length > 0) {
    for (const block of message.streamingHistory) {
      const text = extractBlockText(block);
      if (text) {
        parts.push(text);
      }
    }
  } else {
    // Fallback to legacy content
    if (message.content) {
      parts.push(message.content);
    }

    // Legacy tool calls
    if (message.toolCalls && message.toolCalls.length > 0) {
      for (const tool of message.toolCalls) {
        parts.push(formatToolCallText(tool));
      }
    }
  }

  return parts.join("\n\n");
}
