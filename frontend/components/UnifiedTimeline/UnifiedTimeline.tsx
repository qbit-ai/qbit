import { Loader2, Sparkles } from "lucide-react";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { LiveTerminalBlock } from "@/components/LiveTerminalBlock";
import { Markdown } from "@/components/Markdown";
import { SubAgentCard } from "@/components/SubAgentCard";
import { StreamingThinkingBlock } from "@/components/ThinkingBlock";
import {
  MainToolGroup,
  ToolDetailsModal,
  ToolGroupDetailsModal,
  ToolItem,
} from "@/components/ToolCallDisplay";
import { UdiffResultBlock } from "@/components/UdiffResultBlock";
import { WelcomeScreen } from "@/components/WelcomeScreen";
import { WorkflowTree } from "@/components/WorkflowTree";
import {
  type AnyToolCall,
  type GroupedStreamingBlock,
  groupConsecutiveToolsByAny,
} from "@/lib/toolGrouping";
import {
  type ActiveSubAgent,
  useIsAgentThinking,
  usePendingCommand,
  useSessionTimeline,
  useStore,
  useStreamingBlocks,
  useStreamingTextLength,
  useThinkingContent,
} from "@/store";
import { UnifiedBlock } from "./UnifiedBlock";

/** Hook to check if context compaction is in progress for a session */
function useIsCompacting(sessionId: string): boolean {
  return useStore((state) => state.isCompacting[sessionId] ?? false);
}

/** Block type for rendering - includes sub-agent blocks */
type RenderBlock = GroupedStreamingBlock | { type: "sub_agent"; subAgent: ActiveSubAgent };

interface UnifiedTimelineProps {
  sessionId: string;
}

export function UnifiedTimeline({ sessionId }: UnifiedTimelineProps) {
  const timeline = useSessionTimeline(sessionId);
  const streamingBlocks = useStreamingBlocks(sessionId);
  const streamingTextLength = useStreamingTextLength(sessionId);
  const pendingCommand = usePendingCommand(sessionId);
  const isAgentThinking = useIsAgentThinking(sessionId);
  const thinkingContent = useThinkingContent(sessionId);
  const activeWorkflow = useStore((state) => state.activeWorkflows[sessionId]);
  const activeSubAgents = useStore((state) => state.activeSubAgents[sessionId] || []);
  const workingDirectory = useStore((state) => state.sessions[sessionId]?.workingDirectory || "");
  const isCompacting = useIsCompacting(sessionId);
  const containerRef = useRef<HTMLDivElement>(null);
  const bottomRef = useRef<HTMLDivElement>(null);

  // State for selected tool to show in modal
  const [selectedTool, setSelectedTool] = useState<AnyToolCall | null>(null);

  // State for selected tool group to show in modal
  const [selectedToolGroup, setSelectedToolGroup] = useState<AnyToolCall[] | null>(null);

  // Filter out workflow tool calls (they show in WorkflowTree instead)
  // Note: sub_agent_ tool calls are NOT filtered here - they're handled in renderBlocks
  // where they get replaced inline with SubAgentCard components at the correct position
  const filteredStreamingBlocks = useMemo(() => {
    return streamingBlocks.filter((block) => {
      if (block.type !== "tool") return true;
      const toolCall = block.toolCall;

      // Hide the run_workflow tool call itself since WorkflowTree shows the workflow
      if (toolCall.name === "run_workflow") return false;

      // Hide tool calls from the active workflow (they show nested in WorkflowTree)
      if (activeWorkflow) {
        const source = toolCall.source;
        if (source?.type === "workflow" && source.workflowId === activeWorkflow.workflowId) {
          return false;
        }
      }

      return true;
    });
  }, [streamingBlocks, activeWorkflow]);

  // Group ANY consecutive tool calls for cleaner display
  const groupedBlocks = useMemo(
    () => groupConsecutiveToolsByAny(filteredStreamingBlocks),
    [filteredStreamingBlocks]
  );

  // Transform grouped blocks to replace sub_agent tool calls with SubAgentCard blocks inline
  // This ensures sub-agents appear at their correct position in the timeline (where they were spawned)
  // rather than being appended at the bottom
  const renderBlocks = useMemo((): RenderBlock[] => {
    const matchedParentIds = new Set<string>();
    const result: RenderBlock[] = [];

    for (const block of groupedBlocks) {
      if (block.type === "tool") {
        // Single tool - replace sub-agent spawns with SubAgentCard at this position
        if (block.toolCall.name.startsWith("sub_agent_")) {
          // Match sub-agent by the tool call's ID (which equals the sub-agent's parentRequestId)
          const matchingSubAgent = activeSubAgents.find(
            (a) =>
              a.parentRequestId === block.toolCall.id && !matchedParentIds.has(a.parentRequestId)
          );
          if (matchingSubAgent) {
            matchedParentIds.add(matchingSubAgent.parentRequestId);
            result.push({ type: "sub_agent", subAgent: matchingSubAgent });
          }
          continue;
        }
      } else if (block.type === "tool_group") {
        // Tool group - extract sub_agent tools and replace them with SubAgentCards
        const filteredTools = block.tools.filter((tool) => {
          if (tool.name.startsWith("sub_agent_")) {
            const matchingSubAgent = activeSubAgents.find(
              (a) => a.parentRequestId === tool.id && !matchedParentIds.has(a.parentRequestId)
            );
            if (matchingSubAgent) {
              matchedParentIds.add(matchingSubAgent.parentRequestId);
              result.push({ type: "sub_agent", subAgent: matchingSubAgent });
            }
            return false;
          }
          return true;
        });

        if (filteredTools.length > 0) {
          if (filteredTools.length === 1) {
            result.push({ type: "tool", toolCall: filteredTools[0] });
          } else {
            result.push({ ...block, tools: filteredTools });
          }
        }
        continue;
      }

      result.push(block);
    }

    // Fallback: Add any remaining sub-agents that weren't matched to tool calls
    // This can happen if activeSubAgents state updates before streamingBlocks
    for (const subAgent of activeSubAgents) {
      if (!matchedParentIds.has(subAgent.parentRequestId)) {
        result.push({ type: "sub_agent", subAgent });
      }
    }

    return result;
  }, [groupedBlocks, activeSubAgents]);

  // Reference for pending scroll animation frame
  const pendingScrollRef = useRef<number | null>(null);

  const scrollToBottom = useCallback(() => {
    // Cancel any pending scroll to avoid stacking multiple scrolls
    if (pendingScrollRef.current !== null) {
      cancelAnimationFrame(pendingScrollRef.current);
    }

    // Defer scroll to next animation frame to ensure DOM has updated
    pendingScrollRef.current = requestAnimationFrame(() => {
      if (containerRef.current) {
        containerRef.current.scrollTop = containerRef.current.scrollHeight;
      }
      pendingScrollRef.current = null;
    });
  }, []);

  // Auto-scroll to bottom when new content arrives
  // streamingTextLength triggers scroll during text streaming (throttled to ~50 char buckets)
  const hasThinkingContent = !!thinkingContent;
  const hasPendingCommand = !!pendingCommand?.command;
  const hasActiveWorkflow = !!activeWorkflow;
  const workflowStepCount = activeWorkflow?.steps.length ?? 0;
  const hasActiveSubAgents = activeSubAgents.length > 0;
  const subAgentToolCallCount = activeSubAgents.reduce((acc, a) => acc + a.toolCalls.length, 0);
  // Throttle streaming text scroll triggers to every ~50 characters
  const streamingTextBucket = Math.floor(streamingTextLength / 50);
  // biome-ignore lint/correctness/useExhaustiveDependencies: intentional triggers for auto-scroll
  useEffect(() => {
    scrollToBottom();
  }, [
    scrollToBottom,
    timeline.length,
    streamingBlocks.length,
    streamingTextBucket,
    renderBlocks.length,
    hasPendingCommand,
    hasThinkingContent,
    hasActiveWorkflow,
    workflowStepCount,
    hasActiveSubAgents,
    subAgentToolCallCount,
    isCompacting,
  ]);

  // Cleanup pending scroll on unmount
  useEffect(() => {
    return () => {
      if (pendingScrollRef.current !== null) {
        cancelAnimationFrame(pendingScrollRef.current);
      }
    };
  }, []);

  // Empty state - only show if no timeline, no streaming, no thinking, and no command running
  // Check for both command AND output (output may exist even without command_start if shell integration isn't installed)
  const hasRunningCommand = pendingCommand?.command || pendingCommand?.output;
  const isEmpty =
    timeline.length === 0 &&
    streamingBlocks.length === 0 &&
    !hasRunningCommand &&
    !isAgentThinking &&
    !thinkingContent;

  return (
    <div ref={containerRef} className="flex-1 min-h-0 min-w-0 overflow-auto p-2 space-y-2">
      {isEmpty ? (
        <WelcomeScreen />
      ) : (
        <>
          {timeline.map((block) => (
            <UnifiedBlock key={block.id} block={block} sessionId={sessionId} />
          ))}

          {/* Streaming output for running command */}
          {/* Show if we have a command OR if we have buffered output (fallback for missing command_start) */}
          {(pendingCommand?.command || pendingCommand?.output) && (
            <LiveTerminalBlock sessionId={sessionId} command={pendingCommand?.command || null} />
          )}

          {/* Thinking indicator - shown while waiting for first content (when no thinking content yet) */}
          {isAgentThinking &&
            streamingBlocks.length === 0 &&
            !thinkingContent &&
            !activeWorkflow && (
              <div className="ml-6 border-l-2 border-l-[var(--color-ansi-magenta)] bg-card/50 rounded-r-md p-2">
                <div className="flex items-center gap-1.5 text-xs text-muted-foreground">
                  <Loader2 className="w-3.5 h-3.5 animate-spin text-[var(--color-ansi-magenta)]" />
                  <span>Thinking...</span>
                </div>
              </div>
            )}

          {/* Agent response - contains thinking (if any), streaming content, sub-agents, and workflow tree */}
          {(thinkingContent ||
            streamingBlocks.length > 0 ||
            activeWorkflow ||
            activeSubAgents.length > 0) && (
            <div className="ml-6 border-l-2 border-l-[var(--color-ansi-magenta)] bg-card/50 rounded-r-md p-2 space-y-2">
              {/* Extended thinking block */}
              {thinkingContent && <StreamingThinkingBlock sessionId={sessionId} />}

              {/* Streaming text, tool calls, and sub-agents (grouped and interleaved for cleaner display) */}
              {renderBlocks.map((block, blockIndex) => {
                if (block.type === "text") {
                  const isLast = blockIndex === renderBlocks.length - 1 && !activeWorkflow;
                  return (
                    // biome-ignore lint/suspicious/noArrayIndexKey: blocks are appended and never reordered
                    <div key={`text-${blockIndex}`}>
                      <Markdown
                        content={block.content}
                        className="text-[14px] font-medium leading-relaxed text-foreground/85"
                        streaming
                        sessionId={sessionId}
                        workingDirectory={workingDirectory}
                      />
                      {isLast && (
                        <span className="inline-block w-2 h-4 bg-[var(--color-ansi-magenta)] animate-pulse ml-0.5 align-middle" />
                      )}
                    </div>
                  );
                }
                if (block.type === "sub_agent") {
                  return (
                    <SubAgentCard key={block.subAgent.parentRequestId} subAgent={block.subAgent} />
                  );
                }
                if (block.type === "tool_group") {
                  return (
                    <MainToolGroup
                      key={`group-${block.tools[0].id}`}
                      tools={block.tools}
                      onViewToolDetails={setSelectedTool}
                      onViewGroupDetails={() => setSelectedToolGroup(block.tools)}
                    />
                  );
                }
                if (block.type === "udiff_result") {
                  return (
                    <UdiffResultBlock
                      // biome-ignore lint/suspicious/noArrayIndexKey: blocks are appended and never reordered
                      key={`udiff-${blockIndex}`}
                      response={block.response}
                      durationMs={block.durationMs}
                    />
                  );
                }
                // Single tool - show with inline name
                return (
                  <ToolItem
                    key={block.toolCall.id}
                    tool={block.toolCall}
                    showInlineName
                    onViewDetails={setSelectedTool}
                  />
                );
              })}

              {/* Workflow tree - hierarchical display of workflow steps and tool calls */}
              {activeWorkflow && <WorkflowTree sessionId={sessionId} />}
            </div>
          )}

          {/* Context compaction indicator */}
          {isCompacting && (
            <div className="ml-6 border-l-2 border-l-[var(--color-ansi-yellow)] bg-card/50 rounded-r-md p-3">
              <div className="flex items-center gap-2 text-sm">
                <Sparkles className="w-4 h-4 animate-pulse text-[var(--color-ansi-yellow)]" />
                <span className="font-medium text-foreground/85">Compacting context...</span>
              </div>
              <p className="mt-1 text-xs text-muted-foreground ml-6">
                Summarizing conversation history to free up context space.
              </p>
            </div>
          )}
        </>
      )}

      {/* Scroll anchor */}
      <div ref={bottomRef} />

      {/* Tool Details Modal */}
      <ToolDetailsModal tool={selectedTool} onClose={() => setSelectedTool(null)} />

      {/* Tool Group Details Modal */}
      <ToolGroupDetailsModal
        tools={selectedToolGroup}
        onClose={() => setSelectedToolGroup(null)}
        onViewToolDetails={setSelectedTool}
      />
    </div>
  );
}
