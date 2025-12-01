// Tool policy management commands.

use tauri::State;

use crate::state::AppState;
use super::super::tool_policy::{ToolPolicy, ToolPolicyConfig};

/// Get the current tool policy configuration.
#[tauri::command]
pub async fn get_tool_policy_config(
    state: State<'_, AppState>,
) -> Result<ToolPolicyConfig, String> {
    let bridge_guard = state.ai_state.get_bridge().await?;
    let bridge = bridge_guard.as_ref().unwrap();

    let config = bridge.get_tool_policy_config().await;
    Ok(config)
}

/// Update the tool policy configuration.
#[tauri::command]
pub async fn set_tool_policy_config(
    state: State<'_, AppState>,
    config: ToolPolicyConfig,
) -> Result<(), String> {
    let bridge_guard = state.ai_state.get_bridge().await?;
    let bridge = bridge_guard.as_ref().unwrap();

    bridge
        .set_tool_policy_config(config)
        .await
        .map_err(|e| e.to_string())
}

/// Get the policy for a specific tool.
#[tauri::command]
pub async fn get_tool_policy(
    state: State<'_, AppState>,
    tool_name: String,
) -> Result<ToolPolicy, String> {
    let bridge_guard = state.ai_state.get_bridge().await?;
    let bridge = bridge_guard.as_ref().unwrap();

    let policy = bridge.get_tool_policy(&tool_name).await;
    Ok(policy)
}

/// Set the policy for a specific tool.
#[tauri::command]
pub async fn set_tool_policy(
    state: State<'_, AppState>,
    tool_name: String,
    policy: ToolPolicy,
) -> Result<(), String> {
    let bridge_guard = state.ai_state.get_bridge().await?;
    let bridge = bridge_guard.as_ref().unwrap();

    bridge
        .set_tool_policy(&tool_name, policy)
        .await
        .map_err(|e| e.to_string())
}

/// Reset tool policies to defaults.
#[tauri::command]
pub async fn reset_tool_policies(state: State<'_, AppState>) -> Result<(), String> {
    let bridge_guard = state.ai_state.get_bridge().await?;
    let bridge = bridge_guard.as_ref().unwrap();

    bridge
        .reset_tool_policies()
        .await
        .map_err(|e| e.to_string())
}

/// Enable full-auto mode for tool execution.
///
/// In full-auto mode, tools in the allowed list execute without any approval.
#[tauri::command]
pub async fn enable_full_auto_mode(
    state: State<'_, AppState>,
    allowed_tools: Vec<String>,
) -> Result<(), String> {
    let bridge_guard = state.ai_state.get_bridge().await?;
    let bridge = bridge_guard.as_ref().unwrap();

    bridge.enable_full_auto_mode(allowed_tools).await;
    Ok(())
}

/// Disable full-auto mode for tool execution.
#[tauri::command]
pub async fn disable_full_auto_mode(state: State<'_, AppState>) -> Result<(), String> {
    let bridge_guard = state.ai_state.get_bridge().await?;
    let bridge = bridge_guard.as_ref().unwrap();

    bridge.disable_full_auto_mode().await;
    Ok(())
}

/// Check if full-auto mode is enabled.
#[tauri::command]
pub async fn is_full_auto_mode_enabled(state: State<'_, AppState>) -> Result<bool, String> {
    let bridge_guard = state.ai_state.get_bridge().await?;
    let bridge = bridge_guard.as_ref().unwrap();

    Ok(bridge.is_full_auto_mode_enabled().await)
}
