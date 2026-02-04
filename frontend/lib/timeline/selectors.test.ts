import { describe, expect, it } from "vitest";
import type { AgentMessage, CommandBlock, UnifiedBlock } from "@/store";
import {
  createMemoizedAgentMessagesSelector,
  createMemoizedCommandBlocksSelector,
  memoizedSelectAgentMessages,
  memoizedSelectCommandBlocks,
  selectAgentMessagesFromTimeline,
  selectCommandBlocksFromTimeline,
} from "./selectors";

describe("Timeline Selectors", () => {
  // Helper to create a command block
  const createCommandBlock = (id: string, command: string): CommandBlock => ({
    id,
    sessionId: "session-1",
    command,
    output: `output of ${command}`,
    exitCode: 0,
    startTime: new Date().toISOString(),
    durationMs: 100,
    workingDirectory: "/home/user",
    isCollapsed: false,
  });

  // Helper to create an agent message
  const createAgentMessage = (
    id: string,
    role: "user" | "assistant" | "system",
    content: string
  ): AgentMessage => ({
    id,
    sessionId: "session-1",
    role,
    content,
    timestamp: new Date().toISOString(),
  });

  // Helper to wrap in UnifiedBlock
  const wrapCommand = (block: CommandBlock): UnifiedBlock => ({
    id: block.id,
    type: "command",
    timestamp: block.startTime,
    data: block,
  });

  const wrapMessage = (message: AgentMessage): UnifiedBlock => ({
    id: message.id,
    type: "agent_message",
    timestamp: message.timestamp,
    data: message,
  });

  const createSystemHookBlock = (id: string, hooks: string[]): UnifiedBlock => ({
    id,
    type: "system_hook",
    timestamp: new Date().toISOString(),
    data: { hooks },
  });

  describe("selectCommandBlocksFromTimeline", () => {
    it("should return empty array for undefined timeline", () => {
      expect(selectCommandBlocksFromTimeline(undefined)).toEqual([]);
    });

    it("should return empty array for empty timeline", () => {
      expect(selectCommandBlocksFromTimeline([])).toEqual([]);
    });

    it("should extract command blocks from timeline", () => {
      const cmd1 = createCommandBlock("cmd-1", "ls");
      const cmd2 = createCommandBlock("cmd-2", "pwd");
      const timeline: UnifiedBlock[] = [wrapCommand(cmd1), wrapCommand(cmd2)];

      const result = selectCommandBlocksFromTimeline(timeline);

      expect(result).toHaveLength(2);
      expect(result[0].command).toBe("ls");
      expect(result[1].command).toBe("pwd");
    });

    it("should filter out non-command blocks", () => {
      const cmd = createCommandBlock("cmd-1", "ls");
      const msg = createAgentMessage("msg-1", "user", "Hello");
      const hook = createSystemHookBlock("hook-1", ["some hook"]);

      const timeline: UnifiedBlock[] = [wrapCommand(cmd), wrapMessage(msg), hook];

      const result = selectCommandBlocksFromTimeline(timeline);

      expect(result).toHaveLength(1);
      expect(result[0].command).toBe("ls");
    });

    it("should preserve order of command blocks", () => {
      const cmd1 = createCommandBlock("cmd-1", "first");
      const cmd2 = createCommandBlock("cmd-2", "second");
      const cmd3 = createCommandBlock("cmd-3", "third");
      const msg = createAgentMessage("msg-1", "user", "interleaved");

      const timeline: UnifiedBlock[] = [
        wrapCommand(cmd1),
        wrapMessage(msg),
        wrapCommand(cmd2),
        wrapCommand(cmd3),
      ];

      const result = selectCommandBlocksFromTimeline(timeline);

      expect(result).toHaveLength(3);
      expect(result[0].command).toBe("first");
      expect(result[1].command).toBe("second");
      expect(result[2].command).toBe("third");
    });

    it("should preserve all command block properties", () => {
      const cmd: CommandBlock = {
        id: "cmd-1",
        sessionId: "session-1",
        command: "echo hello",
        output: "hello\n",
        exitCode: 42,
        startTime: "2024-01-01T10:00:00Z",
        durationMs: 150,
        workingDirectory: "/custom/path",
        isCollapsed: true,
      };

      const timeline: UnifiedBlock[] = [wrapCommand(cmd)];
      const result = selectCommandBlocksFromTimeline(timeline);

      expect(result[0]).toEqual(cmd);
    });
  });

  describe("selectAgentMessagesFromTimeline", () => {
    it("should return empty array for undefined timeline", () => {
      expect(selectAgentMessagesFromTimeline(undefined)).toEqual([]);
    });

    it("should return empty array for empty timeline", () => {
      expect(selectAgentMessagesFromTimeline([])).toEqual([]);
    });

    it("should extract agent messages from timeline", () => {
      const msg1 = createAgentMessage("msg-1", "user", "Hello");
      const msg2 = createAgentMessage("msg-2", "assistant", "Hi there!");
      const timeline: UnifiedBlock[] = [wrapMessage(msg1), wrapMessage(msg2)];

      const result = selectAgentMessagesFromTimeline(timeline);

      expect(result).toHaveLength(2);
      expect(result[0].content).toBe("Hello");
      expect(result[1].content).toBe("Hi there!");
    });

    it("should filter out non-message blocks", () => {
      const cmd = createCommandBlock("cmd-1", "ls");
      const msg = createAgentMessage("msg-1", "user", "Hello");
      const hook = createSystemHookBlock("hook-1", ["some hook"]);

      const timeline: UnifiedBlock[] = [wrapCommand(cmd), wrapMessage(msg), hook];

      const result = selectAgentMessagesFromTimeline(timeline);

      expect(result).toHaveLength(1);
      expect(result[0].content).toBe("Hello");
    });

    it("should preserve order of messages", () => {
      const msg1 = createAgentMessage("msg-1", "user", "first");
      const msg2 = createAgentMessage("msg-2", "assistant", "second");
      const msg3 = createAgentMessage("msg-3", "user", "third");
      const cmd = createCommandBlock("cmd-1", "interleaved");

      const timeline: UnifiedBlock[] = [
        wrapMessage(msg1),
        wrapCommand(cmd),
        wrapMessage(msg2),
        wrapMessage(msg3),
      ];

      const result = selectAgentMessagesFromTimeline(timeline);

      expect(result).toHaveLength(3);
      expect(result[0].content).toBe("first");
      expect(result[1].content).toBe("second");
      expect(result[2].content).toBe("third");
    });

    it("should preserve all message properties including tool calls", () => {
      const msg: AgentMessage = {
        id: "msg-1",
        sessionId: "session-1",
        role: "assistant",
        content: "Let me help",
        timestamp: "2024-01-01T10:00:00Z",
        isStreaming: false,
        toolCalls: [
          {
            id: "tool-1",
            name: "read_file",
            args: { path: "/test.txt" },
            status: "completed",
            result: "file contents",
          },
        ],
        streamingHistory: [
          { type: "text", content: "Let me help" },
          {
            type: "tool",
            toolCall: {
              id: "tool-1",
              name: "read_file",
              args: { path: "/test.txt" },
              status: "completed",
            },
          },
        ],
        inputTokens: 100,
        outputTokens: 50,
      };

      const timeline: UnifiedBlock[] = [wrapMessage(msg)];
      const result = selectAgentMessagesFromTimeline(timeline);

      expect(result[0]).toEqual(msg);
    });
  });

  describe("createMemoizedCommandBlocksSelector", () => {
    it("should return same reference for same timeline", () => {
      const selector = createMemoizedCommandBlocksSelector();
      const cmd = createCommandBlock("cmd-1", "ls");
      const timeline: UnifiedBlock[] = [wrapCommand(cmd)];

      const result1 = selector("session-1", timeline);
      const result2 = selector("session-1", timeline);

      expect(result1).toBe(result2); // Same reference
    });

    it("should return new reference when timeline changes", () => {
      const selector = createMemoizedCommandBlocksSelector();
      const cmd1 = createCommandBlock("cmd-1", "ls");
      const cmd2 = createCommandBlock("cmd-2", "pwd");

      const timeline1: UnifiedBlock[] = [wrapCommand(cmd1)];
      const timeline2: UnifiedBlock[] = [wrapCommand(cmd1), wrapCommand(cmd2)];

      const result1 = selector("session-1", timeline1);
      const result2 = selector("session-1", timeline2);

      expect(result1).not.toBe(result2); // Different references
      expect(result1).toHaveLength(1);
      expect(result2).toHaveLength(2);
    });

    it("should cache per session independently", () => {
      const selector = createMemoizedCommandBlocksSelector();
      const cmd1 = createCommandBlock("cmd-1", "ls");
      const cmd2 = createCommandBlock("cmd-2", "pwd");

      const timeline1: UnifiedBlock[] = [wrapCommand(cmd1)];
      const timeline2: UnifiedBlock[] = [wrapCommand(cmd2)];

      const result1a = selector("session-1", timeline1);
      const result2a = selector("session-2", timeline2);

      // Access again
      const result1b = selector("session-1", timeline1);
      const result2b = selector("session-2", timeline2);

      // Same references for same sessions
      expect(result1a).toBe(result1b);
      expect(result2a).toBe(result2b);
    });

    it("should handle undefined timeline", () => {
      const selector = createMemoizedCommandBlocksSelector();

      const result1 = selector("session-1", undefined);
      const result2 = selector("session-1", undefined);

      expect(result1).toEqual([]);
      expect(result1).toBe(result2); // Same reference for cached undefined
    });
  });

  describe("createMemoizedAgentMessagesSelector", () => {
    it("should return same reference for same timeline", () => {
      const selector = createMemoizedAgentMessagesSelector();
      const msg = createAgentMessage("msg-1", "user", "Hello");
      const timeline: UnifiedBlock[] = [wrapMessage(msg)];

      const result1 = selector("session-1", timeline);
      const result2 = selector("session-1", timeline);

      expect(result1).toBe(result2); // Same reference
    });

    it("should return new reference when timeline changes", () => {
      const selector = createMemoizedAgentMessagesSelector();
      const msg1 = createAgentMessage("msg-1", "user", "Hello");
      const msg2 = createAgentMessage("msg-2", "assistant", "Hi");

      const timeline1: UnifiedBlock[] = [wrapMessage(msg1)];
      const timeline2: UnifiedBlock[] = [wrapMessage(msg1), wrapMessage(msg2)];

      const result1 = selector("session-1", timeline1);
      const result2 = selector("session-1", timeline2);

      expect(result1).not.toBe(result2); // Different references
      expect(result1).toHaveLength(1);
      expect(result2).toHaveLength(2);
    });
  });

  describe("singleton memoized selectors", () => {
    it("memoizedSelectCommandBlocks should work correctly", () => {
      const cmd = createCommandBlock("singleton-cmd-1", "singleton ls");
      const timeline: UnifiedBlock[] = [wrapCommand(cmd)];

      const result = memoizedSelectCommandBlocks("singleton-session", timeline);

      expect(result).toHaveLength(1);
      expect(result[0].command).toBe("singleton ls");
    });

    it("memoizedSelectAgentMessages should work correctly", () => {
      const msg = createAgentMessage("singleton-msg-1", "user", "Singleton hello");
      const timeline: UnifiedBlock[] = [wrapMessage(msg)];

      const result = memoizedSelectAgentMessages("singleton-session", timeline);

      expect(result).toHaveLength(1);
      expect(result[0].content).toBe("Singleton hello");
    });
  });

  describe("Shallow Array Comparison (Reference Stability)", () => {
    /**
     * These tests verify that the memoized selectors return stable references
     * when the extracted items haven't changed, even if the timeline array
     * itself is a new reference.
     *
     * This is important for preventing unnecessary re-renders in React components
     * that depend on these derived arrays.
     */

    describe("Command blocks shallow comparison", () => {
      it("should return same reference when command blocks haven't changed (same items)", () => {
        const selector = createMemoizedCommandBlocksSelector();
        const cmd1 = createCommandBlock("cmd-1", "ls");
        const cmd2 = createCommandBlock("cmd-2", "pwd");

        // Same command blocks in both timelines
        const timeline1: UnifiedBlock[] = [wrapCommand(cmd1), wrapCommand(cmd2)];
        const timeline2: UnifiedBlock[] = [wrapCommand(cmd1), wrapCommand(cmd2)];

        // Different timeline references
        expect(timeline1).not.toBe(timeline2);

        const result1 = selector("session-shallow-1", timeline1);
        const result2 = selector("session-shallow-1", timeline2);

        // Should return the same reference because the extracted command blocks
        // are the same (shallow equality on the data references)
        expect(result1).toBe(result2);
      });

      it("should return new reference when command block data changes", () => {
        const selector = createMemoizedCommandBlocksSelector();
        const cmd1 = createCommandBlock("cmd-1", "ls");
        const cmd1Modified = { ...cmd1, output: "new output" };

        const timeline1: UnifiedBlock[] = [wrapCommand(cmd1)];
        const timeline2: UnifiedBlock[] = [wrapCommand(cmd1Modified)];

        const result1 = selector("session-shallow-2", timeline1);
        const result2 = selector("session-shallow-2", timeline2);

        // Should return different references because command block data changed
        expect(result1).not.toBe(result2);
      });

      it("should return new reference when command block count changes", () => {
        const selector = createMemoizedCommandBlocksSelector();
        const cmd1 = createCommandBlock("cmd-1", "ls");
        const cmd2 = createCommandBlock("cmd-2", "pwd");

        const timeline1: UnifiedBlock[] = [wrapCommand(cmd1)];
        const timeline2: UnifiedBlock[] = [wrapCommand(cmd1), wrapCommand(cmd2)];

        const result1 = selector("session-shallow-3", timeline1);
        const result2 = selector("session-shallow-3", timeline2);

        // Should return different references because count changed
        expect(result1).not.toBe(result2);
        expect(result1).toHaveLength(1);
        expect(result2).toHaveLength(2);
      });

      it("should return same reference when only non-command blocks change", () => {
        const selector = createMemoizedCommandBlocksSelector();
        const cmd = createCommandBlock("cmd-1", "ls");
        const msg1 = createAgentMessage("msg-1", "user", "Hello");
        const msg2 = createAgentMessage("msg-2", "user", "World");

        // Same command, different messages
        const timeline1: UnifiedBlock[] = [wrapCommand(cmd), wrapMessage(msg1)];
        const timeline2: UnifiedBlock[] = [wrapCommand(cmd), wrapMessage(msg2)];

        const result1 = selector("session-shallow-4", timeline1);
        const result2 = selector("session-shallow-4", timeline2);

        // Should return same reference because command blocks haven't changed
        expect(result1).toBe(result2);
        expect(result1).toHaveLength(1);
      });
    });

    describe("Agent messages shallow comparison", () => {
      it("should return same reference when agent messages haven't changed (same items)", () => {
        const selector = createMemoizedAgentMessagesSelector();
        const msg1 = createAgentMessage("msg-1", "user", "Hello");
        const msg2 = createAgentMessage("msg-2", "assistant", "Hi");

        // Same messages in both timelines
        const timeline1: UnifiedBlock[] = [wrapMessage(msg1), wrapMessage(msg2)];
        const timeline2: UnifiedBlock[] = [wrapMessage(msg1), wrapMessage(msg2)];

        // Different timeline references
        expect(timeline1).not.toBe(timeline2);

        const result1 = selector("session-shallow-5", timeline1);
        const result2 = selector("session-shallow-5", timeline2);

        // Should return the same reference because the extracted messages
        // are the same (shallow equality on the data references)
        expect(result1).toBe(result2);
      });

      it("should return new reference when message data changes", () => {
        const selector = createMemoizedAgentMessagesSelector();
        const msg1 = createAgentMessage("msg-1", "user", "Hello");
        const msg1Modified = { ...msg1, content: "Hello World" };

        const timeline1: UnifiedBlock[] = [wrapMessage(msg1)];
        const timeline2: UnifiedBlock[] = [wrapMessage(msg1Modified)];

        const result1 = selector("session-shallow-6", timeline1);
        const result2 = selector("session-shallow-6", timeline2);

        // Should return different references because message data changed
        expect(result1).not.toBe(result2);
      });

      it("should return new reference when message count changes", () => {
        const selector = createMemoizedAgentMessagesSelector();
        const msg1 = createAgentMessage("msg-1", "user", "Hello");
        const msg2 = createAgentMessage("msg-2", "assistant", "Hi");

        const timeline1: UnifiedBlock[] = [wrapMessage(msg1)];
        const timeline2: UnifiedBlock[] = [wrapMessage(msg1), wrapMessage(msg2)];

        const result1 = selector("session-shallow-7", timeline1);
        const result2 = selector("session-shallow-7", timeline2);

        // Should return different references because count changed
        expect(result1).not.toBe(result2);
        expect(result1).toHaveLength(1);
        expect(result2).toHaveLength(2);
      });

      it("should return same reference when only non-message blocks change", () => {
        const selector = createMemoizedAgentMessagesSelector();
        const msg = createAgentMessage("msg-1", "user", "Hello");
        const cmd1 = createCommandBlock("cmd-1", "ls");
        const cmd2 = createCommandBlock("cmd-2", "pwd");

        // Same message, different commands
        const timeline1: UnifiedBlock[] = [wrapMessage(msg), wrapCommand(cmd1)];
        const timeline2: UnifiedBlock[] = [wrapMessage(msg), wrapCommand(cmd2)];

        const result1 = selector("session-shallow-8", timeline1);
        const result2 = selector("session-shallow-8", timeline2);

        // Should return same reference because messages haven't changed
        expect(result1).toBe(result2);
        expect(result1).toHaveLength(1);
      });
    });

    describe("Empty array stability", () => {
      it("should return same empty array reference for command blocks", () => {
        const selector = createMemoizedCommandBlocksSelector();
        const msg = createAgentMessage("msg-1", "user", "Hello");

        // Timeline with no commands
        const timeline1: UnifiedBlock[] = [wrapMessage(msg)];
        const timeline2: UnifiedBlock[] = [wrapMessage(msg)];

        const result1 = selector("session-empty-cmd", timeline1);
        const result2 = selector("session-empty-cmd", timeline2);

        expect(result1).toHaveLength(0);
        expect(result1).toBe(result2);
      });

      it("should return same empty array reference for agent messages", () => {
        const selector = createMemoizedAgentMessagesSelector();
        const cmd = createCommandBlock("cmd-1", "ls");

        // Timeline with no messages
        const timeline1: UnifiedBlock[] = [wrapCommand(cmd)];
        const timeline2: UnifiedBlock[] = [wrapCommand(cmd)];

        const result1 = selector("session-empty-msg", timeline1);
        const result2 = selector("session-empty-msg", timeline2);

        expect(result1).toHaveLength(0);
        expect(result1).toBe(result2);
      });
    });
  });
});
