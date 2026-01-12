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
  X,
  XCircle,
} from "lucide-react";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogClose,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { ScrollArea } from "@/components/ui/scroll-area";
import {
  type AnyToolCall,
  computeToolGroupDuration,
  formatPrimaryArg,
  sortToolsByStartedAtAsc,
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

/** Status configuration for icons */
const statusConfig: Record<
  AnyToolCall["status"],
  {
    icon: typeof CheckCircle;
    animate?: boolean;
  }
> = {
  pending: { icon: Loader2 },
  approved: { icon: CheckCircle },
  denied: { icon: XCircle },
  running: { icon: Loader2, animate: true },
  completed: { icon: CheckCircle },
  error: { icon: XCircle },
};

interface ToolGroupDetailsModalProps {
  tools: AnyToolCall[] | null;
  onClose: () => void;
  onViewToolDetails: (tool: AnyToolCall) => void;
}

/** Modal overlay listing all tool calls in a group */
export function ToolGroupDetailsModal({
  tools,
  onClose,
  onViewToolDetails,
}: ToolGroupDetailsModalProps) {
  if (!tools) return null;

  const { label: durationLabel } = computeToolGroupDuration(tools);
  const sortedTools = sortToolsByStartedAtAsc(tools);

  return (
    <Dialog open={!!tools} onOpenChange={(open) => !open && onClose()}>
      <DialogContent showCloseButton={false} className="!w-[calc(100%-2rem)] !h-[calc(100%-4rem)] !max-w-none !max-h-none !top-[calc(50%+1rem)] flex flex-col p-0 gap-0">
        {/* Header */}
        <DialogHeader className="px-6 pt-6 pb-4 border-b shrink-0">
          <div className="flex items-start justify-between">
            <div className="space-y-1">
              <DialogTitle className="text-lg font-semibold">Tool Call Group</DialogTitle>
              <DialogDescription className="text-sm">
                {tools.length} tool call{tools.length === 1 ? "" : "s"}
                {durationLabel && ` • ${durationLabel}`}
              </DialogDescription>
            </div>
            <DialogClose asChild>
              <Button
                variant="ghost"
                size="icon"
                className="h-8 w-8 rounded-full"
                onClick={onClose}
              >
                <X className="h-4 w-4" />
              </Button>
            </DialogClose>
          </div>
        </DialogHeader>

        {/* Body - scrollable list of tools */}
        <ScrollArea className="flex-1 px-6">
          <div className="space-y-1 py-4">
            {sortedTools.map((tool) => (
              <ToolRow key={tool.id} tool={tool} onViewDetails={onViewToolDetails} />
            ))}
          </div>
        </ScrollArea>
      </DialogContent>
    </Dialog>
  );
}

/** Individual tool row in the modal */
function ToolRow({
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

  // Calculate duration if available
  let duration: string | null = null;
  if ("startedAt" in tool && "completedAt" in tool && tool.startedAt && tool.completedAt) {
    const durationMs = new Date(tool.completedAt).getTime() - new Date(tool.startedAt).getTime();
    if (durationMs < 1000) {
      duration = `${durationMs}ms`;
    } else {
      duration = `${(durationMs / 1000).toFixed(1)}s`;
    }
  }

  return (
    <button
      type="button"
      onClick={() => onViewDetails(tool)}
      className="flex w-full items-center justify-between py-2.5 px-3 rounded-md hover:bg-accent/50 cursor-pointer transition-colors text-left"
    >
      <div className="flex items-center gap-2.5 min-w-0 flex-1">
        <Icon className="w-4 h-4 text-muted-foreground shrink-0" />
        <span className="font-mono text-sm text-foreground">{tool.name}</span>
        {primaryArg && (
          <span className="font-mono text-xs text-muted-foreground/70 truncate">{primaryArg}</span>
        )}
        {duration && (
          <span className="text-[10px] text-muted-foreground/60 ml-auto shrink-0">{duration}</span>
        )}
      </div>
      <div className="flex items-center gap-2 shrink-0 ml-2">
        <button
          type="button"
          onClick={(e) => {
            e.stopPropagation();
            onViewDetails(tool);
          }}
          className="p-1 hover:bg-[var(--bg-hover)] rounded transition-colors"
          title="View details"
        >
          <Maximize2 className="w-3.5 h-3.5 text-muted-foreground hover:text-foreground" />
        </button>
        <StatusIcon
          className={cn(
            "w-4 h-4",
            status.animate && "animate-spin",
            tool.status === "completed" && "text-[var(--success)]",
            tool.status === "running" && "text-accent",
            tool.status === "error" && "text-destructive",
            tool.status === "pending" && "text-muted-foreground"
          )}
        />
      </div>
    </button>
  );
}
