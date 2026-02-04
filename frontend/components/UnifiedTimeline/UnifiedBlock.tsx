import { Sparkles } from "lucide-react";
import { memo, useCallback } from "react";
import { AgentMessage } from "@/components/AgentChat/AgentMessage";
import { CommandBlock } from "@/components/CommandBlock/CommandBlock";
import type { UnifiedBlock as UnifiedBlockType } from "@/store";
import { useStore } from "@/store";

interface UnifiedBlockProps {
  block: UnifiedBlockType;
  sessionId: string;
  workingDirectory: string;
}

// Get stable reference to toggleBlockCollapse action without creating a subscription
// This is safe because Zustand actions are stable references that never change
const getToggleBlockCollapse = () => useStore.getState().toggleBlockCollapse;

export const UnifiedBlock = memo(function UnifiedBlock({
  block,
  sessionId,
  workingDirectory,
}: UnifiedBlockProps) {
  // useCallback with empty deps ensures stable reference for the memoized component
  const toggleBlockCollapse = useCallback(
    (blockId: string) => getToggleBlockCollapse()(blockId),
    []
  );

  switch (block.type) {
    case "command":
      return (
        <CommandBlock
          block={block.data}
          sessionId={sessionId}
          onToggleCollapse={toggleBlockCollapse}
        />
      );

    case "agent_message":
      return (
        <AgentMessage
          message={block.data}
          sessionId={sessionId}
          workingDirectory={workingDirectory}
        />
      );

    case "system_hook": {
      const count = block.data.hooks.length;
      return (
        <div className="ml-6 rounded-lg bg-[var(--ansi-yellow)]/10 border-l-2 border-l-[var(--ansi-yellow)] p-2 space-y-2">
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
                {block.data.hooks.map((hook, idx) => (
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
    }

    case "agent_streaming":
      // This shouldn't appear in the timeline as streaming is handled separately
      // but we include it for completeness
      return null;

    default:
      return null;
  }
});
