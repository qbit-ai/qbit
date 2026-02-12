/**
 * Sub-agent related AI event handlers.
 *
 * Handles sub-agent lifecycle events: started, tool_request, tool_result,
 * completed, error.
 */

import type { EventHandler } from "./types";

/**
 * Handle prompt generation started event.
 */
export const handlePromptGenerationStarted: EventHandler<{
  type: "prompt_generation_started";
  agent_id: string;
  parent_request_id: string;
  architect_system_prompt: string;
  architect_user_message: string;
  session_id: string;
  seq?: number;
}> = (event, ctx) => {
  ctx.getState().startPromptGeneration(ctx.sessionId, event.agent_id, event.parent_request_id, {
    architectSystemPrompt: event.architect_system_prompt,
    architectUserMessage: event.architect_user_message,
  });
};

/**
 * Handle prompt generation completed event.
 */
export const handlePromptGenerationCompleted: EventHandler<{
  type: "prompt_generation_completed";
  agent_id: string;
  parent_request_id: string;
  generated_prompt?: string;
  success: boolean;
  duration_ms: number;
  session_id: string;
  seq?: number;
}> = (event, ctx) => {
  ctx.getState().completePromptGeneration(ctx.sessionId, event.agent_id, event.parent_request_id, {
    generatedPrompt: event.generated_prompt,
    success: event.success,
    durationMs: event.duration_ms,
  });
};

/**
 * Handle sub-agent started event.
 */
export const handleSubAgentStarted: EventHandler<{
  type: "sub_agent_started";
  agent_id: string;
  agent_name: string;
  task: string;
  depth: number;
  parent_request_id: string;
  session_id: string;
  seq?: number;
}> = (event, ctx) => {
  ctx.getState().startSubAgent(ctx.sessionId, {
    agentId: event.agent_id,
    agentName: event.agent_name,
    parentRequestId: event.parent_request_id,
    task: event.task,
    depth: event.depth,
  });
};

/**
 * Handle sub-agent tool request event.
 */
export const handleSubAgentToolRequest: EventHandler<{
  type: "sub_agent_tool_request";
  agent_id: string;
  tool_name: string;
  args: unknown;
  request_id: string;
  parent_request_id: string;
  session_id: string;
  seq?: number;
}> = (event, ctx) => {
  ctx.getState().addSubAgentToolCall(ctx.sessionId, event.parent_request_id, {
    id: event.request_id,
    name: event.tool_name,
    args: event.args as Record<string, unknown>,
  });
};

/**
 * Handle sub-agent tool result event.
 */
export const handleSubAgentToolResult: EventHandler<{
  type: "sub_agent_tool_result";
  agent_id: string;
  tool_name: string;
  success: boolean;
  result: unknown;
  request_id: string;
  parent_request_id: string;
  session_id: string;
  seq?: number;
}> = (event, ctx) => {
  ctx
    .getState()
    .completeSubAgentToolCall(
      ctx.sessionId,
      event.parent_request_id,
      event.request_id,
      event.success,
      event.result
    );
};

/**
 * Handle sub-agent completed event.
 */
export const handleSubAgentCompleted: EventHandler<{
  type: "sub_agent_completed";
  agent_id: string;
  response: string;
  duration_ms: number;
  parent_request_id: string;
  session_id: string;
  seq?: number;
}> = (event, ctx) => {
  const state = ctx.getState();

  // Handle coder results with special rendering
  if (event.agent_id === "coder") {
    // Flush pending text deltas to ensure correct ordering
    ctx.flushSessionDeltas(ctx.sessionId);
    state.addUdiffResultBlock(ctx.sessionId, event.response, event.duration_ms);
  }

  state.completeSubAgent(ctx.sessionId, event.parent_request_id, {
    response: event.response,
    durationMs: event.duration_ms,
  });
};

/**
 * Handle sub-agent error event.
 */
export const handleSubAgentError: EventHandler<{
  type: "sub_agent_error";
  agent_id: string;
  error: string;
  parent_request_id: string;
  session_id: string;
  seq?: number;
}> = (event, ctx) => {
  ctx.getState().failSubAgent(ctx.sessionId, event.parent_request_id, event.error);
};
