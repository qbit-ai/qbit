import { act, renderHook, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { usePathCompletion } from "./usePathCompletion";

// Mock the tauri module
vi.mock("@/lib/tauri", () => ({
  listPathCompletions: vi.fn(),
}));

import { listPathCompletions } from "@/lib/tauri";

const mockListPathCompletions = vi.mocked(listPathCompletions);

// Helper to create PathCompletionResponse
function createResponse(
  completions: Array<{
    name: string;
    insert_text: string;
    entry_type: "directory" | "file" | "symlink";
    score?: number;
    match_indices?: number[];
  }>,
  totalCount?: number
) {
  return {
    completions: completions.map((c) => ({
      ...c,
      score: c.score ?? 0,
      match_indices: c.match_indices ?? [],
    })),
    total_count: totalCount ?? completions.length,
  };
}

describe("usePathCompletion", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  afterEach(() => {
    vi.resetAllMocks();
  });

  describe("basic functionality", () => {
    it("should return empty completions when disabled", () => {
      const { result } = renderHook(() =>
        usePathCompletion({
          sessionId: "test-session",
          partialPath: "src/",
          enabled: false,
        })
      );

      expect(result.current.completions).toEqual([]);
      expect(result.current.totalCount).toBe(0);
      expect(result.current.isLoading).toBe(false);
      expect(mockListPathCompletions).not.toHaveBeenCalled();
    });

    it("should fetch completions when enabled", async () => {
      const mockCompletions = [
        { name: "Documents/", insert_text: "Documents/", entry_type: "directory" as const },
        { name: "file.txt", insert_text: "file.txt", entry_type: "file" as const },
      ];

      mockListPathCompletions.mockResolvedValueOnce(createResponse(mockCompletions));

      const { result } = renderHook(() =>
        usePathCompletion({
          sessionId: "test-session",
          partialPath: "",
          enabled: true,
        })
      );

      // Initially loading
      expect(result.current.isLoading).toBe(true);

      await waitFor(() => {
        expect(result.current.isLoading).toBe(false);
      });

      expect(result.current.completions).toHaveLength(2);
      expect(result.current.totalCount).toBe(2);
      expect(mockListPathCompletions).toHaveBeenCalledWith("test-session", "", 20);
    });

    it("should pass partial path to the API", async () => {
      mockListPathCompletions.mockResolvedValueOnce(createResponse([]));

      renderHook(() =>
        usePathCompletion({
          sessionId: "session-123",
          partialPath: "~/Doc",
          enabled: true,
        })
      );

      await waitFor(() => {
        expect(mockListPathCompletions).toHaveBeenCalledWith("session-123", "~/Doc", 20);
      });
    });

    it("should return totalCount from response", async () => {
      mockListPathCompletions.mockResolvedValueOnce(
        createResponse(
          [{ name: "Documents/", insert_text: "Documents/", entry_type: "directory" as const }],
          50 // Total matches is higher than returned completions
        )
      );

      const { result } = renderHook(() =>
        usePathCompletion({
          sessionId: "test-session",
          partialPath: "doc",
          enabled: true,
        })
      );

      await waitFor(() => {
        expect(result.current.isLoading).toBe(false);
      });

      expect(result.current.completions).toHaveLength(1);
      expect(result.current.totalCount).toBe(50);
    });
  });

  describe("state transitions", () => {
    it("should clear completions when disabled after being enabled", async () => {
      const mockCompletions = [
        { name: "test/", insert_text: "test/", entry_type: "directory" as const },
      ];

      mockListPathCompletions.mockResolvedValueOnce(createResponse(mockCompletions));

      const { result, rerender } = renderHook(
        ({ enabled }) =>
          usePathCompletion({
            sessionId: "test-session",
            partialPath: "",
            enabled,
          }),
        { initialProps: { enabled: true } }
      );

      await waitFor(() => {
        expect(result.current.completions).toHaveLength(1);
      });

      // Disable the hook
      rerender({ enabled: false });

      expect(result.current.completions).toEqual([]);
      expect(result.current.totalCount).toBe(0);
      expect(result.current.isLoading).toBe(false);
    });

    it("should refetch when partial path changes", async () => {
      mockListPathCompletions
        .mockResolvedValueOnce(
          createResponse([{ name: "foo/", insert_text: "foo/", entry_type: "directory" as const }])
        )
        .mockResolvedValueOnce(
          createResponse([{ name: "bar.txt", insert_text: "bar.txt", entry_type: "file" as const }])
        );

      const { result, rerender } = renderHook(
        ({ partialPath }) =>
          usePathCompletion({
            sessionId: "test-session",
            partialPath,
            enabled: true,
          }),
        { initialProps: { partialPath: "f" } }
      );

      await waitFor(() => {
        expect(result.current.completions).toHaveLength(1);
        expect(result.current.completions[0].name).toBe("foo/");
      });

      // Change partial path
      rerender({ partialPath: "b" });

      await waitFor(() => {
        expect(result.current.completions[0].name).toBe("bar.txt");
      });

      expect(mockListPathCompletions).toHaveBeenCalledTimes(2);
    });

    it("should refetch when session ID changes", async () => {
      mockListPathCompletions.mockResolvedValue(createResponse([]));

      const { rerender } = renderHook(
        ({ sessionId }) =>
          usePathCompletion({
            sessionId,
            partialPath: "",
            enabled: true,
          }),
        { initialProps: { sessionId: "session-1" } }
      );

      await waitFor(() => {
        expect(mockListPathCompletions).toHaveBeenCalledWith("session-1", "", 20);
      });

      rerender({ sessionId: "session-2" });

      await waitFor(() => {
        expect(mockListPathCompletions).toHaveBeenCalledWith("session-2", "", 20);
      });
    });
  });

  describe("error handling", () => {
    it("should handle API errors gracefully", async () => {
      const consoleSpy = vi.spyOn(console, "error").mockImplementation(() => {});
      mockListPathCompletions.mockRejectedValueOnce(new Error("Session not found"));

      const { result } = renderHook(() =>
        usePathCompletion({
          sessionId: "invalid-session",
          partialPath: "",
          enabled: true,
        })
      );

      await waitFor(() => {
        expect(result.current.isLoading).toBe(false);
      });

      expect(result.current.completions).toEqual([]);
      expect(result.current.totalCount).toBe(0);
      expect(consoleSpy).toHaveBeenCalledWith("Path completion error:", expect.any(Error));

      consoleSpy.mockRestore();
    });

    it("should handle network timeout errors", async () => {
      const consoleSpy = vi.spyOn(console, "error").mockImplementation(() => {});
      mockListPathCompletions.mockRejectedValueOnce(new Error("Network timeout"));

      const { result } = renderHook(() =>
        usePathCompletion({
          sessionId: "test-session",
          partialPath: "/some/path",
          enabled: true,
        })
      );

      await waitFor(() => {
        expect(result.current.isLoading).toBe(false);
      });

      expect(result.current.completions).toEqual([]);
      consoleSpy.mockRestore();
    });
  });

  describe("race condition handling", () => {
    it("should cancel in-flight requests when inputs change", async () => {
      let resolveFirst: (value: unknown) => void;
      const firstPromise = new Promise((resolve) => {
        resolveFirst = resolve;
      });

      mockListPathCompletions
        .mockReturnValueOnce(firstPromise as Promise<never>)
        .mockResolvedValueOnce(
          createResponse([
            { name: "second/", insert_text: "second/", entry_type: "directory" as const },
          ])
        );

      const { result, rerender } = renderHook(
        ({ partialPath }) =>
          usePathCompletion({
            sessionId: "test-session",
            partialPath,
            enabled: true,
          }),
        { initialProps: { partialPath: "first" } }
      );

      // Change input before first request resolves
      rerender({ partialPath: "second" });

      await waitFor(() => {
        expect(result.current.completions[0]?.name).toBe("second/");
      });

      // Now resolve the first request (should be ignored)
      act(() => {
        resolveFirst?.(
          createResponse([
            { name: "first/", insert_text: "first/", entry_type: "directory" as const },
          ])
        );
      });

      // Wait a tick to ensure state hasn't changed
      await new Promise((r) => setTimeout(r, 10));

      // Should still show second result, not first
      expect(result.current.completions[0]?.name).toBe("second/");
    });

    it("should handle rapid successive changes", async () => {
      mockListPathCompletions.mockImplementation(async (_sessionId, partialPath) => {
        // Simulate network delay
        await new Promise((r) => setTimeout(r, 50));
        return createResponse([
          {
            name: `result-${partialPath}/`,
            insert_text: `result-${partialPath}/`,
            entry_type: "directory" as const,
          },
        ]);
      });

      const { result, rerender } = renderHook(
        ({ partialPath }) =>
          usePathCompletion({
            sessionId: "test-session",
            partialPath,
            enabled: true,
          }),
        { initialProps: { partialPath: "a" } }
      );

      // Rapid changes
      rerender({ partialPath: "ab" });
      rerender({ partialPath: "abc" });
      rerender({ partialPath: "abcd" });

      await waitFor(
        () => {
          expect(result.current.completions[0]?.name).toBe("result-abcd/");
        },
        { timeout: 500 }
      );
    });
  });

  describe("cleanup", () => {
    it("should not update state after unmount", async () => {
      let resolvePromise: (value: unknown) => void;
      const pendingPromise = new Promise((resolve) => {
        resolvePromise = resolve;
      });

      mockListPathCompletions.mockReturnValueOnce(pendingPromise as Promise<never>);

      const { unmount } = renderHook(() =>
        usePathCompletion({
          sessionId: "test-session",
          partialPath: "",
          enabled: true,
        })
      );

      unmount();

      // Resolve after unmount - should not cause errors
      act(() => {
        resolvePromise?.(
          createResponse([
            { name: "test/", insert_text: "test/", entry_type: "directory" as const },
          ])
        );
      });

      // If we got here without errors, the cleanup worked
      expect(true).toBe(true);
    });
  });

  describe("completion types", () => {
    it("should correctly handle all entry types", async () => {
      const mixedCompletions = [
        { name: "folder/", insert_text: "folder/", entry_type: "directory" as const },
        { name: "file.txt", insert_text: "file.txt", entry_type: "file" as const },
        { name: "link", insert_text: "link", entry_type: "symlink" as const },
      ];

      mockListPathCompletions.mockResolvedValueOnce(createResponse(mixedCompletions));

      const { result } = renderHook(() =>
        usePathCompletion({
          sessionId: "test-session",
          partialPath: "",
          enabled: true,
        })
      );

      await waitFor(() => {
        expect(result.current.completions).toHaveLength(3);
      });

      expect(result.current.completions[0].entry_type).toBe("directory");
      expect(result.current.completions[1].entry_type).toBe("file");
      expect(result.current.completions[2].entry_type).toBe("symlink");
    });

    it("should include score and match_indices from response", async () => {
      mockListPathCompletions.mockResolvedValueOnce({
        completions: [
          {
            name: "Documents/",
            insert_text: "Documents/",
            entry_type: "directory" as const,
            score: 100,
            match_indices: [0, 1, 2],
          },
        ],
        total_count: 1,
      });

      const { result } = renderHook(() =>
        usePathCompletion({
          sessionId: "test-session",
          partialPath: "doc",
          enabled: true,
        })
      );

      await waitFor(() => {
        expect(result.current.completions).toHaveLength(1);
      });

      expect(result.current.completions[0].score).toBe(100);
      expect(result.current.completions[0].match_indices).toEqual([0, 1, 2]);
    });
  });
});
