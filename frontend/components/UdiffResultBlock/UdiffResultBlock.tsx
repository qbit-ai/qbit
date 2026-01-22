import { Bot } from "lucide-react";
import { memo, useMemo } from "react";
import { DiffView } from "@/components/DiffView";
import { cn } from "@/lib/utils";

interface ParsedDiff {
  filePath: string;
  content: string;
}

/**
 * Parse diff blocks from the coder response.
 * Extracts ```diff fenced code blocks and their file paths.
 */
function parseDiffBlocks(response: string): ParsedDiff[] {
  const regex = /```diff\n([\s\S]*?)```/g;
  const diffs: ParsedDiff[] = [];

  for (const match of response.matchAll(regex)) {
    const content = match[1];
    // Extract file path from --- a/path or +++ b/path header
    const pathMatch = content.match(/^[-+]{3} [ab]\/(.+)$/m);
    const filePath = pathMatch ? pathMatch[1] : "unknown";
    diffs.push({ filePath, content });
  }

  return diffs;
}

/**
 * Extract the summary section from the response (after "---" marker).
 */
function extractSummary(response: string): string | null {
  const summaryMatch = response.match(/\n---\n\*\*Applied Changes:\*\*\n([\s\S]*?)$/);
  if (summaryMatch) {
    return summaryMatch[1].trim();
  }
  return null;
}

interface UdiffResultBlockProps {
  response: string;
  durationMs: number;
  className?: string;
}

/**
 * Renders the result of a coder sub-agent execution.
 * Parses diff blocks from the response and displays them using DiffView.
 */
export const UdiffResultBlock = memo(function UdiffResultBlock({
  response,
  durationMs,
  className,
}: UdiffResultBlockProps) {
  const parsedDiffs = useMemo(() => parseDiffBlocks(response), [response]);
  const summary = useMemo(() => extractSummary(response), [response]);

  // If no diffs were parsed, show the raw response
  if (parsedDiffs.length === 0) {
    return (
      <div
        className={cn(
          "border-l-[3px] border-l-accent bg-[var(--color-accent-dim)] rounded-l-lg overflow-hidden",
          className
        )}
      >
        <div className="flex items-center gap-2 px-3 py-2">
          <Bot className="w-3.5 h-3.5 text-muted-foreground" />
          <span className="font-mono text-xs text-muted-foreground">coder</span>
          <span className="text-[10px] text-muted-foreground/70">{durationMs}ms</span>
        </div>
        <div className="px-3 pb-3">
          <pre className="text-[11px] text-muted-foreground whitespace-pre-wrap font-mono">
            {response}
          </pre>
        </div>
      </div>
    );
  }

  return (
    <div
      className={cn(
        "border-l-[3px] border-l-accent bg-[var(--color-accent-dim)] rounded-l-lg overflow-hidden",
        className
      )}
    >
      {/* Header */}
      <div className="flex items-center gap-2 px-3 py-2">
        <Bot className="w-3.5 h-3.5 text-muted-foreground" />
        <span className="font-mono text-xs text-muted-foreground">coder</span>
        <span className="text-[10px] text-muted-foreground/70">{durationMs}ms</span>
        <span className="text-[10px] text-muted-foreground/50">
          {parsedDiffs.length} file{parsedDiffs.length !== 1 ? "s" : ""}
        </span>
      </div>

      {/* Diff blocks */}
      <div className="px-3 pb-3 space-y-2">
        {parsedDiffs.map((diff, index) => (
          <DiffView
            key={`${diff.filePath}-${index}`}
            diff={diff.content}
            filePath={diff.filePath}
            maxHeight="300px"
          />
        ))}
      </div>

      {/* Summary section */}
      {summary && (
        <div className="px-3 pb-3 pt-1 border-t border-[var(--color-border-subtle)]">
          <div className="text-[11px] text-muted-foreground whitespace-pre-wrap">{summary}</div>
        </div>
      )}
    </div>
  );
});
