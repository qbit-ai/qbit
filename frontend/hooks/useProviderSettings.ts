/**
 * Hook for managing provider settings state with automatic refresh.
 *
 * Consolidates all provider-related state (API keys, credentials, visibility)
 * into a single hook with caching and event-based refresh support.
 */

import { useCallback, useEffect, useState } from "react";
import { getOpenAiApiKey, getOpenRouterApiKey } from "@/lib/ai";
import { logger } from "@/lib/logger";
import {
  buildProviderVisibility,
  getSettings,
  getTelemetryStats,
  isLangfuseActive,
  type ProviderVisibility,
  type TelemetryStats,
} from "@/lib/settings";

// =============================================================================
// Types
// =============================================================================

/**
 * Vertex AI/Gemini credentials configuration.
 */
export interface VertexCredentials {
  credentials_path: string | null;
  project_id: string | null;
  location: string | null;
}

/**
 * Provider enablement state (has valid credentials/API key).
 */
export interface ProviderEnabledState {
  openrouter: boolean;
  openai: boolean;
  anthropic: boolean;
  ollama: boolean;
  gemini: boolean;
  groq: boolean;
  xai: boolean;
  zai_sdk: boolean;
  vertex_ai: boolean;
  vertex_gemini: boolean;
}

/**
 * Provider API keys for direct access.
 */
export interface ProviderApiKeys {
  openrouter: string | null;
  openai: string | null;
  anthropic: string | null;
  gemini: string | null;
  groq: string | null;
  xai: string | null;
  zai_sdk: string | null;
}

/**
 * Combined provider settings state.
 */
export interface ProviderSettingsState {
  /** Whether each provider is enabled (has valid credentials) */
  enabled: ProviderEnabledState;
  /** API keys for providers that use them */
  apiKeys: ProviderApiKeys;
  /** Vertex AI credentials */
  vertexAiCredentials: VertexCredentials | null;
  /** Vertex Gemini credentials */
  vertexGeminiCredentials: VertexCredentials | null;
  /** Provider visibility settings from user preferences */
  visibility: ProviderVisibility;
  /** Whether Langfuse tracing is active */
  langfuseActive: boolean;
  /** Telemetry stats if Langfuse is active */
  telemetryStats: TelemetryStats | null;
}

// =============================================================================
// Default State
// =============================================================================

const DEFAULT_ENABLED: ProviderEnabledState = {
  openrouter: false,
  openai: false,
  anthropic: false,
  ollama: true, // Ollama doesn't require an API key
  gemini: false,
  groq: false,
  xai: false,
  zai_sdk: false,
  vertex_ai: false,
  vertex_gemini: false,
};

const DEFAULT_API_KEYS: ProviderApiKeys = {
  openrouter: null,
  openai: null,
  anthropic: null,
  gemini: null,
  groq: null,
  xai: null,
  zai_sdk: null,
};

const DEFAULT_VISIBILITY: ProviderVisibility = {
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
};

const DEFAULT_STATE: ProviderSettingsState = {
  enabled: DEFAULT_ENABLED,
  apiKeys: DEFAULT_API_KEYS,
  vertexAiCredentials: null,
  vertexGeminiCredentials: null,
  visibility: DEFAULT_VISIBILITY,
  langfuseActive: false,
  telemetryStats: null,
};

// =============================================================================
// Hook
// =============================================================================

/**
 * Hook for managing provider settings with automatic refresh on settings changes.
 *
 * @returns Provider settings state and a manual refresh function
 */
export function useProviderSettings(): [ProviderSettingsState, () => Promise<void>] {
  const [state, setState] = useState<ProviderSettingsState>(DEFAULT_STATE);

  const refresh = useCallback(async () => {
    // Fetch Langfuse status and telemetry stats
    let langfuseActive = false;
    let telemetryStats: TelemetryStats | null = null;
    try {
      const [langfuseEnabled, stats] = await Promise.all([isLangfuseActive(), getTelemetryStats()]);
      langfuseActive = langfuseEnabled;
      telemetryStats = stats;
    } catch {
      // Ignore errors, keep defaults
    }

    try {
      const settings = await getSettings();

      // Build enabled state from settings
      const enabled: ProviderEnabledState = {
        openrouter: !!settings.ai.openrouter.api_key,
        openai: !!settings.ai.openai.api_key,
        anthropic: !!settings.ai.anthropic.api_key,
        ollama: true, // Ollama doesn't require an API key
        gemini: !!settings.ai.gemini.api_key,
        groq: !!settings.ai.groq.api_key,
        xai: !!settings.ai.xai.api_key,
        zai_sdk: !!settings.ai.zai_sdk?.api_key,
        vertex_ai: !!(settings.ai.vertex_ai.credentials_path || settings.ai.vertex_ai.project_id),
        vertex_gemini: !!(
          settings.ai.vertex_gemini?.credentials_path || settings.ai.vertex_gemini?.project_id
        ),
      };

      // Extract API keys
      const apiKeys: ProviderApiKeys = {
        openrouter: settings.ai.openrouter.api_key,
        openai: settings.ai.openai.api_key,
        anthropic: settings.ai.anthropic.api_key,
        gemini: settings.ai.gemini.api_key,
        groq: settings.ai.groq.api_key,
        xai: settings.ai.xai.api_key,
        zai_sdk: settings.ai.zai_sdk?.api_key ?? null,
      };

      // Extract Vertex credentials
      const vertexAiCredentials: VertexCredentials = {
        credentials_path: settings.ai.vertex_ai.credentials_path,
        project_id: settings.ai.vertex_ai.project_id,
        location: settings.ai.vertex_ai.location,
      };

      const vertexGeminiCredentials: VertexCredentials = {
        credentials_path: settings.ai.vertex_gemini?.credentials_path ?? null,
        project_id: settings.ai.vertex_gemini?.project_id ?? null,
        location: settings.ai.vertex_gemini?.location ?? null,
      };

      // Build visibility from settings
      const visibility = buildProviderVisibility(settings);

      setState({
        enabled,
        apiKeys,
        vertexAiCredentials,
        vertexGeminiCredentials,
        visibility,
        langfuseActive,
        telemetryStats,
      });
    } catch (e) {
      logger.warn("Failed to get provider settings:", e);

      // Fallback to legacy method for API keys
      try {
        const [orKey, oaiKey] = await Promise.all([getOpenRouterApiKey(), getOpenAiApiKey()]);
        setState((prev) => ({
          ...prev,
          enabled: {
            ...prev.enabled,
            openrouter: !!orKey,
            openai: !!oaiKey,
          },
          apiKeys: {
            ...prev.apiKeys,
            openrouter: orKey,
            openai: oaiKey,
          },
          langfuseActive,
          telemetryStats,
        }));
      } catch {
        // Keep defaults but update Langfuse state
        setState((prev) => ({
          ...prev,
          langfuseActive,
          telemetryStats,
        }));
      }
    }
  }, []);

  // Fetch settings on mount
  useEffect(() => {
    refresh();
  }, [refresh]);

  // Listen for settings-updated events
  useEffect(() => {
    const handleSettingsUpdated = () => {
      refresh();
    };

    window.addEventListener("settings-updated", handleSettingsUpdated);
    return () => {
      window.removeEventListener("settings-updated", handleSettingsUpdated);
    };
  }, [refresh]);

  return [state, refresh];
}
