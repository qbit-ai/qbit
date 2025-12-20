import { memo, useMemo, useState } from "react";
import { Markdown } from "@/components/Markdown";
import { StaticThinkingBlock } from "@/components/ThinkingBlock";
import { ToolDetailsModal, ToolGroup, ToolItem } from "@/components/ToolCallDisplay";
import { WorkflowProgress } from "@/components/WorkflowProgress";
import type { AnyToolCall } from "@/lib/toolGrouping";
import { groupConsecutiveTools } from "@/lib/toolGrouping";
import { cn } from "@/lib/utils";
import type { AgentMessage as AgentMessageType } from "@/store";

interface AgentMessageProps {
  message: AgentMessageType;
}

export const AgentMessage = memo(function AgentMessage({ message }: AgentMessageProps) {
  const isUser = message.role === "user";
  const isSystem = message.role === "system";

  // State for selected tool to show in modal
  const [selectedTool, setSelectedTool] = useState<AnyToolCall | null>(null);

  // Use streamingHistory if available (interleaved text + tool calls), otherwise fallback to legacy
  const hasStreamingHistory = message.streamingHistory && message.streamingHistory.length > 0;

  // Group consecutive tool calls for cleaner display
  const groupedHistory = useMemo(
    () => (message.streamingHistory ? groupConsecutiveTools(message.streamingHistory) : []),
    [message.streamingHistory]
  );

  return (
    <div
      className={cn(
        "min-w-0 overflow-hidden space-y-2",
        isUser
          ? "w-full border-l-[3px] border-l-[#484f58] bg-[#1c2128] py-4 px-5 rounded-r-lg"
          : isSystem
            ? "ml-6 rounded-lg bg-[var(--ansi-yellow)]/10 border-l-2 border-l-[var(--ansi-yellow)] p-2"
            : "ml-6 rounded-lg bg-card/50 p-2"
      )}
    >
      {/* Thinking content (collapsible) */}
      {message.thinkingContent && <StaticThinkingBlock content={message.thinkingContent} />}

      {/* Workflow progress (if workflow was executed during this message) */}
      {message.workflow && <WorkflowProgress workflow={message.workflow} />}

      {/* Render interleaved streaming history if available (grouped for cleaner display) */}
      {hasStreamingHistory ? (
        <div className="space-y-2">
          {groupedHistory.map((block, blockIndex) => {
            const prevBlock = blockIndex > 0 ? groupedHistory[blockIndex - 1] : null;
            const nextBlock =
              blockIndex < groupedHistory.length - 1 ? groupedHistory[blockIndex + 1] : null;
            const prevWasTool = prevBlock?.type === "tool_group" || prevBlock?.type === "tool";
            const nextIsTool = nextBlock?.type === "tool_group" || nextBlock?.type === "tool";

            if (block.type === "text") {
              return (
                <div
                  // biome-ignore lint/suspicious/noArrayIndexKey: blocks are in fixed order
                  key={`text-${blockIndex}`}
                  className={cn(prevWasTool && "mt-6", nextIsTool && "mb-4")}
                >
                  <Markdown
                    content={block.content}
                    className="text-[14px] font-medium leading-relaxed text-foreground/85"
                  />
                </div>
              );
            }
            if (block.type === "tool_group") {
              return (
                <ToolGroup
                  key={`group-${block.tools[0].id}`}
                  group={block}
                  onViewDetails={setSelectedTool}
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
        </div>
      ) : (
        <>
          {/* Legacy: Message content */}
          {isUser ? (
            <p className="text-[14px] text-foreground whitespace-pre-wrap break-words leading-relaxed">
              {message.content}
            </p>
          ) : (
            <Markdown
              content={message.content}
              className="text-[14px] font-medium leading-relaxed text-foreground/85"
            />
          )}

          {/* Legacy: Tool calls */}
          {message.toolCalls && message.toolCalls.length > 0 && (
            <div className="mt-2 space-y-1.5">
              {message.toolCalls.map((tool) => (
                <ToolItem key={tool.id} tool={tool} onViewDetails={setSelectedTool} />
              ))}
            </div>
          )}
        </>
      )}

      {/* Tool Details Modal */}
      <ToolDetailsModal tool={selectedTool} onClose={() => setSelectedTool(null)} />
    </div>
  );
});
