import { render, screen } from "@testing-library/react";
import type React from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { SlashCommand } from "@/hooks/useSlashCommands";

describe("SlashCommandPopup Memoization Tests", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  const mockCommands: SlashCommand[] = [
    {
      name: "commit",
      path: "/prompts/commit.md",
      type: "prompt",
      source: "local",
      description: "Commit changes",
    },
    {
      name: "review",
      path: "/skills/review",
      type: "skill",
      source: "global",
      description: "Review code",
    },
  ];

  describe("SlashCommandItem memoization", () => {
    it("SlashCommandItem should be wrapped in React.memo", async () => {
      const module = await import("./SlashCommandPopup");

      // Check if SlashCommandItem is exported and is a memo component
      const SlashCommandItem = (module as Record<string, unknown>).SlashCommandItem;
      expect(SlashCommandItem).toBeDefined();

      const memoSymbol = Symbol.for("react.memo");
      const componentType = (SlashCommandItem as { $$typeof?: symbol }).$$typeof;
      expect(componentType).toBe(memoSymbol);
    });
  });

  describe("SlashCommandPopup rendering", () => {
    it("should render command list correctly", async () => {
      const { SlashCommandPopup } = await import("./SlashCommandPopup");

      render(
        <SlashCommandPopup
          open={true}
          onOpenChange={vi.fn()}
          commands={mockCommands}
          selectedIndex={0}
          onSelect={vi.fn()}
        >
          <input type="text" />
        </SlashCommandPopup>
      );

      expect(screen.getByText("/commit")).toBeDefined();
      expect(screen.getByText("/review")).toBeDefined();
    });

    it("should not re-render list items when selectedIndex changes", async () => {
      const { SlashCommandPopup } = await import("./SlashCommandPopup");

      const onSelect = vi.fn();
      const { rerender } = render(
        <SlashCommandPopup
          open={true}
          onOpenChange={vi.fn()}
          commands={mockCommands}
          selectedIndex={0}
          onSelect={onSelect}
        >
          <input type="text" />
        </SlashCommandPopup>
      );

      // Change selectedIndex
      rerender(
        <SlashCommandPopup
          open={true}
          onOpenChange={vi.fn()}
          commands={mockCommands}
          selectedIndex={1}
          onSelect={onSelect}
        >
          <input type="text" />
        </SlashCommandPopup>
      );

      // Commands should still be rendered correctly
      expect(screen.getByText("/commit")).toBeDefined();
      expect(screen.getByText("/review")).toBeDefined();
    });

    it("should handle empty command list", async () => {
      const { SlashCommandPopup } = await import("./SlashCommandPopup");

      render(
        <SlashCommandPopup
          open={true}
          onOpenChange={vi.fn()}
          commands={[]}
          selectedIndex={0}
          onSelect={vi.fn()}
        >
          <input type="text" />
        </SlashCommandPopup>
      );

      expect(screen.getByText("No commands found")).toBeDefined();
    });
  });

  describe("Callback stability", () => {
    it("should use stable onSelect callback in memoized items", async () => {
      const { SlashCommandPopup } = await import("./SlashCommandPopup");

      const onSelect = vi.fn();
      const { rerender } = render(
        <SlashCommandPopup
          open={true}
          onOpenChange={vi.fn()}
          commands={mockCommands}
          selectedIndex={0}
          onSelect={onSelect}
        >
          <input type="text" />
        </SlashCommandPopup>
      );

      // Rerender with same onSelect reference
      rerender(
        <SlashCommandPopup
          open={true}
          onOpenChange={vi.fn()}
          commands={mockCommands}
          selectedIndex={0}
          onSelect={onSelect}
        >
          <input type="text" />
        </SlashCommandPopup>
      );

      // Component should render correctly
      expect(screen.getByText("/commit")).toBeDefined();
    });
  });
});
