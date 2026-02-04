import { act, render, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

// Mock the modules before importing HomeView
vi.mock("@/lib/indexer", () => ({
  listProjectsForHome: vi.fn().mockResolvedValue([]),
  listRecentDirectories: vi.fn().mockResolvedValue([]),
}));

vi.mock("@/hooks/useCreateTerminalTab", () => ({
  useCreateTerminalTab: () => ({
    createTerminalTab: vi.fn(),
  }),
}));

import { listProjectsForHome, listRecentDirectories } from "@/lib/indexer";
import { HOME_VIEW_FOCUS_DEBOUNCE_MS, HOME_VIEW_FOCUS_MIN_INTERVAL_MS, HomeView } from "./HomeView";

describe("HomeView", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  describe("window focus debounce", () => {
    it("should fetch data on initial mount", async () => {
      vi.useRealTimers(); // Use real timers for initial render

      render(<HomeView />);

      await waitFor(() => {
        expect(listProjectsForHome).toHaveBeenCalledTimes(1);
        expect(listRecentDirectories).toHaveBeenCalledTimes(1);
      });
    });

    it("should debounce rapid window focus events", async () => {
      vi.useRealTimers(); // Need real timers for initial render

      render(<HomeView />);

      // Wait for initial fetch
      await waitFor(() => {
        expect(listProjectsForHome).toHaveBeenCalledTimes(1);
      });

      // Clear mocks to track focus events only
      vi.clearAllMocks();
      vi.useFakeTimers();

      // Simulate rapid window focus events
      act(() => {
        window.dispatchEvent(new Event("focus"));
      });

      // Immediately focus again
      act(() => {
        window.dispatchEvent(new Event("focus"));
      });

      // And again
      act(() => {
        window.dispatchEvent(new Event("focus"));
      });

      // Should NOT have called fetch yet (debounced)
      expect(listProjectsForHome).not.toHaveBeenCalled();

      // Advance past debounce period
      await act(async () => {
        vi.advanceTimersByTime(HOME_VIEW_FOCUS_DEBOUNCE_MS + 100);
      });

      // Should have called fetch once (not three times)
      expect(listProjectsForHome).toHaveBeenCalledTimes(1);
    });

    it("should respect minimum interval between focus fetches", async () => {
      vi.useRealTimers();

      render(<HomeView />);

      // Wait for initial fetch
      await waitFor(() => {
        expect(listProjectsForHome).toHaveBeenCalledTimes(1);
      });

      vi.clearAllMocks();
      vi.useFakeTimers();

      // First focus event
      act(() => {
        window.dispatchEvent(new Event("focus"));
      });

      // Wait for debounce
      await act(async () => {
        vi.advanceTimersByTime(HOME_VIEW_FOCUS_DEBOUNCE_MS + 100);
      });

      expect(listProjectsForHome).toHaveBeenCalledTimes(1);

      vi.clearAllMocks();

      // Focus again immediately after (within minimum interval)
      act(() => {
        window.dispatchEvent(new Event("focus"));
      });

      await act(async () => {
        vi.advanceTimersByTime(HOME_VIEW_FOCUS_DEBOUNCE_MS + 100);
      });

      // Should NOT fetch again - minimum interval not elapsed
      expect(listProjectsForHome).not.toHaveBeenCalled();
    });

    it("should fetch again after minimum interval has passed", async () => {
      vi.useRealTimers();

      render(<HomeView />);

      // Wait for initial fetch
      await waitFor(() => {
        expect(listProjectsForHome).toHaveBeenCalledTimes(1);
      });

      vi.clearAllMocks();
      vi.useFakeTimers();

      // First focus event
      act(() => {
        window.dispatchEvent(new Event("focus"));
      });

      await act(async () => {
        vi.advanceTimersByTime(HOME_VIEW_FOCUS_DEBOUNCE_MS + 100);
      });

      expect(listProjectsForHome).toHaveBeenCalledTimes(1);

      vi.clearAllMocks();

      // Wait past the minimum interval
      await act(async () => {
        vi.advanceTimersByTime(HOME_VIEW_FOCUS_MIN_INTERVAL_MS + 100);
      });

      // Now focus again
      act(() => {
        window.dispatchEvent(new Event("focus"));
      });

      await act(async () => {
        vi.advanceTimersByTime(HOME_VIEW_FOCUS_DEBOUNCE_MS + 100);
      });

      // Should fetch again - minimum interval has passed
      expect(listProjectsForHome).toHaveBeenCalledTimes(1);
    });
  });
});
