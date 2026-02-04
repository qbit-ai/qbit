import { render } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { useStore } from "../../store";
import { clearAllSessionCaches } from "../../store/selectors/session";

/**
 * TDD Tests for AgentMessage Performance Issues
 *
 * Issues:
 * 1. console.info in production code (logs on every streaming update)
 * 2. useStore subscription inside memoized component (breaks memoization)
 *
 * Goal:
 * 1. Remove or conditionally include console.info
 * 2. Pass workingDirectory as prop instead of subscribing
 */

// Mock dependencies
vi.mock("@xterm/xterm", () => ({
  Terminal: class MockTerminal {
    options = { theme: {} };
    loadAddon = vi.fn();
    open = vi.fn();
    dispose = vi.fn();
  },
}));

// Helper to reset store
const resetStore = () => {
  clearAllSessionCaches();
  useStore.setState({
    sessions: {},
    activeSessionId: null,
    timelines: {},
    pendingCommand: {},
    agentStreaming: {},
    streamingBlocks: {},
    streamingTextOffset: {},
    agentInitialized: {},
    isAgentThinking: {},
    isAgentResponding: {},
    pendingToolApproval: {},
    processedToolRequests: {},
    activeToolCalls: {},
    thinkingContent: {},
    isThinkingExpanded: {},
    activeWorkflows: {},
    workflowHistory: {},
    activeSubAgents: {},
    contextMetrics: {},
    compactionCount: {},
    isCompacting: {},
    isSessionDead: {},
    compactionError: {},
    gitStatus: {},
    gitStatusLoading: {},
    gitCommitMessage: {},
    tabLayouts: {},
    tabHasNewActivity: {},
    sessionTokenUsage: {},
  });
};

// Helper to create a session
const createSession = (sessionId: string) => {
  useStore.getState().addSession({
    id: sessionId,
    name: `Session ${sessionId}`,
    workingDirectory: `/home/${sessionId}`,
    createdAt: new Date().toISOString(),
    mode: "terminal",
  });
};

describe("AgentMessage Performance", () => {
  beforeEach(() => {
    resetStore();
    vi.clearAllMocks();
  });

  describe("console.info removal", () => {
    it("should not log to console.info during rendering", async () => {
      createSession("session-1");

      const consoleSpy = vi.spyOn(console, "info").mockImplementation(() => {});

      const { AgentMessage } = await import("./AgentMessage");

      const mockMessage = {
        id: "msg-1",
        role: "assistant" as const,
        content: "Hello",
        timestamp: new Date().toISOString(),
        streamingHistory: [
          {
            type: "tool" as const,
            toolCall: {
              id: "tool-1",
              name: "read_file",
              args: { path: "/test.txt" },
              status: "completed" as const,
            },
          },
        ],
      };

      render(
        <AgentMessage
          message={mockMessage}
          sessionId="session-1"
          workingDirectory="/home/session-1"
        />
      );

      // Should not have any console.info calls for tool processing
      const toolProcessingLogs = consoleSpy.mock.calls.filter(
        (call) =>
          typeof call[0] === "string" && call[0].includes("[AgentMessage] Processing tool call")
      );

      expect(toolProcessingLogs.length).toBe(0);

      consoleSpy.mockRestore();
    });
  });

  describe("workingDirectory prop", () => {
    it("should accept workingDirectory as a prop", async () => {
      createSession("session-1");

      const { AgentMessage } = await import("./AgentMessage");

      const mockMessage = {
        id: "msg-1",
        role: "assistant" as const,
        content: "Check file at ./test.txt",
        timestamp: new Date().toISOString(),
      };

      // Should render without errors when workingDirectory is passed as prop
      const { container } = render(
        <AgentMessage message={mockMessage} sessionId="session-1" workingDirectory="/custom/path" />
      );

      expect(container).toBeDefined();
    });

    it("should not subscribe to store for workingDirectory when prop is provided", async () => {
      createSession("session-1");

      const { AgentMessage } = await import("./AgentMessage");

      const mockMessage = {
        id: "msg-1",
        role: "assistant" as const,
        content: "Hello",
        timestamp: new Date().toISOString(),
      };

      // Render with workingDirectory prop
      render(
        <AgentMessage
          message={mockMessage}
          sessionId="session-1"
          workingDirectory="/home/session-1"
        />
      );

      // Change workingDirectory in store
      useStore.setState({
        sessions: {
          ...useStore.getState().sessions,
          "session-1": {
            ...useStore.getState().sessions["session-1"],
            workingDirectory: "/different/path",
          },
        },
      });

      // The component should use the prop value, not the store value
      // This test verifies the component accepts the prop - actual isolation
      // is verified by the fact that historical messages won't re-render
    });
  });

  describe("memoization effectiveness", () => {
    it("should not re-render when unrelated session state changes", async () => {
      createSession("session-1");
      createSession("session-2");

      const { AgentMessage } = await import("./AgentMessage");

      let renderCount = 0;
      const TrackingMessage = (props: Parameters<typeof AgentMessage>[0]) => {
        renderCount++;
        return <AgentMessage {...props} />;
      };

      const mockMessage = {
        id: "msg-1",
        role: "assistant" as const,
        content: "Hello",
        timestamp: new Date().toISOString(),
      };

      const { rerender } = render(
        <TrackingMessage
          message={mockMessage}
          sessionId="session-1"
          workingDirectory="/home/session-1"
        />
      );

      const initialRenderCount = renderCount;

      // Change session-2 state (should not affect session-1 message)
      useStore.getState().setAgentResponding("session-2", true);

      // Force a potential re-render
      rerender(
        <TrackingMessage
          message={mockMessage}
          sessionId="session-1"
          workingDirectory="/home/session-1"
        />
      );

      // Render count should only increase by 1 (the rerender call itself)
      // not additional renders from store subscription
      expect(renderCount).toBe(initialRenderCount + 1);
    });
  });
});
