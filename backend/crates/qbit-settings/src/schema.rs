//! Settings schema definitions for Qbit configuration.
//!
//! All settings structs use `#[serde(default)]` to allow partial configuration files.
//! Missing fields are filled with sensible defaults.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use ts_rs::TS;

// =============================================================================
// Enums for type-safe settings
// =============================================================================

/// AI provider selection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export, export_to = "generated/")]
pub enum AiProvider {
    #[default]
    VertexAi,
    /// Google Gemini on Vertex AI (native Gemini models)
    VertexGemini,
    Openrouter,
    Anthropic,
    Openai,
    Ollama,
    Gemini,
    Groq,
    Xai,
    /// Z.AI via native SDK implementation
    ZaiSdk,
}

impl std::fmt::Display for AiProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            AiProvider::VertexAi => "vertex_ai",
            AiProvider::VertexGemini => "vertex_gemini",
            AiProvider::Openrouter => "openrouter",
            AiProvider::Anthropic => "anthropic",
            AiProvider::Openai => "openai",
            AiProvider::Ollama => "ollama",
            AiProvider::Gemini => "gemini",
            AiProvider::Groq => "groq",
            AiProvider::Xai => "xai",
            AiProvider::ZaiSdk => "zai_sdk",
        };
        write!(f, "{}", s)
    }
}

impl std::str::FromStr for AiProvider {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "vertex_ai" | "vertex" => Ok(AiProvider::VertexAi),
            "vertex_gemini" => Ok(AiProvider::VertexGemini),
            "openrouter" => Ok(AiProvider::Openrouter),
            "anthropic" => Ok(AiProvider::Anthropic),
            "openai" => Ok(AiProvider::Openai),
            "ollama" => Ok(AiProvider::Ollama),
            "gemini" => Ok(AiProvider::Gemini),
            "groq" => Ok(AiProvider::Groq),
            "xai" => Ok(AiProvider::Xai),
            "z_ai_sdk" | "zai_sdk" | "zai" | "z_ai" | "zhipu" => Ok(AiProvider::ZaiSdk),
            _ => Err(format!("Invalid AI provider: {}", s)),
        }
    }
}

/// UI theme selection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Theme {
    #[default]
    Dark,
    Light,
    System,
}

impl std::fmt::Display for Theme {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Theme::Dark => "dark",
            Theme::Light => "light",
            Theme::System => "system",
        };
        write!(f, "{}", s)
    }
}

/// Logging level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum LogLevel {
    Error,
    Warn,
    #[default]
    Info,
    Debug,
    Trace,
}

/// Index storage location configuration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum IndexLocation {
    /// Store indexes globally in ~/.qbit/<codebase-name>/index (new default)
    #[default]
    Global,
    /// Store indexes locally in <workspace>/.qbit/index (legacy behavior)
    Local,
}

impl std::fmt::Display for LogLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            LogLevel::Error => "error",
            LogLevel::Warn => "warn",
            LogLevel::Info => "info",
            LogLevel::Debug => "debug",
            LogLevel::Trace => "trace",
        };
        write!(f, "{}", s)
    }
}

/// Reasoning effort level for models that support it (e.g., OpenAI o-series, GPT-5)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ReasoningEffort {
    Low,
    Medium,
    High,
}

impl std::fmt::Display for ReasoningEffort {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            ReasoningEffort::Low => "low",
            ReasoningEffort::Medium => "medium",
            ReasoningEffort::High => "high",
        };
        write!(f, "{}", s)
    }
}

// =============================================================================
// Settings structs
// =============================================================================

/// Root settings structure for Qbit.
///
/// Loaded from `~/.qbit/settings.toml` with environment variable interpolation support.
/// Version field enables future migrations.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct QbitSettings {
    /// Schema version for migrations
    pub version: u32,

    /// AI provider configuration
    pub ai: AiSettings,

    /// API keys for external services
    pub api_keys: ApiKeysSettings,

    /// Tool enablement settings
    #[serde(default)]
    pub tools: ToolsSettings,

    /// User interface preferences
    pub ui: UiSettings,

    /// Terminal configuration
    pub terminal: TerminalSettings,

    /// Agent behavior settings
    pub agent: AgentSettings,

    /// MCP server definitions
    #[serde(default)]
    pub mcp_servers: HashMap<String, McpServerConfig>,

    /// Repository trust levels
    #[serde(default)]
    pub trust: TrustSettings,

    /// Privacy and telemetry settings
    pub privacy: PrivacySettings,

    /// Advanced/debug settings
    pub advanced: AdvancedSettings,

    /// Sidecar context capture settings
    pub sidecar: SidecarSettings,

    /// Code indexer settings
    pub indexer: IndexerSettings,

    /// Context window management settings
    pub context: ContextSettings,

    /// Telemetry and observability settings
    pub telemetry: TelemetrySettings,

    /// Native OS notification settings
    #[serde(default)]
    pub notifications: NotificationsSettings,

    /// List of indexed codebase paths (deprecated, migrated to `codebases`)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub indexed_codebases: Vec<String>,

    /// Indexed codebases with configuration (new format)
    #[serde(default)]
    pub codebases: Vec<CodebaseConfig>,
}

/// Per-sub-agent model configuration.
///
/// Allows overriding the model used for specific sub-agents (e.g., "coder", "analyzer").
/// When both provider and model are None, the sub-agent inherits the main agent's model.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SubAgentModelConfig {
    /// Provider override (None = inherit from main agent)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<AiProvider>,

    /// Model override (None = inherit from main agent)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
}

/// AI provider configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AiSettings {
    /// Default AI provider
    pub default_provider: AiProvider,

    /// Default model for the selected provider
    pub default_model: String,

    /// Default reasoning effort for models that support it
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_reasoning_effort: Option<ReasoningEffort>,

    /// Per-sub-agent model overrides (key = sub-agent id: "coder", "analyzer", etc.)
    ///
    /// Example in settings.toml:
    /// ```toml
    /// [ai.sub_agent_models.coder]
    /// provider = "openai"
    /// model = "gpt-4o"
    /// ```
    #[serde(default)]
    pub sub_agent_models: HashMap<String, SubAgentModelConfig>,

    /// Model to use for the summarizer agent.
    /// If not specified, uses the session's current model.
    /// Example: "claude-sonnet-4-20250514"
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summarizer_model: Option<String>,

    /// Vertex AI (Anthropic) specific settings
    pub vertex_ai: VertexAiSettings,

    /// Vertex AI Gemini specific settings
    pub vertex_gemini: VertexGeminiSettings,

    /// OpenRouter specific settings
    pub openrouter: OpenRouterSettings,

    /// Direct Anthropic API settings
    pub anthropic: AnthropicSettings,

    /// OpenAI settings
    pub openai: OpenAiSettings,

    /// Ollama settings
    pub ollama: OllamaSettings,

    /// Gemini settings
    pub gemini: GeminiSettings,

    /// Groq settings
    pub groq: GroqSettings,

    /// xAI (Grok) settings
    pub xai: XaiSettings,

    /// Z.AI native SDK settings
    #[serde(alias = "z_ai_sdk")]
    pub zai_sdk: ZaiSdkSettings,
}

/// Vertex AI (Anthropic on Google Cloud) settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct VertexAiSettings {
    /// Path to service account JSON credentials
    #[serde(skip_serializing_if = "Option::is_none")]
    pub credentials_path: Option<String>,

    /// Google Cloud project ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_id: Option<String>,

    /// Vertex AI region (e.g., "us-east5")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<String>,

    /// Whether to show this provider's models in the model selector
    #[serde(default = "default_true")]
    pub show_in_selector: bool,
}

/// Vertex AI Gemini (native Google Gemini on Vertex AI) settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct VertexGeminiSettings {
    /// Path to service account JSON credentials
    #[serde(skip_serializing_if = "Option::is_none")]
    pub credentials_path: Option<String>,

    /// Google Cloud project ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_id: Option<String>,

    /// Vertex AI region (e.g., "us-central1")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<String>,

    /// Whether to show this provider's models in the model selector
    #[serde(default = "default_true")]
    pub show_in_selector: bool,

    /// Whether to include thoughts in the response (for thinking models)
    #[serde(default)]
    pub include_thoughts: bool,
}

/// OpenRouter API settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct OpenRouterSettings {
    /// OpenRouter API key (supports $ENV_VAR syntax)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,

    /// Whether to show this provider's models in the model selector
    #[serde(default = "default_true")]
    pub show_in_selector: bool,
}

/// Direct Anthropic API settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AnthropicSettings {
    /// Anthropic API key (supports $ENV_VAR syntax)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,

    /// Whether to show this provider's models in the model selector
    #[serde(default = "default_true")]
    pub show_in_selector: bool,
}

/// OpenAI API settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct OpenAiSettings {
    /// OpenAI API key (supports $ENV_VAR syntax)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,

    /// Custom base URL for OpenAI-compatible APIs
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,

    /// Whether to show this provider's models in the model selector
    #[serde(default = "default_true")]
    pub show_in_selector: bool,

    /// Enable OpenAI's native web search tool (web_search_preview).
    ///
    /// When enabled, OpenAI models will use server-side web search
    /// similar to Claude's native web tools, instead of Tavily.
    #[serde(default)]
    pub enable_web_search: bool,

    /// Web search context size: "low", "medium", or "high".
    ///
    /// - "low": Faster and cheaper, but may be less accurate
    /// - "medium": Balanced (default)
    /// - "high": Better results, but slower and more expensive
    #[serde(default = "default_web_search_context_size")]
    pub web_search_context_size: String,
}

/// Ollama local LLM settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct OllamaSettings {
    /// Ollama server URL
    pub base_url: String,

    /// Whether to show this provider's models in the model selector
    #[serde(default = "default_true")]
    pub show_in_selector: bool,
}

/// Gemini API settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct GeminiSettings {
    /// Gemini API key (supports $ENV_VAR syntax)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,

    /// Whether to show this provider's models in the model selector
    #[serde(default = "default_true")]
    pub show_in_selector: bool,

    /// Whether to include thoughts in the response (for thinking models)
    #[serde(default)]
    pub include_thoughts: bool,
}

/// Groq API settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct GroqSettings {
    /// Groq API key (supports $ENV_VAR syntax)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,

    /// Whether to show this provider's models in the model selector
    #[serde(default = "default_true")]
    pub show_in_selector: bool,
}

/// xAI (Grok) API settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct XaiSettings {
    /// xAI API key (supports $ENV_VAR syntax)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,

    /// Whether to show this provider's models in the model selector
    #[serde(default = "default_true")]
    pub show_in_selector: bool,
}

/// Z.AI native SDK settings.
///
/// Uses the native Z.AI API via the rig-zai-sdk crate.
/// Default endpoint: https://api.z.ai/api/paas/v4
/// Coding endpoint: https://api.z.ai/api/coding/paas/v4 (for GLM Coding Plan)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ZaiSdkSettings {
    /// Z.AI API key (supports $ENV_VAR syntax)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,

    /// Custom base URL (if None, uses default Z.AI endpoint)
    /// Use "https://api.z.ai/api/coding/paas/v4" for the coding-optimized endpoint
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,

    /// Default model to use
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,

    /// Whether to show this provider's models in the model selector
    #[serde(default = "default_true")]
    pub show_in_selector: bool,
}

/// API keys for external services.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct ApiKeysSettings {
    /// Tavily API key for web search
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tavily: Option<String>,

    /// GitHub token for repository access
    #[serde(skip_serializing_if = "Option::is_none")]
    pub github: Option<String>,
}

/// Tool enablement settings.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ToolsSettings {
    /// Enable web search tools (Tavily)
    pub web_search: bool,
}

/// User interface preferences.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct UiSettings {
    /// Theme
    pub theme: Theme,

    /// Show tips on startup
    pub show_tips: bool,

    /// Hide banner/welcome message
    pub hide_banner: bool,

    /// Window state (persisted on close/resize)
    #[serde(default)]
    pub window: WindowSettings,
}

/// Window state settings (persisted across sessions).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct WindowSettings {
    /// Window width in pixels
    pub width: u32,

    /// Window height in pixels
    pub height: u32,

    /// Window X position (None = centered)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub x: Option<i32>,

    /// Window Y position (None = centered)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub y: Option<i32>,

    /// Whether the window is maximized
    pub maximized: bool,
}

/// Terminal configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct TerminalSettings {
    /// Default shell override
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shell: Option<String>,

    /// Font family
    pub font_family: String,

    /// Font size in pixels
    pub font_size: u32,

    /// Scrollback buffer lines
    pub scrollback: u32,

    /// Additional commands that trigger fullterm mode.
    /// These are merged with the built-in defaults (claude, cc, codex, etc.).
    /// Most TUI apps are auto-detected via ANSI sequences; this is for edge cases.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fullterm_commands: Vec<String>,
}

/// Agent behavior settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AgentSettings {
    /// Auto-save conversations
    pub session_persistence: bool,

    /// Session retention in days (0 = forever)
    pub session_retention_days: u32,

    /// Enable pattern learning for auto-approval
    pub pattern_learning: bool,

    /// Minimum approvals before auto-approve
    pub min_approvals_for_auto: u32,

    /// Approval rate threshold (0.0 - 1.0)
    pub approval_threshold: f64,
}

/// MCP (Model Context Protocol) server configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct McpServerConfig {
    /// Command to start the server
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,

    /// Arguments for the command
    #[serde(default)]
    pub args: Vec<String>,

    /// Environment variables for the server
    #[serde(default)]
    pub env: HashMap<String, String>,

    /// URL for HTTP-based MCP servers
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

/// Repository trust settings.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct TrustSettings {
    /// Paths with full trust (all tools allowed)
    #[serde(default)]
    pub full_trust: Vec<String>,

    /// Paths with read-only trust
    #[serde(default)]
    pub read_only_trust: Vec<String>,

    /// Paths that are never trusted
    #[serde(default)]
    pub never_trust: Vec<String>,

    /// Additional paths accessible outside workspace (supports glob patterns)
    /// Example: ["~/Documents/*", "/tmp/scratch"]
    #[serde(default)]
    pub allowed_paths: Vec<String>,

    /// Disable workspace path restrictions entirely (use with caution)
    #[serde(default)]
    pub disable_path_restrictions: bool,
}

/// Privacy and telemetry settings.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct PrivacySettings {
    /// Enable anonymous usage statistics
    pub usage_statistics: bool,

    /// Log prompts for debugging (local only)
    pub log_prompts: bool,
}

/// Advanced/debug settings.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct AdvancedSettings {
    /// Enable experimental features
    pub enable_experimental: bool,

    /// Log level
    pub log_level: LogLevel,

    /// Enable LLM API request/response logging to ./logs/api/
    /// When enabled, raw JSON request/response data is logged per session
    pub enable_llm_api_logs: bool,

    /// Extract and parse the raw SSE JSON instead of logging escaped strings
    /// When enabled, SSE chunks are logged as parsed JSON objects
    pub extract_raw_sse: bool,
}

/// Code indexer settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct IndexerSettings {
    /// Where to store index files: "global" or "local"
    pub index_location: IndexLocation,
}

/// Telemetry and observability settings.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct TelemetrySettings {
    /// Langfuse integration settings
    pub langfuse: LangfuseSettings,
}

/// Langfuse tracing configuration.
///
/// Langfuse provides LLM observability via OpenTelemetry.
/// See: https://langfuse.com/docs/integrations/opentelemetry
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct LangfuseSettings {
    /// Enable Langfuse tracing
    pub enabled: bool,

    /// Langfuse host URL (defaults to https://cloud.langfuse.com)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub host: Option<String>,

    /// Langfuse public key (supports $ENV_VAR syntax, or set LANGFUSE_PUBLIC_KEY env var)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub public_key: Option<String>,

    /// Langfuse secret key (supports $ENV_VAR syntax, or set LANGFUSE_SECRET_KEY env var)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub secret_key: Option<String>,

    /// Sampling ratio (0.0 to 1.0, default 1.0 = sample everything)
    /// Use lower values for high-traffic production deployments
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sampling_ratio: Option<f64>,
}

/// Context window management settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ContextSettings {
    /// Enable context window management
    #[serde(default = "default_context_enabled")]
    pub enabled: bool,

    /// Context utilization threshold (0.0-1.0) at which compaction is triggered
    #[serde(default = "default_compaction_threshold")]
    pub compaction_threshold: f64,

    /// DEPRECATED: No longer used. Compaction replaces pruning.
    /// Kept for backwards compatibility with existing config files.
    #[serde(default = "default_protected_turns")]
    pub protected_turns: usize,

    /// DEPRECATED: No longer used. Compaction replaces pruning.
    /// Kept for backwards compatibility with existing config files.
    #[serde(default = "default_cooldown_seconds")]
    pub cooldown_seconds: u64,
}

/// Native OS notification settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct NotificationsSettings {
    /// Enable native OS notifications for agent/command completion
    pub native_enabled: bool,

    /// Enable in-app notification sounds (independent of OS notifications).
    /// Defaults to true.
    pub sound_enabled: bool,

    /// Notification sound (macOS system sound name like "Blow" or "Ping").
    /// If None, defaults to "Blow" on macOS and no sound on other platforms.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sound: Option<String>,
}

impl Default for NotificationsSettings {
    fn default() -> Self {
        Self {
            native_enabled: false,
            sound_enabled: true,
            sound: None,
        }
    }
}

impl Default for IndexerSettings {
    fn default() -> Self {
        Self {
            index_location: IndexLocation::Global,
        }
    }
}

impl Default for ContextSettings {
    fn default() -> Self {
        Self {
            enabled: default_context_enabled(),
            compaction_threshold: default_compaction_threshold(),
            protected_turns: default_protected_turns(),
            cooldown_seconds: default_cooldown_seconds(),
        }
    }
}

/// Configuration for an indexed codebase.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CodebaseConfig {
    /// Path to the codebase (supports ~ for home directory)
    pub path: String,

    /// Memory file associated with this codebase: "AGENTS.md", "CLAUDE.md", or None
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory_file: Option<String>,
}

/// Sidecar context capture settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SidecarSettings {
    /// Enable context capture during AI sessions
    pub enabled: bool,

    /// Enable LLM synthesis for commit messages and summaries
    pub synthesis_enabled: bool,

    /// Synthesis backend: "local" | "vertex_anthropic" | "openai" | "grok" | "template"
    pub synthesis_backend: String,

    /// Vertex AI settings for synthesis (when synthesis_backend = "vertex_anthropic")
    pub synthesis_vertex: SynthesisVertexSettings,

    /// OpenAI settings for synthesis (when synthesis_backend = "openai")
    pub synthesis_openai: SynthesisOpenAiSettings,

    /// Grok settings for synthesis (when synthesis_backend = "grok")
    pub synthesis_grok: SynthesisGrokSettings,

    /// Event retention in days (0 = forever)
    pub retention_days: u32,

    /// Capture tool call events
    pub capture_tool_calls: bool,

    /// Capture agent reasoning events
    pub capture_reasoning: bool,
}

/// Vertex AI settings for sidecar synthesis.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SynthesisVertexSettings {
    /// Google Cloud project ID (falls back to ai.vertex_ai.project_id if not set)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_id: Option<String>,

    /// Vertex AI region (falls back to ai.vertex_ai.location if not set)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<String>,

    /// Model to use for synthesis (default: claude-sonnet-4-20250514)
    pub model: String,

    /// Path to credentials (falls back to ai.vertex_ai.credentials_path if not set)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub credentials_path: Option<String>,
}

/// OpenAI settings for sidecar synthesis.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SynthesisOpenAiSettings {
    /// API key (falls back to api_keys or env var)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,

    /// Model to use for synthesis (default: gpt-4o-mini)
    pub model: String,

    /// Custom base URL for OpenAI-compatible APIs
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,
}

/// Grok settings for sidecar synthesis.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SynthesisGrokSettings {
    /// API key (falls back to env var GROK_API_KEY)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,

    /// Model to use for synthesis (default: grok-2)
    pub model: String,
}

// =============================================================================
// Helper functions for serde defaults
// =============================================================================

fn default_true() -> bool {
    true
}

fn default_context_enabled() -> bool {
    true
}

fn default_compaction_threshold() -> f64 {
    0.80
}

fn default_protected_turns() -> usize {
    2
}

fn default_cooldown_seconds() -> u64 {
    60
}

fn default_web_search_context_size() -> String {
    "medium".to_string()
}

// =============================================================================
// Default implementations
// =============================================================================

impl Default for QbitSettings {
    fn default() -> Self {
        Self {
            version: 1,
            ai: AiSettings::default(),
            api_keys: ApiKeysSettings::default(),
            tools: ToolsSettings::default(),
            ui: UiSettings::default(),
            terminal: TerminalSettings::default(),
            agent: AgentSettings::default(),
            mcp_servers: HashMap::new(),
            trust: TrustSettings::default(),
            privacy: PrivacySettings::default(),
            advanced: AdvancedSettings::default(),
            sidecar: SidecarSettings::default(),
            indexer: IndexerSettings::default(),
            context: ContextSettings::default(),
            telemetry: TelemetrySettings::default(),
            notifications: NotificationsSettings::default(),
            indexed_codebases: Vec::new(),
            codebases: Vec::new(),
        }
    }
}

impl Default for AiSettings {
    fn default() -> Self {
        Self {
            default_provider: AiProvider::default(),
            default_model: "claude-opus-4-5@20251101".to_string(),
            default_reasoning_effort: None,
            sub_agent_models: HashMap::new(),
            summarizer_model: None,
            vertex_ai: VertexAiSettings::default(),
            vertex_gemini: VertexGeminiSettings::default(),
            openrouter: OpenRouterSettings::default(),
            anthropic: AnthropicSettings::default(),
            openai: OpenAiSettings::default(),
            ollama: OllamaSettings::default(),
            gemini: GeminiSettings::default(),
            groq: GroqSettings::default(),
            xai: XaiSettings::default(),
            zai_sdk: ZaiSdkSettings::default(),
        }
    }
}

impl Default for VertexAiSettings {
    fn default() -> Self {
        Self {
            credentials_path: None,
            project_id: None,
            location: None,
            show_in_selector: true,
        }
    }
}

impl Default for VertexGeminiSettings {
    fn default() -> Self {
        Self {
            credentials_path: None,
            project_id: None,
            location: None,
            show_in_selector: true,
            include_thoughts: false,
        }
    }
}

impl Default for OpenRouterSettings {
    fn default() -> Self {
        Self {
            api_key: None,
            show_in_selector: true,
        }
    }
}

impl Default for AnthropicSettings {
    fn default() -> Self {
        Self {
            api_key: None,
            show_in_selector: true,
        }
    }
}

impl Default for OpenAiSettings {
    fn default() -> Self {
        Self {
            api_key: None,
            base_url: None,
            show_in_selector: true,
            enable_web_search: false,
            web_search_context_size: "medium".to_string(),
        }
    }
}

impl Default for OllamaSettings {
    fn default() -> Self {
        Self {
            base_url: "http://localhost:11434".to_string(),
            show_in_selector: true,
        }
    }
}

impl Default for GeminiSettings {
    fn default() -> Self {
        Self {
            api_key: None,
            show_in_selector: true,
            include_thoughts: false,
        }
    }
}

impl Default for GroqSettings {
    fn default() -> Self {
        Self {
            api_key: None,
            show_in_selector: true,
        }
    }
}

impl Default for XaiSettings {
    fn default() -> Self {
        Self {
            api_key: None,
            show_in_selector: true,
        }
    }
}

impl Default for ZaiSdkSettings {
    fn default() -> Self {
        Self {
            api_key: None,
            base_url: None,
            model: None,
            show_in_selector: true,
        }
    }
}

impl Default for UiSettings {
    fn default() -> Self {
        Self {
            theme: Theme::default(),
            show_tips: true,
            hide_banner: false,
            window: WindowSettings::default(),
        }
    }
}

impl Default for WindowSettings {
    fn default() -> Self {
        Self {
            width: 1400,
            height: 900,
            x: None,
            y: None,
            maximized: false,
        }
    }
}

impl Default for TerminalSettings {
    fn default() -> Self {
        Self {
            shell: None,
            font_family: "JetBrains Mono".to_string(),
            font_size: 14,
            scrollback: 10000,
            fullterm_commands: Vec::new(),
        }
    }
}

impl Default for AgentSettings {
    fn default() -> Self {
        Self {
            session_persistence: true,
            session_retention_days: 30,
            pattern_learning: true,
            min_approvals_for_auto: 3,
            approval_threshold: 0.8,
        }
    }
}

impl Default for SidecarSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            synthesis_enabled: true,
            synthesis_backend: "template".to_string(),
            synthesis_vertex: SynthesisVertexSettings::default(),
            synthesis_openai: SynthesisOpenAiSettings::default(),
            synthesis_grok: SynthesisGrokSettings::default(),
            retention_days: 30,
            capture_tool_calls: true,
            capture_reasoning: true,
        }
    }
}

impl Default for SynthesisVertexSettings {
    fn default() -> Self {
        Self {
            project_id: None,
            location: None,
            model: "claude-haiku-4-5@20251001".to_string(),
            credentials_path: None,
        }
    }
}

impl Default for SynthesisOpenAiSettings {
    fn default() -> Self {
        Self {
            api_key: None,
            model: "gpt-4o-mini".to_string(),
            base_url: None,
        }
    }
}

impl Default for SynthesisGrokSettings {
    fn default() -> Self {
        Self {
            api_key: None,
            model: "grok-2".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_settings() {
        let settings = QbitSettings::default();
        assert_eq!(settings.version, 1);
        assert_eq!(settings.ai.default_provider, AiProvider::VertexAi);
        assert_eq!(settings.ai.default_model, "claude-opus-4-5@20251101");
        assert_eq!(settings.ui.theme, Theme::Dark);
        assert_eq!(settings.advanced.log_level, LogLevel::Info);
        assert_eq!(settings.terminal.font_size, 14);
        assert!(settings.agent.session_persistence);
    }

    #[test]
    fn test_parse_minimal_toml() {
        let toml = r#"
            version = 1
            [ai]
            default_provider = "openrouter"
        "#;

        let settings: QbitSettings = toml::from_str(toml).unwrap();
        assert_eq!(settings.ai.default_provider, AiProvider::Openrouter);
        // Defaults should fill in missing fields
        assert_eq!(settings.terminal.font_size, 14);
    }

    #[test]
    fn test_serialize_settings() {
        let settings = QbitSettings::default();
        let toml_str = toml::to_string_pretty(&settings).unwrap();
        assert!(toml_str.contains("version = 1"));
        assert!(toml_str.contains("[ai]"));
    }

    #[test]
    fn test_context_settings_defaults() {
        let context = ContextSettings::default();
        assert!(context.enabled);
        assert!((context.compaction_threshold - 0.80).abs() < f64::EPSILON);
        assert_eq!(context.protected_turns, 2);
        assert_eq!(context.cooldown_seconds, 60);
    }

    #[test]
    fn test_context_settings_deserialize_from_toml() {
        let toml = r#"
            [context]
            enabled = false
            compaction_threshold = 0.75
            protected_turns = 3
            cooldown_seconds = 120
        "#;

        let settings: QbitSettings = toml::from_str(toml).unwrap();
        assert!(!settings.context.enabled);
        assert!((settings.context.compaction_threshold - 0.75).abs() < f64::EPSILON);
        assert_eq!(settings.context.protected_turns, 3);
        assert_eq!(settings.context.cooldown_seconds, 120);
    }

    #[test]
    fn test_context_settings_missing_section_uses_defaults() {
        // Test backward compatibility: missing [context] section should use defaults
        let toml = r#"
            version = 1
            [ai]
            default_provider = "anthropic"
        "#;

        let settings: QbitSettings = toml::from_str(toml).unwrap();
        // Context settings should have defaults
        assert!(settings.context.enabled);
        assert!((settings.context.compaction_threshold - 0.80).abs() < f64::EPSILON);
        assert_eq!(settings.context.protected_turns, 2);
        assert_eq!(settings.context.cooldown_seconds, 60);
    }

    #[test]
    fn test_context_settings_partial_section_fills_defaults() {
        // Test that partial [context] section fills in missing fields with defaults
        let toml = r#"
            [context]
            enabled = false
        "#;

        let settings: QbitSettings = toml::from_str(toml).unwrap();
        assert!(!settings.context.enabled);
        // Other fields should have defaults
        assert!((settings.context.compaction_threshold - 0.80).abs() < f64::EPSILON);
        assert_eq!(settings.context.protected_turns, 2);
        assert_eq!(settings.context.cooldown_seconds, 60);
    }

    #[test]
    fn test_summarizer_model_setting() {
        let toml = r#"
            [ai]
            summarizer_model = "claude-haiku-4-5@20251001"
        "#;

        let settings: QbitSettings = toml::from_str(toml).unwrap();
        assert_eq!(
            settings.ai.summarizer_model,
            Some("claude-haiku-4-5@20251001".to_string())
        );
    }

    #[test]
    fn test_summarizer_model_defaults_to_none() {
        let toml = r#"
            [ai]
            default_provider = "anthropic"
        "#;

        let settings: QbitSettings = toml::from_str(toml).unwrap();
        assert!(settings.ai.summarizer_model.is_none());
    }
}
