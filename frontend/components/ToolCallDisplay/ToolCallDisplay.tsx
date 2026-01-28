import {
  Bot,
  CheckCircle,
  Edit,
  FileCode,
  FileText,
  FolderOpen,
  Globe,
  Loader2,
  Maximize2,
  Search,
  Terminal,
  XCircle,
} from "lucide-react";
import { memo } from "react";
import { StreamingOutput } from "@/components/StreamingOutput";
import { TruncatedOutput } from "@/components/TruncatedOutput";
import { Badge } from "@/components/ui/badge";
import { formatPrimaryArg } from "@/lib/toolGrouping";
import { formatShellCommandResult, isAgentTerminalCommand } from "@/lib/tools";
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
  /** Callback when user clicks "View Details" button */
  onViewDetails?: (tool: AnyToolCall) => void;
}

/** Tool name to icon mapping */
const toolIcons: Record<string, typeof FileText> = {
  read_file: FileText,
  write_file: Edit,
  edit_file: Edit,
  list_files: FolderOpen,
  grep_file: Search,
  run_pty_cmd: Terminal,
  run_command: Terminal,
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

/** Single tool call item - click to view details in modal */
export const ToolItem = memo(function ToolItem({
  tool,
  compact = false,
  showInlineName = false,
  onViewDetails,
}: ToolItemProps) {
  const Icon = toolIcons[tool.name] || Terminal;
  const status = statusConfig[tool.status];
  const StatusIcon = status.icon;
  const isTerminalCmd = isAgentTerminalCommand(tool);
  const primaryArg = showInlineName ? formatPrimaryArg(tool) : null;

  return (
    <div
      className={cn(
        "mt-1 mb-3 border-l-[3px] border-r-0 border-t-0 border-b-0 overflow-hidden rounded-l-lg shadow-sm",
        isTerminalCmd
          ? "border-l-accent bg-[var(--accent-dim)]"
          : cn(status.borderColor, "bg-muted/50"),
        compact && "bg-accent"
      )}
    >
      {/* biome-ignore lint/a11y/noStaticElementInteractions: Role and tabIndex are set when interactive */}
      <div
        onClick={onViewDetails ? () => onViewDetails(tool) : undefined}
        onKeyDown={
          onViewDetails
            ? (e) => {
                if (e.key === "Enter" || e.key === " ") onViewDetails(tool);
              }
            : undefined
        }
        role={onViewDetails ? "button" : undefined}
        tabIndex={onViewDetails ? 0 : undefined}
        className={cn(
          "flex items-center justify-between px-3 py-2 transition-colors",
          onViewDetails && "cursor-pointer hover:bg-[var(--bg-hover)]"
        )}
      >
        <div className="flex items-center gap-2 flex-1 min-w-0">
          <Icon
            className={cn(compact ? "w-3 h-3" : "w-3.5 h-3.5", "text-muted-foreground shrink-0")}
          />
          <span
            className={cn("font-mono text-muted-foreground", compact ? "text-[11px]" : "text-xs")}
          >
            {tool.name}
            {primaryArg && (
              <span>
                : <span className="text-muted-foreground/70">{primaryArg}</span>
              </span>
            )}
          </span>
          {isTerminalCmd && (
            <Bot
              className={cn("text-muted-foreground shrink-0", compact ? "w-3 h-3" : "w-3.5 h-3.5")}
            />
          )}
        </div>
        <div className="flex items-center gap-1.5 shrink-0">
          {onViewDetails && (
            <button
              type="button"
              onClick={(e) => {
                e.stopPropagation();
                onViewDetails(tool);
              }}
              className="p-1 hover:bg-[var(--bg-hover)] rounded transition-colors"
              title="View details"
            >
              <Maximize2 className="w-3 h-3 text-muted-foreground hover:text-foreground" />
            </button>
          )}
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
      </div>

      {/* For terminal commands, show output directly */}
      {isTerminalCmd && (
        <div className="px-3 pb-2 pl-5">
          {tool.result !== undefined ? (
            <TruncatedOutput content={formatShellCommandResult(tool.result)} maxLines={10} />
          ) : "streamingOutput" in tool && tool.streamingOutput ? (
            <StreamingOutput content={tool.streamingOutput} maxHeight={200} />
          ) : (
            <span className="text-[10px] text-muted-foreground italic">
              {tool.status === "running" ? "Running..." : "Awaiting output"}
            </span>
          )}
        </div>
      )}
    </div>
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
