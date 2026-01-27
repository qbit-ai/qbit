/**
 * Shared model configuration - single source of truth for all model selectors
 *
 * This module provides both static model definitions (for immediate use) and
 * functions to fetch dynamic model data from the backend registry.
 */

import type { ReasoningEffort } from "./ai";
import {
  ANTHROPIC_MODELS,
  GEMINI_MODELS,
  GROQ_MODELS,
  OLLAMA_MODELS,
  OPENAI_MODELS,
  VERTEX_AI_MODELS,
  VERTEX_GEMINI_MODELS,
  XAI_MODELS,
  ZAI_SDK_MODELS,
} from "./ai";
import {
  type AiProvider as BackendAiProvider,
  getAvailableModels,
  getModelCapabilities,
  getProviders,
  type ModelCapabilities,
  type OwnedModelDefinition,
  type ProviderInfo,
} from "./model-registry";
import type { AiProvider } from "./settings";

export interface ModelInfo {
  id: string;
  name: string;
  reasoningEffort?: ReasoningEffort;
}

/**
 * A model entry that can either be a simple model or a group with sub-options.
 * Supports recursive nesting (e.g., "GPT-5 Series" → "GPT 5.2" → Low/Medium/High).
 */
export interface ModelEntry {
  /** Display name for the model or group */
  name: string;
  /** Model ID (for leaf models) */
  id?: string;
  /** Reasoning effort (for leaf models with reasoning) */
  reasoningEffort?: ReasoningEffort;
  /** Sub-options (supports recursive nesting) */
  subModels?: ModelEntry[];
}

export interface ProviderGroup {
  provider: AiProvider;
  providerName: string;
  icon: string;
  models: ModelInfo[];
}

/**
 * Provider group with nested model entries for sub-menus.
 */
export interface ProviderGroupNested {
  provider: AiProvider;
  providerName: string;
  icon: string;
  models: ModelEntry[];
}

/**
 * All available providers and their models.
 * Sorted alphabetically by provider name.
 */
export const PROVIDER_GROUPS: ProviderGroup[] = [
  {
    provider: "anthropic",
    providerName: "Anthropic",
    icon: "🔶",
    models: [
      { id: ANTHROPIC_MODELS.CLAUDE_OPUS_4_5, name: "Claude Opus 4.5" },
      { id: ANTHROPIC_MODELS.CLAUDE_SONNET_4_5, name: "Claude Sonnet 4.5" },
      { id: ANTHROPIC_MODELS.CLAUDE_HAIKU_4_5, name: "Claude Haiku 4.5" },
    ],
  },
  {
    provider: "gemini",
    providerName: "Gemini",
    icon: "💎",
    models: [
      { id: GEMINI_MODELS.GEMINI_3_PRO_PREVIEW, name: "Gemini 3 Pro Preview" },
      { id: GEMINI_MODELS.GEMINI_2_5_PRO, name: "Gemini 2.5 Pro" },
      { id: GEMINI_MODELS.GEMINI_2_5_FLASH, name: "Gemini 2.5 Flash" },
      {
        id: GEMINI_MODELS.GEMINI_2_5_FLASH_LITE,
        name: "Gemini 2.5 Flash Lite",
      },
    ],
  },
  {
    provider: "groq",
    providerName: "Groq",
    icon: "⚡",
    models: [
      { id: GROQ_MODELS.LLAMA_4_SCOUT, name: "Llama 4 Scout 17B" },
      { id: GROQ_MODELS.LLAMA_4_MAVERICK, name: "Llama 4 Maverick 17B" },
      { id: GROQ_MODELS.LLAMA_3_3_70B, name: "Llama 3.3 70B" },
      { id: GROQ_MODELS.LLAMA_3_1_8B, name: "Llama 3.1 8B Instant" },
      { id: GROQ_MODELS.GPT_OSS_120B, name: "GPT OSS 120B" },
      { id: GROQ_MODELS.GPT_OSS_20B, name: "GPT OSS 20B" },
    ],
  },
  {
    provider: "ollama",
    providerName: "Ollama",
    icon: "🦙",
    models: [
      { id: OLLAMA_MODELS.LLAMA_3_2, name: "Llama 3.2" },
      { id: OLLAMA_MODELS.LLAMA_3_1, name: "Llama 3.1" },
      { id: OLLAMA_MODELS.QWEN_2_5, name: "Qwen 2.5" },
      { id: OLLAMA_MODELS.MISTRAL, name: "Mistral" },
      { id: OLLAMA_MODELS.CODELLAMA, name: "CodeLlama" },
    ],
  },
  {
    provider: "openai",
    providerName: "OpenAI",
    icon: "⚪",
    models: [
      // GPT-5 series (with reasoning effort variants)
      {
        id: OPENAI_MODELS.GPT_5_2,
        name: "GPT 5.2 (Low)",
        reasoningEffort: "low",
      },
      {
        id: OPENAI_MODELS.GPT_5_2,
        name: "GPT 5.2 (Medium)",
        reasoningEffort: "medium",
      },
      {
        id: OPENAI_MODELS.GPT_5_2,
        name: "GPT 5.2 (High)",
        reasoningEffort: "high",
      },
      {
        id: OPENAI_MODELS.GPT_5_2,
        name: "GPT 5.2 (Extra High)",
        reasoningEffort: "high",
      },
      {
        id: OPENAI_MODELS.GPT_5_1,
        name: "GPT 5.1 (Low)",
        reasoningEffort: "low",
      },
      {
        id: OPENAI_MODELS.GPT_5_1,
        name: "GPT 5.1 (Medium)",
        reasoningEffort: "medium",
      },
      {
        id: OPENAI_MODELS.GPT_5_1,
        name: "GPT 5.1 (High)",
        reasoningEffort: "high",
      },
      { id: OPENAI_MODELS.GPT_5, name: "GPT 5 (Low)", reasoningEffort: "low" },
      {
        id: OPENAI_MODELS.GPT_5,
        name: "GPT 5 (Medium)",
        reasoningEffort: "medium",
      },
      {
        id: OPENAI_MODELS.GPT_5,
        name: "GPT 5 (High)",
        reasoningEffort: "high",
      },
      { id: OPENAI_MODELS.GPT_5_MINI, name: "GPT 5 Mini" },
      { id: OPENAI_MODELS.GPT_5_NANO, name: "GPT 5 Nano" },
      // GPT-4.1 series
      { id: OPENAI_MODELS.GPT_4_1, name: "GPT 4.1" },
      { id: OPENAI_MODELS.GPT_4_1_MINI, name: "GPT 4.1 Mini" },
      { id: OPENAI_MODELS.GPT_4_1_NANO, name: "GPT 4.1 Nano" },
      // GPT-4o series
      { id: OPENAI_MODELS.GPT_4O, name: "GPT 4o" },
      { id: OPENAI_MODELS.GPT_4O_MINI, name: "GPT 4o Mini" },
      { id: OPENAI_MODELS.CHATGPT_4O_LATEST, name: "ChatGPT 4o Latest" },
      // o-series reasoning models (with reasoning effort variants)
      {
        id: OPENAI_MODELS.O4_MINI,
        name: "o4 Mini (Low)",
        reasoningEffort: "low",
      },
      {
        id: OPENAI_MODELS.O4_MINI,
        name: "o4 Mini (Medium)",
        reasoningEffort: "medium",
      },
      {
        id: OPENAI_MODELS.O4_MINI,
        name: "o4 Mini (High)",
        reasoningEffort: "high",
      },
      { id: OPENAI_MODELS.O3, name: "o3 (Low)", reasoningEffort: "low" },
      { id: OPENAI_MODELS.O3, name: "o3 (Medium)", reasoningEffort: "medium" },
      { id: OPENAI_MODELS.O3, name: "o3 (High)", reasoningEffort: "high" },
      {
        id: OPENAI_MODELS.O3_MINI,
        name: "o3 Mini (Low)",
        reasoningEffort: "low",
      },
      {
        id: OPENAI_MODELS.O3_MINI,
        name: "o3 Mini (Medium)",
        reasoningEffort: "medium",
      },
      {
        id: OPENAI_MODELS.O3_MINI,
        name: "o3 Mini (High)",
        reasoningEffort: "high",
      },
      { id: OPENAI_MODELS.O1, name: "o1 (Low)", reasoningEffort: "low" },
      { id: OPENAI_MODELS.O1, name: "o1 (Medium)", reasoningEffort: "medium" },
      { id: OPENAI_MODELS.O1, name: "o1 (High)", reasoningEffort: "high" },
      // Codex models (coding-optimized)
      {
        id: OPENAI_MODELS.GPT_5_2_CODEX,
        name: "GPT 5.2 Codex (Low)",
        reasoningEffort: "low",
      },
      {
        id: OPENAI_MODELS.GPT_5_2_CODEX,
        name: "GPT 5.2 Codex (Medium)",
        reasoningEffort: "medium",
      },
      {
        id: OPENAI_MODELS.GPT_5_2_CODEX,
        name: "GPT 5.2 Codex (High)",
        reasoningEffort: "high",
      },
      {
        id: OPENAI_MODELS.GPT_5_2_CODEX,
        name: "GPT 5.2 Codex (Extra High)",
        reasoningEffort: "high",
      },
      {
        id: OPENAI_MODELS.GPT_5_1_CODEX,
        name: "GPT 5.1 Codex (Low)",
        reasoningEffort: "low",
      },
      {
        id: OPENAI_MODELS.GPT_5_1_CODEX,
        name: "GPT 5.1 Codex (Medium)",
        reasoningEffort: "medium",
      },
      {
        id: OPENAI_MODELS.GPT_5_1_CODEX,
        name: "GPT 5.1 Codex (High)",
        reasoningEffort: "high",
      },
      {
        id: OPENAI_MODELS.GPT_5_1_CODEX,
        name: "GPT 5.1 Codex (Extra High)",
        reasoningEffort: "high",
      },
      {
        id: OPENAI_MODELS.GPT_5_1_CODEX_MAX,
        name: "GPT 5.1 Codex Max (Low)",
        reasoningEffort: "low",
      },
      {
        id: OPENAI_MODELS.GPT_5_1_CODEX_MAX,
        name: "GPT 5.1 Codex Max (Medium)",
        reasoningEffort: "medium",
      },
      {
        id: OPENAI_MODELS.GPT_5_1_CODEX_MAX,
        name: "GPT 5.1 Codex Max (High)",
        reasoningEffort: "high",
      },
      {
        id: OPENAI_MODELS.GPT_5_1_CODEX_MAX,
        name: "GPT 5.1 Codex Max (Extra High)",
        reasoningEffort: "high",
      },
      {
        id: OPENAI_MODELS.GPT_5_1_CODEX_MINI,
        name: "GPT 5.1 Codex Mini (Low)",
        reasoningEffort: "low",
      },
      {
        id: OPENAI_MODELS.GPT_5_1_CODEX_MINI,
        name: "GPT 5.1 Codex Mini (Medium)",
        reasoningEffort: "medium",
      },
      {
        id: OPENAI_MODELS.GPT_5_1_CODEX_MINI,
        name: "GPT 5.1 Codex Mini (High)",
        reasoningEffort: "high",
      },
    ],
  },
  {
    provider: "openrouter",
    providerName: "OpenRouter",
    icon: "🔀",
    models: [
      { id: "mistralai/devstral-2512", name: "Devstral 2512" },
      { id: "deepseek/deepseek-v3.2", name: "Deepseek v3.2" },
      { id: "z-ai/glm-4.6", name: "GLM 4.6" },
      { id: "x-ai/grok-code-fast-1", name: "Grok Code Fast 1" },
      { id: "openai/gpt-oss-20b", name: "GPT OSS 20B" },
      { id: "openai/gpt-oss-120b", name: "GPT OSS 120B" },
      { id: "openai/gpt-5.2", name: "GPT 5.2" },
    ],
  },
  {
    provider: "vertex_ai",
    providerName: "Vertex AI",
    icon: "🔷",
    models: [
      { id: VERTEX_AI_MODELS.CLAUDE_OPUS_4_5, name: "Claude Opus 4.5" },
      { id: VERTEX_AI_MODELS.CLAUDE_SONNET_4_5, name: "Claude Sonnet 4.5" },
      { id: VERTEX_AI_MODELS.CLAUDE_HAIKU_4_5, name: "Claude Haiku 4.5" },
    ],
  },
  {
    provider: "vertex_gemini",
    providerName: "Vertex AI Gemini",
    icon: "💎",
    models: [
      { id: VERTEX_GEMINI_MODELS.GEMINI_2_5_PRO, name: "Gemini 2.5 Pro" },
      { id: VERTEX_GEMINI_MODELS.GEMINI_2_5_FLASH, name: "Gemini 2.5 Flash" },
      { id: VERTEX_GEMINI_MODELS.GEMINI_2_5_FLASH_LITE, name: "Gemini 2.5 Flash Lite" },
      { id: VERTEX_GEMINI_MODELS.GEMINI_2_0_FLASH, name: "Gemini 2.0 Flash" },
      { id: VERTEX_GEMINI_MODELS.GEMINI_2_0_FLASH_LITE, name: "Gemini 2.0 Flash Lite" },
    ],
  },
  {
    provider: "xai",
    providerName: "xAI",
    icon: "𝕏",
    models: [
      {
        id: XAI_MODELS.GROK_4_1_FAST_REASONING,
        name: "Grok 4.1 Fast (Reasoning)",
      },
      { id: XAI_MODELS.GROK_4_1_FAST_NON_REASONING, name: "Grok 4.1 Fast" },
      { id: XAI_MODELS.GROK_4_FAST_REASONING, name: "Grok 4 (Reasoning)" },
      { id: XAI_MODELS.GROK_4_FAST_NON_REASONING, name: "Grok 4" },
      { id: XAI_MODELS.GROK_CODE_FAST_1, name: "Grok Code" },
    ],
  },
  {
    provider: "zai_sdk",
    providerName: "Z.AI SDK",
    icon: "🤖",
    models: [
      { id: ZAI_SDK_MODELS.GLM_4_7, name: "GLM 4.7" },
      { id: ZAI_SDK_MODELS.GLM_4_6V, name: "GLM 4.6v" },
      { id: ZAI_SDK_MODELS.GLM_4_5_AIR, name: "GLM 4.5 Air" },
      { id: ZAI_SDK_MODELS.GLM_4_FLASH, name: "GLM 4 Flash" },
    ],
  },
];

/**
 * Nested provider groups with sub-menus for models with reasoning effort variants.
 * Use this for the ModelSelector UI to show compact grouped menus.
 */
export const PROVIDER_GROUPS_NESTED: ProviderGroupNested[] = [
  {
    provider: "anthropic",
    providerName: "Anthropic",
    icon: "🔶",
    models: [
      { id: ANTHROPIC_MODELS.CLAUDE_OPUS_4_5, name: "Claude Opus 4.5" },
      { id: ANTHROPIC_MODELS.CLAUDE_SONNET_4_5, name: "Claude Sonnet 4.5" },
      { id: ANTHROPIC_MODELS.CLAUDE_HAIKU_4_5, name: "Claude Haiku 4.5" },
    ],
  },
  {
    provider: "gemini",
    providerName: "Gemini",
    icon: "💎",
    models: [
      { id: GEMINI_MODELS.GEMINI_3_PRO_PREVIEW, name: "Gemini 3 Pro Preview" },
      { id: GEMINI_MODELS.GEMINI_2_5_PRO, name: "Gemini 2.5 Pro" },
      { id: GEMINI_MODELS.GEMINI_2_5_FLASH, name: "Gemini 2.5 Flash" },
      {
        id: GEMINI_MODELS.GEMINI_2_5_FLASH_LITE,
        name: "Gemini 2.5 Flash Lite",
      },
    ],
  },
  {
    provider: "groq",
    providerName: "Groq",
    icon: "⚡",
    models: [
      { id: GROQ_MODELS.LLAMA_4_SCOUT, name: "Llama 4 Scout 17B" },
      { id: GROQ_MODELS.LLAMA_4_MAVERICK, name: "Llama 4 Maverick 17B" },
      { id: GROQ_MODELS.LLAMA_3_3_70B, name: "Llama 3.3 70B" },
      { id: GROQ_MODELS.LLAMA_3_1_8B, name: "Llama 3.1 8B Instant" },
      { id: GROQ_MODELS.GPT_OSS_120B, name: "GPT OSS 120B" },
      { id: GROQ_MODELS.GPT_OSS_20B, name: "GPT OSS 20B" },
    ],
  },
  {
    provider: "ollama",
    providerName: "Ollama",
    icon: "🦙",
    models: [
      { id: OLLAMA_MODELS.LLAMA_3_2, name: "Llama 3.2" },
      { id: OLLAMA_MODELS.LLAMA_3_1, name: "Llama 3.1" },
      { id: OLLAMA_MODELS.QWEN_2_5, name: "Qwen 2.5" },
      { id: OLLAMA_MODELS.MISTRAL, name: "Mistral" },
      { id: OLLAMA_MODELS.CODELLAMA, name: "CodeLlama" },
    ],
  },
  {
    provider: "openai",
    providerName: "OpenAI",
    icon: "⚪",
    models: [
      // GPT-5 series grouped with 3-level nesting for reasoning effort
      {
        name: "GPT-5 Series",
        subModels: [
          {
            name: "GPT 5.2",
            subModels: [
              {
                id: OPENAI_MODELS.GPT_5_2,
                name: "Low",
                reasoningEffort: "low",
              },
              {
                id: OPENAI_MODELS.GPT_5_2,
                name: "Medium",
                reasoningEffort: "medium",
              },
              {
                id: OPENAI_MODELS.GPT_5_2,
                name: "High",
                reasoningEffort: "high",
              },
              {
                id: OPENAI_MODELS.GPT_5_2,
                name: "Extra High",
                reasoningEffort: "high",
              },
            ],
          },
          {
            name: "GPT 5.1",
            subModels: [
              {
                id: OPENAI_MODELS.GPT_5_1,
                name: "Low",
                reasoningEffort: "low",
              },
              {
                id: OPENAI_MODELS.GPT_5_1,
                name: "Medium",
                reasoningEffort: "medium",
              },
              {
                id: OPENAI_MODELS.GPT_5_1,
                name: "High",
                reasoningEffort: "high",
              },
            ],
          },
          {
            name: "GPT 5",
            subModels: [
              { id: OPENAI_MODELS.GPT_5, name: "Low", reasoningEffort: "low" },
              {
                id: OPENAI_MODELS.GPT_5,
                name: "Medium",
                reasoningEffort: "medium",
              },
              {
                id: OPENAI_MODELS.GPT_5,
                name: "High",
                reasoningEffort: "high",
              },
            ],
          },
          { id: OPENAI_MODELS.GPT_5_MINI, name: "GPT 5 Mini" },
          { id: OPENAI_MODELS.GPT_5_NANO, name: "GPT 5 Nano" },
        ],
      },
      // GPT-4 series grouped (no reasoning effort needed)
      {
        name: "GPT-4 Series",
        subModels: [
          { id: OPENAI_MODELS.GPT_4_1, name: "GPT 4.1" },
          { id: OPENAI_MODELS.GPT_4_1_MINI, name: "GPT 4.1 Mini" },
          { id: OPENAI_MODELS.GPT_4_1_NANO, name: "GPT 4.1 Nano" },
          { id: OPENAI_MODELS.GPT_4O, name: "GPT 4o" },
          { id: OPENAI_MODELS.GPT_4O_MINI, name: "GPT 4o Mini" },
          { id: OPENAI_MODELS.CHATGPT_4O_LATEST, name: "ChatGPT 4o Latest" },
        ],
      },
      // o-series reasoning models with 3-level nesting
      {
        name: "o-Series",
        subModels: [
          {
            name: "o4 Mini",
            subModels: [
              {
                id: OPENAI_MODELS.O4_MINI,
                name: "Low",
                reasoningEffort: "low",
              },
              {
                id: OPENAI_MODELS.O4_MINI,
                name: "Medium",
                reasoningEffort: "medium",
              },
              {
                id: OPENAI_MODELS.O4_MINI,
                name: "High",
                reasoningEffort: "high",
              },
            ],
          },
          {
            name: "o3",
            subModels: [
              { id: OPENAI_MODELS.O3, name: "Low", reasoningEffort: "low" },
              {
                id: OPENAI_MODELS.O3,
                name: "Medium",
                reasoningEffort: "medium",
              },
              { id: OPENAI_MODELS.O3, name: "High", reasoningEffort: "high" },
            ],
          },
          {
            name: "o3 Mini",
            subModels: [
              {
                id: OPENAI_MODELS.O3_MINI,
                name: "Low",
                reasoningEffort: "low",
              },
              {
                id: OPENAI_MODELS.O3_MINI,
                name: "Medium",
                reasoningEffort: "medium",
              },
              {
                id: OPENAI_MODELS.O3_MINI,
                name: "High",
                reasoningEffort: "high",
              },
            ],
          },
          {
            name: "o1",
            subModels: [
              { id: OPENAI_MODELS.O1, name: "Low", reasoningEffort: "low" },
              {
                id: OPENAI_MODELS.O1,
                name: "Medium",
                reasoningEffort: "medium",
              },
              { id: OPENAI_MODELS.O1, name: "High", reasoningEffort: "high" },
            ],
          },
        ],
      },
      // Codex models grouped
      {
        name: "Codex",
        subModels: [
          {
            name: "GPT 5.2 Codex",
            subModels: [
              {
                id: OPENAI_MODELS.GPT_5_2_CODEX,
                name: "Low",
                reasoningEffort: "low",
              },
              {
                id: OPENAI_MODELS.GPT_5_2_CODEX,
                name: "Medium",
                reasoningEffort: "medium",
              },
              {
                id: OPENAI_MODELS.GPT_5_2_CODEX,
                name: "High",
                reasoningEffort: "high",
              },
              {
                id: OPENAI_MODELS.GPT_5_2_CODEX,
                name: "Extra High",
                reasoningEffort: "high",
              },
            ],
          },
          {
            name: "GPT 5.1 Codex",
            subModels: [
              {
                id: OPENAI_MODELS.GPT_5_1_CODEX,
                name: "Low",
                reasoningEffort: "low",
              },
              {
                id: OPENAI_MODELS.GPT_5_1_CODEX,
                name: "Medium",
                reasoningEffort: "medium",
              },
              {
                id: OPENAI_MODELS.GPT_5_1_CODEX,
                name: "High",
                reasoningEffort: "high",
              },
              {
                id: OPENAI_MODELS.GPT_5_1_CODEX,
                name: "Extra High",
                reasoningEffort: "high",
              },
            ],
          },
          {
            name: "GPT 5.1 Codex Max",
            subModels: [
              {
                id: OPENAI_MODELS.GPT_5_1_CODEX_MAX,
                name: "Low",
                reasoningEffort: "low",
              },
              {
                id: OPENAI_MODELS.GPT_5_1_CODEX_MAX,
                name: "Medium",
                reasoningEffort: "medium",
              },
              {
                id: OPENAI_MODELS.GPT_5_1_CODEX_MAX,
                name: "High",
                reasoningEffort: "high",
              },
              {
                id: OPENAI_MODELS.GPT_5_1_CODEX_MAX,
                name: "Extra High",
                reasoningEffort: "high",
              },
            ],
          },
          {
            name: "GPT 5.1 Codex Mini",
            subModels: [
              {
                id: OPENAI_MODELS.GPT_5_1_CODEX_MINI,
                name: "Low",
                reasoningEffort: "low",
              },
              {
                id: OPENAI_MODELS.GPT_5_1_CODEX_MINI,
                name: "Medium",
                reasoningEffort: "medium",
              },
              {
                id: OPENAI_MODELS.GPT_5_1_CODEX_MINI,
                name: "High",
                reasoningEffort: "high",
              },
            ],
          },
        ],
      },
    ],
  },
  {
    provider: "openrouter",
    providerName: "OpenRouter",
    icon: "🔀",
    models: [
      { id: "mistralai/devstral-2512", name: "Devstral 2512" },
      { id: "deepseek/deepseek-v3.2", name: "Deepseek v3.2" },
      { id: "z-ai/glm-4.6", name: "GLM 4.6" },
      { id: "x-ai/grok-code-fast-1", name: "Grok Code Fast 1" },
      { id: "openai/gpt-oss-20b", name: "GPT OSS 20B" },
      { id: "openai/gpt-oss-120b", name: "GPT OSS 120B" },
      { id: "openai/gpt-5.2", name: "GPT 5.2" },
    ],
  },
  {
    provider: "vertex_ai",
    providerName: "Vertex AI",
    icon: "🔷",
    models: [
      { id: VERTEX_AI_MODELS.CLAUDE_OPUS_4_5, name: "Claude Opus 4.5" },
      { id: VERTEX_AI_MODELS.CLAUDE_SONNET_4_5, name: "Claude Sonnet 4.5" },
      { id: VERTEX_AI_MODELS.CLAUDE_HAIKU_4_5, name: "Claude Haiku 4.5" },
    ],
  },
  {
    provider: "vertex_gemini",
    providerName: "Vertex AI Gemini",
    icon: "💎",
    models: [
      { id: VERTEX_GEMINI_MODELS.GEMINI_3_PRO_PREVIEW, name: "Gemini 3 Pro (Preview)" },
      { id: VERTEX_GEMINI_MODELS.GEMINI_3_FLASH_PREVIEW, name: "Gemini 3 Flash (Preview)" },
      { id: VERTEX_GEMINI_MODELS.GEMINI_2_5_PRO, name: "Gemini 2.5 Pro" },
      { id: VERTEX_GEMINI_MODELS.GEMINI_2_5_FLASH, name: "Gemini 2.5 Flash" },
      { id: VERTEX_GEMINI_MODELS.GEMINI_2_5_FLASH_LITE, name: "Gemini 2.5 Flash Lite" },
      { id: VERTEX_GEMINI_MODELS.GEMINI_2_0_FLASH, name: "Gemini 2.0 Flash" },
      { id: VERTEX_GEMINI_MODELS.GEMINI_2_0_FLASH_LITE, name: "Gemini 2.0 Flash Lite" },
    ],
  },
  {
    provider: "xai",
    providerName: "xAI",
    icon: "𝕏",
    models: [
      {
        id: XAI_MODELS.GROK_4_1_FAST_REASONING,
        name: "Grok 4.1 Fast (Reasoning)",
      },
      { id: XAI_MODELS.GROK_4_1_FAST_NON_REASONING, name: "Grok 4.1 Fast" },
      { id: XAI_MODELS.GROK_4_FAST_REASONING, name: "Grok 4 (Reasoning)" },
      { id: XAI_MODELS.GROK_4_FAST_NON_REASONING, name: "Grok 4" },
      { id: XAI_MODELS.GROK_CODE_FAST_1, name: "Grok Code" },
    ],
  },
  {
    provider: "zai_sdk",
    providerName: "Z.AI SDK",
    icon: "🤖",
    models: [
      { id: ZAI_SDK_MODELS.GLM_4_7, name: "GLM 4.7" },
      { id: ZAI_SDK_MODELS.GLM_4_6V, name: "GLM 4.6v" },
      { id: ZAI_SDK_MODELS.GLM_4_5_AIR, name: "GLM 4.5 Air" },
      { id: ZAI_SDK_MODELS.GLM_4_FLASH, name: "GLM 4 Flash" },
    ],
  },
];

/**
 * Get a provider group by provider ID
 */
export function getProviderGroup(provider: AiProvider): ProviderGroup | undefined {
  return PROVIDER_GROUPS.find((g) => g.provider === provider);
}

/**
 * Get a nested provider group by provider ID
 */
export function getProviderGroupNested(provider: AiProvider): ProviderGroupNested | undefined {
  return PROVIDER_GROUPS_NESTED.find((g) => g.provider === provider);
}

/**
 * Get all models as a flat list
 */
export function getAllModels(): (ModelInfo & { provider: AiProvider })[] {
  return PROVIDER_GROUPS.flatMap((group) =>
    group.models.map((model) => ({ ...model, provider: group.provider }))
  );
}

/**
 * Find a model by ID across all providers
 */
export function findModelById(
  modelId: string,
  reasoningEffort?: ReasoningEffort
): (ModelInfo & { provider: AiProvider; providerName: string }) | undefined {
  for (const group of PROVIDER_GROUPS) {
    const model = group.models.find(
      (m) =>
        m.id === modelId && (reasoningEffort === undefined || m.reasoningEffort === reasoningEffort)
    );
    if (model) {
      return {
        ...model,
        provider: group.provider,
        providerName: group.providerName,
      };
    }
  }
  return undefined;
}

/**
 * Format a model ID to a display name
 */
export function formatModelName(modelId: string, reasoningEffort?: ReasoningEffort): string {
  if (!modelId) return "No Model";

  const model = findModelById(modelId, reasoningEffort);
  if (model) return model.name;

  // Fallback: try to find by ID only (for cases where reasoning effort doesn't match)
  const anyModel = findModelById(modelId);
  if (anyModel) {
    // For OpenAI, append reasoning effort if provided
    if (anyModel.provider === "openai" && reasoningEffort) {
      return `GPT 5.2 (${reasoningEffort.charAt(0).toUpperCase() + reasoningEffort.slice(1)})`;
    }
    return anyModel.name;
  }

  return modelId;
}

// =============================================================================
// Backend Model Registry Integration
// =============================================================================

// Re-export types from model-registry for convenience
export type { ModelCapabilities, OwnedModelDefinition, ProviderInfo };

/**
 * Fetch all models from the backend registry.
 * Returns models grouped by provider in the ProviderGroup format.
 */
export async function fetchProviderGroups(): Promise<ProviderGroup[]> {
  const [backendModels, backendProviders] = await Promise.all([
    getAvailableModels(),
    getProviders(),
  ]);

  // Create a map of provider info for quick lookup
  const providerInfoMap = new Map<string, ProviderInfo>();
  for (const p of backendProviders) {
    providerInfoMap.set(p.provider, p);
  }

  // Group models by provider
  const grouped = new Map<string, OwnedModelDefinition[]>();
  for (const model of backendModels) {
    const existing = grouped.get(model.provider) ?? [];
    existing.push(model);
    grouped.set(model.provider, existing);
  }

  // Convert to ProviderGroup format
  const groups: ProviderGroup[] = [];
  for (const [provider, models] of grouped) {
    const info = providerInfoMap.get(provider);
    if (!info) continue;

    groups.push({
      provider: provider as AiProvider,
      providerName: info.name,
      icon: info.icon,
      models: models.map((m) => ({
        id: m.id,
        name: m.display_name,
      })),
    });
  }

  // Sort alphabetically by provider name
  groups.sort((a, b) => a.providerName.localeCompare(b.providerName));

  return groups;
}

/**
 * Fetch models for a specific provider from the backend.
 */
export async function fetchModelsForProvider(provider: AiProvider): Promise<ModelInfo[]> {
  const models = await getAvailableModels(provider as BackendAiProvider);
  return models.map((m) => ({
    id: m.id,
    name: m.display_name,
  }));
}

/**
 * Get model capabilities from the backend.
 */
export async function fetchModelCapabilities(
  provider: AiProvider,
  modelId: string
): Promise<ModelCapabilities> {
  return getModelCapabilities(provider as BackendAiProvider, modelId);
}

/**
 * Check if a model supports a specific capability.
 */
export function modelSupports(
  capabilities: ModelCapabilities,
  capability: keyof ModelCapabilities
): boolean {
  const value = capabilities[capability];
  if (typeof value === "boolean") {
    return value;
  }
  if (typeof value === "number") {
    return value > 0;
  }
  return false;
}

/**
 * Convert backend OwnedModelDefinition to frontend ModelInfo.
 */
export function toModelInfo(model: OwnedModelDefinition): ModelInfo & { provider: AiProvider } {
  return {
    id: model.id,
    name: model.display_name,
    provider: model.provider as AiProvider,
  };
}

/**
 * Fetch provider info from the backend.
 * Useful for getting display names and icons.
 */
export async function fetchProviderInfo(): Promise<ProviderInfo[]> {
  return getProviders();
}
