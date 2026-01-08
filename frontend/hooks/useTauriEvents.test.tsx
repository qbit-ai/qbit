import { render, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { useTauriEvents } from "./useTauriEvents";
import { useStore } from "../store";
import type { GitStatusSummary } from "../lib/tauri";

vi.mock("../lib/settings", () => ({
  getSettings: vi.fn().mockResolvedValue({
    terminal: {
      fullterm_commands: [],
    },
  }),
}));

const getGitBranchMock = vi.fn<(path: string) => Promise<string | null>>();
const gitStatusMock = vi.fn<(workingDirectory: string) => Promise<GitStatusSummary>>();

vi.mock("../lib/tauri", () => ({
  getGitBranch: (path: string) => getGitBranchMock(path),
  gitStatus: (workingDirectory: string) => gitStatusMock(workingDirectory),
  ptyGetForegroundProcess: vi.fn().mockResolvedValue("zsh"),
}));

vi.mock("../lib/terminal", () => ({
  liveTerminalManager: {
    serializeAndDispose: vi.fn().mockResolvedValue(""),
    dispose: vi.fn(),
    getOrCreate: vi.fn(),
    scrollToBottom: vi.fn(),
    write: vi.fn(),
  },
  virtualTerminalManager: {
    dispose: vi.fn(),
    create: vi.fn(),
    write: vi.fn(),
  },
}));

vi.mock("../lib/ai", () => ({
  isAiSessionInitialized: vi.fn().mockResolvedValue(false),
  updateAiWorkspace: vi.fn().mockResolvedValue(undefined),
}));

vi.mock("../lib/notify", () => ({
  notify: {
    info: vi.fn(),
    error: vi.fn(),
  },
}));

function Harness() {
  useTauriEvents();
  return null;
}

type MockListener = (event: { event: string; payload: any }) => void;

describe("useTauriEvents - git branch refresh", () => {
  const listenersByEvent = new Map<string, Set<MockListener>>();

  const emit = (eventName: string, payload: any) => {
    const listeners = listenersByEvent.get(eventName);
    if (!listeners) return;
    for (const listener of listeners) {
      listener({ event: eventName, payload });
    }
  };

  beforeEach(() => {
    // Minimal store reset (merge) so methods remain.
    useStore.setState({
      sessions: {},
      activeSessionId: null,
      pendingCommand: {},
      gitStatus: {},
      gitStatusLoading: {},
      timelines: {},
      commandBlocks: {},
    });

    listenersByEvent.clear();

    (window as any).__MOCK_BROWSER_MODE__ = true;
    (window as any).__MOCK_LISTEN__ = async (eventName: string, callback: MockListener) => {
      if (!listenersByEvent.has(eventName)) listenersByEvent.set(eventName, new Set());
      listenersByEvent.get(eventName)!.add(callback);
      return () => {
        listenersByEvent.get(eventName)?.delete(callback);
      };
    };

    getGitBranchMock.mockReset();
    gitStatusMock.mockReset();
  });

  it("updates git branch + status after `git switch` command_end", async () => {
    const store = useStore.getState();
    store.addSession({
      id: "s1",
      name: "Terminal",
      workingDirectory: "/repo",
      createdAt: new Date().toISOString(),
      mode: "terminal",
    });
    store.updateGitBranch("s1", "main");

    getGitBranchMock.mockResolvedValue("feature");
    gitStatusMock.mockResolvedValue({
      branch: "feature",
      ahead: 0,
      behind: 0,
      entries: [],
      insertions: 0,
      deletions: 0,
    });

    render(<Harness />);

    emit("command_block", {
      session_id: "s1",
      command: "git switch feature",
      exit_code: 0,
      event_type: "command_end",
    });

    await waitFor(() => {
      expect(getGitBranchMock).toHaveBeenCalledWith("/repo");
      expect(gitStatusMock).toHaveBeenCalledWith("/repo");
      expect(useStore.getState().sessions.s1?.gitBranch).toBe("feature");
      expect(useStore.getState().gitStatus.s1?.branch).toBe("feature");
    });
  });

  it("updates git branch + status after `git checkout -b` command_end", async () => {
    const store = useStore.getState();
    store.addSession({
      id: "s1",
      name: "Terminal",
      workingDirectory: "/repo",
      createdAt: new Date().toISOString(),
      mode: "terminal",
    });
    store.updateGitBranch("s1", "main");

    getGitBranchMock.mockResolvedValue("mybranch");
    gitStatusMock.mockResolvedValue({
      branch: "mybranch",
      ahead: 0,
      behind: 0,
      entries: [],
      insertions: 0,
      deletions: 0,
    });

    render(<Harness />);

    emit("command_block", {
      session_id: "s1",
      command: "git checkout -b mybranch",
      exit_code: 0,
      event_type: "command_end",
    });

    await waitFor(() => {
      expect(getGitBranchMock).toHaveBeenCalledWith("/repo");
      expect(gitStatusMock).toHaveBeenCalledWith("/repo");
      expect(useStore.getState().sessions.s1?.gitBranch).toBe("mybranch");
      expect(useStore.getState().gitStatus.s1?.branch).toBe("mybranch");
    });
  });

  it("does not refresh git branch for unrelated commands", async () => {
    const store = useStore.getState();
    store.addSession({
      id: "s1",
      name: "Terminal",
      workingDirectory: "/repo",
      createdAt: new Date().toISOString(),
      mode: "terminal",
    });
    store.updateGitBranch("s1", "main");

    render(<Harness />);

    emit("command_block", {
      session_id: "s1",
      command: "echo hello",
      exit_code: 0,
      event_type: "command_end",
    });

    // Let microtasks run.
    await Promise.resolve();

    expect(getGitBranchMock).not.toHaveBeenCalled();
    expect(gitStatusMock).not.toHaveBeenCalled();
    expect(useStore.getState().sessions.s1?.gitBranch).toBe("main");
  });
});
