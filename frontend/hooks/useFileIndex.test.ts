import { renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { useFileIndex } from "./useFileIndex";

// Mock the indexer module
vi.mock("@/lib/indexer", () => ({
  getAllIndexedFiles: vi.fn(),
  isIndexerInitialized: vi.fn(),
  getIndexerWorkspace: vi.fn(),
  initIndexer: vi.fn(),
  indexDirectory: vi.fn(),
}));

import {
  getAllIndexedFiles,
  getIndexerWorkspace,
  indexDirectory,
  initIndexer,
  isIndexerInitialized,
} from "@/lib/indexer";

const mockGetAllIndexedFiles = vi.mocked(getAllIndexedFiles);
const mockIsIndexerInitialized = vi.mocked(isIndexerInitialized);
const mockGetIndexerWorkspace = vi.mocked(getIndexerWorkspace);
const mockInitIndexer = vi.mocked(initIndexer);
const mockIndexDirectory = vi.mocked(indexDirectory);

describe("useFileIndex", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    // Default mocks: indexer is already initialized for the workspace
    mockIsIndexerInitialized.mockResolvedValue(true);
    mockGetIndexerWorkspace.mockResolvedValue("/workspace");
    mockInitIndexer.mockResolvedValue({
      files_indexed: 0,
      success: true,
      message: "Initialized",
    });
    mockIndexDirectory.mockResolvedValue({
      files_indexed: 0,
      success: true,
      message: "Indexed",
    });
  });

  it("should return null when workspaceRoot is undefined", () => {
    const { result } = renderHook(() => useFileIndex(undefined));

    expect(result.current).toBeNull();
    expect(mockGetAllIndexedFiles).not.toHaveBeenCalled();
  });

  it("should fetch and build index when workspaceRoot is provided", async () => {
    const mockFiles = ["/workspace/src/main.ts", "/workspace/src/utils.ts"];
    mockGetAllIndexedFiles.mockResolvedValueOnce(mockFiles);

    const { result } = renderHook(() => useFileIndex("/workspace"));

    // Initially null while loading
    expect(result.current).toBeNull();

    await waitFor(() => {
      expect(result.current).not.toBeNull();
    });

    expect(result.current?.absolutePaths.size).toBe(2);
    expect(result.current?.byFilename.get("main.ts")).toEqual(["/workspace/src/main.ts"]);
    expect(result.current?.byFilename.get("utils.ts")).toEqual(["/workspace/src/utils.ts"]);
    expect(result.current?.workspaceRoot).toBe("/workspace");
  });

  it("should refetch when workspaceRoot changes", async () => {
    const mockFiles1 = ["/workspace1/file.ts"];
    const mockFiles2 = ["/workspace2/other.ts"];

    mockGetAllIndexedFiles.mockResolvedValueOnce(mockFiles1).mockResolvedValueOnce(mockFiles2);

    const { result, rerender } = renderHook(({ root }) => useFileIndex(root), {
      initialProps: { root: "/workspace1" },
    });

    await waitFor(() => {
      expect(result.current?.workspaceRoot).toBe("/workspace1");
    });

    expect(result.current?.absolutePaths.has("/workspace1/file.ts")).toBe(true);

    rerender({ root: "/workspace2" });

    await waitFor(() => {
      expect(result.current?.workspaceRoot).toBe("/workspace2");
    });

    expect(result.current?.absolutePaths.has("/workspace2/other.ts")).toBe(true);
    expect(mockGetAllIndexedFiles).toHaveBeenCalledTimes(2);
  });

  it("should handle fetch errors gracefully", async () => {
    mockGetAllIndexedFiles.mockRejectedValueOnce(new Error("Failed to fetch"));

    const { result } = renderHook(() => useFileIndex("/workspace"));

    // Should not throw, should return null
    await waitFor(() => {
      // Wait for the effect to complete
      expect(mockGetAllIndexedFiles).toHaveBeenCalled();
    });

    // Give time for error handling
    await new Promise((resolve) => setTimeout(resolve, 50));

    expect(result.current).toBeNull();
  });

  it("should reset to null when workspaceRoot becomes undefined", async () => {
    const mockFiles = ["/workspace/file.ts"];
    mockGetAllIndexedFiles.mockResolvedValueOnce(mockFiles);

    const { result, rerender } = renderHook(({ root }) => useFileIndex(root), {
      initialProps: { root: "/workspace" as string | undefined },
    });

    await waitFor(() => {
      expect(result.current).not.toBeNull();
    });

    rerender({ root: undefined });

    expect(result.current).toBeNull();
  });
});
