import { memo, useMemo } from "react";
import { cn } from "@/lib/utils";

export interface DiffViewProps {
  diff: string;
  /** File path to display in header */
  filePath?: string;
  className?: string;
  maxHeight?: string;
}

interface DiffLine {
  type: "addition" | "deletion" | "context";
  content: string;
  lineNumber?: number;
}

/**
 * Parse a unified diff string into structured lines
 */
function parseDiff(diff: string): DiffLine[] {
  const lines = diff.split("\n");
  const result: DiffLine[] = [];

  for (const line of lines) {
    if (line.startsWith("+")) {
      result.push({
        type: "addition",
        content: line.slice(1),
      });
    } else if (line.startsWith("-")) {
      result.push({
        type: "deletion",
        content: line.slice(1),
      });
    } else if (line.startsWith(" ") || line === "") {
      result.push({
        type: "context",
        content: line.slice(1),
      });
    }
  }

  return result;
}

/**
 * DiffView component renders unified diff format with syntax highlighting
 *
 * - Lines starting with '+' are rendered in green (additions)
 * - Lines starting with '-' are rendered in red (deletions)
 * - Context lines (starting with space) are rendered in neutral color
 */
export const DiffView = memo(function DiffView({
  diff,
  filePath,
  className,
  maxHeight = "400px",
}: DiffViewProps) {
  const parsedLines = useMemo(() => parseDiff(diff), [diff]);

  // Memoize style object to prevent recreation on each render
  const contentStyle = useMemo(() => ({ maxHeight }), [maxHeight]);

  return (
    <div
      className={cn("rounded-md border border-[var(--border-subtle)] overflow-hidden", className)}
    >
      {/* File path header */}
      {filePath && (
        <div className="bg-muted/50 px-3 py-1.5 border-b border-[var(--border-subtle)]">
          <span className="text-[10px] uppercase text-muted-foreground font-medium tracking-wide">
            Changes
          </span>
          <div className="text-[11px] font-mono text-muted-foreground mt-0.5">{filePath}</div>
        </div>
      )}

      {/* Diff content */}
      <div className="overflow-auto bg-background" style={contentStyle}>
        <div className="font-mono text-[11px]">
          {parsedLines.map((line, index) => {
            // Use index + content hash for stable key (diff lines are in fixed order)
            const key = `${index}-${line.type}-${line.content.slice(0, 20)}`;
            return (
              <div
                key={key}
                className={cn(
                  "px-3 py-0.5 leading-relaxed",
                  line.type === "addition" && "bg-[var(--success)]/10 text-[var(--success)]",
                  line.type === "deletion" && "bg-destructive/10 text-destructive",
                  line.type === "context" && "text-muted-foreground"
                )}
              >
                <span
                  className={cn(
                    "inline-block w-4 select-none mr-2",
                    line.type === "addition" && "text-[var(--success)]",
                    line.type === "deletion" && "text-destructive",
                    line.type === "context" && "text-transparent"
                  )}
                >
                  {line.type === "addition" ? "+" : line.type === "deletion" ? "-" : " "}
                </span>
                <span className="whitespace-pre-wrap break-all">{line.content}</span>
              </div>
            );
          })}
        </div>
      </div>
    </div>
  );
});
