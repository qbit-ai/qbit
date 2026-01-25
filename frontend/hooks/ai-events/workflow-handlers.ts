/**
 * Workflow-related AI event handlers.
 *
 * Handles workflow lifecycle events: started, step_started, step_completed,
 * completed, error.
 */

import type { EventHandler } from "./types";

/**
 * Handle workflow started event.
 */
export const handleWorkflowStarted: EventHandler<{
  type: "workflow_started";
  workflow_id: string;
  workflow_name: string;
  session_id: string;
  seq?: number;
}> = (event, ctx) => {
  ctx.getState().startWorkflow(ctx.sessionId, {
    workflowId: event.workflow_id,
    workflowName: event.workflow_name,
    workflowSessionId: event.session_id,
  });
};

/**
 * Handle workflow step started event.
 */
export const handleWorkflowStepStarted: EventHandler<{
  type: "workflow_step_started";
  workflow_id: string;
  step_name: string;
  step_index: number;
  total_steps: number;
  session_id: string;
  seq?: number;
}> = (event, ctx) => {
  ctx.getState().workflowStepStarted(ctx.sessionId, {
    stepName: event.step_name,
    stepIndex: event.step_index,
    totalSteps: event.total_steps,
  });
};

/**
 * Handle workflow step completed event.
 */
export const handleWorkflowStepCompleted: EventHandler<{
  type: "workflow_step_completed";
  workflow_id: string;
  step_name: string;
  output: string | null;
  duration_ms: number;
  session_id: string;
  seq?: number;
}> = (event, ctx) => {
  ctx.getState().workflowStepCompleted(ctx.sessionId, {
    stepName: event.step_name,
    output: event.output,
    durationMs: event.duration_ms,
  });
};

/**
 * Handle workflow completed event.
 */
export const handleWorkflowCompleted: EventHandler<{
  type: "workflow_completed";
  workflow_id: string;
  final_output: string;
  total_duration_ms: number;
  session_id: string;
  seq?: number;
}> = (event, ctx) => {
  ctx.getState().completeWorkflow(ctx.sessionId, {
    finalOutput: event.final_output,
    totalDurationMs: event.total_duration_ms,
  });
};

/**
 * Handle workflow error event.
 */
export const handleWorkflowError: EventHandler<{
  type: "workflow_error";
  workflow_id: string;
  step_name: string | null;
  error: string;
  session_id: string;
  seq?: number;
}> = (event, ctx) => {
  ctx.getState().failWorkflow(ctx.sessionId, {
    stepName: event.step_name,
    error: event.error,
  });
};
