import { ChevronDown, ChevronRight, Clock } from "lucide-react";
import { useMemo } from "react";
import { Collapsible, CollapsibleContent, CollapsibleTrigger } from "@/components/ui/collapsible";
import { stripOscSequences } from "@/lib/ansi";
import { cn } from "@/lib/utils";
import type { CommandBlock as CommandBlockType } from "@/store";
import { StaticTerminalOutput } from "./StaticTerminalOutput";

interface CommandBlockProps {
  block: CommandBlockType;
  sessionId?: string;
  onToggleCollapse: (blockId: string) => void;
}

function formatDuration(ms: number | null): string {
  if (ms === null) return "";
  if (ms < 1000) return `${ms}ms`;
  if (ms < 60000) return `${(ms / 1000).toFixed(1)}s`;
  const minutes = Math.floor(ms / 60000);
  const seconds = ((ms % 60000) / 1000).toFixed(0);
  return `${minutes}m ${seconds}s`;
}

export function CommandBlock({ block, sessionId, onToggleCollapse }: CommandBlockProps) {
  const isSuccess = block.exitCode === 0;

  // Strip OSC sequences but keep ANSI color codes for rendering
  const cleanOutput = useMemo(() => stripOscSequences(block.output), [block.output]);
  const hasOutput = cleanOutput.trim().length > 0;

  return (
    <Collapsible
      open={hasOutput && !block.isCollapsed}
      onOpenChange={() => hasOutput && onToggleCollapse(block.id)}
      className="w-full"
    >
      {/* Header */}
      <CollapsibleTrigger
        className={cn(
          "flex items-center gap-2 px-5 py-3 w-full text-left select-none",
          hasOutput && "cursor-pointer"
        )}
        disabled={!hasOutput}
      >
        {/* Command */}
        <code
          className="flex-1 truncate text-[var(--ansi-white)]"
          style={{
            fontSize: "12px",
            lineHeight: 1.4,
            fontFamily: "JetBrains Mono, Menlo, Monaco, Consolas, monospace",
          }}
        >
          <span className="text-[var(--ansi-green)]">$ </span>
          {block.command || "(empty command)"}
        </code>

        {/* Metadata */}
        <div className="flex items-center gap-3 text-xs text-muted-foreground flex-shrink-0">
          {block.durationMs !== null && (
            <span className="flex items-center gap-1">
              <Clock className="w-3 h-3" />
              {formatDuration(block.durationMs)}
            </span>
          )}
          {/* Show exit code only on failure */}
          {!isSuccess && block.exitCode !== null && (
            <span className="text-[var(--ansi-red)]">exit {block.exitCode}</span>
          )}
          {hasOutput && (
            <span className="flex items-center gap-0.5">
              {block.isCollapsed ? (
                <ChevronRight className="w-3.5 h-3.5" />
              ) : (
                <ChevronDown className="w-3.5 h-3.5" />
              )}
            </span>
          )}
        </div>
      </CollapsibleTrigger>

      {/* Output - now using xterm for consistency with LiveTerminalBlock */}
      <CollapsibleContent>
        <div className="px-5 pb-4">
          <StaticTerminalOutput
            output={cleanOutput}
            sessionId={sessionId}
            workingDirectory={block.workingDirectory}
          />
        </div>
      </CollapsibleContent>
    </Collapsible>
  );
}
