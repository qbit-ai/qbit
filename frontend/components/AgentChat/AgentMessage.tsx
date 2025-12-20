import { memo, useMemo, useState } from "react";
import { Markdown } from "@/components/Markdown";
import { SubAgentCard } from "@/components/SubAgentCard";
import { StaticThinkingBlock } from "@/components/ThinkingBlock";
import { ToolDetailsModal, ToolGroup, ToolItem } from "@/components/ToolCallDisplay";
import { UdiffResultBlock } from "@/components/UdiffResultBlock";
import { WorkflowProgress } from "@/components/WorkflowProgress";
import type { AnyToolCall, GroupedStreamingBlock } from "@/lib/toolGrouping";
import { groupConsecutiveTools } from "@/lib/toolGrouping";
import { cn } from "@/lib/utils";
import type { ActiveSubAgent, AgentMessage as AgentMessageType } from "@/store";

interface AgentMessageProps {
  message: AgentMessageType;
}

/** Block type for rendering - includes sub-agent blocks */
type RenderBlock = GroupedStreamingBlock | { type: "sub_agent"; subAgent: ActiveSubAgent };

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

  // Transform grouped history to replace sub_agent tool calls with SubAgentCard blocks
  const renderBlocks = useMemo((): RenderBlock[] => {
    if (!hasStreamingHistory) return [];

    const subAgents = message.subAgents || [];
    let subAgentIndex = 0;
    const result: RenderBlock[] = [];

    for (const block of groupedHistory) {
      if (block.type === "tool") {
        // Single tool - check if it's a sub-agent spawn
        if (block.toolCall.name.startsWith("sub_agent_")) {
          // Replace with SubAgentCard if we have matching sub-agent data
          if (subAgentIndex < subAgents.length) {
            result.push({ type: "sub_agent", subAgent: subAgents[subAgentIndex] });
            subAgentIndex++;
          }
          // Skip the tool call - don't render it
          continue;
        }
      } else if (block.type === "tool_group") {
        // Tool group - filter out sub_agent tools and potentially split the group
        const filteredTools = block.tools.filter((tool) => {
          if (tool.name.startsWith("sub_agent_")) {
            // Add SubAgentCard for this tool
            if (subAgentIndex < subAgents.length) {
              result.push({ type: "sub_agent", subAgent: subAgents[subAgentIndex] });
              subAgentIndex++;
            }
            return false;
          }
          return true;
        });

        if (filteredTools.length > 0) {
          // Rebuild the group with remaining tools
          if (filteredTools.length === 1) {
            result.push({ type: "tool", toolCall: filteredTools[0] });
          } else {
            result.push({ ...block, tools: filteredTools });
          }
        }
        continue;
      }

      // Pass through text blocks unchanged
      result.push(block);
    }

    return result;
  }, [groupedHistory, message.subAgents, hasStreamingHistory]);

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
          {renderBlocks.map((block, blockIndex) => {
            const prevBlock = blockIndex > 0 ? renderBlocks[blockIndex - 1] : null;
            const nextBlock =
              blockIndex < renderBlocks.length - 1 ? renderBlocks[blockIndex + 1] : null;
            const prevWasTool =
              prevBlock?.type === "tool_group" ||
              prevBlock?.type === "tool" ||
              prevBlock?.type === "sub_agent";
            const nextIsTool =
              nextBlock?.type === "tool_group" ||
              nextBlock?.type === "tool" ||
              nextBlock?.type === "sub_agent";

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
            if (block.type === "sub_agent") {
              return <SubAgentCard key={block.subAgent.agentId} subAgent={block.subAgent} />;
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
            if (block.type === "udiff_result") {
              return (
                <UdiffResultBlock
                  // biome-ignore lint/suspicious/noArrayIndexKey: blocks are in fixed order
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
