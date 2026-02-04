import { act, renderHook } from "@testing-library/react";
import type { ReactNode } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";

/**
 * TDD Tests for useTerminalPortal
 *
 * Issue: useSyncExternalStore receives mutable Map, React may miss updates
 * Goal: Return immutable snapshot from getSnapshot
 */

// Mock xterm.js
vi.mock("@xterm/xterm", () => ({
  Terminal: class MockTerminal {
    options = { theme: {} };
    rows = 24;
    cols = 80;
    loadAddon = vi.fn();
    open = vi.fn();
    write = vi.fn();
    clear = vi.fn();
    dispose = vi.fn();
    scrollToBottom = vi.fn();
    resize = vi.fn();
    element = document.createElement("div");
  },
}));

describe("useTerminalPortal", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe("getSnapshot immutability", () => {
    it("should return a new reference when targets change", async () => {
      const { TerminalPortalProvider, useTerminalPortalTargets, useTerminalPortalTarget } =
        await import("./useTerminalPortal");

      // Create a combined hook that tests the behavior
      function useCombinedTest() {
        const targets = useTerminalPortalTargets();
        const setTarget = useTerminalPortalTarget("session-1");
        return { targets, setTarget };
      }

      // Create wrapper with provider
      const wrapper = ({ children }: { children: ReactNode }) => (
        <TerminalPortalProvider>{children}</TerminalPortalProvider>
      );

      const { result, rerender } = renderHook(() => useCombinedTest(), { wrapper });

      const initialTargets = result.current.targets;

      // Set the target element
      const mockElement = document.createElement("div");
      act(() => {
        result.current.setTarget(mockElement);
      });

      // Force re-render to pick up the change
      rerender();

      // Targets should be a new reference after registration
      const newTargets = result.current.targets;

      // The key test: after a target is registered, we should get a new reference
      // If using mutable Map incorrectly, these would be the same reference
      expect(initialTargets).not.toBe(newTargets);
    });

    it("should return stable reference when no changes occur", async () => {
      const { TerminalPortalProvider, useTerminalPortalTargets } = await import(
        "./useTerminalPortal"
      );

      const wrapper = ({ children }: { children: ReactNode }) => (
        <TerminalPortalProvider>{children}</TerminalPortalProvider>
      );

      const { result, rerender } = renderHook(() => useTerminalPortalTargets(), {
        wrapper,
      });

      const firstResult = result.current;
      rerender();
      const secondResult = result.current;

      // Should be same reference when nothing changed
      expect(firstResult).toBe(secondResult);
    });
  });

  describe("target registration", () => {
    it("should track registered targets correctly", async () => {
      const { TerminalPortalProvider, useTerminalPortalTargets, useTerminalPortalTarget } =
        await import("./useTerminalPortal");

      // Create a combined hook that tests the behavior
      function useCombinedTest() {
        const targets = useTerminalPortalTargets();
        const setTarget = useTerminalPortalTarget("session-1");
        return { targets, setTarget };
      }

      const wrapper = ({ children }: { children: ReactNode }) => (
        <TerminalPortalProvider>{children}</TerminalPortalProvider>
      );

      const { result, rerender } = renderHook(() => useCombinedTest(), { wrapper });

      const mockElement = document.createElement("div");
      act(() => {
        result.current.setTarget(mockElement);
      });

      // Re-render to pick up changes
      rerender();

      // Check that the target is tracked
      expect(result.current.targets.get("session-1")?.element).toBe(mockElement);
    });

    it("should remove target on cleanup", async () => {
      const { TerminalPortalProvider, useTerminalPortalTargets, useTerminalPortalTarget } =
        await import("./useTerminalPortal");

      const wrapper = ({ children }: { children: ReactNode }) => (
        <TerminalPortalProvider>{children}</TerminalPortalProvider>
      );

      // Register a target
      const { result: targetResult, unmount } = renderHook(
        () => useTerminalPortalTarget("session-1"),
        { wrapper }
      );

      const mockElement = document.createElement("div");
      act(() => {
        targetResult.current(mockElement);
      });

      // Unmount to trigger cleanup
      unmount();

      // Check targets - should be empty after cleanup
      const { result: targetsResult } = renderHook(() => useTerminalPortalTargets(), {
        wrapper,
      });

      expect(targetsResult.current.has("session-1")).toBe(false);
    });
  });
});
