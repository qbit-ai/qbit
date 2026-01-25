import { describe, expect, it } from "vitest";
import type { UnifiedBlock } from "@/store";
import { estimateBlockHeight } from "./blockHeightEstimation";

// Helper to create a valid command block
function createCommandBlock(id: string, output: string): UnifiedBlock {
  return {
    id,
    type: "command",
    timestamp: new Date().toISOString(),
    data: {
      id,
      sessionId: "test-session",
      command: "test-command",
      output,
      exitCode: 0,
      startTime: new Date().toISOString(),
      durationMs: 100,
      workingDirectory: "/home/user",
      isCollapsed: false,
    },
  };
}

// Helper to create a valid agent message block
function createAgentMessageBlock(
  id: string,
  content: string,
  options: {
    toolCalls?: Array<{
      id: string;
      name: string;
      args: Record<string, unknown>;
      status: "completed" | "error";
    }>;
    thinkingContent?: string;
    streamingHistory?: Array<
      | { type: "text"; content: string }
      | {
          type: "tool";
          toolCall: {
            id: string;
            name: string;
            args: Record<string, unknown>;
            status: "completed";
          };
        }
    >;
  } = {}
): UnifiedBlock {
  return {
    id,
    type: "agent_message",
    timestamp: new Date().toISOString(),
    data: {
      id,
      sessionId: "test-session",
      role: "assistant",
      content,
      timestamp: new Date().toISOString(),
      toolCalls: options.toolCalls,
      thinkingContent: options.thinkingContent,
      streamingHistory: options.streamingHistory,
    },
  };
}

describe("estimateBlockHeight", () => {
  describe("command blocks", () => {
    it("returns base height for command without output", () => {
      const block = createCommandBlock("cmd-1", "");
      const height = estimateBlockHeight(block);
      expect(height).toBe(52); // Base height for command
    });

    it("increases height based on output length", () => {
      const shortOutput = "file1.txt\nfile2.txt";
      const longOutput = "x".repeat(500); // ~6 lines

      const shortBlock = createCommandBlock("cmd-1", shortOutput);
      const longBlock = createCommandBlock("cmd-2", longOutput);

      const shortHeight = estimateBlockHeight(shortBlock);
      const longHeight = estimateBlockHeight(longBlock);

      expect(longHeight).toBeGreaterThan(shortHeight);
    });

    it("caps height at maximum", () => {
      const hugeOutput = "x".repeat(10000);
      const block = createCommandBlock("cmd-1", hugeOutput);

      const height = estimateBlockHeight(block);
      expect(height).toBeLessThanOrEqual(500); // Max height for command
    });
  });

  describe("agent_message blocks", () => {
    it("returns base height for minimal message", () => {
      const block = createAgentMessageBlock("msg-1", "", { streamingHistory: [] });
      const height = estimateBlockHeight(block);
      expect(height).toBe(80); // Base height for agent_message
    });

    it("increases height based on content length", () => {
      const shortContent = "Hello!";
      const longContent = "x".repeat(1000);

      const shortBlock = createAgentMessageBlock("msg-1", shortContent, { streamingHistory: [] });
      const longBlock = createAgentMessageBlock("msg-2", longContent, { streamingHistory: [] });

      const shortHeight = estimateBlockHeight(shortBlock);
      const longHeight = estimateBlockHeight(longBlock);

      expect(longHeight).toBeGreaterThan(shortHeight);
    });

    it("accounts for tool calls", () => {
      const withoutTools = createAgentMessageBlock("msg-1", "Hello", { streamingHistory: [] });
      const withTools = createAgentMessageBlock("msg-2", "Hello", {
        streamingHistory: [],
        toolCalls: [
          { id: "tc-1", name: "read_file", args: {}, status: "completed" },
          { id: "tc-2", name: "write_file", args: {}, status: "completed" },
        ],
      });

      const heightWithout = estimateBlockHeight(withoutTools);
      const heightWith = estimateBlockHeight(withTools);

      expect(heightWith).toBeGreaterThan(heightWithout);
      // Each tool adds ~44px
      expect(heightWith - heightWithout).toBeGreaterThanOrEqual(80);
    });

    it("accounts for thinking content", () => {
      const withoutThinking = createAgentMessageBlock("msg-1", "Hello", { streamingHistory: [] });
      const withThinking = createAgentMessageBlock("msg-2", "Hello", {
        streamingHistory: [],
        thinkingContent: "Thinking about this... More thoughts...",
      });

      const heightWithout = estimateBlockHeight(withoutThinking);
      const heightWith = estimateBlockHeight(withThinking);

      expect(heightWith).toBeGreaterThan(heightWithout);
    });

    it("accounts for streaming history", () => {
      const withoutHistory = createAgentMessageBlock("msg-1", "", { streamingHistory: [] });
      const withHistory = createAgentMessageBlock("msg-2", "", {
        streamingHistory: [
          { type: "text", content: "Some text content here" },
          { type: "tool", toolCall: { id: "tc-1", name: "read", args: {}, status: "completed" } },
          { type: "text", content: "More text after the tool" },
        ],
      });

      const heightWithout = estimateBlockHeight(withoutHistory);
      const heightWith = estimateBlockHeight(withHistory);

      expect(heightWith).toBeGreaterThan(heightWithout);
    });

    it("caps height at maximum", () => {
      const hugeContent = "x".repeat(50000);
      const block = createAgentMessageBlock("msg-1", hugeContent, { streamingHistory: [] });

      const height = estimateBlockHeight(block);
      expect(height).toBeLessThanOrEqual(800); // Max height for agent_message
    });
  });

  describe("system_hook blocks", () => {
    it("returns base height for single hook", () => {
      const block: UnifiedBlock = {
        id: "hook-1",
        type: "system_hook",
        timestamp: new Date().toISOString(),
        data: {
          hooks: ["Hook content here"],
        },
      };

      const height = estimateBlockHeight(block);
      expect(height).toBeGreaterThanOrEqual(44); // Base height
    });

    it("increases height for multiple hooks", () => {
      const singleHook: UnifiedBlock = {
        id: "hook-1",
        type: "system_hook",
        timestamp: new Date().toISOString(),
        data: {
          hooks: ["Hook 1"],
        },
      };

      const multipleHooks: UnifiedBlock = {
        id: "hook-2",
        type: "system_hook",
        timestamp: new Date().toISOString(),
        data: {
          hooks: ["Hook 1", "Hook 2", "Hook 3", "Hook 4"],
        },
      };

      const singleHeight = estimateBlockHeight(singleHook);
      const multipleHeight = estimateBlockHeight(multipleHooks);

      expect(multipleHeight).toBeGreaterThan(singleHeight);
    });

    it("caps height at maximum", () => {
      const block: UnifiedBlock = {
        id: "hook-1",
        type: "system_hook",
        timestamp: new Date().toISOString(),
        data: {
          hooks: Array(50).fill("Hook content"),
        },
      };

      const height = estimateBlockHeight(block);
      expect(height).toBeLessThanOrEqual(200); // Max height for system_hook
    });
  });

  describe("unknown block types", () => {
    it("returns fallback height for unknown types", () => {
      // Use type assertion to test fallback behavior with unknown type
      const block = {
        id: "unknown-1",
        type: "unknown_type",
        timestamp: new Date().toISOString(),
        data: { hooks: [] },
      } as unknown as UnifiedBlock;

      const height = estimateBlockHeight(block);
      expect(height).toBe(100); // Fallback height
    });
  });
});
