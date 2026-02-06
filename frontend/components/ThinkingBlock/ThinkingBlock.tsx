import { Brain, ChevronDown, ChevronRight } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import { cn } from "@/lib/utils";
import { useIsAgentThinking, useIsThinkingExpanded, useStore, useThinkingContent } from "@/store";

interface ThinkingBlockUIProps {
  content: string;
  isExpanded: boolean;
  isThinking: boolean;
  onToggle: () => void;
}

/**
 * ReactMarkdown components configuration for thinking content.
 * Extracted to module scope to prevent recreation on every render,
 * which would cause unnecessary re-renders of the markdown content.
 */
const MARKDOWN_COMPONENTS = {
  // Compact headings for thinking content
  // Use <div> instead of <p> to avoid invalid HTML nesting (<p> cannot contain <p>),
  // which causes browsers to auto-close elements and break block layout.
  h1: ({ children }: { children?: React.ReactNode }) => (
    <div className="font-bold text-muted-foreground mt-2 mb-1 first:mt-0">{children}</div>
  ),
  h2: ({ children }: { children?: React.ReactNode }) => (
    <div className="font-bold text-muted-foreground mt-2 mb-1 first:mt-0">{children}</div>
  ),
  h3: ({ children }: { children?: React.ReactNode }) => (
    <div className="font-semibold text-muted-foreground mt-1.5 mb-1 first:mt-0">{children}</div>
  ),
  h4: ({ children }: { children?: React.ReactNode }) => (
    <div className="font-semibold text-muted-foreground mt-1.5 mb-1 first:mt-0">{children}</div>
  ),
  // Compact paragraphs
  p: ({ children }: { children?: React.ReactNode }) => (
    <p className="text-muted-foreground mb-2 last:mb-0 leading-relaxed">{children}</p>
  ),
  // Inline styles
  strong: ({ children }: { children?: React.ReactNode }) => (
    <strong className="font-semibold text-foreground">{children}</strong>
  ),
  em: ({ children }: { children?: React.ReactNode }) => <em className="italic">{children}</em>,
  // Compact lists
  ul: ({ children }: { children?: React.ReactNode }) => (
    <ul className="list-disc list-inside mb-2 last:mb-0 space-y-0.5">{children}</ul>
  ),
  ol: ({ children }: { children?: React.ReactNode }) => (
    <ol className="list-decimal list-inside mb-2 last:mb-0 space-y-0.5">{children}</ol>
  ),
  li: ({ children }: { children?: React.ReactNode }) => (
    <li className="text-muted-foreground">{children}</li>
  ),
  // Code
  code: ({ children, className }: { children?: React.ReactNode; className?: string }) => {
    const isBlock = className?.includes("language-");
    if (isBlock) {
      return (
        <pre className="bg-background rounded px-2 py-1 my-1 overflow-x-auto">
          <code className="text-muted-foreground">{children}</code>
        </pre>
      );
    }
    return (
      <code className="bg-background rounded px-1 py-0.5 text-foreground font-mono">
        {children}
      </code>
    );
  },
  // Links
  a: ({ href, children }: { href?: string; children?: React.ReactNode }) => (
    <a
      href={href}
      className="text-accent hover:underline"
      target="_blank"
      rel="noopener noreferrer"
    >
      {children}
    </a>
  ),
};

/**
 * Shared UI component for thinking block display.
 */
function ThinkingBlockUI({ content, isExpanded, isThinking, onToggle }: ThinkingBlockUIProps) {
  const scrollRef = useRef<HTMLDivElement>(null);

  // Auto-scroll to bottom when content changes (only while actively thinking)
  // biome-ignore lint/correctness/useExhaustiveDependencies: content is needed to trigger scroll on update
  useEffect(() => {
    if (isThinking && isExpanded && scrollRef.current) {
      scrollRef.current.scrollTop = scrollRef.current.scrollHeight;
    }
  }, [content, isThinking, isExpanded]);

  return (
    <div className="rounded-md bg-muted overflow-hidden">
      {/* Header - always visible */}
      <button
        type="button"
        onClick={onToggle}
        className="w-full flex items-center gap-2 px-2.5 py-1.5 hover:bg-accent transition-colors text-left"
      >
        <div className="flex items-center gap-2 flex-1">
          <Brain
            className={cn(
              "w-3.5 h-3.5",
              isThinking ? "text-[var(--ansi-magenta)] animate-pulse" : "text-[var(--ansi-cyan)]"
            )}
          />
          <span className="text-xs font-medium text-muted-foreground">
            {isThinking ? "Thinking..." : "Thinking"}
          </span>
          <span className="text-xs text-muted-foreground">
            ({content.length.toLocaleString()} chars)
          </span>
        </div>
        {isExpanded ? (
          <ChevronDown className="w-3.5 h-3.5 text-muted-foreground" />
        ) : (
          <ChevronRight className="w-3.5 h-3.5 text-muted-foreground" />
        )}
      </button>

      {/* Content - collapsible */}
      {isExpanded && (
        <div className="px-2.5 pb-2.5 border-t border-border">
          <div ref={scrollRef} className="mt-2 max-h-48 overflow-y-auto text-xs thinking-content">
            <ReactMarkdown remarkPlugins={[remarkGfm]} components={MARKDOWN_COMPONENTS}>
              {content}
            </ReactMarkdown>
            {isThinking && (
              <span className="inline-block w-1.5 h-3 bg-[var(--ansi-magenta)] animate-pulse ml-0.5 align-middle" />
            )}
          </div>
        </div>
      )}
    </div>
  );
}

interface StreamingThinkingBlockProps {
  sessionId: string;
}

/**
 * StreamingThinkingBlock - Displays live thinking content from the store.
 * Use this in the UnifiedTimeline for active streaming.
 */
export function StreamingThinkingBlock({ sessionId }: StreamingThinkingBlockProps) {
  const content = useThinkingContent(sessionId);
  const isExpanded = useIsThinkingExpanded(sessionId);
  const isThinking = useIsAgentThinking(sessionId);
  const setThinkingExpanded = useStore((state) => state.setThinkingExpanded);

  if (!content) {
    return null;
  }

  return (
    <ThinkingBlockUI
      content={content}
      isExpanded={isExpanded}
      isThinking={isThinking}
      onToggle={() => setThinkingExpanded(sessionId, !isExpanded)}
    />
  );
}

interface StaticThinkingBlockProps {
  content: string;
  isThinking?: boolean;
  /** Initial expanded state (defaults to true for streaming, false for historical) */
  defaultExpanded?: boolean;
}

/**
 * StaticThinkingBlock - Displays thinking content with local expanded state.
 * Use this for inline streaming blocks (in UnifiedTimeline) and persisted messages (in AgentMessage).
 *
 * @param defaultExpanded - Set to true for streaming blocks (expanded by default),
 *                          false for historical/persisted blocks (collapsed by default)
 */
export function StaticThinkingBlock({
  content,
  isThinking = false,
  defaultExpanded = false,
}: StaticThinkingBlockProps) {
  const [isExpanded, setIsExpanded] = useState(defaultExpanded);

  // When isThinking becomes true, expand automatically
  useEffect(() => {
    if (isThinking) {
      setIsExpanded(true);
    }
  }, [isThinking]);

  if (!content) {
    return null;
  }

  return (
    <ThinkingBlockUI
      content={content}
      isExpanded={isExpanded}
      isThinking={isThinking}
      onToggle={() => setIsExpanded(!isExpanded)}
    />
  );
}

interface ThinkingBlockProps {
  /** Session ID for live streaming mode */
  sessionId?: string;
  /** Static thinking content for finalized messages */
  content?: string;
}

/**
 * ThinkingBlock - Facade that selects the appropriate implementation.
 *
 * @deprecated Prefer using StreamingThinkingBlock or StaticThinkingBlock directly.
 */
export function ThinkingBlock({ sessionId, content }: ThinkingBlockProps) {
  if (sessionId !== undefined) {
    return <StreamingThinkingBlock sessionId={sessionId} />;
  }
  if (content !== undefined) {
    return <StaticThinkingBlock content={content} />;
  }
  return null;
}
