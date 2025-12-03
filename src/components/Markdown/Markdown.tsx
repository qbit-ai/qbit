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
          <div className="absolute right-3 top-3 text-[11px] text-[#565f89] uppercase font-mono font-semibold bg-[#0f1018] px-2 py-1 rounded">
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
            background: "#1a1b26",
            border: "1px solid #27293d",
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
        "px-2 py-1 rounded bg-[#1a1b26] border border-[#27293d] text-[#7aa2f7] font-mono text-[0.9em] whitespace-nowrap",
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
    <div className="space-y-3 text-[#c0caf5] break-words leading-relaxed">
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
                className="relative bg-[#1a1b26] border border-[#27293d] rounded text-sm overflow-auto max-h-64"
              >
                {language && (
                  <div className="absolute right-3 top-3 text-[11px] text-[#565f89] uppercase font-mono font-semibold bg-[#0f1018] px-2 py-1 rounded">
                    {language}
                  </div>
                )}
                <pre className="font-mono text-[#9aa5ce] whitespace-pre-wrap break-words p-5 pt-10">
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
              className="text-[#c0caf5] leading-relaxed"
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
        "max-w-none break-words overflow-hidden text-[#c0caf5] leading-relaxed",
        className
      )}
    >
      <ReactMarkdown
        remarkPlugins={[remarkGfm]}
        components={{
          code: CodeBlock,
          // Headings
          h1: ({ children }) => (
            <h1 className="text-2xl font-bold text-[#c0caf5] mt-6 mb-3 first:mt-0 pb-2 border-b border-[#27293d]">
              {children}
            </h1>
          ),
          h2: ({ children }) => (
            <h2 className="text-lg font-bold text-[#7aa2f7] mt-5 mb-3 first:mt-0 pb-2 border-b border-[#27293d]/50 flex items-center gap-2">
              <span className="w-1 h-5 bg-[#7aa2f7] rounded-full" />
              {children}
            </h2>
          ),
          h3: ({ children }) => (
            <h3 className="text-base font-semibold text-[#a9b1d6] mt-4 mb-2 first:mt-0 pl-3 border-l-2 border-[#bb9af7]">
              {children}
            </h3>
          ),
          // Paragraphs
          p: ({ children }) => (
            <p className="text-[#c0caf5] mb-3 last:mb-0 leading-relaxed">{children}</p>
          ),
          // Lists
          ul: ({ children }) => (
            <ul className="list-disc list-outside text-[#c0caf5] mb-3 space-y-2 pl-6">
              {children}
            </ul>
          ),
          ol: ({ children }) => (
            <ol className="list-decimal list-outside text-[#c0caf5] mb-3 space-y-2 pl-6">
              {children}
            </ol>
          ),
          li: ({ children }) => <li className="text-[#c0caf5] leading-relaxed">{children}</li>,
          // Links
          a: ({ href, children }) => (
            <a
              href={href}
              target="_blank"
              rel="noopener noreferrer"
              className="text-[#7aa2f7] hover:text-[#bb9af7] hover:underline transition-colors"
            >
              {children}
            </a>
          ),
          // Blockquotes
          blockquote: ({ children }) => (
            <blockquote className="border-l-4 border-[#bb9af7] bg-[#bb9af7]/5 pl-4 py-2 my-3 text-[#a9b1d6] italic rounded-r">
              {children}
            </blockquote>
          ),
          // Horizontal rule
          hr: () => <hr className="my-4 border-[#27293d]" />,
          // Strong and emphasis
          strong: ({ children }) => (
            <strong className="font-bold text-[#7aa2f7]">{children}</strong>
          ),
          em: ({ children }) => <em className="italic text-[#bb9af7]">{children}</em>,
          // Tables
          table: ({ children }) => (
            <div className="overflow-x-auto my-4 rounded border border-[#27293d]">
              <table className="min-w-full border-collapse text-sm">{children}</table>
            </div>
          ),
          thead: ({ children }) => (
            <thead className="bg-[#1f2335] border-b border-[#27293d]">{children}</thead>
          ),
          tbody: ({ children }) => <tbody>{children}</tbody>,
          tr: ({ children }) => (
            <tr className="border-b border-[#27293d] last:border-b-0">{children}</tr>
          ),
          th: ({ children }) => (
            <th className="px-4 py-3 text-left text-[#7aa2f7] font-semibold border-r border-[#27293d] last:border-r-0">
              {children}
            </th>
          ),
          td: ({ children }) => (
            <td className="px-4 py-2 text-[#a9b1d6] border-r border-[#27293d] last:border-r-0">
              {children}
            </td>
          ),
        }}
      >
        {content}
      </ReactMarkdown>
    </div>
  );
});
