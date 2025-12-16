import {
  Bot,
  CheckCircle,
  ChevronDown,
  ChevronRight,
  Edit,
  FileCode,
  FileText,
  FolderOpen,
  Globe,
  Loader2,
  Search,
  Terminal,
  Workflow,
  XCircle,
} from "lucide-react";
import { memo, useState } from "react";
import { Badge } from "@/components/ui/badge";
import { Collapsible, CollapsibleContent, CollapsibleTrigger } from "@/components/ui/collapsible";
import {
  type AnyToolCall,
  formatPrimaryArg,
  getGroupStatus,
  type ToolGroup as ToolGroupType,
} from "@/lib/toolGrouping";
import { formatToolResult } from "@/lib/tools";
import { cn } from "@/lib/utils";
import type { ToolCallSource } from "@/store";

/** Tool name to icon mapping */
const toolIcons: Record<string, typeof FileText> = {
  read_file: FileText,
  write_file: Edit,
  edit_file: Edit,
  list_files: FolderOpen,
  grep_file: Search,
  run_pty_cmd: Terminal,
  shell: Terminal,
  web_fetch: Globe,
  web_search: Globe,
  web_search_answer: Globe,
  apply_patch: FileCode,
};

/** Source badge to indicate where a tool call came from */
function SourceBadge({ source }: { source?: ToolCallSource }) {
  if (!source || source.type === "main") {
    return null;
  }

  if (source.type === "sub_agent") {
    return (
      <Badge
        variant="outline"
        className="bg-[var(--accent-dim)] text-accent border-accent/30 text-[9px] px-1 py-0 gap-0.5 shrink-0"
      >
        <Bot className="w-2.5 h-2.5" />
        {source.agentName || "sub-agent"}
      </Badge>
    );
  }

  if (source.type === "workflow") {
    return (
      <Badge
        variant="outline"
        className="bg-[var(--success-dim)] text-[var(--success)] border-[var(--success)]/30 text-[9px] px-1 py-0 gap-0.5 shrink-0"
      >
        <Workflow className="w-2.5 h-2.5" />
        {source.workflowName || "workflow"}
      </Badge>
    );
  }

  return null;
}

/** Status configuration for badges and icons */
const statusConfig: Record<
  AnyToolCall["status"],
  {
    icon: typeof CheckCircle;
    borderColor: string;
    badgeClass: string;
    label: string;
    animate?: boolean;
  }
> = {
  pending: {
    icon: Loader2,
    borderColor: "border-l-muted-foreground",
    badgeClass: "bg-muted text-muted-foreground hover:bg-muted/80",
    label: "Pending",
  },
  approved: {
    icon: CheckCircle,
    borderColor: "border-l-[var(--success)]",
    badgeClass: "bg-[var(--success-dim)] text-[var(--success)] hover:bg-[var(--success)]/20",
    label: "Approved",
  },
  denied: {
    icon: XCircle,
    borderColor: "border-l-destructive",
    badgeClass: "bg-destructive/10 text-destructive hover:bg-destructive/20",
    label: "Denied",
  },
  running: {
    icon: Loader2,
    borderColor: "border-l-accent",
    badgeClass: "bg-[var(--accent-dim)] text-accent",
    label: "Running",
    animate: true,
  },
  completed: {
    icon: CheckCircle,
    borderColor: "border-l-[var(--success)]",
    badgeClass: "bg-[var(--success-dim)] text-[var(--success)] hover:bg-[var(--success)]/20",
    label: "Completed",
  },
  error: {
    icon: XCircle,
    borderColor: "border-l-destructive",
    badgeClass: "bg-destructive/10 text-destructive hover:bg-destructive/20",
    label: "Error",
  },
};

interface ToolGroupProps {
  group: ToolGroupType;
  compact?: boolean;
}

/** Displays a group of consecutive tool calls of the same type */
export const ToolGroup = memo(function ToolGroup({ group, compact = false }: ToolGroupProps) {
  const groupStatus = getGroupStatus(group.tools);

  // Auto-expand if any tool is running or errored
  const shouldAutoExpand = groupStatus === "running" || groupStatus === "error";
  const [isOpen, setIsOpen] = useState(shouldAutoExpand);

  const Icon = toolIcons[group.toolName] || Terminal;
  const status = statusConfig[groupStatus];
  const StatusIcon = status.icon;

  // Get source from first tool (all tools in group should have same source)
  const firstTool = group.tools[0];
  const groupSource = "source" in firstTool ? firstTool.source : undefined;

  // Build preview text from primary arguments
  const previewItems = group.tools
    .map((tool) => formatPrimaryArg(tool))
    .filter((arg): arg is string => arg !== null);

  const maxPreviewItems = 3;
  const visiblePreview = previewItems.slice(0, maxPreviewItems);
  const hiddenCount = previewItems.length - visiblePreview.length;

  return (
    <Collapsible open={isOpen} onOpenChange={setIsOpen}>
      <div
        className={cn(
          "border-l-[3px] border-r-0 border-t-0 border-b-0 overflow-hidden rounded-l-lg shadow-sm",
          compact ? "bg-muted" : "bg-muted/50",
          status.borderColor
        )}
      >
        <CollapsibleTrigger asChild>
          <div className="cursor-pointer hover:bg-[var(--bg-hover)] transition-colors">
            {/* Header row */}
            <div className="flex items-center justify-between px-3 py-2">
              <div className="flex items-center gap-2">
                <ChevronRight
                  className={cn(
                    "w-4 h-4 text-muted-foreground/50 transition-transform",
                    isOpen && "rotate-90"
                  )}
                />
                <Icon
                  className={cn(compact ? "w-3 h-3" : "w-3.5 h-3.5", "text-muted-foreground")}
                />
                <span
                  className={cn(
                    "font-mono text-muted-foreground",
                    compact ? "text-[11px]" : "text-xs"
                  )}
                >
                  {group.toolName}
                </span>
                <Badge
                  variant="outline"
                  className="bg-muted/50 text-muted-foreground/60 border-muted-foreground/20 text-[10px] px-1.5 py-0 rounded-full"
                >
                  ×{group.tools.length}
                </Badge>
                <SourceBadge source={groupSource} />
              </div>
              {groupStatus !== "completed" && (
                <Badge
                  variant="outline"
                  className={cn(
                    "gap-1 flex items-center text-[10px] px-2 py-0.5 rounded-full",
                    status.badgeClass
                  )}
                >
                  <StatusIcon className={cn("w-3 h-3", status.animate && "animate-spin")} />
                  {!compact && status.label}
                </Badge>
              )}
            </div>

            {/* Preview line (only when collapsed) */}
            {!isOpen && visiblePreview.length > 0 && (
              <div className="px-3 pb-2 -mt-1 pl-9">
                <span className="text-[11px] text-muted-foreground/70 font-mono">
                  {visiblePreview.join(", ")}
                  {hiddenCount > 0 && (
                    <span className="text-muted-foreground/50">{` +${hiddenCount} more`}</span>
                  )}
                </span>
              </div>
            )}
          </div>
        </CollapsibleTrigger>

        {/* Expanded content - list of individual tools */}
        <CollapsibleContent>
          <div className="px-3 pb-2 space-y-0.5 pl-9">
            {group.tools.map((tool) => (
              <ToolGroupItem key={tool.id} tool={tool} compact={compact} />
            ))}
          </div>
        </CollapsibleContent>
      </div>
    </Collapsible>
  );
});

/** Individual item within a tool group (expandable display) */
const ToolGroupItem = memo(function ToolGroupItem({
  tool,
  compact,
}: {
  tool: AnyToolCall;
  compact?: boolean;
}) {
  const [isExpanded, setIsExpanded] = useState(false);
  const Icon = toolIcons[tool.name] || Terminal;
  const status = statusConfig[tool.status];
  const StatusIcon = status.icon;
  const primaryArg = formatPrimaryArg(tool);
  const hasArgs = Object.keys(tool.args).length > 0;
  const hasResult = tool.result !== undefined && tool.status !== "running";

  return (
    <div className="rounded-md bg-background/50">
      {/* Header row - clickable to expand */}
      <button
        type="button"
        onClick={() => setIsExpanded(!isExpanded)}
        className={cn(
          "flex items-center justify-between py-1.5 px-2 rounded-md cursor-pointer w-full text-left",
          "hover:bg-[var(--bg-hover)] transition-colors"
        )}
      >
        <div className="flex items-center gap-2 min-w-0">
          <ChevronDown
            className={cn(
              "w-3 h-3 text-muted-foreground transition-transform shrink-0",
              !isExpanded && "-rotate-90"
            )}
          />
          <Icon
            className={cn(compact ? "w-3 h-3" : "w-3.5 h-3.5", "text-muted-foreground shrink-0")}
          />
          {primaryArg ? (
            <span
              className={cn(
                "font-mono text-muted-foreground/70 truncate",
                compact ? "text-[10px]" : "text-[11px]"
              )}
            >
              {primaryArg}
            </span>
          ) : (
            <span
              className={cn(
                "font-mono text-muted-foreground/70 italic truncate",
                compact ? "text-[10px]" : "text-[11px]"
              )}
            >
              {tool.name}
            </span>
          )}
        </div>
        <div className="flex items-center gap-1.5 shrink-0">
          {"source" in tool && <SourceBadge source={tool.source} />}
          <StatusIcon
            className={cn(
              "w-3 h-3",
              status.animate && "animate-spin",
              tool.status === "completed" && "text-[var(--success)]",
              tool.status === "running" && "text-accent",
              tool.status === "error" && "text-destructive",
              tool.status === "pending" && "text-muted-foreground"
            )}
          />
        </div>
      </button>

      {/* Expanded content - args and result */}
      {isExpanded && (
        <div className="px-3 pb-2 space-y-2 border-t border-[var(--border-subtle)]">
          {/* Arguments */}
          {hasArgs && (
            <div className="pt-2">
              <span className="text-[10px] uppercase text-muted-foreground font-medium tracking-wide">
                Arguments
              </span>
              <pre className="mt-1 text-[11px] text-accent bg-background rounded-md p-2 overflow-auto max-h-32 whitespace-pre-wrap break-all font-mono">
                {JSON.stringify(tool.args, null, 2)}
              </pre>
            </div>
          )}

          {/* Result */}
          {hasResult && (
            <div>
              <span className="text-[10px] uppercase text-muted-foreground font-medium tracking-wide">
                {tool.status === "error" ? "Error" : "Result"}
              </span>
              <pre
                className={cn(
                  "mt-1 text-[11px] bg-background rounded-md p-2 overflow-auto max-h-40 whitespace-pre-wrap break-all font-mono",
                  tool.status === "error" ? "text-destructive" : "text-accent"
                )}
              >
                {formatToolResult(tool.result)}
              </pre>
            </div>
          )}

          {/* Running state */}
          {tool.status === "running" && (
            <div className="pt-2 text-[10px] text-muted-foreground italic">Running...</div>
          )}
        </div>
      )}
    </div>
  );
});
