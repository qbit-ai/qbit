//! File listing commands for @ file references

use crate::error::Result;
use ignore::WalkBuilder;
use serde::Serialize;
use std::path::PathBuf;

/// Information about a file for the @ command popup
#[derive(Debug, Clone, Serialize)]
pub struct FileInfo {
    /// File name (e.g., "Button.tsx")
    pub name: String,
    /// Relative path from workspace root (e.g., "src/components/Button.tsx")
    pub relative_path: String,
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
