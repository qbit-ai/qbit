import { beforeEach, describe, expect, it } from "vitest";
import { useStore } from "../index";
import {
  clearAllSessionCaches,
  clearSessionCache,
  type SessionState,
  selectSessionState,
} from "./session";

/**
 * Unit tests for the combined session selector.
 *
 * These tests verify:
 * 1. Correct data extraction from store
 * 2. Memoization behavior
 * 3. Cache invalidation
 * 4. Cross-session isolation
 */

// Helper to reset store and caches
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

describe("selectSessionState", () => {
  beforeEach(() => {
    resetStore();
  });

  describe("Data Extraction", () => {
    it("should extract timeline from store", () => {
      createSession("session-1");
      useStore.getState().handleCommandStart("session-1", "ls");
      useStore.getState().handleCommandEnd("session-1", 0);

      const state = useStore.getState();
      const result = selectSessionState(state, "session-1");

      expect(result.timeline).toHaveLength(1);
      expect(result.timeline[0].type).toBe("command");
    });

    it("should extract streamingBlocks from store", () => {
      createSession("session-1");
      useStore.getState().addStreamingToolBlock("session-1", {
        id: "tool-1",
        name: "read_file",
        args: { path: "/test.txt" },
      });

      const state = useStore.getState();
      const result = selectSessionState(state, "session-1");

      expect(result.streamingBlocks).toHaveLength(1);
      expect(result.streamingBlocks[0].type).toBe("tool");
    });

    it("should extract pendingCommand from store", () => {
      createSession("session-1");
      useStore.getState().handleCommandStart("session-1", "long-running-cmd");

      const state = useStore.getState();
      const result = selectSessionState(state, "session-1");

      expect(result.pendingCommand).not.toBeNull();
      expect(result.pendingCommand?.command).toBe("long-running-cmd");
    });

    it("should extract isAgentThinking from store", () => {
      createSession("session-1");
      useStore.getState().setAgentThinking("session-1", true);

      const state = useStore.getState();
      const result = selectSessionState(state, "session-1");

      expect(result.isAgentThinking).toBe(true);
    });

    it("should extract thinkingContent from store", () => {
      createSession("session-1");
      useStore.getState().appendThinkingContent("session-1", "Processing...");

      const state = useStore.getState();
      const result = selectSessionState(state, "session-1");

      expect(result.thinkingContent).toBe("Processing...");
    });

    it("should extract activeWorkflow from store", () => {
      createSession("session-1");
      useStore.getState().startWorkflow("session-1", {
        workflowId: "wf-1",
        workflowName: "test-workflow",
        workflowSessionId: "session-1",
      });

      const state = useStore.getState();
      const result = selectSessionState(state, "session-1");

      expect(result.activeWorkflow).not.toBeNull();
      expect(result.activeWorkflow?.workflowId).toBe("wf-1");
    });

    it("should extract activeSubAgents from store", () => {
      createSession("session-1");
      useStore.getState().startSubAgent("session-1", {
        agentId: "agent-1",
        agentName: "explorer",
        parentRequestId: "req-1",
        task: "explore codebase",
        depth: 1,
      });

      const state = useStore.getState();
      const result = selectSessionState(state, "session-1");

      expect(result.activeSubAgents).toHaveLength(1);
      expect(result.activeSubAgents[0].agentName).toBe("explorer");
    });

    it("should extract workingDirectory from session", () => {
      createSession("session-1");

      const state = useStore.getState();
      const result = selectSessionState(state, "session-1");

      expect(result.workingDirectory).toBe("/home/session-1");
    });

    it("should extract isCompacting from store", () => {
      createSession("session-1");
      useStore.getState().setCompacting("session-1", true);

      const state = useStore.getState();
      const result = selectSessionState(state, "session-1");

      expect(result.isCompacting).toBe(true);
    });

    it("should extract agentStreaming and streamingTextLength", () => {
      createSession("session-1");
      useStore.getState().updateAgentStreaming("session-1", "Hello world");

      const state = useStore.getState();
      const result = selectSessionState(state, "session-1");

      expect(result.agentStreaming).toBe("Hello world");
      expect(result.streamingTextLength).toBe(11);
    });
  });

  describe("Default Values", () => {
    it("should return empty array for missing timeline", () => {
      const state = useStore.getState();
      const result = selectSessionState(state, "non-existent");

      expect(result.timeline).toEqual([]);
    });

    it("should return empty array for missing streamingBlocks", () => {
      const state = useStore.getState();
      const result = selectSessionState(state, "non-existent");

      expect(result.streamingBlocks).toEqual([]);
    });

    it("should return null for missing pendingCommand", () => {
      createSession("session-1");
      const state = useStore.getState();
      const result = selectSessionState(state, "session-1");

      expect(result.pendingCommand).toBeNull();
    });

    it("should return false for missing isAgentThinking", () => {
      const state = useStore.getState();
      const result = selectSessionState(state, "non-existent");

      expect(result.isAgentThinking).toBe(false);
    });

    it("should return empty string for missing thinkingContent", () => {
      const state = useStore.getState();
      const result = selectSessionState(state, "non-existent");

      expect(result.thinkingContent).toBe("");
    });

    it("should return null for missing activeWorkflow", () => {
      createSession("session-1");
      const state = useStore.getState();
      const result = selectSessionState(state, "session-1");

      expect(result.activeWorkflow).toBeNull();
    });

    it("should return empty array for missing activeSubAgents", () => {
      const state = useStore.getState();
      const result = selectSessionState(state, "non-existent");

      expect(result.activeSubAgents).toEqual([]);
    });

    it("should return empty string for missing workingDirectory", () => {
      const state = useStore.getState();
      const result = selectSessionState(state, "non-existent");

      expect(result.workingDirectory).toBe("");
    });

    it("should return false for missing isCompacting", () => {
      const state = useStore.getState();
      const result = selectSessionState(state, "non-existent");

      expect(result.isCompacting).toBe(false);
    });

    it("should return empty string and 0 for missing agentStreaming", () => {
      const state = useStore.getState();
      const result = selectSessionState(state, "non-existent");

      expect(result.agentStreaming).toBe("");
      expect(result.streamingTextLength).toBe(0);
    });
  });

  describe("Memoization", () => {
    it("should return same reference when state unchanged", () => {
      createSession("session-1");

      const state = useStore.getState();
      const result1 = selectSessionState(state, "session-1");
      const result2 = selectSessionState(state, "session-1");

      expect(result1).toBe(result2);
    });

    it("should return new reference when timeline changes", () => {
      createSession("session-1");

      const state1 = useStore.getState();
      const result1 = selectSessionState(state1, "session-1");

      useStore.getState().handleCommandStart("session-1", "ls");
      useStore.getState().handleCommandEnd("session-1", 0);

      const state2 = useStore.getState();
      const result2 = selectSessionState(state2, "session-1");

      expect(result1).not.toBe(result2);
      expect(result2.timeline).toHaveLength(1);
    });

    it("should return new reference when streamingBlocks changes", () => {
      createSession("session-1");

      const state1 = useStore.getState();
      const result1 = selectSessionState(state1, "session-1");

      useStore.getState().addStreamingToolBlock("session-1", {
        id: "tool-1",
        name: "test",
        args: {},
      });

      const state2 = useStore.getState();
      const result2 = selectSessionState(state2, "session-1");

      expect(result1).not.toBe(result2);
    });

    it("should return new reference when isAgentThinking changes", () => {
      createSession("session-1");

      const state1 = useStore.getState();
      const result1 = selectSessionState(state1, "session-1");

      useStore.getState().setAgentThinking("session-1", true);

      const state2 = useStore.getState();
      const result2 = selectSessionState(state2, "session-1");

      expect(result1).not.toBe(result2);
      expect(result2.isAgentThinking).toBe(true);
    });

    it("should preserve stable empty array references", () => {
      createSession("session-1");

      const state = useStore.getState();
      const result1 = selectSessionState(state, "session-1");
      const result2 = selectSessionState(state, "session-1");

      // Empty arrays should be the same reference
      expect(result1.activeSubAgents).toBe(result2.activeSubAgents);
      expect(result1.activeToolCalls).toBe(result2.activeToolCalls);
    });
  });

  describe("Cross-Session Isolation", () => {
    it("changes to session-1 should not affect session-2 result", () => {
      createSession("session-1");
      createSession("session-2");

      const state1 = useStore.getState();
      const session2Result1 = selectSessionState(state1, "session-2");

      // Modify session-1
      useStore.getState().updateAgentStreaming("session-1", "Hello from session 1");
      useStore.getState().setAgentThinking("session-1", true);

      const state2 = useStore.getState();
      const session2Result2 = selectSessionState(state2, "session-2");

      // Session-2 result should be the same reference
      expect(session2Result1).toBe(session2Result2);
    });

    it("should handle many sessions efficiently", () => {
      // Create 10 sessions
      for (let i = 0; i < 10; i++) {
        createSession(`session-${i}`);
      }

      const state = useStore.getState();

      // Access all sessions
      const results: SessionState[] = [];
      for (let i = 0; i < 10; i++) {
        results.push(selectSessionState(state, `session-${i}`));
      }

      // All should be cached
      for (let i = 0; i < 10; i++) {
        const cached = selectSessionState(state, `session-${i}`);
        expect(cached).toBe(results[i]);
      }
    });
  });

  describe("Cache Management", () => {
    it("clearSessionCache should invalidate cache for specific session", () => {
      createSession("session-1");

      const state = useStore.getState();
      const result1 = selectSessionState(state, "session-1");

      clearSessionCache("session-1");

      // Same state, but cache was cleared
      const result2 = selectSessionState(state, "session-1");

      // Should be a new reference (cache was cleared)
      expect(result1).not.toBe(result2);

      // But values should be equal
      expect(result1.timeline).toEqual(result2.timeline);
    });

    it("clearAllSessionCaches should invalidate all caches", () => {
      createSession("session-1");
      createSession("session-2");

      const state = useStore.getState();
      const result1a = selectSessionState(state, "session-1");
      const result2a = selectSessionState(state, "session-2");

      clearAllSessionCaches();

      const result1b = selectSessionState(state, "session-1");
      const result2b = selectSessionState(state, "session-2");

      expect(result1a).not.toBe(result1b);
      expect(result2a).not.toBe(result2b);
    });
  });
});
