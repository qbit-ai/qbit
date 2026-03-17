import { act, render, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

// Use vi.hoisted to ensure mock functions are available during vi.mock hoisting
const {
  mockWrite,
  mockOnData,
  mockFocus,
  mockDispose,
  mockClear,
  mockLoadAddon,
  mockOpen,
  mockOnResize,
  mockOnDataDispose,
  mockFit,
} = vi.hoisted(() => ({
  mockWrite: vi.fn(),
  mockOnData: vi.fn(),
  mockFocus: vi.fn(),
  mockDispose: vi.fn(),
  mockClear: vi.fn(),
  mockLoadAddon: vi.fn(),
  mockOpen: vi.fn(),
  mockOnResize: vi.fn(() => ({ dispose: vi.fn() })),
  mockOnDataDispose: vi.fn(),
  mockFit: vi.fn(),
}));

// vi.mock is hoisted, so we define classes inline in the factory
vi.mock("@xterm/xterm", () => {
  return {
    Terminal: class {
      write = mockWrite;
      onData = mockOnData;
      onResize = mockOnResize;
      focus = mockFocus;
      dispose = mockDispose;
      clear = mockClear;
      loadAddon = mockLoadAddon;
      open = mockOpen;
      rows = 24;
      cols = 80;
    },
  };
});

vi.mock("@xterm/addon-fit", () => {
  return {
    FitAddon: class {
      fit = mockFit;
    },
  };
});

vi.mock("@xterm/addon-web-links", () => {
  return {
    WebLinksAddon: class {},
  };
});

vi.mock("@xterm/addon-webgl", () => {
  return {
    WebglAddon: class {},
  };
});

// Event listener storage for mocking
type EventCallback<T = unknown> = (event: { payload: T }) => void;
type UnlistenFn = () => void;

interface EventListener<T = unknown> {
  eventName: string;
  callback: EventCallback<T>;
}

const mockListeners: EventListener[] = [];

function mockListen<T>(eventName: string, callback: EventCallback<T>): Promise<UnlistenFn> {
  const listener: EventListener<T> = { eventName, callback };
  mockListeners.push(listener as EventListener);
  return Promise.resolve(() => {
    const index = mockListeners.indexOf(listener as EventListener);
    if (index > -1) {
      mockListeners.splice(index, 1);
    }
  });
}

function emitMockEvent<T>(eventName: string, payload: T): void {
  for (const listener of mockListeners) {
    if (listener.eventName === eventName) {
      listener.callback({ payload });
    }
  }
}

function clearMockListeners(): void {
  mockListeners.length = 0;
}

function getListenerCount(eventName: string): number {
  return mockListeners.filter((l) => l.eventName === eventName).length;
}

// Mock Tauri API
vi.mock("@tauri-apps/api/event", () => ({
  listen: (eventName: string, callback: EventCallback) => mockListen(eventName, callback),
}));

// Mock Tauri commands
const mockPtyWrite = vi.fn().mockResolvedValue(undefined);
const mockPtyResize = vi.fn().mockResolvedValue(undefined);

vi.mock("../../lib/tauri", () => ({
  ptyWrite: (...args: unknown[]) => mockPtyWrite(...args),
  ptyResize: (...args: unknown[]) => mockPtyResize(...args),
}));

// Mock TerminalInstanceManager
const mockManagerGet = vi.fn();
const mockManagerRegister = vi.fn();
const mockManagerAttach = vi.fn();
const mockManagerDetach = vi.fn();
const mockManagerDispose = vi.fn();

vi.mock("@/lib/terminal/TerminalInstanceManager", () => ({
  TerminalInstanceManager: {
    get: (...args: unknown[]) => mockManagerGet(...args),
    register: (...args: unknown[]) => mockManagerRegister(...args),
    attachToContainer: (...args: unknown[]) => mockManagerAttach(...args),
    detach: (...args: unknown[]) => mockManagerDetach(...args),
    dispose: (...args: unknown[]) => mockManagerDispose(...args),
  },
}));

// Import component after mocks are set up
import { Terminal } from "./Terminal";

describe("Terminal", () => {
  const sessionId = "test-session-123";

  beforeEach(() => {
    vi.clearAllMocks();
    clearMockListeners();
    // Reset onData mock to capture callbacks and return a disposable (like real xterm.js)
    mockOnData.mockImplementation(() => ({ dispose: mockOnDataDispose }));
    // By default, manager.get() returns undefined (new terminal)
    mockManagerGet.mockReturnValue(undefined);
  });

  afterEach(() => {
    clearMockListeners();
  });

  describe("initialization", () => {
    it("should render without crashing", () => {
      const { container } = render(<Terminal sessionId={sessionId} />);
      expect(container.firstChild).toBeDefined();
    });

    it("should set up terminal output listener", async () => {
      render(<Terminal sessionId={sessionId} />);

      await waitFor(() => {
        expect(getListenerCount("terminal_output")).toBe(1);
      });
    });

    it("should NOT call ptyResize when container has no dimensions (hidden)", async () => {
      // In jsdom, containers have 0 dimensions by default (like when hidden in timeline mode)
      // We intentionally skip ptyResize in this case to avoid sending wrong dimensions to PTY
      render(<Terminal sessionId={sessionId} />);

      // Wait for listeners to be set up
      await waitFor(() => {
        expect(getListenerCount("terminal_output")).toBe(1);
      });

      // ptyResize should NOT have been called since container has no dimensions
      expect(mockPtyResize).not.toHaveBeenCalled();
    });

    it("should focus terminal after setup", async () => {
      render(<Terminal sessionId={sessionId} />);

      await waitFor(() => {
        expect(mockFocus).toHaveBeenCalled();
      });
    });
  });

  describe("race condition prevention", () => {
    it("should enable user input only after listeners are attached", async () => {
      render(<Terminal sessionId={sessionId} />);

      // Wait for setup to complete
      await waitFor(() => {
        expect(mockOnData).toHaveBeenCalled();
      });

      // onData should be called after listeners are set up
      // Verify the order by checking that listeners exist when onData is registered
      expect(getListenerCount("terminal_output")).toBeGreaterThanOrEqual(1);
    });

    it("should write terminal output for matching session only", async () => {
      render(<Terminal sessionId={sessionId} />);

      await waitFor(() => {
        expect(getListenerCount("terminal_output")).toBe(1);
      });

      // Emit output for matching session
      act(() => {
        emitMockEvent("terminal_output", {
          session_id: sessionId,
          data: "hello world",
        });
      });

      expect(mockWrite).toHaveBeenCalledWith("hello world");

      // Emit output for different session
      mockWrite.mockClear();
      act(() => {
        emitMockEvent("terminal_output", {
          session_id: "different-session",
          data: "should not write",
        });
      });

      expect(mockWrite).not.toHaveBeenCalledWith("should not write");
    });

    describe("reattachment grace period", () => {
      // Store captured ResizeObserver callback to trigger resizes manually
      let resizeObserverCallback: (() => void) | null = null;

      // Save originals so we can restore only what we touch (avoids vi.unstubAllGlobals()
      // which would also tear down globals installed by setup.ts, e.g. crypto.randomUUID).
      let originalResizeObserver: typeof ResizeObserver;
      let originalRAF: typeof requestAnimationFrame;
      let originalCAF: typeof cancelAnimationFrame;

      beforeEach(() => {
        originalResizeObserver = globalThis.ResizeObserver;
        originalRAF = globalThis.requestAnimationFrame;
        originalCAF = globalThis.cancelAnimationFrame;

        // Override ResizeObserver to capture callback and fire synchronously.
        vi.stubGlobal(
          "ResizeObserver",
          class {
            callback: () => void;
            constructor(cb: () => void) {
              this.callback = cb;
              resizeObserverCallback = cb;
            }
            observe = () => {
              // Fire synchronously on observe (simulates real behavior)
              this.callback();
            };
            unobserve = vi.fn();
            disconnect = vi.fn();
          }
        );

        // Stub RAF/CAF so tests that await animation frames work reliably in jsdom.
        // Uses setTimeout(0) for async scheduling matching real RAF semantics.
        // A Map tracks rafId -> timeout handle so cancelAnimationFrame correctly
        // cancels the underlying setTimeout (not a mismatched numeric ID).
        let rafId = 0;
        const rafMap = new Map<number, ReturnType<typeof setTimeout>>();
        vi.stubGlobal("requestAnimationFrame", (cb: FrameRequestCallback) => {
          const id = ++rafId;
          rafMap.set(
            id,
            setTimeout(() => {
              rafMap.delete(id);
              cb(performance.now());
            }, 0)
          );
          return id;
        });
        vi.stubGlobal("cancelAnimationFrame", (id: number) => {
          clearTimeout(rafMap.get(id));
          rafMap.delete(id);
        });
      });

      afterEach(() => {
        resizeObserverCallback = null;
        // Restore only the globals we overrode — leave everything else (e.g.
        // crypto.randomUUID from setup.ts) untouched.
        globalThis.ResizeObserver = originalResizeObserver;
        globalThis.requestAnimationFrame = originalRAF;
        globalThis.cancelAnimationFrame = originalCAF;
      });

      it("should skip fit() during reattachment grace period", async () => {
        // Mock an existing terminal instance (reattachment scenario)
        const mockTerminal = {
          write: mockWrite,
          onData: mockOnData,
          onResize: mockOnResize,
          focus: mockFocus,
          dispose: mockDispose,
          clear: mockClear,
          loadAddon: mockLoadAddon,
          open: mockOpen,
          rows: 24,
          cols: 80,
          element: document.createElement("div"),
        };
        const mockFitAddonInstance = { fit: mockFit };

        // Return existing instance to trigger reattachment path
        mockManagerGet.mockReturnValue({
          terminal: mockTerminal,
          fitAddon: mockFitAddonInstance,
        });

        render(<Terminal sessionId={sessionId} />);

        // Wait for setup
        await waitFor(() => {
          expect(mockManagerAttach).toHaveBeenCalled();
        });

        // On reattachment, ResizeObserver fires synchronously, but fit() should be skipped
        // during the grace period. The initial fit() call during setup is normal.
        const fitCallsAfterSetup = mockFit.mock.calls.length;

        // Simulate another resize during the grace period
        act(() => {
          resizeObserverCallback?.();
        });

        // fit() should NOT have been called again during grace period
        expect(mockFit.mock.calls.length).toBe(fitCallsAfterSetup);
      });

      it("should call fit() after grace period ends", async () => {
        // Mock an existing terminal instance (reattachment scenario)
        const mockTerminal = {
          write: mockWrite,
          onData: mockOnData,
          onResize: mockOnResize,
          focus: mockFocus,
          dispose: mockDispose,
          clear: mockClear,
          loadAddon: mockLoadAddon,
          open: mockOpen,
          rows: 24,
          cols: 80,
          element: document.createElement("div"),
        };
        const mockFitAddonInstance = { fit: mockFit };

        mockManagerGet.mockReturnValue({
          terminal: mockTerminal,
          fitAddon: mockFitAddonInstance,
        });

        render(<Terminal sessionId={sessionId} />);

        // Wait for reattachment
        await waitFor(() => {
          expect(mockManagerAttach).toHaveBeenCalled();
        });

        // Wait for double RAF to complete (grace period ends, deferred fit() called)
        await act(async () => {
          // First RAF
          await new Promise((resolve) => requestAnimationFrame(resolve));
          // Second RAF (where grace period ends and fit() is called)
          await new Promise((resolve) => requestAnimationFrame(resolve));
        });

        // After grace period, the deferred fit() should have been called
        expect(mockFit).toHaveBeenCalled();
      });
    });
  });

  describe("user input handling", () => {
    it("should send user input to PTY via ptyWrite", async () => {
      render(<Terminal sessionId={sessionId} />);

      // Wait for setup and capture onData callback
      let dataCallback: ((data: string) => void) | null = null;
      await waitFor(() => {
        expect(mockOnData).toHaveBeenCalled();
        dataCallback = mockOnData.mock.calls[0][0];
      });

      // Simulate user typing
      act(() => {
        dataCallback?.("test input");
      });

      expect(mockPtyWrite).toHaveBeenCalledWith(sessionId, "test input");
    });

    it("should send each keystroke to PTY", async () => {
      render(<Terminal sessionId={sessionId} />);

      let dataCallback: ((data: string) => void) | null = null;
      await waitFor(() => {
        expect(mockOnData).toHaveBeenCalled();
        dataCallback = mockOnData.mock.calls[0][0];
      });

      // Simulate typing "ls" followed by enter
      act(() => {
        dataCallback?.("l");
        dataCallback?.("s");
        dataCallback?.("\r");
      });

      expect(mockPtyWrite).toHaveBeenCalledWith(sessionId, "l");
      expect(mockPtyWrite).toHaveBeenCalledWith(sessionId, "s");
      expect(mockPtyWrite).toHaveBeenCalledWith(sessionId, "\r");
    });
  });

  describe("cleanup", () => {
    it("should unregister listeners on unmount", async () => {
      const { unmount } = render(<Terminal sessionId={sessionId} />);

      await waitFor(() => {
        expect(getListenerCount("terminal_output")).toBe(1);
      });

      unmount();

      // Allow async cleanup to complete
      await new Promise((resolve) => setTimeout(resolve, 10));

      expect(getListenerCount("terminal_output")).toBe(0);
    });

    it("should detach terminal (not dispose) on unmount", async () => {
      const { unmount } = render(<Terminal sessionId={sessionId} />);

      await waitFor(() => {
        expect(mockFocus).toHaveBeenCalled();
      });

      unmount();

      // Terminal should be detached (not disposed) - manager handles lifecycle
      expect(mockManagerDetach).toHaveBeenCalledWith(sessionId);
      // Terminal dispose should NOT be called - manager owns the instance
      expect(mockDispose).not.toHaveBeenCalled();
    });
  });

  describe("duplicate prevention (StrictMode)", () => {
    it("should not create duplicate terminals on re-render", async () => {
      const { rerender } = render(<Terminal sessionId={sessionId} />);

      await waitFor(() => {
        expect(mockOpen).toHaveBeenCalledTimes(1);
      });

      // Re-render with same session
      rerender(<Terminal sessionId={sessionId} />);
      rerender(<Terminal sessionId={sessionId} />);

      // Should still only have one terminal opened
      expect(mockOpen).toHaveBeenCalledTimes(1);
    });

    it("should focus existing terminal when effect re-runs with existing terminal", async () => {
      // This test verifies the early return path in the effect
      // When terminalRef.current already exists, it just focuses and returns
      const { rerender } = render(<Terminal sessionId={sessionId} />);

      await waitFor(() => {
        expect(mockFocus).toHaveBeenCalled();
      });

      // After initial setup, focus should have been called once (at end of setup)
      const initialFocusCalls = mockFocus.mock.calls.length;
      expect(initialFocusCalls).toBe(1);

      // Re-render doesn't trigger the effect again since sessionId is the same
      // This is the expected React behavior for effects with stable dependencies
      rerender(<Terminal sessionId={sessionId} />);

      // Focus count should remain the same
      expect(mockFocus.mock.calls.length).toBe(initialFocusCalls);
    });
  });

  describe("output sequence handling", () => {
    it("should write outputs in correct order", async () => {
      render(<Terminal sessionId={sessionId} />);

      await waitFor(() => {
        expect(getListenerCount("terminal_output")).toBe(1);
      });

      // Emit multiple outputs in sequence
      act(() => {
        emitMockEvent("terminal_output", { session_id: sessionId, data: "first" });
        emitMockEvent("terminal_output", { session_id: sessionId, data: "second" });
        emitMockEvent("terminal_output", { session_id: sessionId, data: "third" });
      });

      // Verify order
      const writeCalls = mockWrite.mock.calls.map((call) => call[0]);
      const firstIndex = writeCalls.indexOf("first");
      const secondIndex = writeCalls.indexOf("second");
      const thirdIndex = writeCalls.indexOf("third");

      expect(firstIndex).toBeLessThan(secondIndex);
      expect(secondIndex).toBeLessThan(thirdIndex);
    });

    it("should handle rapid keystroke echo correctly", async () => {
      render(<Terminal sessionId={sessionId} />);

      let dataCallback: ((data: string) => void) | null = null;
      await waitFor(() => {
        expect(mockOnData).toHaveBeenCalled();
        dataCallback = mockOnData.mock.calls[0][0];
      });

      // Simulate rapid typing and echo
      act(() => {
        // User types 'w'
        dataCallback?.("w");
        // Echo comes back
        emitMockEvent("terminal_output", { session_id: sessionId, data: "w" });
        // User types 'd'
        dataCallback?.("d");
        // Echo comes back
        emitMockEvent("terminal_output", { session_id: sessionId, data: "d" });
      });

      // Verify all keystrokes were sent
      expect(mockPtyWrite).toHaveBeenCalledWith(sessionId, "w");
      expect(mockPtyWrite).toHaveBeenCalledWith(sessionId, "d");

      // Verify all echoes were written
      expect(mockWrite).toHaveBeenCalledWith("w");
      expect(mockWrite).toHaveBeenCalledWith("d");
    });
  });
});
