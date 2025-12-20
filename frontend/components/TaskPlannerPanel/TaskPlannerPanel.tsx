import { CheckCircle2, Circle, GripVertical, ListTodo, Loader2, X } from "lucide-react";
import { memo, useCallback, useEffect, useRef, useState } from "react";
import { Button } from "@/components/ui/button";
import { ScrollArea } from "@/components/ui/scroll-area";
import { cn } from "@/lib/utils";
import { useStore } from "@/store";

interface TaskPlannerPanelProps {
  sessionId: string | null;
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

const MIN_WIDTH = 280;
const MAX_WIDTH = 500;
const DEFAULT_WIDTH = 350;

/**
 * Right-side panel showing the current session's task plan.
 * Renders inline as part of the flex layout (not a modal overlay).
 * Only one right panel (this or ContextPanel) should be visible at a time.
 */
export const TaskPlannerPanel = memo(function TaskPlannerPanel({
  sessionId,
  open,
  onOpenChange,
}: TaskPlannerPanelProps) {
  const plan = useStore((state) => (sessionId ? state.sessions[sessionId]?.plan : undefined));

  // Resize state
  const [width, setWidth] = useState(DEFAULT_WIDTH);
  const isResizing = useRef(false);
  const panelRef = useRef<HTMLDivElement>(null);

  // Handle resize
  const startResizing = useCallback((e: React.MouseEvent) => {
    e.preventDefault();
    isResizing.current = true;
    document.body.style.cursor = "col-resize";
    document.body.style.userSelect = "none";
  }, []);

  useEffect(() => {
    const handleMouseMove = (e: MouseEvent) => {
      if (!isResizing.current || !panelRef.current) return;

      // Calculate new width based on distance from right edge of viewport
      const newWidth = window.innerWidth - e.clientX;
      if (newWidth >= MIN_WIDTH && newWidth <= MAX_WIDTH) {
        setWidth(newWidth);
      }
    };

    const handleMouseUp = () => {
      if (isResizing.current) {
        isResizing.current = false;
        document.body.style.cursor = "";
        document.body.style.userSelect = "";
      }
    };

    document.addEventListener("mousemove", handleMouseMove);
    document.addEventListener("mouseup", handleMouseUp);

    return () => {
      document.removeEventListener("mousemove", handleMouseMove);
      document.removeEventListener("mouseup", handleMouseUp);
    };
  }, []);

  if (!open) return null;

  const hasPlan = plan && plan.steps.length > 0;
  const summary = plan?.summary;
  const steps = plan?.steps ?? [];
  const explanation = plan?.explanation;
  const progressPercentage =
    summary && summary.total > 0 ? (summary.completed / summary.total) * 100 : 0;

  // Find the current in-progress step
  const currentStep = steps.find((s) => s.status === "in_progress");

  return (
    <div
      ref={panelRef}
      className="bg-card border-l border-border flex flex-col relative"
      style={{ width: `${width}px`, minWidth: `${MIN_WIDTH}px`, maxWidth: `${MAX_WIDTH}px` }}
    >
      {/* Resize handle */}
      {/* biome-ignore lint/a11y/noStaticElementInteractions: resize handle is mouse-only */}
      <div
        className="absolute top-0 left-0 w-1 h-full cursor-col-resize hover:bg-[var(--ansi-blue)] transition-colors z-10 group"
        onMouseDown={startResizing}
      >
        <div className="absolute top-1/2 left-0 -translate-y-1/2 opacity-0 group-hover:opacity-100 transition-opacity">
          <GripVertical className="w-3 h-3 text-muted-foreground" />
        </div>
      </div>

      {/* Header */}
      <div className="flex items-center justify-between px-3 py-2 border-b border-border">
        <div className="flex items-center gap-2 min-w-0">
          <ListTodo className="w-4 h-4 text-[#7aa2f7] shrink-0" />
          <h2 className="text-sm font-medium truncate">Task Plan</h2>
          {hasPlan && summary && (
            <span className="text-xs text-muted-foreground shrink-0">
              {summary.completed}/{summary.total}
            </span>
          )}
        </div>
        <div className="flex items-center gap-1 shrink-0">
          <Button
            variant="ghost"
            size="icon"
            className="h-6 w-6"
            onClick={() => onOpenChange(false)}
          >
            <X className="w-3.5 h-3.5" />
          </Button>
        </div>
      </div>

      {/* Progress bar */}
      {hasPlan && (
        <div className="px-3 py-2 border-b border-border">
          <div className="flex items-center justify-between mb-1.5">
            <span className="text-xs text-muted-foreground">Progress</span>
            <span className="text-xs font-medium text-[#7aa2f7]">
              {Math.round(progressPercentage)}%
            </span>
          </div>
          <div className="h-1.5 bg-muted/30 rounded-full overflow-hidden">
            <div
              className="h-full bg-[#7aa2f7] transition-all duration-300 ease-out"
              style={{ width: `${progressPercentage}%` }}
            />
          </div>
        </div>
      )}

      {/* Content */}
      <ScrollArea className="flex-1">
        <div className="p-3 space-y-3">
          {!hasPlan ? (
            <div className="text-center py-8">
              <ListTodo className="w-8 h-8 text-muted-foreground/50 mx-auto mb-2" />
              <p className="text-sm text-muted-foreground">No active task plan</p>
              <p className="text-xs text-muted-foreground/70 mt-1">
                The AI will create a plan when working on multi-step tasks
              </p>
            </div>
          ) : (
            <>
              {/* Explanation (if provided) */}
              {explanation && (
                <div className="border-l-2 border-l-muted pl-2 py-1">
                  <p className="text-xs text-muted-foreground italic">{explanation}</p>
                </div>
              )}

              {/* Current step highlight */}
              {currentStep && (
                <div className="bg-[#7aa2f7]/10 border border-[#7aa2f7]/30 rounded-md p-2">
                  <div className="flex items-start gap-2">
                    <Loader2 className="w-4 h-4 text-[#7aa2f7] animate-spin flex-shrink-0 mt-0.5" />
                    <div>
                      <p className="text-xs font-medium text-[#7aa2f7]">Current Step</p>
                      <p className="text-sm text-foreground mt-0.5">{currentStep.step}</p>
                    </div>
                  </div>
                </div>
              )}

              {/* All steps list */}
              <div className="space-y-1">
                <p className="text-xs font-medium text-muted-foreground mb-2">All Steps</p>
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
            </>
          )}
        </div>
      </ScrollArea>

      {/* Footer */}
      <div className="px-3 py-2 border-t border-border text-xs text-muted-foreground">
        <kbd className="bg-muted px-1 py-0.5 rounded text-[10px]">Cmd+Shift+T</kbd> to toggle
      </div>
    </div>
  );
});
