// Loop detection and protection commands.

use tauri::State;

use super::super::loop_detection::{LoopDetectorStats, LoopProtectionConfig};
use crate::state::AppState;

/// Get the current loop protection configuration.
#[tauri::command]
pub async fn get_loop_protection_config(
    state: State<'_, AppState>,
) -> Result<LoopProtectionConfig, String> {
    let bridge_guard = state.ai_state.get_bridge().await?;
    let bridge = bridge_guard.as_ref().unwrap();
    Ok(bridge.get_loop_protection_config().await)
}

/// Set the loop protection configuration.
#[tauri::command]
pub async fn set_loop_protection_config(
    state: State<'_, AppState>,
    config: LoopProtectionConfig,
) -> Result<(), String> {
    let bridge_guard = state.ai_state.get_bridge().await?;
    let bridge = bridge_guard.as_ref().unwrap();
    bridge.set_loop_protection_config(config).await;
    Ok(())
}

/// Get current loop detector statistics.
#[tauri::command]
pub async fn get_loop_detector_stats(
    state: State<'_, AppState>,
) -> Result<LoopDetectorStats, String> {
    let bridge_guard = state.ai_state.get_bridge().await?;
    let bridge = bridge_guard.as_ref().unwrap();
    Ok(bridge.get_loop_detector_stats().await)
}

/// Check if loop detection is currently enabled.
#[tauri::command]
pub async fn is_loop_detection_enabled(state: State<'_, AppState>) -> Result<bool, String> {
    let bridge_guard = state.ai_state.get_bridge().await?;
    let bridge = bridge_guard.as_ref().unwrap();
    Ok(bridge.is_loop_detection_enabled().await)
}

/// Disable loop detection for the current session.
/// This allows the agent to continue even if loops are detected.
#[tauri::command]
pub async fn disable_loop_detection(state: State<'_, AppState>) -> Result<(), String> {
    let bridge_guard = state.ai_state.get_bridge().await?;
    let bridge = bridge_guard.as_ref().unwrap();
    bridge.disable_loop_detection_for_session().await;
    Ok(())
}

/// Re-enable loop detection.
#[tauri::command]
pub async fn enable_loop_detection(state: State<'_, AppState>) -> Result<(), String> {
    let bridge_guard = state.ai_state.get_bridge().await?;
    let bridge = bridge_guard.as_ref().unwrap();
    bridge.enable_loop_detection().await;
    Ok(())
}

/// Reset the loop detector (clears all tracking).
#[tauri::command]
pub async fn reset_loop_detector(state: State<'_, AppState>) -> Result<(), String> {
    let bridge_guard = state.ai_state.get_bridge().await?;
    let bridge = bridge_guard.as_ref().unwrap();
    bridge.reset_loop_detector().await;
    Ok(())
}
