import {
  Bot,
  CheckCircle2,
  ChevronDown,
  ChevronRight,
  Clock,
  Copy,
  Loader2,
  X,
  XCircle,
} from "lucide-react";
import { useState } from "react";
import { Markdown } from "@/components/Markdown";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Collapsible, CollapsibleContent, CollapsibleTrigger } from "@/components/ui/collapsible";
import {
  Dialog,
  DialogClose,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { ScrollArea } from "@/components/ui/scroll-area";
import { logger } from "@/lib/logger";
import { cn } from "@/lib/utils";
import type { ActiveSubAgent, SubAgentToolCall } from "@/store";

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

/** Format duration in ms to human readable */
function formatDuration(ms?: number): string {
  if (!ms) return "";
  if (ms < 1000) return `${ms}ms`;
  return `${(ms / 1000).toFixed(1)}s`;
}

/** Individual tool call row with expandable details */
function ToolCallRow({ tool }: { tool: SubAgentToolCall }) {
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
      <CollapsibleTrigger className="group flex w-full items-center gap-1.5 rounded px-2 py-1.5 text-xs hover:bg-accent/50 transition-colors">
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
      <CollapsibleContent className="px-6 py-2">
        <div className="space-y-2 text-xs overflow-hidden">
          {/* Arguments */}
          <div className="overflow-hidden">
            <span className="text-muted-foreground font-medium">Arguments:</span>
            <pre className="mt-1 rounded-lg bg-muted/50 border border-border px-3 py-2 text-[11px] overflow-auto max-h-32 whitespace-pre-wrap break-all">
              {JSON.stringify(tool.args, null, 2)}
            </pre>
          </div>

          {/* Result (if available) */}
          {tool.result !== undefined && (
            <div className="overflow-hidden">
              <span className="text-muted-foreground font-medium">Result:</span>
              <pre className="mt-1 max-h-48 overflow-auto rounded-lg bg-muted/50 border border-border px-3 py-2 text-[11px] whitespace-pre-wrap break-all">
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
}

interface SubAgentDetailsModalProps {
  subAgent: ActiveSubAgent | null;
  onClose: () => void;
}

/** Status badge styling */
const statusStyles: Record<
  "running" | "completed" | "error",
  { badgeClass: string; label: string }
> = {
  running: {
    badgeClass: "bg-[var(--accent-dim)] text-accent",
    label: "Running",
  },
  completed: {
    badgeClass: "bg-[var(--success-dim)] text-[var(--success)]",
    label: "Completed",
  },
  error: {
    badgeClass: "bg-destructive/10 text-destructive",
    label: "Error",
  },
};

export function SubAgentDetailsModal({ subAgent, onClose }: SubAgentDetailsModalProps) {
  const [copiedSection, setCopiedSection] = useState<string | null>(null);

  if (!subAgent) return null;

  const status = statusStyles[subAgent.status];

  const handleCopy = async (content: string, section: string) => {
    try {
      await navigator.clipboard.writeText(content);
      setCopiedSection(section);
      setTimeout(() => setCopiedSection(null), 2000);
    } catch (error) {
      logger.error("Failed to copy:", error);
    }
  };

  return (
    <Dialog open={true} onOpenChange={onClose}>
      <DialogContent
        showCloseButton={false}
        className="!w-[calc(100%-2rem)] !h-[calc(100%-4rem)] !max-w-none !max-h-none !top-[calc(50%+1rem)] flex flex-col p-0 gap-0"
      >
        <DialogHeader className="px-6 pt-6 pb-4 border-b border-border">
          <div className="flex items-start justify-between gap-4">
            <div className="flex items-start gap-3 min-w-0 flex-1">
              <Bot className="w-5 h-5 text-[var(--ansi-magenta)] shrink-0 mt-0.5" />
              <div className="min-w-0 flex-1">
                <DialogTitle className="text-lg font-medium text-foreground">
                  {subAgent.agentName}
                </DialogTitle>
                <DialogDescription className="text-sm text-muted-foreground mt-1">
                  {subAgent.toolCalls.length} tool call{subAgent.toolCalls.length !== 1 ? "s" : ""}
                  {subAgent.durationMs !== undefined && ` • ${formatDuration(subAgent.durationMs)}`}
                </DialogDescription>
              </div>
            </div>
            <div className="flex items-center gap-2 shrink-0">
              <Badge
                variant="outline"
                className={cn("gap-1 flex items-center text-xs px-2 py-1", status.badgeClass)}
              >
                <StatusIcon status={subAgent.status} size="sm" />
                {status.label}
              </Badge>
              {subAgent.depth > 1 && (
                <Badge variant="outline" className="text-xs px-2 py-1">
                  depth {subAgent.depth}
                </Badge>
              )}
              <DialogClose asChild>
                <Button variant="ghost" size="icon" className="h-8 w-8 ml-2">
                  <X className="h-4 w-4" />
                  <span className="sr-only">Close</span>
                </Button>
              </DialogClose>
            </div>
          </div>
        </DialogHeader>

        <ScrollArea className="flex-1 min-h-0 overflow-hidden">
          <div className="px-6 py-4 space-y-6 w-full max-w-full overflow-hidden">
            {/* Task Section */}
            <div className="space-y-2">
              <div className="flex items-center justify-between">
                <h3 className="text-sm font-medium text-muted-foreground uppercase tracking-wide">
                  Task
                </h3>
                <Button
                  variant="ghost"
                  size="sm"
                  onClick={() => handleCopy(subAgent.task, "task")}
                  className="h-7 text-xs"
                >
                  <Copy className="w-3.5 h-3.5 mr-1" />
                  {copiedSection === "task" ? "Copied!" : "Copy"}
                </Button>
              </div>
              <div className="bg-muted/50 rounded-lg border border-border p-4 overflow-hidden">
                <p className="text-sm text-foreground/90 whitespace-pre-wrap break-words [overflow-wrap:anywhere]">
                  {subAgent.task}
                </p>
              </div>
            </div>

            {/* Metadata Section */}
            <div className="space-y-2">
              <h3 className="text-sm font-medium text-muted-foreground uppercase tracking-wide">
                Metadata
              </h3>
              <div className="bg-muted/50 rounded-lg border border-border p-4 space-y-2 overflow-hidden">
                <div className="flex items-center gap-2 text-sm min-w-0">
                  <Bot className="w-4 h-4 text-muted-foreground shrink-0" />
                  <span className="text-muted-foreground shrink-0">Agent ID:</span>
                  <span className="font-mono text-foreground/90 truncate">{subAgent.agentId}</span>
                </div>
                {subAgent.durationMs !== undefined && (
                  <div className="flex items-center gap-2 text-sm">
                    <Clock className="w-4 h-4 text-muted-foreground" />
                    <span className="text-muted-foreground">Duration:</span>
                    <span className="font-mono text-foreground/90">
                      {formatDuration(subAgent.durationMs)}
                    </span>
                  </div>
                )}
                <div className="flex items-center gap-2 text-sm">
                  <span className="text-muted-foreground ml-6">Depth:</span>
                  <span className="font-mono text-foreground/90">{subAgent.depth}</span>
                </div>
              </div>
            </div>

            {/* Tool Calls Section */}
            {subAgent.toolCalls.length > 0 && (
              <div className="space-y-2 overflow-hidden">
                <h3 className="text-sm font-medium text-muted-foreground uppercase tracking-wide">
                  Tool Calls ({subAgent.toolCalls.length})
                </h3>
                <div className="bg-muted/50 rounded-lg border border-border overflow-hidden">
                  <div className="divide-y divide-border overflow-hidden">
                    {subAgent.toolCalls.map((tool) => (
                      <ToolCallRow key={tool.id} tool={tool} />
                    ))}
                  </div>
                </div>
              </div>
            )}

            {/* Response Section */}
            {subAgent.response && (
              <div className="space-y-2 overflow-hidden">
                <div className="flex items-center justify-between">
                  <h3 className="text-sm font-medium text-muted-foreground uppercase tracking-wide">
                    Response
                  </h3>
                  <Button
                    variant="ghost"
                    size="sm"
                    onClick={() => handleCopy(subAgent.response ?? "", "response")}
                    className="h-7 text-xs"
                  >
                    <Copy className="w-3.5 h-3.5 mr-1" />
                    {copiedSection === "response" ? "Copied!" : "Copy"}
                  </Button>
                </div>
                <div className="bg-muted/50 rounded-lg border border-border p-4 overflow-hidden">
                  <Markdown
                    content={subAgent.response}
                    className="[overflow-wrap:anywhere] [&_*]:![overflow-wrap:anywhere] [&_pre]:whitespace-pre-wrap"
                  />
                </div>
              </div>
            )}

            {/* Error Section */}
            {subAgent.error && (
              <div className="space-y-2 overflow-hidden">
                <h3 className="text-sm font-medium text-destructive uppercase tracking-wide">
                  Error
                </h3>
                <div className="bg-destructive/10 rounded-lg border border-destructive/30 p-4 overflow-hidden">
                  <p className="text-sm text-destructive whitespace-pre-wrap break-words [overflow-wrap:anywhere]">
                    {subAgent.error}
                  </p>
                </div>
              </div>
            )}
          </div>
        </ScrollArea>
      </DialogContent>
    </Dialog>
  );
}
