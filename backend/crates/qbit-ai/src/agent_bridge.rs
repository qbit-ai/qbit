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
use rig::completion::{AssistantContent, Message};
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

use vtcode_core::tools::ToolRegistry;

use qbit_core::events::AiEvent;
use qbit_core::hitl::ApprovalDecision;
use qbit_hitl::ApprovalRecorder;

use super::agent_mode::AgentMode;
use super::agentic_loop::{run_agentic_loop, run_agentic_loop_generic, AgenticLoopContext};
use super::llm_client::{
    create_anthropic_components, create_gemini_components, create_groq_components,
    create_ollama_components, create_openai_components, create_openrouter_components,
    create_vertex_components, create_xai_components, create_zai_components,
    AgentBridgeComponents, AnthropicClientConfig, GeminiClientConfig, GroqClientConfig,
    LlmClient, OllamaClientConfig, OpenAiClientConfig, OpenRouterClientConfig,
    VertexAnthropicClientConfig, XaiClientConfig, ZaiClientConfig,
};
use super::system_prompt::build_system_prompt;
use super::tool_definitions::ToolConfig;
use qbit_context::ContextManager;
use qbit_core::runtime::{QbitRuntime, RuntimeEvent};
use qbit_loop_detection::LoopDetector;
use qbit_session::QbitSessionManager;
use qbit_sub_agents::{SubAgentContext, SubAgentRegistry, MAX_AGENT_DEPTH};
use qbit_tool_policy::ToolPolicyManager;

use qbit_indexer::IndexerState;
use qbit_planner::PlanManager;
#[cfg(any(feature = "tauri", feature = "cli"))]
use qbit_pty::PtyManager;
use qbit_sidecar::SidecarState;
use qbit_web::tavily::TavilyState;

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

    // External services
    pub(crate) indexer_state: Option<Arc<IndexerState>>,
    pub(crate) tavily_state: Option<Arc<TavilyState>>,

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
        let config = OpenRouterClientConfig {
            workspace,
            model,
            api_key,
        };

        let components = create_openrouter_components(config).await?;

        Ok(Self::from_components_with_runtime(components, runtime))
    }

    /// Create a new AgentBridge for Anthropic on Google Cloud Vertex AI.
    ///
    /// Uses the `QbitRuntime` trait for event emission and approval handling.
    pub async fn new_vertex_anthropic_with_runtime(
        workspace: PathBuf,
        credentials_path: &str,
        project_id: &str,
        location: &str,
        model: &str,
        runtime: Arc<dyn QbitRuntime>,
    ) -> Result<Self> {
        let config = VertexAnthropicClientConfig {
            workspace,
            credentials_path,
            project_id,
            location,
            model,
        };

        let components = create_vertex_components(config).await?;

        Ok(Self::from_components_with_runtime(components, runtime))
    }

    /// Create a new AgentBridge for OpenAI.
    ///
    /// Uses the `QbitRuntime` trait for event emission and approval handling.
    ///
    /// # Arguments
    /// * `workspace` - Path to the workspace directory
    /// * `model` - Model identifier (e.g., "gpt-5.2")
    /// * `api_key` - OpenAI API key
    /// * `base_url` - Optional custom base URL for OpenAI-compatible APIs
    /// * `reasoning_effort` - Optional reasoning effort level ("low", "medium", "high")
    /// * `runtime` - Runtime abstraction for events and approvals
    pub async fn new_openai_with_runtime(
        workspace: PathBuf,
        model: &str,
        api_key: &str,
        base_url: Option<&str>,
        reasoning_effort: Option<&str>,
        runtime: Arc<dyn QbitRuntime>,
    ) -> Result<Self> {
        let config = OpenAiClientConfig {
            workspace,
            model,
            api_key,
            base_url,
            reasoning_effort,
        };

        let components = create_openai_components(config).await?;

        Ok(Self::from_components_with_runtime(components, runtime))
    }

    /// Create a new AgentBridge for direct Anthropic API.
    ///
    /// Uses the `QbitRuntime` trait for event emission and approval handling.
    pub async fn new_anthropic_with_runtime(
        workspace: PathBuf,
        model: &str,
        api_key: &str,
        runtime: Arc<dyn QbitRuntime>,
    ) -> Result<Self> {
        let config = AnthropicClientConfig {
            workspace,
            model,
            api_key,
        };

        let components = create_anthropic_components(config).await?;

        Ok(Self::from_components_with_runtime(components, runtime))
    }

    /// Create a new AgentBridge for Ollama local inference.
    ///
    /// Uses the `QbitRuntime` trait for event emission and approval handling.
    pub async fn new_ollama_with_runtime(
        workspace: PathBuf,
        model: &str,
        base_url: Option<&str>,
        runtime: Arc<dyn QbitRuntime>,
    ) -> Result<Self> {
        let config = OllamaClientConfig {
            workspace,
            model,
            base_url,
        };

        let components = create_ollama_components(config).await?;

        Ok(Self::from_components_with_runtime(components, runtime))
    }

    /// Create a new AgentBridge for Gemini.
    ///
    /// Uses the `QbitRuntime` trait for event emission and approval handling.
    pub async fn new_gemini_with_runtime(
        workspace: PathBuf,
        model: &str,
        api_key: &str,
        runtime: Arc<dyn QbitRuntime>,
    ) -> Result<Self> {
        let config = GeminiClientConfig {
            workspace,
            model,
            api_key,
        };

        let components = create_gemini_components(config).await?;

        Ok(Self::from_components_with_runtime(components, runtime))
    }

    /// Create a new AgentBridge for Groq.
    ///
    /// Uses the `QbitRuntime` trait for event emission and approval handling.
    pub async fn new_groq_with_runtime(
        workspace: PathBuf,
        model: &str,
        api_key: &str,
        runtime: Arc<dyn QbitRuntime>,
    ) -> Result<Self> {
        let config = GroqClientConfig {
            workspace,
            model,
            api_key,
        };

        let components = create_groq_components(config).await?;

        Ok(Self::from_components_with_runtime(components, runtime))
    }

    /// Create a new AgentBridge for xAI (Grok).
    ///
    /// Uses the `QbitRuntime` trait for event emission and approval handling.
    pub async fn new_xai_with_runtime(
        workspace: PathBuf,
        model: &str,
        api_key: &str,
        runtime: Arc<dyn QbitRuntime>,
    ) -> Result<Self> {
        let config = XaiClientConfig {
            workspace,
            model,
            api_key,
        };

        let components = create_xai_components(config).await?;

        Ok(Self::from_components_with_runtime(components, runtime))
    }

    /// Create a new AgentBridge for Z.AI (GLM models).
    ///
    /// Uses the `QbitRuntime` trait for event emission and approval handling.
    ///
    /// # Arguments
    /// * `workspace` - Path to the workspace directory
    /// * `model` - Model identifier (e.g., "GLM-4.7")
    /// * `api_key` - Z.AI API key
    /// * `use_coding_endpoint` - Whether to use the coding-optimized endpoint
    /// * `runtime` - Runtime abstraction for events and approvals
    pub async fn new_zai_with_runtime(
        workspace: PathBuf,
        model: &str,
        api_key: &str,
        use_coding_endpoint: bool,
        runtime: Arc<dyn QbitRuntime>,
    ) -> Result<Self> {
        let config = ZaiClientConfig {
            workspace,
            model,
            api_key,
            use_coding_endpoint,
        };

        let components = create_zai_components(config).await?;

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
            tavily_state: None,
            session_manager: Default::default(),
            session_persistence_enabled: Arc::new(RwLock::new(true)),
            approval_recorder,
            pending_approvals: Default::default(),
            tool_policy_manager,
            context_manager,
            loop_detector,
            tool_config: ToolConfig::main_agent(),
            agent_mode: Arc::new(RwLock::new(AgentMode::default())),
            plan_manager: Arc::new(PlanManager::new()),
            sidecar_state: None,
            memory_file_path: Arc::new(RwLock::new(None)),
            settings_manager: None,
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

    /// Set the TavilyState for web search tools
    pub fn set_tavily_state(&mut self, tavily_state: Arc<TavilyState>) {
        self.tavily_state = Some(tavily_state);
    }

    /// Set the SidecarState for context capture
    pub fn set_sidecar_state(&mut self, sidecar_state: Arc<SidecarState>) {
        self.sidecar_state = Some(sidecar_state);
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

        // Note: vtcode-core's ToolRegistry doesn't support runtime workspace updates
        // The workspace is set during registry creation
        tracing::debug!(
            "[cwd-sync] Updated workspace to: {}",
            new_workspace.display()
        );
    }

    /// Set the agent mode.
    /// This controls how tool approvals are handled.
    pub async fn set_agent_mode(&self, mode: AgentMode) {
        let mut current = self.agent_mode.write().await;
        tracing::info!("Agent mode changed: {} -> {}", *current, mode);
        *current = mode;
    }

    /// Get the current agent mode.
    pub async fn get_agent_mode(&self) -> AgentMode {
        *self.agent_mode.read().await
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

    /// Get the tavily state.
    pub fn tavily_state(&self) -> Option<&Arc<TavilyState>> {
        self.tavily_state.as_ref()
    }

    // ========================================================================
    // Main Execution Methods
    // ========================================================================

    /// Execute a prompt with agentic tool loop.
    pub async fn execute(&self, prompt: &str) -> Result<String> {
        self.execute_with_context(prompt, SubAgentContext::default())
            .await
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
        }
    }

    /// Execute with Vertex AI model using the agentic loop.
    async fn execute_with_vertex_model(
        &self,
        model: &rig_anthropic_vertex::CompletionModel,
        initial_prompt: &str,
        start_time: std::time::Instant,
        context: SubAgentContext,
    ) -> Result<String> {
        // Build system prompt with current agent mode and memory file
        let workspace_path = self.workspace.read().await;
        let agent_mode = *self.agent_mode.read().await;
        let memory_file_path = self.get_memory_file_path_dynamic().await;
        let mut system_prompt =
            build_system_prompt(&workspace_path, agent_mode, memory_file_path.as_deref());
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

        // Build agentic loop context
        let loop_ctx = AgenticLoopContext {
            event_tx: &loop_event_tx,
            tool_registry: &self.tool_registry,
            sub_agent_registry: &self.sub_agent_registry,
            indexer_state: self.indexer_state.as_ref(),
            tavily_state: self.tavily_state.as_ref(),
            workspace: &self.workspace,
            client: &self.client,
            approval_recorder: &self.approval_recorder,
            pending_approvals: &self.pending_approvals,
            tool_policy_manager: &self.tool_policy_manager,
            context_manager: &self.context_manager,
            loop_detector: &self.loop_detector,
            tool_config: &self.tool_config,
            sidecar_state: self.sidecar_state.as_ref(),
            runtime: self.runtime.as_ref(),
            // No cancellation token for non-server execute paths
            // (cancellation is handled at the execute_with_cancellation level)
            agent_mode: &self.agent_mode,
            plan_manager: &self.plan_manager,
        };

        // Run the agentic loop
        let (accumulated_response, _final_history, token_usage) =
            run_agentic_loop(model, &system_prompt, initial_history, context, &loop_ctx).await?;

        let duration_ms = start_time.elapsed().as_millis() as u64;

        // Persist the assistant response
        if !accumulated_response.is_empty() {
            let mut history_guard = self.conversation_history.write().await;
            history_guard.push(Message::Assistant {
                id: None,
                content: OneOrMany::one(AssistantContent::Text(Text {
                    text: accumulated_response.clone(),
                })),
            });
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

        // Note: Sidecar session is NOT ended here - it persists across prompts in the
        // same conversation. The session is only ended when:
        // 1. The AgentBridge is dropped (see Drop impl)
        // 2. The conversation is explicitly cleared
        // 3. A new conversation/session is started

        // Emit completion event
        self.emit_event(AiEvent::Completed {
            response: accumulated_response.clone(),
            input_tokens: token_usage.as_ref().map(|u| u.input_tokens as u32),
            output_tokens: token_usage.as_ref().map(|u| u.output_tokens as u32),
            duration_ms: Some(duration_ms),
        });

        Ok(accumulated_response)
    }

    /// Execute with OpenRouter model using the generic agentic loop.
    async fn execute_with_openrouter_model(
        &self,
        model: &rig_openrouter::CompletionModel,
        initial_prompt: &str,
        start_time: std::time::Instant,
        context: SubAgentContext,
    ) -> Result<String> {
        // Build system prompt with current agent mode
        let workspace_path = self.workspace.read().await;
        let agent_mode = *self.agent_mode.read().await;
        let memory_file_path = self.get_memory_file_path_dynamic().await;
        let mut system_prompt =
            build_system_prompt(&workspace_path, agent_mode, memory_file_path.as_deref());
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
            if let Some(sid) = session_id {
                let prompt_event = SessionEvent::user_prompt(sid, initial_prompt);
                sidecar.capture(prompt_event);
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

        // Build agentic loop context
        let loop_ctx = AgenticLoopContext {
            event_tx: &loop_event_tx,
            tool_registry: &self.tool_registry,
            sub_agent_registry: &self.sub_agent_registry,
            indexer_state: self.indexer_state.as_ref(),
            tavily_state: self.tavily_state.as_ref(),
            workspace: &self.workspace,
            client: &self.client,
            approval_recorder: &self.approval_recorder,
            pending_approvals: &self.pending_approvals,
            tool_policy_manager: &self.tool_policy_manager,
            context_manager: &self.context_manager,
            loop_detector: &self.loop_detector,
            tool_config: &self.tool_config,
            sidecar_state: self.sidecar_state.as_ref(),
            runtime: self.runtime.as_ref(),
            // No cancellation token for non-server execute paths
            agent_mode: &self.agent_mode,
            plan_manager: &self.plan_manager,
        };

        // Run the generic agentic loop (works with any rig CompletionModel)
        let (accumulated_response, _final_history, token_usage) =
            run_agentic_loop_generic(model, &system_prompt, initial_history, context, &loop_ctx)
                .await?;

        let duration_ms = start_time.elapsed().as_millis() as u64;

        // Persist the assistant response
        if !accumulated_response.is_empty() {
            let mut history_guard = self.conversation_history.write().await;
            history_guard.push(Message::Assistant {
                id: None,
                content: OneOrMany::one(AssistantContent::Text(Text {
                    text: accumulated_response.clone(),
                })),
            });
        }

        // Record and save session
        if !accumulated_response.is_empty() {
            self.record_assistant_message(&accumulated_response).await;
            self.save_session().await;
        }

        // End sidecar capture session
        if let Some(ref sidecar) = self.sidecar_state {
            match sidecar.end_session() {
                Ok(Some(session)) => {
                    tracing::info!("Sidecar session {} ended", session.session_id);
                }
                Ok(None) => {
                    tracing::debug!("No active sidecar session to end");
                }
                Err(e) => {
                    tracing::warn!("Failed to end sidecar session: {}", e);
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

        Ok(accumulated_response)
    }

    /// Execute with OpenAI model using the generic agentic loop.
    async fn execute_with_openai_model(
        &self,
        model: &rig_openai::completion::CompletionModel,
        initial_prompt: &str,
        start_time: std::time::Instant,
        context: SubAgentContext,
    ) -> Result<String> {
        // Build system prompt with current agent mode
        let workspace_path = self.workspace.read().await;
        let agent_mode = *self.agent_mode.read().await;
        let memory_file_path = self.get_memory_file_path_dynamic().await;
        let mut system_prompt =
            build_system_prompt(&workspace_path, agent_mode, memory_file_path.as_deref());
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
        if let Some(ref sidecar) = self.sidecar_state {
            use qbit_sidecar::events::SessionEvent;

            let session_id = if let Some(existing_id) = sidecar.current_session_id() {
                tracing::debug!("Reusing existing sidecar session: {}", existing_id);
                Some(existing_id)
            } else {
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

            if let Some(sid) = session_id {
                let prompt_event = SessionEvent::user_prompt(sid, initial_prompt);
                sidecar.capture(prompt_event);
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
        let loop_event_tx = self.get_or_create_event_tx();

        // Build agentic loop context
        let loop_ctx = AgenticLoopContext {
            event_tx: &loop_event_tx,
            tool_registry: &self.tool_registry,
            sub_agent_registry: &self.sub_agent_registry,
            indexer_state: self.indexer_state.as_ref(),
            tavily_state: self.tavily_state.as_ref(),
            workspace: &self.workspace,
            client: &self.client,
            approval_recorder: &self.approval_recorder,
            pending_approvals: &self.pending_approvals,
            tool_policy_manager: &self.tool_policy_manager,
            context_manager: &self.context_manager,
            loop_detector: &self.loop_detector,
            tool_config: &self.tool_config,
            sidecar_state: self.sidecar_state.as_ref(),
            runtime: self.runtime.as_ref(),
            agent_mode: &self.agent_mode,
            plan_manager: &self.plan_manager,
        };

        // Run the generic agentic loop (works with any rig CompletionModel)
        let (accumulated_response, _final_history, token_usage) =
            run_agentic_loop_generic(model, &system_prompt, initial_history, context, &loop_ctx)
                .await?;

        let duration_ms = start_time.elapsed().as_millis() as u64;

        // Persist the assistant response
        if !accumulated_response.is_empty() {
            let mut history_guard = self.conversation_history.write().await;
            history_guard.push(Message::Assistant {
                id: None,
                content: OneOrMany::one(AssistantContent::Text(Text {
                    text: accumulated_response.clone(),
                })),
            });
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

        Ok(accumulated_response)
    }

    /// Execute with Anthropic model using the generic agentic loop.
    async fn execute_with_anthropic_model(
        &self,
        model: &rig_anthropic::completion::CompletionModel,
        initial_prompt: &str,
        start_time: std::time::Instant,
        context: SubAgentContext,
    ) -> Result<String> {
        // Build system prompt with current agent mode
        let workspace_path = self.workspace.read().await;
        let agent_mode = *self.agent_mode.read().await;
        let memory_file_path = self.get_memory_file_path_dynamic().await;
        let mut system_prompt =
            build_system_prompt(&workspace_path, agent_mode, memory_file_path.as_deref());
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
        if let Some(ref sidecar) = self.sidecar_state {
            use qbit_sidecar::events::SessionEvent;

            let session_id = if let Some(existing_id) = sidecar.current_session_id() {
                tracing::debug!("Reusing existing sidecar session: {}", existing_id);
                Some(existing_id)
            } else {
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

            if let Some(sid) = session_id {
                let prompt_event = SessionEvent::user_prompt(sid, initial_prompt);
                sidecar.capture(prompt_event);
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

        let loop_event_tx = self.get_or_create_event_tx();

        let loop_ctx = AgenticLoopContext {
            event_tx: &loop_event_tx,
            tool_registry: &self.tool_registry,
            sub_agent_registry: &self.sub_agent_registry,
            indexer_state: self.indexer_state.as_ref(),
            tavily_state: self.tavily_state.as_ref(),
            workspace: &self.workspace,
            client: &self.client,
            approval_recorder: &self.approval_recorder,
            pending_approvals: &self.pending_approvals,
            tool_policy_manager: &self.tool_policy_manager,
            context_manager: &self.context_manager,
            loop_detector: &self.loop_detector,
            tool_config: &self.tool_config,
            sidecar_state: self.sidecar_state.as_ref(),
            runtime: self.runtime.as_ref(),
            agent_mode: &self.agent_mode,
            plan_manager: &self.plan_manager,
        };

        let (accumulated_response, _final_history, token_usage) =
            run_agentic_loop_generic(model, &system_prompt, initial_history, context, &loop_ctx)
                .await?;

        let duration_ms = start_time.elapsed().as_millis() as u64;

        if !accumulated_response.is_empty() {
            let mut history_guard = self.conversation_history.write().await;
            history_guard.push(Message::Assistant {
                id: None,
                content: OneOrMany::one(AssistantContent::Text(Text {
                    text: accumulated_response.clone(),
                })),
            });
        }

        if !accumulated_response.is_empty() {
            self.record_assistant_message(&accumulated_response).await;
            self.save_session().await;
        }

        if let Some(ref sidecar) = self.sidecar_state {
            use qbit_sidecar::events::SessionEvent;

            if let Some(session_id) = sidecar.current_session_id() {
                if !accumulated_response.is_empty() {
                    let response_event =
                        SessionEvent::ai_response(session_id, &accumulated_response);
                    sidecar.capture(response_event);
                }
            }
        }

        self.emit_event(AiEvent::Completed {
            response: accumulated_response.clone(),
            input_tokens: token_usage.as_ref().map(|u| u.input_tokens as u32),
            output_tokens: token_usage.as_ref().map(|u| u.output_tokens as u32),
            duration_ms: Some(duration_ms),
        });

        Ok(accumulated_response)
    }

    /// Execute with Ollama model using the generic agentic loop.
    async fn execute_with_ollama_model(
        &self,
        model: &rig_ollama::CompletionModel<reqwest::Client>,
        initial_prompt: &str,
        start_time: std::time::Instant,
        context: SubAgentContext,
    ) -> Result<String> {
        // Build system prompt with current agent mode
        let workspace_path = self.workspace.read().await;
        let agent_mode = *self.agent_mode.read().await;
        let memory_file_path = self.get_memory_file_path_dynamic().await;
        let mut system_prompt =
            build_system_prompt(&workspace_path, agent_mode, memory_file_path.as_deref());
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
        if let Some(ref sidecar) = self.sidecar_state {
            use qbit_sidecar::events::SessionEvent;

            let session_id = if let Some(existing_id) = sidecar.current_session_id() {
                tracing::debug!("Reusing existing sidecar session: {}", existing_id);
                Some(existing_id)
            } else {
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

            if let Some(sid) = session_id {
                let prompt_event = SessionEvent::user_prompt(sid, initial_prompt);
                sidecar.capture(prompt_event);
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

        let loop_event_tx = self.get_or_create_event_tx();

        let loop_ctx = AgenticLoopContext {
            event_tx: &loop_event_tx,
            tool_registry: &self.tool_registry,
            sub_agent_registry: &self.sub_agent_registry,
            indexer_state: self.indexer_state.as_ref(),
            tavily_state: self.tavily_state.as_ref(),
            workspace: &self.workspace,
            client: &self.client,
            approval_recorder: &self.approval_recorder,
            pending_approvals: &self.pending_approvals,
            tool_policy_manager: &self.tool_policy_manager,
            context_manager: &self.context_manager,
            loop_detector: &self.loop_detector,
            tool_config: &self.tool_config,
            sidecar_state: self.sidecar_state.as_ref(),
            runtime: self.runtime.as_ref(),
            agent_mode: &self.agent_mode,
            plan_manager: &self.plan_manager,
        };

        let (accumulated_response, _final_history, token_usage) =
            run_agentic_loop_generic(model, &system_prompt, initial_history, context, &loop_ctx)
                .await?;

        let duration_ms = start_time.elapsed().as_millis() as u64;

        if !accumulated_response.is_empty() {
            let mut history_guard = self.conversation_history.write().await;
            history_guard.push(Message::Assistant {
                id: None,
                content: OneOrMany::one(AssistantContent::Text(Text {
                    text: accumulated_response.clone(),
                })),
            });
        }

        if !accumulated_response.is_empty() {
            self.record_assistant_message(&accumulated_response).await;
            self.save_session().await;
        }

        if let Some(ref sidecar) = self.sidecar_state {
            use qbit_sidecar::events::SessionEvent;

            if let Some(session_id) = sidecar.current_session_id() {
                if !accumulated_response.is_empty() {
                    let response_event =
                        SessionEvent::ai_response(session_id, &accumulated_response);
                    sidecar.capture(response_event);
                }
            }
        }

        self.emit_event(AiEvent::Completed {
            response: accumulated_response.clone(),
            input_tokens: token_usage.as_ref().map(|u| u.input_tokens as u32),
            output_tokens: token_usage.as_ref().map(|u| u.output_tokens as u32),
            duration_ms: Some(duration_ms),
        });

        Ok(accumulated_response)
    }

    /// Execute with Gemini model using the generic agentic loop.
    async fn execute_with_gemini_model(
        &self,
        model: &rig_gemini::completion::CompletionModel,
        initial_prompt: &str,
        start_time: std::time::Instant,
        context: SubAgentContext,
    ) -> Result<String> {
        // Build system prompt with current agent mode
        let workspace_path = self.workspace.read().await;
        let agent_mode = *self.agent_mode.read().await;
        let memory_file_path = self.get_memory_file_path_dynamic().await;
        let mut system_prompt =
            build_system_prompt(&workspace_path, agent_mode, memory_file_path.as_deref());
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
        if let Some(ref sidecar) = self.sidecar_state {
            use qbit_sidecar::events::SessionEvent;

            let session_id = if let Some(existing_id) = sidecar.current_session_id() {
                tracing::debug!("Reusing existing sidecar session: {}", existing_id);
                Some(existing_id)
            } else {
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

            if let Some(sid) = session_id {
                let prompt_event = SessionEvent::user_prompt(sid, initial_prompt);
                sidecar.capture(prompt_event);
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

        let loop_event_tx = self.get_or_create_event_tx();

        let loop_ctx = AgenticLoopContext {
            event_tx: &loop_event_tx,
            tool_registry: &self.tool_registry,
            sub_agent_registry: &self.sub_agent_registry,
            indexer_state: self.indexer_state.as_ref(),
            tavily_state: self.tavily_state.as_ref(),
            workspace: &self.workspace,
            client: &self.client,
            approval_recorder: &self.approval_recorder,
            pending_approvals: &self.pending_approvals,
            tool_policy_manager: &self.tool_policy_manager,
            context_manager: &self.context_manager,
            loop_detector: &self.loop_detector,
            tool_config: &self.tool_config,
            sidecar_state: self.sidecar_state.as_ref(),
            runtime: self.runtime.as_ref(),
            agent_mode: &self.agent_mode,
            plan_manager: &self.plan_manager,
        };

        let (accumulated_response, _final_history, token_usage) =
            run_agentic_loop_generic(model, &system_prompt, initial_history, context, &loop_ctx)
                .await?;

        let duration_ms = start_time.elapsed().as_millis() as u64;

        if !accumulated_response.is_empty() {
            let mut history_guard = self.conversation_history.write().await;
            history_guard.push(Message::Assistant {
                id: None,
                content: OneOrMany::one(AssistantContent::Text(Text {
                    text: accumulated_response.clone(),
                })),
            });
        }

        if !accumulated_response.is_empty() {
            self.record_assistant_message(&accumulated_response).await;
            self.save_session().await;
        }

        if let Some(ref sidecar) = self.sidecar_state {
            use qbit_sidecar::events::SessionEvent;

            if let Some(session_id) = sidecar.current_session_id() {
                if !accumulated_response.is_empty() {
                    let response_event =
                        SessionEvent::ai_response(session_id, &accumulated_response);
                    sidecar.capture(response_event);
                }
            }
        }

        self.emit_event(AiEvent::Completed {
            response: accumulated_response.clone(),
            input_tokens: token_usage.as_ref().map(|u| u.input_tokens as u32),
            output_tokens: token_usage.as_ref().map(|u| u.output_tokens as u32),
            duration_ms: Some(duration_ms),
        });

        Ok(accumulated_response)
    }

    /// Execute with Groq model using the generic agentic loop.
    async fn execute_with_groq_model(
        &self,
        model: &rig_groq::CompletionModel<reqwest::Client>,
        initial_prompt: &str,
        start_time: std::time::Instant,
        context: SubAgentContext,
    ) -> Result<String> {
        // Build system prompt with current agent mode
        let workspace_path = self.workspace.read().await;
        let agent_mode = *self.agent_mode.read().await;
        let memory_file_path = self.get_memory_file_path_dynamic().await;
        let mut system_prompt =
            build_system_prompt(&workspace_path, agent_mode, memory_file_path.as_deref());
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
        if let Some(ref sidecar) = self.sidecar_state {
            use qbit_sidecar::events::SessionEvent;

            let session_id = if let Some(existing_id) = sidecar.current_session_id() {
                tracing::debug!("Reusing existing sidecar session: {}", existing_id);
                Some(existing_id)
            } else {
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

            if let Some(sid) = session_id {
                let prompt_event = SessionEvent::user_prompt(sid, initial_prompt);
                sidecar.capture(prompt_event);
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

        let loop_event_tx = self.get_or_create_event_tx();

        let loop_ctx = AgenticLoopContext {
            event_tx: &loop_event_tx,
            tool_registry: &self.tool_registry,
            sub_agent_registry: &self.sub_agent_registry,
            indexer_state: self.indexer_state.as_ref(),
            tavily_state: self.tavily_state.as_ref(),
            workspace: &self.workspace,
            client: &self.client,
            approval_recorder: &self.approval_recorder,
            pending_approvals: &self.pending_approvals,
            tool_policy_manager: &self.tool_policy_manager,
            context_manager: &self.context_manager,
            loop_detector: &self.loop_detector,
            tool_config: &self.tool_config,
            sidecar_state: self.sidecar_state.as_ref(),
            runtime: self.runtime.as_ref(),
            agent_mode: &self.agent_mode,
            plan_manager: &self.plan_manager,
        };

        let (accumulated_response, _final_history, token_usage) =
            run_agentic_loop_generic(model, &system_prompt, initial_history, context, &loop_ctx)
                .await?;

        let duration_ms = start_time.elapsed().as_millis() as u64;

        if !accumulated_response.is_empty() {
            let mut history_guard = self.conversation_history.write().await;
            history_guard.push(Message::Assistant {
                id: None,
                content: OneOrMany::one(AssistantContent::Text(Text {
                    text: accumulated_response.clone(),
                })),
            });
        }

        if !accumulated_response.is_empty() {
            self.record_assistant_message(&accumulated_response).await;
            self.save_session().await;
        }

        if let Some(ref sidecar) = self.sidecar_state {
            use qbit_sidecar::events::SessionEvent;

            if let Some(session_id) = sidecar.current_session_id() {
                if !accumulated_response.is_empty() {
                    let response_event =
                        SessionEvent::ai_response(session_id, &accumulated_response);
                    sidecar.capture(response_event);
                }
            }
        }

        self.emit_event(AiEvent::Completed {
            response: accumulated_response.clone(),
            input_tokens: token_usage.as_ref().map(|u| u.input_tokens as u32),
            output_tokens: token_usage.as_ref().map(|u| u.output_tokens as u32),
            duration_ms: Some(duration_ms),
        });

        Ok(accumulated_response)
    }

    /// Execute with xAI (Grok) model using the generic agentic loop.
    async fn execute_with_xai_model(
        &self,
        model: &rig_xai::completion::CompletionModel<reqwest::Client>,
        initial_prompt: &str,
        start_time: std::time::Instant,
        context: SubAgentContext,
    ) -> Result<String> {
        // Build system prompt with current agent mode
        let workspace_path = self.workspace.read().await;
        let agent_mode = *self.agent_mode.read().await;
        let memory_file_path = self.get_memory_file_path_dynamic().await;
        let mut system_prompt =
            build_system_prompt(&workspace_path, agent_mode, memory_file_path.as_deref());
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
        if let Some(ref sidecar) = self.sidecar_state {
            use qbit_sidecar::events::SessionEvent;

            let session_id = if let Some(existing_id) = sidecar.current_session_id() {
                tracing::debug!("Reusing existing sidecar session: {}", existing_id);
                Some(existing_id)
            } else {
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

            if let Some(sid) = session_id {
                let prompt_event = SessionEvent::user_prompt(sid, initial_prompt);
                sidecar.capture(prompt_event);
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

        let loop_event_tx = self.get_or_create_event_tx();

        let loop_ctx = AgenticLoopContext {
            event_tx: &loop_event_tx,
            tool_registry: &self.tool_registry,
            sub_agent_registry: &self.sub_agent_registry,
            indexer_state: self.indexer_state.as_ref(),
            tavily_state: self.tavily_state.as_ref(),
            workspace: &self.workspace,
            client: &self.client,
            approval_recorder: &self.approval_recorder,
            pending_approvals: &self.pending_approvals,
            tool_policy_manager: &self.tool_policy_manager,
            context_manager: &self.context_manager,
            loop_detector: &self.loop_detector,
            tool_config: &self.tool_config,
            sidecar_state: self.sidecar_state.as_ref(),
            runtime: self.runtime.as_ref(),
            agent_mode: &self.agent_mode,
            plan_manager: &self.plan_manager,
        };

        let (accumulated_response, _final_history, token_usage) =
            run_agentic_loop_generic(model, &system_prompt, initial_history, context, &loop_ctx)
                .await?;

        let duration_ms = start_time.elapsed().as_millis() as u64;

        if !accumulated_response.is_empty() {
            let mut history_guard = self.conversation_history.write().await;
            history_guard.push(Message::Assistant {
                id: None,
                content: OneOrMany::one(AssistantContent::Text(Text {
                    text: accumulated_response.clone(),
                })),
            });
        }

        if !accumulated_response.is_empty() {
            self.record_assistant_message(&accumulated_response).await;
            self.save_session().await;
        }

        if let Some(ref sidecar) = self.sidecar_state {
            use qbit_sidecar::events::SessionEvent;

            if let Some(session_id) = sidecar.current_session_id() {
                if !accumulated_response.is_empty() {
                    let response_event =
                        SessionEvent::ai_response(session_id, &accumulated_response);
                    sidecar.capture(response_event);
                }
            }
        }

        self.emit_event(AiEvent::Completed {
            response: accumulated_response.clone(),
            input_tokens: token_usage.as_ref().map(|u| u.input_tokens as u32),
            output_tokens: token_usage.as_ref().map(|u| u.output_tokens as u32),
            duration_ms: Some(duration_ms),
        });

        Ok(accumulated_response)
    }

    /// Execute with Z.AI (GLM) model using the generic agentic loop.
    async fn execute_with_zai_model(
        &self,
        model: &rig_zai::CompletionModel<reqwest::Client>,
        initial_prompt: &str,
        start_time: std::time::Instant,
        context: SubAgentContext,
    ) -> Result<String> {
        // Build system prompt with current agent mode
        let workspace_path = self.workspace.read().await;
        let agent_mode = *self.agent_mode.read().await;
        let memory_file_path = self.get_memory_file_path_dynamic().await;
        let mut system_prompt =
            build_system_prompt(&workspace_path, agent_mode, memory_file_path.as_deref());
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
        if let Some(ref sidecar) = self.sidecar_state {
            use qbit_sidecar::events::SessionEvent;

            let session_id = if let Some(existing_id) = sidecar.current_session_id() {
                tracing::debug!("Reusing existing sidecar session: {}", existing_id);
                Some(existing_id)
            } else {
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

            if let Some(sid) = session_id {
                let prompt_event = SessionEvent::user_prompt(sid, initial_prompt);
                sidecar.capture(prompt_event);
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

        let loop_event_tx = self.get_or_create_event_tx();

        let loop_ctx = AgenticLoopContext {
            event_tx: &loop_event_tx,
            tool_registry: &self.tool_registry,
            sub_agent_registry: &self.sub_agent_registry,
            indexer_state: self.indexer_state.as_ref(),
            tavily_state: self.tavily_state.as_ref(),
            workspace: &self.workspace,
            client: &self.client,
            approval_recorder: &self.approval_recorder,
            pending_approvals: &self.pending_approvals,
            tool_policy_manager: &self.tool_policy_manager,
            context_manager: &self.context_manager,
            loop_detector: &self.loop_detector,
            tool_config: &self.tool_config,
            sidecar_state: self.sidecar_state.as_ref(),
            runtime: self.runtime.as_ref(),
            agent_mode: &self.agent_mode,
            plan_manager: &self.plan_manager,
        };

        let (accumulated_response, _final_history, token_usage) =
            run_agentic_loop_generic(model, &system_prompt, initial_history, context, &loop_ctx)
                .await?;

        let duration_ms = start_time.elapsed().as_millis() as u64;

        if !accumulated_response.is_empty() {
            let mut history_guard = self.conversation_history.write().await;
            history_guard.push(Message::Assistant {
                id: None,
                content: OneOrMany::one(AssistantContent::Text(Text {
                    text: accumulated_response.clone(),
                })),
            });
        }

        if !accumulated_response.is_empty() {
            self.record_assistant_message(&accumulated_response).await;
            self.save_session().await;
        }

        if let Some(ref sidecar) = self.sidecar_state {
            use qbit_sidecar::events::SessionEvent;

            if let Some(session_id) = sidecar.current_session_id() {
                if !accumulated_response.is_empty() {
                    let response_event =
                        SessionEvent::ai_response(session_id, &accumulated_response);
                    sidecar.capture(response_event);
                }
            }
        }

        self.emit_event(AiEvent::Completed {
            response: accumulated_response.clone(),
            input_tokens: token_usage.as_ref().map(|u| u.input_tokens as u32),
            output_tokens: token_usage.as_ref().map(|u| u.output_tokens as u32),
            duration_ms: Some(duration_ms),
        });

        Ok(accumulated_response)
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
        let tool_names = registry.available_tools().await;

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
