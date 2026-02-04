import { beforeEach, describe, expect, it } from "vitest";
import { useStore } from "../index";
import { clearAllSessionCaches } from "./session";

/**
 * TDD Tests for TabBar Combined Selector
 *
 * Issue: TabBar subscribes to entire Record objects (sessions, tabLayouts, etc.)
 * Goal: Create a combined selector that only returns what TabBar needs
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

describe("TabBar Combined Selector", () => {
  beforeEach(() => {
    resetStore();
  });

  describe("selectTabBarState", () => {
    it("should provide a combined selector for TabBar state", async () => {
      const { selectTabBarState } = await import("./tab-bar");

      expect(selectTabBarState).toBeDefined();
    });

    it("should return stable reference when inputs unchanged", async () => {
      createSession("session-1");

      const { selectTabBarState } = await import("./tab-bar");

      const state = useStore.getState();
      const result1 = selectTabBarState(state);
      const result2 = selectTabBarState(state);

      expect(result1).toBe(result2);
    });

    it("should include tab list with essential info only", async () => {
      createSession("session-1");
      createSession("session-2");

      const { selectTabBarState } = await import("./tab-bar");

      const state = useStore.getState();
      const result = selectTabBarState(state);

      expect(result).toHaveProperty("tabs");
      expect(result).toHaveProperty("activeSessionId");
      expect(Array.isArray(result.tabs)).toBe(true);

      // Each tab should have minimal required info
      if (result.tabs.length > 0) {
        const tab = result.tabs[0];
        expect(tab).toHaveProperty("id");
        expect(tab).toHaveProperty("name");
        expect(tab).toHaveProperty("isRunning");
        expect(tab).toHaveProperty("hasNewActivity");
      }
    });

    it("should only update when relevant tab info changes", async () => {
      createSession("session-1");
      createSession("session-2");

      const { selectTabBarState, clearTabBarCache } = await import("./tab-bar");

      const state1 = useStore.getState();
      const result1 = selectTabBarState(state1);

      // Change something unrelated to tabs (e.g., timeline content)
      useStore.getState().updateAgentStreaming("session-1", "Hello world");

      const state2 = useStore.getState();
      const result2 = selectTabBarState(state2);

      // Tab bar state should still be the same since tab info didn't change
      // (streaming content doesn't affect tab display)
      expect(result1.tabs[0].name).toBe(result2.tabs[0].name);

      clearTabBarCache();
    });

    it("should update when agent responding state changes", async () => {
      createSession("session-1");

      const { selectTabBarState, clearTabBarCache } = await import("./tab-bar");

      const state1 = useStore.getState();
      const result1 = selectTabBarState(state1);

      // Change isAgentResponding (affects "running" indicator)
      useStore.getState().setAgentResponding("session-1", true);

      const state2 = useStore.getState();
      const result2 = selectTabBarState(state2);

      // Should be different since isRunning changed
      expect(result1.tabs[0].isRunning).toBe(false);
      expect(result2.tabs[0].isRunning).toBe(true);

      clearTabBarCache();
    });
  });

  describe("selectTabItemState", () => {
    it("should provide per-tab selector for individual tab items", async () => {
      createSession("session-1");

      const { selectTabItemState } = await import("./tab-bar");

      expect(selectTabItemState).toBeDefined();

      const state = useStore.getState();
      const result = selectTabItemState(state, "session-1");

      expect(result).toHaveProperty("name");
      expect(result).toHaveProperty("isRunning");
      expect(result).toHaveProperty("hasNewActivity");
    });

    it("changes to session-2 should not affect session-1 tab item result", async () => {
      createSession("session-1");
      createSession("session-2");

      const { selectTabItemState } = await import("./tab-bar");

      const state1 = useStore.getState();
      const session1Result1 = selectTabItemState(state1, "session-1");

      // Modify session-2
      useStore.getState().setAgentResponding("session-2", true);

      const state2 = useStore.getState();
      const session1Result2 = selectTabItemState(state2, "session-1");

      // Session-1 result should be the same reference
      expect(session1Result1).toBe(session1Result2);
    });
  });
});
