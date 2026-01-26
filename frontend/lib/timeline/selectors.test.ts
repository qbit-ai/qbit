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
});
