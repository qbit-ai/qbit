import { beforeEach, describe, expect, it } from "vitest";
import { useStore } from "../index";
import {
  clearAllPaneLeafCaches,
  clearPaneLeafCache,
  type PaneLeafState,
  selectPaneLeafState,
} from "./pane-leaf";

/**
 * Unit tests for the PaneLeaf selector.
 *
 * These tests verify:
 * 1. Correct data extraction from store
 * 2. Memoization behavior - stable references when state unchanged
 * 3. Cache invalidation when relevant state changes
 * 4. Cross-session isolation - changes to unrelated state don't cause cache misses
 */

// Helper to reset store and caches
const resetStore = () => {
  clearAllPaneLeafCaches();
  useStore.setState({
    sessions: {},
    activeSessionId: null,
    timelines: {},
    pendingCommand: {},
    agentStreaming: {},
    agentStreamingBuffer: {},
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

describe("selectPaneLeafState", () => {
  beforeEach(() => {
    resetStore();
  });

  describe("Data Extraction", () => {
    it("should extract focusedPaneId from tabLayout", () => {
      createSession("tab-1");

      const state = useStore.getState();
      const result = selectPaneLeafState(state, "tab-1", "tab-1");

      expect(result.focusedPaneId).toBe("tab-1");
    });

    it("should extract renderMode from session", () => {
      createSession("session-1");
      useStore.getState().setRenderMode("session-1", "fullterm");

      const state = useStore.getState();
      const result = selectPaneLeafState(state, "session-1", "session-1");

      expect(result.renderMode).toBe("fullterm");
    });

    it("should extract workingDirectory from session", () => {
      createSession("session-1");

      const state = useStore.getState();
      const result = selectPaneLeafState(state, "session-1", "session-1");

      expect(result.workingDirectory).toBe("/home/session-1");
    });

    it("should extract tabType from session", () => {
      createSession("session-1");

      const state = useStore.getState();
      const result = selectPaneLeafState(state, "session-1", "session-1");

      expect(result.tabType).toBe("terminal");
    });

    it("should detect if session exists", () => {
      createSession("session-1");

      const state = useStore.getState();
      const result = selectPaneLeafState(state, "session-1", "session-1");

      expect(result.sessionExists).toBe(true);
    });

    it("should return sessionExists=false for non-existent session", () => {
      createSession("tab-1");

      const state = useStore.getState();
      const result = selectPaneLeafState(state, "tab-1", "non-existent");

      expect(result.sessionExists).toBe(false);
    });
  });

  describe("Default Values", () => {
    it("should return null for missing focusedPaneId", () => {
      // No tab layout exists
      const state = useStore.getState();
      const result = selectPaneLeafState(state, "non-existent-tab", "session-1");

      expect(result.focusedPaneId).toBeNull();
    });

    it("should return 'timeline' for missing renderMode", () => {
      createSession("session-1");

      const state = useStore.getState();
      const result = selectPaneLeafState(state, "session-1", "session-1");

      expect(result.renderMode).toBe("timeline");
    });

    it("should return undefined for missing workingDirectory", () => {
      createSession("tab-1");

      const state = useStore.getState();
      const result = selectPaneLeafState(state, "tab-1", "non-existent-session");

      expect(result.workingDirectory).toBeUndefined();
    });

    it("should return 'terminal' for missing tabType", () => {
      createSession("tab-1");

      const state = useStore.getState();
      const result = selectPaneLeafState(state, "tab-1", "non-existent-session");

      expect(result.tabType).toBe("terminal");
    });
  });

  describe("Memoization", () => {
    it("should return same reference when state unchanged", () => {
      createSession("session-1");

      const state = useStore.getState();
      const result1 = selectPaneLeafState(state, "session-1", "session-1");
      const result2 = selectPaneLeafState(state, "session-1", "session-1");

      expect(result1).toBe(result2);
    });

    it("should return new reference when focusedPaneId changes", () => {
      createSession("session-1");
      createSession("session-2");

      // Get initial state BEFORE split (focusedPaneId is "session-1")
      const state1 = useStore.getState();
      const result1 = selectPaneLeafState(state1, "session-1", "session-1");

      // Split creates a second pane - note: splitPane automatically focuses the new pane
      useStore.getState().splitPane("session-1", "session-1", "horizontal", "pane-2", "session-2");

      const state2 = useStore.getState();
      const result2 = selectPaneLeafState(state2, "session-1", "session-1");

      expect(result1).not.toBe(result2);
      expect(result1.focusedPaneId).toBe("session-1"); // Before split
      expect(result2.focusedPaneId).toBe("pane-2"); // After split (new pane gets focus)
    });

    it("should return new reference when renderMode changes", () => {
      createSession("session-1");

      const state1 = useStore.getState();
      const result1 = selectPaneLeafState(state1, "session-1", "session-1");

      useStore.getState().setRenderMode("session-1", "fullterm");

      const state2 = useStore.getState();
      const result2 = selectPaneLeafState(state2, "session-1", "session-1");

      expect(result1).not.toBe(result2);
      expect(result1.renderMode).toBe("timeline");
      expect(result2.renderMode).toBe("fullterm");
    });

    it("should return new reference when workingDirectory changes", () => {
      createSession("session-1");

      const state1 = useStore.getState();
      const result1 = selectPaneLeafState(state1, "session-1", "session-1");

      useStore.getState().updateWorkingDirectory("session-1", "/new/path");

      const state2 = useStore.getState();
      const result2 = selectPaneLeafState(state2, "session-1", "session-1");

      expect(result1).not.toBe(result2);
      expect(result1.workingDirectory).toBe("/home/session-1");
      expect(result2.workingDirectory).toBe("/new/path");
    });
  });

  describe("Cross-Session Isolation", () => {
    it("changes to session-2 should not affect session-1 pane leaf state", () => {
      createSession("session-1");
      createSession("session-2");

      const state1 = useStore.getState();
      const result1 = selectPaneLeafState(state1, "session-1", "session-1");

      // Modify session-2 (should not affect session-1's pane leaf state)
      useStore.getState().setRenderMode("session-2", "fullterm");
      useStore.getState().updateWorkingDirectory("session-2", "/different/path");
      useStore.getState().updateAgentStreaming("session-2", "Hello from session 2");

      const state2 = useStore.getState();
      const result2 = selectPaneLeafState(state2, "session-1", "session-1");

      // Session-1's pane leaf state should be the same reference
      expect(result1).toBe(result2);
    });

    it("changes to unrelated store state should not affect pane leaf state", () => {
      createSession("session-1");

      const state1 = useStore.getState();
      const result1 = selectPaneLeafState(state1, "session-1", "session-1");

      // Modify unrelated state
      useStore.getState().setAgentThinking("session-1", true);
      useStore.getState().updateAgentStreaming("session-1", "Thinking...");

      const state2 = useStore.getState();
      const result2 = selectPaneLeafState(state2, "session-1", "session-1");

      // Pane leaf state should still be the same reference
      // (agentThinking and agentStreaming are not part of PaneLeafState)
      expect(result1).toBe(result2);
    });
  });

  describe("Cache Management", () => {
    it("clearPaneLeafCache should invalidate cache for specific tab+session", () => {
      createSession("session-1");

      const state = useStore.getState();
      const result1 = selectPaneLeafState(state, "session-1", "session-1");

      clearPaneLeafCache("session-1", "session-1");

      // Same state, but cache was cleared
      const result2 = selectPaneLeafState(state, "session-1", "session-1");

      // Should be a new reference (cache was cleared)
      expect(result1).not.toBe(result2);

      // But values should be equal
      expect(result1.focusedPaneId).toBe(result2.focusedPaneId);
      expect(result1.renderMode).toBe(result2.renderMode);
    });

    it("clearAllPaneLeafCaches should invalidate all caches", () => {
      createSession("session-1");
      createSession("session-2");

      const state = useStore.getState();
      const result1a = selectPaneLeafState(state, "session-1", "session-1");
      const result2a = selectPaneLeafState(state, "session-2", "session-2");

      clearAllPaneLeafCaches();

      const result1b = selectPaneLeafState(state, "session-1", "session-1");
      const result2b = selectPaneLeafState(state, "session-2", "session-2");

      expect(result1a).not.toBe(result1b);
      expect(result2a).not.toBe(result2b);
    });
  });

  describe("Return Type Correctness", () => {
    it("should return all expected fields in PaneLeafState", () => {
      createSession("session-1");

      const state = useStore.getState();
      const result = selectPaneLeafState(state, "session-1", "session-1");

      // Type check: ensure all fields are present
      const expectedFields: (keyof PaneLeafState)[] = [
        "focusedPaneId",
        "renderMode",
        "workingDirectory",
        "tabType",
        "sessionExists",
        "sessionName",
      ];

      for (const field of expectedFields) {
        expect(field in result).toBe(true);
      }
    });
  });
});
