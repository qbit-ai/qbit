import { useEffect, useRef } from "react";
import { type AiEvent, onAiEvent, type ToolSource } from "@/lib/ai";
import { type ToolCallSource, useStore } from "@/store";

/** Convert AI event source to store source (snake_case to camelCase) */
function convertToolSource(source?: ToolSource): ToolCallSource | undefined {
  if (!source) return undefined;
  if (source.type === "main") return { type: "main" };
  if (source.type === "sub_agent") {
    return {
      type: "sub_agent",
      agentId: source.agent_id,
      agentName: source.agent_name,
    };
  }
  if (source.type === "workflow") {
    return {
      type: "workflow",
      workflowId: source.workflow_id,
      workflowName: source.workflow_name,
      stepName: source.step_name,
      stepIndex: source.step_index,
    };
  }
  return undefined;
}

/**
 * Hook to subscribe to AI events from the Tauri backend
 * and update the store accordingly.
 *
 * Events are routed to the correct session using `event.session_id` from the backend.
 * This ensures proper multi-session isolation even when the user switches tabs
 * during AI streaming.
 */
export function useAiEvents() {
  const unlistenRef = useRef<(() => void) | null>(null);

  useEffect(() => {
    // Track if this effect instance is still mounted (for async cleanup)
    let isMounted = true;

    const handleEvent = (event: AiEvent) => {
      // Get the session ID from the event for proper routing
      const state = useStore.getState();
      let sessionId = event.session_id;

      // Fall back to activeSessionId if session_id is unknown (shouldn't happen in normal operation)
      if (!sessionId || sessionId === "unknown") {
        console.warn("AI event received with unknown session_id, falling back to activeSessionId");
        const fallbackId = state.activeSessionId;
        if (!fallbackId) return;
        sessionId = fallbackId;
      }

      // Verify the session exists in the store
      if (!state.sessions[sessionId]) {
        console.debug("AI event for unknown session:", sessionId);
        return;
      }

      switch (event.type) {
        case "started":
          state.clearAgentStreaming(sessionId);
          state.clearActiveToolCalls(sessionId);
          state.clearThinkingContent(sessionId);
          state.setAgentThinking(sessionId, true);
          break;

        case "text_delta":
          state.setAgentThinking(sessionId, false);
          state.updateAgentStreaming(sessionId, event.delta);
          break;

        case "tool_request": {
          // Deduplicate: ignore already-processed requests
          if (state.isToolRequestProcessed(event.request_id)) {
            console.debug("Ignoring duplicate tool_request:", event.request_id);
            break;
          }
          state.setAgentThinking(sessionId, false);
          const toolCall = {
            id: event.request_id,
            name: event.tool_name,
            args: event.args as Record<string, unknown>,
            // All tool calls from AI events are executed by the agent
            executedByAgent: true,
            source: convertToolSource(event.source),
          };
          // Track the tool call as running (for UI display)
          state.addActiveToolCall(sessionId, toolCall);
          // Also add to streaming blocks for interleaved display
          state.addStreamingToolBlock(sessionId, toolCall);
          break;
        }

        case "tool_approval_request": {
          // Enhanced tool request with HITL metadata
          // Deduplicate: ignore already-processed requests
          if (state.isToolRequestProcessed(event.request_id)) {
            console.debug("Ignoring duplicate tool_approval_request:", event.request_id);
            break;
          }
          state.setAgentThinking(sessionId, false);

          const toolCall = {
            id: event.request_id,
            name: event.tool_name,
            args: event.args as Record<string, unknown>,
            executedByAgent: true,
            riskLevel: event.risk_level,
            stats: event.stats ?? undefined,
            suggestion: event.suggestion ?? undefined,
            canLearn: event.can_learn,
            source: convertToolSource(event.source),
          };

          // Track the tool call
          state.addActiveToolCall(sessionId, toolCall);
          state.addStreamingToolBlock(sessionId, toolCall);

          // Set pending tool approval for the dialog
          state.setPendingToolApproval(sessionId, {
            ...toolCall,
            status: "pending",
          });
          break;
        }

        case "tool_auto_approved": {
          // Tool was auto-approved based on learned patterns
          state.setAgentThinking(sessionId, false);
          const autoApprovedTool = {
            id: event.request_id,
            name: event.tool_name,
            args: event.args as Record<string, unknown>,
            executedByAgent: true,
            autoApproved: true,
            autoApprovalReason: event.reason,
            source: convertToolSource(event.source),
          };
          state.addActiveToolCall(sessionId, autoApprovedTool);
          state.addStreamingToolBlock(sessionId, autoApprovedTool);
          break;
        }

        case "tool_result":
          // Update tool call status to completed/error
          state.completeActiveToolCall(sessionId, event.request_id, event.success, event.result);
          // Also update streaming block
          state.updateStreamingToolBlock(sessionId, event.request_id, event.success, event.result);
          break;

        case "reasoning":
          // Append thinking content to the store for display
          console.log("[Thinking] Received reasoning event:", {
            sessionId,
            contentLength: event.content.length,
            contentPreview: event.content.slice(0, 100),
          });
          state.appendThinkingContent(sessionId, event.content);
          break;

        case "completed": {
          // Convert streaming blocks to a final assistant message preserving interleaved history
          const blocks = state.streamingBlocks[sessionId] || [];
          const streaming = state.agentStreaming[sessionId] || "";
          const thinkingContent = state.thinkingContent[sessionId] || "";
          const activeWorkflow = state.activeWorkflows[sessionId];

          // Filter out workflow tool calls - they're displayed in WorkflowTree instead
          const filteredBlocks = activeWorkflow
            ? blocks.filter((block) => {
                if (block.type !== "tool") return true;
                const source = block.toolCall.source;
                // Hide run_workflow tool and workflow-sourced tool calls
                if (block.toolCall.name === "run_workflow") return false;
                return !(
                  source?.type === "workflow" && source.workflowId === activeWorkflow.workflowId
                );
              })
            : blocks;

          // Preserve the interleaved streaming history (text + tool calls in order)
          const streamingHistory: import("@/store").FinalizedStreamingBlock[] = filteredBlocks.map(
            (block) => {
              if (block.type === "text") {
                return { type: "text" as const, content: block.content };
              }
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
          );

          // Extract tool calls for backwards compatibility
          const toolCalls = streamingHistory
            .filter(
              (b): b is { type: "tool"; toolCall: import("@/store").ToolCall } => b.type === "tool"
            )
            .map((b) => b.toolCall);

          // Use full accumulated text as content (fallback to event.response for edge cases)
          const content = streaming || event.response || "";

          // Preserve workflow tool calls before creating the message
          state.preserveWorkflowToolCalls(sessionId);

          // Create a deep copy of the workflow (with tool calls) for the message
          const workflowForMessage = activeWorkflow
            ? {
                ...activeWorkflow,
                toolCalls: [...(state.activeWorkflows[sessionId]?.toolCalls || [])],
              }
            : undefined;

          if (content || streamingHistory.length > 0 || workflowForMessage) {
            state.addAgentMessage(sessionId, {
              id: crypto.randomUUID(),
              sessionId: sessionId,
              role: "assistant",
              content: content,
              timestamp: new Date().toISOString(),
              toolCalls: toolCalls.length > 0 ? toolCalls : undefined,
              streamingHistory: streamingHistory.length > 0 ? streamingHistory : undefined,
              thinkingContent: thinkingContent || undefined,
              workflow: workflowForMessage,
            });
          }
          state.clearAgentStreaming(sessionId);
          state.clearStreamingBlocks(sessionId);
          state.clearThinkingContent(sessionId);
          state.clearActiveToolCalls(sessionId);
          // Clear the active workflow since it's now stored in the message
          state.clearActiveWorkflow(sessionId);
          state.setAgentThinking(sessionId, false);
          break;
        }

        case "error":
          state.addAgentMessage(sessionId, {
            id: crypto.randomUUID(),
            sessionId: sessionId,
            role: "system",
            content: `Error: ${event.message}`,
            timestamp: new Date().toISOString(),
          });
          state.clearAgentStreaming(sessionId);
          state.setAgentThinking(sessionId, false);
          break;

        // Workflow events
        case "workflow_started":
          state.startWorkflow(sessionId, {
            workflowId: event.workflow_id,
            workflowName: event.workflow_name,
            workflowSessionId: event.session_id,
          });
          break;

        case "workflow_step_started":
          state.workflowStepStarted(sessionId, {
            stepName: event.step_name,
            stepIndex: event.step_index,
            totalSteps: event.total_steps,
          });
          break;

        case "workflow_step_completed":
          state.workflowStepCompleted(sessionId, {
            stepName: event.step_name,
            output: event.output,
            durationMs: event.duration_ms,
          });
          break;

        case "workflow_completed":
          state.completeWorkflow(sessionId, {
            finalOutput: event.final_output,
            totalDurationMs: event.total_duration_ms,
          });
          break;

        case "workflow_error":
          state.failWorkflow(sessionId, {
            stepName: event.step_name,
            error: event.error,
          });
          break;

        // Plan events
        case "plan_updated":
          state.setPlan(sessionId, {
            version: event.version,
            summary: event.summary,
            steps: event.steps,
            explanation: event.explanation,
            updated_at: new Date().toISOString(),
          });
          break;
      }
    };

    // Only set up listener once - the handler uses getState() to access current values
    const setupListener = async () => {
      try {
        const unlisten = await onAiEvent(handleEvent);
        // Only store the unlisten function if we're still mounted
        // This handles the React Strict Mode double-mount where cleanup runs
        // before the async setup completes
        if (isMounted) {
          unlistenRef.current = unlisten;
        } else {
          // We were unmounted before setup completed - clean up immediately
          unlisten();
        }
      } catch {
        // AI backend not yet implemented - this is expected
        console.debug("AI events not available - backend not implemented yet");
      }
    };

    setupListener();

    return () => {
      isMounted = false;
      if (unlistenRef.current) {
        unlistenRef.current();
        unlistenRef.current = null;
      }
    };
  }, []);
}
