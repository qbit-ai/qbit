import { beforeEach, describe, expect, it } from "vitest";
import { create } from "zustand";
import { immer } from "zustand/middleware/immer";
import {
  type ContextSlice,
  createContextSlice,
  selectCompactionCount,
  selectCompactionError,
  selectContextMetrics,
  selectIsCompacting,
  selectIsSessionDead,
  selectSessionTokenUsage,
} from "./context";

describe("Context Slice", () => {
  // Create a test store with just the context slice
  const createTestStore = () =>
    create<ContextSlice>()(immer((set, get) => createContextSlice(set, get)));

  let store: ReturnType<typeof createTestStore>;

  beforeEach(() => {
    store = createTestStore();
  });

  describe("initial state", () => {
    it("should have empty sessionTokenUsage", () => {
      expect(store.getState().sessionTokenUsage).toEqual({});
    });

    it("should have empty contextMetrics", () => {
      expect(store.getState().contextMetrics).toEqual({});
    });

    it("should have empty compaction state", () => {
      expect(store.getState().compactionCount).toEqual({});
      expect(store.getState().isCompacting).toEqual({});
      expect(store.getState().isSessionDead).toEqual({});
      expect(store.getState().compactionError).toEqual({});
    });
  });

  describe("initContextState", () => {
    it("should initialize context state for a session", () => {
      store.getState().initContextState("session-1");

      const metrics = store.getState().contextMetrics["session-1"];
      expect(metrics).toEqual({
        utilization: 0,
        usedTokens: 0,
        maxTokens: 0,
        isWarning: false,
      });
      expect(store.getState().compactionCount["session-1"]).toBe(0);
      expect(store.getState().isCompacting["session-1"]).toBe(false);
      expect(store.getState().isSessionDead["session-1"]).toBe(false);
      expect(store.getState().compactionError["session-1"]).toBeNull();
    });

    it("should initialize multiple sessions independently", () => {
      store.getState().initContextState("session-1");
      store.getState().initContextState("session-2");

      expect(Object.keys(store.getState().contextMetrics)).toHaveLength(2);
      expect(store.getState().contextMetrics["session-1"].utilization).toBe(0);
      expect(store.getState().contextMetrics["session-2"].utilization).toBe(0);
    });
  });

  describe("setContextMetrics", () => {
    it("should set context metrics for a session", () => {
      store.getState().setContextMetrics("session-1", {
        utilization: 0.75,
        usedTokens: 75000,
        maxTokens: 100000,
        isWarning: true,
      });

      const metrics = store.getState().contextMetrics["session-1"];
      expect(metrics.utilization).toBe(0.75);
      expect(metrics.usedTokens).toBe(75000);
      expect(metrics.maxTokens).toBe(100000);
      expect(metrics.isWarning).toBe(true);
    });

    it("should allow partial updates", () => {
      store.getState().initContextState("session-1");
      store.getState().setContextMetrics("session-1", { utilization: 0.5 });

      const metrics = store.getState().contextMetrics["session-1"];
      expect(metrics.utilization).toBe(0.5);
      expect(metrics.usedTokens).toBe(0); // Default unchanged
    });

    it("should not affect other sessions", () => {
      store.getState().initContextState("session-1");
      store.getState().initContextState("session-2");
      store.getState().setContextMetrics("session-1", { utilization: 0.8 });

      expect(store.getState().contextMetrics["session-1"].utilization).toBe(0.8);
      expect(store.getState().contextMetrics["session-2"].utilization).toBe(0);
    });
  });

  describe("setCompacting", () => {
    it("should set compacting state to true", () => {
      store.getState().setCompacting("session-1", true);
      expect(store.getState().isCompacting["session-1"]).toBe(true);
    });

    it("should set compacting state to false", () => {
      store.getState().setCompacting("session-1", true);
      store.getState().setCompacting("session-1", false);
      expect(store.getState().isCompacting["session-1"]).toBe(false);
    });

    it("should not affect other sessions", () => {
      store.getState().initContextState("session-1");
      store.getState().initContextState("session-2");
      store.getState().setCompacting("session-1", true);

      expect(store.getState().isCompacting["session-1"]).toBe(true);
      expect(store.getState().isCompacting["session-2"]).toBe(false);
    });
  });

  describe("handleCompactionSuccess", () => {
    it("should increment compaction count", () => {
      store.getState().initContextState("session-1");
      store.getState().handleCompactionSuccess("session-1");

      expect(store.getState().compactionCount["session-1"]).toBe(1);
    });

    it("should increment count on multiple successes", () => {
      store.getState().initContextState("session-1");
      store.getState().handleCompactionSuccess("session-1");
      store.getState().handleCompactionSuccess("session-1");
      store.getState().handleCompactionSuccess("session-1");

      expect(store.getState().compactionCount["session-1"]).toBe(3);
    });

    it("should clear compacting state", () => {
      store.getState().setCompacting("session-1", true);
      store.getState().handleCompactionSuccess("session-1");

      expect(store.getState().isCompacting["session-1"]).toBe(false);
    });

    it("should clear compaction error", () => {
      store.getState().handleCompactionFailed("session-1", "Previous error");
      store.getState().handleCompactionSuccess("session-1");

      expect(store.getState().compactionError["session-1"]).toBeNull();
    });

    it("should clear session dead state", () => {
      store.getState().setSessionDead("session-1", true);
      store.getState().handleCompactionSuccess("session-1");

      expect(store.getState().isSessionDead["session-1"]).toBe(false);
    });
  });

  describe("handleCompactionFailed", () => {
    it("should set compaction error", () => {
      store.getState().handleCompactionFailed("session-1", "Out of memory");

      expect(store.getState().compactionError["session-1"]).toBe("Out of memory");
    });

    it("should clear compacting state", () => {
      store.getState().setCompacting("session-1", true);
      store.getState().handleCompactionFailed("session-1", "Error");

      expect(store.getState().isCompacting["session-1"]).toBe(false);
    });

    it("should not modify session dead state (left for event handler)", () => {
      store.getState().initContextState("session-1");
      store.getState().handleCompactionFailed("session-1", "Error");

      expect(store.getState().isSessionDead["session-1"]).toBe(false);
    });
  });

  describe("clearCompactionError", () => {
    it("should clear compaction error", () => {
      store.getState().handleCompactionFailed("session-1", "Error");
      store.getState().clearCompactionError("session-1");

      expect(store.getState().compactionError["session-1"]).toBeNull();
    });

    it("should not throw for non-existent session", () => {
      expect(() => {
        store.getState().clearCompactionError("non-existent");
      }).not.toThrow();
    });
  });

  describe("setSessionDead", () => {
    it("should set session dead state to true", () => {
      store.getState().setSessionDead("session-1", true);
      expect(store.getState().isSessionDead["session-1"]).toBe(true);
    });

    it("should set session dead state to false", () => {
      store.getState().setSessionDead("session-1", true);
      store.getState().setSessionDead("session-1", false);
      expect(store.getState().isSessionDead["session-1"]).toBe(false);
    });
  });

  describe("cleanupContextState", () => {
    it("should remove all context state for a session", () => {
      store.getState().initContextState("session-1");
      store.getState().setContextMetrics("session-1", { utilization: 0.5 });
      store.getState().handleCompactionSuccess("session-1");

      store.getState().cleanupContextState("session-1");

      expect(store.getState().contextMetrics["session-1"]).toBeUndefined();
      expect(store.getState().compactionCount["session-1"]).toBeUndefined();
      expect(store.getState().isCompacting["session-1"]).toBeUndefined();
      expect(store.getState().isSessionDead["session-1"]).toBeUndefined();
      expect(store.getState().compactionError["session-1"]).toBeUndefined();
    });

    it("should not affect other sessions", () => {
      store.getState().initContextState("session-1");
      store.getState().initContextState("session-2");

      store.getState().cleanupContextState("session-1");

      expect(store.getState().contextMetrics["session-1"]).toBeUndefined();
      expect(store.getState().contextMetrics["session-2"]).toBeDefined();
    });

    it("should not throw for non-existent session", () => {
      expect(() => {
        store.getState().cleanupContextState("non-existent");
      }).not.toThrow();
    });
  });

  describe("selectors", () => {
    describe("selectContextMetrics", () => {
      it("should return metrics for existing session", () => {
        store.getState().setContextMetrics("session-1", {
          utilization: 0.6,
          usedTokens: 60000,
          maxTokens: 100000,
          isWarning: false,
        });

        const metrics = selectContextMetrics(store.getState(), "session-1");
        expect(metrics.utilization).toBe(0.6);
      });

      it("should return default metrics for non-existent session", () => {
        const metrics = selectContextMetrics(store.getState(), "non-existent");
        expect(metrics).toEqual({
          utilization: 0,
          usedTokens: 0,
          maxTokens: 0,
          isWarning: false,
        });
      });
    });

    describe("selectCompactionCount", () => {
      it("should return count for existing session", () => {
        store.getState().initContextState("session-1");
        store.getState().handleCompactionSuccess("session-1");
        store.getState().handleCompactionSuccess("session-1");

        expect(selectCompactionCount(store.getState(), "session-1")).toBe(2);
      });

      it("should return 0 for non-existent session", () => {
        expect(selectCompactionCount(store.getState(), "non-existent")).toBe(0);
      });
    });

    describe("selectIsCompacting", () => {
      it("should return compacting state for existing session", () => {
        store.getState().setCompacting("session-1", true);
        expect(selectIsCompacting(store.getState(), "session-1")).toBe(true);
      });

      it("should return false for non-existent session", () => {
        expect(selectIsCompacting(store.getState(), "non-existent")).toBe(false);
      });
    });

    describe("selectIsSessionDead", () => {
      it("should return dead state for existing session", () => {
        store.getState().setSessionDead("session-1", true);
        expect(selectIsSessionDead(store.getState(), "session-1")).toBe(true);
      });

      it("should return false for non-existent session", () => {
        expect(selectIsSessionDead(store.getState(), "non-existent")).toBe(false);
      });
    });

    describe("selectCompactionError", () => {
      it("should return error for existing session", () => {
        store.getState().handleCompactionFailed("session-1", "Test error");
        expect(selectCompactionError(store.getState(), "session-1")).toBe("Test error");
      });

      it("should return null for non-existent session", () => {
        expect(selectCompactionError(store.getState(), "non-existent")).toBeNull();
      });
    });

    describe("selectSessionTokenUsage", () => {
      it("should return default for non-existent session", () => {
        const usage = selectSessionTokenUsage(store.getState(), "non-existent");
        expect(usage).toEqual({ input: 0, output: 0 });
      });
    });
  });
});
