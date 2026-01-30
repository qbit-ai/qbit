/**
 * Tool-related AI event handlers.
 *
 * Handles tool request, approval, auto-approval, and result events.
 */

import type { ApprovalPattern, RiskLevel, ToolSource } from "@/lib/ai";
import { respondToToolApproval } from "@/lib/ai";
import { logger } from "@/lib/logger";
import type { EventHandler } from "./types";

/**
 * Handle tool request event.
 * Adds tool call to active calls and streaming blocks.
 */
export const handleToolRequest: EventHandler<{
  type: "tool_request";
  tool_name: string;
  args: unknown;
  request_id: string;
  source?: ToolSource;
  session_id: string;
  seq?: number;
}> = (event, ctx) => {
  const state = ctx.getState();

  // Deduplicate: ignore already-processed requests
  if (state.isToolRequestProcessed(event.request_id)) {
    logger.debug("Ignoring duplicate tool_request:", event.request_id);
    return;
  }

  // Mark as processed immediately to prevent duplicates
  state.markToolRequestProcessed(event.request_id);

  state.setAgentThinking(ctx.sessionId, false);
  // Flush pending text deltas to ensure correct ordering
  ctx.flushSessionDeltas(ctx.sessionId);

  const toolCall = {
    id: event.request_id,
    name: event.tool_name,
    args: event.args as Record<string, unknown>,
    // All tool calls from AI events are executed by the agent
    executedByAgent: true,
    source: ctx.convertToolSource(event.source),
  };

  // Track the tool call as running (for UI display)
  state.addActiveToolCall(ctx.sessionId, toolCall);
  // Also add to streaming blocks for interleaved display
  state.addStreamingToolBlock(ctx.sessionId, toolCall);
};

/**
 * Handle tool approval request event.
 * Enhanced tool request with HITL metadata requiring user approval.
 */
export const handleToolApprovalRequest: EventHandler<{
  type: "tool_approval_request";
  request_id: string;
  tool_name: string;
  args: unknown;
  stats: ApprovalPattern | null;
  risk_level: RiskLevel;
  can_learn: boolean;
  suggestion: string | null;
  source?: ToolSource;
  session_id: string;
  seq?: number;
}> = (event, ctx) => {
  const state = ctx.getState();

  // Deduplicate: ignore already-processed requests
  if (state.isToolRequestProcessed(event.request_id)) {
    logger.debug("Ignoring duplicate tool_approval_request:", event.request_id);
    return;
  }

  // Mark as processed immediately to prevent duplicates
  state.markToolRequestProcessed(event.request_id);

  state.setAgentThinking(ctx.sessionId, false);
  // Flush pending text deltas to ensure correct ordering
  ctx.flushSessionDeltas(ctx.sessionId);

  const toolCall = {
    id: event.request_id,
    name: event.tool_name,
    args: event.args as Record<string, unknown>,
    executedByAgent: true,
    riskLevel: event.risk_level,
    stats: event.stats ?? undefined,
    suggestion: event.suggestion ?? undefined,
    canLearn: event.can_learn,
    source: ctx.convertToolSource(event.source),
  };

  // Track the tool call
  state.addActiveToolCall(ctx.sessionId, toolCall);
  state.addStreamingToolBlock(ctx.sessionId, toolCall);

  // Check if auto-approve mode is enabled for this session
  // This acts as a frontend safeguard in case the backend sent an approval request
  // before the agent mode was fully synchronized
  const session = state.sessions[ctx.sessionId];
  if (session?.agentMode === "auto-approve") {
    respondToToolApproval(ctx.sessionId, {
      request_id: event.request_id,
      approved: true,
      remember: false,
      always_allow: false,
    }).catch((err) => {
      logger.error("Failed to auto-approve tool:", err);
    });
    return;
  }

  // Set pending tool approval for the dialog
  state.setPendingToolApproval(ctx.sessionId, {
    ...toolCall,
    status: "pending",
  });
};

/**
 * Handle tool auto-approved event.
 * Tool was automatically approved based on learned patterns.
 */
export const handleToolAutoApproved: EventHandler<{
  type: "tool_auto_approved";
  request_id: string;
  tool_name: string;
  args: unknown;
  reason: string;
  source?: ToolSource;
  session_id: string;
  seq?: number;
}> = (event, ctx) => {
  const state = ctx.getState();

  // Deduplicate: ignore already-processed requests
  if (state.isToolRequestProcessed(event.request_id)) {
    logger.debug("Ignoring duplicate tool_auto_approved:", event.request_id);
    return;
  }

  // Mark as processed immediately to prevent duplicates
  state.markToolRequestProcessed(event.request_id);

  logger.info("tool_auto_approved: Adding tool block", {
    request_id: event.request_id,
    tool_name: event.tool_name,
  });

  state.setAgentThinking(ctx.sessionId, false);
  // Flush pending text deltas to ensure correct ordering
  ctx.flushSessionDeltas(ctx.sessionId);

  const autoApprovedTool = {
    id: event.request_id,
    name: event.tool_name,
    args: event.args as Record<string, unknown>,
    executedByAgent: true,
    autoApproved: true,
    autoApprovalReason: event.reason,
    source: ctx.convertToolSource(event.source),
  };

  state.addActiveToolCall(ctx.sessionId, autoApprovedTool);
  state.addStreamingToolBlock(ctx.sessionId, autoApprovedTool);
};

/**
 * Handle tool result event.
 * Updates tool call status to completed/error.
 */
export const handleToolResult: EventHandler<{
  type: "tool_result";
  tool_name: string;
  result: unknown;
  success: boolean;
  request_id: string;
  source?: ToolSource;
  session_id: string;
  seq?: number;
}> = (event, ctx) => {
  const state = ctx.getState();
  // Update tool call status to completed/error
  state.completeActiveToolCall(ctx.sessionId, event.request_id, event.success, event.result);
  // Also update streaming block
  state.updateStreamingToolBlock(ctx.sessionId, event.request_id, event.success, event.result);
};

/**
 * Handle tool output chunk event.
 * Appends streaming output to a running tool call (for run_command).
 */
export const handleToolOutputChunk: EventHandler<{
  type: "tool_output_chunk";
  request_id: string;
  tool_name: string;
  chunk: string;
  stream: string;
  source?: ToolSource;
  session_id: string;
  seq?: number;
}> = (event, ctx) => {
  const state = ctx.getState();

  // Debug: Log what blocks exist and which one we're trying to match
  const blocks = state.streamingBlocks[ctx.sessionId] ?? [];
  const toolBlocks = blocks.filter((b) => b.type === "tool");
  const matchingBlock = toolBlocks.find(
    (b) => b.type === "tool" && b.toolCall.id === event.request_id
  );

  if (!matchingBlock) {
    logger.warn("tool_output_chunk: No matching block found for request_id:", event.request_id, {
      availableToolIds: toolBlocks.map((b) => (b as { toolCall: { id: string } }).toolCall.id),
    });
  } else {
    logger.debug("tool_output_chunk: Found matching block for", event.request_id);
  }

  // Append the chunk to the tool's streaming output
  state.appendToolStreamingOutput(ctx.sessionId, event.request_id, event.chunk);
};
