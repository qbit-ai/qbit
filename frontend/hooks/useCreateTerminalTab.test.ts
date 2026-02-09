import { invoke } from "@tauri-apps/api/core";
import { act, renderHook } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { invalidateSettingsCache } from "@/lib/settings";
import { useStore } from "../store";
import { useCreateTerminalTab } from "./useCreateTerminalTab";

// Track invoke call timing to verify parallel execution
let invokeCallTimes: { command: string; time: number }[] = [];

// Helper to flush all pending background promises
async function flushBackgroundWork() {
  // Wait for multiple microtask cycles to let background promises settle
  for (let i = 0; i < 10; i++) {
    await new Promise((resolve) => setTimeout(resolve, 15));
  }
}

describe("useCreateTerminalTab", () => {
  beforeEach(() => {
    // Reset store state
    useStore.setState({
      sessions: {},
      activeSessionId: null,
      timelines: {},
      pendingCommand: {},
      agentStreaming: {},
      agentInitialized: {},
      pendingToolApproval: {},
      processedToolRequests: {},
    });

    // Clear settings cache to ensure fresh state
    invalidateSettingsCache();

    // Reset invoke call tracking
    invokeCallTimes = [];

    // Enhanced mock that tracks call timing
    vi.mocked(invoke).mockImplementation(async (command: string, args?: unknown) => {
      const callTime = Date.now();
      invokeCallTimes.push({ command, time: callTime });

      // Simulate some latency for each call
      await new Promise((resolve) => setTimeout(resolve, 10));

      const argsObj = args as Record<string, unknown> | undefined;
      switch (command) {
        case "pty_create":
          return {
            id: "test-session-id",
            working_directory: argsObj?.working_directory ?? "/test/dir",
          };
        case "get_settings":
          return {
            version: 1,
            terminal: { fullterm_commands: [] },
            ai: {
              default_provider: "anthropic",
              default_model: "claude-3-5-sonnet-20241022",
              openrouter: { api_key: null, show_in_selector: false },
              openai: { api_key: null, show_in_selector: false },
              anthropic: { api_key: "test-key", show_in_selector: true },
              ollama: { show_in_selector: false },
              gemini: { api_key: null, show_in_selector: false },
              groq: { api_key: null, show_in_selector: false },
              xai: { api_key: null, show_in_selector: false },
              zai_sdk: { api_key: null, show_in_selector: false },
              vertex_ai: {
                credentials_path: null,
                project_id: null,
                location: null,
                show_in_selector: false,
              },
              vertex_gemini: {
                credentials_path: null,
                project_id: null,
                location: null,
                show_in_selector: false,
              },
            },
          };
        case "get_project_settings":
          return { provider: null, model: null, agent_mode: null };
        case "get_git_branch":
          return "main";
        case "git_status":
          return { changed: 0, staged: 0, untracked: 0 };
        case "init_ai_session":
          return undefined;
        case "build_provider_config":
          return { provider: "anthropic", model: "claude-3-5-sonnet-20241022" };
        default:
          return undefined;
      }
    });
  });

  afterEach(() => {
    vi.clearAllMocks();
  });

  it("should create a terminal tab successfully", async () => {
    const { result } = renderHook(() => useCreateTerminalTab());

    let sessionId: string | null = null;
    await act(async () => {
      sessionId = await result.current.createTerminalTab("/test/path");
    });

    expect(sessionId).toBe("test-session-id");
    expect(useStore.getState().sessions["test-session-id"]).toBeDefined();
  });

  describe("startup performance optimization", () => {
    it("should return immediately after pty_create without waiting for settings or AI", async () => {
      const { result } = renderHook(() => useCreateTerminalTab());

      await act(async () => {
        await result.current.createTerminalTab("/test/path");
      });

      // pty_create should be the only call that blocks the return
      const ptyCreateCall = invokeCallTimes.find((c) => c.command === "pty_create");
      expect(ptyCreateCall).toBeDefined();

      // Session should be added with initializing status
      const session = useStore.getState().sessions["test-session-id"];
      expect(session).toBeDefined();
      expect(session.aiConfig?.status).toBe("initializing");
    });

    it("should fetch settings and project settings in parallel in background", async () => {
      const { result } = renderHook(() => useCreateTerminalTab());

      await act(async () => {
        await result.current.createTerminalTab("/test/path");
      });

      // Wait for background work to complete
      await act(async () => {
        await flushBackgroundWork();
      });

      // Both settings and project_settings should have been called
      const getSettingsCall = invokeCallTimes.find((c) => c.command === "get_settings");
      const getProjectSettingsCall = invokeCallTimes.find(
        (c) => c.command === "get_project_settings"
      );

      expect(getSettingsCall).toBeDefined();
      expect(getProjectSettingsCall).toBeDefined();

      // They should be called at approximately the same time (parallel)
      if (getSettingsCall && getProjectSettingsCall) {
        const timeDiff = Math.abs(getSettingsCall.time - getProjectSettingsCall.time);
        expect(timeDiff).toBeLessThan(5);
      }
    });

    it("should use cached settings on subsequent tab creations", async () => {
      const { result } = renderHook(() => useCreateTerminalTab());

      // Reset tracking
      invokeCallTimes = [];

      // Create first tab and wait for background work
      await act(async () => {
        await result.current.createTerminalTab("/test/path1");
      });
      await act(async () => {
        await flushBackgroundWork();
      });

      const firstSettingsCallCount = invokeCallTimes.filter(
        (c) => c.command === "get_settings"
      ).length;
      expect(firstSettingsCallCount).toBe(1);

      // Create second tab and wait for background work
      await act(async () => {
        await result.current.createTerminalTab("/test/path2");
      });
      await act(async () => {
        await flushBackgroundWork();
      });

      // Should not have called get_settings again (using cache)
      const totalSettingsCallCount = invokeCallTimes.filter(
        (c) => c.command === "get_settings"
      ).length;
      expect(totalSettingsCallCount).toBe(1); // Still just 1 call total
    });

    it("should call git branch and git status in parallel in background", async () => {
      const { result } = renderHook(() => useCreateTerminalTab());

      await act(async () => {
        await result.current.createTerminalTab("/test/path");
      });

      // Wait for background work to complete
      await act(async () => {
        await flushBackgroundWork();
      });

      // Find the timing of git calls
      const gitBranchCall = invokeCallTimes.find((c) => c.command === "get_git_branch");
      const gitStatusCall = invokeCallTimes.find((c) => c.command === "git_status");

      expect(gitBranchCall).toBeDefined();
      expect(gitStatusCall).toBeDefined();

      // Both git calls should happen at approximately the same time
      if (gitBranchCall && gitStatusCall) {
        const timeDiff = Math.abs(gitBranchCall.time - gitStatusCall.time);
        expect(timeDiff).toBeLessThan(5);
      }
    });

    it("should eventually update AI status to ready after background init", async () => {
      const { result } = renderHook(() => useCreateTerminalTab());

      await act(async () => {
        await result.current.createTerminalTab("/test/path");
      });

      // Initially should be "initializing"
      expect(useStore.getState().sessions["test-session-id"]?.aiConfig?.status).toBe(
        "initializing"
      );

      // Wait for background work
      await act(async () => {
        await flushBackgroundWork();
      });

      // Should now be "ready" (or "error" if AI init fails, but our mock succeeds)
      expect(useStore.getState().sessions["test-session-id"]?.aiConfig?.status).toBe("ready");
    });
  });
});
