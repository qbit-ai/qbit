import Ansi from "ansi-to-react";
import { Loader2, TerminalSquare } from "lucide-react";
import { useCallback, useEffect, useMemo, useRef } from "react";
import { Markdown } from "@/components/Markdown";
import { SubAgentCard } from "@/components/SubAgentCard";
import { StreamingThinkingBlock } from "@/components/ThinkingBlock";
import { ToolGroup, ToolItem } from "@/components/ToolCallDisplay";
import { UdiffResultBlock } from "@/components/UdiffResultBlock";
import { WelcomeScreen } from "@/components/WelcomeScreen";
import { WorkflowTree } from "@/components/WorkflowTree";
import { useProcessedOutput } from "@/hooks/useProcessedOutput";
import { stripOscSequences } from "@/lib/ansi";
import { type GroupedStreamingBlock, groupConsecutiveTools } from "@/lib/toolGrouping";
import {
  type ActiveSubAgent,
  useIsAgentResponding,
  useIsAgentThinking,
  usePendingCommand,
  useSessionTimeline,
  useStore,
  useStreamingBlocks,
  useThinkingContent,
} from "@/store";
import { UnifiedBlock } from "./UnifiedBlock";

/** Block type for rendering - includes sub-agent blocks */
type RenderBlock = GroupedStreamingBlock | { type: "sub_agent"; subAgent: ActiveSubAgent };

interface UnifiedTimelineProps {
  sessionId: string;
}

export function UnifiedTimeline({ sessionId }: UnifiedTimelineProps) {
  const timeline = useSessionTimeline(sessionId);
  const streamingBlocks = useStreamingBlocks(sessionId);
  const pendingCommand = usePendingCommand(sessionId);
  const isAgentThinking = useIsAgentThinking(sessionId);
  const isAgentResponding = useIsAgentResponding(sessionId);
  const thinkingContent = useThinkingContent(sessionId);
  const activeWorkflow = useStore((state) => state.activeWorkflows[sessionId]);
  const activeSubAgents = useStore((state) => state.activeSubAgents[sessionId] || []);
  const containerRef = useRef<HTMLDivElement>(null);
  const bottomRef = useRef<HTMLDivElement>(null);

  // Get processed output from VirtualTerminal for proper animation handling
  // Falls back to stripOscSequences if VirtualTerminal isn't available
  const pendingOutput = useProcessedOutput(sessionId, pendingCommand?.output, stripOscSequences);

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

  // Group consecutive tool calls for cleaner display
  const groupedBlocks = useMemo(
    () => groupConsecutiveTools(filteredStreamingBlocks),
    [filteredStreamingBlocks]
  );

  // Transform grouped blocks to replace sub_agent tool calls with SubAgentCard blocks inline
  // This ensures sub-agents appear at their correct position in the timeline (where they were spawned)
  // rather than being appended at the bottom
  const renderBlocks = useMemo((): RenderBlock[] => {
    let subAgentIndex = 0;
    const result: RenderBlock[] = [];

    for (const block of groupedBlocks) {
      if (block.type === "tool") {
        // Single tool - replace sub-agent spawns with SubAgentCard at this position
        if (block.toolCall.name.startsWith("sub_agent_")) {
          if (subAgentIndex < activeSubAgents.length) {
            result.push({ type: "sub_agent", subAgent: activeSubAgents[subAgentIndex] });
            subAgentIndex++;
          }
          continue;
        }
      } else if (block.type === "tool_group") {
        // Tool group - extract sub_agent tools and replace them with SubAgentCards
        const filteredTools = block.tools.filter((tool) => {
          if (tool.name.startsWith("sub_agent_")) {
            if (subAgentIndex < activeSubAgents.length) {
              result.push({ type: "sub_agent", subAgent: activeSubAgents[subAgentIndex] });
              subAgentIndex++;
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
    while (subAgentIndex < activeSubAgents.length) {
      result.push({ type: "sub_agent", subAgent: activeSubAgents[subAgentIndex] });
      subAgentIndex++;
    }

    return result;
  }, [groupedBlocks, activeSubAgents]);

  // Reference for pending scroll animation frame
  const pendingScrollRef = useRef<number | null>(null);

  const scrollToBottom = useCallback(() => {
    if (containerRef.current) {
      containerRef.current.scrollTop = containerRef.current.scrollHeight;
    }
  }, []);

  // Auto-scroll to bottom when new content arrives
  // Dependencies use length/boolean checks to avoid triggering on every character
  const hasThinkingContent = !!thinkingContent;
  const hasPendingOutput = pendingOutput.length > 0;
  const hasActiveWorkflow = !!activeWorkflow;
  const workflowStepCount = activeWorkflow?.steps.length ?? 0;
  const hasActiveSubAgents = activeSubAgents.length > 0;
  const subAgentToolCallCount = activeSubAgents.reduce((acc, a) => acc + a.toolCalls.length, 0);
  // biome-ignore lint/correctness/useExhaustiveDependencies: intentional triggers for auto-scroll
  useEffect(() => {
    scrollToBottom();
  }, [
    scrollToBottom,
    timeline.length,
    streamingBlocks.length,
    renderBlocks.length,
    hasPendingOutput,
    hasThinkingContent,
    hasActiveWorkflow,
    workflowStepCount,
    hasActiveSubAgents,
    subAgentToolCallCount,
  ]);

  // Cleanup pending scroll on unmount
  useEffect(() => {
    return () => {
      if (pendingScrollRef.current !== null) {
        clearTimeout(pendingScrollRef.current);
      }
    };
  }, []);

  // Empty state - only show if no timeline, no streaming, no thinking, and no command running
  const hasRunningCommand = pendingCommand?.command;
  if (
    timeline.length === 0 &&
    streamingBlocks.length === 0 &&
    !hasRunningCommand &&
    !isAgentThinking &&
    !thinkingContent
  ) {
    return <WelcomeScreen />;
  }

  return (
    <div ref={containerRef} className="flex-1 min-w-0 overflow-auto p-2 space-y-2">
      {timeline.map((block) => (
        <UnifiedBlock key={block.id} block={block} />
      ))}

      {/* Streaming output for running command - only show when there's an actual command */}
      {pendingCommand?.command && (
        <div className="ml-6 border-l-2 border-l-[#7aa2f7] mb-1">
          {/* Header */}
          <div className="flex items-center gap-1.5 px-2 py-1.5">
            <div className="flex items-center gap-1">
              <TerminalSquare className="w-3.5 h-3.5 text-[#7aa2f7]" />
              <span className="w-1.5 h-1.5 bg-[#7aa2f7] rounded-full animate-pulse" />
            </div>
            <code className="text-[#c0caf5] font-mono text-xs flex-1 truncate">
              {pendingCommand.command || "Running..."}
            </code>
          </div>
          {/* Streaming output */}
          {pendingOutput && (
            <div className="px-2 pb-2 pl-7">
              <div className="ansi-output text-xs leading-tight whitespace-pre-wrap break-words bg-[#13131a] rounded-md p-2 border border-[#1f2335] max-h-96 overflow-auto">
                <Ansi useClasses>{pendingOutput}</Ansi>
              </div>
            </div>
          )}
        </div>
      )}

      {/* Thinking indicator - shown while waiting for first content (when no thinking content yet) */}
      {isAgentThinking && streamingBlocks.length === 0 && !thinkingContent && !activeWorkflow && (
        <div className="ml-6 border-l-2 border-l-[var(--ansi-magenta)] bg-card/50 rounded-r-md p-2">
          <div className="flex items-center gap-1.5 text-xs text-muted-foreground">
            <Loader2 className="w-3.5 h-3.5 animate-spin text-[var(--ansi-magenta)]" />
            <span>Thinking...</span>
          </div>
        </div>
      )}

      {/* Agent response - contains thinking (if any), streaming content, sub-agents, and workflow tree */}
      {(thinkingContent ||
        streamingBlocks.length > 0 ||
        activeWorkflow ||
        activeSubAgents.length > 0) && (
        <div className="ml-6 border-l-2 border-l-[var(--ansi-magenta)] bg-card/50 rounded-r-md p-2 space-y-2">
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
                  />
                  {isLast && (
                    <span className="inline-block w-2 h-4 bg-[var(--ansi-magenta)] animate-pulse ml-0.5 align-middle" />
                  )}
                </div>
              );
            }
            if (block.type === "sub_agent") {
              return <SubAgentCard key={block.subAgent.agentId} subAgent={block.subAgent} />;
            }
            if (block.type === "tool_group") {
              return <ToolGroup key={`group-${block.tools[0].id}`} group={block} />;
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
            return <ToolItem key={block.toolCall.id} tool={block.toolCall} showInlineName />;
          })}

          {/* Workflow tree - hierarchical display of workflow steps and tool calls */}
          {activeWorkflow && <WorkflowTree sessionId={sessionId} />}
        </div>
      )}

      {/* Loading indicator - shown at the bottom when agent is responding */}
      {isAgentResponding && (
        <div className="flex items-center gap-2 py-2 px-3 text-xs text-muted-foreground">
          <Loader2 className="w-4 h-4 animate-spin text-accent" />
        </div>
      )}

      {/* Scroll anchor */}
      <div ref={bottomRef} />
    </div>
  );
}
