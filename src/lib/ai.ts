import { invoke } from "@tauri-apps/api/core";
import { listen as tauriListen, type UnlistenFn } from "@tauri-apps/api/event";
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
  | "xai";

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
      credentials_path: string;
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
        tokens_used?: number;
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
      }
    | {
        type: "sub_agent_tool_request";
        agent_id: string;
        tool_name: string;
        args: unknown;
      }
    | {
        type: "sub_agent_tool_result";
        agent_id: string;
        tool_name: string;
        success: boolean;
      }
    | {
        type: "sub_agent_completed";
        agent_id: string;
        response: string;
        duration_ms: number;
      }
    | {
        type: "sub_agent_error";
        agent_id: string;
        error: string;
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
 * Context information to inject into user messages.
 * This context is prepended as XML tags and not shown to the user.
 */
export interface PromptContext {
  /** The current working directory in the terminal */
  workingDirectory?: string;
  /** The session ID of the user's terminal (for running commands in the same terminal) */
  sessionId?: string;
}

/**
 * Send a prompt to the AI agent
 * Response will be streamed via the ai-event listener
 *
 * @param prompt - The user's message
 * @param context - Optional context to inject (working directory, etc.)
 */
export async function sendPrompt(prompt: string, context?: PromptContext): Promise<string> {
  // Convert to snake_case for Rust backend
  const contextPayload = context
    ? {
        working_directory: context.workingDirectory,
        session_id: context.sessionId,
      }
    : undefined;

  return invoke("send_ai_prompt", { prompt, context: contextPayload });
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
 * @param context - Optional context to inject (working directory, etc.)
 */
export async function sendPromptSession(
  sessionId: string,
  prompt: string,
  context?: PromptContext
): Promise<string> {
  // Nested struct fields use snake_case (serde default)
  const contextPayload = context
    ? {
        working_directory: context.workingDirectory,
        session_id: context.sessionId,
      }
    : undefined;

  return invoke("send_ai_prompt_session", {
    sessionId,
    prompt,
    context: contextPayload,
  });
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
 */
export const OPENAI_MODELS = {
  GPT_5_2: "gpt-5.2",
} as const;

/**
 * Available Claude models via direct Anthropic API.
 */
export const ANTHROPIC_MODELS = {
  CLAUDE_OPUS_4_5: "claude-opus-4-5-20251101",
  CLAUDE_SONNET_4_5: "claude-sonnet-4-5-20250514",
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
export async function restoreAiSession(identifier: string): Promise<SessionSnapshot> {
  return invoke("restore_ai_session", { identifier });
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
 */
export async function setAgentMode(sessionId: string, mode: AgentMode): Promise<void> {
  return invoke("set_agent_mode", { sessionId, mode });
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
