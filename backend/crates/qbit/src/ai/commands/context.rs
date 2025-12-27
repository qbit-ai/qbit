// Context and token management commands.

use tauri::State;

use crate::state::AppState;
use qbit_context::token_budget::{TokenAlertLevel, TokenUsageStats};
use qbit_context::{ContextSummary, ContextTrimConfig};

/// Get the current context summary including token usage and alert level.
#[tauri::command]
pub async fn get_context_summary(state: State<'_, AppState>) -> Result<ContextSummary, String> {
    let bridge_guard = state.ai_state.get_bridge().await?;
    let bridge = bridge_guard.as_ref().unwrap();
    Ok(bridge.get_context_summary().await)
}

/// Get detailed token usage statistics.
#[tauri::command]
pub async fn get_token_usage_stats(state: State<'_, AppState>) -> Result<TokenUsageStats, String> {
    let bridge_guard = state.ai_state.get_bridge().await?;
    let bridge = bridge_guard.as_ref().unwrap();
    Ok(bridge.get_token_usage_stats().await)
}

/// Get the current token alert level.
#[tauri::command]
pub async fn get_token_alert_level(state: State<'_, AppState>) -> Result<TokenAlertLevel, String> {
    let bridge_guard = state.ai_state.get_bridge().await?;
    let bridge = bridge_guard.as_ref().unwrap();
    Ok(bridge.get_token_alert_level().await)
}

/// Get the context utilization percentage (0.0 - 1.0+).
#[tauri::command]
pub async fn get_context_utilization(state: State<'_, AppState>) -> Result<f64, String> {
    let bridge_guard = state.ai_state.get_bridge().await?;
    let bridge = bridge_guard.as_ref().unwrap();
    Ok(bridge.get_context_utilization().await)
}

/// Get remaining available tokens in the context window.
#[tauri::command]
pub async fn get_remaining_tokens(state: State<'_, AppState>) -> Result<usize, String> {
    let bridge_guard = state.ai_state.get_bridge().await?;
    let bridge = bridge_guard.as_ref().unwrap();
    Ok(bridge.get_remaining_tokens().await)
}

/// Manually enforce context window limits by pruning old messages.
/// Returns the number of messages that were pruned.
#[tauri::command]
pub async fn enforce_context_window(state: State<'_, AppState>) -> Result<usize, String> {
    let bridge_guard = state.ai_state.get_bridge().await?;
    let bridge = bridge_guard.as_ref().unwrap();
    Ok(bridge.enforce_context_window().await)
}

/// Reset the context manager (clear all token tracking).
/// This does not clear the conversation history, only the token stats.
#[tauri::command]
pub async fn reset_context_manager(state: State<'_, AppState>) -> Result<(), String> {
    let bridge_guard = state.ai_state.get_bridge().await?;
    let bridge = bridge_guard.as_ref().unwrap();
    bridge.reset_context_manager().await;
    Ok(())
}

/// Get the context trim configuration.
#[tauri::command]
pub async fn get_context_trim_config(
    state: State<'_, AppState>,
) -> Result<ContextTrimConfig, String> {
    state
        .ai_state
        .with_bridge(|b| b.get_context_trim_config())
        .await
}

/// Check if context management is enabled.
#[tauri::command]
pub async fn is_context_management_enabled(state: State<'_, AppState>) -> Result<bool, String> {
    state
        .ai_state
        .with_bridge(|b| b.is_context_management_enabled())
        .await
}
