import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { SystemHooksCard } from "./SystemHooksCard";

describe("SystemHooksCard", () => {
  describe("rendering", () => {
    it("should render with correct header text", () => {
      render(<SystemHooksCard hooks={["hook1"]} />);

      expect(screen.getByText(/System hooks injected/)).toBeInTheDocument();
    });

    it("should show count when hooks are provided", () => {
      render(<SystemHooksCard hooks={["hook1", "hook2", "hook3"]} />);

      expect(screen.getByText(/\(3\)/)).toBeInTheDocument();
    });

    it("should not show count when no hooks are provided", () => {
      render(<SystemHooksCard hooks={[]} />);

      expect(screen.queryByText(/\(\d+\)/)).not.toBeInTheDocument();
    });

    it("should render sparkles icon", () => {
      render(<SystemHooksCard hooks={["hook1"]} />);

      // The icon should be present (check by its container class)
      const icon = document.querySelector(".text-\\[var\\(--ansi-yellow\\)\\]");
      expect(icon).toBeInTheDocument();
    });
  });

  describe("expandable details", () => {
    it("should show 'View hook' for single hook", () => {
      render(<SystemHooksCard hooks={["single hook"]} />);

      expect(screen.getByText("View hook")).toBeInTheDocument();
    });

    it("should show 'View hooks' for multiple hooks", () => {
      render(<SystemHooksCard hooks={["hook1", "hook2"]} />);

      expect(screen.getByText("View hooks")).toBeInTheDocument();
    });

    it("should not show details section when no hooks", () => {
      render(<SystemHooksCard hooks={[]} />);

      expect(screen.queryByText(/View hook/)).not.toBeInTheDocument();
    });

    it("should expand to show hook content when clicked", async () => {
      const user = userEvent.setup();
      render(<SystemHooksCard hooks={["Test hook content"]} />);

      // Initially, hook content should not be visible (in closed details)
      const details = document.querySelector("details");
      expect(details).not.toHaveAttribute("open");

      // Click to expand
      await user.click(screen.getByText("View hook"));

      // Now the hook content should be visible
      expect(screen.getByText("Test hook content")).toBeInTheDocument();
    });

    it("should render all hooks when expanded", async () => {
      const user = userEvent.setup();
      render(<SystemHooksCard hooks={["Hook A", "Hook B", "Hook C"]} />);

      await user.click(screen.getByText("View hooks"));

      expect(screen.getByText("Hook A")).toBeInTheDocument();
      expect(screen.getByText("Hook B")).toBeInTheDocument();
      expect(screen.getByText("Hook C")).toBeInTheDocument();
    });
  });

  describe("styling", () => {
    it("should have yellow left border", () => {
      render(<SystemHooksCard hooks={["hook"]} />);

      const card = document.querySelector(".border-l-\\[var\\(--ansi-yellow\\)\\]");
      expect(card).toBeInTheDocument();
    });

    it("should have yellow background tint", () => {
      render(<SystemHooksCard hooks={["hook"]} />);

      const card = document.querySelector(".bg-\\[var\\(--ansi-yellow\\)\\]\\/10");
      expect(card).toBeInTheDocument();
    });
  });

  describe("hook content formatting", () => {
    it("should preserve whitespace in hook content", async () => {
      const user = userEvent.setup();
      const multilineHook = "Line 1\n  Line 2 (indented)\nLine 3";
      render(<SystemHooksCard hooks={[multilineHook]} />);

      await user.click(screen.getByText("View hook"));

      const pre = document.querySelector("pre");
      expect(pre).toBeInTheDocument();
      expect(pre).toHaveClass("whitespace-pre-wrap");
    });
  });
});
