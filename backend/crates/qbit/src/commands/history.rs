use crate::error::Result;
use crate::history::{HistoryEntry, HistoryManager};
use crate::state::AppState;
use std::sync::Arc;
use tauri::State;
use tokio::sync::RwLock;

#[tauri::command]
pub async fn add_command_history(
    _state: State<'_, AppState>,
    history: State<'_, Arc<RwLock<Option<HistoryManager>>>>,
    session_id: String,
    command: String,
    exit_code: i32,
) -> Result<()> {
    if let Some(history) = history.read().await.as_ref() {
        history
            .add_command(session_id, command, exit_code)
            .map_err(|e| crate::error::QbitError::Internal(e.to_string()))?;
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
#[tauri::command]
pub async fn add_prompt_history(
    _state: State<'_, AppState>,
    history: State<'_, Arc<RwLock<Option<HistoryManager>>>>,
    session_id: String,
    prompt: String,
    model: String,
    provider: String,
    tokens_in: u32,
    tokens_out: u32,
    success: bool,
) -> Result<()> {
    if let Some(history) = history.read().await.as_ref() {
        history
            .add_prompt(
                session_id, prompt, model, provider, tokens_in, tokens_out, success,
            )
            .map_err(|e| crate::error::QbitError::Internal(e.to_string()))?;
    }
    Ok(())
}

#[tauri::command]
pub async fn load_history(
    _state: State<'_, AppState>,
    history: State<'_, Arc<RwLock<Option<HistoryManager>>>>,
    limit: usize,
    entry_type: Option<String>,
) -> std::result::Result<Vec<HistoryEntry>, String> {
    let Some(ref history) = *history.read().await else {
        return Ok(vec![]);
    };
    let et = entry_type.as_deref();
    history.load_recent(limit, et).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn search_history(
    _state: State<'_, AppState>,
    history: State<'_, Arc<RwLock<Option<HistoryManager>>>>,
    query: String,
    include_archives: bool,
    limit: usize,
    entry_type: Option<String>,
) -> std::result::Result<Vec<HistoryEntry>, String> {
    let Some(ref history) = *history.read().await else {
        return Ok(vec![]);
    };
    let et = entry_type.as_deref();
    history
        .search(query, include_archives, limit, et)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn clear_history(
    _state: State<'_, AppState>,
    history: State<'_, Arc<RwLock<Option<HistoryManager>>>>,
) -> std::result::Result<(), String> {
    let Some(ref history) = *history.read().await else {
        return Ok(());
    };
    history.clear_all().map_err(|e| e.to_string())
}
