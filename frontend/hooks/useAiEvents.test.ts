import { act, renderHook } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { useStore } from "../store";
import { clearMockListeners, emitMockEvent, getListenerCount } from "../test/mocks/tauri-event";
import {
  getSessionSequenceCount,
  resetAllSequences,
  resetSessionSequence,
  useAiEvents,
} from "./useAiEvents";

/**
 * Helper to wait for requestAnimationFrame to flush.
 * Text deltas are debounced via requestAnimationFrame, so tests need to wait
 * for the next frame before checking the store.
 */
const waitForAnimationFrame = () =>
  act(async () => {
    await new Promise((resolve) => requestAnimationFrame(resolve));
  });

// Mock the signalFrontendReady function
vi.mock("@/lib/ai", async () => {
  const actual = await vi.importActual("@/lib/ai");
  return {
    ...actual,
    signalFrontendReady: vi.fn().mockResolvedValue(undefined),
  };
});

describe("useAiEvents", () => {
  const createTestSession = (id: string, name = "Test") => {
    useStore.getState().addSession({
      id,
      name,
      workingDirectory: "/test",
      createdAt: new Date().toISOString(),
      mode: "agent",
    });
  };

  beforeEach(() => {
    // Reset store state
    useStore.setState({
      sessions: {},
      activeSessionId: null,
      timelines: {},
      pendingCommand: {},
      agentStreaming: {},
      agentInitialized: {},
      pendingToolApproval: {},
      processedToolRequests: {},
      streamingBlocks: {},
      activeToolCalls: {},
      thinkingContent: {},
      isAgentThinking: {},
      isAgentResponding: {},
    });

    // Clear any existing listeners
    clearMockListeners();

    // Reset sequence tracking
    resetAllSequences();

    // Create a test session
    createTestSession("test-session");
  });

  afterEach(() => {
    clearMockListeners();
    resetAllSequences();
    vi.clearAllMocks();
  });

  it("should register event listeners on mount", async () => {
    renderHook(() => useAiEvents());

    // Wait for async listener setup
    await act(async () => {
      await new Promise((resolve) => setTimeout(resolve, 10));
    });

    // Should have registered listener for ai-event
    expect(getListenerCount("ai-event")).toBe(1);
  });

  it("should unregister listeners on unmount", async () => {
    const { unmount } = renderHook(() => useAiEvents());

    // Wait for async listener setup
    await act(async () => {
      await new Promise((resolve) => setTimeout(resolve, 10));
    });

    expect(getListenerCount("ai-event")).toBe(1);

    unmount();

    // Give promises time to resolve
    await new Promise((resolve) => setTimeout(resolve, 10));

    expect(getListenerCount("ai-event")).toBe(0);
  });

  describe("event deduplication", () => {
    it("skips duplicate events by seq", async () => {
      renderHook(() => useAiEvents());

      // Wait for listener setup
      await act(async () => {
        await new Promise((resolve) => setTimeout(resolve, 10));
      });

      // Send first event with seq 1
      act(() => {
        emitMockEvent("ai-event", {
          type: "started",
          session_id: "test-session",
          turn_id: "turn-1",
          seq: 1,
          ts: "2024-01-01T00:00:00Z",
        });
      });

      // Agent should be thinking
      expect(useStore.getState().isAgentThinking["test-session"]).toBe(true);

      // Reset thinking state to false
      useStore.getState().setAgentThinking("test-session", false);

      // Send duplicate event with same seq (should be skipped)
      act(() => {
        emitMockEvent("ai-event", {
          type: "started",
          session_id: "test-session",
          turn_id: "turn-1",
          seq: 1,
          ts: "2024-01-01T00:00:01Z",
        });
      });

      // Agent should still not be thinking (duplicate was skipped)
      expect(useStore.getState().isAgentThinking["test-session"]).toBe(false);
    });

    it("processes events with incrementing sequence numbers", async () => {
      renderHook(() => useAiEvents());

      // Wait for listener setup
      await act(async () => {
        await new Promise((resolve) => setTimeout(resolve, 10));
      });

      // Send event with seq 1
      act(() => {
        emitMockEvent("ai-event", {
          type: "started",
          session_id: "test-session",
          turn_id: "turn-1",
          seq: 1,
          ts: "2024-01-01T00:00:00Z",
        });
      });

      expect(useStore.getState().isAgentThinking["test-session"]).toBe(true);

      // Send event with seq 2 (should be processed)
      act(() => {
        emitMockEvent("ai-event", {
          type: "text_delta",
          session_id: "test-session",
          delta: "Hello",
          accumulated: "Hello",
          seq: 2,
          ts: "2024-01-01T00:00:01Z",
        });
      });

      // Wait for debounced flush
      await waitForAnimationFrame();

      expect(useStore.getState().agentStreaming["test-session"]).toBe("Hello");
    });

    it("warns on sequence gaps", async () => {
      const warnSpy = vi.spyOn(console, "warn").mockImplementation(() => {});

      renderHook(() => useAiEvents());

      // Wait for listener setup
      await act(async () => {
        await new Promise((resolve) => setTimeout(resolve, 10));
      });

      // Send event with seq 1
      act(() => {
        emitMockEvent("ai-event", {
          type: "started",
          session_id: "test-session",
          turn_id: "turn-1",
          seq: 1,
          ts: "2024-01-01T00:00:00Z",
        });
      });

      // Send event with seq 5 (gap of 3)
      act(() => {
        emitMockEvent("ai-event", {
          type: "text_delta",
          session_id: "test-session",
          delta: "Hello",
          accumulated: "Hello",
          seq: 5,
          ts: "2024-01-01T00:00:01Z",
        });
      });

      expect(warnSpy).toHaveBeenCalledWith(expect.stringContaining("Event sequence gap"));
      expect(warnSpy).toHaveBeenCalledWith(expect.stringContaining("expected 2, got 5"));

      warnSpy.mockRestore();
    });

    it("handles events without seq (backwards compatibility)", async () => {
      renderHook(() => useAiEvents());

      // Wait for listener setup
      await act(async () => {
        await new Promise((resolve) => setTimeout(resolve, 10));
      });

      // Send event without seq (should be processed)
      act(() => {
        emitMockEvent("ai-event", {
          type: "started",
          session_id: "test-session",
          turn_id: "turn-1",
        });
      });

      expect(useStore.getState().isAgentThinking["test-session"]).toBe(true);
    });

    it("tracks sequence per session independently", async () => {
      createTestSession("session-2");

      renderHook(() => useAiEvents());

      // Wait for listener setup
      await act(async () => {
        await new Promise((resolve) => setTimeout(resolve, 10));
      });

      // Send seq 1 to session 1
      act(() => {
        emitMockEvent("ai-event", {
          type: "started",
          session_id: "test-session",
          turn_id: "turn-1",
          seq: 1,
          ts: "2024-01-01T00:00:00Z",
        });
      });

      // Send seq 1 to session 2 (should NOT be skipped - different session)
      act(() => {
        emitMockEvent("ai-event", {
          type: "started",
          session_id: "session-2",
          turn_id: "turn-2",
          seq: 1,
          ts: "2024-01-01T00:00:00Z",
        });
      });

      // Both sessions should be thinking
      expect(useStore.getState().isAgentThinking["test-session"]).toBe(true);
      expect(useStore.getState().isAgentThinking["session-2"]).toBe(true);
    });

    it("skips events with seq <= last seen", async () => {
      renderHook(() => useAiEvents());

      // Wait for listener setup
      await act(async () => {
        await new Promise((resolve) => setTimeout(resolve, 10));
      });

      // Send seq 5
      act(() => {
        emitMockEvent("ai-event", {
          type: "started",
          session_id: "test-session",
          turn_id: "turn-1",
          seq: 5,
          ts: "2024-01-01T00:00:00Z",
        });
      });

      expect(useStore.getState().isAgentThinking["test-session"]).toBe(true);
      useStore.getState().setAgentThinking("test-session", false);

      // Send seq 3 (older than 5, should be skipped)
      act(() => {
        emitMockEvent("ai-event", {
          type: "started",
          session_id: "test-session",
          turn_id: "turn-1",
          seq: 3,
          ts: "2024-01-01T00:00:00Z",
        });
      });

      // Should not have set thinking to true again
      expect(useStore.getState().isAgentThinking["test-session"]).toBe(false);
    });
  });

  describe("basic event handling", () => {
    it("should handle started event", async () => {
      renderHook(() => useAiEvents());

      await act(async () => {
        await new Promise((resolve) => setTimeout(resolve, 10));
      });

      act(() => {
        emitMockEvent("ai-event", {
          type: "started",
          session_id: "test-session",
          turn_id: "turn-1",
        });
      });

      const state = useStore.getState();
      expect(state.isAgentThinking["test-session"]).toBe(true);
      expect(state.isAgentResponding["test-session"]).toBe(true);
    });

    it("should handle text_delta event", async () => {
      renderHook(() => useAiEvents());

      await act(async () => {
        await new Promise((resolve) => setTimeout(resolve, 10));
      });

      act(() => {
        emitMockEvent("ai-event", {
          type: "text_delta",
          session_id: "test-session",
          delta: "Hello",
          accumulated: "Hello",
        });
      });

      // Wait for debounced flush
      await waitForAnimationFrame();

      const state = useStore.getState();
      expect(state.agentStreaming["test-session"]).toBe("Hello");
    });

    it("should handle error event", async () => {
      renderHook(() => useAiEvents());

      await act(async () => {
        await new Promise((resolve) => setTimeout(resolve, 10));
      });

      act(() => {
        emitMockEvent("ai-event", {
          type: "error",
          session_id: "test-session",
          message: "Test error",
          error_type: "test",
        });
      });

      const state = useStore.getState();
      expect(state.isAgentThinking["test-session"]).toBe(false);
      expect(state.isAgentResponding["test-session"]).toBe(false);
    });
  });

  describe("signalFrontendReady", () => {
    it("should signal frontend ready after listener setup", async () => {
      const { signalFrontendReady } = await import("@/lib/ai");

      renderHook(() => useAiEvents());

      // Wait for async listener setup and signalFrontendReady call
      await act(async () => {
        await new Promise((resolve) => setTimeout(resolve, 50));
      });

      // Should have been called for the existing session
      expect(signalFrontendReady).toHaveBeenCalledWith("test-session");
    });
  });

  describe("lastSeenSeq memory management", () => {
    it("should expose getSessionSequenceCount for testing", () => {
      expect(getSessionSequenceCount()).toBe(0);
    });

    it("should track sequences for active sessions", async () => {
      renderHook(() => useAiEvents());

      await act(async () => {
        await new Promise((resolve) => setTimeout(resolve, 10));
      });

      // Send an event to establish sequence tracking
      act(() => {
        emitMockEvent("ai-event", {
          type: "started",
          session_id: "test-session",
          turn_id: "turn-1",
          seq: 1,
          ts: "2024-01-01T00:00:00Z",
        });
      });

      expect(getSessionSequenceCount()).toBe(1);
    });

    it("should clean up session sequence synchronously with resetSessionSequence", async () => {
      renderHook(() => useAiEvents());

      await act(async () => {
        await new Promise((resolve) => setTimeout(resolve, 10));
      });

      // Establish sequences for multiple sessions
      createTestSession("session-2");

      act(() => {
        emitMockEvent("ai-event", {
          type: "started",
          session_id: "test-session",
          turn_id: "turn-1",
          seq: 1,
        });
      });

      act(() => {
        emitMockEvent("ai-event", {
          type: "started",
          session_id: "session-2",
          turn_id: "turn-2",
          seq: 1,
        });
      });

      expect(getSessionSequenceCount()).toBe(2);

      // Synchronously clean up one session
      resetSessionSequence("test-session");

      // Should immediately be cleaned up - no async operation
      expect(getSessionSequenceCount()).toBe(1);
    });

    it("should clean up all sequences synchronously with resetAllSequences", async () => {
      renderHook(() => useAiEvents());

      await act(async () => {
        await new Promise((resolve) => setTimeout(resolve, 10));
      });

      createTestSession("session-2");
      createTestSession("session-3");

      act(() => {
        emitMockEvent("ai-event", {
          type: "started",
          session_id: "test-session",
          turn_id: "turn-1",
          seq: 1,
        });
      });

      act(() => {
        emitMockEvent("ai-event", {
          type: "started",
          session_id: "session-2",
          turn_id: "turn-2",
          seq: 1,
        });
      });

      act(() => {
        emitMockEvent("ai-event", {
          type: "started",
          session_id: "session-3",
          turn_id: "turn-3",
          seq: 1,
        });
      });

      expect(getSessionSequenceCount()).toBe(3);

      // Synchronously clean up all sequences
      resetAllSequences();

      // Should immediately be empty - no async operation
      expect(getSessionSequenceCount()).toBe(0);
    });

    it("should not leak memory when sessions are removed", async () => {
      renderHook(() => useAiEvents());

      await act(async () => {
        await new Promise((resolve) => setTimeout(resolve, 10));
      });

      // Create many sessions and events
      for (let i = 0; i < 10; i++) {
        const sessionId = `session-${i}`;
        createTestSession(sessionId);

        act(() => {
          emitMockEvent("ai-event", {
            type: "started",
            session_id: sessionId,
            turn_id: `turn-${i}`,
            seq: 1,
          });
        });
      }

      expect(getSessionSequenceCount()).toBe(10);

      // Clean up each session
      for (let i = 0; i < 10; i++) {
        resetSessionSequence(`session-${i}`);
      }

      expect(getSessionSequenceCount()).toBe(0);
    });

    it("resetSessionSequence is idempotent", () => {
      // Should not throw when called on non-existent session
      expect(() => resetSessionSequence("non-existent")).not.toThrow();
      expect(getSessionSequenceCount()).toBe(0);
    });
  });
});
