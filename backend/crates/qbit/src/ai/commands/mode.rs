//! Agent mode commands for controlling tool approval behavior.
//!
//! These commands allow the frontend to get and set the agent mode for
//! a specific session, controlling how tool approvals are handled.

use qbit_settings::ProjectSettingsManager;
use std::path::PathBuf;
use tauri::State;

use crate::ai::agent_mode::AgentMode;
use crate::state::AppState;

use super::ai_session_not_initialized_error;

/// Set the agent mode for a session.
///
/// # Arguments
/// * `session_id` - The session ID to set the mode for
/// * `mode` - The agent mode ("default", "auto-approve", or "planning")
/// * `workspace` - Optional workspace path to persist the mode to project settings
#[tauri::command]
pub async fn set_agent_mode(
    session_id: String,
    mode: AgentMode,
    workspace: Option<PathBuf>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let bridges = state.ai_state.bridges.read().await;
    let bridge = bridges
        .get(&session_id)
        .ok_or_else(|| ai_session_not_initialized_error(&session_id))?;

    bridge.set_agent_mode(mode).await;

    // If workspace is provided, also persist to project settings
    if let Some(workspace_path) = workspace {
        let project_settings = ProjectSettingsManager::new(&workspace_path).await;
        project_settings
            .set_agent_mode(mode.to_string())
            .await
            .map_err(|e| e.to_string())?;
    }

    Ok(())
}

/// Save the agent mode to project settings explicitly.
///
/// # Arguments
/// * `workspace` - The workspace path to save settings to
/// * `mode` - The agent mode to save
#[tauri::command]
pub async fn save_project_agent_mode(workspace: PathBuf, mode: AgentMode) -> Result<(), String> {
    let project_settings = ProjectSettingsManager::new(&workspace).await;
    project_settings
        .set_agent_mode(mode.to_string())
        .await
        .map_err(|e| e.to_string())
}

/// Get the current agent mode for a session.
///
/// # Arguments
/// * `session_id` - The session ID to get the mode for
///
/// # Returns
/// The current agent mode ("default", "auto-approve", or "planning")
#[tauri::command]
pub async fn get_agent_mode(
    session_id: String,
    state: State<'_, AppState>,
) -> Result<AgentMode, String> {
    let bridges = state.ai_state.bridges.read().await;
    let bridge = bridges
        .get(&session_id)
        .ok_or_else(|| ai_session_not_initialized_error(&session_id))?;

    Ok(bridge.get_agent_mode().await)
}
