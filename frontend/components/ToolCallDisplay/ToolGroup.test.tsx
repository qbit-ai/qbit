import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import type { AnyToolCall, ToolGroup as ToolGroupType } from "@/lib/toolGrouping";
import type { ActiveToolCall } from "@/store";
import { ToolGroup } from "./ToolGroup";

// Mock the ToolItem component since we're testing ToolGroup structure
vi.mock("./ToolCallDisplay", () => ({
  ToolItem: ({ tool }: { tool: AnyToolCall }) => <div data-testid="tool-item">{tool.name}</div>,
}));

function createToolCall(
  id: string,
  name: string,
  status: ActiveToolCall["status"] = "completed"
): ActiveToolCall {
  return {
    id,
    name,
    args: { path: "test.txt" },
    status,
    startedAt: new Date().toISOString(),
    completedAt: new Date().toISOString(),
  };
}

describe("ToolGroup", () => {
  it("renders the group header with tool name and count", () => {
    const group: ToolGroupType = {
      type: "tool_group",
      toolName: "read_file",
      tools: [createToolCall("1", "read_file"), createToolCall("2", "read_file")],
    };

    render(<ToolGroup group={group} />);

    expect(screen.getByText("read_file")).toBeInTheDocument();
    expect(screen.getByText("×2")).toBeInTheDocument();
  });

  it("is collapsed by default for completed tools", () => {
    const group: ToolGroupType = {
      type: "tool_group",
      toolName: "read_file",
      tools: [createToolCall("1", "read_file", "completed")],
    };

    render(<ToolGroup group={group} />);

    // The individual tool items should not be visible initially (CollapsibleContent is hidden)
    // Note: Radix Collapsible might render content with hidden attribute or not render at all.
    // We check if the trigger is present.
    const trigger = screen.getByText("read_file").closest("div[data-state]");
    expect(trigger).toHaveAttribute("data-state", "closed");
  });

  it("auto-expands if any tool is running", () => {
    const group: ToolGroupType = {
      type: "tool_group",
      toolName: "read_file",
      tools: [createToolCall("1", "read_file", "running")],
    };

    render(<ToolGroup group={group} />);

    // Should be open
    const trigger = screen.getByText("read_file").closest("div[data-state]");
    expect(trigger).toHaveAttribute("data-state", "open");
  });

  it("auto-expands if any tool has error", () => {
    const group: ToolGroupType = {
      type: "tool_group",
      toolName: "read_file",
      tools: [createToolCall("1", "read_file", "error")],
    };

    render(<ToolGroup group={group} />);

    // Should be open
    const trigger = screen.getByText("read_file").closest("div[data-state]");
    expect(trigger).toHaveAttribute("data-state", "open");
  });

  it("toggles expansion on click", () => {
    const group: ToolGroupType = {
      type: "tool_group",
      toolName: "read_file",
      tools: [createToolCall("1", "read_file", "completed")],
    };

    render(<ToolGroup group={group} />);

    const trigger = screen.getByText("read_file");

    // Click to open
    fireEvent.click(trigger);
    expect(trigger.closest("div[data-state]")).toHaveAttribute("data-state", "open");

    // Click to close
    fireEvent.click(trigger);
    expect(trigger.closest("div[data-state]")).toHaveAttribute("data-state", "closed");
  });

  it("shows preview of arguments when collapsed", () => {
    const group: ToolGroupType = {
      type: "tool_group",
      toolName: "read_file",
      tools: [
        { ...createToolCall("1", "read_file"), args: { path: "file1.txt" } },
        { ...createToolCall("2", "read_file"), args: { path: "file2.txt" } },
      ],
    };

    render(<ToolGroup group={group} />);

    // Should see file names in preview
    expect(screen.getByText(/file1\.txt/)).toBeInTheDocument();
    expect(screen.getByText(/file2\.txt/)).toBeInTheDocument();
  });
});
