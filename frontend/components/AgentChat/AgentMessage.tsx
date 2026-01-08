import { memo, useMemo, useState } from "react";
import { Markdown } from "@/components/Markdown";
import { CopyButton } from "@/components/Markdown/CopyButton";
import { SubAgentCard } from "@/components/SubAgentCard";
import { StaticThinkingBlock } from "@/components/ThinkingBlock";
import { ToolDetailsModal, ToolGroup, ToolItem } from "@/components/ToolCallDisplay";
import { UdiffResultBlock } from "@/components/UdiffResultBlock";
import { WorkflowProgress } from "@/components/WorkflowProgress";
import { extractMessageText } from "@/lib/messageUtils";
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

  // Transform grouped history to:
  // 1. Extract sub-agent blocks to render at the top (before agent's response)
  // 2. Filter out sub_agent tool calls from the main history
  const { subAgentBlocks, contentBlocks } = useMemo((): {
    subAgentBlocks: RenderBlock[];
    contentBlocks: RenderBlock[];
  } => {
    if (!hasStreamingHistory) return { subAgentBlocks: [], contentBlocks: [] };

    const subAgents = message.subAgents || [];

    // Check if we have parentRequestId for ID-based matching (newer data)
    const hasParentRequestIds = subAgents.length > 0 && subAgents[0].parentRequestId;

    const matchedParentIds = new Set<string>();
    const subAgentBlocks: RenderBlock[] = [];
    const contentBlocks: RenderBlock[] = [];
    let subAgentIndex = 0; // Fallback for legacy data

    for (const block of groupedHistory) {
      if (block.type === "tool") {
        // Single tool - check if it's a sub-agent spawn
        if (block.toolCall.name.startsWith("sub_agent_")) {
          // Match sub-agent by tool call ID (which equals parentRequestId)
          if (hasParentRequestIds) {
            const matchingSubAgent = subAgents.find(
              (a) =>
                a.parentRequestId === block.toolCall.id && !matchedParentIds.has(a.parentRequestId)
            );
            if (matchingSubAgent) {
              matchedParentIds.add(matchingSubAgent.parentRequestId);
              subAgentBlocks.push({ type: "sub_agent", subAgent: matchingSubAgent });
            }
          } else {
            // Fallback to index-based matching for legacy data
            if (subAgentIndex < subAgents.length) {
              subAgentBlocks.push({ type: "sub_agent", subAgent: subAgents[subAgentIndex] });
              subAgentIndex++;
            }
          }
          // Skip the tool call - don't add to content blocks
          continue;
        }
      } else if (block.type === "tool_group") {
        // Tool group - filter out sub_agent tools and potentially split the group
        const filteredTools = block.tools.filter((tool) => {
          if (tool.name.startsWith("sub_agent_")) {
            // Match sub-agent by tool ID
            if (hasParentRequestIds) {
              const matchingSubAgent = subAgents.find(
                (a) => a.parentRequestId === tool.id && !matchedParentIds.has(a.parentRequestId)
              );
              if (matchingSubAgent) {
                matchedParentIds.add(matchingSubAgent.parentRequestId);
                subAgentBlocks.push({ type: "sub_agent", subAgent: matchingSubAgent });
              }
            } else {
              // Fallback to index-based matching for legacy data
              if (subAgentIndex < subAgents.length) {
                subAgentBlocks.push({ type: "sub_agent", subAgent: subAgents[subAgentIndex] });
                subAgentIndex++;
              }
            }
            return false;
          }
          return true;
        });

        if (filteredTools.length > 0) {
          // Rebuild the group with remaining tools
          if (filteredTools.length === 1) {
            contentBlocks.push({ type: "tool", toolCall: filteredTools[0] });
          } else {
            contentBlocks.push({ ...block, tools: filteredTools });
          }
        }
        continue;
      }

      // Pass through text blocks unchanged
      contentBlocks.push(block);
    }

    return { subAgentBlocks, contentBlocks };
  }, [groupedHistory, message.subAgents, hasStreamingHistory]);

  // Extract copyable text for assistant messages
  const copyableText = useMemo(() => {
    if (isUser || isSystem) return "";
    return extractMessageText(message);
  }, [message, isUser, isSystem]);

  const isAssistant = !isUser && !isSystem;

  return (
    <div
      className={cn(
        "min-w-0 overflow-hidden space-y-2",
        isUser
          ? "w-full border-l-[3px] border-l-[#484f58] bg-[#1c2128] py-4 px-5 rounded-r-lg relative group"
          : isSystem
            ? "ml-6 rounded-lg bg-[var(--ansi-yellow)]/10 border-l-2 border-l-[var(--ansi-yellow)] p-2"
            : "ml-6 rounded-lg bg-card/50 p-2 relative group"
      )}
    >
      {/* Copy button for user messages */}
      {isUser && message.content && (
        <CopyButton
          content={message.content}
          className="absolute right-2 top-2 opacity-0 group-hover:opacity-100 transition-opacity z-10"
          data-testid="user-message-copy-button"
        />
      )}
      {/* Copy button for assistant messages */}
      {isAssistant && copyableText && (
        <CopyButton
          content={copyableText}
          className="absolute right-2 top-2 opacity-0 group-hover:opacity-100 transition-opacity z-10"
          data-testid="assistant-message-copy-button"
        />
      )}
      {/* Thinking content (collapsible) */}
      {message.thinkingContent && <StaticThinkingBlock content={message.thinkingContent} />}

      {/* Workflow progress (if workflow was executed during this message) */}
      {message.workflow && <WorkflowProgress workflow={message.workflow} />}

      {/* Render interleaved streaming history if available (grouped for cleaner display) */}
      {hasStreamingHistory ? (
        <div className="space-y-2">
          {/* Sub-agent cards rendered first, above the main response */}
          {subAgentBlocks.map((block) => {
            if (block.type === "sub_agent") {
              return (
                <SubAgentCard
                  key={block.subAgent.parentRequestId || block.subAgent.agentId}
                  subAgent={block.subAgent}
                />
              );
            }
            return null;
          })}

          {/* Main content blocks (text, tools, etc.) */}
          {contentBlocks.map((block, blockIndex) => {
            const prevBlock = blockIndex > 0 ? contentBlocks[blockIndex - 1] : null;
            const nextBlock =
              blockIndex < contentBlocks.length - 1 ? contentBlocks[blockIndex + 1] : null;
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
            if (block.type === "tool") {
              return (
                <ToolItem
                  key={block.toolCall.id}
                  tool={block.toolCall}
                  showInlineName
                  onViewDetails={setSelectedTool}
                />
              );
            }
            return null;
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
