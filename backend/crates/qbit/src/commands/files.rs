//! File listing and workspace file commands

use crate::error::Result;
use chrono::{DateTime, Utc};
use ignore::WalkBuilder;
use serde::{Deserialize, Serialize};
use std::io;
use std::path::PathBuf;
use tokio::fs;

/// Information about a file for the @ command popup
#[derive(Debug, Clone, Serialize)]
pub struct FileInfo {
    /// File name (e.g., "Button.tsx")
    pub name: String,
    /// Relative path from workspace root (e.g., "src/components/Button.tsx")
    pub relative_path: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileReadResult {
    pub content: String,
    pub modified_at: Option<String>,
    pub encoding: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct FileWriteOptions {
    pub encoding: Option<String>,
    pub expected_modified_at: Option<String>,
    pub create_if_missing: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileWriteResult {
    pub modified_at: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileStatResult {
    pub modified_at: String,
    pub size: u64,
}

fn workspace_root() -> PathBuf {
    std::env::var("QBIT_WORKSPACE")
        .map(PathBuf::from)
        .unwrap_or_else(|_| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
}

fn resolve_workspace_path(path: &str) -> PathBuf {
    let target = PathBuf::from(path);
    if target.is_absolute() {
        target
    } else {
        workspace_root().join(target)
    }
}

fn format_modified_time(metadata: &std::fs::Metadata) -> Option<String> {
    metadata
        .modified()
        .ok()
        .map(|t| DateTime::<Utc>::from(t).to_rfc3339())
}

/// List files in the workspace, respecting .gitignore.
/// Returns files matching the optional query, limited to `limit` results.
#[tauri::command]
pub async fn list_workspace_files(
    working_directory: String,
    query: Option<String>,
    limit: Option<usize>,
) -> Result<Vec<FileInfo>> {
    let workspace = PathBuf::from(&working_directory);
    let limit = limit.unwrap_or(5);
    let query = query.unwrap_or_default().to_lowercase();

    if !workspace.exists() {
        return Ok(Vec::new());
    }

    let mut files: Vec<FileInfo> = WalkBuilder::new(&workspace)
        .hidden(true) // Skip hidden files/dirs (but respect .gitignore)
        .git_ignore(true) // Respect .gitignore
        .git_global(true) // Respect global gitignore
        .git_exclude(true) // Respect .git/info/exclude
        .build()
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            // Only include files, not directories
            entry.file_type().map(|ft| ft.is_file()).unwrap_or(false)
        })
        .filter_map(|entry| {
            let path = entry.path();
            let relative = path.strip_prefix(&workspace).ok()?;
            let relative_str = relative.to_string_lossy().to_string();
            let name = path.file_name()?.to_string_lossy().to_string();

            // Filter by query if provided
            if !query.is_empty() {
                let name_lower = name.to_lowercase();
                let path_lower = relative_str.to_lowercase();
                if !name_lower.contains(&query) && !path_lower.contains(&query) {
                    return None;
                }
            }

            Some(FileInfo {
                name,
                relative_path: relative_str,
            })
        })
        .take(limit)
        .collect();

    // Sort by name for consistent ordering
    files.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

    Ok(files)
}

/// Read a workspace file as UTF-8 (lossy) and return its content and metadata.
#[tauri::command]
pub async fn read_workspace_file(path: String) -> Result<FileReadResult> {
    let path = resolve_workspace_path(&path);
    let metadata = fs::metadata(&path).await?;

    if !metadata.is_file() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("Not a file: {}", path.display()),
        )
        .into());
    }

    let bytes = fs::read(&path).await?;
    let content = String::from_utf8_lossy(&bytes).to_string();

    Ok(FileReadResult {
        content,
        modified_at: format_modified_time(&metadata),
        encoding: Some("utf-8".to_string()),
    })
}

/// Write content to a workspace file, optionally guarding on last modified time.
#[tauri::command]
pub async fn write_workspace_file(
    path: String,
    content: String,
    options: Option<FileWriteOptions>,
) -> Result<FileWriteResult> {
    let path = resolve_workspace_path(&path);
    let opts = options.unwrap_or_default();
    let _ = opts.encoding.as_deref();

    let metadata = fs::metadata(&path).await.ok();

    if metadata.is_none() && !opts.create_if_missing.unwrap_or(false) {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("File not found: {}", path.display()),
        )
        .into());
    }

    if let Some(expected) = opts.expected_modified_at.as_ref() {
        if let Some(meta) = metadata.as_ref() {
            if let Some(current) = format_modified_time(meta) {
                if &current != expected {
                    return Err(io::Error::other("File has changed since last read").into());
                }
            }
        }
    }

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).await?;
    }

    fs::write(&path, content).await?;
    let metadata = fs::metadata(&path).await?;

    Ok(FileWriteResult {
        modified_at: format_modified_time(&metadata),
    })
}

/// Return metadata for a workspace file (size + modified time).
#[tauri::command]
pub async fn stat_workspace_file(path: String) -> Result<FileStatResult> {
    let path = resolve_workspace_path(&path);
    let metadata = fs::metadata(&path).await?;

    let modified_at = format_modified_time(&metadata).ok_or_else(|| {
        io::Error::other(format!(
            "Failed to read modified time for {}",
            path.display()
        ))
    })?;

    Ok(FileStatResult {
        modified_at,
        size: metadata.len(),
    })
}
