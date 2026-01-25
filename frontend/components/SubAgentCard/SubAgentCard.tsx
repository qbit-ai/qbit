import {
  Bot,
  CheckCircle2,
  ChevronDown,
  ChevronRight,
  Loader2,
  Maximize2,
  XCircle,
} from "lucide-react";
import { memo, useState } from "react";
import { Badge } from "@/components/ui/badge";
import { Collapsible, CollapsibleContent, CollapsibleTrigger } from "@/components/ui/collapsible";
import { cn } from "@/lib/utils";
import type { ActiveSubAgent, SubAgentToolCall } from "@/store";
import { SubAgentDetailsModal } from "./SubAgentDetailsModal";

interface SubAgentCardProps {
  subAgent: ActiveSubAgent;
}

/** Status icon component */
function StatusIcon({
  status,
  size = "md",
}: {
  status: "running" | "completed" | "error";
  size?: "sm" | "md";
}) {
  const sizeClass = size === "sm" ? "w-3 h-3" : "w-4 h-4";

  switch (status) {
    case "completed":
      return <CheckCircle2 className={cn(sizeClass, "text-[var(--ansi-green)]")} />;
    case "running":
      return <Loader2 className={cn(sizeClass, "text-[var(--ansi-blue)] animate-spin")} />;
    case "error":
      return <XCircle className={cn(sizeClass, "text-[var(--ansi-red)]")} />;
  }
}

/** Status badge component - styled like ToolGroup's running indicator */
function StatusBadge({ status }: { status: "running" | "completed" | "error" }) {
  // Only show badge for running status (completed/error show via other indicators)
  if (status !== "running") return null;

  return (
    <Badge
      variant="outline"
      className="ml-auto gap-1 flex items-center text-[10px] px-2 py-0.5 rounded-full bg-[var(--accent-dim)] text-accent"
    >
      <Loader2 className="w-3 h-3 animate-spin" />
      Running
    </Badge>
  );
}

/** Format duration in ms to human readable */
function formatDuration(ms?: number): string {
  if (!ms) return "";
  if (ms < 1000) return `${ms}ms`;
  return `${(ms / 1000).toFixed(1)}s`;
}

/** Individual tool call row */
const ToolCallRow = memo(function ToolCallRow({ tool }: { tool: SubAgentToolCall }) {
  const [isExpanded, setIsExpanded] = useState(false);
  const status =
    tool.status === "completed" ? "completed" : tool.status === "error" ? "error" : "running";

  // Get primary argument for display
  const primaryArg = (() => {
    const args = tool.args;
    if (typeof args === "object" && args !== null) {
      if ("path" in args) return String(args.path);
      if ("file_path" in args) return String(args.file_path);
      if ("command" in args) return String(args.command);
      if ("pattern" in args) return String(args.pattern);
    }
    return null;
  })();

  return (
    <Collapsible open={isExpanded} onOpenChange={setIsExpanded}>
      <CollapsibleTrigger className="group flex w-full items-center gap-1.5 rounded px-1.5 py-0.5 text-xs hover:bg-accent/50">
        {isExpanded ? (
          <ChevronDown className="h-3 w-3 text-muted-foreground" />
        ) : (
          <ChevronRight className="h-3 w-3 text-muted-foreground" />
        )}
        <StatusIcon status={status} size="sm" />
        <span className="font-mono text-[var(--ansi-cyan)]">{tool.name}</span>
        {primaryArg && (
          <span className="truncate text-muted-foreground" title={primaryArg}>
            {primaryArg}
          </span>
        )}
        {tool.completedAt && (
          <span className="ml-auto text-[10px] text-muted-foreground">
            {formatDuration(
              new Date(tool.completedAt).getTime() - new Date(tool.startedAt).getTime()
            )}
          </span>
        )}
      </CollapsibleTrigger>
      <CollapsibleContent className="px-4 py-1">
        <div className="space-y-1 text-xs">
          {/* Arguments */}
          <div>
            <span className="text-muted-foreground">Args:</span>
            <pre className="mt-0.5 rounded bg-muted px-2 py-1 text-[10px]">
              {JSON.stringify(tool.args, null, 2)}
            </pre>
          </div>

          {/* Result (if available) */}
          {tool.result !== undefined && (
            <div>
              <span className="text-muted-foreground">Result:</span>
              <pre className="mt-0.5 max-h-40 overflow-auto rounded bg-muted px-2 py-1 text-[10px]">
                {typeof tool.result === "string"
                  ? tool.result
                  : JSON.stringify(tool.result, null, 2)}
              </pre>
            </div>
          )}
        </div>
      </CollapsibleContent>
    </Collapsible>
  );
});

/** Number of tool calls to show by default */
const VISIBLE_TOOL_CALLS = 3;

/** Sub-agent card component */
export const SubAgentCard = memo(function SubAgentCard({ subAgent }: SubAgentCardProps) {
  const [isExpanded, setIsExpanded] = useState(subAgent.status === "running");
  const [showAllToolCalls, setShowAllToolCalls] = useState(false);
  const [showDetailsModal, setShowDetailsModal] = useState(false);

  // Calculate which tool calls to show
  const totalToolCalls = subAgent.toolCalls.length;
  const hiddenCount = Math.max(0, totalToolCalls - VISIBLE_TOOL_CALLS);
  const visibleToolCalls = showAllToolCalls
    ? subAgent.toolCalls
    : subAgent.toolCalls.slice(-VISIBLE_TOOL_CALLS);

  const hasExpandableContent = totalToolCalls > 0 || !!subAgent.error;

  return (
    <>
      <div className="mt-1 mb-3 rounded-lg border border-border bg-card">
        {hasExpandableContent ? (
          <Collapsible open={isExpanded} onOpenChange={setIsExpanded}>
            <div className="flex items-center gap-2 px-3 py-2">
              <CollapsibleTrigger className="flex flex-1 items-center gap-2 hover:bg-accent/30 rounded -ml-1 pl-1 py-0.5">
                {isExpanded ? (
                  <ChevronDown className="h-4 w-4 text-muted-foreground" />
                ) : (
                  <ChevronRight className="h-4 w-4 text-muted-foreground" />
                )}
                <Bot className="h-4 w-4 text-[var(--ansi-magenta)]" />
                <span className="font-medium text-sm">{subAgent.agentName}</span>
                <StatusBadge status={subAgent.status} />
                {subAgent.depth > 1 && (
                  <Badge variant="outline" className="text-[10px] px-1.5 py-0">
                    depth {subAgent.depth}
                  </Badge>
                )}
              </CollapsibleTrigger>
              <div className="flex items-center gap-2">
                {subAgent.durationMs !== undefined && (
                  <span className="text-xs text-muted-foreground">
                    {formatDuration(subAgent.durationMs)}
                  </span>
                )}
                <button
                  type="button"
                  onClick={() => setShowDetailsModal(true)}
                  className="p-1 hover:bg-accent/50 rounded transition-colors"
                  title="View details"
                >
                  <Maximize2 className="w-3.5 h-3.5 text-muted-foreground hover:text-foreground" />
                </button>
              </div>
            </div>

            <CollapsibleContent className="px-3 pb-2">
              {/* Tool calls */}
              {totalToolCalls > 0 && (
                <div className="space-y-0.5">
                  {/* Show "N previous tool calls" expander if there are hidden calls */}
                  {hiddenCount > 0 && !showAllToolCalls && (
                    <button
                      type="button"
                      onClick={() => setShowAllToolCalls(true)}
                      className="flex w-full items-center gap-1.5 rounded px-1.5 py-1 text-xs text-muted-foreground hover:bg-accent/50 hover:text-foreground"
                    >
                      <ChevronRight className="h-3 w-3" />
                      <span>
                        {hiddenCount} previous tool call{hiddenCount > 1 ? "s" : ""}
                      </span>
                    </button>
                  )}

                  {/* Show collapse button when expanded */}
                  {showAllToolCalls && hiddenCount > 0 && (
                    <button
                      type="button"
                      onClick={() => setShowAllToolCalls(false)}
                      className="flex w-full items-center gap-1.5 rounded px-1.5 py-1 text-xs text-muted-foreground hover:bg-accent/50 hover:text-foreground"
                    >
                      <ChevronDown className="h-3 w-3" />
                      <span>Hide {hiddenCount} tool calls</span>
                    </button>
                  )}

                  {visibleToolCalls.map((tool) => (
                    <ToolCallRow key={tool.id} tool={tool} />
                  ))}
                </div>
              )}

              {/* Error indicator (when failed) */}
              {subAgent.error && (
                <div className="mt-2 rounded bg-[var(--ansi-red)]/10 px-2 py-1.5 text-xs text-[var(--ansi-red)]">
                  <span className="font-medium">Error: </span>
                  {subAgent.error}
                </div>
              )}
            </CollapsibleContent>
          </Collapsible>
        ) : (
          <div className="flex items-center gap-2 px-3 py-2">
            <div className="flex flex-1 items-center gap-2 -ml-1 pl-1 py-0.5">
              <Bot className="h-4 w-4 text-[var(--ansi-magenta)]" />
              <span className="font-medium text-sm">{subAgent.agentName}</span>
              <StatusBadge status={subAgent.status} />
              {subAgent.depth > 1 && (
                <Badge variant="outline" className="text-[10px] px-1.5 py-0">
                  depth {subAgent.depth}
                </Badge>
              )}
            </div>
            <div className="flex items-center gap-2">
              {subAgent.durationMs !== undefined && (
                <span className="text-xs text-muted-foreground">
                  {formatDuration(subAgent.durationMs)}
                </span>
              )}
              <button
                type="button"
                onClick={() => setShowDetailsModal(true)}
                className="p-1 hover:bg-accent/50 rounded transition-colors"
                title="View details"
              >
                <Maximize2 className="w-3.5 h-3.5 text-muted-foreground hover:text-foreground" />
              </button>
            </div>
          </div>
        )}
      </div>

      {/* Details Modal */}
      {showDetailsModal && (
        <SubAgentDetailsModal subAgent={subAgent} onClose={() => setShowDetailsModal(false)} />
      )}
    </>
  );
});
