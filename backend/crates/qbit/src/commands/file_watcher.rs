//! File watcher for the text editor sidebar.
//! Watches open files for external changes and emits events to the frontend.

use crate::error::Result;
use chrono::{DateTime, Utc};
use notify::RecursiveMode;
use notify_debouncer_mini::{new_debouncer, DebouncedEventKind};
use parking_lot::Mutex;
use serde::Serialize;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter};

/// Event payload sent to the frontend when a watched file changes.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileChangedEvent {
    pub path: String,
    pub modified_at: Option<String>,
}

/// Managed state for the file watcher system.
pub struct FileWatcherState {
    inner: Mutex<FileWatcherInner>,
}

struct FileWatcherInner {
    /// One debouncer instance that watches all files
    debouncer: Option<notify_debouncer_mini::Debouncer<notify::RecommendedWatcher>>,
    /// Set of currently watched paths
    watched_paths: HashMap<PathBuf, ()>,
    /// App handle for emitting events
    app_handle: Option<AppHandle>,
}

impl FileWatcherState {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(FileWatcherInner {
                debouncer: None,
                watched_paths: HashMap::new(),
                app_handle: None,
            }),
        }
    }
}

fn resolve_path(path: &str) -> PathBuf {
    let target = PathBuf::from(path);
    if target.is_absolute() {
        target
    } else {
        std::env::var("QBIT_WORKSPACE")
            .map(PathBuf::from)
            .unwrap_or_else(|_| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
            .join(target)
    }
}

fn format_modified_time(path: &PathBuf) -> Option<String> {
    std::fs::metadata(path)
        .ok()
        .and_then(|m| m.modified().ok())
        .map(|t| DateTime::<Utc>::from(t).to_rfc3339())
}

fn ensure_debouncer(inner: &mut FileWatcherInner) {
    if inner.debouncer.is_some() {
        return;
    }

    let Some(app_handle) = inner.app_handle.clone() else {
        tracing::warn!("[file-watcher] No app handle set, cannot create debouncer");
        return;
    };

    match new_debouncer(Duration::from_millis(300), move |res: std::result::Result<Vec<notify_debouncer_mini::DebouncedEvent>, notify::Error>| {
        match res {
            Ok(events) => {
                for event in events {
                    if event.kind == DebouncedEventKind::Any {
                        let path = event.path.clone();
                        let modified_at = format_modified_time(&path);
                        let payload = FileChangedEvent {
                            path: path.to_string_lossy().to_string(),
                            modified_at,
                        };
                        tracing::debug!("[file-watcher] File changed: {}", payload.path);
                        if let Err(e) = app_handle.emit("file-changed", &payload) {
                            tracing::error!("[file-watcher] Failed to emit file-changed event: {}", e);
                        }
                    }
                }
            }
            Err(e) => {
                tracing::error!("[file-watcher] Watch error: {}", e);
            }
        }
    }) {
        Ok(debouncer) => {
            inner.debouncer = Some(debouncer);
        }
        Err(e) => {
            tracing::error!("[file-watcher] Failed to create debouncer: {}", e);
        }
    }
}

/// Start watching a file for external changes.
#[tauri::command]
pub async fn watch_file(
    path: String,
    state: tauri::State<'_, Arc<FileWatcherState>>,
    app: AppHandle,
) -> Result<()> {
    let resolved = resolve_path(&path);

    if !resolved.exists() {
        tracing::warn!("[file-watcher] File does not exist: {}", resolved.display());
        return Ok(());
    }

    let mut inner = state.inner.lock();

    // Set app handle if not already set
    if inner.app_handle.is_none() {
        inner.app_handle = Some(app);
    }

    // Already watching this path
    if inner.watched_paths.contains_key(&resolved) {
        return Ok(());
    }

    // Ensure we have a debouncer
    ensure_debouncer(&mut inner);

    if let Some(ref mut debouncer) = inner.debouncer {
        match debouncer.watcher().watch(&resolved, RecursiveMode::NonRecursive) {
            Ok(()) => {
                inner.watched_paths.insert(resolved.clone(), ());
                tracing::debug!("[file-watcher] Watching: {}", resolved.display());
            }
            Err(e) => {
                tracing::error!("[file-watcher] Failed to watch {}: {}", resolved.display(), e);
            }
        }
    }

    Ok(())
}

/// Stop watching a file.
#[tauri::command]
pub async fn unwatch_file(
    path: String,
    state: tauri::State<'_, Arc<FileWatcherState>>,
) -> Result<()> {
    let resolved = resolve_path(&path);
    let mut inner = state.inner.lock();

    if inner.watched_paths.remove(&resolved).is_some() {
        if let Some(ref mut debouncer) = inner.debouncer {
            if let Err(e) = debouncer.watcher().unwatch(&resolved) {
                tracing::warn!("[file-watcher] Failed to unwatch {}: {}", resolved.display(), e);
            }
        }
        tracing::debug!("[file-watcher] Unwatched: {}", resolved.display());
    }

    // If no more watched paths, drop the debouncer to free resources
    if inner.watched_paths.is_empty() {
        inner.debouncer = None;
        tracing::debug!("[file-watcher] No more watched files, dropped debouncer");
    }

    Ok(())
}

/// Stop watching all files.
#[tauri::command]
pub async fn unwatch_all_files(
    state: tauri::State<'_, Arc<FileWatcherState>>,
) -> Result<()> {
    let mut inner = state.inner.lock();

    inner.watched_paths.clear();
    inner.debouncer = None;
    tracing::debug!("[file-watcher] Unwatched all files");

    Ok(())
}