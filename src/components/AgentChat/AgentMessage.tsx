import { memo, useMemo } from "react";
import { Markdown } from "@/components/Markdown";
import { StaticThinkingBlock } from "@/components/ThinkingBlock";
import { ToolGroup, ToolItem } from "@/components/ToolCallDisplay";
import { WorkflowProgress } from "@/components/WorkflowProgress";
import { groupConsecutiveTools } from "@/lib/toolGrouping";
import { cn } from "@/lib/utils";
import type { AgentMessage as AgentMessageType } from "@/store";

interface AgentMessageProps {
  message: AgentMessageType;
}

export const AgentMessage = memo(function AgentMessage({ message }: AgentMessageProps) {
  const isUser = message.role === "user";
  const isSystem = message.role === "system";

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
          ? "ml-auto max-w-[70%] rounded-[12px_12px_4px_12px] bg-muted border border-[var(--border-medium)] px-3.5 py-2.5"
          : isSystem
            ? "max-w-[95%] rounded-lg bg-[var(--ansi-yellow)]/10 border-l-2 border-l-[var(--ansi-yellow)] p-2"
            : "max-w-[95%] rounded-lg bg-card/50 p-2"
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
            if (block.type === "text") {
              return (
                // biome-ignore lint/suspicious/noArrayIndexKey: blocks are in fixed order and never reordered
                <div key={`text-${blockIndex}`}>
                  <Markdown
                    content={block.content}
                    className="text-[13px] leading-relaxed text-muted-foreground"
                  />
                </div>
              );
            }
            if (block.type === "tool_group") {
              return <ToolGroup key={`group-${block.tools[0].id}`} group={block} />;
            }
            // Single tool - show with inline name
            return <ToolItem key={block.toolCall.id} tool={block.toolCall} showInlineName />;
          })}
        </div>
      ) : (
        <>
          {/* Legacy: Message content */}
          {isUser ? (
            <p className="text-[13px] text-foreground whitespace-pre-wrap break-words leading-relaxed">
              {message.content}
            </p>
          ) : (
            <Markdown
              content={message.content}
              className="text-[13px] leading-relaxed text-muted-foreground"
            />
          )}

          {/* Legacy: Tool calls */}
          {message.toolCalls && message.toolCalls.length > 0 && (
            <div className="mt-2 space-y-1.5">
              {message.toolCalls.map((tool) => (
                <ToolItem key={tool.id} tool={tool} />
              ))}
            </div>
          )}
        </>
      )}
    </div>
  );
});
