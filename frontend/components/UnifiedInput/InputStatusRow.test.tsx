import { act, render, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { useStore } from "../../store";
import { clearAllSessionCaches } from "../../store/selectors/session";

/**
 * TDD Tests for InputStatusRow DRY Settings Update Logic
 *
 * Issue: Lines 290-357 duplicate the settings update logic (~60 lines copied):
 * 1. Initial mount effect calls refreshProviderSettings()
 * 2. settings-updated event handler duplicates the entire logic inline
 *
 * Goal: The event handler should simply call refreshProviderSettings()
 * instead of duplicating all the state updates.
 *
 * Pattern to fix:
 * ```typescript
 * // Mount effect
 * useEffect(() => {
 *   refreshProviderSettings();
 * }, [refreshProviderSettings]);
 *
 * // Event listener
 * useEffect(() => {
 *   const handleSettingsUpdated = () => refreshProviderSettings();
 *   window.addEventListener("settings-updated", handleSettingsUpdated);
 *   return () => window.removeEventListener("settings-updated", handleSettingsUpdated);
 * }, [refreshProviderSettings]);
 * ```
 */

// Mock dependencies
vi.mock("@/lib/ai", () => ({
  getApiRequestStats: vi.fn(() => Promise.resolve({ providers: {} })),
  getOpenAiApiKey: vi.fn(() => Promise.resolve(null)),
  getOpenRouterApiKey: vi.fn(() => Promise.resolve(null)),
  initAiSession: vi.fn(() => Promise.resolve()),
  saveProjectModel: vi.fn(() => Promise.resolve()),
}));

const mockSettings = {
  ai: {
    openrouter: { api_key: "test-or-key" },
    openai: { api_key: "test-oai-key" },
    anthropic: { api_key: "test-ant-key" },
    gemini: { api_key: null },
    groq: { api_key: null },
    xai: { api_key: null },
    zai_sdk: { api_key: null },
    vertex_ai: { credentials_path: null, project_id: null, location: null },
    vertex_gemini: { credentials_path: null, project_id: null, location: null },
    provider_visibility: {},
  },
};

vi.mock("@/lib/settings", () => ({
  getSettings: vi.fn(() => Promise.resolve(mockSettings)),
  isLangfuseActive: vi.fn(() => Promise.resolve(false)),
  getTelemetryStats: vi.fn(() => Promise.resolve(null)),
  buildProviderVisibility: vi.fn(() => ({
    vertex_ai: true,
    vertex_gemini: true,
    openrouter: true,
    openai: true,
    anthropic: true,
    ollama: true,
    gemini: true,
    groq: true,
    xai: true,
    zai_sdk: true,
  })),
}));

vi.mock("@/lib/models", () => ({
  formatModelName: vi.fn((model: string) => model),
  getProviderGroup: vi.fn(() => ({ models: [] })),
  getProviderGroupNested: vi.fn(() => ({ models: [] })),
}));

vi.mock("@/lib/notify", () => ({
  notify: {
    error: vi.fn(),
    warning: vi.fn(),
    success: vi.fn(),
  },
}));

vi.mock("@/lib/logger", () => ({
  logger: {
    debug: vi.fn(),
    info: vi.fn(),
    warn: vi.fn(),
    error: vi.fn(),
  },
}));

vi.mock("@/mocks", () => ({
  isMockBrowserMode: vi.fn(() => false),
}));

// Helper to reset store
const resetStore = () => {
  clearAllSessionCaches();
  useStore.setState({
    sessions: {},
    activeSessionId: null,
    timelines: {},
    pendingCommand: {},
    agentStreaming: {},
    streamingBlocks: {},
    streamingTextOffset: {},
    agentInitialized: {},
    isAgentThinking: {},
    isAgentResponding: {},
    pendingToolApproval: {},
    processedToolRequests: {},
    activeToolCalls: {},
    thinkingContent: {},
    isThinkingExpanded: {},
    activeWorkflows: {},
    workflowHistory: {},
    activeSubAgents: {},
    contextMetrics: {},
    compactionCount: {},
    isCompacting: {},
    isSessionDead: {},
    compactionError: {},
    gitStatus: {},
    gitStatusLoading: {},
    gitCommitMessage: {},
    tabLayouts: {},
    tabHasNewActivity: {},
    sessionTokenUsage: {},
  });
};

// Helper to create a session
const createSession = (sessionId: string) => {
  useStore.getState().addSession({
    id: sessionId,
    name: `Session ${sessionId}`,
    workingDirectory: `/home/${sessionId}`,
    createdAt: new Date().toISOString(),
    mode: "terminal",
    inputMode: "agent",
  });
  // Initialize AI config for the session
  useStore.getState().setSessionAiConfig(sessionId, {
    status: "ready",
    model: "claude-3-5-sonnet",
    provider: "anthropic",
  });
};

describe("InputStatusRow DRY Settings Update", () => {
  beforeEach(() => {
    resetStore();
    vi.clearAllMocks();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  describe("refreshProviderSettings reuse", () => {
    it("should call refreshProviderSettings on mount", async () => {
      const { getSettings } = await import("@/lib/settings");
      createSession("session-1");

      const { InputStatusRow } = await import("./InputStatusRow");
      render(<InputStatusRow sessionId="session-1" />);

      // Wait for settings to be fetched
      await waitFor(() => {
        expect(getSettings).toHaveBeenCalled();
      });
    });

    it("should call refreshProviderSettings when settings-updated event fires", async () => {
      const { getSettings } = await import("@/lib/settings");
      createSession("session-1");

      const { InputStatusRow } = await import("./InputStatusRow");
      render(<InputStatusRow sessionId="session-1" />);

      // Clear call count from mount
      vi.mocked(getSettings).mockClear();

      // Dispatch settings-updated event
      act(() => {
        window.dispatchEvent(new CustomEvent("settings-updated"));
      });

      // Wait for settings to be re-fetched
      await waitFor(() => {
        expect(getSettings).toHaveBeenCalled();
      });
    });

    it("should only call getSettings once per event (not duplicated)", async () => {
      const { getSettings } = await import("@/lib/settings");
      createSession("session-1");

      const { InputStatusRow } = await import("./InputStatusRow");
      render(<InputStatusRow sessionId="session-1" />);

      // Wait for initial mount
      await waitFor(() => {
        expect(getSettings).toHaveBeenCalled();
      });

      // Clear and dispatch event
      vi.mocked(getSettings).mockClear();

      act(() => {
        window.dispatchEvent(new CustomEvent("settings-updated"));
      });

      // Wait for event handling
      await waitFor(() => {
        expect(getSettings).toHaveBeenCalled();
      });

      // After the fix, getSettings should be called exactly once per event
      // (not duplicated because we reuse refreshProviderSettings)
      expect(vi.mocked(getSettings).mock.calls.length).toBe(1);
    });
  });

  describe("event listener cleanup", () => {
    it("should remove event listener on unmount", async () => {
      const { getSettings } = await import("@/lib/settings");
      createSession("session-1");

      const { InputStatusRow } = await import("./InputStatusRow");
      const { unmount } = render(<InputStatusRow sessionId="session-1" />);

      // Wait for mount
      await waitFor(() => {
        expect(getSettings).toHaveBeenCalled();
      });

      // Unmount
      unmount();

      // Clear call count
      vi.mocked(getSettings).mockClear();

      // Dispatch event after unmount
      act(() => {
        window.dispatchEvent(new CustomEvent("settings-updated"));
      });

      // Give time for any potential handler to fire
      await new Promise((resolve) => setTimeout(resolve, 50));

      // Should not have been called since component is unmounted
      expect(vi.mocked(getSettings).mock.calls.length).toBe(0);
    });
  });

  describe("refreshProviderSettings callback stability", () => {
    it("should have stable refreshProviderSettings callback", async () => {
      // This is implicit - if refreshProviderSettings changes on every render,
      // the useEffect will run on every render, causing excessive fetches.
      const { getSettings } = await import("@/lib/settings");
      createSession("session-1");

      const { InputStatusRow } = await import("./InputStatusRow");
      const { rerender } = render(<InputStatusRow sessionId="session-1" />);

      // Wait for initial mount
      await waitFor(() => {
        expect(getSettings).toHaveBeenCalled();
      });

      const callCountAfterMount = vi.mocked(getSettings).mock.calls.length;

      // Rerender the component (should not trigger additional fetches
      // if refreshProviderSettings has stable reference via useCallback)
      rerender(<InputStatusRow sessionId="session-1" />);

      // Wait a bit
      await new Promise((resolve) => setTimeout(resolve, 50));

      // Call count should not have increased from rerender
      expect(vi.mocked(getSettings).mock.calls.length).toBe(callCountAfterMount);
    });
  });
});
