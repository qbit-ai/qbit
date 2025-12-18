import { type ComponentPropsWithoutRef, memo } from "react";
import ReactMarkdown from "react-markdown";
import { Prism as SyntaxHighlighter } from "react-syntax-highlighter";
import { oneDark } from "react-syntax-highlighter/dist/esm/styles/prism";
import remarkGfm from "remark-gfm";
import { cn } from "@/lib/utils";

interface MarkdownProps {
  content: string;
  className?: string;
  /** Lightweight mode for streaming content - avoids expensive parsing */
  streaming?: boolean;
}

function CodeBlock({
  inline,
  className,
  children,
  ...props
}: ComponentPropsWithoutRef<"code"> & { inline?: boolean }) {
  const match = /language-(\w+)/.exec(className || "");
  const language = match ? match[1] : "";
  const codeString = String(children).replace(/\n$/, "");

  if (!inline && (match || codeString.includes("\n"))) {
    return (
      <div className="relative group my-4">
        {language && (
          <div className="absolute right-3 top-3 text-[11px] text-muted-foreground uppercase font-mono font-semibold bg-background px-2 py-1 rounded">
            {language}
          </div>
        )}
        <SyntaxHighlighter
          // biome-ignore lint/suspicious/noExplicitAny: SyntaxHighlighter style prop typing is incompatible
          style={oneDark as any}
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
          {codeString}
        </SyntaxHighlighter>
      </div>
    );
  }

  return (
    <code
      className={cn(
        "px-1.5 py-0.5 rounded bg-background border border-[var(--border-medium)] text-foreground/80 font-mono text-[0.85em] whitespace-nowrap",
        className
      )}
      {...props}
    >
      {children}
    </code>
  );
}

/** Lightweight renderer for streaming content - minimal parsing overhead */
function StreamingMarkdown({ content }: { content: string }) {
  return (
    <div className="space-y-3 text-[14px] font-medium text-foreground/85 break-words leading-relaxed">
      {content.split("\n\n").map((paragraph, idx) => {
        // Detect code blocks (triple backticks)
        if (paragraph.trim().startsWith("```") && paragraph.trim().endsWith("```")) {
          const match = /```(\w*)\n([\s\S]*?)\n```/.exec(paragraph);
          if (match) {
            const [, language, code] = match;
            return (
              <div
                // biome-ignore lint/suspicious/noArrayIndexKey: paragraphs are in fixed order
                key={idx}
                className="relative bg-background border border-[var(--border-medium)] rounded text-sm overflow-auto max-h-64"
              >
                {language && (
                  <div className="absolute right-3 top-3 text-[11px] text-muted-foreground uppercase font-mono font-semibold bg-card px-2 py-1 rounded">
                    {language}
                  </div>
                )}
                <pre className="font-mono text-muted-foreground whitespace-pre-wrap break-words p-5 pt-10">
                  {code.trim()}
                </pre>
              </div>
            );
          }
        }

        // Regular paragraph
        if (paragraph.trim()) {
          return (
            <p
              // biome-ignore lint/suspicious/noArrayIndexKey: paragraphs are in fixed order
              key={idx}
              className="leading-relaxed"
            >
              {paragraph}
            </p>
          );
        }
        return null;
      })}
    </div>
  );
}

export const Markdown = memo(function Markdown({ content, className, streaming }: MarkdownProps) {
  // Use lightweight renderer while streaming
  if (streaming) {
    return <StreamingMarkdown content={content} />;
  }

  return (
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
            <p className="text-foreground mb-3 last:mb-0 leading-relaxed">{children}</p>
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
          li: ({ children }) => <li className="text-foreground leading-relaxed">{children}</li>,
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
              {children}
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
            <thead className="bg-muted/50 border-b border-[var(--border-subtle)]">{children}</thead>
          ),
          tbody: ({ children }) => <tbody>{children}</tbody>,
          tr: ({ children }) => (
            <tr className="border-b border-[var(--border-subtle)] last:border-b-0 [tbody>&]:hover:bg-muted/30">
              {children}
            </tr>
          ),
          th: ({ children }) => (
            <th className="px-3 py-1.5 text-left text-foreground/80 font-medium text-[12px] uppercase tracking-wide">
              {children}
            </th>
          ),
          td: ({ children }) => <td className="px-3 py-2 text-muted-foreground">{children}</td>,
        }}
      >
        {content}
      </ReactMarkdown>
    </div>
  );
});
