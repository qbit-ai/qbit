import { Sparkles } from "lucide-react";
import { memo } from "react";

interface SystemHooksCardProps {
  /** Array of system hook strings to display */
  hooks: string[];
}

/**
 * Displays system hooks that were injected during an agent turn.
 *
 * The card shows a collapsed summary with the count of hooks,
 * which can be expanded to view the full hook contents.
 *
 * Used in both UnifiedTimeline (for standalone system_hook blocks)
 * and AgentMessage (for hooks associated with a specific message).
 */
export const SystemHooksCard = memo(function SystemHooksCard({ hooks }: SystemHooksCardProps) {
  const count = hooks.length;

  return (
    <div className="rounded-lg bg-[var(--ansi-yellow)]/10 border-l-2 border-l-[var(--ansi-yellow)] p-2 space-y-2">
      <div className="flex items-center gap-2 text-xs text-muted-foreground">
        <Sparkles className="w-3.5 h-3.5 text-[var(--ansi-yellow)]" />
        <span>System hooks injected{count > 0 ? ` (${count})` : ""}</span>
      </div>

      {count > 0 && (
        <details className="text-xs">
          <summary className="cursor-pointer select-none text-muted-foreground hover:text-foreground/80">
            View hook{count === 1 ? "" : "s"}
          </summary>
          <div className="mt-2 space-y-2">
            {hooks.map((hook, idx) => (
              <pre
                // biome-ignore lint/suspicious/noArrayIndexKey: hooks have no stable id
                key={idx}
                className="whitespace-pre-wrap rounded-md bg-card/50 border border-border p-2 overflow-auto"
              >
                {hook}
              </pre>
            ))}
          </div>
        </details>
      )}
    </div>
  );
});
