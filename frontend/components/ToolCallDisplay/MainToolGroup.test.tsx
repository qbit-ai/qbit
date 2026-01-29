import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import type { ActiveToolCall } from "@/store";
import { MainToolGroup } from "./MainToolGroup";

describe("MainToolGroup", () => {
  it("shows the 3 most recent tool calls in chronological order (oldest → newest)", () => {
    const tools: ActiveToolCall[] = Array.from({ length: 10 }, (_, i) => {
      const n = i + 1;
      return {
        id: `tool-${n}`,
        name: "read_file",
        status: "completed" as const,
        startedAt: new Date(2020, 0, n).toISOString(),
        args: { path: `tool-${n}` },
      };
    });

    render(
      <MainToolGroup tools={tools} onViewToolDetails={vi.fn()} onViewGroupDetails={vi.fn()} />
    );

    // The preview should include only the last 3 tools: 8, 9, 10
    // and display them in that order.
    const rows = screen.getAllByText("read_file");
    expect(rows).toHaveLength(3);

    // Use the tool IDs as the primaryArg so we can assert ordering from rendered text.
    const text =
      screen.getByText("tool-8").closest("div")?.parentElement?.parentElement?.textContent ?? "";

    const idx8 = text.indexOf("tool-8");
    const idx9 = text.indexOf("tool-9");
    const idx10 = text.indexOf("tool-10");

    expect(idx8).toBeGreaterThanOrEqual(0);
    expect(idx9).toBeGreaterThan(idx8);
    expect(idx10).toBeGreaterThan(idx9);
  });

  it("calls onViewToolDetails when a tool is clicked", () => {
    const tools: ActiveToolCall[] = [
      {
        id: "tool-1",
        name: "read_file",
        status: "completed",
        startedAt: new Date().toISOString(),
        args: { path: "test.txt" },
      },
    ];

    const onViewToolDetails = vi.fn();
    render(
      <MainToolGroup
        tools={tools}
        onViewToolDetails={onViewToolDetails}
        onViewGroupDetails={vi.fn()}
      />
    );

    const viewDetailsButton = screen.getByTitle("View details");
    viewDetailsButton.click();

    expect(onViewToolDetails).toHaveBeenCalledWith(tools[0]);
  });

  it("calls onViewGroupDetails when 'View all' is clicked", () => {
    // Create 4 tools to trigger the "View all" button (limit is 3)
    const tools: ActiveToolCall[] = Array.from({ length: 4 }, (_, i) => ({
      id: `tool-${i}`,
      name: "read_file",
      status: "completed",
      startedAt: new Date().toISOString(),
      args: { path: `file-${i}.txt` },
    }));

    const onViewGroupDetails = vi.fn();
    render(
      <MainToolGroup
        tools={tools}
        onViewToolDetails={vi.fn()}
        onViewGroupDetails={onViewGroupDetails}
      />
    );

    const viewAllButton = screen.getByText("View all");
    viewAllButton.click();

    expect(onViewGroupDetails).toHaveBeenCalled();
  });

  it("shows correct count of hidden tools", () => {
    const tools: ActiveToolCall[] = Array.from({ length: 5 }, (_, i) => ({
      id: `tool-${i}`,
      name: "read_file",
      status: "completed",
      startedAt: new Date().toISOString(),
      args: { path: `file-${i}.txt` },
    }));

    render(
      <MainToolGroup tools={tools} onViewToolDetails={vi.fn()} onViewGroupDetails={vi.fn()} />
    );

    // 5 tools total, 3 shown -> 2 hidden
    expect(screen.getByText(/2 previous tool calls/)).toBeInTheDocument();
  });
});
