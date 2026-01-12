import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { MainToolGroup } from "./MainToolGroup";

describe("MainToolGroup", () => {
  it("shows the 3 most recent tool calls in chronological order (oldest → newest)", () => {
    const tools = Array.from({ length: 10 }, (_, i) => {
      const n = i + 1;
      return {
        id: `tool-${n}`,
        name: "read_file",
        status: "completed",
        startedAt: new Date(2020, 0, n).toISOString(),
        args: { path: `tool-${n}` },
      };
    });

    render(
      <MainToolGroup
        tools={tools as any}
        onViewToolDetails={vi.fn()}
        onViewGroupDetails={vi.fn()}
      />
    );

    // The preview should include only the last 3 tools: 8, 9, 10
    // and display them in that order.
    const rows = screen.getAllByText("read_file");
    expect(rows).toHaveLength(3);

    // Use the tool IDs as the primaryArg so we can assert ordering from rendered text.
    const text = screen.getByText("tool-8").closest("div")?.parentElement?.parentElement?.textContent ?? "";

    const idx8 = text.indexOf("tool-8");
    const idx9 = text.indexOf("tool-9");
    const idx10 = text.indexOf("tool-10");

    expect(idx8).toBeGreaterThanOrEqual(0);
    expect(idx9).toBeGreaterThan(idx8);
    expect(idx10).toBeGreaterThan(idx9);
  });
});
