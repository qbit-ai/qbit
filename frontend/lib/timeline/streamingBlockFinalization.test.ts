import { describe, expect, it } from "vitest";
import type { FinalizedStreamingBlock, StreamingBlock } from "@/store";
import { finalizeStreamingBlocks } from "./streamingBlockFinalization";

describe("finalizeStreamingBlocks", () => {
  describe("empty input", () => {
    it("should return empty array for empty input", () => {
      const result = finalizeStreamingBlocks([]);
      expect(result).toEqual([]);
    });
  });

  describe("text blocks", () => {
    it("should convert text block to finalized text block", () => {
      const blocks: StreamingBlock[] = [{ type: "text", content: "Hello world" }];

      const result = finalizeStreamingBlocks(blocks);

      expect(result).toEqual([{ type: "text", content: "Hello world" }]);
    });

    it("should preserve multiple text blocks in order", () => {
      const blocks: StreamingBlock[] = [
        { type: "text", content: "First" },
        { type: "text", content: "Second" },
        { type: "text", content: "Third" },
      ];

      const result = finalizeStreamingBlocks(blocks);

      expect(result).toHaveLength(3);
      expect(result[0]).toEqual({ type: "text", content: "First" });
      expect(result[1]).toEqual({ type: "text", content: "Second" });
      expect(result[2]).toEqual({ type: "text", content: "Third" });
    });
  });

  describe("udiff_result blocks", () => {
    it("should convert udiff_result block to finalized udiff_result block", () => {
      const blocks: StreamingBlock[] = [
        { type: "udiff_result", response: "diff output here", durationMs: 1500 },
      ];

      const result = finalizeStreamingBlocks(blocks);

      expect(result).toEqual([
        { type: "udiff_result", response: "diff output here", durationMs: 1500 },
      ]);
    });
  });

  describe("tool blocks", () => {
    it("should convert completed tool block to finalized tool block", () => {
      const blocks: StreamingBlock[] = [
        {
          type: "tool",
          toolCall: {
            id: "tool-1",
            name: "read_file",
            args: { path: "/test.txt" },
            status: "completed",
            result: "file contents",
            startedAt: "2024-01-01T00:00:00Z",
            completedAt: "2024-01-01T00:00:01Z",
          },
        },
      ];

      const result = finalizeStreamingBlocks(blocks);

      expect(result).toHaveLength(1);
      expect(result[0]).toEqual({
        type: "tool",
        toolCall: {
          id: "tool-1",
          name: "read_file",
          args: { path: "/test.txt" },
          status: "completed",
          result: "file contents",
          executedByAgent: undefined,
        },
      });
    });

    it("should convert error tool block to finalized tool block with error status", () => {
      const blocks: StreamingBlock[] = [
        {
          type: "tool",
          toolCall: {
            id: "tool-1",
            name: "read_file",
            args: { path: "/nonexistent.txt" },
            status: "error",
            result: "File not found",
            startedAt: "2024-01-01T00:00:00Z",
          },
        },
      ];

      const result = finalizeStreamingBlocks(blocks);

      expect(result).toHaveLength(1);
      expect(result[0]).toEqual({
        type: "tool",
        toolCall: {
          id: "tool-1",
          name: "read_file",
          args: { path: "/nonexistent.txt" },
          status: "error",
          result: "File not found",
          executedByAgent: undefined,
        },
      });
    });

    it("should convert running tool block to completed status (finalization happens at end)", () => {
      const blocks: StreamingBlock[] = [
        {
          type: "tool",
          toolCall: {
            id: "tool-1",
            name: "read_file",
            args: { path: "/test.txt" },
            status: "running",
            startedAt: "2024-01-01T00:00:00Z",
          },
        },
      ];

      const result = finalizeStreamingBlocks(blocks);

      expect(result).toHaveLength(1);
      // Running tools are finalized as "completed" since the turn is ending
      expect((result[0] as { type: "tool"; toolCall: { status: string } }).toolCall.status).toBe(
        "completed"
      );
    });

    it("should preserve executedByAgent flag", () => {
      const blocks: StreamingBlock[] = [
        {
          type: "tool",
          toolCall: {
            id: "tool-1",
            name: "read_file",
            args: {},
            status: "completed",
            executedByAgent: true,
            startedAt: "2024-01-01T00:00:00Z",
          },
        },
      ];

      const result = finalizeStreamingBlocks(blocks);

      expect(result).toHaveLength(1);
      expect(
        (result[0] as { type: "tool"; toolCall: { executedByAgent?: boolean } }).toolCall
          .executedByAgent
      ).toBe(true);
    });
  });

  describe("mixed blocks", () => {
    it("should preserve interleaved order of text and tool blocks", () => {
      const blocks: StreamingBlock[] = [
        { type: "text", content: "I will read the file" },
        {
          type: "tool",
          toolCall: {
            id: "tool-1",
            name: "read_file",
            args: { path: "/test.txt" },
            status: "completed",
            result: "contents",
            startedAt: "2024-01-01T00:00:00Z",
          },
        },
        { type: "text", content: "The file contains: contents" },
      ];

      const result = finalizeStreamingBlocks(blocks);

      expect(result).toHaveLength(3);
      expect(result[0].type).toBe("text");
      expect(result[1].type).toBe("tool");
      expect(result[2].type).toBe("text");
    });

    it("should handle complex interleaving with all block types", () => {
      const blocks: StreamingBlock[] = [
        { type: "text", content: "Starting" },
        {
          type: "tool",
          toolCall: {
            id: "tool-1",
            name: "read_file",
            args: {},
            status: "completed",
            startedAt: "2024-01-01T00:00:00Z",
          },
        },
        { type: "udiff_result", response: "diff", durationMs: 100 },
        { type: "text", content: "Done" },
      ];

      const result = finalizeStreamingBlocks(blocks);

      expect(result).toHaveLength(4);
      expect(result.map((b) => b.type)).toEqual(["text", "tool", "udiff_result", "text"]);
    });
  });

  describe("extract tool calls helper", () => {
    it("should extract only tool calls from finalized blocks", () => {
      const blocks: StreamingBlock[] = [
        { type: "text", content: "text" },
        {
          type: "tool",
          toolCall: {
            id: "tool-1",
            name: "read_file",
            args: {},
            status: "completed",
            startedAt: "2024-01-01T00:00:00Z",
          },
        },
        { type: "udiff_result", response: "diff", durationMs: 100 },
        {
          type: "tool",
          toolCall: {
            id: "tool-2",
            name: "write_file",
            args: {},
            status: "completed",
            startedAt: "2024-01-01T00:00:00Z",
          },
        },
      ];

      const finalized = finalizeStreamingBlocks(blocks);
      const toolCalls = finalized
        .filter((b): b is FinalizedStreamingBlock & { type: "tool" } => b.type === "tool")
        .map((b) => b.toolCall);

      expect(toolCalls).toHaveLength(2);
      expect(toolCalls[0].id).toBe("tool-1");
      expect(toolCalls[1].id).toBe("tool-2");
    });
  });
});
