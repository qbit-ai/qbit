import Ansi from "ansi-to-react";
import { Check, ChevronDown, ChevronRight, Clock, X } from "lucide-react";
import { useMemo } from "react";
import { Badge } from "@/components/ui/badge";
import { Collapsible, CollapsibleContent, CollapsibleTrigger } from "@/components/ui/collapsible";
import { stripOscSequences } from "@/lib/ansi";
import { cn } from "@/lib/utils";
import type { CommandBlock as CommandBlockType } from "@/store";

interface CommandBlockProps {
  block: CommandBlockType;
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

export function CommandBlock({ block, onToggleCollapse }: CommandBlockProps) {
  const isSuccess = block.exitCode === 0;

  // Strip OSC sequences but keep ANSI color codes for rendering
  const cleanOutput = useMemo(() => stripOscSequences(block.output), [block.output]);
  const hasOutput = cleanOutput.trim().length > 0;

  return (
    <Collapsible
      open={hasOutput && !block.isCollapsed}
      onOpenChange={() => hasOutput && onToggleCollapse(block.id)}
      className={cn(
        "border-l-2 mb-1 transition-colors hover:bg-card",
        isSuccess ? "border-l-[var(--ansi-green)]" : "border-l-[var(--ansi-red)]"
      )}
    >
      {/* Header */}
      <CollapsibleTrigger
        className={cn(
          "flex items-center gap-1.5 px-2 py-1.5 w-full text-left select-none transition-colors",
          hasOutput && "hover:bg-white/5 cursor-pointer",
          isSuccess ? "bg-[var(--ansi-green)]/5" : "bg-[var(--ansi-red)]/5"
        )}
        disabled={!hasOutput}
      >
        {/* Exit code badge */}
        {block.exitCode !== null && (
          <Badge
            variant={isSuccess ? "default" : "destructive"}
            className={cn(
              "h-5 px-1.5 gap-1",
              isSuccess
                ? "bg-[var(--ansi-green)]/20 text-[var(--ansi-green)] hover:bg-[var(--ansi-green)]/30"
                : "bg-[var(--ansi-red)]/20 text-[var(--ansi-red)] hover:bg-[var(--ansi-red)]/30"
            )}
          >
            {isSuccess ? <Check className="w-3 h-3" /> : <X className="w-3 h-3" />}
            {!isSuccess && block.exitCode}
          </Badge>
        )}

        {/* Command */}
        <code className="text-foreground font-mono text-xs flex-1 truncate">
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
          {hasOutput && (
            <span className="flex items-center gap-0.5 px-1 py-0.5 rounded hover:bg-white/10 transition-colors">
              {block.isCollapsed ? (
                <ChevronRight className="w-3 h-3" />
              ) : (
                <ChevronDown className="w-3 h-3" />
              )}
              <span className="text-[10px] uppercase tracking-wide">
                {block.isCollapsed ? "Show" : "Hide"}
              </span>
            </span>
          )}
        </div>
      </CollapsibleTrigger>

      {/* Output */}
      <CollapsibleContent>
        <div className="px-2 pb-2 pl-7">
          <div className="ansi-output text-xs leading-tight whitespace-pre-wrap break-words bg-background rounded-md p-2 border border-border">
            <Ansi useClasses>{cleanOutput}</Ansi>
          </div>
        </div>
      </CollapsibleContent>
    </Collapsible>
  );
}
