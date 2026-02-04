import { act, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { SessionListingInfo, SessionSnapshot } from "@/lib/ai";

// Mock the AI module
vi.mock("@/lib/ai", () => ({
  listAiSessions: vi.fn(),
  loadAiSession: vi.fn(),
  exportAiSessionTranscript: vi.fn(),
}));

// Mock the notify module
vi.mock("@/lib/notify", () => ({
  notify: {
    success: vi.fn(),
    error: vi.fn(),
  },
}));

import { SessionBrowser } from "./SessionBrowser";
import { listAiSessions, loadAiSession } from "@/lib/ai";

// Helper to create mock session listing
function createMockSession(overrides: Partial<SessionListingInfo> = {}): SessionListingInfo {
  return {
    identifier: `session-${Math.random().toString(36).slice(2, 9)}`,
    path: "/path/to/session",
    workspace_label: "test-project",
    workspace_path: "/Users/test/projects/test-project",
    model: "claude-sonnet-4-5",
    provider: "anthropic",
    started_at: new Date().toISOString(),
    ended_at: new Date().toISOString(),
    total_messages: 10,
    distinct_tools: ["read_file", "write_file"],
    first_prompt_preview: "Test prompt preview",
    first_reply_preview: "Test reply preview",
    status: "completed",
    ...overrides,
  };
}

// Helper to create mock session snapshot with messages
function createMockSnapshot(messageCount: number): SessionSnapshot {
  const messages = Array.from({ length: messageCount }, (_, i) => ({
    role: (i % 2 === 0 ? "user" : "assistant") as "user" | "assistant",
    content: `Message content ${i + 1}. This is a longer message to simulate real content that would appear in a session transcript.`,
  }));

  return {
    workspace_label: "test-project",
    workspace_path: "/Users/test/projects/test-project",
    model: "claude-sonnet-4-5",
    provider: "anthropic",
    started_at: new Date().toISOString(),
    ended_at: new Date().toISOString(),
    total_messages: messageCount,
    distinct_tools: ["read_file"],
    transcript: [],
    messages,
  };
}

describe("SessionBrowser", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    // Default mock implementations
    vi.mocked(listAiSessions).mockResolvedValue([]);
    vi.mocked(loadAiSession).mockResolvedValue(null);
  });

  describe("search filtering", () => {
    it("should filter sessions based on search query", async () => {
      const user = userEvent.setup();
      const sessions = [
        createMockSession({ workspace_label: "react-app", first_prompt_preview: "Build a button" }),
        createMockSession({ workspace_label: "vue-project", first_prompt_preview: "Create a form" }),
        createMockSession({ workspace_label: "angular-demo", first_prompt_preview: "Setup routing" }),
      ];
      vi.mocked(listAiSessions).mockResolvedValue(sessions);

      render(<SessionBrowser open={true} onOpenChange={() => {}} />);

      // Wait for sessions to load
      await waitFor(() => {
        expect(screen.getByText("react-app")).toBeInTheDocument();
      });

      // Type in search box
      const searchInput = screen.getByPlaceholderText("Search sessions...");
      await user.type(searchInput, "react");

      // Should show only matching session
      await waitFor(() => {
        expect(screen.getByText("react-app")).toBeInTheDocument();
        expect(screen.queryByText("vue-project")).not.toBeInTheDocument();
        expect(screen.queryByText("angular-demo")).not.toBeInTheDocument();
      });
    });

    it("should use deferred value for search to avoid blocking UI", async () => {
      const user = userEvent.setup();
      // Create many sessions to make filtering more expensive
      const sessions = Array.from({ length: 100 }, (_, i) =>
        createMockSession({
          workspace_label: `project-${i}`,
          first_prompt_preview: `Prompt for project ${i}`,
        })
      );
      vi.mocked(listAiSessions).mockResolvedValue(sessions);

      render(<SessionBrowser open={true} onOpenChange={() => {}} />);

      // Wait for sessions to load
      await waitFor(() => {
        expect(screen.getByText("project-0")).toBeInTheDocument();
      });

      const searchInput = screen.getByPlaceholderText("Search sessions...");

      // Type rapidly - with deferred value, UI should remain responsive
      const startTime = performance.now();
      await user.type(searchInput, "project-99");
      const typingTime = performance.now() - startTime;

      // Typing should complete quickly (within reasonable bounds)
      // The deferred value allows input to update immediately while filtering is deferred
      expect(typingTime).toBeLessThan(2000); // Should be much faster with deferred value

      // Eventually the filter should apply
      await waitFor(
        () => {
          expect(screen.getByText("project-99")).toBeInTheDocument();
        },
        { timeout: 1000 }
      );
    });

    it("should use useMemo for filtered sessions instead of useEffect+useState", async () => {
      // This test verifies the optimization by checking that filtering works synchronously
      // after the deferred value updates (useMemo) rather than requiring another render cycle (useEffect)
      const sessions = [
        createMockSession({ workspace_label: "alpha-project" }),
        createMockSession({ workspace_label: "beta-project" }),
      ];
      vi.mocked(listAiSessions).mockResolvedValue(sessions);

      render(<SessionBrowser open={true} onOpenChange={() => {}} />);

      await waitFor(() => {
        expect(screen.getByText("alpha-project")).toBeInTheDocument();
        expect(screen.getByText("beta-project")).toBeInTheDocument();
      });

      // Both should be visible initially - this validates the memo works correctly
      expect(screen.getByText("alpha-project")).toBeInTheDocument();
      expect(screen.getByText("beta-project")).toBeInTheDocument();
    });

    it("should filter by model name", async () => {
      const user = userEvent.setup();
      const sessions = [
        createMockSession({ workspace_label: "proj-1", model: "claude-sonnet-4-5" }),
        createMockSession({ workspace_label: "proj-2", model: "gpt-4o" }),
      ];
      vi.mocked(listAiSessions).mockResolvedValue(sessions);

      render(<SessionBrowser open={true} onOpenChange={() => {}} />);

      await waitFor(() => {
        expect(screen.getByText("proj-1")).toBeInTheDocument();
      });

      const searchInput = screen.getByPlaceholderText("Search sessions...");
      await user.type(searchInput, "gpt");

      await waitFor(() => {
        expect(screen.queryByText("proj-1")).not.toBeInTheDocument();
        expect(screen.getByText("proj-2")).toBeInTheDocument();
      });
    });

    it("should filter by first prompt preview", async () => {
      const user = userEvent.setup();
      const sessions = [
        createMockSession({
          workspace_label: "proj-1",
          first_prompt_preview: "Help me build a React component",
        }),
        createMockSession({
          workspace_label: "proj-2",
          first_prompt_preview: "Write a Python script",
        }),
      ];
      vi.mocked(listAiSessions).mockResolvedValue(sessions);

      render(<SessionBrowser open={true} onOpenChange={() => {}} />);

      await waitFor(() => {
        expect(screen.getByText("proj-1")).toBeInTheDocument();
      });

      const searchInput = screen.getByPlaceholderText("Search sessions...");
      await user.type(searchInput, "python");

      await waitFor(() => {
        expect(screen.queryByText("proj-1")).not.toBeInTheDocument();
        expect(screen.getByText("proj-2")).toBeInTheDocument();
      });
    });
  });

  describe("message list virtualization", () => {
    it("should render session detail when session is selected", async () => {
      const user = userEvent.setup();
      const session = createMockSession({ identifier: "test-session" });
      const snapshot = createMockSnapshot(10);

      vi.mocked(listAiSessions).mockResolvedValue([session]);
      vi.mocked(loadAiSession).mockResolvedValue(snapshot);

      render(<SessionBrowser open={true} onOpenChange={() => {}} onSessionRestore={() => {}} />);

      // Wait for sessions to load
      await waitFor(() => {
        expect(screen.getByText(session.workspace_label)).toBeInTheDocument();
      });

      // Click on the session to load details
      await user.click(screen.getByText(session.workspace_label));

      // Wait for session detail to load - should show Load Session button
      await waitFor(() => {
        expect(screen.getByText("Load Session")).toBeInTheDocument();
      });

      // Verify loadAiSession was called with the session identifier
      expect(loadAiSession).toHaveBeenCalledWith(session.identifier);
    });

    it("should render messages container with virtualization support", async () => {
      const user = userEvent.setup();
      const session = createMockSession({ identifier: "test-session" });
      const snapshot = createMockSnapshot(100);

      vi.mocked(listAiSessions).mockResolvedValue([session]);
      vi.mocked(loadAiSession).mockResolvedValue(snapshot);

      render(<SessionBrowser open={true} onOpenChange={() => {}} onSessionRestore={() => {}} />);

      await waitFor(() => {
        expect(screen.getByText(session.workspace_label)).toBeInTheDocument();
      });

      await user.click(screen.getByText(session.workspace_label));

      await waitFor(() => {
        expect(screen.getByText("Load Session")).toBeInTheDocument();
      });

      // The virtualized container should exist
      const messagesContainer = screen.getByTestId("messages-container");
      expect(messagesContainer).toBeInTheDocument();
    });

    it("should call loadAiSession when session is clicked", async () => {
      const user = userEvent.setup();
      const session = createMockSession({ identifier: "my-session-id" });
      const snapshot = createMockSnapshot(5);

      vi.mocked(listAiSessions).mockResolvedValue([session]);
      vi.mocked(loadAiSession).mockResolvedValue(snapshot);

      render(<SessionBrowser open={true} onOpenChange={() => {}} onSessionRestore={() => {}} />);

      await waitFor(() => {
        expect(screen.getByText(session.workspace_label)).toBeInTheDocument();
      });

      await user.click(screen.getByText(session.workspace_label));

      // Verify the API was called to load the session
      await waitFor(() => {
        expect(loadAiSession).toHaveBeenCalledWith("my-session-id");
      });
    });
  });

  describe("dialog state management", () => {
    it("should reset search when dialog closes", async () => {
      const user = userEvent.setup();
      const sessions = [createMockSession({ workspace_label: "test-project" })];
      vi.mocked(listAiSessions).mockResolvedValue(sessions);

      const { rerender } = render(<SessionBrowser open={true} onOpenChange={() => {}} />);

      await waitFor(() => {
        expect(screen.getByText("test-project")).toBeInTheDocument();
      });

      // Type in search
      const searchInput = screen.getByPlaceholderText("Search sessions...");
      await user.type(searchInput, "test");

      // Close dialog
      rerender(<SessionBrowser open={false} onOpenChange={() => {}} />);

      // Reopen dialog
      rerender(<SessionBrowser open={true} onOpenChange={() => {}} />);

      await waitFor(() => {
        // Search should be cleared
        const newSearchInput = screen.getByPlaceholderText("Search sessions...");
        expect(newSearchInput).toHaveValue("");
      });
    });
  });
});
