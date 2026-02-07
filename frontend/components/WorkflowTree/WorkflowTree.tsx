import {
  CheckCircle2,
  ChevronDown,
  ChevronRight,
  Circle,
  Loader2,
  Terminal,
  Workflow,
  XCircle,
} from "lucide-react";
import { memo, useMemo, useState } from "react";
import { Badge } from "@/components/ui/badge";
import { Collapsible, CollapsibleContent, CollapsibleTrigger } from "@/components/ui/collapsible";
import { cn } from "@/lib/utils";
import { type ActiveToolCall, useStore, type WorkflowStep } from "@/store";

const EMPTY_TOOL_CALLS: ActiveToolCall[] = [];

interface WorkflowTreeProps {
  sessionId: string;
}

/** Status icon component */
function StatusIcon({
  status,
  size = "md",
}: {
  status: "pending" | "running" | "completed" | "error";
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
    default:
      return <Circle className={cn(sizeClass, "text-muted-foreground")} />;
  }
}

/** Status badge component */
function StatusBadge({ status }: { status: "idle" | "running" | "completed" | "error" }) {
  const config = {
    idle: { bg: "bg-muted", text: "text-muted-foreground", label: "Idle" },
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

/** Group tool calls by step index */
function groupToolCallsByStepIndex(
  toolCalls: ActiveToolCall[],
  workflowId: string
): Map<number, ActiveToolCall[]> {
  const groups = new Map<number, ActiveToolCall[]>();

  for (const tool of toolCalls) {
    const source = tool.source;
    if (!source || source.type !== "workflow" || source.workflowId !== workflowId) {
      continue;
    }

    // Use step index for grouping (default to 0 if not specified)
    const stepIndex = source.stepIndex ?? 0;
    const existing = groups.get(stepIndex) || [];
    existing.push(tool);
    groups.set(stepIndex, existing);
  }

  return groups;
}

/** Individual tool call row (simplified) */
const ToolCallRow = memo(function ToolCallRow({ tool }: { tool: ActiveToolCall }) {
  const status =
    tool.status === "completed" ? "completed" : tool.status === "error" ? "error" : "running";

  // Get primary argument for display
  const primaryArg = (() => {
    const args = tool.args;
    // Common patterns for primary arguments
    if ("command" in args && typeof args.command === "string") return args.command;
    if ("path" in args && typeof args.path === "string") return args.path;
    if ("file_path" in args && typeof args.file_path === "string") return args.file_path;
    if ("query" in args && typeof args.query === "string") return args.query;
    return null;
  })();

  return (
    <div className="flex items-center gap-2 py-0.5 pl-4 text-xs">
      <ChevronRight className="w-2.5 h-2.5 text-muted-foreground" />
      <Terminal className="w-3 h-3 text-[var(--ansi-blue)]" />
      <span className="font-mono text-[var(--ansi-cyan)] truncate flex-1">
        {primaryArg || tool.name}
      </span>
      <StatusIcon status={status} size="sm" />
    </div>
  );
});

/** Tool group within a step (collapsed view with count) */
const StepToolGroup = memo(function StepToolGroup({
  toolName,
  tools,
}: {
  toolName: string;
  tools: ActiveToolCall[];
}) {
  const [isExpanded, setIsExpanded] = useState(false);

  // Determine group status
  const hasRunning = tools.some((t) => t.status === "running");
  const hasError = tools.some((t) => t.status === "error");
  const allCompleted = tools.every((t) => t.status === "completed");
  const groupStatus = hasError
    ? "error"
    : hasRunning
      ? "running"
      : allCompleted
        ? "completed"
        : "running";

  // Get preview of primary arguments
  const previews = tools
    .map((t) => {
      const args = t.args;
      if ("command" in args && typeof args.command === "string") return args.command;
      if ("path" in args && typeof args.path === "string") return args.path;
      if ("file_path" in args && typeof args.file_path === "string") return args.file_path;
      return null;
    })
    .filter((p): p is string => p !== null)
    .slice(0, 3);

  return (
    <Collapsible open={isExpanded} onOpenChange={setIsExpanded}>
      <CollapsibleTrigger asChild>
        <button
          type="button"
          className="flex items-center gap-2 py-1 pl-2 w-full text-left hover:bg-card/50 rounded cursor-pointer"
        >
          <ChevronRight
            className={cn(
              "w-3 h-3 text-muted-foreground transition-transform",
              isExpanded && "rotate-90"
            )}
          />
          <Terminal className="w-3.5 h-3.5 text-[var(--ansi-blue)]" />
          <span className="font-mono text-xs text-foreground">{toolName}</span>
          <Badge
            variant="outline"
            className="bg-[var(--ansi-blue)]/10 text-[var(--ansi-blue)] border-[var(--ansi-blue)]/30 text-[9px] px-1 py-0"
          >
            x{tools.length}
          </Badge>
          <span className="flex-1" />
          <StatusIcon status={groupStatus} size="sm" />
        </button>
      </CollapsibleTrigger>

      {/* Preview when collapsed */}
      {!isExpanded && previews.length > 0 && (
        <div className="pl-8 pb-1">
          <span className="text-[10px] text-muted-foreground font-mono">
            {previews.join(", ")}
            {tools.length > 3 && (
              <span className="text-[var(--ansi-blue)]"> +{tools.length - 3} more</span>
            )}
          </span>
        </div>
      )}

      <CollapsibleContent>
        <div className="pl-4 space-y-0.5">
          {tools.map((tool) => (
            <ToolCallRow key={tool.id} tool={tool} />
          ))}
        </div>
      </CollapsibleContent>
    </Collapsible>
  );
});

/** Step node in the tree */
const StepNode = memo(function StepNode({
  step,
  toolCalls,
  isLast,
}: {
  step: WorkflowStep;
  toolCalls: ActiveToolCall[];
  isLast: boolean;
}) {
  const [isExpanded, setIsExpanded] = useState(step.status === "running");

  // Group tool calls by name for this step
  const toolGroups = useMemo(() => {
    const groups = new Map<string, ActiveToolCall[]>();
    for (const tool of toolCalls) {
      const existing = groups.get(tool.name) || [];
      existing.push(tool);
      groups.set(tool.name, existing);
    }
    return Array.from(groups.entries());
  }, [toolCalls]);

  const hasToolCalls = toolCalls.length > 0;

  return (
    <div className={cn("relative", !isLast && "pb-1")}>
      {/* Vertical connector line */}
      {!isLast && <div className="absolute left-[11px] top-6 bottom-0 w-px bg-[#3b4261]" />}

      <Collapsible open={isExpanded} onOpenChange={setIsExpanded}>
        <CollapsibleTrigger asChild>
          <button
            type="button"
            className="flex items-center gap-2 py-1 w-full text-left hover:bg-[#1f2335]/50 rounded cursor-pointer"
          >
            {hasToolCalls ? (
              <ChevronRight
                className={cn(
                  "w-3.5 h-3.5 text-[#565f89] transition-transform",
                  isExpanded && "rotate-90"
                )}
              />
            ) : (
              <div className="w-3.5" />
            )}
            <StatusIcon status={step.status} />
            <span
              className={cn(
                "text-sm",
                step.status === "completed" && "text-[#9ece6a]",
                step.status === "running" && "text-[#7aa2f7]",
                step.status === "error" && "text-[#f7768e]",
                step.status === "pending" && "text-[#565f89]"
              )}
            >
              Step {step.index + 1}: {step.name}
            </span>
            {step.durationMs !== undefined && (
              <span className="text-[10px] text-[#565f89]">{formatDuration(step.durationMs)}</span>
            )}
          </button>
        </CollapsibleTrigger>

        {hasToolCalls && (
          <CollapsibleContent>
            <div className="pl-6 space-y-0.5 border-l border-[#3b4261] ml-[11px]">
              {toolGroups.map(([toolName, tools]) => (
                <StepToolGroup key={toolName} toolName={toolName} tools={tools} />
              ))}
            </div>
          </CollapsibleContent>
        )}
      </Collapsible>
    </div>
  );
});

/** Main workflow tree component */
export const WorkflowTree = memo(function WorkflowTree({ sessionId }: WorkflowTreeProps) {
  const activeWorkflow = useStore((state) => state.activeWorkflows[sessionId]);
  const activeToolCalls = useStore((state) => state.activeToolCalls[sessionId] ?? EMPTY_TOOL_CALLS);
  const [isExpanded, setIsExpanded] = useState(true);

  // Combine active tool calls with preserved workflow tool calls
  // Active tool calls are used during streaming, preserved ones after completion
  const allToolCalls = useMemo(() => {
    if (!activeWorkflow) return [];
    const preserved = activeWorkflow.toolCalls || [];
    // Deduplicate by ID (active calls take precedence for most up-to-date status)
    const byId = new Map(preserved.map((t) => [t.id, t]));
    for (const tool of activeToolCalls) {
      byId.set(tool.id, tool);
    }
    return Array.from(byId.values());
  }, [activeToolCalls, activeWorkflow]);

  // Group tool calls by step index
  const toolCallsByStepIndex = useMemo(
    () =>
      activeWorkflow
        ? groupToolCallsByStepIndex(allToolCalls, activeWorkflow.workflowId)
        : new Map(),
    [allToolCalls, activeWorkflow]
  );

  if (!activeWorkflow) {
    return null;
  }

  return (
    <div className="bg-[#1a1b26] border border-[#3b4261] rounded-lg overflow-hidden">
      {/* Workflow header */}
      <Collapsible open={isExpanded} onOpenChange={setIsExpanded}>
        <CollapsibleTrigger asChild>
          <button
            type="button"
            className="flex items-center gap-2 p-3 w-full text-left hover:bg-[#1f2335]/50 cursor-pointer"
          >
            <ChevronDown
              className={cn(
                "w-4 h-4 text-[#565f89] transition-transform",
                !isExpanded && "-rotate-90"
              )}
            />
            <Terminal className="w-4 h-4 text-[#bb9af7]" />
            <span className="font-mono text-sm text-[#c0caf5]">run_workflow</span>
            <span className="text-[#565f89]">-</span>
            <Workflow className="w-4 h-4 text-[#9ece6a]" />
            <span className="font-medium text-[#c0caf5]">{activeWorkflow.workflowName}</span>
            <span className="flex-1" />
            <StatusBadge status={activeWorkflow.status} />
          </button>
        </CollapsibleTrigger>

        <CollapsibleContent>
          <div className="px-3 pb-3 space-y-1">
            {activeWorkflow.steps.map((step, index) => (
              <StepNode
                key={`${step.name}-${index}`}
                step={step}
                toolCalls={toolCallsByStepIndex.get(step.index) || []}
                isLast={index === activeWorkflow.steps.length - 1}
              />
            ))}

            {/* Error message */}
            {activeWorkflow.error && (
              <div className="mt-2 bg-[#f7768e]/10 border border-[#f7768e]/30 rounded-md p-2 text-sm text-[#f7768e]">
                {activeWorkflow.error}
              </div>
            )}

            {/* Duration */}
            {activeWorkflow.totalDurationMs !== undefined && (
              <div className="mt-2 text-xs text-[#565f89] text-right">
                Total: {formatDuration(activeWorkflow.totalDurationMs)}
              </div>
            )}
          </div>
        </CollapsibleContent>
      </Collapsible>
    </div>
  );
});

export default WorkflowTree;
