/**
 * Context management AI event handlers.
 *
 * Handles context window events: context_warning, compaction_started,
 * compaction_completed, compaction_failed, tool_response_truncated.
 */

import { logger } from "@/lib/logger";
import type { EventHandler } from "./types";

/**
 * Handle context warning event.
 * Updates context metrics when utilization is high.
 */
export const handleContextWarning: EventHandler<{
  type: "context_warning";
  utilization: number;
  total_tokens: number;
  max_tokens: number;
  session_id: string;
  seq?: number;
}> = (event, ctx) => {
  ctx.getState().setContextMetrics(ctx.sessionId, {
    utilization: event.utilization,
    usedTokens: event.total_tokens,
    maxTokens: event.max_tokens,
    isWarning: true,
  });
};

/**
 * Handle compaction started event.
 * Sets compacting state to show UI indicator.
 */
export const handleCompactionStarted: EventHandler<{
  type: "compaction_started";
  tokens_before: number;
  messages_before: number;
  session_id: string;
  seq?: number;
}> = (event, ctx) => {
  ctx.getState().setCompacting(ctx.sessionId, true);
  logger.info(
    `[Compaction] Started: ${event.tokens_before.toLocaleString()} tokens, ${event.messages_before} messages`
  );
};

/**
 * Handle compaction completed event.
 * Clears timeline and adds compaction result message.
 */
export const handleCompactionCompleted: EventHandler<{
  type: "compaction_completed";
  tokens_before: number;
  messages_before: number;
  messages_after: number;
  summary_length: number;
  summary?: string;
  summarizer_input?: string;
  session_id: string;
  seq?: number;
}> = (event, ctx) => {
  const state = ctx.getState();

  state.handleCompactionSuccess(ctx.sessionId);
  state.setContextMetrics(ctx.sessionId, {
    utilization: 0,
    usedTokens: 0,
    isWarning: false,
  });

  // Clear the timeline - compaction summarizes old content, so start fresh
  state.clearTimeline(ctx.sessionId);
  state.clearThinkingContent(ctx.sessionId);

  // Add only the compaction result message
  state.addAgentMessage(ctx.sessionId, {
    id: crypto.randomUUID(),
    sessionId: ctx.sessionId,
    role: "system",
    content: "",
    timestamp: new Date().toISOString(),
    compaction: {
      status: "success",
      tokensBefore: event.tokens_before,
      messagesBefore: event.messages_before,
      messagesAfter: event.messages_after,
      summaryLength: event.summary_length,
      summary: event.summary,
      summarizerInput: event.summarizer_input,
    },
  });

  logger.info(
    `[Compaction] Completed: ${event.messages_before} → ${event.messages_after} messages, summary: ${event.summary_length} chars`
  );
};

/**
 * Handle compaction failed event.
 * Finalizes any streaming content and adds failure message.
 */
export const handleCompactionFailed: EventHandler<{
  type: "compaction_failed";
  tokens_before: number;
  messages_before: number;
  error: string;
  summarizer_input?: string;
  session_id: string;
  seq?: number;
}> = (event, ctx) => {
  const state = ctx.getState();

  state.handleCompactionFailed(ctx.sessionId, event.error);

  // Finalize any streaming content that occurred BEFORE compaction failed
  const preFailBlocks = state.streamingBlocks[ctx.sessionId] || [];
  const preFailStreaming = state.agentStreaming[ctx.sessionId] || "";
  const preFailThinking = state.thinkingContent[ctx.sessionId] || "";

  if (preFailBlocks.length > 0 || preFailStreaming || preFailThinking) {
    // Convert pre-compaction streaming to a finalized message
    const streamingHistory: import("@/store").FinalizedStreamingBlock[] = preFailBlocks
      .map((block): import("@/store").FinalizedStreamingBlock | null => {
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
        if (block.type === "tool") {
          return {
            type: "tool" as const,
            toolCall: {
              id: block.toolCall.id,
              name: block.toolCall.name,
              args: block.toolCall.args,
              status:
                block.toolCall.status === "completed"
                  ? ("completed" as const)
                  : block.toolCall.status === "error"
                    ? ("error" as const)
                    : ("completed" as const),
              result: block.toolCall.result,
              executedByAgent: block.toolCall.executedByAgent,
            },
          };
        }
        return null;
      })
      .filter((block): block is import("@/store").FinalizedStreamingBlock => block !== null);

    const toolCalls = streamingHistory
      .filter((b): b is { type: "tool"; toolCall: import("@/store").ToolCall } => b.type === "tool")
      .map((b) => b.toolCall);

    const content = preFailStreaming || "";

    if (content || streamingHistory.length > 0) {
      state.addAgentMessage(ctx.sessionId, {
        id: crypto.randomUUID(),
        sessionId: ctx.sessionId,
        role: "assistant",
        content: content,
        timestamp: new Date().toISOString(),
        toolCalls: toolCalls.length > 0 ? toolCalls : undefined,
        streamingHistory: streamingHistory.length > 0 ? streamingHistory : undefined,
        thinkingContent: preFailThinking || undefined,
      });
    }

    // Clear streaming state for post-failure content
    state.clearAgentStreaming(ctx.sessionId);
    state.clearStreamingBlocks(ctx.sessionId);
    state.clearThinkingContent(ctx.sessionId);
  }

  // Add the compaction failure message immediately
  state.addAgentMessage(ctx.sessionId, {
    id: crypto.randomUUID(),
    sessionId: ctx.sessionId,
    role: "system",
    content: "",
    timestamp: new Date().toISOString(),
    compaction: {
      status: "failed",
      tokensBefore: event.tokens_before,
      messagesBefore: event.messages_before,
      error: event.error,
      summarizerInput: event.summarizer_input,
    },
  });

  logger.warn(`[Compaction] Failed: ${event.error}`);
};

/**
 * Handle tool response truncated event.
 * Logs truncation for debugging.
 */
export const handleToolResponseTruncated: EventHandler<{
  type: "tool_response_truncated";
  tool_name: string;
  original_tokens: number;
  truncated_tokens: number;
  session_id: string;
  seq?: number;
}> = (event, _ctx) => {
  logger.debug(
    `[Context] Tool response truncated: ${event.tool_name} (${event.original_tokens} → ${event.truncated_tokens} tokens)`
  );
};
