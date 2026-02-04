import { render } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

// Mock dependencies
vi.mock("@/lib/terminal", () => ({
  liveTerminalManager: {
    getOrCreate: vi.fn(),
    attachToContainer: vi.fn(),
  },
}));

// Import after mocks
import { CODE_STYLE, LiveTerminalBlock } from "./LiveTerminalBlock";

describe("LiveTerminalBlock", () => {
  describe("Static styles optimization", () => {
    it("should use static CODE_STYLE object to prevent re-renders", () => {
      // Check if CODE_STYLE is defined as a module-level constant
      // This verifies the pattern of extracting inline styles
      expect(CODE_STYLE).toBeDefined();
      expect(CODE_STYLE).toEqual({
        fontSize: "12px",
        lineHeight: 1.4,
        fontFamily: "JetBrains Mono, Menlo, Monaco, Consolas, monospace",
      });
    });

    it("should render correctly with command", () => {
      const { container } = render(<LiveTerminalBlock sessionId="test-session" command="ls -la" />);

      expect(container).toBeDefined();
      // Check for the prompt and command text
      const codeEl = container.querySelector("code");
      expect(codeEl).toBeDefined();
      expect(codeEl?.textContent).toContain("$");
      expect(codeEl?.textContent).toContain("ls -la");
    });

    it("should render correctly without command", () => {
      const { container, queryByText } = render(
        <LiveTerminalBlock sessionId="test-session" command={null} />
      );

      expect(container).toBeDefined();
      expect(queryByText("$ ")).toBeNull();
    });

    it("should apply consistent styles across renders", () => {
      const { container, rerender } = render(
        <LiveTerminalBlock sessionId="test-session" command="ls -la" />
      );

      const codeElement = container.querySelector("code");
      const initialStyle = codeElement?.getAttribute("style");

      // Rerender with same props
      rerender(<LiveTerminalBlock sessionId="test-session" command="ls -la" />);

      const codeElementAfter = container.querySelector("code");
      const styleAfter = codeElementAfter?.getAttribute("style");

      // Styles should be identical
      expect(styleAfter).toBe(initialStyle);
    });
  });
});
