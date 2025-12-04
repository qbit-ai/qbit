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
      return <CheckCircle2 className="w-4 h-4 text-[#9ece6a]" />;
    case "running":
      return <Loader2 className="w-4 h-4 text-[#7aa2f7] animate-spin" />;
    case "error":
      return <XCircle className="w-4 h-4 text-[#f7768e]" />;
    default:
      return <Circle className="w-4 h-4 text-[#565f89]" />;
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
          "text-sm flex-1",
          step.status === "completed" && "text-[#9ece6a]",
          step.status === "running" && "text-[#7aa2f7]",
          step.status === "error" && "text-[#f7768e]",
          step.status === "pending" && "text-[#565f89]"
        )}
      >
        {step.name}
      </span>
      {step.durationMs !== undefined && (
        <span className="text-xs text-[#565f89]">{formatDuration(step.durationMs)}</span>
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
    <div className="bg-[#1f2335] border border-[#3b4261] rounded-lg p-4 space-y-3">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <Workflow className="w-5 h-5 text-[#bb9af7]" />
          <span className="font-medium text-[#c0caf5]">{workflow.workflowName}</span>
        </div>
        <div
          className={cn(
            "px-2 py-0.5 text-xs font-medium rounded-full",
            workflow.status === "running" && "bg-[#7aa2f7]/20 text-[#7aa2f7]",
            workflow.status === "completed" && "bg-[#9ece6a]/20 text-[#9ece6a]",
            workflow.status === "error" && "bg-[#f7768e]/20 text-[#f7768e]"
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
          <div className="flex items-center justify-between text-xs text-[#565f89]">
            <span>
              Step {workflow.currentStepIndex + 1} of {workflow.totalSteps}
            </span>
            <span>{progressPercent}%</span>
          </div>
          <div className="h-1.5 bg-[#16161e] rounded-full overflow-hidden">
            <div
              className="h-full bg-[#7aa2f7] transition-all duration-300"
              style={{ width: `${progressPercent}%` }}
            />
          </div>
        </div>
      )}

      {/* Steps list */}
      {workflow.steps.length > 0 && (
        <div className="border-t border-[#3b4261] pt-3 space-y-1">
          {workflow.steps.map((step, index) => (
            <WorkflowStepItem key={`${step.name}-${index}`} step={step} />
          ))}
        </div>
      )}

      {/* Error message */}
      {workflow.error && (
        <div className="bg-[#f7768e]/10 border border-[#f7768e]/30 rounded-md p-2 text-sm text-[#f7768e]">
          {workflow.error}
        </div>
      )}

      {/* Duration */}
      {workflow.totalDurationMs !== undefined && (
        <div className="text-xs text-[#565f89] text-right">
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
