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
        "min-w-0 rounded-md overflow-hidden border-l-2 p-2 space-y-2",
        isUser
          ? "ml-auto max-w-[85%] bg-[var(--ansi-blue)]/10 border-l-[var(--ansi-blue)]"
          : isSystem
            ? "max-w-[95%] bg-[var(--ansi-yellow)]/10 border-l-[var(--ansi-yellow)]"
            : "max-w-[95%] bg-card/50 border-l-[var(--ansi-magenta)]"
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
                  <Markdown content={block.content} className="text-sm" />
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
            <p className="text-sm text-foreground whitespace-pre-wrap break-words">
              {message.content}
            </p>
          ) : (
            <Markdown content={message.content} className="text-sm" />
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

      {/* Timestamp */}
      <div className="text-[10px] text-muted-foreground">
        {new Date(message.timestamp).toLocaleTimeString([], {
          hour: "2-digit",
          minute: "2-digit",
        })}
      </div>
    </div>
  );
});
