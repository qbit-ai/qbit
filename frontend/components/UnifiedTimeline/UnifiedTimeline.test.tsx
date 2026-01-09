import { render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { useStore } from "../../store";
import { UnifiedTimeline } from "./UnifiedTimeline";

// Mock xterm.js and addons - they don't work in jsdom
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
    registerLinkProvider = vi.fn(() => ({ dispose: vi.fn() }));
    buffer = {
      active: {
        getLine: vi.fn(() => ({
          translateToString: vi.fn(() => ""),
        })),
      },
    };
  },
}));

vi.mock("@xterm/addon-fit", () => ({
  FitAddon: class MockFitAddon {
    fit = vi.fn();
  },
}));

vi.mock("@xterm/addon-serialize", () => ({
  SerializeAddon: class MockSerializeAddon {
    serialize = vi.fn(() => "");
  },
}));

describe("UnifiedTimeline", () => {
  beforeEach(() => {
    // Reset store state
    useStore.setState({
      sessions: {},
      activeSessionId: null,
      timelines: {},
      commandBlocks: {},
      pendingCommand: {},
      agentMessages: {},
      agentStreaming: {},
      agentInitialized: {},
      pendingToolApproval: {},
      processedToolRequests: new Set<string>(),
    });

    // Create a test session
    useStore.getState().addSession({
      id: "test-session",
      name: "Test",
      workingDirectory: "/test",
      createdAt: new Date().toISOString(),
      mode: "terminal",
    });
  });

  describe("Empty State", () => {
    it("should show empty state when no timeline, no streaming, and no running command", () => {
      render(<UnifiedTimeline sessionId="test-session" />);

      // WelcomeScreen renders empty state (no running command indicators)
      expect(screen.queryByText("Running...")).not.toBeInTheDocument();
      // Verify the WelcomeScreen container is rendered
      expect(document.querySelector(".h-full")).toBeInTheDocument();
    });

    it("should NOT show empty state when there is a running command with command text", () => {
      useStore.getState().handleCommandStart("test-session", "ls -la");

      render(<UnifiedTimeline sessionId="test-session" />);

      // Empty state text should NOT be visible
      expect(screen.queryByText("Qbit")).not.toBeInTheDocument();
      // Command header should show the command text
      expect(screen.getByText("ls -la")).toBeInTheDocument();
      // Terminal container should be rendered
      const terminalContainer = document.querySelector(".h-96.overflow-hidden");
      expect(terminalContainer).toBeInTheDocument();
    });

    it("should show empty state when pendingCommand exists but command is null", () => {
      // This simulates receiving terminal_output before command_start
      // which shouldn't happen but we should handle gracefully
      useStore.getState().handleCommandStart("test-session", null);

      render(<UnifiedTimeline sessionId="test-session" />);

      // Should still show empty state since there's no actual command
      expect(screen.queryByText("Running...")).not.toBeInTheDocument();
      expect(document.querySelector(".h-full")).toBeInTheDocument();
    });

    it("should NOT show empty state when agent is streaming", () => {
      useStore.getState().updateAgentStreaming("test-session", "Thinking...");

      render(<UnifiedTimeline sessionId="test-session" />);

      expect(screen.queryByText("Qbit")).not.toBeInTheDocument();
      expect(screen.getByText("Thinking...")).toBeInTheDocument();
    });
  });

  describe("Running Command Display", () => {
    it("should show terminal container when command is running", () => {
      useStore.getState().handleCommandStart("test-session", "ping localhost");

      render(<UnifiedTimeline sessionId="test-session" />);

      // Terminal block should be rendered (command text is not shown in header anymore)
      const terminalContainer = document.querySelector(".h-96.overflow-hidden");
      expect(terminalContainer).toBeInTheDocument();
    });

    it("should NOT show running indicator when pendingCommand.command is null", () => {
      useStore.getState().handleCommandStart("test-session", null);

      render(<UnifiedTimeline sessionId="test-session" />);

      // The running command section shouldn't render
      expect(screen.queryByText("Running...")).not.toBeInTheDocument();
    });

    it("should show terminal container for running command with output", () => {
      useStore.getState().handleCommandStart("test-session", "cat file.txt");
      useStore.getState().appendOutput("test-session", "line 1\nline 2\n");

      render(<UnifiedTimeline sessionId="test-session" />);

      // Output is rendered in an embedded xterm.js terminal (mocked in tests)
      // Verify the terminal container exists
      const terminalContainer = document.querySelector(".h-96.overflow-hidden");
      expect(terminalContainer).toBeInTheDocument();
    });

    it("should show terminal container even when pendingCommand has no output yet", () => {
      useStore.getState().handleCommandStart("test-session", "ls");

      render(<UnifiedTimeline sessionId="test-session" />);

      // Terminal container should still be rendered for running commands
      const terminalContainer = document.querySelector(".h-96.overflow-hidden");
      expect(terminalContainer).toBeInTheDocument();
    });
  });

  describe("Completed Commands in Timeline", () => {
    it("should show completed command block in timeline", () => {
      useStore.getState().handleCommandStart("test-session", "echo hello");
      useStore.getState().appendOutput("test-session", "hello\n");
      useStore.getState().handleCommandEnd("test-session", 0);

      render(<UnifiedTimeline sessionId="test-session" />);

      // Command should be in the timeline (via UnifiedBlock)
      expect(screen.getByText("echo hello")).toBeInTheDocument();
    });

    it("should show multiple completed commands in order", () => {
      const store = useStore.getState();

      store.handleCommandStart("test-session", "first");
      store.appendOutput("test-session", "1\n");
      store.handleCommandEnd("test-session", 0);

      store.handleCommandStart("test-session", "second");
      store.appendOutput("test-session", "2\n");
      store.handleCommandEnd("test-session", 0);

      render(<UnifiedTimeline sessionId="test-session" />);

      screen.getAllByRole("code");
      // Both commands should be visible
      expect(screen.getByText("first")).toBeInTheDocument();
      expect(screen.getByText("second")).toBeInTheDocument();
    });
  });

  describe("Agent Streaming", () => {
    it("should show agent streaming indicator with content", () => {
      useStore
        .getState()
        .updateAgentStreaming("test-session", "I am thinking about your request...");

      render(<UnifiedTimeline sessionId="test-session" />);

      expect(screen.getByText(/I am thinking about your request/)).toBeInTheDocument();
    });

    it("should show pulsing cursor during agent streaming", () => {
      useStore.getState().updateAgentStreaming("test-session", "Response...");

      render(<UnifiedTimeline sessionId="test-session" />);

      // There should be a pulsing cursor element
      const cursor = document.querySelector(".animate-pulse");
      expect(cursor).toBeInTheDocument();
    });
  });

  describe("Bug Prevention - The Issues We Fixed", () => {
    it("BUG: should NOT show Running or empty command when app starts fresh", () => {
      // Fresh state - no commands started
      render(<UnifiedTimeline sessionId="test-session" />);

      // Should show empty state (WelcomeScreen), not "Running..."
      expect(screen.queryByText("Running...")).not.toBeInTheDocument();
      expect(document.querySelector(".h-full")).toBeInTheDocument();
    });

    it("BUG: should NOT create (empty command) blocks", () => {
      const store = useStore.getState();

      // Simulate what was happening: command_start with null followed by command_end
      store.handleCommandStart("test-session", null);
      store.handleCommandEnd("test-session", 0);

      render(<UnifiedTimeline sessionId="test-session" />);

      // Should show empty state, not a block with "(empty command)"
      expect(screen.queryByText("Running...")).not.toBeInTheDocument();
      expect(useStore.getState().commandBlocks["test-session"]).toHaveLength(0);
    });

    it("terminal output before command_start SHOULD create pendingCommand (fallback for missing shell integration)", () => {
      const store = useStore.getState();

      // This simulates receiving output when no command is running (shell integration missing)
      // The new behavior is to show output even without command_start, as a fallback
      store.appendOutput("test-session", "prompt text\n");

      render(<UnifiedTimeline sessionId="test-session" />);

      // Should show the terminal block (no header, just the terminal container)
      // pendingCommand should be auto-created with null command
      expect(useStore.getState().pendingCommand["test-session"]).toBeDefined();
      expect(useStore.getState().pendingCommand["test-session"]?.command).toBeNull();
      expect(useStore.getState().pendingCommand["test-session"]?.output).toBe("prompt text\n");
    });

    it("BUG: empty string command should NOT create a block", () => {
      const store = useStore.getState();

      store.handleCommandStart("test-session", "");
      store.handleCommandEnd("test-session", 0);

      render(<UnifiedTimeline sessionId="test-session" />);

      // Should show empty state
      expect(screen.queryByText("Running...")).not.toBeInTheDocument();
      expect(useStore.getState().commandBlocks["test-session"]).toHaveLength(0);
    });
  });
});
