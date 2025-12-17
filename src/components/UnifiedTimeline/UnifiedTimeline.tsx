import Ansi from "ansi-to-react";
import { Loader2, TerminalSquare } from "lucide-react";
import { useCallback, useEffect, useMemo, useRef } from "react";
import { Markdown } from "@/components/Markdown";
import { StreamingThinkingBlock } from "@/components/ThinkingBlock";
import { ToolGroup, ToolItem } from "@/components/ToolCallDisplay";
import { WelcomeScreen } from "@/components/WelcomeScreen";
import { WorkflowTree } from "@/components/WorkflowTree";
import { stripOscSequences } from "@/lib/ansi";
import { groupConsecutiveTools } from "@/lib/toolGrouping";
import {
  useIsAgentThinking,
  usePendingCommand,
  useSessionTimeline,
  useStore,
  useStreamingBlocks,
  useThinkingContent,
} from "@/store";
import { UnifiedBlock } from "./UnifiedBlock";

interface UnifiedTimelineProps {
  sessionId: string;
}

export function UnifiedTimeline({ sessionId }: UnifiedTimelineProps) {
  const timeline = useSessionTimeline(sessionId);
  const streamingBlocks = useStreamingBlocks(sessionId);
  const pendingCommand = usePendingCommand(sessionId);
  const isAgentThinking = useIsAgentThinking(sessionId);
  const thinkingContent = useThinkingContent(sessionId);
  const activeWorkflow = useStore((state) => state.activeWorkflows[sessionId]);
  const containerRef = useRef<HTMLDivElement>(null);
  const bottomRef = useRef<HTMLDivElement>(null);

  // Strip OSC sequences from pending output for display
  const pendingOutput = useMemo(
    () => (pendingCommand?.output ? stripOscSequences(pendingCommand.output) : ""),
    [pendingCommand?.output]
  );

  // Filter out workflow tool calls when a workflow is active (they show in WorkflowTree instead)
  const filteredStreamingBlocks = useMemo(() => {
    if (!activeWorkflow) return streamingBlocks;

    return streamingBlocks.filter((block) => {
      if (block.type !== "tool") return true;
      const toolCall = block.toolCall;

      // Hide the run_workflow tool call itself since WorkflowTree shows the workflow
      if (toolCall.name === "run_workflow") return false;

      // Hide tool calls from the active workflow (they show nested in WorkflowTree)
      const source = toolCall.source;
      return !(source?.type === "workflow" && source.workflowId === activeWorkflow.workflowId);
    });
  }, [streamingBlocks, activeWorkflow]);

  // Group consecutive tool calls for cleaner display
  const groupedBlocks = useMemo(
    () => groupConsecutiveTools(filteredStreamingBlocks),
    [filteredStreamingBlocks]
  );

  // Throttled scroll with trailing edge - scrolls immediately on first call,
  // then at most once per interval while updates keep coming
  const lastScrollTimeRef = useRef<number>(0);
  const pendingScrollRef = useRef<number | null>(null);
  const SCROLL_THROTTLE_MS = 100;

  const scrollToBottom = useCallback(() => {
    const now = Date.now();
    const timeSinceLastScroll = now - lastScrollTimeRef.current;

    // If enough time has passed, scroll immediately
    if (timeSinceLastScroll >= SCROLL_THROTTLE_MS) {
      lastScrollTimeRef.current = now;
      // Use RAF for smooth visual sync
      requestAnimationFrame(() => {
        bottomRef.current?.scrollIntoView({ behavior: "smooth" });
      });
    } else {
      // Otherwise, schedule a trailing scroll if not already scheduled
      if (pendingScrollRef.current === null) {
        const delay = SCROLL_THROTTLE_MS - timeSinceLastScroll;
        pendingScrollRef.current = window.setTimeout(() => {
          lastScrollTimeRef.current = Date.now();
          pendingScrollRef.current = null;
          requestAnimationFrame(() => {
            bottomRef.current?.scrollIntoView({ behavior: "smooth" });
          });
        }, delay);
      }
    }
  }, []);

  // Auto-scroll to bottom when new content arrives
  // Dependencies use length/boolean checks to avoid triggering on every character
  const hasThinkingContent = !!thinkingContent;
  const hasPendingOutput = pendingOutput.length > 0;
  const hasActiveWorkflow = !!activeWorkflow;
  const workflowStepCount = activeWorkflow?.steps.length ?? 0;
  // biome-ignore lint/correctness/useExhaustiveDependencies: intentional triggers for auto-scroll
  useEffect(() => {
    scrollToBottom();
  }, [
    scrollToBottom,
    timeline.length,
    streamingBlocks.length,
    hasPendingOutput,
    hasThinkingContent,
    hasActiveWorkflow,
    workflowStepCount,
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

      {/* Agent response - contains thinking (if any), streaming content, and workflow tree */}
      {(thinkingContent || streamingBlocks.length > 0 || activeWorkflow) && (
        <div className="ml-6 border-l-2 border-l-[var(--ansi-magenta)] bg-card/50 rounded-r-md p-2 space-y-2">
          {/* Extended thinking block */}
          {thinkingContent && <StreamingThinkingBlock sessionId={sessionId} />}

          {/* Streaming text and tool calls (grouped for cleaner display) */}
          {groupedBlocks.map((block, blockIndex) => {
            if (block.type === "text") {
              const isLast = blockIndex === groupedBlocks.length - 1 && !activeWorkflow;
              return (
                // biome-ignore lint/suspicious/noArrayIndexKey: blocks are appended and never reordered
                <div key={`text-${blockIndex}`}>
                  <Markdown content={block.content} className="text-sm" streaming />
                  {isLast && (
                    <span className="inline-block w-2 h-4 bg-[var(--ansi-magenta)] animate-pulse ml-0.5 align-middle" />
                  )}
                </div>
              );
            }
            if (block.type === "tool_group") {
              return <ToolGroup key={`group-${block.tools[0].id}`} group={block} />;
            }
            // Single tool - show with inline name
            return <ToolItem key={block.toolCall.id} tool={block.toolCall} showInlineName />;
          })}

          {/* Workflow tree - hierarchical display of workflow steps and tool calls */}
          {activeWorkflow && <WorkflowTree sessionId={sessionId} />}
        </div>
      )}

      {/* Scroll anchor */}
      <div ref={bottomRef} />
    </div>
  );
}
