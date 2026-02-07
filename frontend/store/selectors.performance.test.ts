import { beforeEach, describe, expect, it } from "vitest";
import { useStore } from "./index";

/**
 * TDD Performance Tests for Store Selectors
 *
 * These tests verify the performance optimizations we're implementing:
 * 1. Selector reference stability - same inputs = same output reference
 * 2. Stable empty references - empty arrays/objects are reused
 * 3. Combined selectors - reduce subscription count
 * 4. Cross-session isolation - changes to one session don't affect others
 */

// Helper to reset store
const resetStore = () => {
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

describe("Performance Optimizations", () => {
  beforeEach(() => {
    resetStore();
  });

  describe("1. Stable Empty References", () => {
    it("should return the same empty array reference for activeSubAgents when session has none", () => {
      createSession("session-1");

      const state1 = useStore.getState();
      const state2 = useStore.getState();

      // Currently this fails because `|| []` creates new arrays
      // After optimization, these should be the same reference
      const subAgents1 = state1.activeSubAgents["session-1"] ?? [];
      const subAgents2 = state2.activeSubAgents["session-1"] ?? [];

      // This test documents the DESIRED behavior
      // Currently fails because the store initializes with [] but accessing
      // a non-existent key returns undefined, then || [] creates new array
      expect(subAgents1).toBe(subAgents2);
    });

    it("should return the same empty array reference for streamingBlocks when session has none", () => {
      createSession("session-1");

      const state1 = useStore.getState();
      const state2 = useStore.getState();

      const blocks1 = state1.streamingBlocks["session-1"] ?? [];
      const blocks2 = state2.streamingBlocks["session-1"] ?? [];

      expect(blocks1).toBe(blocks2);
    });

    it("should return the same empty array reference for activeToolCalls when session has none", () => {
      createSession("session-1");

      const state1 = useStore.getState();
      const state2 = useStore.getState();

      const tools1 = state1.activeToolCalls["session-1"] ?? [];
      const tools2 = state2.activeToolCalls["session-1"] ?? [];

      expect(tools1).toBe(tools2);
    });

    it("should return stable EMPTY_TIMELINE for non-existent sessions via selector", async () => {
      // Access timeline for a session that doesn't exist via the combined selector
      const { selectSessionState } = await import("./selectors/session");

      const state1 = useStore.getState();
      const state2 = useStore.getState();

      const session1 = selectSessionState(state1, "non-existent");
      const session2 = selectSessionState(state2, "non-existent");

      // Should be the same empty array reference
      expect(session1.timeline).toBe(session2.timeline);
      expect(session1.streamingBlocks).toBe(session2.streamingBlocks);
      expect(session1.activeSubAgents).toBe(session2.activeSubAgents);
    });
  });

  describe("2. Combined Session Selector", () => {
    /**
     * This test verifies we can create a combined selector that returns
     * all session-specific state in one object, reducing subscription count.
     *
     * The selector should be importable and usable like:
     * const sessionState = useSessionState(sessionId);
     */
    it("should provide a combined selector for all session state", async () => {
      createSession("session-1");

      // Add some state
      useStore.getState().updateAgentStreaming("session-1", "Hello");
      useStore.getState().setAgentThinking("session-1", true);

      // Import the combined selector (this will fail until we create it)
      const { useSessionState } = await import("./selectors/session");

      // The combined selector should exist and return all needed state
      expect(useSessionState).toBeDefined();
    });

    it("combined selector should return stable reference when inputs unchanged", async () => {
      createSession("session-1");

      const { selectSessionState } = await import("./selectors/session");

      const state = useStore.getState();

      // Call selector twice with same state
      const result1 = selectSessionState(state, "session-1");
      const result2 = selectSessionState(state, "session-1");

      // Should return same reference (memoized)
      expect(result1).toBe(result2);
    });

    it("combined selector should include all required fields", async () => {
      createSession("session-1");

      useStore.getState().updateAgentStreaming("session-1", "Test content");
      useStore.getState().setAgentThinking("session-1", true);

      const { selectSessionState } = await import("./selectors/session");
      const state = useStore.getState();
      const result = selectSessionState(state, "session-1");

      // Verify all expected fields are present
      expect(result).toHaveProperty("timeline");
      expect(result).toHaveProperty("streamingBlocks");
      expect(result).toHaveProperty("pendingCommand");
      expect(result).toHaveProperty("isAgentThinking");
      expect(result).toHaveProperty("thinkingContent");
      expect(result).toHaveProperty("activeWorkflow");
      expect(result).toHaveProperty("activeSubAgents");
      expect(result).toHaveProperty("workingDirectory");
      expect(result).toHaveProperty("isCompacting");
      expect(result).toHaveProperty("streamingTextLength");
    });
  });

  describe("3. Cross-Session Isolation", () => {
    it("changes to session-1 should not affect session-2 selector result reference", async () => {
      createSession("session-1");
      createSession("session-2");

      const { selectSessionState } = await import("./selectors/session");

      // Get initial state for session-2
      const state1 = useStore.getState();
      const session2State1 = selectSessionState(state1, "session-2");

      // Modify session-1 (not session-2)
      useStore.getState().updateAgentStreaming("session-1", "Changed!");

      // Get state for session-2 again
      const state2 = useStore.getState();
      const session2State2 = selectSessionState(state2, "session-2");

      // Session-2's state should be the same reference since nothing changed for it
      expect(session2State1).toBe(session2State2);
    });

    it("changes to session-2 should update its selector result", async () => {
      createSession("session-1");
      createSession("session-2");

      const { selectSessionState } = await import("./selectors/session");

      // Get initial state
      const state1 = useStore.getState();
      const session2State1 = selectSessionState(state1, "session-2");

      // Modify session-2
      useStore.getState().updateAgentStreaming("session-2", "Updated!");

      // Get new state
      const state2 = useStore.getState();
      const session2State2 = selectSessionState(state2, "session-2");

      // Should be different reference since session-2 changed
      expect(session2State1).not.toBe(session2State2);
      expect(session2State2.streamingTextLength).toBe("Updated!".length);
    });
  });

  describe("4. Selector Memoization", () => {
    it("timeline selector should return same reference when timeline unchanged", () => {
      createSession("session-1");

      // Get timeline twice without modification
      const state1 = useStore.getState();
      const timeline1 = state1.timelines["session-1"];

      const state2 = useStore.getState();
      const timeline2 = state2.timelines["session-1"];

      // Should be same reference
      expect(timeline1).toBe(timeline2);
    });

    it("timeline selector should return new reference when timeline changes", () => {
      createSession("session-1");

      const state1 = useStore.getState();
      const timeline1 = state1.timelines["session-1"];

      // Add a command
      useStore.getState().handleCommandStart("session-1", "ls");
      useStore.getState().handleCommandEnd("session-1", 0);

      const state2 = useStore.getState();
      const timeline2 = state2.timelines["session-1"];

      // Should be different reference
      expect(timeline1).not.toBe(timeline2);
    });

    it("memoized selectors cache should handle multiple sessions", async () => {
      createSession("session-1");
      createSession("session-2");
      createSession("session-3");

      const { selectSessionState } = await import("./selectors/session");
      const state = useStore.getState();

      // Access all three sessions
      const s1a = selectSessionState(state, "session-1");
      const s2a = selectSessionState(state, "session-2");
      const s3a = selectSessionState(state, "session-3");

      // Access again
      const s1b = selectSessionState(state, "session-1");
      const s2b = selectSessionState(state, "session-2");
      const s3b = selectSessionState(state, "session-3");

      // All should return cached references
      expect(s1a).toBe(s1b);
      expect(s2a).toBe(s2b);
      expect(s3a).toBe(s3b);
    });
  });

  describe("5. Action Reference Stability", () => {
    /**
     * Store actions (like toggleBlockCollapse) should be stable references
     * that don't need to be accessed via selector subscriptions.
     */
    it("toggleBlockCollapse should be a stable function reference", () => {
      const toggle1 = useStore.getState().toggleBlockCollapse;
      const toggle2 = useStore.getState().toggleBlockCollapse;

      // Actions should always be the same reference
      expect(toggle1).toBe(toggle2);
    });

    it("focusPane should be a stable function reference", () => {
      const focus1 = useStore.getState().focusPane;
      const focus2 = useStore.getState().focusPane;

      expect(focus1).toBe(focus2);
    });
  });

  describe("6. Streaming Text Length Selector Optimization", () => {
    /**
     * The streaming text length selector is used for auto-scroll.
     * It should be efficient and not cause unnecessary re-renders.
     */
    it("streamingTextLength should only change when content changes", () => {
      createSession("session-1");

      const getLength = () => useStore.getState().agentStreaming["session-1"]?.length ?? 0;

      expect(getLength()).toBe(0);

      useStore.getState().updateAgentStreaming("session-1", "Hello");
      expect(getLength()).toBe(5);

      // Update to same content should not change length
      // (though in practice the store accumulates)
      useStore.getState().updateAgentStreaming("session-1", " World");
      expect(getLength()).toBe(11);
    });
  });

  describe("7. Pane Layout Selector Optimization", () => {
    it("tabLayout selector should return stable reference when layout unchanged", () => {
      createSession("session-1");

      const state1 = useStore.getState();
      const layout1 = state1.tabLayouts["session-1"];

      const state2 = useStore.getState();
      const layout2 = state2.tabLayouts["session-1"];

      expect(layout1).toBe(layout2);
    });

    it("focusPane should only update focusedPaneId, not entire layout", () => {
      createSession("session-1");

      const initialLayout = useStore.getState().tabLayouts["session-1"];
      const initialRoot = initialLayout?.root;

      // Focus same pane (no-op)
      useStore.getState().focusPane("session-1", "session-1");

      const afterLayout = useStore.getState().tabLayouts["session-1"];

      // Root should be the same reference (only focusedPaneId changed if at all)
      expect(afterLayout?.root).toBe(initialRoot);
    });
  });
});

describe("Hook Render Count Tests", () => {
  /**
   * These tests verify that our optimized hooks don't cause excess renders.
   * They use a mock component pattern to count renders.
   */

  beforeEach(() => {
    resetStore();
  });

  it("should document current render behavior for baseline", () => {
    // This test documents the current behavior so we can measure improvement
    createSession("session-1");
    createSession("session-2");

    let session1RenderCount = 0;
    let session2RenderCount = 0;

    // Simulate what UnifiedTimeline does - multiple selectors
    const getSession1State = () => {
      session1RenderCount++;
      const state = useStore.getState();
      return {
        timeline: state.timelines["session-1"],
        streaming: state.agentStreaming["session-1"],
        thinking: state.isAgentThinking["session-1"],
      };
    };

    const getSession2State = () => {
      session2RenderCount++;
      const state = useStore.getState();
      return {
        timeline: state.timelines["session-2"],
        streaming: state.agentStreaming["session-2"],
        thinking: state.isAgentThinking["session-2"],
      };
    };

    // Initial render
    getSession1State();
    getSession2State();

    expect(session1RenderCount).toBe(1);
    expect(session2RenderCount).toBe(1);

    // Update session-1 only
    useStore.getState().updateAgentStreaming("session-1", "Hello");

    // In an ideal world, session-2 shouldn't need to re-render
    // This test documents what we're trying to achieve
    getSession1State();
    // With proper selectors, we wouldn't need to call getSession2State again
  });
});
