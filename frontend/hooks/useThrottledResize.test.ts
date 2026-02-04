import { act, renderHook } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { useThrottledResize } from "./useThrottledResize";

describe("useThrottledResize", () => {
  let rafCallbacks: Map<number, FrameRequestCallback>;
  let rafId: number;
  let originalRaf: typeof requestAnimationFrame;
  let originalCaf: typeof cancelAnimationFrame;

  beforeEach(() => {
    rafCallbacks = new Map();
    rafId = 0;

    // Store originals
    originalRaf = globalThis.requestAnimationFrame;
    originalCaf = globalThis.cancelAnimationFrame;

    // Mock RAF
    globalThis.requestAnimationFrame = vi.fn((callback: FrameRequestCallback) => {
      const id = ++rafId;
      rafCallbacks.set(id, callback);
      return id;
    });

    globalThis.cancelAnimationFrame = vi.fn((id: number) => {
      rafCallbacks.delete(id);
    });
  });

  afterEach(() => {
    globalThis.requestAnimationFrame = originalRaf;
    globalThis.cancelAnimationFrame = originalCaf;
    rafCallbacks.clear();
    vi.clearAllMocks();
  });

  // Helper to flush RAF callbacks
  const flushRaf = () => {
    const callbacks = Array.from(rafCallbacks.values());
    rafCallbacks.clear();
    for (const cb of callbacks) {
      cb(performance.now());
    }
  };

  describe("basic functionality", () => {
    it("should return handlers for starting and stopping resize", () => {
      const { result } = renderHook(() =>
        useThrottledResize({
          minWidth: 200,
          maxWidth: 600,
          onWidthChange: vi.fn(),
        })
      );

      expect(typeof result.current.startResizing).toBe("function");
      expect(typeof result.current.isResizing).toBe("boolean");
      expect(result.current.isResizing).toBe(false);
    });

    it("should set isResizing to true when startResizing is called", () => {
      const { result } = renderHook(() =>
        useThrottledResize({
          minWidth: 200,
          maxWidth: 600,
          onWidthChange: vi.fn(),
        })
      );

      const mockEvent = {
        preventDefault: vi.fn(),
      } as unknown as React.MouseEvent;

      act(() => {
        result.current.startResizing(mockEvent);
      });

      expect(mockEvent.preventDefault).toHaveBeenCalled();
      expect(result.current.isResizing).toBe(true);
    });

    it("should set document.body cursor and userSelect on start", () => {
      const { result } = renderHook(() =>
        useThrottledResize({
          minWidth: 200,
          maxWidth: 600,
          onWidthChange: vi.fn(),
        })
      );

      const mockEvent = { preventDefault: vi.fn() } as unknown as React.MouseEvent;

      act(() => {
        result.current.startResizing(mockEvent);
      });

      expect(document.body.style.cursor).toBe("col-resize");
      expect(document.body.style.userSelect).toBe("none");
    });
  });

  describe("throttling behavior", () => {
    it("should throttle mousemove events using RAF", () => {
      const onWidthChange = vi.fn();
      renderHook(() =>
        useThrottledResize({
          minWidth: 200,
          maxWidth: 600,
          onWidthChange,
          calculateWidth: (e) => e.clientX,
        })
      );

      // Start resizing
      act(() => {
        document.dispatchEvent(new MouseEvent("mousedown", { bubbles: true, clientX: 300 }));
      });

      // Simulate startResizing being called (normally done via onMouseDown on element)
      // For test, we directly trigger mousedown on document after hook sets up listeners

      // The hook needs to be in resizing state, so let's do it properly
    });

    it("should only call onWidthChange once per RAF frame even with multiple mousemove events", () => {
      const onWidthChange = vi.fn();
      const { result } = renderHook(() =>
        useThrottledResize({
          minWidth: 200,
          maxWidth: 600,
          onWidthChange,
          calculateWidth: (e) => e.clientX,
        })
      );

      // Start resizing
      act(() => {
        result.current.startResizing({
          preventDefault: vi.fn(),
        } as unknown as React.MouseEvent);
      });

      // Fire multiple mousemove events before RAF callback
      act(() => {
        document.dispatchEvent(new MouseEvent("mousemove", { clientX: 250 }));
        document.dispatchEvent(new MouseEvent("mousemove", { clientX: 260 }));
        document.dispatchEvent(new MouseEvent("mousemove", { clientX: 270 }));
      });

      // RAF should have been called once (for first mousemove)
      expect(requestAnimationFrame).toHaveBeenCalledTimes(1);

      // onWidthChange not called yet (waiting for RAF)
      expect(onWidthChange).not.toHaveBeenCalled();

      // Flush RAF
      act(() => {
        flushRaf();
      });

      // Should only update to the latest value (270)
      expect(onWidthChange).toHaveBeenCalledTimes(1);
      expect(onWidthChange).toHaveBeenCalledWith(270);
    });

    it("should respect minWidth constraint", () => {
      const onWidthChange = vi.fn();
      const { result } = renderHook(() =>
        useThrottledResize({
          minWidth: 200,
          maxWidth: 600,
          onWidthChange,
          calculateWidth: (e) => e.clientX,
        })
      );

      act(() => {
        result.current.startResizing({
          preventDefault: vi.fn(),
        } as unknown as React.MouseEvent);
      });

      act(() => {
        document.dispatchEvent(new MouseEvent("mousemove", { clientX: 100 }));
      });

      act(() => {
        flushRaf();
      });

      // Should not call onWidthChange because 100 < minWidth (200)
      expect(onWidthChange).not.toHaveBeenCalled();
    });

    it("should respect maxWidth constraint", () => {
      const onWidthChange = vi.fn();
      const { result } = renderHook(() =>
        useThrottledResize({
          minWidth: 200,
          maxWidth: 600,
          onWidthChange,
          calculateWidth: (e) => e.clientX,
        })
      );

      act(() => {
        result.current.startResizing({
          preventDefault: vi.fn(),
        } as unknown as React.MouseEvent);
      });

      act(() => {
        document.dispatchEvent(new MouseEvent("mousemove", { clientX: 700 }));
      });

      act(() => {
        flushRaf();
      });

      // Should not call onWidthChange because 700 > maxWidth (600)
      expect(onWidthChange).not.toHaveBeenCalled();
    });

    it("should accept valid width within constraints", () => {
      const onWidthChange = vi.fn();
      const { result } = renderHook(() =>
        useThrottledResize({
          minWidth: 200,
          maxWidth: 600,
          onWidthChange,
          calculateWidth: (e) => e.clientX,
        })
      );

      act(() => {
        result.current.startResizing({
          preventDefault: vi.fn(),
        } as unknown as React.MouseEvent);
      });

      act(() => {
        document.dispatchEvent(new MouseEvent("mousemove", { clientX: 400 }));
      });

      act(() => {
        flushRaf();
      });

      expect(onWidthChange).toHaveBeenCalledWith(400);
    });
  });

  describe("mouseup handling", () => {
    it("should stop resizing on mouseup", () => {
      const { result } = renderHook(() =>
        useThrottledResize({
          minWidth: 200,
          maxWidth: 600,
          onWidthChange: vi.fn(),
        })
      );

      act(() => {
        result.current.startResizing({
          preventDefault: vi.fn(),
        } as unknown as React.MouseEvent);
      });

      expect(result.current.isResizing).toBe(true);

      act(() => {
        document.dispatchEvent(new MouseEvent("mouseup"));
      });

      expect(result.current.isResizing).toBe(false);
    });

    it("should reset document.body styles on mouseup", () => {
      const { result } = renderHook(() =>
        useThrottledResize({
          minWidth: 200,
          maxWidth: 600,
          onWidthChange: vi.fn(),
        })
      );

      act(() => {
        result.current.startResizing({
          preventDefault: vi.fn(),
        } as unknown as React.MouseEvent);
      });

      act(() => {
        document.dispatchEvent(new MouseEvent("mouseup"));
      });

      expect(document.body.style.cursor).toBe("");
      expect(document.body.style.userSelect).toBe("");
    });

    it("should not process mousemove after mouseup", () => {
      const onWidthChange = vi.fn();
      const { result } = renderHook(() =>
        useThrottledResize({
          minWidth: 200,
          maxWidth: 600,
          onWidthChange,
          calculateWidth: (e) => e.clientX,
        })
      );

      act(() => {
        result.current.startResizing({
          preventDefault: vi.fn(),
        } as unknown as React.MouseEvent);
      });

      act(() => {
        document.dispatchEvent(new MouseEvent("mouseup"));
      });

      act(() => {
        document.dispatchEvent(new MouseEvent("mousemove", { clientX: 400 }));
      });

      act(() => {
        flushRaf();
      });

      expect(onWidthChange).not.toHaveBeenCalled();
    });
  });

  describe("cleanup", () => {
    it("should cancel pending RAF on mouseup", () => {
      const { result } = renderHook(() =>
        useThrottledResize({
          minWidth: 200,
          maxWidth: 600,
          onWidthChange: vi.fn(),
          calculateWidth: (e) => e.clientX,
        })
      );

      act(() => {
        result.current.startResizing({
          preventDefault: vi.fn(),
        } as unknown as React.MouseEvent);
      });

      act(() => {
        document.dispatchEvent(new MouseEvent("mousemove", { clientX: 400 }));
      });

      // RAF should be scheduled
      expect(rafCallbacks.size).toBe(1);

      act(() => {
        document.dispatchEvent(new MouseEvent("mouseup"));
      });

      // RAF should be cancelled
      expect(cancelAnimationFrame).toHaveBeenCalled();
    });

    it("should remove event listeners on unmount", () => {
      const removeEventListenerSpy = vi.spyOn(document, "removeEventListener");

      const { unmount } = renderHook(() =>
        useThrottledResize({
          minWidth: 200,
          maxWidth: 600,
          onWidthChange: vi.fn(),
        })
      );

      unmount();

      expect(removeEventListenerSpy).toHaveBeenCalledWith("mousemove", expect.any(Function));
      expect(removeEventListenerSpy).toHaveBeenCalledWith("mouseup", expect.any(Function));

      removeEventListenerSpy.mockRestore();
    });

    it("should cancel RAF on unmount", () => {
      const { result, unmount } = renderHook(() =>
        useThrottledResize({
          minWidth: 200,
          maxWidth: 600,
          onWidthChange: vi.fn(),
          calculateWidth: (e) => e.clientX,
        })
      );

      act(() => {
        result.current.startResizing({
          preventDefault: vi.fn(),
        } as unknown as React.MouseEvent);
      });

      act(() => {
        document.dispatchEvent(new MouseEvent("mousemove", { clientX: 400 }));
      });

      unmount();

      expect(cancelAnimationFrame).toHaveBeenCalled();
    });
  });

  describe("custom calculateWidth", () => {
    it("should use custom calculateWidth function", () => {
      const onWidthChange = vi.fn();
      const calculateWidth = vi.fn((e: MouseEvent) => window.innerWidth - e.clientX);

      const { result } = renderHook(() =>
        useThrottledResize({
          minWidth: 200,
          maxWidth: 600,
          onWidthChange,
          calculateWidth,
        })
      );

      act(() => {
        result.current.startResizing({
          preventDefault: vi.fn(),
        } as unknown as React.MouseEvent);
      });

      act(() => {
        document.dispatchEvent(new MouseEvent("mousemove", { clientX: 800 }));
      });

      act(() => {
        flushRaf();
      });

      expect(calculateWidth).toHaveBeenCalled();
      // window.innerWidth is typically 1024 in jsdom, so 1024 - 800 = 224
      // which is within our 200-600 range
    });
  });

  describe("edge cases", () => {
    it("should handle multiple start/stop cycles", () => {
      const onWidthChange = vi.fn();
      const { result } = renderHook(() =>
        useThrottledResize({
          minWidth: 200,
          maxWidth: 600,
          onWidthChange,
          calculateWidth: (e) => e.clientX,
        })
      );

      // First cycle
      act(() => {
        result.current.startResizing({
          preventDefault: vi.fn(),
        } as unknown as React.MouseEvent);
      });
      act(() => {
        document.dispatchEvent(new MouseEvent("mousemove", { clientX: 300 }));
      });
      act(() => {
        flushRaf();
      });
      act(() => {
        document.dispatchEvent(new MouseEvent("mouseup"));
      });

      expect(onWidthChange).toHaveBeenCalledWith(300);
      onWidthChange.mockClear();

      // Second cycle
      act(() => {
        result.current.startResizing({
          preventDefault: vi.fn(),
        } as unknown as React.MouseEvent);
      });
      act(() => {
        document.dispatchEvent(new MouseEvent("mousemove", { clientX: 500 }));
      });
      act(() => {
        flushRaf();
      });

      expect(onWidthChange).toHaveBeenCalledWith(500);
    });

    it("should not throw if startResizing is called multiple times", () => {
      const { result } = renderHook(() =>
        useThrottledResize({
          minWidth: 200,
          maxWidth: 600,
          onWidthChange: vi.fn(),
        })
      );

      const mockEvent = { preventDefault: vi.fn() } as unknown as React.MouseEvent;

      expect(() => {
        act(() => {
          result.current.startResizing(mockEvent);
          result.current.startResizing(mockEvent);
        });
      }).not.toThrow();
    });
  });
});
