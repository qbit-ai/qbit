// Configuration commands for AI agent setup and workspace management.

use std::sync::Arc;
use tauri::State;

use super::super::agent_bridge::AgentBridge;
use super::configure_bridge;
use crate::runtime::{QbitRuntime, TauriRuntime};
use crate::settings::get_with_env_fallback;
use crate::state::AppState;

/// Get the OpenRouter API key from settings with environment variable fallback.
/// Priority: settings.ai.openrouter.api_key > $OPENROUTER_API_KEY
#[tauri::command]
pub async fn get_openrouter_api_key(state: State<'_, AppState>) -> Result<Option<String>, String> {
    let settings = state.settings_manager.get().await;
    Ok(get_with_env_fallback(
        &settings.ai.openrouter.api_key,
        &["OPENROUTER_API_KEY"],
        None,
    ))
}

/// Get the OpenAI API key from settings with environment variable fallback.
/// Priority: settings.ai.openai.api_key > $OPENAI_API_KEY
#[tauri::command]
pub async fn get_openai_api_key(state: State<'_, AppState>) -> Result<Option<String>, String> {
    let settings = state.settings_manager.get().await;
    Ok(get_with_env_fallback(
        &settings.ai.openai.api_key,
        &["OPENAI_API_KEY"],
        None,
    ))
}

/// Initialize the AI agent with OpenAI.
///
/// If an existing AI agent is running, its session will be finalized and the
/// sidecar session will be ended before the new agent is initialized.
///
/// # Arguments
/// * `workspace` - Path to the workspace directory
/// * `model` - Model identifier (e.g., "gpt-5.2")
/// * `api_key` - OpenAI API key
/// * `base_url` - Optional custom base URL for OpenAI-compatible APIs
/// * `reasoning_effort` - Optional reasoning effort level ("low", "medium", "high")
#[tauri::command]
pub async fn init_ai_agent_openai(
    state: State<'_, AppState>,
    app: tauri::AppHandle,
    workspace: String,
    model: String,
    api_key: String,
    base_url: Option<String>,
    reasoning_effort: Option<String>,
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

    let workspace_path: std::path::PathBuf = workspace.into();

    // Create bridge with OpenAI
    let mut bridge = AgentBridge::new_openai_with_runtime(
        workspace_path.clone(),
        &model,
        &api_key,
        base_url.as_deref(),
        reasoning_effort.as_deref(),
        runtime,
    )
    .await
    .map_err(|e| e.to_string())?;

    configure_bridge(&mut bridge, &state).await;

    // Replace the bridge
    *state.ai_state.bridge.write().await = Some(bridge);

    // Initialize sidecar with the workspace
    if let Err(e) = state.sidecar_state.initialize(workspace_path).await {
        tracing::warn!("Failed to initialize sidecar: {}", e);
    } else {
        tracing::info!("Sidecar initialized for workspace");
    }

    tracing::info!(
        "AI agent initialized with OpenAI, model: {}, reasoning_effort: {:?}",
        model,
        reasoning_effort
    );
    Ok(())
}

/// Initialize the AI agent with Anthropic on Google Cloud Vertex AI.
///
/// If an existing AI agent is running, its session will be finalized and the
/// sidecar session will be ended before the new agent is initialized.
///
/// # Arguments
/// * `workspace` - Path to the workspace directory
/// * `credentials_path` - Path to the service account JSON file
/// * `project_id` - Google Cloud project ID
/// * `location` - Vertex AI location (e.g., "us-east5")
/// * `model` - Model identifier (e.g., "claude-opus-4-5@20251101")
#[tauri::command]
pub async fn init_ai_agent_vertex(
    state: State<'_, AppState>,
    app: tauri::AppHandle,
    workspace: String,
    credentials_path: String,
    project_id: String,
    location: String,
    model: String,
) -> Result<(), String> {
    // Clean up existing session before replacing the bridge
    // This ensures sessions are properly finalized when switching models
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

    let workspace_path: std::path::PathBuf = workspace.into();

    // Create bridge with runtime (Phase 5 - new path)
    let mut bridge = AgentBridge::new_vertex_anthropic_with_runtime(
        workspace_path.clone(),
        &credentials_path,
        &project_id,
        &location,
        &model,
        runtime,
    )
    .await
    .map_err(|e| e.to_string())?;

    configure_bridge(&mut bridge, &state).await;

    // Replace the bridge (old bridge's Drop impl will finalize its session)
    *state.ai_state.bridge.write().await = Some(bridge);

    // Initialize sidecar with the workspace
    if let Err(e) = state.sidecar_state.initialize(workspace_path).await {
        tracing::warn!("Failed to initialize sidecar: {}", e);
        // Don't fail the whole init - sidecar is optional
    } else {
        tracing::info!("Sidecar initialized for workspace");
    }

    tracing::info!(
        "AI agent initialized with Vertex AI Anthropic, project: {}, model: {}",
        project_id,
        model
    );
    Ok(())
}

/// Update the AI agent's workspace/working directory.
/// This allows the agent to stay in sync with the user's terminal directory.
///
/// # Arguments
/// * `workspace` - New workspace/working directory path
/// * `session_id` - Optional session ID to update the session-specific bridge
#[tauri::command]
pub async fn update_ai_workspace(
    state: State<'_, AppState>,
    workspace: String,
    session_id: Option<String>,
) -> Result<(), String> {
    tracing::debug!(
        "[cwd-sync] update_ai_workspace called with: {}, session_id: {:?}",
        workspace,
        session_id
    );

    let workspace_path: std::path::PathBuf = workspace.into();

    // Update session-specific bridge if session_id is provided
    if let Some(ref sid) = session_id {
        let bridges = state.ai_state.get_bridges().await;
        if let Some(bridge) = bridges.get(sid) {
            bridge.set_workspace(workspace_path.clone()).await;
            // Note: set_workspace logs at trace if unchanged, so we don't duplicate here
        } else {
            tracing::warn!("[cwd-sync] No session bridge found for session_id: {}", sid);
        }
    }

    // Also update legacy bridge if initialized (for backwards compatibility)
    if let Ok(bridge_guard) = state.ai_state.get_bridge().await {
        let bridge = bridge_guard.as_ref().unwrap();
        bridge.set_workspace(workspace_path.clone()).await;
    }

    // Re-initialize sidecar if workspace changed
    let status = state.sidecar_state.status();
    if status.enabled && status.workspace_path.as_ref() != Some(&workspace_path) {
        if let Err(e) = state.sidecar_state.initialize(workspace_path).await {
            tracing::warn!("[cwd-sync] Failed to initialize sidecar: {}", e);
        } else {
            tracing::debug!("[cwd-sync] Sidecar re-initialized for new workspace");
        }
    }

    Ok(())
}

/// Load environment variables from a .env file.
/// Returns the number of variables loaded.
#[tauri::command]
pub fn load_env_file(path: String) -> Result<usize, String> {
    match dotenvy::from_path(&path) {
        Ok(_) => {
            // Count how many vars we can read
            let count = dotenvy::from_path_iter(&path)
                .map(|iter| iter.count())
                .unwrap_or(0);
            tracing::info!("Loaded {} environment variables from {}", count, path);
            Ok(count)
        }
        Err(e) => Err(format!("Failed to load .env file: {}", e)),
    }
}

/// Vertex AI configuration from settings with environment variable fallback.
#[derive(Debug, Clone, serde::Serialize)]
pub struct VertexAiEnvConfig {
    pub credentials_path: Option<String>,
    pub project_id: Option<String>,
    pub location: Option<String>,
}

/// Get Vertex AI configuration from settings with environment variable fallback.
///
/// Priority for each field:
/// - credentials_path: settings > $VERTEX_AI_CREDENTIALS_PATH > $GOOGLE_APPLICATION_CREDENTIALS
/// - project_id: settings > $VERTEX_AI_PROJECT_ID > $GOOGLE_CLOUD_PROJECT
/// - location: settings > $VERTEX_AI_LOCATION > "us-east5"
#[tauri::command]
pub async fn get_vertex_ai_config(state: State<'_, AppState>) -> Result<VertexAiEnvConfig, String> {
    let settings = state.settings_manager.get().await;

    let credentials_path = get_with_env_fallback(
        &settings.ai.vertex_ai.credentials_path,
        &[
            "VERTEX_AI_CREDENTIALS_PATH",
            "GOOGLE_APPLICATION_CREDENTIALS",
        ],
        None,
    );

    let project_id = get_with_env_fallback(
        &settings.ai.vertex_ai.project_id,
        &["VERTEX_AI_PROJECT_ID", "GOOGLE_CLOUD_PROJECT"],
        None,
    );

    let location = get_with_env_fallback(
        &settings.ai.vertex_ai.location,
        &["VERTEX_AI_LOCATION"],
        Some("us-east5".to_string()),
    );

    Ok(VertexAiEnvConfig {
        credentials_path,
        project_id,
        location,
    })
}
