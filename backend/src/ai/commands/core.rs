// Core AI agent commands for initialization and execution.

use std::sync::Arc;
use tauri::{AppHandle, State};

use super::super::agent_bridge::AgentBridge;
use super::super::llm_client::ProviderConfig;
use super::configure_bridge;
use crate::runtime::{QbitRuntime, TauriRuntime};
use crate::state::AppState;

/// Initialize the AI agent with the specified configuration.
///
/// If an existing AI agent is running, its session will be finalized and the
/// sidecar session will be ended before the new agent is initialized.
///
/// # Arguments
/// * `workspace` - Path to the workspace directory
/// * `provider` - LLM provider name (e.g., "openrouter", "anthropic")
/// * `model` - Model identifier (e.g., "anthropic/claude-3.5-sonnet")
/// * `api_key` - API key for the provider
#[tauri::command]
pub async fn init_ai_agent(
    state: State<'_, AppState>,
    app: AppHandle,
    workspace: String,
    provider: String,
    model: String,
    api_key: String,
) -> Result<(), String> {
    // Clean up existing session before replacing the bridge
    // This ensures sessions are properly finalized when switching models/providers
    {
        let bridge_guard = state.ai_state.bridge.read().await;
        if bridge_guard.is_some() {
            // End the sidecar session (the bridge's Drop impl will finalize its session)
            if let Err(e) = state.sidecar_state.end_session() {
                tracing::warn!("Failed to end sidecar session during agent reinit: {}", e);
            } else {
                tracing::debug!("Sidecar session ended during agent reinit");
            }
        }
    }

    // Phase 5: Use runtime-based constructor
    // TauriRuntime handles event emission via Tauri's event system
    let runtime: Arc<dyn QbitRuntime> = Arc::new(TauriRuntime::new(app));

    // Store runtime in AiState (for potential future use by other components)
    *state.ai_state.runtime.write().await = Some(runtime.clone());

    // Create bridge with runtime (Phase 5 - new path)
    let mut bridge =
        AgentBridge::new_with_runtime(workspace.into(), &provider, &model, &api_key, runtime)
            .await
            .map_err(|e| e.to_string())?;

    configure_bridge(&mut bridge, &state);

    // Replace the bridge (old bridge's Drop impl will finalize its session)
    *state.ai_state.bridge.write().await = Some(bridge);

    tracing::info!(
        "AI agent initialized with provider: {}, model: {}",
        provider,
        model
    );
    Ok(())
}

/// Initialize the AI agent using unified provider configuration.
///
/// This is the unified initialization command that can handle any provider
/// using the ProviderConfig enum. It routes to the appropriate AgentBridge
/// constructor based on the provider variant.
///
/// If an existing AI agent is running, its session will be finalized and the
/// sidecar session will be ended before the new agent is initialized.
///
/// # Arguments
/// * `config` - Provider-specific configuration (VertexAi, Openrouter, Openai, etc.)
#[tauri::command]
pub async fn init_ai_agent_unified(
    state: State<'_, AppState>,
    app: AppHandle,
    config: ProviderConfig,
) -> Result<(), String> {
    // Clean up existing session before replacing the bridge
    {
        let bridge_guard = state.ai_state.bridge.read().await;
        if bridge_guard.is_some() {
            if let Err(e) = state.sidecar_state.end_session() {
                tracing::warn!("Failed to end sidecar session during agent reinit: {}", e);
            } else {
                tracing::debug!("Sidecar session ended during agent reinit");
            }
        }
    }

    // Create runtime for event emission
    let runtime: Arc<dyn QbitRuntime> = Arc::new(TauriRuntime::new(app));
    *state.ai_state.runtime.write().await = Some(runtime.clone());

    let workspace_path: std::path::PathBuf = config.workspace().into();
    let provider_name = config.provider_name().to_string();
    let model_name = config.model().to_string();

    // Dispatch to appropriate AgentBridge constructor based on provider
    let mut bridge = match config {
        ProviderConfig::VertexAi {
            workspace: _,
            model,
            credentials_path,
            project_id,
            location,
        } => {
            AgentBridge::new_vertex_anthropic_with_runtime(
                workspace_path.clone(),
                &credentials_path,
                &project_id,
                &location,
                &model,
                runtime,
            )
            .await
        }
        ProviderConfig::Openrouter {
            workspace: _,
            model,
            api_key,
        } => {
            AgentBridge::new_with_runtime(
                workspace_path.clone(),
                "openrouter",
                &model,
                &api_key,
                runtime,
            )
            .await
        }
        ProviderConfig::Openai {
            workspace: _,
            model,
            api_key,
            base_url,
            reasoning_effort,
        } => {
            AgentBridge::new_openai_with_runtime(
                workspace_path.clone(),
                &model,
                &api_key,
                base_url.as_deref(),
                reasoning_effort.as_deref(),
                runtime,
            )
            .await
        }
        ProviderConfig::Anthropic {
            workspace: _,
            model,
            api_key,
        } => {
            AgentBridge::new_anthropic_with_runtime(
                workspace_path.clone(),
                &model,
                &api_key,
                runtime,
            )
            .await
        }
        ProviderConfig::Ollama {
            workspace: _,
            model,
            base_url,
        } => {
            AgentBridge::new_ollama_with_runtime(
                workspace_path.clone(),
                &model,
                base_url.as_deref(),
                runtime,
            )
            .await
        }
        ProviderConfig::Gemini {
            workspace: _,
            model,
            api_key,
        } => {
            AgentBridge::new_gemini_with_runtime(workspace_path.clone(), &model, &api_key, runtime)
                .await
        }
        ProviderConfig::Groq {
            workspace: _,
            model,
            api_key,
        } => {
            AgentBridge::new_groq_with_runtime(workspace_path.clone(), &model, &api_key, runtime)
                .await
        }
        ProviderConfig::Xai {
            workspace: _,
            model,
            api_key,
        } => {
            AgentBridge::new_xai_with_runtime(workspace_path.clone(), &model, &api_key, runtime)
                .await
        }
    }
    .map_err(|e| e.to_string())?;

    configure_bridge(&mut bridge, &state);

    // Replace the bridge
    *state.ai_state.bridge.write().await = Some(bridge);

    // Initialize sidecar with the workspace
    if let Err(e) = state.sidecar_state.initialize(workspace_path).await {
        tracing::warn!("Failed to initialize sidecar: {}", e);
    } else {
        tracing::info!("Sidecar initialized for workspace");
    }

    tracing::info!(
        "AI agent initialized with provider: {}, model: {}",
        provider_name,
        model_name
    );
    Ok(())
}

/// Send a prompt to the AI agent and receive streaming response via events.
/// This is the legacy command - prefer send_ai_prompt_session for new code.
///
/// # Arguments
/// * `prompt` - The user's message
#[tauri::command]
pub async fn send_ai_prompt(state: State<'_, AppState>, prompt: String) -> Result<String, String> {
    let bridge_guard = state.ai_state.get_bridge().await?;
    let bridge = bridge_guard.as_ref().unwrap();

    bridge.execute(&prompt).await.map_err(|e| e.to_string())
}

/// Execute a specific tool with the given arguments.
#[tauri::command]
pub async fn execute_ai_tool(
    state: State<'_, AppState>,
    tool_name: String,
    args: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let bridge_guard = state.ai_state.get_bridge().await?;
    let bridge = bridge_guard.as_ref().unwrap();

    bridge
        .execute_tool(&tool_name, args)
        .await
        .map_err(|e| e.to_string())
}

/// Get the list of available tools.
#[tauri::command]
pub async fn get_available_tools(
    state: State<'_, AppState>,
) -> Result<Vec<serde_json::Value>, String> {
    let bridge_guard = state.ai_state.get_bridge().await?;
    let bridge = bridge_guard.as_ref().unwrap();
    Ok(bridge.available_tools().await)
}

/// Sub-agent information for the frontend.
#[derive(serde::Serialize)]
pub struct SubAgentInfo {
    pub id: String,
    pub name: String,
    pub description: String,
}

/// Get the list of available sub-agents.
#[tauri::command]
pub async fn list_sub_agents(state: State<'_, AppState>) -> Result<Vec<SubAgentInfo>, String> {
    let bridge_guard = state.ai_state.get_bridge().await?;
    let bridge = bridge_guard.as_ref().unwrap();
    let registry = bridge.sub_agent_registry.read().await;

    Ok(registry
        .all()
        .map(|agent| SubAgentInfo {
            id: agent.id.clone(),
            name: agent.name.clone(),
            description: agent.description.clone(),
        })
        .collect())
}

/// Shutdown the AI agent and cleanup resources.
#[tauri::command]
pub async fn shutdown_ai_agent(state: State<'_, AppState>) -> Result<(), String> {
    let mut bridge_guard = state.ai_state.bridge.write().await;
    *bridge_guard = None;
    tracing::info!("AI agent shut down");
    Ok(())
}

/// Check if the AI agent is initialized.
#[tauri::command]
pub async fn is_ai_initialized(state: State<'_, AppState>) -> Result<bool, String> {
    Ok(state.ai_state.bridge.read().await.is_some())
}

// ========== Session-specific commands ==========

/// Initialize AI agent for a specific session (tab).
///
/// Each session can have its own provider/model configuration, allowing
/// different tabs to use different AI providers simultaneously.
///
/// # Arguments
/// * `session_id` - The terminal session ID (tab) to initialize AI for
/// * `config` - Provider-specific configuration (VertexAi, Openrouter, Openai, etc.)
#[tauri::command]
pub async fn init_ai_session(
    state: State<'_, AppState>,
    app: AppHandle,
    session_id: String,
    config: ProviderConfig,
) -> Result<(), String> {
    // Clean up existing session bridge if present
    if state.ai_state.has_session_bridge(&session_id).await {
        state.ai_state.remove_session_bridge(&session_id).await;
        tracing::debug!("Removed existing AI bridge for session {}", session_id);
    }

    // Create runtime for event emission
    let runtime: Arc<dyn QbitRuntime> = Arc::new(TauriRuntime::new(app));

    let workspace_path: std::path::PathBuf = config.workspace().into();
    let provider_name = config.provider_name().to_string();
    let model_name = config.model().to_string();

    // Dispatch to appropriate AgentBridge constructor based on provider
    let mut bridge = match config {
        ProviderConfig::VertexAi {
            workspace: _,
            model,
            credentials_path,
            project_id,
            location,
        } => {
            AgentBridge::new_vertex_anthropic_with_runtime(
                workspace_path.clone(),
                &credentials_path,
                &project_id,
                &location,
                &model,
                runtime,
            )
            .await
        }
        ProviderConfig::Openrouter {
            workspace: _,
            model,
            api_key,
        } => {
            AgentBridge::new_with_runtime(
                workspace_path.clone(),
                "openrouter",
                &model,
                &api_key,
                runtime,
            )
            .await
        }
        ProviderConfig::Openai {
            workspace: _,
            model,
            api_key,
            base_url,
            reasoning_effort,
        } => {
            AgentBridge::new_openai_with_runtime(
                workspace_path.clone(),
                &model,
                &api_key,
                base_url.as_deref(),
                reasoning_effort.as_deref(),
                runtime,
            )
            .await
        }
        ProviderConfig::Anthropic {
            workspace: _,
            model,
            api_key,
        } => {
            AgentBridge::new_anthropic_with_runtime(
                workspace_path.clone(),
                &model,
                &api_key,
                runtime,
            )
            .await
        }
        ProviderConfig::Ollama {
            workspace: _,
            model,
            base_url,
        } => {
            AgentBridge::new_ollama_with_runtime(
                workspace_path.clone(),
                &model,
                base_url.as_deref(),
                runtime,
            )
            .await
        }
        ProviderConfig::Gemini {
            workspace: _,
            model,
            api_key,
        } => {
            AgentBridge::new_gemini_with_runtime(workspace_path.clone(), &model, &api_key, runtime)
                .await
        }
        ProviderConfig::Groq {
            workspace: _,
            model,
            api_key,
        } => {
            AgentBridge::new_groq_with_runtime(workspace_path.clone(), &model, &api_key, runtime)
                .await
        }
        ProviderConfig::Xai {
            workspace: _,
            model,
            api_key,
        } => {
            AgentBridge::new_xai_with_runtime(workspace_path.clone(), &model, &api_key, runtime)
                .await
        }
    }
    .map_err(|e| e.to_string())?;

    configure_bridge(&mut bridge, &state);

    // Set the session_id for event routing (for per-tab AI event isolation)
    bridge.set_event_session_id(session_id.clone());

    // Set the session_id on the bridge for terminal command execution
    bridge.set_session_id(Some(session_id.clone())).await;

    // Store the bridge in the session map
    state
        .ai_state
        .insert_session_bridge(session_id.clone(), bridge)
        .await;

    tracing::info!(
        "AI agent initialized for session {}: provider={}, model={}",
        session_id,
        provider_name,
        model_name
    );
    Ok(())
}

/// Shutdown AI agent for a specific session.
///
/// Removes the AI agent bridge for the specified session, freeing resources.
/// This should be called when a tab is closed.
#[tauri::command]
pub async fn shutdown_ai_session(
    state: State<'_, AppState>,
    session_id: String,
) -> Result<(), String> {
    if state
        .ai_state
        .remove_session_bridge(&session_id)
        .await
        .is_some()
    {
        tracing::info!("AI agent shut down for session {}", session_id);
        Ok(())
    } else {
        // Not an error - session may not have had AI initialized
        tracing::debug!("No AI agent found for session {} to shut down", session_id);
        Ok(())
    }
}

/// Check if AI agent is initialized for a specific session.
#[tauri::command]
pub async fn is_ai_session_initialized(
    state: State<'_, AppState>,
    session_id: String,
) -> Result<bool, String> {
    Ok(state.ai_state.has_session_bridge(&session_id).await)
}

/// Session AI configuration info.
#[derive(serde::Serialize)]
pub struct SessionAiConfig {
    pub provider: String,
    pub model: String,
}

/// Get the AI configuration for a specific session.
#[tauri::command]
pub async fn get_session_ai_config(
    state: State<'_, AppState>,
    session_id: String,
) -> Result<Option<SessionAiConfig>, String> {
    let bridges = state.ai_state.get_bridges().await;
    if let Some(bridge) = bridges.get(&session_id) {
        Ok(Some(SessionAiConfig {
            provider: bridge.provider_name.clone(),
            model: bridge.model_name.clone(),
        }))
    } else {
        Ok(None)
    }
}

/// Send a prompt to the AI agent for a specific session.
///
/// This is the session-specific version of send_ai_prompt that routes to
/// the correct agent bridge based on session_id.
#[tauri::command]
pub async fn send_ai_prompt_session(
    state: State<'_, AppState>,
    session_id: String,
    prompt: String,
) -> Result<String, String> {
    let bridges = state.ai_state.get_bridges().await;
    let bridge = bridges
        .get(&session_id)
        .ok_or_else(|| super::ai_session_not_initialized_error(&session_id))?;

    bridge.execute(&prompt).await.map_err(|e| e.to_string())
}

/// Clear the conversation history for a specific session.
#[tauri::command]
pub async fn clear_ai_conversation_session(
    state: State<'_, AppState>,
    session_id: String,
) -> Result<(), String> {
    let bridges = state.ai_state.get_bridges().await;
    let bridge = bridges
        .get(&session_id)
        .ok_or_else(|| super::ai_session_not_initialized_error(&session_id))?;
    bridge.clear_conversation_history().await;
    tracing::info!("Conversation cleared for session {}", session_id);
    Ok(())
}

/// Get the conversation length for a specific session.
#[tauri::command]
pub async fn get_ai_conversation_length_session(
    state: State<'_, AppState>,
    session_id: String,
) -> Result<usize, String> {
    let bridges = state.ai_state.get_bridges().await;
    let bridge = bridges
        .get(&session_id)
        .ok_or_else(|| super::ai_session_not_initialized_error(&session_id))?;
    Ok(bridge.conversation_history_len().await)
}
