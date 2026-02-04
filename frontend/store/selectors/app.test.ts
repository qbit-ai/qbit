/**
 * Tests for App.tsx optimized selectors
 *
 * Verifies that the useAppState selector:
 * 1. Only returns the data App.tsx needs (not entire sessions/tabLayouts)
 * 2. Returns stable references when irrelevant state changes
 * 3. Re-renders only when relevant data changes
 */

import { beforeEach, describe, expect, it } from "vitest";
import { useStore } from "../index";
import { clearAppStateCache, selectAppState } from "./app";

describe("App selectors", () => {
  beforeEach(() => {
    // Reset store state
    useStore.setState({
      sessions: {},
      tabLayouts: {},
      activeSessionId: null,
      homeTabId: null,
      pendingCommand: {},
      isAgentResponding: {},
      tabHasNewActivity: {},
      notifications: [],
      notificationsExpanded: false,
    });
    // Clear selector cache
    clearAppStateCache();
  });

  describe("selectAppState", () => {
    it("should return null values when no sessions exist", () => {
      const state = useStore.getState();
      const appState = selectAppState(state);

      expect(appState.activeSessionId).toBeNull();
      expect(appState.focusedWorkingDirectory).toBeUndefined();
      expect(appState.tabLayouts).toEqual([]);
    });

    it("should return active session ID and tab layouts", () => {
      const sessionId = "test-session-1";
      const paneId = "pane-1";

      useStore.setState({
        activeSessionId: sessionId,
        sessions: {
          [sessionId]: {
            id: sessionId,
            name: "Test Session",
            workingDirectory: "/test/path",
            createdAt: new Date().toISOString(),
            mode: "terminal",
          },
        },
        tabLayouts: {
          [sessionId]: {
            focusedPaneId: paneId,
            root: {
              type: "leaf",
              id: paneId,
              sessionId: sessionId,
            },
          },
        },
      });

      const state = useStore.getState();
      const appState = selectAppState(state);

      expect(appState.activeSessionId).toBe(sessionId);
      expect(appState.tabLayouts).toHaveLength(1);
      expect(appState.tabLayouts[0].tabId).toBe(sessionId);
      expect(appState.tabLayouts[0].root.type).toBe("leaf");
    });

    it("should return focused session working directory", () => {
      const sessionId = "test-session-1";
      const paneId = "pane-1";
      const workingDir = "/test/workspace";

      useStore.setState({
        activeSessionId: sessionId,
        sessions: {
          [sessionId]: {
            id: sessionId,
            name: "Test Session",
            workingDirectory: workingDir,
            createdAt: new Date().toISOString(),
            mode: "terminal",
          },
        },
        tabLayouts: {
          [sessionId]: {
            focusedPaneId: paneId,
            root: {
              type: "leaf",
              id: paneId,
              sessionId: sessionId,
            },
          },
        },
      });

      const state = useStore.getState();
      const appState = selectAppState(state);

      expect(appState.focusedWorkingDirectory).toBe(workingDir);
    });

    it("should return stable reference when irrelevant session data changes", () => {
      const sessionId = "test-session-1";
      const paneId = "pane-1";

      useStore.setState({
        activeSessionId: sessionId,
        sessions: {
          [sessionId]: {
            id: sessionId,
            name: "Test Session",
            workingDirectory: "/test/path",
            createdAt: new Date().toISOString(),
            mode: "terminal",
          },
        },
        tabLayouts: {
          [sessionId]: {
            focusedPaneId: paneId,
            root: {
              type: "leaf",
              id: paneId,
              sessionId: sessionId,
            },
          },
        },
      });

      const state1 = useStore.getState();
      const appState1 = selectAppState(state1);

      // Change irrelevant session data (e.g., processName, gitBranch)
      useStore.setState({
        sessions: {
          [sessionId]: {
            ...useStore.getState().sessions[sessionId],
            processName: "zsh", // This shouldn't cause a new reference
            gitBranch: "main", // This shouldn't cause a new reference
          },
        },
      });

      const state2 = useStore.getState();
      const appState2 = selectAppState(state2);

      // Should return the same reference since workingDirectory didn't change
      expect(appState1).toBe(appState2);
    });

    it("should return new reference when workingDirectory changes", () => {
      const sessionId = "test-session-1";
      const paneId = "pane-1";

      useStore.setState({
        activeSessionId: sessionId,
        sessions: {
          [sessionId]: {
            id: sessionId,
            name: "Test Session",
            workingDirectory: "/test/path",
            createdAt: new Date().toISOString(),
            mode: "terminal",
          },
        },
        tabLayouts: {
          [sessionId]: {
            focusedPaneId: paneId,
            root: {
              type: "leaf",
              id: paneId,
              sessionId: sessionId,
            },
          },
        },
      });

      const state1 = useStore.getState();
      const appState1 = selectAppState(state1);

      // Change workingDirectory - this SHOULD cause a new reference
      useStore.setState({
        sessions: {
          [sessionId]: {
            ...useStore.getState().sessions[sessionId],
            workingDirectory: "/different/path",
          },
        },
      });

      const state2 = useStore.getState();
      const appState2 = selectAppState(state2);

      // Should return a different reference
      expect(appState1).not.toBe(appState2);
      expect(appState2.focusedWorkingDirectory).toBe("/different/path");
    });

    it("should return new reference when tabLayout root changes", () => {
      const sessionId = "test-session-1";
      const paneId = "pane-1";

      useStore.setState({
        activeSessionId: sessionId,
        sessions: {
          [sessionId]: {
            id: sessionId,
            name: "Test Session",
            workingDirectory: "/test/path",
            createdAt: new Date().toISOString(),
            mode: "terminal",
          },
        },
        tabLayouts: {
          [sessionId]: {
            focusedPaneId: paneId,
            root: {
              type: "leaf",
              id: paneId,
              sessionId: sessionId,
            },
          },
        },
      });

      const state1 = useStore.getState();
      const appState1 = selectAppState(state1);

      // Change tabLayout root (e.g., split pane)
      const newPaneId = "pane-2";
      const newSessionId = "test-session-2";
      useStore.setState({
        sessions: {
          ...useStore.getState().sessions,
          [newSessionId]: {
            id: newSessionId,
            name: "Test Session 2",
            workingDirectory: "/test/path",
            createdAt: new Date().toISOString(),
            mode: "terminal",
          },
        },
        tabLayouts: {
          [sessionId]: {
            focusedPaneId: paneId,
            root: {
              type: "split",
              id: "split-1",
              direction: "horizontal",
              children: [
                { type: "leaf", id: paneId, sessionId: sessionId },
                { type: "leaf", id: newPaneId, sessionId: newSessionId },
              ],
              ratios: [0.5, 0.5],
            },
          },
        },
      });

      const state2 = useStore.getState();
      const appState2 = selectAppState(state2);

      // Should return a different reference since root changed
      expect(appState1).not.toBe(appState2);
    });

    it("should not include data that App.tsx does not need", () => {
      const sessionId = "test-session-1";
      const paneId = "pane-1";

      useStore.setState({
        activeSessionId: sessionId,
        sessions: {
          [sessionId]: {
            id: sessionId,
            name: "Test Session",
            workingDirectory: "/test/path",
            createdAt: new Date().toISOString(),
            mode: "terminal",
            inputMode: "agent",
            renderMode: "timeline",
            processName: "vim",
            gitBranch: "main",
            aiConfig: {
              provider: "anthropic",
              model: "claude-3-opus",
              status: "ready",
            },
          },
        },
        tabLayouts: {
          [sessionId]: {
            focusedPaneId: paneId,
            root: {
              type: "leaf",
              id: paneId,
              sessionId: sessionId,
            },
          },
        },
      });

      const state = useStore.getState();
      const appState = selectAppState(state);

      // AppState should only have these properties
      const keys = Object.keys(appState);
      expect(keys).toEqual(["activeSessionId", "focusedWorkingDirectory", "tabLayouts"]);

      // TabLayoutInfo should only have tabId and root
      const tabLayoutKeys = Object.keys(appState.tabLayouts[0]);
      expect(tabLayoutKeys).toEqual(["tabId", "root"]);
    });
  });
});
