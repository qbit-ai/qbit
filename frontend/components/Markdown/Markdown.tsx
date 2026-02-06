import {
  type ComponentPropsWithoutRef,
  createContext,
  lazy,
  memo,
  type ReactNode,
  Suspense,
  useContext,
  useDeferredValue,
  useMemo,
} from "react";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";

// Lazy load the SyntaxHighlighter component (~170KB)
// This significantly improves initial page load time
const LazySyntaxHighlighter = lazy(() =>
  import("react-syntax-highlighter").then((mod) => ({
    default: mod.Prism,
  }))
);

// Lazy load the theme (imported separately to avoid blocking)
const getCodeTheme = async () => {
  const { oneDark } = await import("react-syntax-highlighter/dist/esm/styles/prism");
  return {
    ...oneDark,
    'code[class*="language-"]': {
      ...oneDark['code[class*="language-"]'],
      background: "transparent",
    },
    'pre[class*="language-"]': {
      ...oneDark['pre[class*="language-"]'],
      background: "transparent",
    },
  };
};

// Cache the theme once loaded
let cachedCodeTheme: Record<string, unknown> | null = null;
// Start loading the theme immediately on module load
getCodeTheme().then((theme) => {
  cachedCodeTheme = theme;
});

import { FilePathLink } from "@/components/FilePathLink";
import { useFileIndex } from "@/hooks/useFileIndex";
import type { FileIndex } from "@/lib/fileIndex";
import { detectFilePathsWithIndex } from "@/lib/pathDetection";
import { cn } from "@/lib/utils";
import { CopyButton } from "./CopyButton";

interface MarkdownContextValue {
  sessionId?: string;
  workingDirectory?: string;
  fileIndex?: FileIndex;
}

const MarkdownContext = createContext<MarkdownContextValue>({});

export function useMarkdownContext() {
  return useContext(MarkdownContext);
}

function processTextWithFilePaths(text: string, context: MarkdownContextValue): ReactNode {
  const { sessionId, workingDirectory, fileIndex } = context;

  // If no context or fileIndex available, return text as-is (no links)
  if (!sessionId || !workingDirectory || !fileIndex) {
    return text;
  }

  // Detect file paths in the text
  const detectedPaths = detectFilePathsWithIndex(text, fileIndex);

  // If no paths detected, return text as-is
  if (detectedPaths.length === 0) {
    return text;
  }

  // Split text into segments with file path links
  const segments: ReactNode[] = [];
  let lastIndex = 0;

  for (let idx = 0; idx < detectedPaths.length; idx++) {
    const detected = detectedPaths[idx];

    // Add text before this path
    if (detected.start > lastIndex) {
      segments.push(text.substring(lastIndex, detected.start));
    }

    // Add FilePathLink component
    segments.push(
      <FilePathLink
        key={`path-${idx}`}
        detected={detected}
        workingDirectory={workingDirectory}
        absolutePath={detected.absolutePath}
      >
        {detected.raw}
      </FilePathLink>
    );

    lastIndex = detected.end;
  }

  // Add remaining text
  if (lastIndex < text.length) {
    segments.push(text.substring(lastIndex));
  }

  return <>{segments}</>;
}

interface MarkdownProps {
  content: string;
  className?: string;
  /** Lightweight mode for streaming content - avoids expensive parsing */
  streaming?: boolean;
  sessionId?: string;
  workingDirectory?: string;
}

// Fallback component shown while SyntaxHighlighter is loading
// Displays the raw code with basic styling
function CodeBlockFallback({ code, language: _language }: { code: string; language: string }) {
  return (
    <div
      className="font-mono text-muted-foreground whitespace-pre-wrap break-words"
      style={{
        margin: 0,
        padding: "1.25rem",
        paddingTop: "2.5rem",
        background: "var(--background)",
        border: "1px solid var(--border-medium)",
        borderRadius: "0.5rem",
      }}
    >
      {code}
    </div>
  );
}

// Inner component that renders the syntax highlighted code
// This is wrapped in Suspense to handle the lazy-loaded SyntaxHighlighter
function SyntaxHighlightedCode({ code, language, ...props }: { code: string; language: string }) {
  // Use cached theme or a fallback empty object (theme will be loaded)
  const theme = cachedCodeTheme || {};

  return (
    <LazySyntaxHighlighter
      // biome-ignore lint/suspicious/noExplicitAny: SyntaxHighlighter style prop typing is incompatible
      style={theme as any}
      language={language || "text"}
      PreTag="div"
      customStyle={{
        margin: 0,
        padding: "1.25rem",
        paddingTop: "2.5rem",
        background: "var(--background)",
        border: "1px solid var(--border-medium)",
        borderRadius: "0.5rem",
      }}
      {...props}
    >
      {code}
    </LazySyntaxHighlighter>
  );
}

function CodeBlock({
  inline,
  className,
  children,
  ...props
}: ComponentPropsWithoutRef<"code"> & { inline?: boolean }) {
  const context = useMarkdownContext();
  const match = /language-(\w+)/.exec(className || "");
  const language = match ? match[1] : "";
  const codeString = String(children).replace(/\n$/, "");

  if (!inline && (match || codeString.includes("\n"))) {
    return (
      <div className="relative group my-4">
        <div className="absolute right-3 top-3 flex items-center gap-2 z-10">
          <CopyButton
            content={codeString}
            className="opacity-0 group-hover:opacity-100 transition-opacity"
          />
          {language && (
            <div className="text-[11px] text-muted-foreground uppercase font-mono font-semibold bg-background px-2 py-1 rounded">
              {language}
            </div>
          )}
        </div>
        <Suspense fallback={<CodeBlockFallback code={codeString} language={language} />}>
          <SyntaxHighlightedCode code={codeString} language={language} {...props} />
        </Suspense>
      </div>
    );
  }

  // For inline code, try to detect file paths
  const processedContent = processTextWithFilePaths(codeString, context);
  const hasFileLinks = processedContent !== codeString;

  return (
    <code
      className={cn(
        "px-1.5 py-0.5 rounded bg-background border border-[var(--border-medium)] text-foreground/80 font-mono text-[0.85em]",
        // Remove whitespace-nowrap if we have file links to allow proper styling
        !hasFileLinks && "whitespace-nowrap",
        className
      )}
      {...props}
    >
      {processedContent}
    </code>
  );
}

// Stable reference — never changes between renders
const remarkPlugins = [remarkGfm];

export const Markdown = memo(function Markdown({
  content,
  className,
  streaming,
  sessionId,
  workingDirectory,
}: MarkdownProps) {
  const fileIndex = useFileIndex(workingDirectory);
  // During streaming, defer markdown parsing so React can skip intermediate
  // renders and keep the UI responsive even on long responses.
  const deferredContent = useDeferredValue(content);
  const renderedContent = streaming ? deferredContent : content;

  const contextValue = useMemo(
    () => ({ sessionId, workingDirectory, fileIndex: fileIndex ?? undefined }),
    [sessionId, workingDirectory, fileIndex]
  );

  // Memoize components so ReactMarkdown doesn't re-parse when only the parent
  // re-renders but renderedContent hasn't changed yet (deferred).
  const components = useMemo(
    () => ({
      code: CodeBlock,
      // Headings
      h1: ({ children }: { children?: ReactNode }) => (
        <h1 className="text-2xl font-bold text-foreground mt-6 mb-3 first:mt-0 pb-2 border-b border-[var(--border-medium)]">
          {children}
        </h1>
      ),
      h2: ({ children }: { children?: ReactNode }) => (
        <h2 className="text-lg font-bold text-accent mt-5 mb-3 first:mt-0 pb-2 border-b border-[var(--border-subtle)] flex items-center gap-2">
          <span className="w-1 h-5 bg-accent rounded-full" />
          {children}
        </h2>
      ),
      h3: ({ children }: { children?: ReactNode }) => (
        <h3 className="text-base font-semibold text-muted-foreground mt-4 mb-2 first:mt-0 pl-3 border-l-2 border-accent">
          {children}
        </h3>
      ),
      // Paragraphs
      p: ({ children }: { children?: ReactNode }) => (
        <p className="text-foreground mb-3 last:mb-0 leading-relaxed">
          {typeof children === "string"
            ? processTextWithFilePaths(children, contextValue)
            : children}
        </p>
      ),
      // Lists
      ul: ({ children }: { children?: ReactNode }) => (
        <ul className="list-disc list-outside text-foreground mb-3 space-y-2 pl-6">{children}</ul>
      ),
      ol: ({ children }: { children?: ReactNode }) => (
        <ol className="list-decimal list-outside text-foreground mb-3 space-y-2 pl-6">
          {children}
        </ol>
      ),
      li: ({ children }: { children?: ReactNode }) => (
        <li className="text-foreground leading-relaxed">
          {typeof children === "string"
            ? processTextWithFilePaths(children, contextValue)
            : children}
        </li>
      ),
      // Links
      a: ({ href, children }: { href?: string; children?: ReactNode }) => (
        <a
          href={href}
          target="_blank"
          rel="noopener noreferrer"
          className="text-accent hover:text-[var(--success)] hover:underline transition-colors"
        >
          {children}
        </a>
      ),
      // Blockquotes
      blockquote: ({ children }: { children?: ReactNode }) => (
        <blockquote className="border-l-4 border-accent bg-[var(--accent-dim)] pl-4 py-2 my-3 text-muted-foreground italic rounded-r">
          {typeof children === "string"
            ? processTextWithFilePaths(children, contextValue)
            : children}
        </blockquote>
      ),
      // Horizontal rule
      hr: () => <hr className="my-4 border-[var(--border-medium)]" />,
      // Strong and emphasis
      strong: ({ children }: { children?: ReactNode }) => (
        <strong className="font-bold text-accent">{children}</strong>
      ),
      em: ({ children }: { children?: ReactNode }) => (
        <em className="italic text-[var(--success)]">{children}</em>
      ),
      // Tables
      table: ({ children }: { children?: ReactNode }) => (
        <div className="overflow-x-auto my-3">
          <table className="border-collapse text-[13px]">{children}</table>
        </div>
      ),
      thead: ({ children }: { children?: ReactNode }) => (
        <thead className="bg-muted/50 border-b border-[var(--border-subtle)]">{children}</thead>
      ),
      tbody: ({ children }: { children?: ReactNode }) => <tbody>{children}</tbody>,
      tr: ({ children }: { children?: ReactNode }) => (
        <tr className="border-b border-[var(--border-subtle)] last:border-b-0 [tbody>&]:hover:bg-muted/30">
          {children}
        </tr>
      ),
      th: ({ children }: { children?: ReactNode }) => (
        <th className="px-3 py-1.5 text-left text-foreground/80 font-medium text-[12px] uppercase tracking-wide">
          {typeof children === "string"
            ? processTextWithFilePaths(children, contextValue)
            : children}
        </th>
      ),
      td: ({ children }: { children?: ReactNode }) => (
        <td className="px-3 py-2 text-muted-foreground">
          {typeof children === "string"
            ? processTextWithFilePaths(children, contextValue)
            : children}
        </td>
      ),
    }),
    [contextValue]
  );

  // Memoize the ReactMarkdown output so remark parsing only runs when the
  // deferred content actually changes, not on every parent re-render.
  const markdownElement = useMemo(
    () => (
      <ReactMarkdown remarkPlugins={remarkPlugins} components={components}>
        {renderedContent}
      </ReactMarkdown>
    ),
    [renderedContent, components]
  );

  return (
    <MarkdownContext.Provider value={contextValue}>
      <div
        className={cn(
          "max-w-none break-words overflow-hidden text-foreground leading-relaxed",
          className
        )}
      >
        {markdownElement}
      </div>
    </MarkdownContext.Provider>
  );
});
