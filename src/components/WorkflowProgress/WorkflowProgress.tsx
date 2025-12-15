import { CheckCircle2, Circle, Loader2, Workflow, XCircle } from "lucide-react";
import { cn } from "@/lib/utils";
import { type ActiveWorkflow, useStore, type WorkflowStep } from "@/store";

interface WorkflowProgressProps {
  /** Session ID to fetch active workflow from store */
  sessionId?: string;
  /** Direct workflow data (for finalized messages) */
  workflow?: ActiveWorkflow;
}

function StepIcon({ status }: { status: WorkflowStep["status"] }) {
  switch (status) {
    case "completed":
      return <CheckCircle2 className="w-4 h-4 text-[var(--success)]" />;
    case "running":
      return <Loader2 className="w-4 h-4 text-accent animate-spin" />;
    case "error":
      return <XCircle className="w-4 h-4 text-destructive" />;
    default:
      return <Circle className="w-4 h-4 text-muted-foreground" />;
  }
}

function formatDuration(ms?: number): string {
  if (!ms) return "";
  if (ms < 1000) return `${ms}ms`;
  return `${(ms / 1000).toFixed(1)}s`;
}

function WorkflowStepItem({ step }: { step: WorkflowStep }) {
  return (
    <div className="flex items-center gap-2 py-1.5">
      <StepIcon status={step.status} />
      <span
        className={cn(
          "text-xs flex-1",
          step.status === "completed" && "text-[var(--success)]",
          step.status === "running" && "text-accent",
          step.status === "error" && "text-destructive",
          step.status === "pending" && "text-muted-foreground"
        )}
      >
        {step.name}
      </span>
      {step.durationMs !== undefined && (
        <span className="text-xs text-muted-foreground">{formatDuration(step.durationMs)}</span>
      )}
    </div>
  );
}

function WorkflowCard({ workflow }: { workflow: ActiveWorkflow }) {
  const progressPercent =
    workflow.totalSteps > 0
      ? Math.round(
          (workflow.steps.filter((s) => s.status === "completed").length / workflow.totalSteps) *
            100
        )
      : 0;

  return (
    <div className="bg-muted border border-[var(--border-medium)] rounded-lg p-3.5 space-y-2.5">
      {/* Header */}
      <div className="flex items-center justify-between pb-2.5 mb-2.5 border-b border-[var(--border-subtle)]">
        <div className="flex items-center gap-2">
          <Workflow className="w-4 h-4 text-accent" />
          <span className="font-medium text-xs text-foreground">{workflow.workflowName}</span>
        </div>
        <div
          className={cn(
            "px-2 py-0.5 text-[10px] font-medium rounded-full",
            workflow.status === "running" && "bg-[var(--accent-dim)] text-accent",
            workflow.status === "completed" && "bg-[var(--success-dim)] text-[var(--success)]",
            workflow.status === "error" && "bg-destructive/10 text-destructive"
          )}
        >
          {workflow.status === "running" && "Running"}
          {workflow.status === "completed" && "Completed"}
          {workflow.status === "error" && "Error"}
        </div>
      </div>

      {/* Progress bar */}
      {workflow.status === "running" && (
        <div className="space-y-1">
          <div className="flex items-center justify-between text-[10px] text-muted-foreground">
            <span>
              Step {workflow.currentStepIndex + 1} of {workflow.totalSteps}
            </span>
            <span>{progressPercent}%</span>
          </div>
          <div className="h-1.5 bg-background rounded-full overflow-hidden">
            <div
              className="h-full bg-accent transition-all duration-300"
              style={{ width: `${progressPercent}%` }}
            />
          </div>
        </div>
      )}

      {/* Steps list */}
      {workflow.steps.length > 0 && (
        <div className="space-y-1">
          {workflow.steps.map((step, index) => (
            <WorkflowStepItem key={`${step.name}-${index}`} step={step} />
          ))}
        </div>
      )}

      {/* Error message */}
      {workflow.error && (
        <div className="bg-destructive/10 border border-destructive/30 rounded-md p-2 text-xs text-destructive">
          {workflow.error}
        </div>
      )}

      {/* Duration */}
      {workflow.totalDurationMs !== undefined && (
        <div className="text-[10px] text-muted-foreground text-right">
          Total: {formatDuration(workflow.totalDurationMs)}
        </div>
      )}
    </div>
  );
}

export function WorkflowProgress({ sessionId, workflow: directWorkflow }: WorkflowProgressProps) {
  const storeWorkflow = useStore((state) =>
    sessionId ? state.activeWorkflows[sessionId] : undefined
  );

  // Use direct workflow if provided, otherwise fetch from store
  const workflow = directWorkflow ?? storeWorkflow;

  if (!workflow) {
    return null;
  }

  return (
    <div className="px-4 py-2">
      <WorkflowCard workflow={workflow} />
    </div>
  );
}
