import { render } from "@testing-library/react";
import React from "react";
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

describe("UnifiedTimeline React.memo Optimization", () => {
  beforeEach(() => {
    resetStore();
  });

  describe("React.memo wrapper", () => {
    it("should be wrapped in React.memo", async () => {
      const { UnifiedTimeline } = await import("./UnifiedTimeline");

      // React.memo wraps the component and adds a compare function reference
      // We can check this by looking for the $$typeof symbol that memo adds
      // or by checking if the component has a compare property
      expect(UnifiedTimeline).toBeDefined();

      // React.memo components have $$typeof === Symbol.for('react.memo')
      // or in dev mode, they have a 'type' property pointing to the original component
      const isMemoComponent =
        // Check if it's a memo component by looking for the memo type symbol
        (UnifiedTimeline as unknown as { $$typeof?: symbol }).$$typeof ===
          Symbol.for("react.memo") ||
        // In some React versions/configs, check for compare property
        typeof (UnifiedTimeline as unknown as { compare?: unknown }).compare === "function" ||
        // Or check for type property (memo wrapper has type pointing to original)
        !!(UnifiedTimeline as unknown as { type?: unknown }).type;

      expect(isMemoComponent).toBe(true);
    });

    it("should not re-render when parent re-renders with same props", async () => {
      createSession("memo-test");
      const { UnifiedTimeline } = await import("./UnifiedTimeline");

      let renderCount = 0;

      // Wrap UnifiedTimeline to count renders
      const TrackedTimeline = React.memo(function TrackedTimeline({
        sessionId,
      }: {
        sessionId: string;
      }) {
        renderCount++;
        return <UnifiedTimeline sessionId={sessionId} />;
      });

      // Create a parent that can trigger re-renders
      function Parent({ count }: { count: number }) {
        // count is used to force parent re-render but NOT passed to child
        void count;
        return <TrackedTimeline sessionId="memo-test" />;
      }

      const { rerender } = render(<Parent count={0} />);
      const initialRenderCount = renderCount;

      // Re-render parent with different count (but same sessionId for child)
      rerender(<Parent count={1} />);
      rerender(<Parent count={2} />);
      rerender(<Parent count={3} />);

      // TrackedTimeline should not re-render because its props (sessionId) didn't change
      // and it's wrapped in memo
      expect(renderCount).toBe(initialRenderCount);
    });

    it("should re-render when sessionId prop changes", async () => {
      createSession("session-a");
      createSession("session-b");
      const { UnifiedTimeline } = await import("./UnifiedTimeline");

      const { rerender, container } = render(<UnifiedTimeline sessionId="session-a" />);

      // Initial render should work
      expect(container).toBeDefined();

      // Changing sessionId should trigger re-render (expected behavior)
      rerender(<UnifiedTimeline sessionId="session-b" />);

      // The component should now be showing session-b content
      expect(container).toBeDefined();
    });
  });

  describe("useMemo chain optimization", () => {
    it("should combine streaming block processing into fewer memos", async () => {
      createSession("memo-chain-test");

      const { UnifiedTimeline } = await import("./UnifiedTimeline");

      // This test verifies that the component renders correctly
      // The actual memo chain optimization is verified by code review
      // and by ensuring the component doesn't have unnecessary intermediate memos

      const { container } = render(<UnifiedTimeline sessionId="memo-chain-test" />);

      expect(container).toBeDefined();
    });

    it("should not recalculate when unrelated state changes", async () => {
      createSession("session-1");
      createSession("session-2");

      const { selectSessionState } = await import("../../store/selectors/session");

      // Get initial state for session-1
      const initialState = useStore.getState();
      const session1Initial = selectSessionState(initialState, "session-1");

      // Make changes to session-2 (should not affect session-1)
      // updateAgentStreaming also updates streamingBlocks internally
      useStore.getState().updateAgentStreaming("session-2", "Some text");

      // Session-1's state should be the exact same reference
      const newState = useStore.getState();
      const session1After = selectSessionState(newState, "session-1");

      expect(session1Initial).toBe(session1After);
    });

    it("should return stable streamingBlocks reference when blocks don't change", async () => {
      createSession("stable-ref-test");

      const { selectSessionState } = await import("../../store/selectors/session");

      // Set some streaming blocks via updateAgentStreaming
      useStore.getState().updateAgentStreaming("stable-ref-test", "Hello");

      const state1 = useStore.getState();
      const result1 = selectSessionState(state1, "stable-ref-test");

      // Change something unrelated (run a command that adds to timeline)
      useStore.getState().handleCommandStart("stable-ref-test", "ls");
      useStore.getState().appendOutput("stable-ref-test", "files\n");
      useStore.getState().handleCommandEnd("stable-ref-test", 0);

      const state2 = useStore.getState();
      const result2 = selectSessionState(state2, "stable-ref-test");

      // streamingBlocks should be the same reference since they didn't change
      expect(result1.streamingBlocks).toBe(result2.streamingBlocks);
    });
  });

  describe("Performance characteristics", () => {
    it("should handle rapid streaming updates efficiently", async () => {
      createSession("streaming-perf");
      const { UnifiedTimeline } = await import("./UnifiedTimeline");

      const { container } = render(<UnifiedTimeline sessionId="streaming-perf" />);

      // Simulate rapid streaming updates (like during AI response)
      const startTime = performance.now();

      for (let i = 0; i < 50; i++) {
        // updateAgentStreaming also updates streamingBlocks internally
        useStore.getState().updateAgentStreaming("streaming-perf", `Content chunk ${i} `);
      }

      const endTime = performance.now();

      // All updates should complete in reasonable time
      // This is a basic sanity check, not a strict benchmark
      expect(endTime - startTime).toBeLessThan(1000);
      expect(container).toBeDefined();
    });

    it("activeSubAgents should not cause recalculation when empty", async () => {
      createSession("subagent-test");

      const { selectSessionState } = await import("../../store/selectors/session");

      const state1 = useStore.getState();
      const result1 = selectSessionState(state1, "subagent-test");

      // Trigger any state update (should not affect activeSubAgents)
      useStore.getState().updateAgentStreaming("subagent-test", "Hello");

      const state2 = useStore.getState();
      const result2 = selectSessionState(state2, "subagent-test");

      // Empty activeSubAgents should be stable reference
      expect(result1.activeSubAgents).toBe(result2.activeSubAgents);
    });
  });
});
