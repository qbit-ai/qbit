/**
 * Settings API for Qbit configuration management.
 *
 * Settings are stored in `~/.qbit/settings.toml` and support environment variable
 * interpolation. The backend provides fallback to environment variables for
 * backward compatibility.
 */

import { invoke } from "@tauri-apps/api/core";

// =============================================================================
// Type Definitions
// =============================================================================

/**
 * Configuration for an indexed codebase.
 */
export interface CodebaseConfig {
  /** Path to the codebase (supports ~ for home directory) */
  path: string;
  /** Memory file associated with this codebase: "AGENTS.md", "CLAUDE.md", or undefined */
  memory_file?: string;
}

/**
 * Root settings structure for Qbit.
 */
export interface QbitSettings {
  version: number;
  ai: AiSettings;
  api_keys: ApiKeysSettings;
  ui: UiSettings;
  terminal: TerminalSettings;
  agent: AgentSettings;
  tools: ToolsSettings;
  mcp_servers: Record<string, McpServerConfig>;
  trust: TrustSettings;
  privacy: PrivacySettings;
  advanced: AdvancedSettings;
  sidecar: SidecarSettings;
  /** @deprecated Use `codebases` instead */
  indexed_codebases: string[];
  /** Indexed codebases with configuration */
  codebases: CodebaseConfig[];
}

/**
 * Tool enablement settings.
 */
export interface ToolsSettings {
  /** Enable Tavily-powered web search tools (requires TAVILY_API_KEY) */
  web_search: boolean;
}

/**
 * Reasoning effort level for models that support it.
 */
export type ReasoningEffort = "low" | "medium" | "high";

/**
 * Per-sub-agent model override configuration.
 */
export interface SubAgentModelConfig {
  provider?: AiProvider;
  model?: string;
}

/**
 * AI provider configuration.
 */
export interface AiSettings {
  default_provider: AiProvider;
  default_model: string;
  default_reasoning_effort?: ReasoningEffort;
  /** Per-sub-agent model overrides (key = sub-agent id: "coder", "analyzer", etc.) */
  sub_agent_models: Record<string, SubAgentModelConfig>;
  vertex_ai: VertexAiSettings;
  openrouter: OpenRouterSettings;
  anthropic: AnthropicSettings;
  openai: OpenAiSettings;
  ollama: OllamaSettings;
  gemini: GeminiSettings;
  groq: GroqSettings;
  xai: XaiSettings;
  zai: ZaiSettings;
}

export type AiProvider =
  | "vertex_ai"
  | "openrouter"
  | "anthropic"
  | "openai"
  | "ollama"
  | "gemini"
  | "groq"
  | "xai"
  | "zai";

/**
 * Vertex AI (Anthropic on Google Cloud) settings.
 */
export interface VertexAiSettings {
  credentials_path: string | null;
  project_id: string | null;
  location: string | null;
  show_in_selector: boolean;
}

/**
 * OpenRouter API settings.
 */
export interface OpenRouterSettings {
  api_key: string | null;
  show_in_selector: boolean;
}

/**
 * Direct Anthropic API settings.
 */
export interface AnthropicSettings {
  api_key: string | null;
  show_in_selector: boolean;
}

/**
 * Web search context size for OpenAI's web_search_preview tool.
 */
export type WebSearchContextSize = "low" | "medium" | "high";

/**
 * OpenAI API settings.
 */
export interface OpenAiSettings {
  api_key: string | null;
  base_url: string | null;
  show_in_selector: boolean;
  /** Enable OpenAI's native web search tool (web_search_preview) */
  enable_web_search: boolean;
  /** Web search context size: "low", "medium", or "high" */
  web_search_context_size: WebSearchContextSize;
}

/**
 * Ollama local LLM settings.
 */
export interface OllamaSettings {
  base_url: string;
  show_in_selector: boolean;
}

/**
 * Google Gemini API settings.
 */
export interface GeminiSettings {
  api_key: string | null;
  show_in_selector: boolean;
}

/**
 * Groq API settings.
 */
export interface GroqSettings {
  api_key: string | null;
  show_in_selector: boolean;
}

/**
 * xAI (Grok) API settings.
 */
export interface XaiSettings {
  api_key: string | null;
  show_in_selector: boolean;
}

/**
 * Z.AI (GLM) API settings.
 */
export interface ZaiSettings {
  api_key: string | null;
  /** Use coding-optimized API endpoint instead of general endpoint */
  use_coding_endpoint: boolean;
  show_in_selector: boolean;
}

/**
 * API keys for external services.
 */
export interface ApiKeysSettings {
  tavily: string | null;
  github: string | null;
}

/**
 * User interface preferences.
 */
export interface UiSettings {
  theme: "dark" | "light" | "system";
  show_tips: boolean;
  hide_banner: boolean;
  window: WindowSettings;
}

/**
 * Window state settings (persisted across sessions).
 */
export interface WindowSettings {
  /** Window width in pixels */
  width: number;
  /** Window height in pixels */
  height: number;
  /** Window X position (null = centered) */
  x: number | null;
  /** Window Y position (null = centered) */
  y: number | null;
  /** Whether the window is maximized */
  maximized: boolean;
}

/**
 * Terminal configuration.
 */
export interface TerminalSettings {
  shell: string | null;
  font_family: string;
  font_size: number;
  scrollback: number;
  /** Additional commands that trigger fullterm mode (merged with built-in defaults) */
  fullterm_commands: string[];
}

/**
 * Agent behavior settings.
 */
export interface AgentSettings {
  session_persistence: boolean;
  session_retention_days: number;
  pattern_learning: boolean;
  min_approvals_for_auto: number;
  approval_threshold: number;
}

/**
 * MCP (Model Context Protocol) server configuration.
 */
export interface McpServerConfig {
  command: string | null;
  args: string[];
  env: Record<string, string>;
  url: string | null;
}

/**
 * Repository trust settings.
 */
export interface TrustSettings {
  full_trust: string[];
  read_only_trust: string[];
  never_trust: string[];
}

/**
 * Privacy and telemetry settings.
 */
export interface PrivacySettings {
  usage_statistics: boolean;
  log_prompts: boolean;
}

/**
 * Advanced/debug settings.
 */
export interface AdvancedSettings {
  enable_experimental: boolean;
  log_level: "error" | "warn" | "info" | "debug" | "trace";
  /** Log raw LLM API request/response JSON to ./logs/api/ */
  enable_llm_api_logs: boolean;
  /** Extract and parse raw SSE JSON instead of logging escaped strings */
  extract_raw_sse: boolean;
}

/**
 * Sidecar context capture settings.
 */
export interface SidecarSettings {
  enabled: boolean;
  synthesis_enabled: boolean;
  synthesis_backend: SynthesisBackendType;
  synthesis_vertex: SynthesisVertexSettings;
  synthesis_openai: SynthesisOpenAiSettings;
  synthesis_grok: SynthesisGrokSettings;
  retention_days: number;
  capture_tool_calls: boolean;
  capture_reasoning: boolean;
}

export type SynthesisBackendType = "local" | "vertex_anthropic" | "openai" | "grok" | "template";

/**
 * Vertex AI settings for sidecar synthesis.
 */
export interface SynthesisVertexSettings {
  project_id: string | null;
  location: string | null;
  model: string;
  credentials_path: string | null;
}

/**
 * OpenAI settings for sidecar synthesis.
 */
export interface SynthesisOpenAiSettings {
  api_key: string | null;
  model: string;
  base_url: string | null;
}

/**
 * Grok settings for sidecar synthesis.
 */
export interface SynthesisGrokSettings {
  api_key: string | null;
  model: string;
}

// =============================================================================
// API Functions
// =============================================================================

/**
 * Get all settings.
 */
export async function getSettings(): Promise<QbitSettings> {
  return invoke("get_settings");
}

/**
 * Update all settings.
 */
export async function updateSettings(settings: QbitSettings): Promise<void> {
  return invoke("update_settings", { settings });
}

/**
 * Get a specific setting by dot-notation key.
 * @example getSetting("ai.vertex_ai.project_id")
 */
export async function getSetting<T = unknown>(key: string): Promise<T> {
  return invoke("get_setting", { key });
}

/**
 * Set a specific setting by dot-notation key.
 * @example setSetting("ui.theme", "light")
 */
export async function setSetting(key: string, value: unknown): Promise<void> {
  return invoke("set_setting", { key, value });
}

/**
 * Reset all settings to defaults.
 */
export async function resetSettings(): Promise<void> {
  return invoke("reset_settings");
}

/**
 * Reload settings from disk.
 */
export async function reloadSettings(): Promise<void> {
  return invoke("reload_settings");
}

/**
 * Check if settings file exists.
 */
export async function settingsFileExists(): Promise<boolean> {
  return invoke("settings_file_exists");
}

/**
 * Get the path to the settings file.
 */
export async function getSettingsPath(): Promise<string> {
  return invoke("get_settings_path");
}

/**
 * Check if Langfuse tracing is active.
 *
 * Returns true if Langfuse was enabled in settings and properly configured
 * (i.e., valid API keys were available) at startup.
 */
export async function isLangfuseActive(): Promise<boolean> {
  return invoke("is_langfuse_active");
}

// =============================================================================
// Default Settings
// =============================================================================

/**
 * Default settings matching the Rust defaults.
 */
export const DEFAULT_SETTINGS: QbitSettings = {
  version: 1,
  ai: {
    default_provider: "vertex_ai",
    default_model: "claude-opus-4-5@20251101",
    default_reasoning_effort: undefined,
    sub_agent_models: {},
    vertex_ai: {
      credentials_path: null,
      project_id: null,
      location: null,
      show_in_selector: true,
    },
    openrouter: {
      api_key: null,
      show_in_selector: true,
    },
    anthropic: {
      api_key: null,
      show_in_selector: true,
    },
    openai: {
      api_key: null,
      base_url: null,
      show_in_selector: true,
      enable_web_search: false,
      web_search_context_size: "medium",
    },
    ollama: {
      base_url: "http://localhost:11434",
      show_in_selector: true,
    },
    gemini: {
      api_key: null,
      show_in_selector: true,
    },
    groq: {
      api_key: null,
      show_in_selector: true,
    },
    xai: {
      api_key: null,
      show_in_selector: true,
    },
    zai: {
      api_key: null,
      use_coding_endpoint: true,
      show_in_selector: true,
    },
  },
  api_keys: {
    tavily: null,
    github: null,
  },
  ui: {
    theme: "dark",
    show_tips: true,
    hide_banner: false,
    window: {
      width: 1400,
      height: 900,
      x: null,
      y: null,
      maximized: false,
    },
  },
  terminal: {
    shell: null,
    font_family: "JetBrains Mono",
    font_size: 14,
    scrollback: 10000,
    fullterm_commands: [],
  },
  agent: {
    session_persistence: true,
    session_retention_days: 30,
    pattern_learning: true,
    min_approvals_for_auto: 3,
    approval_threshold: 0.8,
  },
  tools: {
    web_search: false,
  },
  mcp_servers: {},
  trust: {
    full_trust: [],
    read_only_trust: [],
    never_trust: [],
  },
  privacy: {
    usage_statistics: false,
    log_prompts: false,
  },
  advanced: {
    enable_experimental: false,
    log_level: "info",
    enable_llm_api_logs: false,
    extract_raw_sse: false,
  },
  sidecar: {
    enabled: false,
    synthesis_enabled: true,
    synthesis_backend: "template",
    synthesis_vertex: {
      project_id: null,
      location: null,
      model: "claude-sonnet-4-5-20250514",
      credentials_path: null,
    },
    synthesis_openai: {
      api_key: null,
      model: "gpt-4o-mini",
      base_url: null,
    },
    synthesis_grok: {
      api_key: null,
      model: "grok-2",
    },
    retention_days: 30,
    capture_tool_calls: true,
    capture_reasoning: true,
  },
  indexed_codebases: [],
  codebases: [],
};
