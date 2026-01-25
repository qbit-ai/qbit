import { describe, expect, it } from "vitest";
import type { StreamingBlock } from "@/store";
import { groupConsecutiveTools, groupConsecutiveToolsByAny } from "./toolGrouping";

function createToolBlock(name: string, id: string): StreamingBlock {
  return {
    type: "tool",
    toolCall: {
      id,
      name,
      args: {},
      status: "completed",
      startedAt: new Date().toISOString(),
    },
  };
}

function createTextBlock(content: string): StreamingBlock {
  return { type: "text", content };
}

describe("groupConsecutiveToolsByAny", () => {
  it("groups consecutive tool calls together", () => {
    const blocks: StreamingBlock[] = [
      createToolBlock("read_file", "1"),
      createToolBlock("read_file", "2"),
      createToolBlock("list_files", "3"),
    ];

    const result = groupConsecutiveToolsByAny(blocks);

    expect(result).toHaveLength(1);
    expect(result[0].type).toBe("tool_group");
    if (result[0].type === "tool_group") {
      expect(result[0].tools).toHaveLength(3);
    }
  });

  it("keeps single tools as individual", () => {
    const blocks: StreamingBlock[] = [createToolBlock("read_file", "1")];

    const result = groupConsecutiveToolsByAny(blocks);

    expect(result).toHaveLength(1);
    expect(result[0].type).toBe("tool");
  });

  it("breaks groups on text content", () => {
    const blocks: StreamingBlock[] = [
      createToolBlock("read_file", "1"),
      createTextBlock("Some text here"),
      createToolBlock("read_file", "2"),
    ];

    const result = groupConsecutiveToolsByAny(blocks);

    expect(result).toHaveLength(3);
    expect(result[0].type).toBe("tool");
    expect(result[1].type).toBe("text");
    expect(result[2].type).toBe("tool");
  });

  it("does NOT break groups on whitespace-only text", () => {
    const blocks: StreamingBlock[] = [
      createToolBlock("read_file", "1"),
      createTextBlock("\n"),
      createToolBlock("read_file", "2"),
      createTextBlock("  \n  "),
      createToolBlock("list_files", "3"),
    ];

    const result = groupConsecutiveToolsByAny(blocks);

    expect(result).toHaveLength(1);
    expect(result[0].type).toBe("tool_group");
    if (result[0].type === "tool_group") {
      expect(result[0].tools).toHaveLength(3);
    }
  });

  it("handles text before tool group", () => {
    const blocks: StreamingBlock[] = [
      createTextBlock("Let me check these files:"),
      createToolBlock("read_file", "1"),
      createToolBlock("read_file", "2"),
    ];

    const result = groupConsecutiveToolsByAny(blocks);

    expect(result).toHaveLength(2);
    expect(result[0].type).toBe("text");
    expect(result[1].type).toBe("tool_group");
  });

  it("handles text after tool group", () => {
    const blocks: StreamingBlock[] = [
      createToolBlock("read_file", "1"),
      createToolBlock("read_file", "2"),
      createTextBlock("Now let me analyze the results."),
    ];

    const result = groupConsecutiveToolsByAny(blocks);

    expect(result).toHaveLength(2);
    expect(result[0].type).toBe("tool_group");
    expect(result[1].type).toBe("text");
  });

  it("accumulates whitespace before text", () => {
    const blocks: StreamingBlock[] = [createTextBlock("\n\n"), createTextBlock("Hello")];

    const result = groupConsecutiveToolsByAny(blocks);

    expect(result).toHaveLength(1);
    expect(result[0].type).toBe("text");
    if (result[0].type === "text") {
      expect(result[0].content).toBe("\n\nHello");
    }
  });

  it("preserves trailing whitespace", () => {
    const blocks: StreamingBlock[] = [createToolBlock("read_file", "1"), createTextBlock("\n")];

    const result = groupConsecutiveToolsByAny(blocks);

    expect(result).toHaveLength(2);
    expect(result[0].type).toBe("tool");
    expect(result[1].type).toBe("text");
    if (result[1].type === "text") {
      expect(result[1].content).toBe("\n");
    }
  });

  it("handles complex interleaved sequence", () => {
    const blocks: StreamingBlock[] = [
      createTextBlock("Let me check:"),
      createToolBlock("read_file", "1"),
      createTextBlock("\n"), // whitespace - should NOT break
      createToolBlock("read_file", "2"),
      createTextBlock("\n\n"), // whitespace - should NOT break
      createToolBlock("list_files", "3"),
      createTextBlock("Now analyzing..."),
      createToolBlock("read_file", "4"),
    ];

    const result = groupConsecutiveToolsByAny(blocks);

    expect(result).toHaveLength(4);
    expect(result[0].type).toBe("text"); // "Let me check:"
    expect(result[1].type).toBe("tool_group"); // 3 tools grouped
    if (result[1].type === "tool_group") {
      expect(result[1].tools).toHaveLength(3);
    }
    expect(result[2].type).toBe("text"); // "Now analyzing..."
    expect(result[3].type).toBe("tool"); // single tool
  });
});

describe("groupConsecutiveTools", () => {
  it("groups consecutive same-name tool calls", () => {
    const blocks: StreamingBlock[] = [
      createToolBlock("read_file", "1"),
      createToolBlock("read_file", "2"),
      createToolBlock("read_file", "3"),
    ];

    const result = groupConsecutiveTools(blocks);

    expect(result).toHaveLength(1);
    expect(result[0].type).toBe("tool_group");
    if (result[0].type === "tool_group") {
      expect(result[0].tools).toHaveLength(3);
      expect(result[0].toolName).toBe("read_file");
    }
  });

  it("separates different tool types", () => {
    const blocks: StreamingBlock[] = [
      createToolBlock("read_file", "1"),
      createToolBlock("read_file", "2"),
      createToolBlock("list_files", "3"),
    ];

    const result = groupConsecutiveTools(blocks);

    expect(result).toHaveLength(2);
    expect(result[0].type).toBe("tool_group");
    expect(result[1].type).toBe("tool");
  });

  it("does NOT break groups on whitespace-only text", () => {
    const blocks: StreamingBlock[] = [
      createToolBlock("read_file", "1"),
      createTextBlock("\n"),
      createToolBlock("read_file", "2"),
    ];

    const result = groupConsecutiveTools(blocks);

    expect(result).toHaveLength(1);
    expect(result[0].type).toBe("tool_group");
    if (result[0].type === "tool_group") {
      expect(result[0].tools).toHaveLength(2);
    }
  });

  it("breaks groups on non-whitespace text", () => {
    const blocks: StreamingBlock[] = [
      createToolBlock("read_file", "1"),
      createTextBlock("Some content"),
      createToolBlock("read_file", "2"),
    ];

    const result = groupConsecutiveTools(blocks);

    expect(result).toHaveLength(3);
    expect(result[0].type).toBe("tool");
    expect(result[1].type).toBe("text");
    expect(result[2].type).toBe("tool");
  });
});
