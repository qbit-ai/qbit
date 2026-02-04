/**
 * Hook for handling AI model switching with provider configuration building.
 *
 * Extracts and consolidates the model switch logic that was previously duplicated
 * across ~150 lines in InputStatusRow. This hook:
 * - Builds provider configs from useProviderSettings state
 * - Handles session initialization
 * - Updates session state
 * - Persists model selection to project settings
 */

import { useCallback } from "react";
import type { ProviderApiKeys, VertexCredentials } from "@/hooks/useProviderSettings";
import {
  getOpenAiApiKey,
  getOpenRouterApiKey,
  initAiSession,
  type ProviderConfig,
  type ReasoningEffort,
  saveProjectModel,
} from "@/lib/ai";
import { logger } from "@/lib/logger";
import { formatModelName } from "@/lib/models";
import { notify } from "@/lib/notify";
import { useStore } from "@/store";

// Provider type as used in the model selector UI (slightly different from backend ProviderConfig)
export type ModelProvider =
  | "vertex"
  | "vertex_gemini"
  | "openrouter"
  | "openai"
  | "anthropic"
  | "ollama"
  | "gemini"
  | "groq"
  | "xai"
  | "zai_sdk";

// Maps UI provider names to store provider names
const PROVIDER_STORE_MAP: Record<ModelProvider, string> = {
  vertex: "anthropic_vertex",
  vertex_gemini: "vertex_gemini",
  openrouter: "openrouter",
  openai: "openai",
  anthropic: "anthropic",
  ollama: "ollama",
  gemini: "gemini",
  groq: "groq",
  xai: "xai",
  zai_sdk: "zai_sdk",
};

// Maps UI provider names to settings provider names (for saveProjectModel)
const PROVIDER_SETTINGS_MAP: Record<ModelProvider, string> = {
  vertex: "vertex_ai",
  vertex_gemini: "vertex_gemini",
  openrouter: "openrouter",
  openai: "openai",
  anthropic: "anthropic",
  ollama: "ollama",
  gemini: "gemini",
  groq: "groq",
  xai: "xai",
  zai_sdk: "zai_sdk",
};

interface BuildConfigParams {
  modelProvider: ModelProvider;
  modelId: string;
  workspace: string;
  apiKeys: ProviderApiKeys;
  vertexAiCredentials: VertexCredentials | null;
  vertexGeminiCredentials: VertexCredentials | null;
  /** For vertex provider, allows using session-stored credentials */
  sessionVertexConfig?: {
    credentialsPath: string;
    projectId: string;
    location: string;
  } | null;
  reasoningEffort?: ReasoningEffort;
}

/**
 * Build a ProviderConfig from provider settings state.
 * Throws if required credentials are missing.
 */
export async function buildProviderConfigFromState(
  params: BuildConfigParams
): Promise<ProviderConfig> {
  const {
    modelProvider,
    modelId,
    workspace,
    apiKeys,
    vertexAiCredentials,
    vertexGeminiCredentials,
    sessionVertexConfig,
    reasoningEffort,
  } = params;

  switch (modelProvider) {
    case "vertex": {
      // Use session config if available, otherwise use settings credentials
      const credentials = sessionVertexConfig
        ? {
            credentials_path: sessionVertexConfig.credentialsPath,
            project_id: sessionVertexConfig.projectId,
            location: sessionVertexConfig.location,
          }
        : vertexAiCredentials;

      if (!credentials?.credentials_path && !credentials?.project_id) {
        throw new Error("Vertex AI credentials not configured");
      }

      return {
        provider: "vertex_ai",
        workspace,
        model: modelId,
        credentials_path: credentials.credentials_path ?? "",
        project_id: credentials.project_id ?? "",
        location: credentials.location ?? "us-east5",
      };
    }

    case "vertex_gemini": {
      if (!vertexGeminiCredentials?.credentials_path && !vertexGeminiCredentials?.project_id) {
        throw new Error("Vertex Gemini credentials not configured");
      }

      return {
        provider: "vertex_gemini",
        workspace,
        model: modelId,
        credentials_path: vertexGeminiCredentials.credentials_path ?? "",
        project_id: vertexGeminiCredentials.project_id ?? "",
        location: vertexGeminiCredentials.location ?? "us-central1",
      };
    }

    case "openrouter": {
      const apiKey = apiKeys.openrouter ?? (await getOpenRouterApiKey());
      if (!apiKey) {
        throw new Error("OpenRouter API key not configured");
      }
      return {
        provider: "openrouter",
        workspace,
        model: modelId,
        api_key: apiKey,
      };
    }

    case "openai": {
      const apiKey = apiKeys.openai ?? (await getOpenAiApiKey());
      if (!apiKey) {
        throw new Error("OpenAI API key not configured");
      }
      return {
        provider: "openai",
        workspace,
        model: modelId,
        api_key: apiKey,
        reasoning_effort: reasoningEffort,
      };
    }

    case "anthropic": {
      const apiKey = apiKeys.anthropic;
      if (!apiKey) {
        throw new Error("Anthropic API key not configured");
      }
      return {
        provider: "anthropic",
        workspace,
        model: modelId,
        api_key: apiKey,
      };
    }

    case "ollama": {
      return {
        provider: "ollama",
        workspace,
        model: modelId,
      };
    }

    case "gemini": {
      const apiKey = apiKeys.gemini;
      if (!apiKey) {
        throw new Error("Gemini API key not configured");
      }
      return {
        provider: "gemini",
        workspace,
        model: modelId,
        api_key: apiKey,
      };
    }

    case "groq": {
      const apiKey = apiKeys.groq;
      if (!apiKey) {
        throw new Error("Groq API key not configured");
      }
      return {
        provider: "groq",
        workspace,
        model: modelId,
        api_key: apiKey,
      };
    }

    case "xai": {
      const apiKey = apiKeys.xai;
      if (!apiKey) {
        throw new Error("xAI API key not configured");
      }
      return {
        provider: "xai",
        workspace,
        model: modelId,
        api_key: apiKey,
      };
    }

    case "zai_sdk": {
      const apiKey = apiKeys.zai_sdk;
      if (!apiKey) {
        throw new Error("Z.AI SDK API key not configured");
      }
      return {
        provider: "zai_sdk",
        workspace,
        model: modelId,
        api_key: apiKey,
      };
    }

    default: {
      const _exhaustive: never = modelProvider;
      throw new Error(`Unknown provider: ${_exhaustive}`);
    }
  }
}

interface UseModelSwitchParams {
  sessionId: string;
  currentModel: string;
  currentProvider: string;
  currentReasoningEffort?: ReasoningEffort;
  apiKeys: ProviderApiKeys;
  vertexAiCredentials: VertexCredentials | null;
  vertexGeminiCredentials: VertexCredentials | null;
  /** For vertex provider, allows using session-stored credentials */
  sessionVertexConfig?: {
    workspace: string;
    credentialsPath: string;
    projectId: string;
    location: string;
  } | null;
  workspace: string;
}

interface ModelSwitchResult {
  /** The store provider name after switching */
  storeProvider: string;
  /** Vertex config to store in session (for vertex provider only) */
  vertexConfig?: {
    workspace: string;
    credentialsPath: string;
    projectId: string;
    location: string;
  };
  /** Reasoning effort (for OpenAI models only) */
  reasoningEffort?: ReasoningEffort;
}

/**
 * Hook for handling model switching with automatic config building and state updates.
 *
 * Returns a function that can be called to switch models. The function handles:
 * - Building the appropriate ProviderConfig
 * - Initializing the AI session
 * - Updating session state
 * - Persisting to project settings
 * - Error handling with notifications
 */
export function useModelSwitch(params: UseModelSwitchParams) {
  const setSessionAiConfig = useStore((state) => state.setSessionAiConfig);

  const switchModel = useCallback(
    async (
      modelId: string,
      modelProvider: ModelProvider,
      reasoningEffort?: ReasoningEffort
    ): Promise<boolean> => {
      const {
        sessionId,
        currentModel,
        currentProvider,
        currentReasoningEffort,
        apiKeys,
        vertexAiCredentials,
        vertexGeminiCredentials,
        sessionVertexConfig,
        workspace,
      } = params;

      const storeProviderName = PROVIDER_STORE_MAP[modelProvider];

      // Don't switch if already on this model
      if (currentModel === modelId && currentProvider === storeProviderName) {
        // For OpenAI, also check reasoning effort
        if (modelProvider !== "openai" || reasoningEffort === currentReasoningEffort) {
          return false;
        }
      }

      const modelName = formatModelName(modelId, reasoningEffort);

      try {
        setSessionAiConfig(sessionId, { status: "initializing", model: modelId });

        const config = await buildProviderConfigFromState({
          modelProvider,
          modelId,
          workspace,
          apiKeys,
          vertexAiCredentials,
          vertexGeminiCredentials,
          sessionVertexConfig,
          reasoningEffort,
        });

        await initAiSession(sessionId, config);

        // Build the result for state update
        const result: ModelSwitchResult = {
          storeProvider: storeProviderName,
        };

        // Store vertex config for future model switches within session
        if (modelProvider === "vertex" && config.provider === "vertex_ai") {
          result.vertexConfig = {
            workspace,
            credentialsPath: config.credentials_path ?? "",
            projectId: config.project_id,
            location: config.location,
          };
        }

        if (modelProvider === "openai" && reasoningEffort) {
          result.reasoningEffort = reasoningEffort;
        }

        // Update session state
        setSessionAiConfig(sessionId, {
          status: "ready",
          provider: result.storeProvider,
          ...(result.vertexConfig && { vertexConfig: result.vertexConfig }),
          ...(result.reasoningEffort && { reasoningEffort: result.reasoningEffort }),
        });

        notify.success(`Switched to ${modelName}`);

        // Save model selection to per-project settings
        try {
          const providerForSettings = PROVIDER_SETTINGS_MAP[modelProvider];
          await saveProjectModel(workspace, providerForSettings, modelId);
        } catch (saveError) {
          // Don't fail the switch if saving settings fails
          logger.warn("Failed to save project model settings:", saveError);
        }

        return true;
      } catch (error) {
        logger.error("Failed to switch model:", error);
        setSessionAiConfig(sessionId, {
          status: "error",
          errorMessage: error instanceof Error ? error.message : "Failed to switch model",
        });
        notify.error(`Failed to switch to ${modelName}`);
        return false;
      }
    },
    [params, setSessionAiConfig]
  );

  return switchModel;
}
