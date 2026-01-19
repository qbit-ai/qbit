// HITL (Human-in-the-Loop) approval commands.

use tauri::State;

use super::ai_session_not_initialized_error;
use crate::state::AppState;
use qbit_core::hitl::ApprovalDecision;
use qbit_hitl::{ApprovalPattern, ToolApprovalConfig};

/// Get approval patterns for all tools.
#[tauri::command]
pub async fn get_approval_patterns(
    state: State<'_, AppState>,
) -> Result<Vec<ApprovalPattern>, String> {
    let bridge_guard = state.ai_state.get_bridge().await?;
    let bridge = bridge_guard.as_ref().unwrap();

    let patterns = bridge.get_approval_patterns().await;
    Ok(patterns)
}

/// Get the approval pattern for a specific tool.
#[tauri::command]
pub async fn get_tool_approval_pattern(
    state: State<'_, AppState>,
    tool_name: String,
) -> Result<Option<ApprovalPattern>, String> {
    let bridge_guard = state.ai_state.get_bridge().await?;
    let bridge = bridge_guard.as_ref().unwrap();

    let pattern = bridge.get_tool_approval_pattern(&tool_name).await;
    Ok(pattern)
}

/// Get the HITL configuration.
#[tauri::command]
pub async fn get_hitl_config(state: State<'_, AppState>) -> Result<ToolApprovalConfig, String> {
    let bridge_guard = state.ai_state.get_bridge().await?;
    let bridge = bridge_guard.as_ref().unwrap();

    let config = bridge.get_hitl_config().await;
    Ok(config)
}

/// Update the HITL configuration.
#[tauri::command]
pub async fn set_hitl_config(
    state: State<'_, AppState>,
    config: ToolApprovalConfig,
) -> Result<(), String> {
    let bridge_guard = state.ai_state.get_bridge().await?;
    let bridge = bridge_guard.as_ref().unwrap();

    bridge
        .set_hitl_config(config)
        .await
        .map_err(|e| e.to_string())
}

/// Add a tool to the always-allow list.
#[tauri::command]
pub async fn add_tool_always_allow(
    state: State<'_, AppState>,
    tool_name: String,
) -> Result<(), String> {
    let bridge_guard = state.ai_state.get_bridge().await?;
    let bridge = bridge_guard.as_ref().unwrap();

    bridge
        .add_tool_always_allow(&tool_name)
        .await
        .map_err(|e| e.to_string())
}

/// Remove a tool from the always-allow list.
#[tauri::command]
pub async fn remove_tool_always_allow(
    state: State<'_, AppState>,
    tool_name: String,
) -> Result<(), String> {
    let bridge_guard = state.ai_state.get_bridge().await?;
    let bridge = bridge_guard.as_ref().unwrap();

    bridge
        .remove_tool_always_allow(&tool_name)
        .await
        .map_err(|e| e.to_string())
}

/// Reset all approval patterns (does not reset configuration).
#[tauri::command]
pub async fn reset_approval_patterns(state: State<'_, AppState>) -> Result<(), String> {
    let bridge_guard = state.ai_state.get_bridge().await?;
    let bridge = bridge_guard.as_ref().unwrap();

    bridge
        .reset_approval_patterns()
        .await
        .map_err(|e| e.to_string())
}

/// Respond to a tool approval request.
///
/// This is called by the frontend after the user makes a decision in the approval dialog.
///
/// # Arguments
/// * `session_id` - The session ID where the approval request originated
/// * `decision` - The user's approval decision
///
/// IMPORTANT: Uses get_session_bridge() to clone the Arc and release the map
/// lock immediately, avoiding deadlocks when other tasks need write access.
#[tauri::command]
pub async fn respond_to_tool_approval(
    state: State<'_, AppState>,
    session_id: String,
    decision: ApprovalDecision,
) -> Result<(), String> {
    let bridge = state
        .ai_state
        .get_session_bridge(&session_id)
        .await
        .ok_or_else(|| ai_session_not_initialized_error(&session_id))?;

    bridge
        .respond_to_approval(decision)
        .await
        .map_err(|e| e.to_string())
}
