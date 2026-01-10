//! Tauri commands for settings management.
//!
//! These commands expose the settings system to the frontend, allowing
//! the UI to read and update configuration.

use tauri::State;

use crate::state::AppState;
use qbit_settings::QbitSettings;

/// Get all settings.
#[tauri::command]
pub async fn get_settings(state: State<'_, AppState>) -> Result<QbitSettings, String> {
    Ok(state.settings_manager.get().await)
}

/// Update all settings.
#[tauri::command]
pub async fn update_settings(
    state: State<'_, AppState>,
    settings: QbitSettings,
) -> Result<(), String> {
    state
        .settings_manager
        .update(settings)
        .await
        .map_err(|e| e.to_string())
}

/// Get a specific setting by key (dot notation: "ai.vertex_ai.project_id").
#[tauri::command]
pub async fn get_setting(
    state: State<'_, AppState>,
    key: String,
) -> Result<serde_json::Value, String> {
    state
        .settings_manager
        .get_value(&key)
        .await
        .map_err(|e| e.to_string())
}

/// Set a specific setting by key.
#[tauri::command]
pub async fn set_setting(
    state: State<'_, AppState>,
    key: String,
    value: serde_json::Value,
) -> Result<(), String> {
    state
        .settings_manager
        .set_value(&key, value)
        .await
        .map_err(|e| e.to_string())
}

/// Reset all settings to defaults.
#[tauri::command]
pub async fn reset_settings(state: State<'_, AppState>) -> Result<(), String> {
    state
        .settings_manager
        .reset()
        .await
        .map_err(|e| e.to_string())
}

/// Check if settings file exists.
#[tauri::command]
pub fn settings_file_exists(state: State<'_, AppState>) -> bool {
    state.settings_manager.exists()
}

/// Get the path to the settings file.
#[tauri::command]
pub fn get_settings_path(state: State<'_, AppState>) -> String {
    state.settings_manager.path().display().to_string()
}

/// Reload settings from disk.
#[tauri::command]
pub async fn reload_settings(state: State<'_, AppState>) -> Result<(), String> {
    state
        .settings_manager
        .reload()
        .await
        .map_err(|e| e.to_string())
}

/// Save window state (size, position, maximized).
///
/// Called when the window is resized, moved, or closed to persist state.
#[tauri::command]
pub async fn save_window_state(
    state: State<'_, AppState>,
    width: u32,
    height: u32,
    x: Option<i32>,
    y: Option<i32>,
    maximized: bool,
) -> Result<(), String> {
    let mut settings = state.settings_manager.get().await;
    settings.ui.window.width = width;
    settings.ui.window.height = height;
    settings.ui.window.x = x;
    settings.ui.window.y = y;
    settings.ui.window.maximized = maximized;

    state
        .settings_manager
        .update(settings)
        .await
        .map_err(|e| e.to_string())
}

/// Get window state from settings.
///
/// Used on startup to restore window size and position.
#[tauri::command]
pub async fn get_window_state(
    state: State<'_, AppState>,
) -> Result<qbit_settings::WindowSettings, String> {
    let settings = state.settings_manager.get().await;
    Ok(settings.ui.window)
}

/// Check if Langfuse tracing is active.
///
/// Returns true if Langfuse was enabled in settings and properly configured
/// (i.e., valid API keys were available) at startup.
#[tauri::command]
pub fn is_langfuse_active(state: State<'_, AppState>) -> bool {
    state.langfuse_active
}
