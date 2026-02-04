import {
  type ComponentPropsWithoutRef,
  Suspense,
  createContext,
  lazy,
  memo,
  type ReactNode,
  useContext,
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
const loadTheme = getCodeTheme().then((theme) => {
  cachedCodeTheme = theme;
  return theme;
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
function CodeBlockFallback({
  code,
  language,
}: {
  code: string;
  language: string;
}) {
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
function SyntaxHighlightedCode({
  code,
  language,
  ...props
}: {
  code: string;
  language: string;
}) {
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

const MemoizedParagraph = memo(function MemoizedParagraph({
  text,
  context,
}: {
  text: string;
  context: MarkdownContextValue;
}) {
  return <p className="leading-relaxed">{processTextWithFilePaths(text, context)}</p>;
});

const MemoizedCodeBlock = memo(function MemoizedCodeBlock({
  language,
  code,
}: {
  language: string;
  code: string;
}) {
  return (
    <div className="relative group bg-background border border-[var(--border-medium)] rounded text-sm overflow-auto max-h-64">
      <div className="absolute right-3 top-3 flex items-center gap-2">
        <CopyButton
          content={code}
          className="opacity-0 group-hover:opacity-100 transition-opacity"
        />
        {language && (
          <div className="text-[11px] text-muted-foreground uppercase font-mono font-semibold bg-card px-2 py-1 rounded">
            {language}
          </div>
        )}
      </div>
      <pre className="font-mono text-muted-foreground whitespace-pre-wrap break-words p-5 pt-10">
        {code}
      </pre>
    </div>
  );
});

/** Lightweight renderer for streaming content - minimal parsing overhead */
function StreamingMarkdown({
  content,
  sessionId,
  workingDirectory,
}: {
  content: string;
  sessionId?: string;
  workingDirectory?: string;
}) {
  const fileIndex = useFileIndex(workingDirectory);
  const context = useMemo(
    () => ({ sessionId, workingDirectory, fileIndex: fileIndex ?? undefined }),
    [sessionId, workingDirectory, fileIndex]
  );

  return (
    <div className="space-y-3 text-[14px] font-medium text-foreground/85 break-words leading-relaxed">
      {content.split("\n\n").map((paragraph, idx) => {
        // Detect code blocks (triple backticks)
        if (paragraph.trim().startsWith("```") && paragraph.trim().endsWith("```")) {
          const match = /```(\w*)\n([\s\S]*?)\n```/.exec(paragraph);
          if (match) {
            const [, language, code] = match;
            const trimmedCode = code.trim();
            return (
              <MemoizedCodeBlock
                // biome-ignore lint/suspicious/noArrayIndexKey: paragraphs are in fixed order
                key={idx}
                language={language}
                code={trimmedCode}
              />
            );
          }
        }

        // Regular paragraph
        if (paragraph.trim()) {
          return (
            <MemoizedParagraph
              // biome-ignore lint/suspicious/noArrayIndexKey: paragraphs are in fixed order
              key={idx}
              text={paragraph}
              context={context}
            />
          );
        }
        return null;
      })}
    </div>
  );
}

export const Markdown = memo(function Markdown({
  content,
  className,
  streaming,
  sessionId,
  workingDirectory,
}: MarkdownProps) {
  const fileIndex = useFileIndex(workingDirectory);

  const contextValue = { sessionId, workingDirectory, fileIndex: fileIndex ?? undefined };

  // Use lightweight renderer while streaming
  if (streaming) {
    return (
      <StreamingMarkdown
        content={content}
        sessionId={sessionId}
        workingDirectory={workingDirectory}
      />
    );
  }

  return (
    <MarkdownContext.Provider value={contextValue}>
      <div
        className={cn(
          "max-w-none break-words overflow-hidden text-foreground leading-relaxed",
          className
        )}
      >
        <ReactMarkdown
          remarkPlugins={[remarkGfm]}
          components={{
            code: CodeBlock,
            // Headings
            h1: ({ children }) => (
              <h1 className="text-2xl font-bold text-foreground mt-6 mb-3 first:mt-0 pb-2 border-b border-[var(--border-medium)]">
                {children}
              </h1>
            ),
            h2: ({ children }) => (
              <h2 className="text-lg font-bold text-accent mt-5 mb-3 first:mt-0 pb-2 border-b border-[var(--border-subtle)] flex items-center gap-2">
                <span className="w-1 h-5 bg-accent rounded-full" />
                {children}
              </h2>
            ),
            h3: ({ children }) => (
              <h3 className="text-base font-semibold text-muted-foreground mt-4 mb-2 first:mt-0 pl-3 border-l-2 border-accent">
                {children}
              </h3>
            ),
            // Paragraphs
            p: ({ children }) => (
              <p className="text-foreground mb-3 last:mb-0 leading-relaxed">
                {typeof children === "string"
                  ? processTextWithFilePaths(children, contextValue)
                  : children}
              </p>
            ),
            // Lists
            ul: ({ children }) => (
              <ul className="list-disc list-outside text-foreground mb-3 space-y-2 pl-6">
                {children}
              </ul>
            ),
            ol: ({ children }) => (
              <ol className="list-decimal list-outside text-foreground mb-3 space-y-2 pl-6">
                {children}
              </ol>
            ),
            li: ({ children }) => (
              <li className="text-foreground leading-relaxed">
                {typeof children === "string"
                  ? processTextWithFilePaths(children, contextValue)
                  : children}
              </li>
            ),
            // Links
            a: ({ href, children }) => (
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
            blockquote: ({ children }) => (
              <blockquote className="border-l-4 border-accent bg-[var(--accent-dim)] pl-4 py-2 my-3 text-muted-foreground italic rounded-r">
                {typeof children === "string"
                  ? processTextWithFilePaths(children, contextValue)
                  : children}
              </blockquote>
            ),
            // Horizontal rule
            hr: () => <hr className="my-4 border-[var(--border-medium)]" />,
            // Strong and emphasis
            strong: ({ children }) => <strong className="font-bold text-accent">{children}</strong>,
            em: ({ children }) => <em className="italic text-[var(--success)]">{children}</em>,
            // Tables
            table: ({ children }) => (
              <div className="overflow-x-auto my-3">
                <table className="border-collapse text-[13px]">{children}</table>
              </div>
            ),
            thead: ({ children }) => (
              <thead className="bg-muted/50 border-b border-[var(--border-subtle)]">
                {children}
              </thead>
            ),
            tbody: ({ children }) => <tbody>{children}</tbody>,
            tr: ({ children }) => (
              <tr className="border-b border-[var(--border-subtle)] last:border-b-0 [tbody>&]:hover:bg-muted/30">
                {children}
              </tr>
            ),
            th: ({ children }) => (
              <th className="px-3 py-1.5 text-left text-foreground/80 font-medium text-[12px] uppercase tracking-wide">
                {typeof children === "string"
                  ? processTextWithFilePaths(children, contextValue)
                  : children}
              </th>
            ),
            td: ({ children }) => (
              <td className="px-3 py-2 text-muted-foreground">
                {typeof children === "string"
                  ? processTextWithFilePaths(children, contextValue)
                  : children}
              </td>
            ),
          }}
        >
          {content}
        </ReactMarkdown>
      </div>
    </MarkdownContext.Provider>
  );
});
