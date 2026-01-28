import Ansi from "ansi-to-react";
import { useEffect, useRef } from "react";
import { stripOscSequences } from "@/lib/ansi";
import { cn } from "@/lib/utils";

interface StreamingOutputProps {
  content: string;
  /** Maximum height in pixels (default: 200) */
  maxHeight?: number;
  className?: string;
  /** Whether to auto-scroll to bottom on new content (default: true) */
  autoScroll?: boolean;
}

/**
 * A fixed-height output component that auto-scrolls as new content arrives.
 * Used for displaying streaming command output in real-time.
 */
export function StreamingOutput({
  content,
  maxHeight = 200,
  className,
  autoScroll = true,
}: StreamingOutputProps) {
  const containerRef = useRef<HTMLPreElement>(null);
  const cleanContent = stripOscSequences(content);

  // Auto-scroll to bottom when content changes
  useEffect(() => {
    if (autoScroll && containerRef.current) {
      containerRef.current.scrollTop = containerRef.current.scrollHeight;
    }
  }, [cleanContent, autoScroll]);

  if (!cleanContent.trim()) {
    return <span className="text-[10px] text-muted-foreground italic">No output</span>;
  }

  return (
    <pre
      ref={containerRef}
      style={{ maxHeight }}
      className={cn(
        "ansi-output text-[11px] text-[var(--ansi-cyan)] bg-background rounded p-2",
        "whitespace-pre-wrap break-all",
        "overflow-y-auto overflow-x-auto",
        className
      )}
    >
      <Ansi useClasses>{cleanContent}</Ansi>
    </pre>
  );
}
