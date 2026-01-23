import { CheckCircle2, ChevronDown, ChevronRight, Circle, Loader2 } from "lucide-react";
import { memo, useState } from "react";
import { Collapsible, CollapsibleContent, CollapsibleTrigger } from "@/components/ui/collapsible";
import { cn } from "@/lib/utils";
import { useStore } from "@/store";

interface InlineTaskPlanProps {
  sessionId: string;
  className?: string;
}

/**
 * Inline Task Plan component that displays above the path/git badges in UnifiedInput.
 * Shows a compact progress bar when collapsed, and full step list when expanded.
 * Only renders when a plan exists for the session.
 */
export const InlineTaskPlan = memo(function InlineTaskPlan({
  sessionId,
  className,
}: InlineTaskPlanProps) {
  const plan = useStore((state) => state.sessions[sessionId]?.plan);
  const [isExpanded, setIsExpanded] = useState(false);

  const isPlanComplete =
    !!plan && plan.summary.total > 0 && plan.summary.completed === plan.summary.total;

  // Don't render if no plan, empty steps, or plan is complete
  if (!plan || plan.steps.length === 0 || isPlanComplete) {
    return null;
  }

  const { summary, steps, explanation } = plan;
  const progressPercentage = summary.total > 0 ? (summary.completed / summary.total) * 100 : 0;

  return (
    <Collapsible open={isExpanded} onOpenChange={setIsExpanded}>
      <div className={cn("border-b border-[var(--border-subtle)]", className)}>
        <CollapsibleTrigger className="w-full">
          <div className="flex items-center gap-2 px-4 py-1.5 hover:bg-accent/30 transition-colors">
            {/* Chevron */}
            {isExpanded ? (
              <ChevronDown className="w-3.5 h-3.5 text-[#7aa2f7] flex-shrink-0" />
            ) : (
              <ChevronRight className="w-3.5 h-3.5 text-[#7aa2f7] flex-shrink-0" />
            )}

            {/* Label */}
            <span className="text-xs font-medium text-foreground">Task Plan</span>

            {/* Progress bar (inline, compact) */}
            <div className="flex-1 mx-2 h-1.5 bg-muted/30 rounded-full overflow-hidden max-w-[200px]">
              <div
                className="h-full bg-[#7aa2f7] transition-all duration-300 ease-out"
                style={{ width: `${progressPercentage}%` }}
              />
            </div>

            {/* Step count and percentage */}
            <span className="text-xs text-muted-foreground">
              {summary.completed}/{summary.total} steps
            </span>
            <span className="text-xs font-medium text-[#7aa2f7]">
              ({Math.round(progressPercentage)}%)
            </span>
          </div>
        </CollapsibleTrigger>

        <CollapsibleContent>
          <div className="px-4 pb-2 space-y-2">
            {/* Explanation (if provided) */}
            {explanation && (
              <p className="text-xs text-muted-foreground italic border-l-2 border-l-muted pl-2 py-1 ml-5">
                {explanation}
              </p>
            )}

            {/* Steps list */}
            <div className="space-y-0.5 ml-5">
              {steps.map((step, index) => {
                const isCompleted = step.status === "completed";
                const isInProgress = step.status === "in_progress";
                const isPending = step.status === "pending";

                return (
                  <div
                    key={`${index}-${step.step}`}
                    className={cn(
                      "flex items-start gap-2 px-2 py-1 rounded text-xs transition-colors",
                      isInProgress && "bg-accent/30",
                      isCompleted && "opacity-60"
                    )}
                  >
                    {/* Status icon */}
                    {isCompleted && (
                      <CheckCircle2 className="w-3.5 h-3.5 text-green-500 flex-shrink-0 mt-0.5" />
                    )}
                    {isInProgress && (
                      <Loader2 className="w-3.5 h-3.5 text-[#7aa2f7] animate-spin flex-shrink-0 mt-0.5" />
                    )}
                    {isPending && (
                      <Circle className="w-3.5 h-3.5 text-muted-foreground flex-shrink-0 mt-0.5" />
                    )}

                    {/* Step text */}
                    <span
                      className={cn(
                        "flex-1 leading-relaxed",
                        isCompleted && "line-through text-muted-foreground",
                        isInProgress && "font-medium text-foreground",
                        isPending && "text-muted-foreground"
                      )}
                    >
                      {step.step}
                    </span>
                  </div>
                );
              })}
            </div>
          </div>
        </CollapsibleContent>
      </div>
    </Collapsible>
  );
});
