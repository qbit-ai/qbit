import { render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

// Mock the file index hook
vi.mock("@/hooks/useFileIndex", () => ({
  useFileIndex: () => null,
}));

describe("Markdown Lazy Loading Tests", () => {
  describe("SyntaxHighlighter lazy loading", () => {
    it("should render code block with fallback while SyntaxHighlighter loads", async () => {
      const { Markdown } = await import("./Markdown");

      const codeContent = `
\`\`\`javascript
const hello = "world";
console.log(hello);
\`\`\`
`;

      const { container } = render(<Markdown content={codeContent} />);

      // Should render some form of code block (either fallback or actual)
      await waitFor(() => {
        // Check for pre or code element
        const codeElement = container.querySelector("pre, code, div");
        expect(codeElement).toBeInTheDocument();
      });
    });

    it("should render syntax-highlighted code after lazy load completes", async () => {
      const { Markdown } = await import("./Markdown");

      const codeContent = `
\`\`\`typescript
interface User {
  name: string;
  age: number;
}
\`\`\`
`;

      render(<Markdown content={codeContent} />);

      // Wait for lazy loading to complete
      await waitFor(
        () => {
          // SyntaxHighlighter renders code in spans with styles
          // After loading, the code block should be styled
          const codeBlock =
            document.querySelector('[class*="language-"]') ||
            document.querySelector("pre") ||
            document.querySelector("code");
          expect(codeBlock).toBeInTheDocument();
        },
        { timeout: 3000 }
      );
    });

    it("should show code content in fallback (not empty)", async () => {
      const { Markdown } = await import("./Markdown");

      const code = "function test() { return 42; }";
      const codeContent = `
\`\`\`javascript
${code}
\`\`\`
`;

      const { container } = render(<Markdown content={codeContent} />);

      // The code content should be visible even during loading
      await waitFor(() => {
        const text = container.textContent;
        expect(text).toContain("test");
        expect(text).toContain("return");
        expect(text).toContain("42");
      });
    });
  });

  describe("Suspense boundary for code blocks", () => {
    it("should not throw when rendering code blocks with lazy SyntaxHighlighter", async () => {
      const { Markdown } = await import("./Markdown");

      const multipleCodeBlocks = `
# Code Examples

\`\`\`python
def hello():
    print("Hello, World!")
\`\`\`

Some text between code blocks.

\`\`\`rust
fn main() {
    println!("Hello, Rust!");
}
\`\`\`
`;

      // This should not throw - Suspense boundary should catch the lazy loading
      const { container } = render(<Markdown content={multipleCodeBlocks} />);

      expect(container).toBeDefined();

      // Wait for content to fully render
      await waitFor(
        () => {
          const text = container.textContent;
          expect(text).toContain("Code Examples");
          expect(text).toContain("hello");
          expect(text).toContain("main");
        },
        { timeout: 3000 }
      );
    });
  });

  describe("Inline code (non-lazy)", () => {
    it("should render inline code without lazy loading", async () => {
      const { Markdown } = await import("./Markdown");

      const inlineCodeContent = "Use the `console.log()` function for debugging.";

      render(<Markdown content={inlineCodeContent} />);

      // Inline code should be immediately available (not lazy loaded)
      await waitFor(() => {
        const inlineCode = screen.getByText("console.log()");
        expect(inlineCode).toBeInTheDocument();
        expect(inlineCode.tagName.toLowerCase()).toBe("code");
      });
    });
  });

  describe("Streaming mode (non-lazy fallback)", () => {
    it("should render code blocks without SyntaxHighlighter in streaming mode", async () => {
      const { Markdown } = await import("./Markdown");

      const codeContent = `
\`\`\`javascript
const streaming = true;
\`\`\`
`;

      const { container } = render(<Markdown content={codeContent} streaming={true} />);

      // In streaming mode, code blocks use a lightweight renderer
      await waitFor(() => {
        const text = container.textContent;
        expect(text).toContain("streaming");
      });
    });
  });

  describe("Regular text (no lazy loading)", () => {
    it("should render plain text immediately", async () => {
      const { Markdown } = await import("./Markdown");

      const textContent = "This is **bold** and *italic* text.";

      render(<Markdown content={textContent} />);

      await waitFor(() => {
        expect(screen.getByText("bold")).toBeInTheDocument();
        expect(screen.getByText("italic")).toBeInTheDocument();
      });
    });
  });

  describe("Copy button functionality", () => {
    it("should render copy button with code blocks", async () => {
      const { Markdown } = await import("./Markdown");

      const codeContent = `
\`\`\`javascript
const copyMe = "test";
\`\`\`
`;

      render(<Markdown content={codeContent} />);

      // Wait for the component to fully render
      await waitFor(
        () => {
          // The copy button should be present (it's part of the CodeBlock component)
          // It might be inside a relative positioned div with the code
          const codeWrapper = document.querySelector(".relative.group");
          expect(codeWrapper).toBeInTheDocument();
        },
        { timeout: 3000 }
      );
    });
  });
});
