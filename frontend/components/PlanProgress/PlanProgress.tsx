import { CheckCircle2, ChevronDown, ChevronRight, Circle, Loader2 } from "lucide-react";
import { memo, useState } from "react";
import { Collapsible, CollapsibleContent, CollapsibleTrigger } from "@/components/ui/collapsible";
import { cn } from "@/lib/utils";
import type { TaskPlan } from "@/store";

interface PlanProgressProps {
  plan: TaskPlan;
  className?: string;
}

export const PlanProgress = memo(function PlanProgress({ plan, className }: PlanProgressProps) {
  const [isOpen, setIsOpen] = useState(true);

  if (!plan || plan.steps.length === 0) {
    return null;
  }

  const isPlanComplete = plan.summary.total > 0 && plan.summary.completed === plan.summary.total;

  if (isPlanComplete) {
    return null;
  }

  const { summary, steps, explanation } = plan;
  const progressPercentage = summary.total > 0 ? (summary.completed / summary.total) * 100 : 0;

  return (
    <Collapsible open={isOpen} onOpenChange={setIsOpen}>
      <div className={cn("border-l-2 border-l-[#7aa2f7] rounded-r-md bg-card/50", className)}>
        <CollapsibleTrigger className="w-full group">
          <div className="flex items-center justify-between px-3 py-2 hover:bg-accent/50 transition-colors">
            <div className="flex items-center gap-2 min-w-0">
              {isOpen ? (
                <ChevronDown className="w-4 h-4 text-[#7aa2f7] flex-shrink-0" />
              ) : (
                <ChevronRight className="w-4 h-4 text-[#7aa2f7] flex-shrink-0" />
              )}
              <span className="text-sm font-medium text-foreground">Task Plan</span>
              <span className="text-xs text-muted-foreground">
                {summary.completed}/{summary.total} steps
              </span>
            </div>
            <div className="flex items-center gap-2">
              {/* Progress percentage */}
              <span className="text-xs font-medium text-[#7aa2f7]">
                {Math.round(progressPercentage)}%
              </span>
            </div>
          </div>

          {/* Progress bar */}
          <div className="mx-3 mb-2 h-1.5 bg-muted/30 rounded-full overflow-hidden">
            <div
              className="h-full bg-[#7aa2f7] transition-all duration-300 ease-out"
              style={{ width: `${progressPercentage}%` }}
            />
          </div>
        </CollapsibleTrigger>

        <CollapsibleContent>
          <div className="px-3 pb-3 space-y-2">
            {/* Explanation (if provided) */}
            {explanation && (
              <p className="text-xs text-muted-foreground italic border-l-2 border-l-muted pl-2 py-1">
                {explanation}
              </p>
            )}

            {/* All steps list */}
            <div className="space-y-1">
              {steps.map((step, index) => {
                const isCompleted = step.status === "completed";
                const isInProgress = step.status === "in_progress";
                const isPending = step.status === "pending";

                return (
                  <div
                    key={`${index}-${step.step}`}
                    className={cn(
                      "flex items-start gap-2 px-2 py-1.5 rounded text-sm transition-colors",
                      isInProgress && "bg-accent/30",
                      isCompleted && "opacity-60"
                    )}
                  >
                    {/* Status icon */}
                    {isCompleted && (
                      <CheckCircle2 className="w-4 h-4 text-green-500 flex-shrink-0 mt-0.5" />
                    )}
                    {isInProgress && (
                      <Loader2 className="w-4 h-4 text-[#7aa2f7] animate-spin flex-shrink-0 mt-0.5" />
                    )}
                    {isPending && (
                      <Circle className="w-4 h-4 text-muted-foreground flex-shrink-0 mt-0.5" />
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
