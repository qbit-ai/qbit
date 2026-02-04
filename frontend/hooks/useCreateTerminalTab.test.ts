import { act, renderHook, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import { useStore } from "../store";
import { useCreateTerminalTab } from "./useCreateTerminalTab";
import { invalidateSettingsCache } from "@/lib/settings";

// Track invoke call timing to verify parallel execution
let invokeCallTimes: { command: string; time: number }[] = [];
const originalInvoke = invoke;

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
    vi.mocked(invoke).mockImplementation(async (command: string, args?: Record<string, unknown>) => {
      const callTime = Date.now();
      invokeCallTimes.push({ command, time: callTime });

      // Simulate some latency for each call
      await new Promise((resolve) => setTimeout(resolve, 10));

      switch (command) {
        case "pty_create":
          return {
            id: "test-session-id",
            working_directory: args?.working_directory ?? "/test/dir",
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

  describe("parallel request optimization", () => {
    it("should call pty_create and get_settings in parallel", async () => {
      const { result } = renderHook(() => useCreateTerminalTab());

      await act(async () => {
        await result.current.createTerminalTab("/test/path");
      });

      // Find the timing of pty_create and get_settings calls
      const ptyCreateCall = invokeCallTimes.find((c) => c.command === "pty_create");
      const getSettingsCall = invokeCallTimes.find((c) => c.command === "get_settings");

      expect(ptyCreateCall).toBeDefined();
      expect(getSettingsCall).toBeDefined();

      // Both calls should happen at approximately the same time (within 5ms)
      // If they were sequential, there would be at least 10ms delay
      const timeDiff = Math.abs(ptyCreateCall!.time - getSettingsCall!.time);
      expect(timeDiff).toBeLessThan(5);
    });

    it("should use cached settings on subsequent tab creations", async () => {
      const { result } = renderHook(() => useCreateTerminalTab());

      // Reset tracking
      invokeCallTimes = [];

      // Create first tab
      await act(async () => {
        await result.current.createTerminalTab("/test/path1");
      });

      const firstSettingsCallCount = invokeCallTimes.filter(
        (c) => c.command === "get_settings"
      ).length;
      expect(firstSettingsCallCount).toBe(1);

      // Reset tracking for second call
      const firstCallCount = invokeCallTimes.length;

      // Create second tab
      await act(async () => {
        await result.current.createTerminalTab("/test/path2");
      });

      // Should not have called get_settings again (using cache)
      const totalSettingsCallCount = invokeCallTimes.filter(
        (c) => c.command === "get_settings"
      ).length;
      expect(totalSettingsCallCount).toBe(1); // Still just 1 call total
    });

    it("should call git branch and git status in parallel", async () => {
      const { result } = renderHook(() => useCreateTerminalTab());

      await act(async () => {
        await result.current.createTerminalTab("/test/path");
      });

      // Find the timing of git calls
      const gitBranchCall = invokeCallTimes.find((c) => c.command === "get_git_branch");
      const gitStatusCall = invokeCallTimes.find((c) => c.command === "git_status");

      expect(gitBranchCall).toBeDefined();
      expect(gitStatusCall).toBeDefined();

      // Both git calls should happen at approximately the same time
      const timeDiff = Math.abs(gitBranchCall!.time - gitStatusCall!.time);
      expect(timeDiff).toBeLessThan(5);
    });
  });
});
