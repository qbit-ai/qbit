/**
 * Tests for the Project Settings API
 *
 * These tests verify the TypeScript interface for per-project settings,
 * including type correctness and function signatures.
 */

import { beforeEach, describe, expect, it, vi } from "vitest";
import type { AgentMode, AiProvider, ProjectSettings } from "./ai";

// Mock the Tauri invoke function
vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

import { invoke } from "@tauri-apps/api/core";
import { getProjectSettings, saveProjectAgentMode, saveProjectModel, setAgentMode } from "./ai";

const mockInvoke = invoke as ReturnType<typeof vi.fn>;

describe("Project Settings API", () => {
  beforeEach(() => {
    mockInvoke.mockReset();
  });

  describe("getProjectSettings", () => {
    it("should call invoke with correct command and parameters", async () => {
      const mockSettings: ProjectSettings = {
        provider: "anthropic",
        model: "claude-sonnet-4-20250514",
        agent_mode: "auto-approve",
      };
      mockInvoke.mockResolvedValueOnce(mockSettings);

      const result = await getProjectSettings("/path/to/workspace");

      expect(mockInvoke).toHaveBeenCalledWith("get_project_settings", {
        workspace: "/path/to/workspace",
      });
      expect(result).toEqual(mockSettings);
    });

    it("should handle null values in response", async () => {
      const mockSettings: ProjectSettings = {
        provider: null,
        model: null,
        agent_mode: null,
      };
      mockInvoke.mockResolvedValueOnce(mockSettings);

      const result = await getProjectSettings("/path/to/workspace");

      expect(result.provider).toBeNull();
      expect(result.model).toBeNull();
      expect(result.agent_mode).toBeNull();
    });

    it("should handle partial settings", async () => {
      const mockSettings: ProjectSettings = {
        provider: "openai",
        model: "gpt-4",
        agent_mode: null,
      };
      mockInvoke.mockResolvedValueOnce(mockSettings);

      const result = await getProjectSettings("/path/to/workspace");

      expect(result.provider).toBe("openai");
      expect(result.model).toBe("gpt-4");
      expect(result.agent_mode).toBeNull();
    });
  });

  describe("saveProjectModel", () => {
    it("should call invoke with correct command and parameters", async () => {
      mockInvoke.mockResolvedValueOnce(undefined);

      await saveProjectModel("/path/to/workspace", "anthropic", "claude-sonnet-4-20250514");

      expect(mockInvoke).toHaveBeenCalledWith("save_project_model", {
        workspace: "/path/to/workspace",
        provider: "anthropic",
        model: "claude-sonnet-4-20250514",
      });
    });

    it("should work with all provider types", async () => {
      const providers: AiProvider[] = [
        "vertex_ai",
        "openrouter",
        "anthropic",
        "openai",
        "ollama",
        "gemini",
        "groq",
        "xai",
        "zai",
      ];

      for (const provider of providers) {
        mockInvoke.mockResolvedValueOnce(undefined);
        await saveProjectModel("/workspace", provider, "test-model");
        expect(mockInvoke).toHaveBeenLastCalledWith("save_project_model", {
          workspace: "/workspace",
          provider,
          model: "test-model",
        });
      }
    });
  });

  describe("saveProjectAgentMode", () => {
    it("should call invoke with correct command and parameters", async () => {
      mockInvoke.mockResolvedValueOnce(undefined);

      await saveProjectAgentMode("/path/to/workspace", "planning");

      expect(mockInvoke).toHaveBeenCalledWith("save_project_agent_mode", {
        workspace: "/path/to/workspace",
        mode: "planning",
      });
    });

    it("should work with all agent modes", async () => {
      const modes: AgentMode[] = ["default", "auto-approve", "planning"];

      for (const mode of modes) {
        mockInvoke.mockResolvedValueOnce(undefined);
        await saveProjectAgentMode("/workspace", mode);
        expect(mockInvoke).toHaveBeenLastCalledWith("save_project_agent_mode", {
          workspace: "/workspace",
          mode,
        });
      }
    });
  });

  describe("setAgentMode with workspace persistence", () => {
    it("should pass workspace to backend when provided", async () => {
      mockInvoke.mockResolvedValueOnce(undefined);

      await setAgentMode("session-123", "auto-approve", "/path/to/workspace");

      expect(mockInvoke).toHaveBeenCalledWith("set_agent_mode", {
        sessionId: "session-123",
        mode: "auto-approve",
        workspace: "/path/to/workspace",
      });
    });

    it("should work without workspace parameter", async () => {
      mockInvoke.mockResolvedValueOnce(undefined);

      await setAgentMode("session-123", "planning");

      expect(mockInvoke).toHaveBeenCalledWith("set_agent_mode", {
        sessionId: "session-123",
        mode: "planning",
        workspace: undefined,
      });
    });
  });
});

describe("ProjectSettings type", () => {
  it("should correctly type provider values", () => {
    const settings: ProjectSettings = {
      provider: "anthropic",
      model: "claude-sonnet-4-20250514",
      agent_mode: "default",
    };

    // TypeScript compilation ensures these are valid
    expect(settings.provider).toBe("anthropic");
  });

  it("should correctly type agent_mode values", () => {
    const settings: ProjectSettings = {
      provider: null,
      model: null,
      agent_mode: "auto-approve",
    };

    expect(settings.agent_mode).toBe("auto-approve");
  });

  it("should allow all null values", () => {
    const settings: ProjectSettings = {
      provider: null,
      model: null,
      agent_mode: null,
    };

    expect(settings).toBeDefined();
  });
});
