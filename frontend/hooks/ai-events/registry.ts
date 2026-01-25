/**
 * AI Event Handler Registry.
 *
 * Combines all event handlers into a single registry that maps
 * event types to their handlers.
 */

import type { AiEvent } from "@/lib/ai";
import {
  handleCompactionCompleted,
  handleCompactionFailed,
  handleCompactionStarted,
  handleContextWarning,
  handleToolResponseTruncated,
} from "./context-handlers";
import {
  handleCompleted,
  handleError,
  handleReasoning,
  handleStarted,
  handleSystemHooksInjected,
  handleTextDelta,
} from "./core-handlers";
import {
  handlePlanUpdated,
  handleServerToolStarted,
  handleWebFetchResult,
  handleWebSearchResult,
} from "./misc-handlers";
import {
  handleSubAgentCompleted,
  handleSubAgentError,
  handleSubAgentStarted,
  handleSubAgentToolRequest,
  handleSubAgentToolResult,
} from "./sub-agent-handlers";
import {
  handleToolApprovalRequest,
  handleToolAutoApproved,
  handleToolRequest,
  handleToolResult,
} from "./tool-handlers";
import type { EventHandler, EventHandlerContext, EventHandlerRegistry } from "./types";
import {
  handleWorkflowCompleted,
  handleWorkflowError,
  handleWorkflowStarted,
  handleWorkflowStepCompleted,
  handleWorkflowStepStarted,
} from "./workflow-handlers";

/**
 * Registry of all AI event handlers.
 * Maps event type to its handler function.
 */
export const eventHandlerRegistry: EventHandlerRegistry = {
  // Core lifecycle events
  started: handleStarted,
  system_hooks_injected: handleSystemHooksInjected,
  text_delta: handleTextDelta,
  reasoning: handleReasoning,
  completed: handleCompleted,
  error: handleError,

  // Tool events
  tool_request: handleToolRequest,
  tool_approval_request: handleToolApprovalRequest,
  tool_auto_approved: handleToolAutoApproved,
  tool_result: handleToolResult,

  // Workflow events
  workflow_started: handleWorkflowStarted,
  workflow_step_started: handleWorkflowStepStarted,
  workflow_step_completed: handleWorkflowStepCompleted,
  workflow_completed: handleWorkflowCompleted,
  workflow_error: handleWorkflowError,

  // Sub-agent events
  sub_agent_started: handleSubAgentStarted,
  sub_agent_tool_request: handleSubAgentToolRequest,
  sub_agent_tool_result: handleSubAgentToolResult,
  sub_agent_completed: handleSubAgentCompleted,
  sub_agent_error: handleSubAgentError,

  // Context management events
  context_warning: handleContextWarning,
  compaction_started: handleCompactionStarted,
  compaction_completed: handleCompactionCompleted,
  compaction_failed: handleCompactionFailed,
  tool_response_truncated: handleToolResponseTruncated,

  // Plan events
  plan_updated: handlePlanUpdated,

  // Server tool events
  server_tool_started: handleServerToolStarted,
  web_search_result: handleWebSearchResult,
  web_fetch_result: handleWebFetchResult,
};

/**
 * Dispatch an event to its registered handler.
 *
 * @param event - The AI event to dispatch
 * @param ctx - The handler context
 * @returns true if the event was handled, false if no handler was found
 */
export function dispatchEvent(event: AiEvent, ctx: EventHandlerContext): boolean {
  const handler = eventHandlerRegistry[event.type] as EventHandler<AiEvent> | undefined;
  if (handler) {
    handler(event, ctx);
    return true;
  }
  return false;
}

// Re-export types for convenience
export type { EventHandler, EventHandlerContext, EventHandlerRegistry };
