import {
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
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  type AnyToolCall,
  computeToolGroupDuration,
  formatPrimaryArg,
  getGroupStatus,
  sortToolsByStartedAtDesc,
} from "@/lib/toolGrouping";
import { cn } from "@/lib/utils";

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

interface MainToolGroupProps {
  tools: AnyToolCall[];
  onViewToolDetails: (tool: AnyToolCall) => void;
  onViewGroupDetails: () => void;
}

/** Inline tool-group preview component for main-agent timeline */
export function MainToolGroup({
  tools,
  onViewToolDetails,
  onViewGroupDetails,
}: MainToolGroupProps) {
  const groupStatus = getGroupStatus(tools);
  const status = statusConfig[groupStatus];
  const StatusIcon = status.icon;
  const { label: durationLabel } = computeToolGroupDuration(tools);

  // Get 3 most recent tools (by startedAt desc)
  const sortedDesc = sortToolsByStartedAtDesc(tools);
  const previewTools = sortedDesc.slice(0, 3);
  const hiddenCount = tools.length - 3;

  return (
    <div
      className={cn(
        "border-l-[3px] border-r-0 border-t-0 border-b-0 overflow-hidden rounded-l-lg shadow-sm bg-muted/50",
        status.borderColor
      )}
    >
      {/* Header row */}
      <div className="flex items-center justify-between px-3 py-2">
        <div className="flex items-center gap-2">
          <StatusIcon className={cn("w-3.5 h-3.5", status.animate && "animate-spin")} />
          <Badge
            variant="outline"
            className="bg-muted/50 text-muted-foreground/60 border-muted-foreground/20 text-[10px] px-1.5 py-0 rounded-full"
          >
            {tools.length} tool{tools.length === 1 ? "" : "s"}
          </Badge>
          {durationLabel && (
            <span className="text-[10px] text-muted-foreground/70">{durationLabel}</span>
          )}
        </div>
      </div>

      {/* Preview of tools */}
      <div className="px-3 pb-2 space-y-0.5 pl-6">
        {hiddenCount > 0 && (
          <div className="flex items-center gap-2 text-[11px] text-muted-foreground/50 py-1">
            <span>
              ▸ {hiddenCount} previous tool call{hiddenCount === 1 ? "" : "s"}
            </span>
            <Button
              variant="ghost"
              size="sm"
              onClick={onViewGroupDetails}
              className="h-5 px-1.5 text-[10px]"
            >
              View all
            </Button>
          </div>
        )}
        {previewTools.map((tool) => (
          <ToolPreviewRow key={tool.id} tool={tool} onViewDetails={onViewToolDetails} />
        ))}
      </div>
    </div>
  );
}

/** Individual tool preview row */
function ToolPreviewRow({
  tool,
  onViewDetails,
}: {
  tool: AnyToolCall;
  onViewDetails: (tool: AnyToolCall) => void;
}) {
  const Icon = toolIcons[tool.name] || Terminal;
  const status = statusConfig[tool.status];
  const StatusIcon = status.icon;
  const primaryArg = formatPrimaryArg(tool);

  return (
    <div className="flex items-center justify-between py-1.5 px-2 rounded-md bg-background/50">
      <div className="flex items-center gap-2 min-w-0">
        <Icon className="w-3.5 h-3.5 text-muted-foreground shrink-0" />
        <span className="font-mono text-xs text-muted-foreground/70">{tool.name}</span>
        {primaryArg && (
          <span className="font-mono text-[11px] text-muted-foreground/60 truncate">
            {primaryArg}
          </span>
        )}
      </div>
      <div className="flex items-center gap-1.5 shrink-0">
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
    </div>
  );
}
