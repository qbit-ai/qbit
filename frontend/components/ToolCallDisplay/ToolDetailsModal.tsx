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
  X,
  XCircle,
} from "lucide-react";
import { useState } from "react";
import { Prism as SyntaxHighlighter } from "react-syntax-highlighter";
import { DiffView } from "@/components/DiffView";
import { Badge } from "@/components/ui/badge";
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
import { logger } from "@/lib/logger";
import { type AnyToolCall, formatPrimaryArg } from "@/lib/toolGrouping";
import {
  formatToolName,
  formatToolResult,
  getRiskLevel,
  isAgentTerminalCommand,
  isEditFileResult,
} from "@/lib/tools";
import { cn } from "@/lib/utils";
import type { RiskLevel } from "@/store";

/**
 * Custom syntax highlighting theme that matches the app's color palette.
 * Clean, minimal styling without token backgrounds.
 */
const jsonTheme: Record<string, React.CSSProperties> = {
  'code[class*="language-"]': {
    color: "#c0caf5",
    fontFamily: "var(--font-mono), ui-monospace, monospace",
    fontSize: "0.75rem",
    lineHeight: "1.6",
  },
  'pre[class*="language-"]': {
    color: "#c0caf5",
    margin: 0,
    padding: "1rem",
    overflow: "auto",
  },
  // Property names (keys)
  property: {
    color: "#7dcfff",
  },
  // String values
  string: {
    color: "#9ece6a",
  },
  // Numbers
  number: {
    color: "#ff9e64",
  },
  // Booleans and null
  boolean: {
    color: "#bb9af7",
  },
  null: {
    color: "#bb9af7",
  },
  // Punctuation (braces, brackets, colons, commas)
  punctuation: {
    color: "#545c7e",
  },
  operator: {
    color: "#89ddff",
  },
};

/** Check if content looks like JSON */
function isJsonContent(content: string): boolean {
  const trimmed = content.trim();
  if (
    (trimmed.startsWith("{") && trimmed.endsWith("}")) ||
    (trimmed.startsWith("[") && trimmed.endsWith("]"))
  ) {
    try {
      JSON.parse(trimmed);
      return true;
    } catch {
      return false;
    }
  }
  return false;
}

/** Code block with optional JSON syntax highlighting */
function CodeBlock({
  content,
  maxHeight = "16rem",
  isError = false,
}: {
  content: string;
  maxHeight?: string;
  isError?: boolean;
}) {
  const isJson = isJsonContent(content);

  if (isJson) {
    return (
      <div
        className="overflow-auto rounded-lg border border-border bg-background"
        style={{ maxHeight }}
      >
        <SyntaxHighlighter
          style={jsonTheme}
          language="json"
          PreTag="div"
          customStyle={{
            margin: 0,
            padding: "1rem",
            background: "transparent",
          }}
          wrapLongLines
        >
          {content}
        </SyntaxHighlighter>
      </div>
    );
  }

  return (
    <pre
      className={cn(
        "bg-background border border-border rounded-lg p-4 overflow-auto text-xs font-mono whitespace-pre-wrap break-all",
        isError ? "text-destructive" : "text-foreground/90"
      )}
      style={{ maxHeight }}
    >
      {content}
    </pre>
  );
}

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
    badgeClass: "bg-[var(--color-success-dim)] text-[var(--color-success)] hover:bg-[var(--color-success)]/20",
    label: "Approved",
  },
  denied: {
    icon: XCircle,
    badgeClass: "bg-destructive/10 text-destructive hover:bg-destructive/20",
    label: "Denied",
  },
  running: {
    icon: Loader2,
    badgeClass: "bg-[var(--color-accent-dim)] text-accent",
    label: "Running",
    animate: true,
  },
  completed: {
    icon: CheckCircle,
    badgeClass: "bg-[var(--color-success-dim)] text-[var(--color-success)] hover:bg-[var(--color-success)]/20",
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
      logger.error("Failed to copy:", error);
    }
  };

  const argsString = JSON.stringify(tool.args, null, 2);
  const resultString = formatToolResult(tool.result);

  return (
    <Dialog open={true} onOpenChange={onClose}>
      <DialogContent
        showCloseButton={false}
        className="!w-[calc(100%-2rem)] !h-[calc(100%-4rem)] !max-w-none !max-h-none !top-[calc(50%+1rem)] flex flex-col p-0 gap-0"
      >
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
              <DialogClose asChild>
                <Button variant="ghost" size="icon" className="h-8 w-8 ml-2">
                  <X className="h-4 w-4" />
                  <span className="sr-only">Close</span>
                </Button>
              </DialogClose>
            </div>
          </div>
        </DialogHeader>

        <ScrollArea className="flex-1 min-h-0 px-6 py-4">
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
                <CodeBlock content={argsString} maxHeight="12rem" />
              </div>
            )}

            {/* Result Section */}
            {tool.result !== undefined && (
              <div className="space-y-2">
                <div className="flex items-center justify-between">
                  <h3 className="text-sm font-medium text-muted-foreground uppercase tracking-wide">
                    {tool.status === "error" ? "Error" : "Result"}
                  </h3>
                  {!(tool.name === "edit_file" && isEditFileResult(tool.result)) && (
                    <Button
                      variant="ghost"
                      size="sm"
                      onClick={() => handleCopy(resultString, "result")}
                      className="h-7 text-xs"
                    >
                      <Copy className="w-3.5 h-3.5 mr-1" />
                      {copiedSection === "result" ? "Copied!" : "Copy"}
                    </Button>
                  )}
                </div>
                {tool.name === "edit_file" && isEditFileResult(tool.result) ? (
                  <div className="max-h-80 overflow-auto">
                    <DiffView
                      diff={tool.result.diff}
                      filePath={tool.result.path}
                      className="border border-border rounded-lg overflow-hidden"
                    />
                  </div>
                ) : isTerminalCmd ? (
                  <pre
                    className={cn(
                      "ansi-output bg-background border border-border rounded-lg p-4 overflow-auto text-xs whitespace-pre-wrap break-all",
                      tool.status === "error" ? "text-destructive" : "text-[var(--color-ansi-cyan)]"
                    )}
                    style={{ maxHeight: "20rem" }}
                  >
                    <Ansi useClasses>{resultString}</Ansi>
                  </pre>
                ) : (
                  <CodeBlock
                    content={resultString}
                    maxHeight="20rem"
                    isError={tool.status === "error"}
                  />
                )}
              </div>
            )}
          </div>
        </ScrollArea>
      </DialogContent>
    </Dialog>
  );
}
