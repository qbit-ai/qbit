import { Bot, CheckCircle2, ChevronDown, ChevronRight, Loader2, XCircle } from "lucide-react";
import { memo, useState } from "react";
import { Markdown } from "@/components/Markdown";
import { Badge } from "@/components/ui/badge";
import { Collapsible, CollapsibleContent, CollapsibleTrigger } from "@/components/ui/collapsible";
import { cn } from "@/lib/utils";
import type { ActiveSubAgent, SubAgentToolCall } from "@/store";

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

/** Status badge component */
function StatusBadge({ status }: { status: "running" | "completed" | "error" }) {
  const config = {
    running: { bg: "bg-[var(--ansi-blue)]/20", text: "text-[var(--ansi-blue)]", label: "Running" },
    completed: {
      bg: "bg-[var(--ansi-green)]/20",
      text: "text-[var(--ansi-green)]",
      label: "Completed",
    },
    error: { bg: "bg-[var(--ansi-red)]/20", text: "text-[var(--ansi-red)]", label: "Error" },
  }[status];

  return (
    <Badge variant="outline" className={cn("text-[10px] px-1.5 py-0", config.bg, config.text)}>
      {config.label}
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

/** Sub-agent card component */
export const SubAgentCard = memo(function SubAgentCard({ subAgent }: SubAgentCardProps) {
  const [isExpanded, setIsExpanded] = useState(subAgent.status === "running");

  return (
    <div className="my-2 rounded-lg border border-border bg-card">
      <Collapsible open={isExpanded} onOpenChange={setIsExpanded}>
        <CollapsibleTrigger className="flex w-full items-center gap-2 px-3 py-2 hover:bg-accent/30">
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
          {subAgent.durationMs !== undefined && (
            <span className="ml-auto text-xs text-muted-foreground">
              {formatDuration(subAgent.durationMs)}
            </span>
          )}
        </CollapsibleTrigger>

        <CollapsibleContent className="px-3 pb-2">
          {/* Task description */}
          <div className="mb-2 rounded bg-muted/50 px-2 py-1.5 text-xs">
            <span className="text-muted-foreground">Task: </span>
            <span>{subAgent.task}</span>
          </div>

          {/* Tool calls */}
          {subAgent.toolCalls.length > 0 && (
            <div className="mb-2 space-y-0.5">
              <div className="text-xs text-muted-foreground">Tool calls:</div>
              {subAgent.toolCalls.map((tool) => (
                <ToolCallRow key={tool.id} tool={tool} />
              ))}
            </div>
          )}

          {/* Response (when completed) */}
          {subAgent.response && (
            <div className="mt-2 rounded bg-muted/30 px-2 py-1.5 text-xs">
              <span className="text-muted-foreground">Response: </span>
              <Markdown content={subAgent.response} className="mt-1" />
            </div>
          )}

          {/* Error (when failed) */}
          {subAgent.error && (
            <div className="mt-2 rounded bg-[var(--ansi-red)]/10 px-2 py-1.5 text-xs text-[var(--ansi-red)]">
              <span className="font-medium">Error: </span>
              {subAgent.error}
            </div>
          )}
        </CollapsibleContent>
      </Collapsible>
    </div>
  );
});
