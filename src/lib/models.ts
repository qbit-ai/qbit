/**
 * Shared model configuration - single source of truth for all model selectors
 */

import type { ReasoningEffort } from "./ai";
import {
  ANTHROPIC_MODELS,
  GEMINI_MODELS,
  GROQ_MODELS,
  OLLAMA_MODELS,
  OPENAI_MODELS,
  VERTEX_AI_MODELS,
  XAI_MODELS,
} from "./ai";
import type { AiProvider } from "./settings";

export interface ModelInfo {
  id: string;
  name: string;
  reasoningEffort?: ReasoningEffort;
}

export interface ProviderGroup {
  provider: AiProvider;
  providerName: string;
  icon: string;
  models: ModelInfo[];
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
      { id: GEMINI_MODELS.GEMINI_2_5_FLASH_LITE, name: "Gemini 2.5 Flash Lite" },
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
      { id: OPENAI_MODELS.GPT_5_2, name: "GPT 5.2 (Low)", reasoningEffort: "low" },
      { id: OPENAI_MODELS.GPT_5_2, name: "GPT 5.2 (Medium)", reasoningEffort: "medium" },
      { id: OPENAI_MODELS.GPT_5_2, name: "GPT 5.2 (High)", reasoningEffort: "high" },
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
    provider: "xai",
    providerName: "xAI",
    icon: "𝕏",
    models: [
      { id: XAI_MODELS.GROK_4_1_FAST_REASONING, name: "Grok 4.1 Fast (Reasoning)" },
      { id: XAI_MODELS.GROK_4_1_FAST_NON_REASONING, name: "Grok 4.1 Fast" },
      { id: XAI_MODELS.GROK_4_FAST_REASONING, name: "Grok 4 (Reasoning)" },
      { id: XAI_MODELS.GROK_4_FAST_NON_REASONING, name: "Grok 4" },
      { id: XAI_MODELS.GROK_CODE_FAST_1, name: "Grok Code" },
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
      (m) => m.id === modelId && (reasoningEffort === undefined || m.reasoningEffort === reasoningEffort)
    );
    if (model) {
      return { ...model, provider: group.provider, providerName: group.providerName };
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
