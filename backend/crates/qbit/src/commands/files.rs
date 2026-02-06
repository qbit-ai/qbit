//! File listing and workspace file commands

use crate::error::Result;
use base64::prelude::*;
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

/// Read a file as base64 data URL.
/// Accepts absolute paths (for drag-drop from anywhere on the system).
#[tauri::command]
pub async fn read_file_as_base64(path: String) -> Result<String> {
    let path = PathBuf::from(&path);

    if !path.exists() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("File not found: {}", path.display()),
        )
        .into());
    }

    let metadata = fs::metadata(&path).await?;
    if !metadata.is_file() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("Not a file: {}", path.display()),
        )
        .into());
    }

    // Determine MIME type from extension
    let mime_type = path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| match ext.to_lowercase().as_str() {
            "png" => "image/png",
            "jpg" | "jpeg" => "image/jpeg",
            "gif" => "image/gif",
            "webp" => "image/webp",
            "svg" => "image/svg+xml",
            "pdf" => "application/pdf",
            _ => "application/octet-stream",
        })
        .unwrap_or("application/octet-stream");

    let bytes = fs::read(&path).await?;
    let base64_data = BASE64_STANDARD.encode(&bytes);

    Ok(format!("data:{};base64,{}", mime_type, base64_data))
}

/// Type of filesystem entry
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DirEntryType {
    File,
    Directory,
    Symlink,
}

/// A single directory entry for the file browser
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DirEntry {
    /// Entry name (e.g., "Documents")
    pub name: String,
    /// Full path
    pub path: String,
    /// Type of entry
    pub entry_type: DirEntryType,
    /// Size in bytes (for files)
    pub size: Option<u64>,
    /// Last modified time (ISO 8601)
    pub modified_at: Option<String>,
}

/// List entries in a directory for the file browser.
/// Returns files and directories, sorted with directories first.
#[tauri::command]
pub async fn list_directory(path: String, show_hidden: Option<bool>) -> Result<Vec<DirEntry>> {
    let dir_path = if path.is_empty() {
        workspace_root()
    } else {
        resolve_workspace_path(&path)
    };

    if !dir_path.exists() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("Directory not found: {}", dir_path.display()),
        )
        .into());
    }

    if !dir_path.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("Not a directory: {}", dir_path.display()),
        )
        .into());
    }

    let mut entries: Vec<DirEntry> = Vec::new();
    let mut read_dir = fs::read_dir(&dir_path).await?;

    while let Some(entry) = read_dir.next_entry().await? {
        let name = entry.file_name().to_string_lossy().to_string();

        // Skip hidden files (starting with .) unless show_hidden is true
        if !show_hidden.unwrap_or(false) && name.starts_with('.') {
            continue;
        }

        let entry_path = entry.path();
        let metadata = entry.metadata().await.ok();

        let entry_type = if let Some(ref meta) = metadata {
            if meta.is_dir() {
                DirEntryType::Directory
            } else if meta.is_symlink() {
                DirEntryType::Symlink
            } else {
                DirEntryType::File
            }
        } else {
            DirEntryType::File
        };

        let size = metadata.as_ref().filter(|m| m.is_file()).map(|m| m.len());
        let modified_at = metadata.as_ref().and_then(format_modified_time);

        entries.push(DirEntry {
            name,
            path: entry_path.to_string_lossy().to_string(),
            entry_type,
            size,
            modified_at,
        });
    }

    // Sort: directories first, then alphabetically by name
    entries.sort_by(|a, b| {
        let a_is_dir = a.entry_type == DirEntryType::Directory;
        let b_is_dir = b.entry_type == DirEntryType::Directory;

        match (a_is_dir, b_is_dir) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
        }
    });

    Ok(entries)
}
