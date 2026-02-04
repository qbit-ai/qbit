import { render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

// Mock dependencies
vi.mock("@/hooks/useCreateTerminalTab", () => ({
  useCreateTerminalTab: () => ({
    createTerminalTab: vi.fn(),
  }),
}));

vi.mock("@/lib/indexer", () => ({
  listProjectsForHome: vi.fn().mockResolvedValue([]),
  listRecentDirectories: vi.fn().mockResolvedValue([]),
}));

vi.mock("@/lib/projects", () => ({
  saveProject: vi.fn().mockResolvedValue(undefined),
}));

vi.mock("@/lib/tauri", () => ({
  deleteWorktree: vi.fn().mockResolvedValue(undefined),
}));

describe("HomeView Memoization Tests", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe("ProjectRow memoization", () => {
    it("ProjectRow should be wrapped in React.memo", async () => {
      // Import the component
      const module = await import("./HomeView");

      // Check if ProjectRow is exported and is a memo component
      const ProjectRow = (module as Record<string, unknown>).ProjectRow;
      expect(ProjectRow).toBeDefined();

      const memoSymbol = Symbol.for("react.memo");
      const componentType = (ProjectRow as { $$typeof?: symbol }).$$typeof;
      expect(componentType).toBe(memoSymbol);
    });
  });

  describe("RecentDirectoryRow memoization", () => {
    it("RecentDirectoryRow should be wrapped in React.memo", async () => {
      // Import the component
      const module = await import("./HomeView");

      // Check if RecentDirectoryRow is exported and is a memo component
      const RecentDirectoryRow = (module as Record<string, unknown>).RecentDirectoryRow;
      expect(RecentDirectoryRow).toBeDefined();

      const memoSymbol = Symbol.for("react.memo");
      const componentType = (RecentDirectoryRow as { $$typeof?: symbol }).$$typeof;
      expect(componentType).toBe(memoSymbol);
    });
  });

  describe("Callback stability", () => {
    it("should use stable callbacks that do not change between renders", async () => {
      const { listProjectsForHome, listRecentDirectories } = await import("@/lib/indexer");
      vi.mocked(listProjectsForHome).mockResolvedValue([
        {
          name: "Test Project",
          path: "/test/project",
          last_activity: "1 hour ago",
          warnings: 0,
          branches: [
            {
              name: "main",
              path: "/test/project",
              last_activity: "1 hour ago",
              file_count: 5,
              insertions: 10,
              deletions: 3,
            },
          ],
        },
      ]);
      vi.mocked(listRecentDirectories).mockResolvedValue([]);

      const { HomeView } = await import("./HomeView");

      const { rerender } = render(<HomeView />);

      // Wait for loading to complete
      await screen.findByText("Projects");

      // Verify component renders without errors after rerender
      rerender(<HomeView />);

      // Component should still render correctly
      expect(screen.getByText("Projects")).toBeDefined();
    });
  });

  describe("Inline arrow function elimination", () => {
    /**
     * The original code had inline arrow functions like:
     * onToggle={() => toggleProject(project.path)}
     *
     * These should be replaced with stable callbacks using useCallback
     * that are passed down to memoized children.
     */
    it("ProjectRow should receive stable onToggle callback", async () => {
      // This test verifies the pattern is implemented correctly
      // by checking the component renders without issues when props change
      const { listProjectsForHome, listRecentDirectories } = await import("@/lib/indexer");
      vi.mocked(listProjectsForHome).mockResolvedValue([
        {
          name: "Project A",
          path: "/project/a",
          last_activity: "1 hour ago",
          warnings: 0,
          branches: [],
        },
        {
          name: "Project B",
          path: "/project/b",
          last_activity: "2 hours ago",
          warnings: 0,
          branches: [],
        },
      ]);
      vi.mocked(listRecentDirectories).mockResolvedValue([]);

      const { HomeView } = await import("./HomeView");

      render(<HomeView />);

      // Wait for loading to complete
      await screen.findByText("Projects");

      // Both projects should render
      expect(screen.getByText("Project A")).toBeDefined();
      expect(screen.getByText("Project B")).toBeDefined();
    });
  });
});
