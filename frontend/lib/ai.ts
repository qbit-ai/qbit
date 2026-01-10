import { invoke } from "@tauri-apps/api/core";
import { listen as tauriListen, type UnlistenFn } from "@tauri-apps/api/event";
import type { QbitSettings } from "./settings";
import type { RiskLevel } from "./tools";

// In browser mode, use the mock listen function if available
declare global {
  interface Window {
    __MOCK_LISTEN__?: typeof tauriListen;
    __MOCK_BROWSER_MODE__?: boolean;
  }
}

// Use mock listen in browser mode, otherwise use real Tauri listen
const listen: typeof tauriListen = (...args) => {
  if (window.__MOCK_BROWSER_MODE__ && window.__MOCK_LISTEN__) {
    return window.__MOCK_LISTEN__(...args);
  }
  return tauriListen(...args);
};

export type { RiskLevel };

export type AiProvider =
  | "vertex_ai"
  | "openrouter"
  | "openai"
  | "anthropic"
  | "ollama"
  | "gemini"
  | "groq"
  | "xai"
  | "zai";

/** Per-project settings from .qbit/project.toml */
export interface ProjectSettings {
  provider: AiProvider | null;
  model: string | null;
  agent_mode: AgentMode | null;
}

export interface AiConfig {
  workspace: string;
  provider: AiProvider;
  model: string;
  apiKey: string;
}

/** Unified configuration for all LLM providers (matches Rust ProviderConfig) */
export type ProviderConfig =
  | {
      provider: "vertex_ai";
      workspace: string;
      model: string;
      credentials_path: string | null;
      project_id: string;
      location: string;
    }
  | {
      provider: "openrouter";
      workspace: string;
      model: string;
      api_key: string;
    }
  | {
      provider: "openai";
      workspace: string;
      model: string;
      api_key: string;
      base_url?: string;
      reasoning_effort?: string;
    }
  | {
      provider: "anthropic";
      workspace: string;
      model: string;
      api_key: string;
    }
  | {
      provider: "ollama";
      workspace: string;
      model: string;
      base_url?: string;
    }
  | {
      provider: "gemini";
      workspace: string;
      model: string;
      api_key: string;
    }
  | {
      provider: "groq";
      workspace: string;
      model: string;
      api_key: string;
    }
  | {
      provider: "xai";
      workspace: string;
      model: string;
      api_key: string;
    }
  | {
      provider: "zai";
      workspace: string;
      model: string;
      api_key: string;
      use_coding_endpoint?: boolean;
    };

/**
 * Approval pattern/statistics for a specific tool.
 */
export interface ApprovalPattern {
  tool_name: string;
  total_requests: number;
  approvals: number;
  denials: number;
  always_allow: boolean;
  last_updated: string;
  justifications: string[];
}

/** Source of a tool call - indicates where the tool request originated */
export type ToolSource =
  | { type: "main" }
  | { type: "sub_agent"; agent_id: string; agent_name: string }
  | {
      type: "workflow";
      workflow_id: string;
      workflow_name: string;
      step_name?: string;
      step_index?: number;
    };

/** Base AI event with session routing */
interface AiEventBase {
  /** Session ID for routing events to the correct tab */
  session_id: string;
}

export type AiEvent = AiEventBase &
  (
    | { type: "started"; turn_id: string }
    | { type: "text_delta"; delta: string; accumulated: string }
    | {
        type: "tool_request";
        tool_name: string;
        args: unknown;
        request_id: string;
        source?: ToolSource;
      }
    | {
        type: "tool_approval_request";
        request_id: string;
        tool_name: string;
        args: unknown;
        stats: ApprovalPattern | null;
        risk_level: RiskLevel;
        can_learn: boolean;
        suggestion: string | null;
        source?: ToolSource;
      }
    | {
        type: "tool_auto_approved";
        request_id: string;
        tool_name: string;
        args: unknown;
        reason: string;
        source?: ToolSource;
      }
    | {
        type: "tool_result";
        tool_name: string;
        result: unknown;
        success: boolean;
        request_id: string;
        source?: ToolSource;
      }
    | { type: "reasoning"; content: string }
    | {
        type: "completed";
        response: string;
        input_tokens?: number;
        output_tokens?: number;
        duration_ms?: number;
      }
    | { type: "error"; message: string; error_type: string }
    // Sub-agent events
    | {
        type: "sub_agent_started";
        agent_id: string;
        agent_name: string;
        task: string;
        depth: number;
        parent_request_id: string;
      }
    | {
        type: "sub_agent_tool_request";
        agent_id: string;
        tool_name: string;
        args: unknown;
        request_id: string;
        parent_request_id: string;
      }
    | {
        type: "sub_agent_tool_result";
        agent_id: string;
        tool_name: string;
        success: boolean;
        result: unknown;
        request_id: string;
        parent_request_id: string;
      }
    | {
        type: "sub_agent_completed";
        agent_id: string;
        response: string;
        duration_ms: number;
        parent_request_id: string;
      }
    | {
        type: "sub_agent_error";
        agent_id: string;
        error: string;
        parent_request_id: string;
      }
    // Workflow events
    | {
        type: "workflow_started";
        workflow_id: string;
        workflow_name: string;
        session_id: string;
      }
    | {
        type: "workflow_step_started";
        workflow_id: string;
        step_name: string;
        step_index: number;
        total_steps: number;
      }
    | {
        type: "workflow_step_completed";
        workflow_id: string;
        step_name: string;
        output: string | null;
        duration_ms: number;
      }
    | {
        type: "workflow_completed";
        workflow_id: string;
        final_output: string;
        total_duration_ms: number;
      }
    | {
        type: "workflow_error";
        workflow_id: string;
        step_name: string | null;
        error: string;
      }
    // Plan events
    | {
        type: "plan_updated";
        version: number;
        summary: {
          total: number;
          completed: number;
          in_progress: number;
          pending: number;
        };
        steps: Array<{ step: string; status: "pending" | "in_progress" | "completed" }>;
        explanation: string | null;
      }
    // Context management events
    | {
        type: "context_warning";
        utilization: number;
        total_tokens: number;
        max_tokens: number;
      }
    | {
        type: "context_pruned";
        messages_removed: number;
        tokens_freed: number;
        utilization_before: number;
        utilization_after: number;
      }
    | {
        type: "tool_response_truncated";
        tool_name: string;
        original_tokens: number;
        truncated_tokens: number;
      }
    // Server tool events (Claude's native web_search/web_fetch)
    | {
        type: "server_tool_started";
        request_id: string;
        tool_name: string;
        input: unknown;
      }
    | {
        type: "web_search_result";
        request_id: string;
        results: unknown;
      }
    | {
        type: "web_fetch_result";
        request_id: string;
        url: string;
        content_preview: string;
      }
    // Warning event (e.g., images stripped from non-vision provider)
    | {
        type: "warning";
        message: string;
      }
  );

export interface ToolDefinition {
  name: string;
  description: string;
  parameters: Record<string, unknown>;
}

export interface WorkflowInfo {
  name: string;
  description: string;
}

export interface SubAgentInfo {
  id: string;
  name: string;
  description: string;
  /** Model override if set: [provider, model] tuple */
  model_override: [string, string] | null;
}

/**
 * Initialize the AI agent with the specified configuration
 */
export async function initAiAgent(config: AiConfig): Promise<void> {
  return invoke("init_ai_agent", {
    workspace: config.workspace,
    provider: config.provider,
    model: config.model,
    apiKey: config.apiKey,
  });
}

/**
 * Send a prompt to the AI agent
 * Response will be streamed via the ai-event listener
 *
 * @param prompt - The user's message
 */
export async function sendPrompt(prompt: string): Promise<string> {
  return invoke("send_ai_prompt", { prompt });
}

/**
 * Execute a specific tool with arguments
 */
export async function executeTool(toolName: string, args: unknown): Promise<unknown> {
  return invoke("execute_ai_tool", { toolName, args });
}

/**
 * Get list of available tools
 */
export async function getAvailableTools(): Promise<ToolDefinition[]> {
  return invoke("get_available_tools");
}

/**
 * Get list of available workflows
 */
export async function getAvailableWorkflows(): Promise<WorkflowInfo[]> {
  return invoke("list_workflows");
}

/**
 * Get list of available sub-agents
 */
export async function getAvailableSubAgents(): Promise<SubAgentInfo[]> {
  return invoke("list_sub_agents");
}

// =============================================================================
// Sub-Agent Model Configuration
// =============================================================================

/**
 * Model override configuration for a sub-agent.
 */
export interface SubAgentModelOverride {
  provider: AiProvider;
  model: string;
}

/**
 * Set the model override for a sub-agent.
 * This allows a sub-agent to use a different model than the main agent.
 *
 * @param sessionId - The session ID
 * @param agentId - Sub-agent identifier (e.g., "coder", "researcher")
 * @param provider - Provider name (e.g., "openai", "vertex_ai"). Pass null to clear.
 * @param model - Model name (e.g., "gpt-4o"). Pass null to clear.
 */
export async function setSubAgentModel(
  sessionId: string,
  agentId: string,
  provider: AiProvider | null,
  model: string | null
): Promise<void> {
  return invoke("set_sub_agent_model", {
    sessionId,
    agentId,
    provider,
    model,
  });
}

/**
 * Get the current model override for a sub-agent.
 *
 * @param sessionId - The session ID
 * @param agentId - Sub-agent identifier
 * @returns The model override [provider, model] tuple, or null if using main agent's model
 */
export async function getSubAgentModel(
  sessionId: string,
  agentId: string
): Promise<[string, string] | null> {
  return invoke("get_sub_agent_model", { sessionId, agentId });
}

/**
 * Clear the model override for a sub-agent (revert to main agent's model).
 *
 * @param sessionId - The session ID
 * @param agentId - Sub-agent identifier
 */
export async function clearSubAgentModel(sessionId: string, agentId: string): Promise<void> {
  return setSubAgentModel(sessionId, agentId, null, null);
}

/**
 * Shutdown the AI agent
 */
export async function shutdownAiAgent(): Promise<void> {
  return invoke("shutdown_ai_agent");
}

/**
 * Subscribe to AI events
 * Returns an unlisten function to stop listening
 */
export function onAiEvent(callback: (event: AiEvent) => void): Promise<UnlistenFn> {
  return listen<AiEvent>("ai-event", (event) => callback(event.payload));
}

/**
 * Check if AI agent is initialized
 */
export async function isAiInitialized(): Promise<boolean> {
  return invoke("is_ai_initialized");
}

/**
 * Update the AI agent's workspace/working directory.
 * This keeps the agent in sync with the user's terminal directory.
 *
 * @param workspace - New workspace/working directory path
 * @param sessionId - Optional session ID to update the session-specific bridge
 */
export async function updateAiWorkspace(workspace: string, sessionId?: string): Promise<void> {
  return invoke("update_ai_workspace", { workspace, sessionId });
}

/**
 * Clear the AI agent's conversation history.
 * Call this when starting a new conversation or when the user wants to reset context.
 */
export async function clearAiConversation(): Promise<void> {
  return invoke("clear_ai_conversation");
}

/**
 * Get the current conversation history length.
 * Useful for debugging or showing context status in the UI.
 */
export async function getAiConversationLength(): Promise<number> {
  return invoke("get_ai_conversation_length");
}

// =============================================================================
// Session-specific AI Functions (Per-tab AI isolation)
// =============================================================================

/**
 * Session AI configuration returned from the backend.
 */
export interface SessionAiConfigInfo {
  provider: string;
  model: string;
}

/**
 * Initialize AI agent for a specific session (tab).
 * Each session can have its own provider/model configuration.
 *
 * @param sessionId - The terminal session ID (tab) to initialize AI for
 * @param config - Provider-specific configuration
 */
export async function initAiSession(sessionId: string, config: ProviderConfig): Promise<void> {
  return invoke("init_ai_session", { sessionId, config });
}

/**
 * Shutdown AI agent for a specific session.
 * Call this when a tab is closed to clean up resources.
 *
 * @param sessionId - The terminal session ID to shut down AI for
 */
export async function shutdownAiSession(sessionId: string): Promise<void> {
  return invoke("shutdown_ai_session", { sessionId });
}

/**
 * Check if AI agent is initialized for a specific session.
 *
 * @param sessionId - The terminal session ID to check
 */
export async function isAiSessionInitialized(sessionId: string): Promise<boolean> {
  return invoke("is_ai_session_initialized", { sessionId });
}

/**
 * Get the AI configuration for a specific session.
 *
 * @param sessionId - The terminal session ID
 */
export async function getSessionAiConfig(sessionId: string): Promise<SessionAiConfigInfo | null> {
  return invoke("get_session_ai_config", { sessionId });
}

/**
 * Send a prompt to the AI agent for a specific session.
 * Response will be streamed via the ai-event listener.
 *
 * @param sessionId - The terminal session ID to send the prompt to
 * @param prompt - The user's message
 */
export async function sendPromptSession(sessionId: string, prompt: string): Promise<string> {
  return invoke("send_ai_prompt_session", { sessionId, prompt });
}

/**
 * Clear the AI conversation history for a specific session.
 *
 * @param sessionId - The terminal session ID
 */
export async function clearAiConversationSession(sessionId: string): Promise<void> {
  return invoke("clear_ai_conversation_session", { sessionId });
}

/**
 * Get the conversation history length for a specific session.
 *
 * @param sessionId - The terminal session ID
 */
export async function getAiConversationLengthSession(sessionId: string): Promise<number> {
  return invoke("get_ai_conversation_length_session", { sessionId });
}

/**
 * Get the OpenRouter API key from environment variables.
 * Returns null if not set.
 */
export async function getOpenRouterApiKey(): Promise<string | null> {
  return invoke("get_openrouter_api_key");
}

/**
 * Load environment variables from a .env file.
 * Returns the number of variables loaded.
 */
export async function loadEnvFile(path: string): Promise<number> {
  return invoke("load_env_file", { path });
}

/**
 * Vertex AI configuration from environment variables.
 */
export interface VertexAiEnvConfig {
  credentials_path: string | null;
  project_id: string | null;
  location: string | null;
}

/**
 * Get Vertex AI configuration from environment variables.
 * Reads from:
 * - VERTEX_AI_CREDENTIALS_PATH or GOOGLE_APPLICATION_CREDENTIALS
 * - VERTEX_AI_PROJECT_ID or GOOGLE_CLOUD_PROJECT
 * - VERTEX_AI_LOCATION (defaults to "us-east5" if not set)
 */
export async function getVertexAiConfig(): Promise<VertexAiEnvConfig> {
  return invoke("get_vertex_ai_config");
}

/**
 * Default configuration for Claude Opus 4.5 via OpenRouter.
 * API key should be provided from environment or user input.
 */
export const DEFAULT_AI_CONFIG = {
  provider: "openrouter" as AiProvider,
  // OpenRouter model ID for Claude Opus 4.5
  model: "anthropic/claude-opus-4.5",
};

/**
 * Initialize AI with Claude Opus 4.5 via OpenRouter.
 * This is a convenience function that uses sensible defaults.
 */
export async function initClaudeOpus(workspace: string, apiKey: string): Promise<void> {
  return initAiAgent({
    workspace,
    provider: DEFAULT_AI_CONFIG.provider,
    model: DEFAULT_AI_CONFIG.model,
    apiKey,
  });
}

/**
 * Configuration for Vertex AI Anthropic.
 */
export interface VertexAiConfig {
  workspace: string;
  credentialsPath: string;
  projectId: string;
  location: string;
  model: string;
}

/**
 * Available Claude models on Vertex AI.
 */
export const VERTEX_AI_MODELS = {
  CLAUDE_OPUS_4_5: "claude-opus-4-5@20251101",
  CLAUDE_SONNET_4_5: "claude-sonnet-4-5@20250929",
  CLAUDE_HAIKU_4_5: "claude-haiku-4-5@20251001",
} as const;

/**
 * Available OpenAI models.
 * @see https://platform.openai.com/docs/models
 */
export const OPENAI_MODELS = {
  // GPT-5 series
  GPT_5_2: "gpt-5.2",
  GPT_5_1: "gpt-5.1",
  GPT_5: "gpt-5",
  GPT_5_MINI: "gpt-5-mini",
  GPT_5_NANO: "gpt-5-nano",
  // GPT-4.1 series
  GPT_4_1: "gpt-4.1",
  GPT_4_1_MINI: "gpt-4.1-mini",
  GPT_4_1_NANO: "gpt-4.1-nano",
  // GPT-4o series
  GPT_4O: "gpt-4o",
  GPT_4O_MINI: "gpt-4o-mini",
  CHATGPT_4O_LATEST: "chatgpt-4o-latest",
  // o-series reasoning models
  O4_MINI: "o4-mini",
  O3: "o3",
  O3_MINI: "o3-mini",
  O1: "o1",
  // Codex models (coding-optimized)
  GPT_5_1_CODEX: "gpt-5.1-codex",
  GPT_5_1_CODEX_MAX: "gpt-5.1-codex-max",
  CODEX_MINI_LATEST: "codex-mini-latest",
} as const;

/**
 * Available Claude models via direct Anthropic API.
 */
export const ANTHROPIC_MODELS = {
  CLAUDE_OPUS_4_5: "claude-opus-4-5-20251101",
  CLAUDE_SONNET_4_5: "claude-sonnet-4-5-20250929",
  CLAUDE_HAIKU_4_5: "claude-haiku-4-5-20250514",
} as const;

/**
 * Common Ollama models.
 */
export const OLLAMA_MODELS = {
  LLAMA_3_2: "llama3.2",
  LLAMA_3_1: "llama3.1",
  MISTRAL: "mistral",
  CODELLAMA: "codellama",
  QWEN_2_5: "qwen2.5",
} as const;

/**
 * Available Gemini models.
 * @see https://ai.google.dev/gemini-api/docs/models
 */
export const GEMINI_MODELS = {
  GEMINI_3_PRO_PREVIEW: "gemini-3-pro-preview",
  GEMINI_2_5_PRO: "gemini-2.5-pro",
  GEMINI_2_5_FLASH: "gemini-2.5-flash",
  GEMINI_2_5_FLASH_LITE: "gemini-2.5-flash-lite",
} as const;

/**
 * Available Groq models.
 * @see https://console.groq.com/docs/models
 */
export const GROQ_MODELS = {
  LLAMA_4_SCOUT: "meta-llama/llama-4-scout-17b-16e-instruct",
  LLAMA_4_MAVERICK: "meta-llama/llama-4-maverick-17b-128e-instruct",
  LLAMA_3_3_70B: "llama-3.3-70b-versatile",
  LLAMA_3_1_8B: "llama-3.1-8b-instant",
  GPT_OSS_120B: "openai/gpt-oss-120b",
  GPT_OSS_20B: "openai/gpt-oss-20b",
} as const;

/**
 * Available xAI models.
 * @see https://docs.x.ai/docs/models
 */
export const XAI_MODELS = {
  GROK_4_1_FAST_REASONING: "grok-4-1-fast-reasoning",
  GROK_4_1_FAST_NON_REASONING: "grok-4-1-fast-non-reasoning",
  GROK_CODE_FAST_1: "grok-code-fast-1",
  GROK_4_FAST_REASONING: "grok-4-fast-reasoning",
  GROK_4_FAST_NON_REASONING: "grok-4-fast-non-reasoning",
} as const;

/**
 * Available Z.AI (GLM) models.
 * @see https://docs.z.ai/devpack/tool/others
 */
export const ZAI_MODELS = {
  GLM_4_7: "GLM-4.7",
  GLM_4_5_AIR: "GLM-4.5-air",
} as const;

/**
 * Reasoning effort levels for OpenAI models that support it.
 */
export type ReasoningEffort = "low" | "medium" | "high";

/**
 * Initialize AI with Anthropic on Google Cloud Vertex AI.
 * This uses a service account JSON file for authentication.
 */
export async function initVertexAiAgent(config: VertexAiConfig): Promise<void> {
  return invoke("init_ai_agent_vertex", {
    workspace: config.workspace,
    credentialsPath: config.credentialsPath,
    projectId: config.projectId,
    location: config.location,
    model: config.model,
  });
}

/**
 * Initialize AI with Claude Opus 4.5 on Vertex AI.
 * This is a convenience function that uses the latest Opus 4.5 model.
 */
export async function initVertexClaudeOpus(
  workspace: string,
  credentialsPath: string,
  projectId: string,
  location: string = "us-east5"
): Promise<void> {
  return initVertexAiAgent({
    workspace,
    credentialsPath,
    projectId,
    location,
    model: VERTEX_AI_MODELS.CLAUDE_OPUS_4_5,
  });
}

/**
 * Configuration for OpenAI.
 */
export interface OpenAiConfig {
  workspace: string;
  model: string;
  apiKey: string;
  baseUrl?: string;
  reasoningEffort?: ReasoningEffort;
}

/**
 * Get the OpenAI API key from settings or environment.
 */
export async function getOpenAiApiKey(): Promise<string | null> {
  return invoke("get_openai_api_key");
}

/**
 * Initialize AI with OpenAI.
 */
export async function initOpenAiAgent(config: OpenAiConfig): Promise<void> {
  return invoke("init_ai_agent_openai", {
    workspace: config.workspace,
    model: config.model,
    apiKey: config.apiKey,
    baseUrl: config.baseUrl,
    reasoningEffort: config.reasoningEffort,
  });
}

// =============================================================================
// Unified Provider Initialization
// =============================================================================

/**
 * Initialize AI agent with unified configuration.
 * This is the preferred method for initializing any provider.
 *
 * @param config - Provider-specific configuration with discriminator
 */
export async function initAiAgentUnified(config: ProviderConfig): Promise<void> {
  return invoke("init_ai_agent_unified", { config });
}

/**
 * Initialize AI with direct Anthropic API.
 */
export async function initWithAnthropic(
  workspace: string,
  apiKey: string,
  model: string = ANTHROPIC_MODELS.CLAUDE_SONNET_4_5
): Promise<void> {
  return initAiAgentUnified({
    provider: "anthropic",
    workspace,
    model,
    api_key: apiKey,
  });
}

/**
 * Initialize AI with Ollama local inference.
 */
export async function initWithOllama(
  workspace: string,
  model: string = OLLAMA_MODELS.LLAMA_3_2,
  baseUrl?: string
): Promise<void> {
  return initAiAgentUnified({
    provider: "ollama",
    workspace,
    model,
    base_url: baseUrl,
  });
}

/**
 * Initialize AI with Gemini.
 */
export async function initWithGemini(
  workspace: string,
  apiKey: string,
  model: string = GEMINI_MODELS.GEMINI_2_5_FLASH
): Promise<void> {
  return initAiAgentUnified({
    provider: "gemini",
    workspace,
    model,
    api_key: apiKey,
  });
}

/**
 * Initialize AI with Groq.
 */
export async function initWithGroq(
  workspace: string,
  apiKey: string,
  model: string = GROQ_MODELS.LLAMA_4_SCOUT
): Promise<void> {
  return initAiAgentUnified({
    provider: "groq",
    workspace,
    model,
    api_key: apiKey,
  });
}

/**
 * Initialize AI with xAI (Grok).
 */
export async function initWithXai(
  workspace: string,
  apiKey: string,
  model: string = XAI_MODELS.GROK_4_1_FAST_REASONING
): Promise<void> {
  return initAiAgentUnified({
    provider: "xai",
    workspace,
    model,
    api_key: apiKey,
  });
}

/**
 * Get the Anthropic API key from settings or environment.
 */
export async function getAnthropicApiKey(): Promise<string | null> {
  return invoke("get_anthropic_api_key");
}

// =============================================================================
// Session Persistence API
// =============================================================================

/**
 * Role of a message in the conversation.
 */
export type SessionMessageRole = "user" | "assistant" | "system" | "tool";

/**
 * A message in a session.
 */
export interface SessionMessage {
  role: SessionMessageRole;
  content: string;
  tool_call_id?: string;
  tool_name?: string;
}

/**
 * Information about a saved session (listing view).
 */
export interface SessionListingInfo {
  identifier: string;
  path: string;
  workspace_label: string;
  workspace_path: string;
  model: string;
  provider: string;
  started_at: string;
  ended_at: string;
  total_messages: number;
  distinct_tools: string[];
  first_prompt_preview?: string;
  first_reply_preview?: string;
  /** Session status: "active", "completed", or "abandoned" */
  status?: "active" | "completed" | "abandoned";
  /** LLM-generated session title */
  title?: string;
}

/**
 * Full session snapshot with all messages.
 */
export interface SessionSnapshot {
  workspace_label: string;
  workspace_path: string;
  model: string;
  provider: string;
  started_at: string;
  ended_at: string;
  total_messages: number;
  distinct_tools: string[];
  transcript: string[];
  messages: SessionMessage[];
  /** Agent mode used in this session ("default", "auto-approve", "planning") */
  agent_mode?: string;
}

/**
 * List recent AI conversation sessions.
 *
 * @param limit - Maximum number of sessions to return (default: 20)
 */
export async function listAiSessions(limit?: number): Promise<SessionListingInfo[]> {
  return invoke("list_ai_sessions", { limit });
}

/**
 * Find a specific session by its identifier.
 *
 * @param identifier - The session identifier (file stem)
 */
export async function findAiSession(identifier: string): Promise<SessionListingInfo | null> {
  return invoke("find_ai_session", { identifier });
}

/**
 * Load a full session with all messages by its identifier.
 *
 * @param identifier - The session identifier (file stem)
 */
export async function loadAiSession(identifier: string): Promise<SessionSnapshot | null> {
  return invoke("load_ai_session", { identifier });
}

/**
 * Export a session transcript to a file.
 *
 * @param identifier - The session identifier (file stem)
 * @param outputPath - Path where the transcript should be saved
 */
export async function exportAiSessionTranscript(
  identifier: string,
  outputPath: string
): Promise<void> {
  return invoke("export_ai_session_transcript", { identifier, outputPath });
}

/**
 * Enable or disable session persistence.
 * When enabled, AI conversations are automatically saved to disk.
 *
 * @param enabled - Whether to enable session persistence
 */
export async function setAiSessionPersistence(enabled: boolean): Promise<void> {
  return invoke("set_ai_session_persistence", { enabled });
}

/**
 * Check if session persistence is enabled.
 */
export async function isAiSessionPersistenceEnabled(): Promise<boolean> {
  return invoke("is_ai_session_persistence_enabled");
}

/**
 * Manually finalize and save the current session.
 * Returns the path to the saved session file, if any.
 */
export async function finalizeAiSession(): Promise<string | null> {
  return invoke("finalize_ai_session");
}

/**
 * Restore a previous session by loading its conversation history.
 * This loads the session's messages into the AI agent's conversation history,
 * allowing the user to continue from where they left off.
 *
 * @param identifier - The session identifier (file stem)
 * @returns The restored session snapshot
 */
export async function restoreAiSession(
  sessionId: string,
  identifier: string
): Promise<SessionSnapshot> {
  return invoke("restore_ai_session", { sessionId, identifier });
}

// =============================================================================
// HITL (Human-in-the-Loop) API
// =============================================================================

/**
 * Configuration for tool approval behavior.
 */
export interface ToolApprovalConfig {
  /** Tools that are always allowed without approval */
  always_allow: string[];
  /** Tools that always require approval (cannot be auto-approved) */
  always_require_approval: string[];
  /** Whether pattern learning is enabled */
  pattern_learning_enabled: boolean;
  /** Minimum approvals before auto-approve */
  min_approvals: number;
  /** Approval rate threshold (0.0 - 1.0) */
  approval_threshold: number;
}

/**
 * User's decision on an approval request.
 */
export interface ApprovalDecision {
  /** The request ID this decision is for */
  request_id: string;
  /** Whether the tool was approved */
  approved: boolean;
  /** Optional reason/justification for the decision */
  reason?: string;
  /** Whether to remember this decision for future auto-approval */
  remember: boolean;
  /** Whether to always allow this specific tool */
  always_allow: boolean;
}

/**
 * Get approval patterns for all tools.
 */
export async function getApprovalPatterns(): Promise<ApprovalPattern[]> {
  return invoke("get_approval_patterns");
}

/**
 * Get the approval pattern for a specific tool.
 */
export async function getToolApprovalPattern(toolName: string): Promise<ApprovalPattern | null> {
  return invoke("get_tool_approval_pattern", { toolName });
}

/**
 * Get the HITL configuration.
 */
export async function getHitlConfig(): Promise<ToolApprovalConfig> {
  return invoke("get_hitl_config");
}

/**
 * Update the HITL configuration.
 */
export async function setHitlConfig(config: ToolApprovalConfig): Promise<void> {
  return invoke("set_hitl_config", { config });
}

/**
 * Add a tool to the always-allow list.
 */
export async function addToolAlwaysAllow(toolName: string): Promise<void> {
  return invoke("add_tool_always_allow", { toolName });
}

/**
 * Remove a tool from the always-allow list.
 */
export async function removeToolAlwaysAllow(toolName: string): Promise<void> {
  return invoke("remove_tool_always_allow", { toolName });
}

/**
 * Reset all approval patterns (does not reset configuration).
 */
export async function resetApprovalPatterns(): Promise<void> {
  return invoke("reset_approval_patterns");
}

/**
 * Respond to a tool approval request.
 * This is called by the frontend after the user makes a decision in the approval dialog.
 *
 * @param sessionId - The session ID where the approval request originated
 * @param decision - The user's approval decision
 */
export async function respondToToolApproval(
  sessionId: string,
  decision: ApprovalDecision
): Promise<void> {
  return invoke("respond_to_tool_approval", { sessionId, decision });
}

/**
 * Calculate the approval rate from an ApprovalPattern.
 */
export function calculateApprovalRate(pattern: ApprovalPattern): number {
  if (pattern.total_requests === 0) return 0;
  return pattern.approvals / pattern.total_requests;
}

/**
 * Check if a pattern qualifies for auto-approval based on default thresholds.
 */
export function qualifiesForAutoApprove(
  pattern: ApprovalPattern,
  minApprovals = 3,
  threshold = 0.8
): boolean {
  return pattern.approvals >= minApprovals && calculateApprovalRate(pattern) >= threshold;
}

// =============================================================================
// Agent Mode API
// =============================================================================

/**
 * Agent mode determines how tool approvals are handled.
 * - default: Tool approval required based on policy (normal HITL)
 * - auto-approve: All tool calls are automatically approved
 * - planning: Only read-only tools allowed (no modifications)
 */
export type AgentMode = "default" | "auto-approve" | "planning";

/**
 * Set the agent mode for a session.
 * This controls how tool approvals are handled.
 *
 * @param sessionId - The session ID to set the mode for
 * @param mode - The agent mode to set
 * @param workspace - Optional workspace path to persist the setting
 */
export async function setAgentMode(
  sessionId: string,
  mode: AgentMode,
  workspace?: string
): Promise<void> {
  return invoke("set_agent_mode", { sessionId, mode, workspace });
}

/**
 * Get the current agent mode for a session.
 *
 * @param sessionId - The session ID to get the mode for
 */
export async function getAgentMode(sessionId: string): Promise<AgentMode> {
  return invoke("get_agent_mode", { sessionId });
}

// =============================================================================
// Project Settings API
// =============================================================================

/**
 * Get per-project settings from {workspace}/.qbit/project.toml
 * Returns the stored provider, model, and agent_mode (all optional)
 */
export async function getProjectSettings(workspace: string): Promise<ProjectSettings> {
  return invoke("get_project_settings", { workspace });
}

/**
 * Save the provider and model to per-project settings.
 * This persists the selection to {workspace}/.qbit/project.toml
 */
export async function saveProjectModel(
  workspace: string,
  provider: string,
  model: string
): Promise<void> {
  return invoke("save_project_model", { workspace, provider, model });
}

/**
 * Save the agent mode to per-project settings.
 */
export async function saveProjectAgentMode(workspace: string, mode: AgentMode): Promise<void> {
  return invoke("save_project_agent_mode", { workspace, mode });
}

// =============================================================================
// Plan API
// =============================================================================

/**
 * Task plan interface (matches backend TaskPlan)
 */
export interface TaskPlan {
  explanation: string | null;
  steps: Array<{ step: string; status: "pending" | "in_progress" | "completed" }>;
  summary: {
    total: number;
    completed: number;
    in_progress: number;
    pending: number;
  };
  version: number;
  updated_at: string;
}

/**
 * Get the current task plan for a session.
 *
 * @param sessionId - The session ID to get the plan for
 */
export async function getPlan(sessionId: string): Promise<TaskPlan> {
  return invoke("get_plan", { sessionId });
}

// =============================================================================
// Vision & Multi-Modal API
// =============================================================================

/**
 * Vision capabilities for a provider/model.
 * Indicates whether images can be sent and any size/format restrictions.
 */
export interface VisionCapabilities {
  /** Whether this provider/model supports vision (images) */
  supports_vision: boolean;
  /** Maximum image size in bytes */
  max_image_size_bytes: number;
  /** Supported MIME types (e.g., "image/png", "image/jpeg") */
  supported_formats: string[];
}

/**
 * Text part of a prompt payload.
 */
export interface TextPart {
  type: "text";
  text: string;
}

/**
 * Image part of a prompt payload.
 */
export interface ImagePart {
  type: "image";
  /** Base64-encoded image data (or data URL) */
  data: string;
  /** MIME type (e.g., "image/png") */
  media_type?: string;
  /** Optional filename */
  filename?: string;
}

/**
 * A part of a prompt - can be text or an image.
 */
export type PromptPart = TextPart | ImagePart;

/**
 * Multi-modal prompt payload with text and/or images.
 */
export interface PromptPayload {
  parts: PromptPart[];
}

/**
 * Get vision capabilities for a session's provider/model.
 *
 * @param sessionId - The session ID to check capabilities for
 */
export async function getVisionCapabilities(sessionId: string): Promise<VisionCapabilities> {
  return invoke("get_vision_capabilities", { sessionId });
}

/**
 * Send a multi-modal prompt with text and/or images to the AI agent.
 * If the provider doesn't support vision, images will be stripped and a warning emitted.
 *
 * @param sessionId - The session ID to send the prompt to
 * @param payload - The prompt payload with text and/or images
 */
export async function sendPromptWithAttachments(
  sessionId: string,
  payload: PromptPayload
): Promise<string> {
  return invoke("send_ai_prompt_with_attachments", { sessionId, payload });
}

/**
 * Helper to create a text-only prompt payload.
 */
export function createTextPayload(text: string): PromptPayload {
  return {
    parts: [{ type: "text", text }],
  };
}

/**
 * Helper to check if a payload contains any images.
 */
export function hasImages(payload: PromptPayload): boolean {
  return payload.parts.some((part) => part.type === "image");
}

/**
 * Helper to extract text content from a payload.
 */
export function extractText(payload: PromptPayload): string {
  return payload.parts
    .filter((part): part is TextPart => part.type === "text")
    .map((part) => part.text)
    .join("\n");
}

// =============================================================================
// Provider Configuration Builder
// =============================================================================

/**
 * Build a ProviderConfig for the given provider/model settings.
 * This is used for initializing AI sessions with proper credentials.
 *
 * @param settings - The user's QbitSettings
 * @param workspace - The workspace/working directory path
 * @param overrides - Optional provider/model overrides (defaults to settings.ai.default_*)
 */
export async function buildProviderConfig(
  settings: QbitSettings,
  workspace: string,
  overrides?: { provider?: AiProvider | null; model?: string | null }
): Promise<ProviderConfig> {
  const default_provider = overrides?.provider ?? settings.ai.default_provider;
  const default_model = overrides?.model ?? settings.ai.default_model;

  switch (default_provider) {
    case "vertex_ai": {
      const { vertex_ai } = settings.ai;
      if (!vertex_ai.project_id) {
        throw new Error("Vertex AI project_id not configured");
      }
      return {
        provider: "vertex_ai",
        workspace,
        credentials_path: vertex_ai.credentials_path || null,
        project_id: vertex_ai.project_id,
        location: vertex_ai.location || "us-east5",
        model: default_model,
      };
    }

    case "anthropic": {
      const apiKey = settings.ai.anthropic.api_key || (await getAnthropicApiKey());
      if (!apiKey) throw new Error("Anthropic API key not configured");
      return { provider: "anthropic", workspace, model: default_model, api_key: apiKey };
    }

    case "openai": {
      const apiKey = settings.ai.openai.api_key || (await getOpenAiApiKey());
      if (!apiKey) throw new Error("OpenAI API key not configured");
      return { provider: "openai", workspace, model: default_model, api_key: apiKey };
    }

    case "openrouter": {
      const apiKey = settings.ai.openrouter.api_key || (await getOpenRouterApiKey());
      if (!apiKey) throw new Error("OpenRouter API key not configured");
      return { provider: "openrouter", workspace, model: default_model, api_key: apiKey };
    }

    case "ollama": {
      const baseUrl = settings.ai.ollama.base_url;
      return { provider: "ollama", workspace, model: default_model, base_url: baseUrl };
    }

    case "gemini": {
      const apiKey = settings.ai.gemini.api_key;
      if (!apiKey) throw new Error("Gemini API key not configured");
      return { provider: "gemini", workspace, model: default_model, api_key: apiKey };
    }

    case "groq": {
      const apiKey = settings.ai.groq.api_key;
      if (!apiKey) throw new Error("Groq API key not configured");
      return { provider: "groq", workspace, model: default_model, api_key: apiKey };
    }

    case "xai": {
      const apiKey = settings.ai.xai.api_key;
      if (!apiKey) throw new Error("xAI API key not configured");
      return { provider: "xai", workspace, model: default_model, api_key: apiKey };
    }

    case "zai": {
      const apiKey = settings.ai.zai.api_key;
      if (!apiKey) throw new Error("Z.AI API key not configured");
      return {
        provider: "zai",
        workspace,
        model: default_model,
        api_key: apiKey,
        use_coding_endpoint: settings.ai.zai.use_coding_endpoint,
      };
    }

    default:
      throw new Error(`Unknown provider: ${default_provider}`);
  }
}

// =============================================================================
// Isolated Commit Writer Agent
// =============================================================================

/**
 * Response from the commit message generator.
 */
export interface CommitMessageResponse {
  /** The generated commit summary (first line, max 72 chars) */
  summary: string;
  /** The generated commit description (optional, can be empty) */
  description: string;
}

/**
 * Generate a commit message using an isolated AI agent.
 *
 * This agent is completely separate from the main agent and sub-agents.
 * It cannot be called by other agents and has no tools - it simply
 * analyzes a diff and generates a conventional commit message.
 *
 * @param sessionId - The session ID to use for the LLM client
 * @param diff - The git diff to analyze
 * @param fileSummary - Optional summary of files changed
 * @returns The generated commit message with summary and description
 */
export async function generateCommitMessage(
  sessionId: string,
  diff: string,
  fileSummary?: string
): Promise<CommitMessageResponse> {
  return invoke("generate_commit_message", {
    sessionId,
    diff,
    fileSummary,
  });
}
