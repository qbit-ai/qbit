import Ansi from "ansi-to-react";
import {
  AlertTriangle,
  CheckCircle,
  Clock,
  Copy,
  Loader2,
  Shield,
  ShieldCheck,
  Terminal,
  XCircle,
} from "lucide-react";
import { useState } from "react";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { ScrollArea } from "@/components/ui/scroll-area";
import { type AnyToolCall, formatPrimaryArg } from "@/lib/toolGrouping";
import {
  formatToolName,
  formatToolResult,
  getRiskLevel,
  isAgentTerminalCommand,
} from "@/lib/tools";
import { cn } from "@/lib/utils";
import type { RiskLevel } from "@/store";

interface ToolDetailsModalProps {
  tool: AnyToolCall | null;
  onClose: () => void;
}

// Tool icons mapping
const toolIcons: Record<string, typeof Terminal> = {
  read_file: Terminal,
  write_file: Terminal,
  edit_file: Terminal,
  list_files: Terminal,
  grep_file: Terminal,
  run_pty_cmd: Terminal,
  shell: Terminal,
  web_fetch: Terminal,
  web_search: Terminal,
  web_search_answer: Terminal,
  apply_patch: Terminal,
};

// Risk level styling
const RISK_STYLES: Record<RiskLevel, { color: string; bg: string; icon: typeof Shield }> = {
  low: { color: "text-[#9ece6a]", bg: "bg-[#9ece6a]/10", icon: ShieldCheck },
  medium: { color: "text-[#7aa2f7]", bg: "bg-[#7aa2f7]/10", icon: Shield },
  high: { color: "text-[#e0af68]", bg: "bg-[#e0af68]/10", icon: AlertTriangle },
  critical: { color: "text-[#f7768e]", bg: "bg-[#f7768e]/10", icon: AlertTriangle },
};

// Status configuration
const statusConfig: Record<
  AnyToolCall["status"],
  {
    icon: typeof CheckCircle;
    badgeClass: string;
    label: string;
    animate?: boolean;
  }
> = {
  pending: {
    icon: Loader2,
    badgeClass: "bg-muted text-muted-foreground hover:bg-muted/80",
    label: "Pending",
  },
  approved: {
    icon: CheckCircle,
    badgeClass: "bg-[var(--success-dim)] text-[var(--success)] hover:bg-[var(--success)]/20",
    label: "Approved",
  },
  denied: {
    icon: XCircle,
    badgeClass: "bg-destructive/10 text-destructive hover:bg-destructive/20",
    label: "Denied",
  },
  running: {
    icon: Loader2,
    badgeClass: "bg-[var(--accent-dim)] text-accent",
    label: "Running",
    animate: true,
  },
  completed: {
    icon: CheckCircle,
    badgeClass: "bg-[var(--success-dim)] text-[var(--success)] hover:bg-[var(--success)]/20",
    label: "Completed",
  },
  error: {
    icon: XCircle,
    badgeClass: "bg-destructive/10 text-destructive hover:bg-destructive/20",
    label: "Error",
  },
};

export function ToolDetailsModal({ tool, onClose }: ToolDetailsModalProps) {
  const [copiedSection, setCopiedSection] = useState<string | null>(null);

  if (!tool) return null;

  const Icon = toolIcons[tool.name] || Terminal;
  const riskLevel =
    "riskLevel" in tool && tool.riskLevel ? tool.riskLevel : getRiskLevel(tool.name);
  const { color: riskColor, bg: riskBg, icon: RiskIcon } = RISK_STYLES[riskLevel];
  const status = statusConfig[tool.status];
  const StatusIcon = status.icon;
  const isTerminalCmd = isAgentTerminalCommand(tool);
  const primaryArg = formatPrimaryArg(tool);

  // Calculate duration if we have start and end times
  const duration =
    "startedAt" in tool && tool.startedAt && "completedAt" in tool && tool.completedAt
      ? new Date(tool.completedAt).getTime() - new Date(tool.startedAt).getTime()
      : null;

  const handleCopy = async (content: string, section: string) => {
    try {
      await navigator.clipboard.writeText(content);
      setCopiedSection(section);
      setTimeout(() => setCopiedSection(null), 2000);
    } catch (error) {
      console.error("Failed to copy:", error);
    }
  };

  const argsString = JSON.stringify(tool.args, null, 2);
  const resultString = formatToolResult(tool.result);

  return (
    <Dialog open={true} onOpenChange={onClose}>
      <DialogContent className="max-w-4xl max-h-[85vh] flex flex-col p-0 gap-0">
        <DialogHeader className="px-6 pt-6 pb-4 border-b border-border">
          <div className="flex items-start justify-between gap-4">
            <div className="flex items-start gap-3 min-w-0 flex-1">
              <Icon className="w-5 h-5 text-muted-foreground shrink-0 mt-0.5" />
              <div className="min-w-0 flex-1">
                <DialogTitle className="text-lg font-mono text-foreground">
                  {formatToolName(tool.name)}
                </DialogTitle>
                {primaryArg && (
                  <DialogDescription className="text-sm text-muted-foreground font-mono mt-1">
                    {primaryArg}
                  </DialogDescription>
                )}
              </div>
            </div>
            <div className="flex items-center gap-2 shrink-0">
              <Badge
                variant="outline"
                className={cn("gap-1 flex items-center text-xs px-2 py-1", status.badgeClass)}
              >
                <StatusIcon className={cn("w-3.5 h-3.5", status.animate && "animate-spin")} />
                {status.label}
              </Badge>
              <Badge
                variant="outline"
                className={cn("text-xs px-2 py-1 capitalize", riskBg, riskColor)}
              >
                <RiskIcon className="w-3.5 h-3.5 mr-1" />
                {riskLevel}
              </Badge>
            </div>
          </div>
        </DialogHeader>

        <ScrollArea className="flex-1 px-6 py-4">
          <div className="space-y-6">
            {/* Metadata Section */}
            <div className="space-y-2">
              <h3 className="text-sm font-medium text-muted-foreground uppercase tracking-wide">
                Metadata
              </h3>
              <div className="bg-muted/50 rounded-lg p-4 space-y-2">
                <div className="flex items-center gap-2 text-sm">
                  <Terminal className="w-4 h-4 text-muted-foreground" />
                  <span className="text-muted-foreground">Tool ID:</span>
                  <span className="font-mono text-foreground/90">{tool.id}</span>
                </div>
                {"startedAt" in tool && tool.startedAt && (
                  <div className="flex items-center gap-2 text-sm">
                    <Clock className="w-4 h-4 text-muted-foreground" />
                    <span className="text-muted-foreground">Started:</span>
                    <span className="font-mono text-foreground/90">
                      {new Date(tool.startedAt).toLocaleString()}
                    </span>
                  </div>
                )}
                {"completedAt" in tool && tool.completedAt && (
                  <div className="flex items-center gap-2 text-sm">
                    <Clock className="w-4 h-4 text-muted-foreground" />
                    <span className="text-muted-foreground">Completed:</span>
                    <span className="font-mono text-foreground/90">
                      {new Date(tool.completedAt).toLocaleString()}
                    </span>
                  </div>
                )}
                {duration !== null && (
                  <div className="flex items-center gap-2 text-sm">
                    <Clock className="w-4 h-4 text-muted-foreground" />
                    <span className="text-muted-foreground">Duration:</span>
                    <span className="font-mono text-foreground/90">
                      {duration < 1000 ? `${duration}ms` : `${(duration / 1000).toFixed(2)}s`}
                    </span>
                  </div>
                )}
              </div>
            </div>

            {/* Arguments Section */}
            {Object.keys(tool.args).length > 0 && (
              <div className="space-y-2">
                <div className="flex items-center justify-between">
                  <h3 className="text-sm font-medium text-muted-foreground uppercase tracking-wide">
                    Arguments
                  </h3>
                  <Button
                    variant="ghost"
                    size="sm"
                    onClick={() => handleCopy(argsString, "args")}
                    className="h-7 text-xs"
                  >
                    <Copy className="w-3.5 h-3.5 mr-1" />
                    {copiedSection === "args" ? "Copied!" : "Copy"}
                  </Button>
                </div>
                <pre className="bg-background border border-border rounded-lg p-4 overflow-auto text-xs font-mono text-foreground/90 whitespace-pre-wrap break-all">
                  {argsString}
                </pre>
              </div>
            )}

            {/* Result Section */}
            {tool.result !== undefined && tool.status !== "running" && (
              <div className="space-y-2">
                <div className="flex items-center justify-between">
                  <h3 className="text-sm font-medium text-muted-foreground uppercase tracking-wide">
                    {tool.status === "error" ? "Error" : "Result"}
                  </h3>
                  <Button
                    variant="ghost"
                    size="sm"
                    onClick={() => handleCopy(resultString, "result")}
                    className="h-7 text-xs"
                  >
                    <Copy className="w-3.5 h-3.5 mr-1" />
                    {copiedSection === "result" ? "Copied!" : "Copy"}
                  </Button>
                </div>
                {isTerminalCmd ? (
                  <pre
                    className={cn(
                      "ansi-output bg-background border border-border rounded-lg p-4 overflow-auto text-xs whitespace-pre-wrap break-all",
                      tool.status === "error" ? "text-destructive" : "text-[var(--ansi-cyan)]"
                    )}
                  >
                    <Ansi useClasses>{resultString}</Ansi>
                  </pre>
                ) : (
                  <pre
                    className={cn(
                      "bg-background border border-border rounded-lg p-4 overflow-auto text-xs font-mono whitespace-pre-wrap break-all",
                      tool.status === "error" ? "text-destructive" : "text-foreground/90"
                    )}
                  >
                    {resultString}
                  </pre>
                )}
              </div>
            )}
          </div>
        </ScrollArea>
      </DialogContent>
    </Dialog>
  );
}
