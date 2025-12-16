import {
  Bot,
  CheckCircle,
  ChevronRight,
  Edit,
  FileCode,
  FileText,
  FolderOpen,
  Globe,
  Loader2,
  Search,
  Terminal,
  XCircle,
} from "lucide-react";
import { memo, useState } from "react";
import { TruncatedOutput } from "@/components/TruncatedOutput";
import { Badge } from "@/components/ui/badge";
import { Collapsible, CollapsibleContent, CollapsibleTrigger } from "@/components/ui/collapsible";
import { formatPrimaryArg } from "@/lib/toolGrouping";
import { formatToolResult, isAgentTerminalCommand } from "@/lib/tools";
import { cn } from "@/lib/utils";
import type { ActiveToolCall, ToolCall } from "@/store";

/** Union type for both ToolCall and ActiveToolCall */
type AnyToolCall = ToolCall | ActiveToolCall;

/** Props for a single tool item */
interface ToolItemProps {
  tool: AnyToolCall;
  /** Compact mode uses less visual weight, suitable for inline display */
  compact?: boolean;
  /** Show primary argument inline with tool name (e.g., "read_file: utils.ts") */
  showInlineName?: boolean;
}

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

/** Single tool call item with collapsible details */
export const ToolItem = memo(function ToolItem({
  tool,
  compact = false,
  showInlineName = false,
}: ToolItemProps) {
  const [isOpen, setIsOpen] = useState(false);
  const Icon = toolIcons[tool.name] || Terminal;
  const status = statusConfig[tool.status];
  const StatusIcon = status.icon;
  const isTerminalCmd = isAgentTerminalCommand(tool);
  const hasArgs = Object.keys(tool.args).length > 0;
  const primaryArg = showInlineName ? formatPrimaryArg(tool) : null;

  // For terminal commands, always show output (non-collapsible header behavior)
  // For other tools, make the header clickable to expand
  const canExpand = !isTerminalCmd;

  return (
    <Collapsible open={isOpen} onOpenChange={canExpand ? setIsOpen : undefined}>
      <div
        className={cn(
          "border-l-[3px] border-r-0 border-t-0 border-b-0 overflow-hidden rounded-l-lg shadow-sm",
          isTerminalCmd
            ? "border-l-accent bg-[var(--accent-dim)]"
            : cn(status.borderColor, "bg-muted/50"),
          compact && "bg-accent"
        )}
      >
        <CollapsibleTrigger asChild disabled={!canExpand}>
          <div
            className={cn(
              "flex items-center justify-between px-3 py-2 transition-colors",
              canExpand && "cursor-pointer hover:bg-[var(--bg-hover)]"
            )}
          >
            <div className="flex items-center gap-2">
              {canExpand && (
                <ChevronRight
                  className={cn(
                    "w-4 h-4 text-muted-foreground/50 transition-transform",
                    isOpen && "rotate-90"
                  )}
                />
              )}
              <Icon className={cn(compact ? "w-3 h-3" : "w-3.5 h-3.5", "text-muted-foreground")} />
              <span
                className={cn(
                  "font-mono text-muted-foreground",
                  compact ? "text-[11px]" : "text-xs"
                )}
              >
                {tool.name}
                {primaryArg && (
                  <span>
                    : <span className="text-muted-foreground/70">{primaryArg}</span>
                  </span>
                )}
              </span>
              {isTerminalCmd && (
                <Bot className={cn("text-muted-foreground", compact ? "w-3 h-3" : "w-3.5 h-3.5")} />
              )}
            </div>
            {tool.status !== "completed" && (
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
        </CollapsibleTrigger>

        {/* For terminal commands, show output directly (not collapsible) */}
        {isTerminalCmd && (
          <div className="px-3 pb-2 pl-9">
            {tool.result !== undefined && tool.status !== "running" ? (
              <TruncatedOutput content={formatToolResult(tool.result)} maxLines={10} />
            ) : (
              <span className="text-[10px] text-muted-foreground italic">
                {tool.status === "running" ? "Running..." : "Awaiting output"}
              </span>
            )}
          </div>
        )}

        {/* For non-terminal tools, show collapsible args/result */}
        <CollapsibleContent>
          <div className="px-3 pb-2.5 pl-9 space-y-2">
            {/* Arguments */}
            {hasArgs && (
              <div>
                <span className="text-[10px] uppercase text-muted-foreground font-medium tracking-wide">
                  Arguments
                </span>
                <pre className="mt-1 text-[11px] text-accent bg-background rounded-md p-2 overflow-auto max-h-32 whitespace-pre-wrap break-all font-mono">
                  {JSON.stringify(tool.args, null, 2)}
                </pre>
              </div>
            )}

            {/* Result */}
            {tool.result !== undefined && tool.status !== "running" && (
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
          </div>
        </CollapsibleContent>
      </div>
    </Collapsible>
  );
});

/** Props for the tool call list display */
interface ToolCallDisplayProps {
  toolCalls: AnyToolCall[];
  /** Compact mode uses less visual weight */
  compact?: boolean;
}

/** Display a list of tool calls with their status */
export function ToolCallDisplay({ toolCalls, compact = false }: ToolCallDisplayProps) {
  if (toolCalls.length === 0) return null;

  return (
    <div className="space-y-1 my-1.5">
      {toolCalls.map((tool) => (
        <ToolItem key={tool.id} tool={tool} compact={compact} />
      ))}
    </div>
  );
}
