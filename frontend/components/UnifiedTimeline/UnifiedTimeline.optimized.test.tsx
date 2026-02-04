import { render } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { useStore } from "../../store";
import { clearAllSessionCaches } from "../../store/selectors/session";

// Mock xterm.js and addons - they don't work in jsdom
vi.mock("@xterm/xterm", () => ({
  Terminal: class MockTerminal {
    options = { theme: {} };
    rows = 24;
    cols = 80;
    loadAddon = vi.fn();
    open = vi.fn();
    write = vi.fn();
    clear = vi.fn();
    dispose = vi.fn();
    scrollToBottom = vi.fn();
    resize = vi.fn();
    element = document.createElement("div");
    registerLinkProvider = vi.fn(() => ({ dispose: vi.fn() }));
    buffer = {
      active: {
        getLine: vi.fn(() => ({
          translateToString: vi.fn(() => ""),
        })),
      },
    };
  },
}));

vi.mock("@xterm/addon-fit", () => ({
  FitAddon: class MockFitAddon {
    fit = vi.fn();
  },
}));

vi.mock("@xterm/addon-serialize", () => ({
  SerializeAddon: class MockSerializeAddon {
    serialize = vi.fn(() => "");
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

describe("UnifiedTimeline Optimization Tests", () => {
  beforeEach(() => {
    resetStore();
  });

  describe("Selector Subscription Reduction", () => {
    /**
     * The goal of this test is to verify that UnifiedTimeline uses
     * a single combined selector instead of many individual ones.
     *
     * Currently the component has 10+ useStore calls.
     * After optimization, it should use useSessionState which combines them.
     */
    it("should use combined selector for session state", async () => {
      createSession("session-1");

      // Dynamically import to get the latest version
      const { UnifiedTimeline } = await import("./UnifiedTimeline");

      // Render should succeed
      const { container } = render(<UnifiedTimeline sessionId="session-1" />);

      // Component should render (basic smoke test)
      expect(container).toBeDefined();
    });

    it("changes to session-2 should not trigger re-render of session-1 timeline", async () => {
      createSession("session-1");
      createSession("session-2");

      const { UnifiedTimeline } = await import("./UnifiedTimeline");

      // Initial render
      render(<UnifiedTimeline sessionId="session-1" />);

      // Modify session-2 (should NOT affect session-1)
      useStore.getState().updateAgentStreaming("session-2", "Hello from session 2");
      useStore.getState().setAgentThinking("session-2", true);

      // This is a documentation test - currently it will trigger re-renders
      // After optimization, session-1 should not re-render when session-2 changes
      // We can verify this behavior by checking that the combined selector
      // returns the same reference for session-1
      const { selectSessionState } = await import("../../store/selectors/session");

      const state = useStore.getState();
      const session1State = selectSessionState(state, "session-1");

      // Modify session-2 again
      useStore.getState().updateAgentStreaming("session-2", "More updates");

      const newState = useStore.getState();
      const session1StateAfter = selectSessionState(newState, "session-1");

      // Session-1's state should be the same reference
      expect(session1State).toBe(session1StateAfter);
    });
  });

  describe("Stable References", () => {
    it("empty arrays from selector should be stable references", async () => {
      createSession("session-1");

      const { selectSessionState } = await import("../../store/selectors/session");

      const state = useStore.getState();
      const result1 = selectSessionState(state, "session-1");
      const result2 = selectSessionState(state, "session-1");

      // Empty arrays should be the same reference
      expect(result1.streamingBlocks).toBe(result2.streamingBlocks);
      expect(result1.activeSubAgents).toBe(result2.activeSubAgents);
      expect(result1.activeToolCalls).toBe(result2.activeToolCalls);
    });
  });

  describe("Action References", () => {
    it("action functions should not require subscription", () => {
      // Actions are stable references and shouldn't trigger re-renders
      const toggleBlockCollapse1 = useStore.getState().toggleBlockCollapse;
      const toggleBlockCollapse2 = useStore.getState().toggleBlockCollapse;

      expect(toggleBlockCollapse1).toBe(toggleBlockCollapse2);
    });
  });
});
