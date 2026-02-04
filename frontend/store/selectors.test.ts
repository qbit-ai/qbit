import { beforeEach, describe, expect, it } from "vitest";
import {
  selectAgentMessagesFromTimeline,
  selectCommandBlocksFromTimeline,
} from "@/lib/timeline/selectors";
import type { AgentMessage } from "./index";
import { useStore } from "./index";

/**
 * Tests for derived selectors that compute commandBlocks and agentMessages
 * from the unified timelines array.
 *
 * These tests establish the contract for the Single Source of Truth refactoring:
 * - timelines is the only source of truth
 * - commandBlocks and agentMessages are derived from timelines
 * - Derived values should match what was previously stored in duplicate arrays
 */
describe("Derived Timeline Selectors", () => {
  beforeEach(() => {
    // Reset store state before each test
    useStore.setState({
      sessions: {},
      activeSessionId: null,
      timelines: {},
      pendingCommand: {},
      agentStreaming: {},
      streamingBlocks: {},
      streamingTextOffset: {},
      agentInitialized: {},
      pendingToolApproval: {},
      processedToolRequests: {},
      activeToolCalls: {},
      thinkingContent: {},
      isThinkingExpanded: {},
      contextMetrics: {},
      compactionCount: {},
      isCompacting: {},
      isSessionDead: {},
      compactionError: {},
      gitStatus: {},
      gitStatusLoading: {},
      gitCommitMessage: {},
    });
  });

  // Helper to create a session
  const createSession = (sessionId: string) => {
    useStore.getState().addSession({
      id: sessionId,
      name: "Test Session",
      workingDirectory: "/home/user",
      createdAt: "2024-01-01T00:00:00Z",
      mode: "terminal",
    });
  };

  // Helper to create an agent message
  const createAgentMessage = (
    sessionId: string,
    role: "user" | "assistant" | "system",
    content: string
  ): AgentMessage => ({
    id: crypto.randomUUID(),
    sessionId,
    role,
    content,
    timestamp: new Date().toISOString(),
  });

  describe("selectCommandBlocks (derived from timelines)", () => {
    it("should return empty array for non-existent session", () => {
      const state = useStore.getState();
      // Use derived selector for a session that doesn't exist
      const blocks = selectCommandBlocksFromTimeline(state.timelines["non-existent"]);
      expect(blocks).toEqual([]);
    });

    it("should return empty array for session with no commands", () => {
      createSession("session-1");
      const state = useStore.getState();
      const blocks = selectCommandBlocksFromTimeline(state.timelines["session-1"]);
      expect(blocks).toEqual([]);
    });

    it("should extract command blocks from timeline", () => {
      createSession("session-1");
      const store = useStore.getState();

      // Add a command via handleCommandEnd (now writes only to timelines)
      store.handleCommandStart("session-1", "ls -la");
      store.appendOutput("session-1", "file1.txt\nfile2.txt\n");
      store.handleCommandEnd("session-1", 0);

      const state = useStore.getState();

      // Verify command is in timeline
      expect(state.timelines["session-1"]).toHaveLength(1);
      expect(state.timelines["session-1"][0].type).toBe("command");

      // Verify derived selector extracts it correctly
      const blocks = selectCommandBlocksFromTimeline(state.timelines["session-1"]);
      expect(blocks).toHaveLength(1);
      expect(blocks[0].command).toBe("ls -la");
    });

    it("should filter out non-command blocks when deriving", () => {
      createSession("session-1");
      const store = useStore.getState();

      // Add a command
      store.handleCommandStart("session-1", "ls");
      store.handleCommandEnd("session-1", 0);

      // Add an agent message (different block type)
      store.addAgentMessage("session-1", createAgentMessage("session-1", "user", "Hello"));

      // Add another command
      store.handleCommandStart("session-1", "pwd");
      store.handleCommandEnd("session-1", 0);

      const state = useStore.getState();

      // Timeline should have 3 blocks (2 commands + 1 agent message)
      expect(state.timelines["session-1"]).toHaveLength(3);

      // Derived selector should only return the 2 commands
      const blocks = selectCommandBlocksFromTimeline(state.timelines["session-1"]);
      expect(blocks).toHaveLength(2);
      expect(blocks[0].command).toBe("ls");
      expect(blocks[1].command).toBe("pwd");
    });

    it("should preserve command block data integrity", () => {
      createSession("session-1");
      const store = useStore.getState();

      store.handleCommandStart("session-1", "echo hello");
      store.appendOutput("session-1", "hello\n");
      store.handleCommandEnd("session-1", 42);

      const state = useStore.getState();
      const blocks = selectCommandBlocksFromTimeline(state.timelines["session-1"]);
      const block = blocks[0];

      expect(block.command).toBe("echo hello");
      expect(block.output).toBe("hello\n");
      expect(block.exitCode).toBe(42);
      expect(block.sessionId).toBe("session-1");
      expect(block.workingDirectory).toBe("/home/user");
      expect(block.isCollapsed).toBe(false);
      expect(block.startTime).toBeDefined();
      // durationMs can be 0 in fast test environments, just verify it's defined and non-negative
      expect(block.durationMs).toBeGreaterThanOrEqual(0);
    });

    it("should handle multiple sessions independently", () => {
      createSession("session-1");
      createSession("session-2");
      const store = useStore.getState();

      // Add commands to session-1
      store.handleCommandStart("session-1", "ls");
      store.handleCommandEnd("session-1", 0);

      // Add commands to session-2
      store.handleCommandStart("session-2", "pwd");
      store.handleCommandEnd("session-2", 0);
      store.handleCommandStart("session-2", "whoami");
      store.handleCommandEnd("session-2", 0);

      const state = useStore.getState();

      const blocks1 = selectCommandBlocksFromTimeline(state.timelines["session-1"]);
      const blocks2 = selectCommandBlocksFromTimeline(state.timelines["session-2"]);
      expect(blocks1).toHaveLength(1);
      expect(blocks2).toHaveLength(2);
    });
  });

  describe("selectAgentMessages (derived from timelines)", () => {
    it("should return empty array for non-existent session", () => {
      const state = useStore.getState();
      const messages = selectAgentMessagesFromTimeline(state.timelines["non-existent"]);
      expect(messages).toEqual([]);
    });

    it("should return empty array for session with no messages", () => {
      createSession("session-1");
      const state = useStore.getState();
      const messages = selectAgentMessagesFromTimeline(state.timelines["session-1"]);
      expect(messages).toEqual([]);
    });

    it("should extract agent messages from timeline", () => {
      createSession("session-1");
      const store = useStore.getState();

      const message = createAgentMessage("session-1", "user", "Hello, Claude!");
      store.addAgentMessage("session-1", message);

      const state = useStore.getState();

      // Verify message is in timeline
      expect(state.timelines["session-1"]).toHaveLength(1);
      expect(state.timelines["session-1"][0].type).toBe("agent_message");

      // Verify derived selector extracts it correctly
      const messages = selectAgentMessagesFromTimeline(state.timelines["session-1"]);
      expect(messages).toHaveLength(1);
      expect(messages[0].content).toBe("Hello, Claude!");
    });

    it("should filter out non-message blocks when deriving", () => {
      createSession("session-1");
      const store = useStore.getState();

      // Add a message
      store.addAgentMessage("session-1", createAgentMessage("session-1", "user", "First"));

      // Add a command (different block type)
      store.handleCommandStart("session-1", "ls");
      store.handleCommandEnd("session-1", 0);

      // Add another message
      store.addAgentMessage("session-1", createAgentMessage("session-1", "assistant", "Second"));

      const state = useStore.getState();

      // Timeline should have 3 blocks
      expect(state.timelines["session-1"]).toHaveLength(3);

      // Derived selector should only return the 2 messages
      const messages = selectAgentMessagesFromTimeline(state.timelines["session-1"]);
      expect(messages).toHaveLength(2);
      expect(messages[0].content).toBe("First");
      expect(messages[1].content).toBe("Second");
    });

    it("should preserve message data integrity including tool calls", () => {
      createSession("session-1");
      const store = useStore.getState();

      const message: AgentMessage = {
        id: "msg-with-tools",
        sessionId: "session-1",
        role: "assistant",
        content: "Let me read that file.",
        timestamp: "2024-01-01T10:00:00Z",
        toolCalls: [
          {
            id: "tool-1",
            name: "read_file",
            args: { path: "/test.txt" },
            status: "completed",
            result: "file contents",
          },
        ],
        inputTokens: 100,
        outputTokens: 50,
      };

      store.addAgentMessage("session-1", message);

      const state = useStore.getState();
      const messages = selectAgentMessagesFromTimeline(state.timelines["session-1"]);
      const restored = messages[0];

      expect(restored.id).toBe("msg-with-tools");
      expect(restored.role).toBe("assistant");
      expect(restored.content).toBe("Let me read that file.");
      expect(restored.toolCalls).toHaveLength(1);
      expect(restored.toolCalls?.[0].name).toBe("read_file");
      expect(restored.inputTokens).toBe(100);
      expect(restored.outputTokens).toBe(50);
    });

    it("should handle multiple sessions independently", () => {
      createSession("session-1");
      createSession("session-2");
      const store = useStore.getState();

      // Add messages to session-1
      store.addAgentMessage("session-1", createAgentMessage("session-1", "user", "Hi"));

      // Add messages to session-2
      store.addAgentMessage("session-2", createAgentMessage("session-2", "user", "Hello"));
      store.addAgentMessage("session-2", createAgentMessage("session-2", "assistant", "Hi there"));

      const state = useStore.getState();

      const messages1 = selectAgentMessagesFromTimeline(state.timelines["session-1"]);
      const messages2 = selectAgentMessagesFromTimeline(state.timelines["session-2"]);
      expect(messages1).toHaveLength(1);
      expect(messages2).toHaveLength(2);
    });

    it("should preserve messages with streamingHistory", () => {
      createSession("session-1");
      const store = useStore.getState();

      const message: AgentMessage = {
        id: "msg-with-history",
        sessionId: "session-1",
        role: "assistant",
        content: "Full response",
        timestamp: "2024-01-01T10:00:00Z",
        streamingHistory: [
          { type: "text", content: "First part" },
          {
            type: "tool",
            toolCall: {
              id: "tool-1",
              name: "bash",
              args: { command: "ls" },
              status: "completed",
            },
          },
          { type: "text", content: "Second part" },
        ],
      };

      store.addAgentMessage("session-1", message);

      const state = useStore.getState();
      const messages = selectAgentMessagesFromTimeline(state.timelines["session-1"]);
      const restored = messages[0];

      expect(restored.streamingHistory).toHaveLength(3);
      expect(restored.streamingHistory?.[0].type).toBe("text");
      expect(restored.streamingHistory?.[1].type).toBe("tool");
      expect(restored.streamingHistory?.[2].type).toBe("text");
    });
  });

  describe("Consistency between timelines and derived selectors", () => {
    it("should maintain consistency after adding commands", () => {
      createSession("session-1");
      const store = useStore.getState();

      store.handleCommandStart("session-1", "cmd1");
      store.handleCommandEnd("session-1", 0);
      store.handleCommandStart("session-1", "cmd2");
      store.handleCommandEnd("session-1", 0);

      const state = useStore.getState();

      // Extract commands from timeline using derived selector
      const derivedCommands = selectCommandBlocksFromTimeline(state.timelines["session-1"]);

      // Verify derived values are correct
      expect(derivedCommands).toHaveLength(2);
      expect(derivedCommands[0].command).toBe("cmd1");
      expect(derivedCommands[1].command).toBe("cmd2");
    });

    it("should maintain consistency after adding messages", () => {
      createSession("session-1");
      const store = useStore.getState();

      store.addAgentMessage("session-1", createAgentMessage("session-1", "user", "msg1"));
      store.addAgentMessage("session-1", createAgentMessage("session-1", "assistant", "msg2"));

      const state = useStore.getState();

      // Extract messages using derived selector
      const derivedMessages = selectAgentMessagesFromTimeline(state.timelines["session-1"]);

      // Verify derived values are correct
      expect(derivedMessages).toHaveLength(2);
      expect(derivedMessages[0].content).toBe("msg1");
      expect(derivedMessages[1].content).toBe("msg2");
    });

    it("should maintain consistency after mixed additions", () => {
      createSession("session-1");
      const store = useStore.getState();

      store.addAgentMessage("session-1", createAgentMessage("session-1", "user", "Hello"));
      store.handleCommandStart("session-1", "ls");
      store.handleCommandEnd("session-1", 0);
      store.addAgentMessage("session-1", createAgentMessage("session-1", "assistant", "Sure"));
      store.handleCommandStart("session-1", "pwd");
      store.handleCommandEnd("session-1", 0);

      const state = useStore.getState();

      // Timeline has all blocks
      expect(state.timelines["session-1"]).toHaveLength(4);

      // Verify derived commands
      const derivedCommands = selectCommandBlocksFromTimeline(state.timelines["session-1"]);
      expect(derivedCommands).toHaveLength(2);

      // Verify derived messages
      const derivedMessages = selectAgentMessagesFromTimeline(state.timelines["session-1"]);
      expect(derivedMessages).toHaveLength(2);
    });
  });

  describe("toggleBlockCollapse updates", () => {
    it("should update collapse state in timeline", () => {
      createSession("session-1");
      const store = useStore.getState();

      store.handleCommandStart("session-1", "ls");
      store.handleCommandEnd("session-1", 0);

      const stateAfterAdd = useStore.getState();
      const blocks = selectCommandBlocksFromTimeline(stateAfterAdd.timelines["session-1"]);
      const blockId = blocks[0].id;

      // Initially not collapsed
      expect(blocks[0].isCollapsed).toBe(false);

      // Toggle collapse
      store.toggleBlockCollapse(blockId);

      const stateAfterToggle = useStore.getState();

      // Timeline block should be collapsed
      const timelineBlock = stateAfterToggle.timelines["session-1"].find(
        (b) => b.type === "command" && b.id === blockId
      );
      expect(timelineBlock?.type === "command" && timelineBlock.data.isCollapsed).toBe(true);

      // Derived selector should also reflect the change
      const blocksAfterToggle = selectCommandBlocksFromTimeline(
        stateAfterToggle.timelines["session-1"]
      );
      expect(blocksAfterToggle[0].isCollapsed).toBe(true);
    });
  });

  describe("updateToolCallStatus updates", () => {
    it("should update tool call status in timeline", () => {
      createSession("session-1");
      const store = useStore.getState();

      const message: AgentMessage = {
        id: "msg-1",
        sessionId: "session-1",
        role: "assistant",
        content: "Let me help.",
        timestamp: new Date().toISOString(),
        toolCalls: [
          {
            id: "tool-1",
            name: "read_file",
            args: { path: "/test.txt" },
            status: "pending",
          },
        ],
      };

      store.addAgentMessage("session-1", message);

      // Update tool status
      store.updateToolCallStatus("session-1", "tool-1", "completed", "file contents");

      const state = useStore.getState();
      const messages = selectAgentMessagesFromTimeline(state.timelines["session-1"]);
      const toolCall = messages[0].toolCalls?.[0];

      expect(toolCall?.status).toBe("completed");
      expect(toolCall?.result).toBe("file contents");
    });
  });

  describe("clearTimeline clears timeline", () => {
    it("should clear timelines so derived selectors return empty arrays", () => {
      createSession("session-1");
      const store = useStore.getState();

      // Add mixed content
      store.handleCommandStart("session-1", "ls");
      store.handleCommandEnd("session-1", 0);
      store.addAgentMessage("session-1", createAgentMessage("session-1", "user", "Hello"));

      // Verify content exists via derived selectors
      const stateBefore = useStore.getState();
      expect(stateBefore.timelines["session-1"].length).toBeGreaterThan(0);
      expect(
        selectCommandBlocksFromTimeline(stateBefore.timelines["session-1"]).length
      ).toBeGreaterThan(0);
      expect(
        selectAgentMessagesFromTimeline(stateBefore.timelines["session-1"]).length
      ).toBeGreaterThan(0);

      // Clear timeline
      store.clearTimeline("session-1");

      // Verify derived selectors return empty arrays
      const stateAfter = useStore.getState();
      expect(stateAfter.timelines["session-1"]).toHaveLength(0);
      expect(selectCommandBlocksFromTimeline(stateAfter.timelines["session-1"])).toHaveLength(0);
      expect(selectAgentMessagesFromTimeline(stateAfter.timelines["session-1"])).toHaveLength(0);
    });
  });

  describe("restoreAgentMessages", () => {
    it("should restore messages to timeline", () => {
      createSession("session-1");

      const messages: AgentMessage[] = [
        {
          id: "msg-1",
          sessionId: "session-1",
          role: "user",
          content: "Hello",
          timestamp: "2024-01-01T10:00:00Z",
        },
        {
          id: "msg-2",
          sessionId: "session-1",
          role: "assistant",
          content: "Hi there!",
          timestamp: "2024-01-01T10:00:01Z",
        },
      ];

      useStore.getState().restoreAgentMessages("session-1", messages);

      const state = useStore.getState();

      // timeline should be populated
      expect(state.timelines["session-1"]).toHaveLength(2);
      expect(state.timelines["session-1"][0].type).toBe("agent_message");

      // Derived selector should return restored messages
      const derivedMessages = selectAgentMessagesFromTimeline(state.timelines["session-1"]);
      expect(derivedMessages).toHaveLength(2);
      expect(derivedMessages[0].content).toBe("Hello");
    });

    it("should replace existing messages, not append", () => {
      createSession("session-1");
      const store = useStore.getState();

      // Add initial message
      store.addAgentMessage("session-1", createAgentMessage("session-1", "user", "Old message"));

      // Restore with new messages
      const newMessages: AgentMessage[] = [
        {
          id: "new-msg",
          sessionId: "session-1",
          role: "user",
          content: "New message only",
          timestamp: "2024-01-01T10:00:00Z",
        },
      ];

      store.restoreAgentMessages("session-1", newMessages);

      const state = useStore.getState();
      const derivedMessages = selectAgentMessagesFromTimeline(state.timelines["session-1"]);
      expect(derivedMessages).toHaveLength(1);
      expect(derivedMessages[0].content).toBe("New message only");
      expect(state.timelines["session-1"]).toHaveLength(1);
    });
  });

  describe("Derived selectors return empty arrays for missing sessions", () => {
    it("should return empty array for missing session commandBlocks", () => {
      const state = useStore.getState();

      // Derived selector should return empty array
      const blocks1 = selectCommandBlocksFromTimeline(state.timelines["non-existent"]);
      const blocks2 = selectCommandBlocksFromTimeline(state.timelines["non-existent"]);

      expect(blocks1).toEqual([]);
      expect(blocks2).toEqual([]);
    });

    it("should return empty array for missing session agentMessages", () => {
      const state = useStore.getState();

      // Derived selector should return empty array
      const messages1 = selectAgentMessagesFromTimeline(state.timelines["non-existent"]);
      const messages2 = selectAgentMessagesFromTimeline(state.timelines["non-existent"]);

      expect(messages1).toEqual([]);
      expect(messages2).toEqual([]);
    });
  });
});
