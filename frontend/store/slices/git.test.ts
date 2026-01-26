import { beforeEach, describe, expect, it } from "vitest";
import { create } from "zustand";
import { immer } from "zustand/middleware/immer";
import {
  createGitSlice,
  type GitSlice,
  selectGitCommitMessage,
  selectGitStatus,
  selectGitStatusLoading,
} from "./git";

describe("Git Slice", () => {
  // Create a test store with just the git slice
  const createTestStore = () => create<GitSlice>()(immer((set, get) => createGitSlice(set, get)));

  let store: ReturnType<typeof createTestStore>;

  beforeEach(() => {
    store = createTestStore();
  });

  describe("initial state", () => {
    it("should have empty gitStatus object", () => {
      expect(store.getState().gitStatus).toEqual({});
    });

    it("should have empty gitStatusLoading object", () => {
      expect(store.getState().gitStatusLoading).toEqual({});
    });

    it("should have empty gitCommitMessage object", () => {
      expect(store.getState().gitCommitMessage).toEqual({});
    });
  });

  describe("initGitState", () => {
    it("should initialize git state for a session", () => {
      store.getState().initGitState("session-1");

      expect(store.getState().gitStatus["session-1"]).toBeNull();
      expect(store.getState().gitStatusLoading["session-1"]).toBe(true);
      expect(store.getState().gitCommitMessage["session-1"]).toBe("");
    });

    it("should initialize multiple sessions independently", () => {
      store.getState().initGitState("session-1");
      store.getState().initGitState("session-2");

      expect(store.getState().gitStatus["session-1"]).toBeNull();
      expect(store.getState().gitStatus["session-2"]).toBeNull();
      expect(Object.keys(store.getState().gitStatus)).toHaveLength(2);
    });
  });

  describe("setGitStatus", () => {
    const mockStatus = {
      branch: "main",
      ahead: 1,
      behind: 0,
      entries: [],
      insertions: 10,
      deletions: 5,
    };

    it("should set git status for a session", () => {
      store.getState().setGitStatus("session-1", mockStatus);
      expect(store.getState().gitStatus["session-1"]).toEqual(mockStatus);
    });

    it("should allow setting status to null", () => {
      store.getState().setGitStatus("session-1", mockStatus);
      store.getState().setGitStatus("session-1", null);
      expect(store.getState().gitStatus["session-1"]).toBeNull();
    });

    it("should not affect other sessions", () => {
      store.getState().initGitState("session-1");
      store.getState().initGitState("session-2");
      store.getState().setGitStatus("session-1", mockStatus);

      expect(store.getState().gitStatus["session-1"]).toEqual(mockStatus);
      expect(store.getState().gitStatus["session-2"]).toBeNull();
    });
  });

  describe("setGitStatusLoading", () => {
    it("should set loading state to true", () => {
      store.getState().setGitStatusLoading("session-1", true);
      expect(store.getState().gitStatusLoading["session-1"]).toBe(true);
    });

    it("should set loading state to false", () => {
      store.getState().setGitStatusLoading("session-1", true);
      store.getState().setGitStatusLoading("session-1", false);
      expect(store.getState().gitStatusLoading["session-1"]).toBe(false);
    });

    it("should not affect other sessions", () => {
      store.getState().initGitState("session-1");
      store.getState().initGitState("session-2");
      store.getState().setGitStatusLoading("session-1", false);

      expect(store.getState().gitStatusLoading["session-1"]).toBe(false);
      expect(store.getState().gitStatusLoading["session-2"]).toBe(true);
    });
  });

  describe("setGitCommitMessage", () => {
    it("should set commit message for a session", () => {
      store.getState().setGitCommitMessage("session-1", "feat: add new feature");
      expect(store.getState().gitCommitMessage["session-1"]).toBe("feat: add new feature");
    });

    it("should allow setting message to empty string", () => {
      store.getState().setGitCommitMessage("session-1", "test");
      store.getState().setGitCommitMessage("session-1", "");
      expect(store.getState().gitCommitMessage["session-1"]).toBe("");
    });

    it("should not affect other sessions", () => {
      store.getState().initGitState("session-1");
      store.getState().initGitState("session-2");
      store.getState().setGitCommitMessage("session-1", "my message");

      expect(store.getState().gitCommitMessage["session-1"]).toBe("my message");
      expect(store.getState().gitCommitMessage["session-2"]).toBe("");
    });
  });

  describe("cleanupGitState", () => {
    it("should remove all git state for a session", () => {
      store.getState().initGitState("session-1");
      store.getState().setGitStatus("session-1", {
        branch: "main",
        ahead: 0,
        behind: 0,
        entries: [],
        insertions: 0,
        deletions: 0,
      });
      store.getState().setGitCommitMessage("session-1", "test");

      store.getState().cleanupGitState("session-1");

      expect(store.getState().gitStatus["session-1"]).toBeUndefined();
      expect(store.getState().gitStatusLoading["session-1"]).toBeUndefined();
      expect(store.getState().gitCommitMessage["session-1"]).toBeUndefined();
    });

    it("should not affect other sessions", () => {
      store.getState().initGitState("session-1");
      store.getState().initGitState("session-2");

      store.getState().cleanupGitState("session-1");

      expect(store.getState().gitStatus["session-1"]).toBeUndefined();
      expect(store.getState().gitStatus["session-2"]).toBeNull();
    });

    it("should not throw for non-existent session", () => {
      expect(() => {
        store.getState().cleanupGitState("non-existent");
      }).not.toThrow();
    });
  });

  describe("selectors", () => {
    describe("selectGitStatus", () => {
      it("should return status for existing session", () => {
        const mockStatus = {
          branch: "feature",
          ahead: 2,
          behind: 1,
          entries: [],
          insertions: 15,
          deletions: 3,
        };
        store.getState().setGitStatus("session-1", mockStatus);
        expect(selectGitStatus(store.getState(), "session-1")).toEqual(mockStatus);
      });

      it("should return null for non-existent session", () => {
        expect(selectGitStatus(store.getState(), "non-existent")).toBeNull();
      });

      it("should return null for session with null status", () => {
        store.getState().initGitState("session-1");
        expect(selectGitStatus(store.getState(), "session-1")).toBeNull();
      });
    });

    describe("selectGitStatusLoading", () => {
      it("should return loading state for existing session", () => {
        store.getState().initGitState("session-1");
        expect(selectGitStatusLoading(store.getState(), "session-1")).toBe(true);
      });

      it("should return false for non-existent session", () => {
        expect(selectGitStatusLoading(store.getState(), "non-existent")).toBe(false);
      });
    });

    describe("selectGitCommitMessage", () => {
      it("should return message for existing session", () => {
        store.getState().setGitCommitMessage("session-1", "test message");
        expect(selectGitCommitMessage(store.getState(), "session-1")).toBe("test message");
      });

      it("should return empty string for non-existent session", () => {
        expect(selectGitCommitMessage(store.getState(), "non-existent")).toBe("");
      });
    });
  });
});
