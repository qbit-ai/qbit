/**
 * Core AI event handlers.
 *
 * Handles the fundamental agent lifecycle events:
 * started, text_delta, reasoning, completed, error, system_hooks_injected
 */

import { addPromptHistory } from "@/lib/history";
import { logger } from "@/lib/logger";
import type { EventHandler } from "./types";

const lastPersistedUserMessageId = new Map<string, string>();

function getLatestUserMessageForSession(
  state: ReturnType<typeof import("@/store").useStore.getState>,
  sessionId: string
) {
  const timeline = state.timelines[sessionId] || [];
  for (let i = timeline.length - 1; i >= 0; i--) {
    const block = timeline[i];
    if (block.type === "agent_message" && block.data.role === "user") {
      return block.data;
    }
  }
  return null;
}

/**
 * Handle agent turn started event.
 * Clears streaming state and sets thinking/responding flags.
 */
export const handleStarted: EventHandler<{
  type: "started";
  turn_id: string;
  session_id: string;
  seq?: number;
}> = (event, ctx) => {
  logger.info("AI turn started:", { sessionId: ctx.sessionId, turnId: event.turn_id });
  const state = ctx.getState();
  state.clearAgentStreaming(ctx.sessionId);
  state.clearActiveToolCalls(ctx.sessionId);
  state.clearThinkingContent(ctx.sessionId);
  state.setAgentThinking(ctx.sessionId, true);
  state.setAgentResponding(ctx.sessionId, true);
};

/**
 * Handle system hooks injected event.
 * Adds hooks to streaming blocks and timeline.
 */
export const handleSystemHooksInjected: EventHandler<{
  type: "system_hooks_injected";
  hooks: string[];
  session_id: string;
  seq?: number;
}> = (event, ctx) => {
  const state = ctx.getState();
  state.setAgentThinking(ctx.sessionId, false);
  // Flush pending text deltas to ensure correct ordering
  ctx.flushSessionDeltas(ctx.sessionId);
  // Add to streaming blocks for correct inline positioning within the message
  state.addStreamingSystemHooksBlock(ctx.sessionId, event.hooks);
  // Also render as a dedicated timeline entry (not a chat message)
  state.addSystemHookBlock(ctx.sessionId, event.hooks);
};

/**
 * Handle text delta event.
 * Batches text deltas for throttled updates (~60fps).
 */
export const handleTextDelta: EventHandler<{
  type: "text_delta";
  delta: string;
  accumulated: string;
  session_id: string;
  seq?: number;
}> = (event, ctx) => {
  ctx.getState().setAgentThinking(ctx.sessionId, false);
  // Batch deltas and flush at ~60fps
  ctx.batchTextDelta(ctx.sessionId, event.delta);
};

/**
 * Handle reasoning/thinking event.
 * Appends thinking content for extended thinking models.
 */
export const handleReasoning: EventHandler<{
  type: "reasoning";
  content: string;
  session_id: string;
  seq?: number;
}> = (event, ctx) => {
  ctx.getState().appendThinkingContent(ctx.sessionId, event.content);
};

/**
 * Handle agent turn completed event.
 * Finalizes streaming content into a persisted message.
 */
export const handleCompleted: EventHandler<{
  type: "completed";
  response: string;
  reasoning?: string;
  input_tokens?: number;
  output_tokens?: number;
  duration_ms?: number;
  session_id: string;
  seq?: number;
}> = (event, ctx) => {
  logger.info("AI turn completed:", {
    sessionId: ctx.sessionId,
    inputTokens: event.input_tokens,
    outputTokens: event.output_tokens,
    durationMs: event.duration_ms,
  });

  // Flush any pending text deltas before finalizing the message
  ctx.flushSessionDeltas(ctx.sessionId);

  // Re-read state after flush to get the updated streaming content
  const state = ctx.getState();

  // Persist the user's prompt (best-effort). We record it here (on completion)
  // so we can also attach model/provider/tokens.
  const latestUserMsg = getLatestUserMessageForSession(state, ctx.sessionId);
  if (latestUserMsg) {
    const lastId = lastPersistedUserMessageId.get(ctx.sessionId);
    if (lastId !== latestUserMsg.id) {
      const provider = state.sessions[ctx.sessionId]?.aiConfig?.provider;
      const model = state.sessions[ctx.sessionId]?.aiConfig?.model;
      if (provider && model) {
        addPromptHistory(
          ctx.sessionId,
          latestUserMsg.content,
          model,
          provider,
          event.input_tokens ?? 0,
          event.output_tokens ?? 0,
          true
        ).catch((err) => {
          logger.debug("Failed to save prompt history:", err);
        });
      }
      lastPersistedUserMessageId.set(ctx.sessionId, latestUserMsg.id);
    }
  }
  const blocks = state.streamingBlocks[ctx.sessionId] || [];
  const streaming = state.agentStreaming[ctx.sessionId] || "";
  const thinkingContent = state.thinkingContent[ctx.sessionId] || "";
  const activeWorkflow = state.activeWorkflows[ctx.sessionId];
  const activeSubAgents = state.activeSubAgents[ctx.sessionId] || [];

  // Filter out workflow tool calls - they're displayed in WorkflowTree instead
  const filteredBlocks = activeWorkflow
    ? blocks.filter((block) => {
        if (block.type !== "tool") return true;
        const source = block.toolCall.source;
        // Hide run_workflow tool and workflow-sourced tool calls
        if (block.toolCall.name === "run_workflow") return false;
        return !(source?.type === "workflow" && source.workflowId === activeWorkflow.workflowId);
      })
    : blocks;

  // Preserve the interleaved streaming history (text + tool calls in order)
  const streamingHistory: import("@/store").FinalizedStreamingBlock[] = filteredBlocks
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
        // Convert ActiveToolCall to ToolCall format
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

  // Extract tool calls for backwards compatibility
  const toolCalls = streamingHistory
    .filter((b): b is { type: "tool"; toolCall: import("@/store").ToolCall } => b.type === "tool")
    .map((b) => b.toolCall);

  // Use full accumulated text as content (fallback to event.response for edge cases)
  const content = streaming || event.response || "";

  // Preserve workflow tool calls before creating the message
  state.preserveWorkflowToolCalls(ctx.sessionId);

  // Create a deep copy of the workflow (with tool calls) for the message
  const workflowForMessage = activeWorkflow
    ? {
        ...activeWorkflow,
        toolCalls: [...(state.activeWorkflows[ctx.sessionId]?.toolCalls || [])],
      }
    : undefined;

  // Extract system hooks from the timeline that were injected during this turn
  // These are system_hook blocks that don't have a subsequent agent_message yet
  const timeline = state.timelines[ctx.sessionId] || [];
  const systemHooks: string[] = [];
  for (let i = timeline.length - 1; i >= 0; i--) {
    const block = timeline[i];
    if (block.type === "system_hook") {
      systemHooks.push(...(block.data.hooks as string[]));
    } else if (block.type === "agent_message") {
      // Stop at the previous agent message
      break;
    }
  }

  if (content || streamingHistory.length > 0 || workflowForMessage || activeSubAgents.length > 0) {
    state.addAgentMessage(ctx.sessionId, {
      id: crypto.randomUUID(),
      sessionId: ctx.sessionId,
      role: "assistant",
      content: content,
      timestamp: new Date().toISOString(),
      toolCalls: toolCalls.length > 0 ? toolCalls : undefined,
      streamingHistory: streamingHistory.length > 0 ? streamingHistory : undefined,
      thinkingContent: thinkingContent || undefined,
      workflow: workflowForMessage,
      subAgents: activeSubAgents.length > 0 ? [...activeSubAgents] : undefined,
      systemHooks: systemHooks.length > 0 ? systemHooks : undefined,
      inputTokens: event.input_tokens,
      outputTokens: event.output_tokens,
    });
  }

  state.clearAgentStreaming(ctx.sessionId);
  state.clearStreamingBlocks(ctx.sessionId);
  state.clearThinkingContent(ctx.sessionId);
  state.clearActiveToolCalls(ctx.sessionId);
  // Clear the active workflow since it's now stored in the message
  state.clearActiveWorkflow(ctx.sessionId);
  // Clear active sub-agents since they're now stored in the message
  state.clearActiveSubAgents(ctx.sessionId);
  state.setAgentThinking(ctx.sessionId, false);
  state.setAgentResponding(ctx.sessionId, false);
};

/**
 * Handle agent error event.
 * Adds error message and clears streaming state.
 */
export const handleError: EventHandler<{
  type: "error";
  message: string;
  error_type: string;
  session_id: string;
  seq?: number;
}> = (event, ctx) => {
  logger.error("AI turn error:", {
    sessionId: ctx.sessionId,
    errorType: event.error_type,
    message: event.message,
  });
  const state = ctx.getState();

  // Persist the user's prompt (best-effort) as a failed prompt.
  const latestUserMsg = getLatestUserMessageForSession(state, ctx.sessionId);
  if (latestUserMsg) {
    const lastId = lastPersistedUserMessageId.get(ctx.sessionId);
    if (lastId !== latestUserMsg.id) {
      const provider = state.sessions[ctx.sessionId]?.aiConfig?.provider;
      const model = state.sessions[ctx.sessionId]?.aiConfig?.model;
      if (provider && model) {
        addPromptHistory(ctx.sessionId, latestUserMsg.content, model, provider, 0, 0, false).catch(
          (err) => {
            logger.debug("Failed to save prompt history:", err);
          }
        );
      }
      lastPersistedUserMessageId.set(ctx.sessionId, latestUserMsg.id);
    }
  }

  state.addAgentMessage(ctx.sessionId, {
    id: crypto.randomUUID(),
    sessionId: ctx.sessionId,
    role: "system",
    content: `Error: ${event.message}`,
    timestamp: new Date().toISOString(),
  });
  state.clearAgentStreaming(ctx.sessionId);
  state.clearActiveSubAgents(ctx.sessionId);
  state.setAgentThinking(ctx.sessionId, false);
  state.setAgentResponding(ctx.sessionId, false);
};
