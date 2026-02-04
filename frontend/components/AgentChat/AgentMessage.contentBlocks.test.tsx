import { describe, expect, it } from "vitest";

/**
 * Tests for AgentMessage ContentBlocks Consolidation
 *
 * OPTIMIZATION COMPLETED: AgentMessage previously had 3 cascading useMemo calls:
 * 1. filteredHistory - filters tool calls (was a no-op, now removed)
 * 2. groupedHistory - groups consecutive tools
 * 3. contentBlocks - extracts sub-agent blocks
 *
 * These have been consolidated into a single useMemo that:
 * - Performs single-pass processing (group -> extract -> check hooks)
 * - Eliminates 3 separate dependency comparisons
 * - Removes intermediate array allocations
 * - Returns both contentBlocks and hasSystemHooksInBlocks in one computation
 */

// Import the utilities to test the consolidated approach
import { extractSubAgentBlocks } from "@/lib/timeline";
import { groupConsecutiveToolsByAny } from "@/lib/toolGrouping";
import type { StreamingBlock } from "@/store";

// Empty constants that match the component
const EMPTY_BLOCKS: never[] = [];
const EMPTY_SUB_AGENTS: never[] = [];

/**
 * Helper function that mimics the consolidated single-pass computation.
 * This matches the implementation in AgentMessage.tsx's consolidated useMemo.
 * Returns both contentBlocks and hasSystemHooksInBlocks.
 */
function computeContentBlocks(
  streamingHistory: StreamingBlock[] | undefined,
  subAgents: unknown[] = EMPTY_SUB_AGENTS
): {
  contentBlocks: ReturnType<typeof extractSubAgentBlocks>["contentBlocks"];
  hasSystemHooksInBlocks: boolean;
} {
  // Early return with stable reference for empty case
  if (!streamingHistory?.length) {
    return { contentBlocks: EMPTY_BLOCKS as never[], hasSystemHooksInBlocks: false };
  }

  // Single pass: group -> extract sub-agents -> check for system hooks
  // Note: The filter was a no-op (always returns true), so it's been removed
  const grouped = groupConsecutiveToolsByAny(streamingHistory);
  const { contentBlocks } = extractSubAgentBlocks(grouped, subAgents as never[]);
  const hasSystemHooksInBlocks = contentBlocks.some((block) => block.type === "system_hooks");

  return { contentBlocks, hasSystemHooksInBlocks };
}

// Legacy helper for backward compatibility with existing tests
function computeContentBlocksLegacy(
  streamingHistory: StreamingBlock[] | undefined,
  subAgents: unknown[] = EMPTY_SUB_AGENTS
) {
  return computeContentBlocks(streamingHistory, subAgents).contentBlocks;
}

describe("AgentMessage ContentBlocks Consolidation", () => {
  describe("single-pass computation correctness", () => {
    it("should return empty array for undefined streamingHistory", () => {
      const result = computeContentBlocksLegacy(undefined);
      expect(result).toBe(EMPTY_BLOCKS);
    });

    it("should return empty array for empty streamingHistory", () => {
      const result = computeContentBlocksLegacy([]);
      expect(result).toBe(EMPTY_BLOCKS);
    });

    it("should process text blocks", () => {
      const history: StreamingBlock[] = [{ type: "text", content: "Hello world" }];

      const result = computeContentBlocksLegacy(history);

      expect(result).toHaveLength(1);
      expect(result[0]).toEqual({ type: "text", content: "Hello world" });
    });

    it("should process tool blocks", () => {
      const history: StreamingBlock[] = [
        {
          type: "tool",
          toolCall: {
            id: "tool-1",
            name: "read_file",
            args: { path: "/test.txt" },
            status: "completed",
            startedAt: "2024-01-01T00:00:00Z",
          },
        },
      ];

      const result = computeContentBlocksLegacy(history);

      expect(result).toHaveLength(1);
      expect(result[0]).toMatchObject({
        type: "tool",
        toolCall: expect.objectContaining({ id: "tool-1", name: "read_file" }),
      });
    });

    it("should group consecutive tool blocks", () => {
      const history: StreamingBlock[] = [
        {
          type: "tool",
          toolCall: {
            id: "tool-1",
            name: "read_file",
            args: { path: "/a.txt" },
            status: "completed",
            startedAt: "2024-01-01T00:00:00Z",
          },
        },
        {
          type: "tool",
          toolCall: {
            id: "tool-2",
            name: "read_file",
            args: { path: "/b.txt" },
            status: "completed",
            startedAt: "2024-01-01T00:00:00Z",
          },
        },
      ];

      const result = computeContentBlocksLegacy(history);

      expect(result).toHaveLength(1);
      expect(result[0]).toMatchObject({
        type: "tool_group",
        tools: expect.arrayContaining([
          expect.objectContaining({ id: "tool-1" }),
          expect.objectContaining({ id: "tool-2" }),
        ]),
      });
    });

    it("should process udiff_result blocks", () => {
      const history: StreamingBlock[] = [
        { type: "udiff_result", response: "diff output", durationMs: 100 },
      ];

      const result = computeContentBlocksLegacy(history);

      expect(result).toHaveLength(1);
      expect(result[0]).toEqual({
        type: "udiff_result",
        response: "diff output",
        durationMs: 100,
      });
    });

    it("should process system_hooks blocks", () => {
      const history: StreamingBlock[] = [{ type: "system_hooks", hooks: ["hook1", "hook2"] }];

      const result = computeContentBlocksLegacy(history);

      expect(result).toHaveLength(1);
      expect(result[0]).toEqual({
        type: "system_hooks",
        hooks: ["hook1", "hook2"],
      });
    });

    it("should process thinking blocks", () => {
      const history: StreamingBlock[] = [{ type: "thinking", content: "reasoning content" }];

      const result = computeContentBlocksLegacy(history);

      expect(result).toHaveLength(1);
      expect(result[0]).toEqual({
        type: "thinking",
        content: "reasoning content",
      });
    });

    it("should process interleaved text and tools", () => {
      const history: StreamingBlock[] = [
        { type: "text", content: "Let me read the file" },
        {
          type: "tool",
          toolCall: {
            id: "tool-1",
            name: "read_file",
            args: { path: "/test.txt" },
            status: "completed",
            startedAt: "2024-01-01T00:00:00Z",
          },
        },
        { type: "text", content: "The file contains:" },
      ];

      const result = computeContentBlocksLegacy(history);

      expect(result).toHaveLength(3);
      expect(result[0]).toEqual({ type: "text", content: "Let me read the file" });
      expect(result[1]).toMatchObject({
        type: "tool",
        toolCall: expect.objectContaining({ id: "tool-1" }),
      });
      expect(result[2]).toEqual({ type: "text", content: "The file contains:" });
    });
  });

  describe("stable reference optimization", () => {
    it("should return same empty array reference for empty inputs", () => {
      const result1 = computeContentBlocksLegacy(undefined);
      const result2 = computeContentBlocksLegacy([]);
      const result3 = computeContentBlocksLegacy(undefined, []);

      // All should be the exact same reference (not just equal)
      expect(result1).toBe(EMPTY_BLOCKS);
      expect(result2).toBe(EMPTY_BLOCKS);
      expect(result3).toBe(EMPTY_BLOCKS);
    });
  });

  describe("filter elimination", () => {
    it("should pass through all tool types since filter was a no-op", () => {
      // The original filter was:
      // .filter((block) => { if (block.type !== "tool") return true; return true; })
      // This always returns true, so we can eliminate it entirely

      const history: StreamingBlock[] = [
        { type: "text", content: "text" },
        {
          type: "tool",
          toolCall: {
            id: "tool-1",
            name: "any_tool",
            args: {},
            status: "completed",
            startedAt: "2024-01-01T00:00:00Z",
          },
        },
        { type: "thinking", content: "thinking" },
      ];

      const result = computeContentBlocksLegacy(history);

      // All blocks should pass through
      expect(result).toHaveLength(3);
    });
  });

  describe("consolidated hasSystemHooksInBlocks check", () => {
    it("should detect system_hooks in blocks", () => {
      const history: StreamingBlock[] = [
        { type: "text", content: "Some text" },
        { type: "system_hooks", hooks: ["hook1"] },
        { type: "text", content: "More text" },
      ];

      const { hasSystemHooksInBlocks } = computeContentBlocks(history);

      expect(hasSystemHooksInBlocks).toBe(true);
    });

    it("should return false when no system_hooks in blocks", () => {
      const history: StreamingBlock[] = [
        { type: "text", content: "Some text" },
        {
          type: "tool",
          toolCall: {
            id: "tool-1",
            name: "read_file",
            args: { path: "/test.txt" },
            status: "completed",
            startedAt: "2024-01-01T00:00:00Z",
          },
        },
      ];

      const { hasSystemHooksInBlocks } = computeContentBlocks(history);

      expect(hasSystemHooksInBlocks).toBe(false);
    });

    it("should return false for empty history", () => {
      const { hasSystemHooksInBlocks } = computeContentBlocks([]);

      expect(hasSystemHooksInBlocks).toBe(false);
    });

    it("should return false for undefined history", () => {
      const { hasSystemHooksInBlocks } = computeContentBlocks(undefined);

      expect(hasSystemHooksInBlocks).toBe(false);
    });
  });
});
