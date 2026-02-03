import { beforeEach, describe, expect, it } from "vitest";
import { useStore } from "../index";
import { clearAllSessionCaches } from "./session";

/**
 * TDD Tests for UnifiedInput Combined Selector
 *
 * Issue: UnifiedInput has ~15+ separate store subscriptions
 * Goal: Create a combined selector similar to useSessionState
 */

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
    processedToolRequests: new Set<string>(),
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

describe("UnifiedInput Combined Selector", () => {
  beforeEach(() => {
    resetStore();
  });

  describe("selectUnifiedInputState", () => {
    it("should provide a combined selector for all UnifiedInput state", async () => {
      createSession("session-1");

      const { selectUnifiedInputState } = await import("./unified-input");

      expect(selectUnifiedInputState).toBeDefined();
    });

    it("should return stable reference when inputs unchanged", async () => {
      createSession("session-1");

      const { selectUnifiedInputState } = await import("./unified-input");

      const state = useStore.getState();
      const result1 = selectUnifiedInputState(state, "session-1");
      const result2 = selectUnifiedInputState(state, "session-1");

      expect(result1).toBe(result2);
    });

    it("should include all required fields for UnifiedInput", async () => {
      createSession("session-1");

      const { selectUnifiedInputState } = await import("./unified-input");

      const state = useStore.getState();
      const result = selectUnifiedInputState(state, "session-1");

      // Core input state
      expect(result).toHaveProperty("inputMode");
      expect(result).toHaveProperty("workingDirectory");
      expect(result).toHaveProperty("virtualEnv");

      // Agent state
      expect(result).toHaveProperty("isAgentResponding");
      expect(result).toHaveProperty("isCompacting");
      expect(result).toHaveProperty("isSessionDead");
      expect(result).toHaveProperty("streamingBlocksLength");

      // Git state
      expect(result).toHaveProperty("gitBranch");
      expect(result).toHaveProperty("gitStatus");
    });

    it("should return new reference when session state changes", async () => {
      createSession("session-1");

      const { selectUnifiedInputState, clearUnifiedInputCache } = await import(
        "./unified-input"
      );

      const state1 = useStore.getState();
      const result1 = selectUnifiedInputState(state1, "session-1");

      // Change input mode
      useStore.getState().setInputMode("session-1", "agent");

      const state2 = useStore.getState();
      const result2 = selectUnifiedInputState(state2, "session-1");

      expect(result1).not.toBe(result2);
      expect(result2.inputMode).toBe("agent");

      clearUnifiedInputCache("session-1");
    });

    it("changes to session-1 should not affect session-2 result", async () => {
      createSession("session-1");
      createSession("session-2");

      const { selectUnifiedInputState } = await import("./unified-input");

      const state1 = useStore.getState();
      const session2Result1 = selectUnifiedInputState(state1, "session-2");

      // Modify session-1
      useStore.getState().setInputMode("session-1", "agent");
      useStore.getState().setCompacting("session-1", true);

      const state2 = useStore.getState();
      const session2Result2 = selectUnifiedInputState(state2, "session-2");

      // Session-2 result should be the same reference
      expect(session2Result1).toBe(session2Result2);
    });
  });
});
