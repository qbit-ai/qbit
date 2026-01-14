//! Agent bridge for LLM interaction.
//!
//! This module provides the main AgentBridge struct that orchestrates:
//! - LLM communication (vtcode-core and Vertex AI Anthropic)
//! - Tool execution with HITL approval
//! - Conversation history management
//! - Session persistence
//! - Context window management
//! - Loop detection
//!
//! The implementation is split across multiple extension modules:
//! - `bridge_session` - Session persistence and conversation history
//! - `bridge_hitl` - HITL approval handling
//! - `bridge_policy` - Tool policies and loop protection
//! - `bridge_context` - Context window management
//!
//! Core execution logic is in:
//! - `agentic_loop` - Main tool execution loop
//! - `system_prompt` - System prompt building
//! - `sub_agent_executor` - Sub-agent execution

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use rig::completion::Message;
use rig::message::{Text, UserContent};
use rig::one_or_many::OneOrMany;
use rig::providers::anthropic as rig_anthropic;
use rig::providers::gemini as rig_gemini;
use rig::providers::groq as rig_groq;
use rig::providers::ollama as rig_ollama;
use rig::providers::openai as rig_openai;
use rig::providers::openrouter as rig_openrouter;
use rig::providers::xai as rig_xai;
use tokio::sync::{mpsc, oneshot, RwLock};

use qbit_tools::ToolRegistry;

use qbit_core::events::AiEvent;
use qbit_core::hitl::ApprovalDecision;
use qbit_hitl::ApprovalRecorder;

use super::agent_mode::AgentMode;
use super::agentic_loop::{run_agentic_loop, run_agentic_loop_generic, AgenticLoopContext};
use super::contributors::create_default_contributors;
use super::llm_client::{
    create_anthropic_components, create_gemini_components, create_groq_components,
    create_ollama_components, create_openai_components, create_openrouter_components,
    create_vertex_components, create_xai_components, create_zai_anthropic_components,
    create_zai_components, AgentBridgeComponents, AnthropicClientConfig, GeminiClientConfig,
    GroqClientConfig, LlmClient, OllamaClientConfig, OpenAiClientConfig, OpenRouterClientConfig,
    SharedComponentsConfig, VertexAnthropicClientConfig, XaiClientConfig, ZaiAnthropicClientConfig,
    ZaiClientConfig,
};
use super::prompt_registry::PromptContributorRegistry;
use super::system_prompt::build_system_prompt_with_contributions;
use super::tool_definitions::ToolConfig;
use qbit_context::token_budget::TokenUsage;
use qbit_context::{CompactionState, ContextManager, ContextManagerConfig};
use qbit_core::runtime::{QbitRuntime, RuntimeEvent};
use qbit_core::PromptContext;
use qbit_loop_detection::LoopDetector;
use qbit_session::QbitSessionManager;
use qbit_sub_agents::{SubAgentContext, SubAgentRegistry, MAX_AGENT_DEPTH};
use qbit_tool_policy::ToolPolicyManager;

use qbit_indexer::IndexerState;
use qbit_planner::PlanManager;
#[cfg(any(feature = "tauri", feature = "cli"))]
use qbit_pty::PtyManager;
use qbit_sidecar::SidecarState;

use crate::transcript::TranscriptWriter;

/// Bridge between Qbit and LLM providers.
/// Handles LLM streaming and tool execution.
pub struct AgentBridge {
    // Core fields
    pub(crate) workspace: Arc<RwLock<PathBuf>>,
    pub(crate) provider_name: String,
    pub(crate) model_name: String,
    pub(crate) tool_registry: Arc<RwLock<ToolRegistry>>,
    pub(crate) client: Arc<RwLock<LlmClient>>,

    // Event emission - dual mode during transition
    // The event_tx channel is the legacy path, runtime is the new abstraction.
    // During transition, emit_event() sends through BOTH to verify parity.
    pub(crate) event_tx: Option<mpsc::UnboundedSender<AiEvent>>,
    pub(crate) runtime: Option<Arc<dyn QbitRuntime>>,
    /// Session ID for event routing (set for per-session bridges)
    pub(crate) event_session_id: Option<String>,

    // Sub-agents
    pub(crate) sub_agent_registry: Arc<RwLock<SubAgentRegistry>>,

    // Terminal integration
    #[cfg(any(feature = "tauri", feature = "cli"))]
    pub(crate) pty_manager: Option<Arc<PtyManager>>,
    pub(crate) current_session_id: Arc<RwLock<Option<String>>>,

    // Conversation state
    pub(crate) conversation_history: Arc<RwLock<Vec<Message>>>,

    // Session persistence
    pub(crate) session_manager: Arc<RwLock<Option<QbitSessionManager>>>,
    pub(crate) session_persistence_enabled: Arc<RwLock<bool>>,

    // HITL approval
    pub(crate) approval_recorder: Arc<ApprovalRecorder>,
    pub(crate) pending_approvals: Arc<RwLock<HashMap<String, oneshot::Sender<ApprovalDecision>>>>,

    // Tool policy
    pub(crate) tool_policy_manager: Arc<ToolPolicyManager>,

    // Context management
    pub(crate) context_manager: Arc<ContextManager>,

    // Compaction state for tracking token usage
    pub(crate) compaction_state: Arc<RwLock<CompactionState>>,

    // Loop detection
    pub(crate) loop_detector: Arc<RwLock<LoopDetector>>,

    // Tool configuration
    pub(crate) tool_config: ToolConfig,

    // Agent mode (controls tool approval behavior)
    pub(crate) agent_mode: Arc<RwLock<AgentMode>>,

    // Plan manager for update_plan tool
    pub(crate) plan_manager: Arc<PlanManager>,

    // Sidecar context capture
    pub(crate) sidecar_state: Option<Arc<SidecarState>>,

    // Memory file path for project instructions (from codebase settings)
    pub(crate) memory_file_path: Arc<RwLock<Option<PathBuf>>>,

    // Settings manager for dynamic memory file lookup
    pub(crate) settings_manager: Option<Arc<qbit_settings::SettingsManager>>,

    // OpenAI web search configuration (if enabled)
    pub(crate) openai_web_search_config: Option<qbit_llm_providers::OpenAiWebSearchConfig>,

    // Factory for creating sub-agent model override clients (optional)
    pub(crate) model_factory: Option<Arc<super::llm_client::LlmClientFactory>>,

    // External services
    pub(crate) indexer_state: Option<Arc<IndexerState>>,

    // Transcript writer for persisting AI events to JSONL
    pub(crate) transcript_writer: Option<Arc<TranscriptWriter>>,

    // Base directory for transcript files (e.g., `~/.qbit/transcripts`)
    // Used to create separate transcript files for sub-agent internal events.
    pub(crate) transcript_base_dir: Option<PathBuf>,
}

impl AgentBridge {
    // ========================================================================
    // Constructor Methods
    // ========================================================================

    /// Create a new AgentBridge for OpenRouter.
    ///
    /// Uses the `QbitRuntime` trait for event emission and approval handling.
    ///
    /// Note: For Vertex AI providers, use `new_vertex_anthropic_with_runtime` instead.
    pub async fn new_with_runtime(
        workspace: PathBuf,
        _provider: &str,
        model: &str,
        api_key: &str,
        runtime: Arc<dyn QbitRuntime>,
    ) -> Result<Self> {
        Self::new_openrouter_with_runtime(workspace, model, api_key, None, runtime).await
    }

    /// Create a new AgentBridge for OpenRouter with optional context config.
    pub async fn new_openrouter_with_runtime(
        workspace: PathBuf,
        model: &str,
        api_key: &str,
        context_config: Option<ContextManagerConfig>,
        runtime: Arc<dyn QbitRuntime>,
    ) -> Result<Self> {
        let shared_config = SharedComponentsConfig {
            context_config,
            settings: qbit_settings::QbitSettings::default(),
        };
        Self::new_openrouter_with_shared_config(workspace, model, api_key, shared_config, runtime)
            .await
    }

    /// Create a new AgentBridge for OpenRouter with full shared config.
    pub async fn new_openrouter_with_shared_config(
        workspace: PathBuf,
        model: &str,
        api_key: &str,
        shared_config: SharedComponentsConfig,
        runtime: Arc<dyn QbitRuntime>,
    ) -> Result<Self> {
        let config = OpenRouterClientConfig {
            workspace,
            model,
            api_key,
        };

        let components = create_openrouter_components(config, shared_config).await?;

        Ok(Self::from_components_with_runtime(components, runtime))
    }

    /// Create a new AgentBridge for Anthropic on Google Cloud Vertex AI.
    ///
    /// Uses the `QbitRuntime` trait for event emission and approval handling.
    /// If `credentials_path` is None, uses application default credentials.
    pub async fn new_vertex_anthropic_with_runtime(
        workspace: PathBuf,
        credentials_path: Option<&str>,
        project_id: &str,
        location: &str,
        model: &str,
        runtime: Arc<dyn QbitRuntime>,
    ) -> Result<Self> {
        Self::new_vertex_anthropic_with_context(
            workspace,
            credentials_path,
            project_id,
            location,
            model,
            None,
            runtime,
        )
        .await
    }

    /// Create a new AgentBridge for Anthropic on Vertex AI with optional context config.
    pub async fn new_vertex_anthropic_with_context(
        workspace: PathBuf,
        credentials_path: Option<&str>,
        project_id: &str,
        location: &str,
        model: &str,
        context_config: Option<ContextManagerConfig>,
        runtime: Arc<dyn QbitRuntime>,
    ) -> Result<Self> {
        let shared_config = SharedComponentsConfig {
            context_config,
            settings: qbit_settings::QbitSettings::default(),
        };
        Self::new_vertex_anthropic_with_shared_config(
            workspace,
            credentials_path,
            project_id,
            location,
            model,
            shared_config,
            runtime,
        )
        .await
    }

    /// Create a new AgentBridge for Anthropic on Vertex AI with full shared config.
    pub async fn new_vertex_anthropic_with_shared_config(
        workspace: PathBuf,
        credentials_path: Option<&str>,
        project_id: &str,
        location: &str,
        model: &str,
        shared_config: SharedComponentsConfig,
        runtime: Arc<dyn QbitRuntime>,
    ) -> Result<Self> {
        let config = VertexAnthropicClientConfig {
            workspace,
            credentials_path,
            project_id,
            location,
            model,
        };

        let components = create_vertex_components(config, shared_config).await?;

        Ok(Self::from_components_with_runtime(components, runtime))
    }

    /// Create a new AgentBridge for OpenAI.
    pub async fn new_openai_with_runtime(
        workspace: PathBuf,
        model: &str,
        api_key: &str,
        base_url: Option<&str>,
        reasoning_effort: Option<&str>,
        runtime: Arc<dyn QbitRuntime>,
    ) -> Result<Self> {
        Self::new_openai_with_context(
            workspace,
            model,
            api_key,
            base_url,
            reasoning_effort,
            None,
            runtime,
        )
        .await
    }

    /// Create a new AgentBridge for OpenAI with optional context config.
    pub async fn new_openai_with_context(
        workspace: PathBuf,
        model: &str,
        api_key: &str,
        base_url: Option<&str>,
        reasoning_effort: Option<&str>,
        context_config: Option<ContextManagerConfig>,
        runtime: Arc<dyn QbitRuntime>,
    ) -> Result<Self> {
        let shared_config = SharedComponentsConfig {
            context_config,
            settings: qbit_settings::QbitSettings::default(),
        };
        Self::new_openai_with_shared_config(
            workspace,
            model,
            api_key,
            base_url,
            reasoning_effort,
            shared_config,
            runtime,
        )
        .await
    }

    /// Create a new AgentBridge for OpenAI with full shared config.
    pub async fn new_openai_with_shared_config(
        workspace: PathBuf,
        model: &str,
        api_key: &str,
        base_url: Option<&str>,
        reasoning_effort: Option<&str>,
        shared_config: SharedComponentsConfig,
        runtime: Arc<dyn QbitRuntime>,
    ) -> Result<Self> {
        let config = OpenAiClientConfig {
            workspace,
            model,
            api_key,
            base_url,
            reasoning_effort,
            enable_web_search: false,
            web_search_context_size: "medium",
        };
        let components = create_openai_components(config, shared_config).await?;
        Ok(Self::from_components_with_runtime(components, runtime))
    }

    /// Create a new AgentBridge for direct Anthropic API.
    pub async fn new_anthropic_with_runtime(
        workspace: PathBuf,
        model: &str,
        api_key: &str,
        runtime: Arc<dyn QbitRuntime>,
    ) -> Result<Self> {
        Self::new_anthropic_with_context(workspace, model, api_key, None, runtime).await
    }

    /// Create a new AgentBridge for Anthropic with optional context config.
    pub async fn new_anthropic_with_context(
        workspace: PathBuf,
        model: &str,
        api_key: &str,
        context_config: Option<ContextManagerConfig>,
        runtime: Arc<dyn QbitRuntime>,
    ) -> Result<Self> {
        let shared_config = SharedComponentsConfig {
            context_config,
            settings: qbit_settings::QbitSettings::default(),
        };
        Self::new_anthropic_with_shared_config(workspace, model, api_key, shared_config, runtime)
            .await
    }

    /// Create a new AgentBridge for Anthropic with full shared config.
    pub async fn new_anthropic_with_shared_config(
        workspace: PathBuf,
        model: &str,
        api_key: &str,
        shared_config: SharedComponentsConfig,
        runtime: Arc<dyn QbitRuntime>,
    ) -> Result<Self> {
        let config = AnthropicClientConfig {
            workspace,
            model,
            api_key,
        };
        let components = create_anthropic_components(config, shared_config).await?;
        Ok(Self::from_components_with_runtime(components, runtime))
    }

    /// Create a new AgentBridge for Ollama local inference.
    pub async fn new_ollama_with_runtime(
        workspace: PathBuf,
        model: &str,
        base_url: Option<&str>,
        runtime: Arc<dyn QbitRuntime>,
    ) -> Result<Self> {
        Self::new_ollama_with_context(workspace, model, base_url, None, runtime).await
    }

    /// Create a new AgentBridge for Ollama with optional context config.
    pub async fn new_ollama_with_context(
        workspace: PathBuf,
        model: &str,
        base_url: Option<&str>,
        context_config: Option<ContextManagerConfig>,
        runtime: Arc<dyn QbitRuntime>,
    ) -> Result<Self> {
        let shared_config = SharedComponentsConfig {
            context_config,
            settings: qbit_settings::QbitSettings::default(),
        };
        Self::new_ollama_with_shared_config(workspace, model, base_url, shared_config, runtime)
            .await
    }

    /// Create a new AgentBridge for Ollama with full shared config.
    pub async fn new_ollama_with_shared_config(
        workspace: PathBuf,
        model: &str,
        base_url: Option<&str>,
        shared_config: SharedComponentsConfig,
        runtime: Arc<dyn QbitRuntime>,
    ) -> Result<Self> {
        let config = OllamaClientConfig {
            workspace,
            model,
            base_url,
        };
        let components = create_ollama_components(config, shared_config).await?;
        Ok(Self::from_components_with_runtime(components, runtime))
    }

    /// Create a new AgentBridge for Gemini.
    pub async fn new_gemini_with_runtime(
        workspace: PathBuf,
        model: &str,
        api_key: &str,
        runtime: Arc<dyn QbitRuntime>,
    ) -> Result<Self> {
        Self::new_gemini_with_context(workspace, model, api_key, None, runtime).await
    }

    /// Create a new AgentBridge for Gemini with optional context config.
    pub async fn new_gemini_with_context(
        workspace: PathBuf,
        model: &str,
        api_key: &str,
        context_config: Option<ContextManagerConfig>,
        runtime: Arc<dyn QbitRuntime>,
    ) -> Result<Self> {
        let shared_config = SharedComponentsConfig {
            context_config,
            settings: qbit_settings::QbitSettings::default(),
        };
        Self::new_gemini_with_shared_config(workspace, model, api_key, shared_config, runtime).await
    }

    /// Create a new AgentBridge for Gemini with full shared config.
    pub async fn new_gemini_with_shared_config(
        workspace: PathBuf,
        model: &str,
        api_key: &str,
        shared_config: SharedComponentsConfig,
        runtime: Arc<dyn QbitRuntime>,
    ) -> Result<Self> {
        let config = GeminiClientConfig {
            workspace,
            model,
            api_key,
        };
        let components = create_gemini_components(config, shared_config).await?;
        Ok(Self::from_components_with_runtime(components, runtime))
    }

    /// Create a new AgentBridge for Groq.
    pub async fn new_groq_with_runtime(
        workspace: PathBuf,
        model: &str,
        api_key: &str,
        runtime: Arc<dyn QbitRuntime>,
    ) -> Result<Self> {
        Self::new_groq_with_context(workspace, model, api_key, None, runtime).await
    }

    /// Create a new AgentBridge for Groq with optional context config.
    pub async fn new_groq_with_context(
        workspace: PathBuf,
        model: &str,
        api_key: &str,
        context_config: Option<ContextManagerConfig>,
        runtime: Arc<dyn QbitRuntime>,
    ) -> Result<Self> {
        let shared_config = SharedComponentsConfig {
            context_config,
            settings: qbit_settings::QbitSettings::default(),
        };
        Self::new_groq_with_shared_config(workspace, model, api_key, shared_config, runtime).await
    }

    /// Create a new AgentBridge for Groq with full shared config.
    pub async fn new_groq_with_shared_config(
        workspace: PathBuf,
        model: &str,
        api_key: &str,
        shared_config: SharedComponentsConfig,
        runtime: Arc<dyn QbitRuntime>,
    ) -> Result<Self> {
        let config = GroqClientConfig {
            workspace,
            model,
            api_key,
        };
        let components = create_groq_components(config, shared_config).await?;
        Ok(Self::from_components_with_runtime(components, runtime))
    }

    /// Create a new AgentBridge for xAI (Grok).
    pub async fn new_xai_with_runtime(
        workspace: PathBuf,
        model: &str,
        api_key: &str,
        runtime: Arc<dyn QbitRuntime>,
    ) -> Result<Self> {
        Self::new_xai_with_context(workspace, model, api_key, None, runtime).await
    }

    /// Create a new AgentBridge for xAI with optional context config.
    pub async fn new_xai_with_context(
        workspace: PathBuf,
        model: &str,
        api_key: &str,
        context_config: Option<ContextManagerConfig>,
        runtime: Arc<dyn QbitRuntime>,
    ) -> Result<Self> {
        let shared_config = SharedComponentsConfig {
            context_config,
            settings: qbit_settings::QbitSettings::default(),
        };
        Self::new_xai_with_shared_config(workspace, model, api_key, shared_config, runtime).await
    }

    /// Create a new AgentBridge for xAI with full shared config.
    pub async fn new_xai_with_shared_config(
        workspace: PathBuf,
        model: &str,
        api_key: &str,
        shared_config: SharedComponentsConfig,
        runtime: Arc<dyn QbitRuntime>,
    ) -> Result<Self> {
        let config = XaiClientConfig {
            workspace,
            model,
            api_key,
        };
        let components = create_xai_components(config, shared_config).await?;
        Ok(Self::from_components_with_runtime(components, runtime))
    }

    /// Create a new AgentBridge for Z.AI (GLM models).
    pub async fn new_zai_with_runtime(
        workspace: PathBuf,
        model: &str,
        api_key: &str,
        use_coding_endpoint: bool,
        runtime: Arc<dyn QbitRuntime>,
    ) -> Result<Self> {
        Self::new_zai_with_context(
            workspace,
            model,
            api_key,
            use_coding_endpoint,
            None,
            runtime,
        )
        .await
    }

    /// Create a new AgentBridge for Z.AI with optional context config.
    pub async fn new_zai_with_context(
        workspace: PathBuf,
        model: &str,
        api_key: &str,
        use_coding_endpoint: bool,
        context_config: Option<ContextManagerConfig>,
        runtime: Arc<dyn QbitRuntime>,
    ) -> Result<Self> {
        let shared_config = SharedComponentsConfig {
            context_config,
            settings: qbit_settings::QbitSettings::default(),
        };
        Self::new_zai_with_shared_config(
            workspace,
            model,
            api_key,
            use_coding_endpoint,
            shared_config,
            runtime,
        )
        .await
    }

    /// Create a new AgentBridge for Z.AI with full shared config.
    pub async fn new_zai_with_shared_config(
        workspace: PathBuf,
        model: &str,
        api_key: &str,
        use_coding_endpoint: bool,
        shared_config: SharedComponentsConfig,
        runtime: Arc<dyn QbitRuntime>,
    ) -> Result<Self> {
        let config = ZaiClientConfig {
            workspace,
            model,
            api_key,
            use_coding_endpoint,
        };
        let components = create_zai_components(config, shared_config).await?;
        Ok(Self::from_components_with_runtime(components, runtime))
    }

    /// Create a new AgentBridge for Z.AI via Anthropic-compatible API with runtime abstraction.
    pub async fn new_zai_anthropic_with_runtime(
        workspace: PathBuf,
        model: &str,
        api_key: &str,
        runtime: Arc<dyn QbitRuntime>,
    ) -> Result<Self> {
        Self::new_zai_anthropic_with_context(workspace, model, api_key, None, runtime).await
    }

    /// Create a new AgentBridge for Z.AI Anthropic with optional context config.
    pub async fn new_zai_anthropic_with_context(
        workspace: PathBuf,
        model: &str,
        api_key: &str,
        context_config: Option<ContextManagerConfig>,
        runtime: Arc<dyn QbitRuntime>,
    ) -> Result<Self> {
        let shared_config = SharedComponentsConfig {
            context_config,
            settings: qbit_settings::QbitSettings::default(),
        };
        Self::new_zai_anthropic_with_shared_config(
            workspace,
            model,
            api_key,
            shared_config,
            runtime,
        )
        .await
    }

    /// Create a new AgentBridge for Z.AI Anthropic with shared components config.
    pub async fn new_zai_anthropic_with_shared_config(
        workspace: PathBuf,
        model: &str,
        api_key: &str,
        shared_config: SharedComponentsConfig,
        runtime: Arc<dyn QbitRuntime>,
    ) -> Result<Self> {
        let config = ZaiAnthropicClientConfig {
            workspace,
            model,
            api_key,
        };
        let components = create_zai_anthropic_components(config, shared_config).await?;
        Ok(Self::from_components_with_runtime(components, runtime))
    }

    /// Create an AgentBridge from pre-built components with runtime abstraction.
    fn from_components_with_runtime(
        components: AgentBridgeComponents,
        runtime: Arc<dyn QbitRuntime>,
    ) -> Self {
        let AgentBridgeComponents {
            workspace,
            provider_name,
            model_name,
            tool_registry,
            client,
            sub_agent_registry,
            approval_recorder,
            tool_policy_manager,
            context_manager,
            loop_detector,
            openai_web_search_config,
            model_factory,
        } = components;

        Self {
            workspace,
            provider_name,
            model_name,
            tool_registry,
            client,
            event_tx: None,
            runtime: Some(runtime),
            event_session_id: None,
            sub_agent_registry,
            #[cfg(any(feature = "tauri", feature = "cli"))]
            pty_manager: None,
            current_session_id: Default::default(),
            conversation_history: Default::default(),
            indexer_state: None,
            transcript_writer: None,
            transcript_base_dir: None,
            session_manager: Default::default(),
            session_persistence_enabled: Arc::new(RwLock::new(true)),
            approval_recorder,
            pending_approvals: Default::default(),
            tool_policy_manager,
            context_manager,
            compaction_state: Arc::new(RwLock::new(CompactionState::new())),
            loop_detector,
            tool_config: ToolConfig::main_agent(),
            agent_mode: Arc::new(RwLock::new(AgentMode::default())),
            plan_manager: Arc::new(PlanManager::new()),
            sidecar_state: None,
            memory_file_path: Arc::new(RwLock::new(None)),
            settings_manager: None,
            openai_web_search_config,
            model_factory,
        }
    }

    // ========================================================================
    // Event Emission Helpers
    // ========================================================================

    /// Helper to emit events through available channels.
    ///
    /// During the transition period, this emits through BOTH `event_tx` and `runtime`
    /// if both are available. This ensures no events are lost during migration.
    ///
    /// After migration is complete, only `runtime` will be used.
    ///
    /// Uses `event_session_id` for routing events to the correct frontend tab.
    pub fn emit_event(&self, event: AiEvent) {
        // Write to transcript if configured
        // Skip: streaming events (TextDelta/Reasoning), sub-agent internal events (go to separate file)
        if let Some(ref writer) = self.transcript_writer {
            if !matches!(
                event,
                AiEvent::TextDelta { .. }
                    | AiEvent::Reasoning { .. }
                    | AiEvent::SubAgentToolRequest { .. }
                    | AiEvent::SubAgentToolResult { .. }
            ) {
                let writer = Arc::clone(writer);
                let event_clone = event.clone();
                tokio::spawn(async move {
                    if let Err(e) = writer.append(&event_clone).await {
                        tracing::warn!("Failed to write to transcript: {}", e);
                    }
                });
            }
        }

        // Emit through legacy event_tx channel if available
        if let Some(ref tx) = self.event_tx {
            let _ = tx.send(event.clone());
        }

        // Emit through runtime abstraction if available
        if let Some(ref rt) = self.runtime {
            // Use stored session_id for routing, fall back to "unknown" if not set
            let session_id = self
                .event_session_id
                .clone()
                .unwrap_or_else(|| "unknown".to_string());
            if let Err(e) = rt.emit(RuntimeEvent::Ai {
                session_id,
                event: Box::new(event),
            }) {
                tracing::warn!("Failed to emit event through runtime: {}", e);
            }
        }
    }

    /// Get or create an event channel for the agentic loop.
    ///
    /// If `event_tx` is available, returns a clone of that sender.
    /// If only `runtime` is available, creates a forwarding channel that sends to runtime.
    ///
    /// This is a transition helper - once we update AgenticLoopContext to use runtime
    /// directly, this method will be removed.
    ///
    /// Uses `event_session_id` for routing events to the correct frontend tab.
    pub fn get_or_create_event_tx(&self) -> mpsc::UnboundedSender<AiEvent> {
        // If we have an event_tx, use it
        if let Some(ref tx) = self.event_tx {
            return tx.clone();
        }

        // Otherwise, create a forwarding channel to runtime
        let runtime = self.runtime.clone().expect(
            "AgentBridge must have either event_tx or runtime - this is a bug in construction",
        );

        // Use stored session_id for routing, fall back to "unknown" if not set
        let session_id = self
            .event_session_id
            .clone()
            .unwrap_or_else(|| "unknown".to_string());

        let (tx, mut rx) = mpsc::unbounded_channel::<AiEvent>();

        tokio::spawn(async move {
            while let Some(event) = rx.recv().await {
                if let Err(e) = runtime.emit(RuntimeEvent::Ai {
                    session_id: session_id.clone(),
                    event: Box::new(event),
                }) {
                    tracing::warn!("Failed to forward event to runtime: {}", e);
                }
            }
            tracing::debug!("Agentic loop event forwarder shut down");
        });

        tx
    }

    // ========================================================================
    // Execution Helper Methods (DRY extraction for execute_with_*_model)
    // ========================================================================

    /// Prepare the execution context for an agent turn.
    ///
    /// This extracts common setup code that was duplicated across all execute_with_*_model methods:
    /// 1. Build system prompt with agent mode and memory file
    /// 2. Inject session context
    /// 3. Start session for persistence
    /// 4. Record user message
    /// 5. Handle sidecar capture (start session, capture prompt)
    /// 6. Prepare initial history with user message
    /// 7. Get or create event channel
    ///
    /// Returns the system prompt, initial history, and event channel sender.
    async fn prepare_execution_context(
        &self,
        initial_prompt: &str,
    ) -> (String, Vec<Message>, mpsc::UnboundedSender<AiEvent>) {
        // Build system prompt with current agent mode and memory file
        let workspace_path = self.workspace.read().await;
        let agent_mode = *self.agent_mode.read().await;
        let memory_file_path = self.get_memory_file_path_dynamic().await;

        // Create prompt contributor registry with default contributors
        let contributors = create_default_contributors(self.sub_agent_registry.clone());
        let mut registry = PromptContributorRegistry::new();
        for contributor in contributors {
            registry.register(contributor);
        }

        // Create prompt context with provider, model, and feature flags
        let has_web_search = self
            .tool_registry
            .read()
            .await
            .available_tools()
            .iter()
            .any(|t| t.starts_with("web_"));
        let has_sub_agents = true; // Main agent always has sub-agents available
        let prompt_context = PromptContext::new(&self.provider_name, &self.model_name)
            .with_web_search(has_web_search)
            .with_sub_agents(has_sub_agents)
            .with_workspace(workspace_path.display().to_string());

        let mut system_prompt = build_system_prompt_with_contributions(
            &workspace_path,
            agent_mode,
            memory_file_path.as_deref(),
            Some(&registry),
            Some(&prompt_context),
        );
        drop(workspace_path);

        // Inject Layer 1 session context if available
        if let Some(session_context) = self.get_session_context().await {
            if !session_context.is_empty() {
                tracing::debug!(
                    "[agent] Injecting Layer 1 session context ({} chars)",
                    session_context.len()
                );
                system_prompt.push_str("\n\n");
                system_prompt.push_str(&session_context);
            }
        }

        // Start session for persistence
        self.start_session().await;
        self.record_user_message(initial_prompt).await;

        // Capture user prompt in sidecar session
        // Only start a new session if one doesn't already exist (sessions span conversations)
        if let Some(ref sidecar) = self.sidecar_state {
            use qbit_sidecar::events::SessionEvent;

            let session_id = if let Some(existing_id) = sidecar.current_session_id() {
                // Reuse existing session
                tracing::debug!("Reusing existing sidecar session: {}", existing_id);
                Some(existing_id)
            } else {
                // Start a new session
                match sidecar.start_session(initial_prompt) {
                    Ok(new_id) => {
                        tracing::info!("Started new sidecar session: {}", new_id);
                        Some(new_id)
                    }
                    Err(e) => {
                        tracing::warn!("Failed to start sidecar session: {}", e);
                        None
                    }
                }
            };

            // Capture the user prompt as an event (if we have a session)
            if let Some(ref sid) = session_id {
                let prompt_event = SessionEvent::user_prompt(sid.clone(), initial_prompt);
                sidecar.capture(prompt_event);

                // Store sidecar session ID in AI session manager for later restoration
                self.with_session_manager(|m| {
                    m.set_sidecar_session_id(sid.clone());
                })
                .await;
            }
        }

        // Prepare initial history with user message
        let mut history_guard = self.conversation_history.write().await;
        history_guard.push(Message::User {
            content: OneOrMany::one(UserContent::Text(Text {
                text: initial_prompt.to_string(),
            })),
        });
        let initial_history = history_guard.clone();
        drop(history_guard);

        // Get or create event channel for the agentic loop
        // This handles both legacy (event_tx) and new (runtime) paths
        let loop_event_tx = self.get_or_create_event_tx();

        (system_prompt, initial_history, loop_event_tx)
    }

    /// Prepare execution context with rich content (text + images).
    ///
    /// Similar to `prepare_execution_context` but accepts `Vec<UserContent>`
    /// instead of a plain string, enabling multi-modal prompts.
    async fn prepare_execution_context_with_content(
        &self,
        content: Vec<UserContent>,
        text_for_logging: &str,
    ) -> (String, Vec<Message>, mpsc::UnboundedSender<AiEvent>) {
        // Build system prompt with current agent mode and memory file
        let workspace_path = self.workspace.read().await;
        let agent_mode = *self.agent_mode.read().await;
        let memory_file_path = self.get_memory_file_path_dynamic().await;

        // Create prompt contributor registry with default contributors
        let contributors = create_default_contributors(self.sub_agent_registry.clone());
        let mut registry = PromptContributorRegistry::new();
        for contributor in contributors {
            registry.register(contributor);
        }

        // Create prompt context with provider, model, and feature flags
        let has_web_search = self
            .tool_registry
            .read()
            .await
            .available_tools()
            .iter()
            .any(|t| t.starts_with("web_"));
        let has_sub_agents = true;
        let prompt_context = PromptContext::new(&self.provider_name, &self.model_name)
            .with_web_search(has_web_search)
            .with_sub_agents(has_sub_agents)
            .with_workspace(workspace_path.display().to_string());

        let mut system_prompt = build_system_prompt_with_contributions(
            &workspace_path,
            agent_mode,
            memory_file_path.as_deref(),
            Some(&registry),
            Some(&prompt_context),
        );
        drop(workspace_path);

        // Inject Layer 1 session context if available
        if let Some(session_context) = self.get_session_context().await {
            if !session_context.is_empty() {
                tracing::debug!(
                    "[agent] Injecting Layer 1 session context ({} chars)",
                    session_context.len()
                );
                system_prompt.push_str("\n\n");
                system_prompt.push_str(&session_context);
            }
        }

        // Start session for persistence
        self.start_session().await;
        self.record_user_message(text_for_logging).await;

        // Capture user prompt in sidecar session
        if let Some(ref sidecar) = self.sidecar_state {
            use qbit_sidecar::events::SessionEvent;

            let session_id = if let Some(existing_id) = sidecar.current_session_id() {
                tracing::debug!("Reusing existing sidecar session: {}", existing_id);
                Some(existing_id)
            } else {
                match sidecar.start_session(text_for_logging) {
                    Ok(new_id) => {
                        tracing::info!("Started new sidecar session: {}", new_id);
                        Some(new_id)
                    }
                    Err(e) => {
                        tracing::warn!("Failed to start sidecar session: {}", e);
                        None
                    }
                }
            };

            if let Some(ref sid) = session_id {
                let prompt_event = SessionEvent::user_prompt(sid.clone(), text_for_logging);
                sidecar.capture(prompt_event);

                self.with_session_manager(|m| {
                    m.set_sidecar_session_id(sid.clone());
                })
                .await;
            }
        }

        // Prepare initial history with user message (rich content)
        let mut history_guard = self.conversation_history.write().await;

        // Log content parts before creating message
        let incoming_text_count = content
            .iter()
            .filter(|c| matches!(c, UserContent::Text(_)))
            .count();
        let incoming_image_count = content
            .iter()
            .filter(|c| matches!(c, UserContent::Image(_)))
            .count();
        tracing::debug!(
            "prepare_context: {} text part(s), {} image(s)",
            incoming_text_count,
            incoming_image_count
        );

        // Build the user message from content parts
        let user_content = match OneOrMany::many(content) {
            Ok(many) => {
                tracing::debug!(
                    "prepare_execution_context_with_content: Created OneOrMany with {} items",
                    many.len()
                );
                many
            }
            Err(_) => {
                // Empty content - use a placeholder text
                tracing::warn!(
                    "prepare_execution_context_with_content: Empty content, using placeholder"
                );
                OneOrMany::one(UserContent::Text(Text {
                    text: "".to_string(),
                }))
            }
        };
        let user_message = Message::User {
            content: user_content,
        };

        history_guard.push(user_message);
        let initial_history = history_guard.clone();
        drop(history_guard);

        // Get or create event channel for the agentic loop
        let loop_event_tx = self.get_or_create_event_tx();

        (system_prompt, initial_history, loop_event_tx)
    }

    /// Build the AgenticLoopContext with references to all required components.
    ///
    /// This is a helper to construct the context struct without duplication.
    fn build_loop_context<'a>(
        &'a self,
        loop_event_tx: &'a mpsc::UnboundedSender<AiEvent>,
    ) -> AgenticLoopContext<'a> {
        AgenticLoopContext {
            event_tx: loop_event_tx,
            tool_registry: &self.tool_registry,
            sub_agent_registry: &self.sub_agent_registry,
            indexer_state: self.indexer_state.as_ref(),
            workspace: &self.workspace,
            client: &self.client,
            approval_recorder: &self.approval_recorder,
            pending_approvals: &self.pending_approvals,
            tool_policy_manager: &self.tool_policy_manager,
            context_manager: &self.context_manager,
            compaction_state: &self.compaction_state,
            loop_detector: &self.loop_detector,
            tool_config: &self.tool_config,
            sidecar_state: self.sidecar_state.as_ref(),
            runtime: self.runtime.as_ref(),
            agent_mode: &self.agent_mode,
            plan_manager: &self.plan_manager,
            provider_name: &self.provider_name,
            model_name: &self.model_name,
            openai_web_search_config: self.openai_web_search_config.as_ref(),
            model_factory: self.model_factory.as_ref(),
            session_id: self.event_session_id.as_deref(),
            transcript_writer: self.transcript_writer.as_ref(),
            transcript_base_dir: self.transcript_base_dir.as_deref(),
        }
    }

    /// Finalize execution after the agentic loop completes.
    ///
    /// This extracts common post-execution code that was duplicated:
    /// 1. Persist assistant response to conversation history
    /// 2. Record and save session
    /// 3. Capture AI response in sidecar session
    /// 4. Emit completion event
    ///
    /// Returns the accumulated response (passed through for convenience).
    async fn finalize_execution(
        &self,
        accumulated_response: String,
        final_history: Vec<Message>,
        token_usage: Option<TokenUsage>,
        start_time: std::time::Instant,
    ) -> String {
        let duration_ms = start_time.elapsed().as_millis() as u64;

        // Replace conversation history with the full history from the agentic loop.
        // This is critical for OpenAI Responses API where reasoning IDs must be preserved
        // in the history for function calls to work correctly across turns.
        {
            let mut history_guard = self.conversation_history.write().await;
            *history_guard = final_history;
        }

        // Record and save session
        if !accumulated_response.is_empty() {
            self.record_assistant_message(&accumulated_response).await;
            self.save_session().await;
        }

        // Capture AI response in sidecar session
        if let Some(ref sidecar) = self.sidecar_state {
            use qbit_sidecar::events::SessionEvent;

            if let Some(session_id) = sidecar.current_session_id() {
                if !accumulated_response.is_empty() {
                    let response_event =
                        SessionEvent::ai_response(session_id, &accumulated_response);
                    sidecar.capture(response_event);
                    tracing::debug!(
                        "[agent] Captured AI response in sidecar ({} chars)",
                        accumulated_response.len()
                    );
                }
            }
        }

        // Emit completion event
        self.emit_event(AiEvent::Completed {
            response: accumulated_response.clone(),
            input_tokens: token_usage.as_ref().map(|u| u.input_tokens as u32),
            output_tokens: token_usage.as_ref().map(|u| u.output_tokens as u32),
            duration_ms: Some(duration_ms),
        });

        accumulated_response
    }

    // ========================================================================
    // Configuration Methods
    // ========================================================================

    /// Set the PtyManager for executing commands in user's terminal
    #[cfg(any(feature = "tauri", feature = "cli"))]
    pub fn set_pty_manager(&mut self, pty_manager: Arc<PtyManager>) {
        self.pty_manager = Some(pty_manager);
    }

    /// Set the IndexerState for code analysis tools
    pub fn set_indexer_state(&mut self, indexer_state: Arc<IndexerState>) {
        self.indexer_state = Some(indexer_state);
    }

    /// Set the SidecarState for context capture
    pub fn set_sidecar_state(&mut self, sidecar_state: Arc<SidecarState>) {
        self.sidecar_state = Some(sidecar_state);
    }

    /// Set the TranscriptWriter for persisting AI events to JSONL.
    pub fn set_transcript_writer(&mut self, writer: TranscriptWriter, base_dir: PathBuf) {
        self.transcript_writer = Some(Arc::new(writer));
        self.transcript_base_dir = Some(base_dir);
    }

    /// Set the memory file path for project instructions.
    /// This overrides the default CLAUDE.md lookup.
    pub async fn set_memory_file_path(&self, path: Option<PathBuf>) {
        *self.memory_file_path.write().await = path;
    }

    /// Set the SettingsManager for dynamic memory file lookup.
    pub fn set_settings_manager(&mut self, settings_manager: Arc<qbit_settings::SettingsManager>) {
        self.settings_manager = Some(settings_manager);
    }

    /// Get the memory file path dynamically from current settings.
    /// This ensures we always use the latest settings, even if they changed
    /// after the AI session was initialized.
    /// Falls back to cached value if settings_manager is not available.
    async fn get_memory_file_path_dynamic(&self) -> Option<PathBuf> {
        // Try dynamic lookup if settings_manager is available (tauri only)
        #[cfg(feature = "tauri")]
        if let Some(ref settings_manager) = self.settings_manager {
            let workspace_path = self.workspace.read().await;
            let settings = settings_manager.get().await;
            if let Some(path) = crate::memory_file::find_memory_file_for_workspace(
                &workspace_path,
                &settings.codebases,
            ) {
                return Some(path);
            }
        }

        // Fall back to cached value
        self.memory_file_path.read().await.clone()
    }

    /// Set the session ID for event routing.
    /// This is used to route AI events to the correct frontend tab.
    pub fn set_event_session_id(&mut self, session_id: String) {
        self.event_session_id = Some(session_id);
    }

    /// Set the current session ID for terminal execution
    pub async fn set_session_id(&self, session_id: Option<String>) {
        *self.current_session_id.write().await = session_id;
    }

    /// Update the workspace/working directory.
    /// This also updates the tool registry's workspace so file operations
    /// use the new directory as the base for relative paths.
    pub async fn set_workspace(&self, new_workspace: PathBuf) {
        // Check if workspace actually changed
        {
            let current = self.workspace.read().await;
            if *current == new_workspace {
                tracing::trace!(
                    "[cwd-sync] Workspace unchanged, skipping update: {}",
                    new_workspace.display()
                );
                return;
            }
        }

        // Update bridge workspace
        let mut workspace = self.workspace.write().await;
        *workspace = new_workspace.clone();

        // Also update the tool registry's workspace so file operations
        // resolve relative paths against the new directory
        {
            let mut registry = self.tool_registry.write().await;
            registry.set_workspace(new_workspace.clone());
        }

        // Also update the session manager's workspace so sessions capture the correct path
        self.update_session_workspace(new_workspace.clone()).await;

        tracing::debug!(
            "[cwd-sync] Updated workspace to: {}",
            new_workspace.display()
        );
    }

    /// Set the agent mode.
    /// This controls how tool approvals are handled.
    pub async fn set_agent_mode(&self, mode: AgentMode) {
        let mut current = self.agent_mode.write().await;
        tracing::debug!("Agent mode changed: {} -> {}", *current, mode);
        *current = mode;
    }

    /// Get the current agent mode.
    pub async fn get_agent_mode(&self) -> AgentMode {
        *self.agent_mode.read().await
    }

    // ========================================================================
    // System Prompt Methods
    // ========================================================================

    /// Build the system prompt for the agent.
    ///
    /// This is a simplified version of the prompt building logic from
    /// `prepare_execution_context`.
    pub async fn build_system_prompt(&self) -> String {
        use super::system_prompt::build_system_prompt_with_contributions;

        let workspace_path = self.workspace.read().await;
        let agent_mode = *self.agent_mode.read().await;
        let memory_file_path = self.get_memory_file_path_dynamic().await;

        build_system_prompt_with_contributions(
            &workspace_path,
            agent_mode,
            memory_file_path.as_deref(),
            None, // No prompt contributors for base prompt
            None, // No prompt context for base prompt
        )
    }

    // ========================================================================
    // Public Accessors (for qbit crate)
    // ========================================================================

    /// Get the sub-agent registry.
    pub fn sub_agent_registry(&self) -> &Arc<RwLock<SubAgentRegistry>> {
        &self.sub_agent_registry
    }

    /// Get the provider name.
    pub fn provider_name(&self) -> &str {
        &self.provider_name
    }

    /// Get the model name.
    pub fn model_name(&self) -> &str {
        &self.model_name
    }

    /// Get the plan manager.
    pub fn plan_manager(&self) -> &Arc<PlanManager> {
        &self.plan_manager
    }

    /// Get the LLM client.
    pub fn client(&self) -> &Arc<RwLock<LlmClient>> {
        &self.client
    }

    /// Get the tool registry.
    pub fn tool_registry(&self) -> &Arc<RwLock<ToolRegistry>> {
        &self.tool_registry
    }

    /// Get the workspace path.
    pub fn workspace(&self) -> &Arc<RwLock<PathBuf>> {
        &self.workspace
    }

    /// Get the indexer state.
    pub fn indexer_state(&self) -> Option<&Arc<IndexerState>> {
        self.indexer_state.as_ref()
    }

    /// Get the model factory (for sub-agent model overrides).
    pub fn model_factory(&self) -> Option<&Arc<super::llm_client::LlmClientFactory>> {
        self.model_factory.as_ref()
    }

    /// Set the model factory for sub-agent model overrides.
    pub fn set_model_factory(&mut self, factory: Arc<super::llm_client::LlmClientFactory>) {
        self.model_factory = Some(factory);
    }

    // ========================================================================
    // Main Execution Methods
    // ========================================================================

    /// Execute a prompt with agentic tool loop.
    pub async fn execute(&self, prompt: &str) -> Result<String> {
        self.execute_with_context(prompt, SubAgentContext::default())
            .await
    }

    /// Execute with rich content (text + images).
    ///
    /// This method accepts multiple content parts, enabling multi-modal prompts
    /// with images for vision-capable models.
    ///
    /// # Arguments
    ///
    /// * `content` - Vector of UserContent (text, images, etc.)
    ///
    /// # Example
    ///
    /// ```ignore
    /// use rig::message::UserContent;
    ///
    /// let content = vec![
    ///     UserContent::text("What's in this image?"),
    ///     UserContent::image_base64("...", Some(ImageMediaType::PNG), None),
    /// ];
    /// let response = bridge.execute_with_content(content).await?;
    /// ```
    pub async fn execute_with_content(&self, content: Vec<UserContent>) -> Result<String> {
        // Log content types for debugging
        let image_count = content
            .iter()
            .filter(|c| matches!(c, UserContent::Image(_)))
            .count();
        let text_count = content
            .iter()
            .filter(|c| matches!(c, UserContent::Text(_)))
            .count();
        tracing::debug!(
            "execute_with_content: {} text part(s), {} image(s)",
            text_count,
            image_count
        );

        self.execute_with_content_and_context(content, SubAgentContext::default())
            .await
    }

    /// Execute with rich content and sub-agent context.
    pub async fn execute_with_content_and_context(
        &self,
        content: Vec<UserContent>,
        context: SubAgentContext,
    ) -> Result<String> {
        // Check recursion depth
        if context.depth >= MAX_AGENT_DEPTH {
            return Err(anyhow::anyhow!(
                "Maximum agent recursion depth ({}) exceeded",
                MAX_AGENT_DEPTH
            ));
        }

        // Generate a unique turn ID
        let turn_id = uuid::Uuid::new_v4().to_string();

        // Emit turn started event
        self.emit_event(AiEvent::Started {
            turn_id: turn_id.clone(),
        });

        let start_time = std::time::Instant::now();

        // Extract text for logging/session recording
        let text_for_logging = content
            .iter()
            .filter_map(|c| match c {
                UserContent::Text(t) => Some(t.text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("\n");

        // Emit user message event for transcript
        self.emit_event(AiEvent::UserMessage {
            content: text_for_logging.clone(),
        });

        // Prepare execution context with rich content
        let (system_prompt, initial_history, loop_event_tx) = self
            .prepare_execution_context_with_content(content, &text_for_logging)
            .await;

        let client = self.client.read().await;

        // Currently only Vertex Anthropic supports images properly
        // Other providers would need their own image handling
        match &*client {
            LlmClient::VertexAnthropic(vertex_model) => {
                let vertex_model = vertex_model.clone();
                drop(client);

                let loop_ctx = self.build_loop_context(&loop_event_tx);
                let (accumulated_response, final_history, token_usage) = run_agentic_loop(
                    &vertex_model,
                    &system_prompt,
                    initial_history,
                    context,
                    &loop_ctx,
                )
                .await?;

                Ok(self
                    .finalize_execution(
                        accumulated_response,
                        final_history,
                        token_usage,
                        start_time,
                    )
                    .await)
            }
            // For other providers, fall back to text-only execution
            _ => {
                drop(client);
                tracing::warn!(
                    "execute_with_content called on non-Vertex provider, images may not work correctly"
                );

                let loop_ctx = self.build_loop_context(&loop_event_tx);
                let client = self.client.read().await;

                // Use generic execution for other providers
                match &*client {
                    LlmClient::RigAnthropic(model) => {
                        let model = model.clone();
                        drop(client);
                        let (accumulated_response, final_history, token_usage) =
                            run_agentic_loop_generic(
                                &model,
                                &system_prompt,
                                initial_history,
                                context,
                                &loop_ctx,
                            )
                            .await?;
                        Ok(self
                            .finalize_execution(
                                accumulated_response,
                                final_history,
                                token_usage,
                                start_time,
                            )
                            .await)
                    }
                    LlmClient::RigGemini(model) => {
                        let model = model.clone();
                        drop(client);
                        let (accumulated_response, final_history, token_usage) =
                            run_agentic_loop_generic(
                                &model,
                                &system_prompt,
                                initial_history,
                                context,
                                &loop_ctx,
                            )
                            .await?;
                        Ok(self
                            .finalize_execution(
                                accumulated_response,
                                final_history,
                                token_usage,
                                start_time,
                            )
                            .await)
                    }
                    LlmClient::RigOpenAi(model) => {
                        let model = model.clone();
                        drop(client);
                        let (accumulated_response, final_history, token_usage) =
                            run_agentic_loop_generic(
                                &model,
                                &system_prompt,
                                initial_history,
                                context,
                                &loop_ctx,
                            )
                            .await?;
                        Ok(self
                            .finalize_execution(
                                accumulated_response,
                                final_history,
                                token_usage,
                                start_time,
                            )
                            .await)
                    }
                    LlmClient::RigOpenAiResponses(model) => {
                        let model = model.clone();
                        drop(client);
                        let (accumulated_response, final_history, token_usage) =
                            run_agentic_loop_generic(
                                &model,
                                &system_prompt,
                                initial_history,
                                context,
                                &loop_ctx,
                            )
                            .await?;
                        Ok(self
                            .finalize_execution(
                                accumulated_response,
                                final_history,
                                token_usage,
                                start_time,
                            )
                            .await)
                    }
                    _ => {
                        drop(client);
                        Err(anyhow::anyhow!(
                            "execute_with_content not fully supported for this provider"
                        ))
                    }
                }
            }
        }
    }

    // ========================================================================
    // Cancellation-Enabled Execution Methods (server feature only)
    // ========================================================================

    /// Execute a prompt with cancellation support.
    ///
    /// The cancellation token allows external cancellation of the execution,
    /// which is essential for HTTP server timeouts and client disconnections.
    ///
    /// # Arguments
    ///
    /// * `prompt` - The user prompt to execute
    /// * `cancel_token` - Token that can be used to cancel the execution
    ///
    /// # Returns
    ///
    /// * `Ok(String)` - The accumulated response from the agent
    /// * `Err` - If execution was cancelled or failed
    ///
    /// Execute a prompt with context (for sub-agent calls).
    pub async fn execute_with_context(
        &self,
        prompt: &str,
        context: SubAgentContext,
    ) -> Result<String> {
        // Check recursion depth
        if context.depth >= MAX_AGENT_DEPTH {
            return Err(anyhow::anyhow!(
                "Maximum agent recursion depth ({}) exceeded",
                MAX_AGENT_DEPTH
            ));
        }

        // Generate a unique turn ID
        let turn_id = uuid::Uuid::new_v4().to_string();

        // Emit turn started event
        self.emit_event(AiEvent::Started {
            turn_id: turn_id.clone(),
        });

        // Emit user message event for transcript
        self.emit_event(AiEvent::UserMessage {
            content: prompt.to_string(),
        });

        let start_time = std::time::Instant::now();
        let client = self.client.read().await;

        match &*client {
            LlmClient::VertexAnthropic(vertex_model) => {
                let vertex_model = vertex_model.clone();
                drop(client);

                self.execute_with_vertex_model(&vertex_model, prompt, start_time, context)
                    .await
            }
            LlmClient::RigOpenRouter(openrouter_model) => {
                let openrouter_model = openrouter_model.clone();
                drop(client);

                self.execute_with_openrouter_model(&openrouter_model, prompt, start_time, context)
                    .await
            }
            LlmClient::RigOpenAi(openai_model) => {
                let openai_model = openai_model.clone();
                drop(client);

                self.execute_with_openai_model(&openai_model, prompt, start_time, context)
                    .await
            }
            LlmClient::RigOpenAiResponses(openai_model) => {
                let openai_model = openai_model.clone();
                drop(client);

                self.execute_with_openai_responses_model(&openai_model, prompt, start_time, context)
                    .await
            }
            LlmClient::RigAnthropic(anthropic_model) => {
                let anthropic_model = anthropic_model.clone();
                drop(client);

                // Use the generic execution path (same as OpenRouter/OpenAI)
                self.execute_with_anthropic_model(&anthropic_model, prompt, start_time, context)
                    .await
            }
            LlmClient::RigOllama(ollama_model) => {
                let ollama_model = ollama_model.clone();
                drop(client);

                // Use the generic execution path (same as OpenRouter/OpenAI)
                self.execute_with_ollama_model(&ollama_model, prompt, start_time, context)
                    .await
            }
            LlmClient::RigGemini(gemini_model) => {
                let gemini_model = gemini_model.clone();
                drop(client);

                self.execute_with_gemini_model(&gemini_model, prompt, start_time, context)
                    .await
            }
            LlmClient::RigGroq(groq_model) => {
                let groq_model = groq_model.clone();
                drop(client);

                self.execute_with_groq_model(&groq_model, prompt, start_time, context)
                    .await
            }
            LlmClient::RigXai(xai_model) => {
                let xai_model = xai_model.clone();
                drop(client);

                self.execute_with_xai_model(&xai_model, prompt, start_time, context)
                    .await
            }
            LlmClient::RigZai(zai_model) => {
                let zai_model = zai_model.clone();
                drop(client);

                self.execute_with_zai_model(&zai_model, prompt, start_time, context)
                    .await
            }
            LlmClient::RigZaiAnthropic(zai_anthropic_model) => {
                let zai_anthropic_model = zai_anthropic_model.clone();
                drop(client);

                // Z.AI Anthropic uses the same Anthropic API format
                self.execute_with_anthropic_model(&zai_anthropic_model, prompt, start_time, context)
                    .await
            }
            LlmClient::RigZaiAnthropicLogging(zai_anthropic_model) => {
                let zai_anthropic_model = zai_anthropic_model.clone();
                drop(client);

                // Z.AI Anthropic with logging uses the same Anthropic API format
                self.execute_with_anthropic_model(&zai_anthropic_model, prompt, start_time, context)
                    .await
            }
            LlmClient::Mock => {
                drop(client);
                Err(anyhow::anyhow!(
                    "Mock client cannot execute - use for testing infrastructure only"
                ))
            }
        }
    }

    /// Execute with Vertex AI model using the agentic loop.
    ///
    /// Uses `run_agentic_loop` which is Anthropic-specific (supports extended thinking).
    async fn execute_with_vertex_model(
        &self,
        model: &rig_anthropic_vertex::CompletionModel,
        initial_prompt: &str,
        start_time: std::time::Instant,
        context: SubAgentContext,
    ) -> Result<String> {
        // Prepare common execution context (system prompt, history, event channel)
        let (system_prompt, initial_history, loop_event_tx) =
            self.prepare_execution_context(initial_prompt).await;

        // Build agentic loop context
        let loop_ctx = self.build_loop_context(&loop_event_tx);

        // Run the Anthropic-specific agentic loop (supports extended thinking)
        let (accumulated_response, final_history, token_usage) =
            run_agentic_loop(model, &system_prompt, initial_history, context, &loop_ctx).await?;

        // Finalize execution (persist response and full history, emit events)
        // Note: Sidecar session is NOT ended here - it persists across prompts.
        // See finalize_execution and Drop impl for session lifecycle.
        Ok(self
            .finalize_execution(accumulated_response, final_history, token_usage, start_time)
            .await)
    }

    /// Execute with OpenRouter model using the generic agentic loop.
    async fn execute_with_openrouter_model(
        &self,
        model: &rig_openrouter::CompletionModel,
        initial_prompt: &str,
        start_time: std::time::Instant,
        context: SubAgentContext,
    ) -> Result<String> {
        // Prepare common execution context (system prompt, history, event channel)
        let (system_prompt, initial_history, loop_event_tx) =
            self.prepare_execution_context(initial_prompt).await;

        // Build agentic loop context
        let loop_ctx = self.build_loop_context(&loop_event_tx);

        // Run the generic agentic loop (works with any rig CompletionModel)
        let (accumulated_response, final_history, token_usage) =
            run_agentic_loop_generic(model, &system_prompt, initial_history, context, &loop_ctx)
                .await?;

        // Finalize execution (persist response and full history, emit events)
        Ok(self
            .finalize_execution(accumulated_response, final_history, token_usage, start_time)
            .await)
    }

    /// Execute with OpenAI model using the generic agentic loop.
    async fn execute_with_openai_model(
        &self,
        model: &rig_openai::completion::CompletionModel,
        initial_prompt: &str,
        start_time: std::time::Instant,
        context: SubAgentContext,
    ) -> Result<String> {
        let (system_prompt, initial_history, loop_event_tx) =
            self.prepare_execution_context(initial_prompt).await;
        let loop_ctx = self.build_loop_context(&loop_event_tx);

        let (accumulated_response, final_history, token_usage) =
            run_agentic_loop_generic(model, &system_prompt, initial_history, context, &loop_ctx)
                .await?;

        Ok(self
            .finalize_execution(accumulated_response, final_history, token_usage, start_time)
            .await)
    }

    /// Execute with OpenAI Responses API model using the generic agentic loop.
    /// This uses the Responses API which has better tool support than the Chat Completions API.
    async fn execute_with_openai_responses_model(
        &self,
        model: &rig_openai::responses_api::ResponsesCompletionModel,
        initial_prompt: &str,
        start_time: std::time::Instant,
        context: SubAgentContext,
    ) -> Result<String> {
        let (system_prompt, initial_history, loop_event_tx) =
            self.prepare_execution_context(initial_prompt).await;
        let loop_ctx = self.build_loop_context(&loop_event_tx);

        let (accumulated_response, final_history, token_usage) =
            run_agentic_loop_generic(model, &system_prompt, initial_history, context, &loop_ctx)
                .await?;

        Ok(self
            .finalize_execution(accumulated_response, final_history, token_usage, start_time)
            .await)
    }

    /// Execute with Anthropic model using the generic agentic loop.
    ///
    /// This method is generic over the HTTP client type H, allowing it to work
    /// with both standard reqwest::Client and custom logging clients.
    async fn execute_with_anthropic_model<H>(
        &self,
        model: &rig_anthropic::completion::CompletionModel<H>,
        initial_prompt: &str,
        start_time: std::time::Instant,
        context: SubAgentContext,
    ) -> Result<String>
    where
        H: rig::http_client::HttpClientExt + Clone + Send + Sync + Default + 'static,
    {
        let (system_prompt, initial_history, loop_event_tx) =
            self.prepare_execution_context(initial_prompt).await;
        let loop_ctx = self.build_loop_context(&loop_event_tx);

        let (accumulated_response, final_history, token_usage) =
            run_agentic_loop_generic(model, &system_prompt, initial_history, context, &loop_ctx)
                .await?;

        Ok(self
            .finalize_execution(accumulated_response, final_history, token_usage, start_time)
            .await)
    }

    /// Execute with Ollama model using the generic agentic loop.
    async fn execute_with_ollama_model(
        &self,
        model: &rig_ollama::CompletionModel<reqwest::Client>,
        initial_prompt: &str,
        start_time: std::time::Instant,
        context: SubAgentContext,
    ) -> Result<String> {
        let (system_prompt, initial_history, loop_event_tx) =
            self.prepare_execution_context(initial_prompt).await;
        let loop_ctx = self.build_loop_context(&loop_event_tx);

        let (accumulated_response, final_history, token_usage) =
            run_agentic_loop_generic(model, &system_prompt, initial_history, context, &loop_ctx)
                .await?;

        Ok(self
            .finalize_execution(accumulated_response, final_history, token_usage, start_time)
            .await)
    }

    /// Execute with Gemini model using the generic agentic loop.
    async fn execute_with_gemini_model(
        &self,
        model: &rig_gemini::completion::CompletionModel,
        initial_prompt: &str,
        start_time: std::time::Instant,
        context: SubAgentContext,
    ) -> Result<String> {
        let (system_prompt, initial_history, loop_event_tx) =
            self.prepare_execution_context(initial_prompt).await;
        let loop_ctx = self.build_loop_context(&loop_event_tx);

        let (accumulated_response, final_history, token_usage) =
            run_agentic_loop_generic(model, &system_prompt, initial_history, context, &loop_ctx)
                .await?;

        Ok(self
            .finalize_execution(accumulated_response, final_history, token_usage, start_time)
            .await)
    }

    /// Execute with Groq model using the generic agentic loop.
    async fn execute_with_groq_model(
        &self,
        model: &rig_groq::CompletionModel<reqwest::Client>,
        initial_prompt: &str,
        start_time: std::time::Instant,
        context: SubAgentContext,
    ) -> Result<String> {
        let (system_prompt, initial_history, loop_event_tx) =
            self.prepare_execution_context(initial_prompt).await;
        let loop_ctx = self.build_loop_context(&loop_event_tx);

        let (accumulated_response, final_history, token_usage) =
            run_agentic_loop_generic(model, &system_prompt, initial_history, context, &loop_ctx)
                .await?;

        Ok(self
            .finalize_execution(accumulated_response, final_history, token_usage, start_time)
            .await)
    }

    /// Execute with xAI (Grok) model using the generic agentic loop.
    async fn execute_with_xai_model(
        &self,
        model: &rig_xai::completion::CompletionModel<reqwest::Client>,
        initial_prompt: &str,
        start_time: std::time::Instant,
        context: SubAgentContext,
    ) -> Result<String> {
        let (system_prompt, initial_history, loop_event_tx) =
            self.prepare_execution_context(initial_prompt).await;
        let loop_ctx = self.build_loop_context(&loop_event_tx);

        let (accumulated_response, final_history, token_usage) =
            run_agentic_loop_generic(model, &system_prompt, initial_history, context, &loop_ctx)
                .await?;

        Ok(self
            .finalize_execution(accumulated_response, final_history, token_usage, start_time)
            .await)
    }

    /// Execute with Z.AI (GLM) model using the generic agentic loop.
    async fn execute_with_zai_model(
        &self,
        model: &rig_zai::CompletionModel<reqwest::Client>,
        initial_prompt: &str,
        start_time: std::time::Instant,
        context: SubAgentContext,
    ) -> Result<String> {
        let (system_prompt, initial_history, loop_event_tx) =
            self.prepare_execution_context(initial_prompt).await;
        let loop_ctx = self.build_loop_context(&loop_event_tx);

        let (accumulated_response, final_history, token_usage) =
            run_agentic_loop_generic(model, &system_prompt, initial_history, context, &loop_ctx)
                .await?;

        Ok(self
            .finalize_execution(accumulated_response, final_history, token_usage, start_time)
            .await)
    }

    /// Execute a tool by name (public API).
    pub async fn execute_tool(
        &self,
        tool_name: &str,
        args: serde_json::Value,
    ) -> Result<serde_json::Value> {
        let mut registry = self.tool_registry.write().await;
        let result = registry.execute_tool(tool_name, args).await;

        result.map_err(|e| anyhow::anyhow!(e))
    }

    /// Get available tools for the LLM.
    pub async fn available_tools(&self) -> Vec<serde_json::Value> {
        let registry = self.tool_registry.read().await;
        let tool_names = registry.available_tools();

        tool_names
            .into_iter()
            .map(|name| serde_json::json!({ "name": name }))
            .collect()
    }

    /// Get session context for injection into agent prompt
    pub async fn get_session_context(&self) -> Option<String> {
        let sidecar = self.sidecar_state.as_ref()?;

        // Use the simplified sidecar API to get injectable context (state.md content)
        match sidecar.get_injectable_context().await {
            Ok(context) => context,
            Err(e) => {
                tracing::warn!("Failed to get session context: {}", e);
                None
            }
        }
    }
}

// ============================================================================
// Drop Implementation for Session Cleanup
// ============================================================================

impl Drop for AgentBridge {
    fn drop(&mut self) {
        // Best-effort session finalization on drop.
        // This ensures sessions are saved even if the bridge is replaced without
        // explicit finalization (e.g., during model switching).
        //
        // We use try_write() because:
        // 1. Drop cannot be async, so we can't use .await
        // 2. If the lock is held, another operation is in progress and will handle cleanup
        // 3. At drop time, we should typically be the only owner
        if let Ok(mut guard) = self.session_manager.try_write() {
            if let Some(ref mut manager) = guard.take() {
                match manager.finalize() {
                    Ok(path) => {
                        tracing::debug!(
                            "AgentBridge::drop - session finalized: {}",
                            path.display()
                        );
                    }
                    Err(e) => {
                        tracing::warn!("AgentBridge::drop - failed to finalize session: {}", e);
                    }
                }
            }
        } else {
            tracing::debug!(
                "AgentBridge::drop - could not acquire session_manager lock, skipping finalization"
            );
        }

        // End sidecar session on bridge drop.
        // This ensures the sidecar session is properly finalized when:
        // - The conversation is cleared
        // - The AgentBridge is replaced (e.g., model switching)
        // - The application shuts down
        if let Some(ref sidecar) = self.sidecar_state {
            match sidecar.end_session() {
                Ok(Some(session)) => {
                    tracing::debug!(
                        "AgentBridge::drop - sidecar session {} ended",
                        session.session_id
                    );
                }
                Ok(None) => {
                    tracing::debug!("AgentBridge::drop - no active sidecar session to end");
                }
                Err(e) => {
                    tracing::warn!("AgentBridge::drop - failed to end sidecar session: {}", e);
                }
            }
        }
    }
}
