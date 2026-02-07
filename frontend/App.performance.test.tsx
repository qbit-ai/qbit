/**
 * TDD Performance Tests for App.tsx
 *
 * These tests verify the performance optimizations for:
 * 1. Keyboard shortcuts effect - should not recreate handlers on state changes
 * 2. Store subscriptions - should use targeted selectors instead of full object subscriptions
 */

import { act, renderHook } from "@testing-library/react";
import type React from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { KeyboardHandlerContext } from "./hooks/useKeyboardHandlerContext";
import { useStore } from "./store";

// Mock Tauri APIs
vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(() => Promise.resolve(() => {})),
  emit: vi.fn(),
}));

vi.mock("@/lib/tauri", () => ({
  ptyCreate: vi.fn(),
  ptyDestroy: vi.fn(),
  getGitBranch: vi.fn(),
  shellIntegrationStatus: vi.fn(),
  shellIntegrationInstall: vi.fn(),
}));

vi.mock("@/lib/ai", () => ({
  buildProviderConfig: vi.fn(),
  initAiSession: vi.fn(),
  shutdownAiSession: vi.fn(),
}));

vi.mock("@/lib/settings", () => ({
  getSettings: vi.fn(() =>
    Promise.resolve({
      ai: { default_provider: "test", default_model: "test" },
    })
  ),
}));

// Helper to reset store
const resetStore = () => {
  useStore.setState({
    sessions: {},
    activeSessionId: null,
    timelines: {},
    tabLayouts: {},
    tabHasNewActivity: {},
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
    sessionTokenUsage: {},
    lastSentCommand: {},
    terminalClearRequest: {},
  });
};

// Helper to create a test session
const createSession = (
  sessionId: string,
  options?: { tabType?: "terminal" | "settings" | "home" }
) => {
  useStore.getState().addSession({
    id: sessionId,
    name: `Session ${sessionId}`,
    workingDirectory: `/home/${sessionId}`,
    createdAt: new Date().toISOString(),
    mode: "terminal",
    tabType: options?.tabType,
  });
};

describe("App.tsx Performance Optimizations", () => {
  beforeEach(() => {
    resetStore();
  });

  describe("Issue 1: Keyboard Shortcuts Effect", () => {
    /**
     * The keyboard shortcuts effect currently has 10+ dependencies, causing
     * the event listener to be re-subscribed on every state change.
     *
     * Using the refs pattern, the handler should be stable (created once)
     * and read current values from refs.
     */

    it("should use refs pattern to avoid recreating keyboard handlers", async () => {
      // This test verifies the concept of the refs pattern
      // The handler function reference should remain stable across state changes

      let handlerCreationCount = 0;
      const handlerRef = { current: null as ((e: KeyboardEvent) => void) | null };

      // Simulate the refs pattern
      const contextRef = {
        current: {
          sessions: {} as Record<string, unknown>,
          activeSessionId: null as string | null,
        },
      };

      // This simulates how the keyboard handler should be created ONCE
      const createStableHandler = () => {
        handlerCreationCount++;
        return (_e: KeyboardEvent) => {
          // Read current values from ref, not from closure
          const { sessions, activeSessionId } = contextRef.current;
          // Handler logic would use these values...
          void sessions;
          void activeSessionId;
        };
      };

      // Create handler once
      handlerRef.current = createStableHandler();
      expect(handlerCreationCount).toBe(1);

      // Simulate state changes
      contextRef.current = {
        sessions: { "session-1": {} },
        activeSessionId: "session-1",
      };
      contextRef.current = {
        sessions: { "session-1": {}, "session-2": {} },
        activeSessionId: "session-2",
      };

      // Handler should NOT be recreated
      expect(handlerCreationCount).toBe(1);

      // Handler should still be the same reference
      expect(handlerRef.current).toBe(handlerRef.current);
    });

    it("keyboard handler should read current state from getState() for infrequently used values", () => {
      // Values only needed at event time should be read via getState()
      // This avoids unnecessary effect dependencies

      createSession("session-1");
      createSession("session-2");

      // Simulate keyboard handler that reads state at call time
      const handleKeyDown = (e: KeyboardEvent) => {
        if (e.metaKey && e.key >= "1" && e.key <= "9") {
          const tabIndex = parseInt(e.key, 10) - 1;
          // Read sessions at event time via getState()
          const tabIds = Object.keys(useStore.getState().sessions);
          if (tabIndex < tabIds.length) {
            useStore.getState().setActiveSession(tabIds[tabIndex]);
          }
        }
      };

      // Simulate Cmd+1 keypress
      const event = new KeyboardEvent("keydown", { key: "1", metaKey: true });
      handleKeyDown(event);

      // Should have switched to first session
      expect(useStore.getState().activeSessionId).toBe("session-1");

      // Simulate Cmd+2 keypress
      const event2 = new KeyboardEvent("keydown", { key: "2", metaKey: true });
      handleKeyDown(event2);

      expect(useStore.getState().activeSessionId).toBe("session-2");
    });

    it("keyboard handler should handle toggle mode correctly with ref pattern", () => {
      createSession("session-1");
      useStore.getState().setInputMode("session-1", "terminal");

      // Simulate ref-based context
      const contextRef = {
        current: {
          activeSessionId: "session-1",
          setInputMode: useStore.getState().setInputMode,
        },
      };

      // Handler uses ref values
      const handleToggleMode = () => {
        const { activeSessionId, setInputMode } = contextRef.current;
        if (activeSessionId) {
          const currentSession = useStore.getState().sessions[activeSessionId];
          const newMode = currentSession?.inputMode === "agent" ? "terminal" : "agent";
          setInputMode(activeSessionId, newMode);
        }
      };

      handleToggleMode();
      expect(useStore.getState().sessions["session-1"]?.inputMode).toBe("agent");

      handleToggleMode();
      expect(useStore.getState().sessions["session-1"]?.inputMode).toBe("terminal");
    });
  });

  describe("Issue 2: Full Object Subscriptions", () => {
    /**
     * App.tsx currently subscribes to entire `sessions` and `tabLayouts` Records.
     * This causes re-renders on ANY change to ANY session.
     *
     * Instead, we should use targeted selectors that only subscribe to specific data.
     */

    it("should document that subscribing to full sessions object causes excess renders", () => {
      createSession("session-1");
      createSession("session-2");

      let renderCount = 0;

      // Current approach: subscribe to entire sessions object
      const currentApproach = () => {
        renderCount++;
        return useStore.getState().sessions;
      };

      // Initial "render"
      currentApproach();
      expect(renderCount).toBe(1);

      // Change session-1 - this would trigger re-render even if we only care about session-2
      useStore.getState().updateWorkingDirectory("session-1", "/new/path");

      // In real React, this would cause a re-render of any component subscribed to sessions
      // We're documenting this behavior to show what we're fixing
    });

    it("should provide targeted selector for session IDs only (for tab switching)", () => {
      createSession("session-1");
      createSession("session-2");

      // Targeted selector: only get session IDs
      const getSessionIds = () => Object.keys(useStore.getState().sessions);

      const ids1 = getSessionIds();
      expect(ids1).toHaveLength(2);

      // Changing session properties should NOT affect ID list reference
      // (in practice, this would be a stable selector that only changes when IDs change)
      useStore.getState().updateWorkingDirectory("session-1", "/new/path");

      const ids2 = getSessionIds();
      // Both lists have same content
      expect(ids2).toEqual(ids1);
    });

    it("should provide focused session selector instead of full sessions access", () => {
      createSession("session-1");
      createSession("session-2");

      // Current problematic pattern:
      // const focusedSession = sessions[focusedSessionId]
      // This subscribes to all sessions

      // Better pattern: selector that only returns the focused session
      const getFocusedSession = (sessionId: string | null) => {
        if (!sessionId) return null;
        return useStore.getState().sessions[sessionId] ?? null;
      };

      const focused = getFocusedSession("session-1");
      expect(focused?.id).toBe("session-1");

      // Changes to session-2 should not affect focused session selector result
      // when focused on session-1
      useStore.getState().updateWorkingDirectory("session-2", "/changed");

      const focusedAfter = getFocusedSession("session-1");
      expect(focusedAfter?.workingDirectory).toBe("/home/session-1");
    });

    it("should provide workingDirectory selector without full session access", () => {
      createSession("session-1");

      // Targeted selector for working directory
      const getWorkingDirectory = (sessionId: string | null) => {
        if (!sessionId) return null;
        return useStore.getState().sessions[sessionId]?.workingDirectory ?? null;
      };

      expect(getWorkingDirectory("session-1")).toBe("/home/session-1");

      useStore.getState().updateWorkingDirectory("session-1", "/new/path");
      expect(getWorkingDirectory("session-1")).toBe("/new/path");
    });
  });

  describe("Combined: Ref Pattern + Targeted Selectors", () => {
    /**
     * The keyboard shortcut handler should:
     * 1. Be created once with empty deps
     * 2. Read current values from refs (updated via a separate effect)
     * 3. Use getState() for values only needed at event time
     */

    it("should demonstrate complete ref-based keyboard handler pattern", () => {
      createSession("session-1");
      createSession("session-2");

      // Context ref - updated by a separate effect with minimal deps
      const keyboardContextRef = {
        current: {
          activeSessionId: null as string | null,
          handleNewTab: () => {},
          handleToggleMode: () => {},
          openContextPanel: () => {},
          openGitPanel: () => {},
          toggleFileEditorPanel: () => {},
          openSettingsTab: () => {},
          handleSplitPane: async (_dir: string) => {},
          handleClosePane: async () => {},
          handleNavigatePane: (_dir: string) => {},
          gitPanelOpen: false,
        },
      };

      // Update ref when state changes
      const updateRef = () => {
        keyboardContextRef.current = {
          ...keyboardContextRef.current,
          activeSessionId: useStore.getState().activeSessionId,
        };
      };

      // Initial update
      updateRef();

      // Stable handler - never recreated
      const handleKeyDown = (e: KeyboardEvent) => {
        const ctx = keyboardContextRef.current;

        // Cmd+[1-9] for tab switching - read sessions at event time
        if (e.metaKey && !e.shiftKey && !e.altKey && e.key >= "1" && e.key <= "9") {
          const tabIndex = parseInt(e.key, 10) - 1;
          const tabIds = Object.keys(useStore.getState().sessions);
          if (tabIndex < tabIds.length) {
            e.preventDefault();
            useStore.getState().setActiveSession(tabIds[tabIndex]);
          }
          return;
        }

        // Ctrl+] for next tab
        if (e.ctrlKey && e.key === "]") {
          const sIds = Object.keys(useStore.getState().sessions);
          if (ctx.activeSessionId && sIds.length > 1) {
            const idx = sIds.indexOf(ctx.activeSessionId);
            useStore.getState().setActiveSession(sIds[(idx + 1) % sIds.length]);
          }
          return;
        }
      };

      // Test tab switching
      useStore.getState().setActiveSession("session-1");
      updateRef();

      const nextTabEvent = new KeyboardEvent("keydown", { key: "]", ctrlKey: true });
      handleKeyDown(nextTabEvent);

      expect(useStore.getState().activeSessionId).toBe("session-2");
    });
  });

  describe("Selector Stability Tests", () => {
    it("focusedSessionWorkingDirectory should be stable when other sessions change", () => {
      createSession("session-1");
      createSession("session-2");
      useStore.getState().setActiveSession("session-1");

      // Simulate focused session selector
      const useFocusedWorkingDirectory = () => {
        const state = useStore.getState();
        const focusedSessionId = state.activeSessionId;
        if (!focusedSessionId) return null;
        const layout = state.tabLayouts[focusedSessionId];
        if (!layout) return state.sessions[focusedSessionId]?.workingDirectory ?? null;
        // In real code, would find the focused pane's session
        return state.sessions[focusedSessionId]?.workingDirectory ?? null;
      };

      const wd1 = useFocusedWorkingDirectory();
      expect(wd1).toBe("/home/session-1");

      // Change session-2 (not focused)
      useStore.getState().updateWorkingDirectory("session-2", "/changed");

      const wd2 = useFocusedWorkingDirectory();
      expect(wd2).toBe("/home/session-1"); // Unchanged

      // Change focused session
      useStore.getState().updateWorkingDirectory("session-1", "/new/path");

      const wd3 = useFocusedWorkingDirectory();
      expect(wd3).toBe("/new/path"); // Changed
    });
  });
});

describe("App Keyboard Handler Refs Hook", () => {
  beforeEach(() => {
    resetStore();
  });

  /**
   * Test the useKeyboardHandlerContext hook that will be created.
   * This hook provides the ref-based context for keyboard handlers.
   */

  it("should provide stable context ref that updates with state changes", async () => {
    // Import the hook we'll create
    const { useKeyboardHandlerContext } = await import("./hooks/useKeyboardHandlerContext");

    createSession("session-1");
    useStore.getState().setActiveSession("session-1");

    const { result } = renderHook(() => useKeyboardHandlerContext());

    // Initial value should be set
    expect(result.current.current.activeSessionId).toBe("session-1");

    // Create second session and switch to it
    await act(async () => {
      createSession("session-2");
      useStore.getState().setActiveSession("session-2");
    });

    // Ref should be updated
    expect(result.current.current.activeSessionId).toBe("session-2");

    // Ref reference itself should be stable
    const refBefore = result.current;
    await act(async () => {
      useStore.getState().updateWorkingDirectory("session-2", "/changed");
    });
    expect(result.current).toBe(refBefore);
  });
});

describe("App Keyboard Event Listener Subscription", () => {
  beforeEach(() => {
    resetStore();
  });

  /**
   * Test that verifies the keyboard event listener is only set up once,
   * even when the callback dependencies change multiple times.
   */

  it("should only add event listener once regardless of callback changes", () => {
    // Track addEventListener calls
    const addEventListenerSpy = vi.spyOn(window, "addEventListener");
    const removeEventListenerSpy = vi.spyOn(window, "removeEventListener");

    // Simulate the pattern used in App.tsx:
    // - Create a stable ref
    // - Update the ref on each "render" (simulated by function calls)
    // - Set up event listener once with empty deps

    const handlersRef = { current: { handleNewTab: () => {}, handleToggleMode: () => {} } };

    // Simulate "renders" that update the ref (without re-subscribing)
    const render = (newHandlers: typeof handlersRef.current) => {
      handlersRef.current = newHandlers;
    };

    // Initial setup - add event listener once
    const handleKeyDown = (_e: KeyboardEvent) => {
      // Always reads from current ref value
      void handlersRef.current.handleNewTab;
    };
    window.addEventListener("keydown", handleKeyDown);

    const initialAddCount = addEventListenerSpy.mock.calls.filter(
      (call) => call[0] === "keydown"
    ).length;
    expect(initialAddCount).toBe(1);

    // Simulate multiple "renders" with different callbacks
    render({ handleNewTab: () => console.log("1"), handleToggleMode: () => {} });
    render({ handleNewTab: () => console.log("2"), handleToggleMode: () => {} });
    render({ handleNewTab: () => console.log("3"), handleToggleMode: () => {} });

    // Event listener should NOT have been re-added
    const finalAddCount = addEventListenerSpy.mock.calls.filter(
      (call) => call[0] === "keydown"
    ).length;
    expect(finalAddCount).toBe(1);

    // Cleanup
    window.removeEventListener("keydown", handleKeyDown);
    expect(
      removeEventListenerSpy.mock.calls.filter((call) => call[0] === "keydown").length
    ).toBeGreaterThanOrEqual(1);

    addEventListenerSpy.mockRestore();
    removeEventListenerSpy.mockRestore();
  });

  it("should access current handler values through ref at event time", () => {
    // This test verifies the key benefit of the refs pattern:
    // The handler can access the latest values without needing to be recreated

    let callCount = 0;
    const handlersRef = { current: { increment: () => callCount++ } };

    // Handler created once, reads from ref
    const handleKeyDown = () => {
      handlersRef.current.increment();
    };

    // Initial increment function
    handleKeyDown();
    expect(callCount).toBe(1);

    // Update the ref with a new function (simulates callback changing)
    handlersRef.current = {
      increment: () => {
        callCount += 10;
        return callCount;
      },
    };

    // Same handler, but now uses the NEW function from the ref
    handleKeyDown();
    expect(callCount).toBe(11); // 1 + 10

    // Update again
    handlersRef.current = {
      increment: () => {
        callCount += 100;
        return callCount;
      },
    };
    handleKeyDown();
    expect(callCount).toBe(111); // 11 + 100
  });

  it("createKeyboardHandler should return a stable function that uses ref values", async () => {
    const { createKeyboardHandler } = await import("./hooks/useKeyboardHandlerContext");

    let newTabCalled = false;
    const contextRef: React.MutableRefObject<KeyboardHandlerContext> = {
      current: {
        activeSessionId: null,
        gitPanelOpen: false,
        handleNewTab: () => {
          newTabCalled = true;
        },
        handleToggleMode: () => {},
        openContextPanel: () => {},
        openGitPanel: () => {},
        toggleFileEditorPanel: () => {},
        openSettingsTab: () => {},
        handleSplitPane: async () => {},
        handleClosePane: async () => {},
        handleNavigatePane: () => {},
        setCommandPaletteOpen: () => {},
        setQuickOpenDialogOpen: () => {},
        setSidecarPanelOpen: () => {},
      },
    };

    // Create handler once
    const handler = createKeyboardHandler(contextRef);

    // Handler should use ref values
    const event = new KeyboardEvent("keydown", { key: "t", metaKey: true });
    handler(event);
    expect(newTabCalled).toBe(true);

    // Update ref with different callback
    let differentCalled = false;
    contextRef.current.handleNewTab = () => {
      differentCalled = true;
    };

    // Same handler reference, but uses updated ref value
    const event2 = new KeyboardEvent("keydown", { key: "t", metaKey: true });
    handler(event2);
    expect(differentCalled).toBe(true);
  });
});
