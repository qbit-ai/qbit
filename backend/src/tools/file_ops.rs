//! File operation tools: read_file, write_file, create_file, edit_file, delete_file.

use std::fs;
use std::path::Path;

use anyhow::Result;
use serde_json::{json, Value};

use super::traits::Tool;

/// Check if a path is likely a binary file by examining the first bytes.
fn is_binary_file(content: &[u8]) -> bool {
    // Check first 8000 bytes for null bytes (common indicator of binary)
    let check_len = content.len().min(8000);
    content[..check_len].contains(&0)
}

/// Resolve a path relative to workspace and ensure it's within the workspace.
fn resolve_path(path_str: &str, workspace: &Path) -> Result<std::path::PathBuf, String> {
    let path = Path::new(path_str);

    // If path is absolute, check if it's within workspace
    let resolved = if path.is_absolute() {
        path.to_path_buf()
    } else {
        workspace.join(path)
    };

    // Canonicalize workspace for comparison
    let workspace_canonical = workspace
        .canonicalize()
        .map_err(|e| format!("Cannot resolve workspace path: {}", e))?;

    // Try to canonicalize the resolved path
    // For new files, canonicalize the parent directory
    let canonical = if resolved.exists() {
        resolved
            .canonicalize()
            .map_err(|e| format!("Cannot resolve path: {}", e))?
    } else {
        // For non-existent paths, find the deepest existing ancestor
        let mut check_path = resolved.as_path();
        let mut non_existent_parts: Vec<&std::ffi::OsStr> = Vec::new();

        // Walk up until we find an existing directory
        while !check_path.exists() {
            if let Some(name) = check_path.file_name() {
                non_existent_parts.push(name);
            }
            match check_path.parent() {
                Some(parent) if !parent.as_os_str().is_empty() => {
                    check_path = parent;
                }
                _ => {
                    // Reached root without finding existing ancestor
                    // Fall back to workspace as base
                    check_path = workspace;
                    break;
                }
            }
        }

        // Canonicalize the existing ancestor
        let canonical_ancestor = check_path
            .canonicalize()
            .map_err(|e| format!("Cannot resolve path: {}", e))?;

        // Check that the existing ancestor is within workspace
        if !canonical_ancestor.starts_with(&workspace_canonical) {
            return Err(format!(
                "Path '{}' is outside workspace (workspace: {})",
                path_str,
                workspace.display()
            ));
        }

        // Rebuild the full path with non-existent parts
        non_existent_parts.reverse();
        let mut result = canonical_ancestor;
        for part in non_existent_parts {
            result = result.join(part);
        }
        result
    };

    // Ensure path is within workspace
    if !canonical.starts_with(&workspace_canonical) {
        return Err(format!(
            "Path '{}' is outside workspace (workspace: {})",
            path_str,
            workspace.display()
        ));
    }

    Ok(canonical)
}

/// Get a string argument from JSON, returning an error if missing.
fn get_required_str<'a>(args: &'a Value, key: &str) -> Result<&'a str, Value> {
    args.get(key)
        .and_then(|v| v.as_str())
        .ok_or_else(|| json!({"error": format!("Missing required argument: {}", key)}))
}

/// Get an optional string argument from JSON.
fn get_optional_str<'a>(args: &'a Value, key: &str) -> Option<&'a str> {
    args.get(key).and_then(|v| v.as_str())
}

/// Get an optional integer argument from JSON.
fn get_optional_i64(args: &Value, key: &str) -> Option<i64> {
    args.get(key).and_then(|v| v.as_i64())
}

// ============================================================================
// read_file
// ============================================================================

/// Tool for reading file contents.
pub struct ReadFileTool;

#[async_trait::async_trait]
impl Tool for ReadFileTool {
    fn name(&self) -> &'static str {
        "read_file"
    }

    fn description(&self) -> &'static str {
        "Read the contents of a file. Supports optional line range for reading specific sections."
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Path to the file (relative to workspace)"
                },
                "line_start": {
                    "type": "integer",
                    "description": "Starting line number (1-indexed)"
                },
                "line_end": {
                    "type": "integer",
                    "description": "Ending line number (1-indexed, inclusive)"
                }
            },
            "required": ["path"]
        })
    }

    async fn execute(&self, args: Value, workspace: &Path) -> Result<Value> {
        let path_str = match get_required_str(&args, "path") {
            Ok(p) => p,
            Err(e) => return Ok(e),
        };

        let resolved = match resolve_path(path_str, workspace) {
            Ok(p) => p,
            Err(e) => return Ok(json!({"error": e})),
        };

        // Check if file exists
        if !resolved.exists() {
            return Ok(json!({
                "error": format!("File not found: {}", path_str),
                "resolved_path": resolved.display().to_string(),
                "workspace": workspace.display().to_string(),
                "hint": "If workspace is wrong, the terminal cwd may not be synced"
            }));
        }

        // Check if it's a directory
        if resolved.is_dir() {
            return Ok(json!({"error": format!("Path is a directory: {}", path_str)}));
        }

        // Read raw bytes first to check for binary
        let bytes = match fs::read(&resolved) {
            Ok(b) => b,
            Err(e) => return Ok(json!({"error": format!("Failed to read file: {}", e)})),
        };

        if is_binary_file(&bytes) {
            return Ok(json!({"error": format!("Cannot read binary file: {}", path_str)}));
        }

        // Convert to string
        let content = match String::from_utf8(bytes) {
            Ok(s) => s,
            Err(e) => return Ok(json!({"error": format!("File is not valid UTF-8: {}", e)})),
        };

        // Apply line range if specified
        let line_start = get_optional_i64(&args, "line_start").map(|n| n as usize);
        let line_end = get_optional_i64(&args, "line_end").map(|n| n as usize);

        let result_content = match (line_start, line_end) {
            (Some(start), Some(end)) => {
                let lines: Vec<&str> = content.lines().collect();
                let start_idx = start.saturating_sub(1); // Convert to 0-indexed
                let end_idx = end.min(lines.len());
                if start_idx >= lines.len() {
                    return Ok(json!({
                        "error": format!("Line {} is beyond end of file ({} lines)", start, lines.len())
                    }));
                }
                lines[start_idx..end_idx].join("\n")
            }
            (Some(start), None) => {
                let lines: Vec<&str> = content.lines().collect();
                let start_idx = start.saturating_sub(1);
                if start_idx >= lines.len() {
                    return Ok(json!({
                        "error": format!("Line {} is beyond end of file ({} lines)", start, lines.len())
                    }));
                }
                lines[start_idx..].join("\n")
            }
            (None, Some(end)) => {
                let lines: Vec<&str> = content.lines().collect();
                let end_idx = end.min(lines.len());
                lines[..end_idx].join("\n")
            }
            (None, None) => content,
        };

        Ok(json!({
            "content": result_content,
            "path": path_str
        }))
    }
}

// ============================================================================
// write_file
// ============================================================================

/// Tool for writing file contents (creates or overwrites).
pub struct WriteFileTool;

#[async_trait::async_trait]
impl Tool for WriteFileTool {
    fn name(&self) -> &'static str {
        "write_file"
    }

    fn description(&self) -> &'static str {
        "Write content to a file, replacing existing content. Creates the file and parent directories if they don't exist."
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Path to the file (relative to workspace)"
                },
                "content": {
                    "type": "string",
                    "description": "Content to write to the file"
                }
            },
            "required": ["path", "content"]
        })
    }

    async fn execute(&self, args: Value, workspace: &Path) -> Result<Value> {
        let path_str = match get_required_str(&args, "path") {
            Ok(p) => p,
            Err(e) => return Ok(e),
        };

        let content = match get_required_str(&args, "content") {
            Ok(c) => c,
            Err(e) => return Ok(e),
        };

        let resolved = match resolve_path(path_str, workspace) {
            Ok(p) => p,
            Err(e) => return Ok(json!({"error": e})),
        };

        // Check if it's a directory
        if resolved.is_dir() {
            return Ok(json!({"error": format!("Path is a directory: {}", path_str)}));
        }

        // Create parent directories if needed
        if let Some(parent) = resolved.parent() {
            if let Err(e) = fs::create_dir_all(parent) {
                return Ok(json!({"error": format!("Failed to create parent directories: {}", e)}));
            }
        }

        // Write the file
        match fs::write(&resolved, content) {
            Ok(()) => Ok(json!({
                "success": true,
                "path": path_str,
                "bytes_written": content.len()
            })),
            Err(e) => Ok(json!({"error": format!("Failed to write file: {}", e)})),
        }
    }
}

// ============================================================================
// create_file
// ============================================================================

/// Tool for creating a new file (fails if file exists).
pub struct CreateFileTool;

#[async_trait::async_trait]
impl Tool for CreateFileTool {
    fn name(&self) -> &'static str {
        "create_file"
    }

    fn description(&self) -> &'static str {
        "Create a new file with the specified content. Fails if the file already exists."
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Path for the new file (relative to workspace)"
                },
                "content": {
                    "type": "string",
                    "description": "Initial content for the file"
                }
            },
            "required": ["path", "content"]
        })
    }

    async fn execute(&self, args: Value, workspace: &Path) -> Result<Value> {
        let path_str = match get_required_str(&args, "path") {
            Ok(p) => p,
            Err(e) => return Ok(e),
        };

        let content = match get_required_str(&args, "content") {
            Ok(c) => c,
            Err(e) => return Ok(e),
        };

        // For create_file, we need to handle non-existent parent directories
        let path = Path::new(path_str);
        let resolved = if path.is_absolute() {
            path.to_path_buf()
        } else {
            workspace.join(path)
        };

        // Check if file already exists
        if resolved.exists() {
            return Ok(json!({"error": format!("File already exists: {}", path_str)}));
        }

        // Create parent directories if needed
        if let Some(parent) = resolved.parent() {
            if let Err(e) = fs::create_dir_all(parent) {
                return Ok(json!({"error": format!("Failed to create parent directories: {}", e)}));
            }
        }

        // Now validate the path is within workspace
        let workspace_canonical = match workspace.canonicalize() {
            Ok(p) => p,
            Err(e) => return Ok(json!({"error": format!("Cannot resolve workspace: {}", e)})),
        };

        let parent_canonical = match resolved.parent().and_then(|p| p.canonicalize().ok()) {
            Some(p) => p,
            None => return Ok(json!({"error": "Invalid path: no parent directory"})),
        };

        if !parent_canonical.starts_with(&workspace_canonical) {
            return Ok(json!({"error": format!("Path '{}' is outside workspace", path_str)}));
        }

        // Write the file
        match fs::write(&resolved, content) {
            Ok(()) => Ok(json!({
                "success": true,
                "path": path_str,
                "bytes_written": content.len()
            })),
            Err(e) => Ok(json!({"error": format!("Failed to create file: {}", e)})),
        }
    }
}

// ============================================================================
// edit_file
// ============================================================================

/// Tool for editing a file by search/replace.
pub struct EditFileTool;

#[async_trait::async_trait]
impl Tool for EditFileTool {
    fn name(&self) -> &'static str {
        "edit_file"
    }

    fn description(&self) -> &'static str {
        "Edit a file by replacing text. The old_text must match exactly once in the file."
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Path to the file (relative to workspace)"
                },
                "old_text": {
                    "type": "string",
                    "description": "Text to find and replace (must match exactly once)"
                },
                "new_text": {
                    "type": "string",
                    "description": "Replacement text"
                },
                "display_description": {
                    "type": "string",
                    "description": "Human-readable description of the edit"
                }
            },
            "required": ["path", "old_text", "new_text"]
        })
    }

    async fn execute(&self, args: Value, workspace: &Path) -> Result<Value> {
        let path_str = match get_required_str(&args, "path") {
            Ok(p) => p,
            Err(e) => return Ok(e),
        };

        let old_text = match get_required_str(&args, "old_text") {
            Ok(t) => t,
            Err(e) => return Ok(e),
        };

        let new_text = match get_required_str(&args, "new_text") {
            Ok(t) => t,
            Err(e) => return Ok(e),
        };

        let description = get_optional_str(&args, "display_description");

        let resolved = match resolve_path(path_str, workspace) {
            Ok(p) => p,
            Err(e) => return Ok(json!({"error": e})),
        };

        // Check if file exists
        if !resolved.exists() {
            return Ok(json!({
                "error": format!("File not found: {}", path_str),
                "resolved_path": resolved.display().to_string(),
                "workspace": workspace.display().to_string(),
                "hint": "If workspace is wrong, the terminal cwd may not be synced"
            }));
        }

        // Read the file
        let content = match fs::read_to_string(&resolved) {
            Ok(c) => c,
            Err(e) => return Ok(json!({"error": format!("Failed to read file: {}", e)})),
        };

        // Count occurrences
        let match_count = content.matches(old_text).count();

        if match_count == 0 {
            return Ok(json!({
                "error": "Edit failed: no matches found for the search text",
                "search_text": old_text,
                "suggestion": "Verify the exact text to replace, including whitespace and line endings"
            }));
        }

        if match_count > 1 {
            return Ok(json!({
                "error": format!("Edit failed: found {} matches, expected exactly 1", match_count),
                "match_count": match_count,
                "suggestion": "Provide more context to make the match unique"
            }));
        }

        // Perform the replacement
        let new_content = content.replacen(old_text, new_text, 1);

        // Generate diff preview
        let diff = generate_diff(&content, &new_content);

        // Write the file
        match fs::write(&resolved, &new_content) {
            Ok(()) => {
                let mut result = json!({
                    "success": true,
                    "path": path_str,
                    "diff": diff
                });
                if let Some(desc) = description {
                    result["description"] = json!(desc);
                }
                Ok(result)
            }
            Err(e) => Ok(json!({"error": format!("Failed to write file: {}", e)})),
        }
    }
}

/// Generate a simple unified diff between old and new content.
fn generate_diff(old: &str, new: &str) -> String {
    use similar::{ChangeTag, TextDiff};

    let diff = TextDiff::from_lines(old, new);
    let mut result = String::new();

    for change in diff.iter_all_changes() {
        let sign = match change.tag() {
            ChangeTag::Delete => "-",
            ChangeTag::Insert => "+",
            ChangeTag::Equal => " ",
        };
        result.push_str(sign);
        result.push_str(change.value());
        if !change.value().ends_with('\n') {
            result.push('\n');
        }
    }

    result
}

// ============================================================================
// delete_file
// ============================================================================

/// Tool for deleting a file.
pub struct DeleteFileTool;

#[async_trait::async_trait]
impl Tool for DeleteFileTool {
    fn name(&self) -> &'static str {
        "delete_file"
    }

    fn description(&self) -> &'static str {
        "Delete a file from the filesystem."
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Path to the file to delete (relative to workspace)"
                }
            },
            "required": ["path"]
        })
    }

    async fn execute(&self, args: Value, workspace: &Path) -> Result<Value> {
        let path_str = match get_required_str(&args, "path") {
            Ok(p) => p,
            Err(e) => return Ok(e),
        };

        let resolved = match resolve_path(path_str, workspace) {
            Ok(p) => p,
            Err(e) => return Ok(json!({"error": e})),
        };

        // Check if file exists
        if !resolved.exists() {
            return Ok(json!({
                "error": format!("File not found: {}", path_str),
                "resolved_path": resolved.display().to_string(),
                "workspace": workspace.display().to_string(),
                "hint": "If workspace is wrong, the terminal cwd may not be synced"
            }));
        }

        // Check if it's a directory
        if resolved.is_dir() {
            return Ok(json!({"error": format!("Path is a directory, not a file: {}", path_str)}));
        }

        // Delete the file
        match fs::remove_file(&resolved) {
            Ok(()) => Ok(json!({
                "success": true,
                "path": path_str
            })),
            Err(e) => Ok(json!({"error": format!("Failed to delete file: {}", e)})),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    // ========================================================================
    // read_file tests
    // ========================================================================

    #[tokio::test]
    async fn test_read_file_success() {
        let dir = tempdir().unwrap();
        let workspace = dir.path();

        fs::write(workspace.join("test.txt"), "hello world").unwrap();

        let tool = ReadFileTool;
        let result = tool
            .execute(json!({"path": "test.txt"}), workspace)
            .await
            .unwrap();

        assert!(result.get("error").is_none());
        assert_eq!(result["content"].as_str().unwrap(), "hello world");
    }

    #[tokio::test]
    async fn test_read_file_not_found() {
        let dir = tempdir().unwrap();
        let tool = ReadFileTool;
        let result = tool
            .execute(json!({"path": "nonexistent.txt"}), dir.path())
            .await
            .unwrap();

        assert!(result.get("error").is_some());
        assert!(result["error"].as_str().unwrap().contains("not found"));
    }

    #[tokio::test]
    async fn test_read_file_line_range() {
        let dir = tempdir().unwrap();
        let workspace = dir.path();

        fs::write(
            workspace.join("test.txt"),
            "line1\nline2\nline3\nline4\nline5",
        )
        .unwrap();

        let tool = ReadFileTool;
        let result = tool
            .execute(
                json!({"path": "test.txt", "line_start": 2, "line_end": 4}),
                workspace,
            )
            .await
            .unwrap();

        assert!(result.get("error").is_none());
        assert_eq!(result["content"].as_str().unwrap(), "line2\nline3\nline4");
    }

    #[tokio::test]
    async fn test_read_file_binary_detection() {
        let dir = tempdir().unwrap();
        let workspace = dir.path();

        // Write binary content (contains null bytes)
        fs::write(workspace.join("binary.bin"), b"hello\x00world").unwrap();

        let tool = ReadFileTool;
        let result = tool
            .execute(json!({"path": "binary.bin"}), workspace)
            .await
            .unwrap();

        assert!(result.get("error").is_some());
        assert!(result["error"].as_str().unwrap().contains("binary"));
    }

    #[tokio::test]
    async fn test_read_file_missing_path_arg() {
        let dir = tempdir().unwrap();
        let tool = ReadFileTool;
        let result = tool.execute(json!({}), dir.path()).await.unwrap();

        assert!(result.get("error").is_some());
        assert!(result["error"].as_str().unwrap().contains("Missing"));
    }

    // ========================================================================
    // write_file tests
    // ========================================================================

    #[tokio::test]
    async fn test_write_file_creates_new() {
        let dir = tempdir().unwrap();
        let workspace = dir.path();

        let tool = WriteFileTool;
        let result = tool
            .execute(
                json!({"path": "new.txt", "content": "new content"}),
                workspace,
            )
            .await
            .unwrap();

        assert!(result.get("error").is_none());
        assert_eq!(result["success"].as_bool(), Some(true));
        assert_eq!(
            fs::read_to_string(workspace.join("new.txt")).unwrap(),
            "new content"
        );
    }

    #[tokio::test]
    async fn test_write_file_overwrites() {
        let dir = tempdir().unwrap();
        let workspace = dir.path();

        fs::write(workspace.join("existing.txt"), "old content").unwrap();

        let tool = WriteFileTool;
        let result = tool
            .execute(
                json!({"path": "existing.txt", "content": "new content"}),
                workspace,
            )
            .await
            .unwrap();

        assert!(result.get("error").is_none());
        assert_eq!(
            fs::read_to_string(workspace.join("existing.txt")).unwrap(),
            "new content"
        );
    }

    #[tokio::test]
    async fn test_write_file_creates_parent_dirs() {
        let dir = tempdir().unwrap();
        let workspace = dir.path();

        let tool = WriteFileTool;
        let result = tool
            .execute(
                json!({"path": "deep/nested/dir/file.txt", "content": "content"}),
                workspace,
            )
            .await
            .unwrap();

        assert!(result.get("error").is_none());
        assert!(workspace.join("deep/nested/dir/file.txt").exists());
    }

    // ========================================================================
    // create_file tests
    // ========================================================================

    #[tokio::test]
    async fn test_create_file_new() {
        let dir = tempdir().unwrap();
        let workspace = dir.path();

        let tool = CreateFileTool;
        let result = tool
            .execute(json!({"path": "new.txt", "content": "content"}), workspace)
            .await
            .unwrap();

        assert!(result.get("error").is_none());
        assert_eq!(result["success"].as_bool(), Some(true));
    }

    #[tokio::test]
    async fn test_create_file_fails_if_exists() {
        let dir = tempdir().unwrap();
        let workspace = dir.path();

        fs::write(workspace.join("existing.txt"), "existing").unwrap();

        let tool = CreateFileTool;
        let result = tool
            .execute(json!({"path": "existing.txt", "content": "new"}), workspace)
            .await
            .unwrap();

        assert!(result.get("error").is_some());
        assert!(result["error"].as_str().unwrap().contains("exists"));
    }

    // ========================================================================
    // edit_file tests
    // ========================================================================

    #[tokio::test]
    async fn test_edit_file_success() {
        let dir = tempdir().unwrap();
        let workspace = dir.path();

        fs::write(workspace.join("test.txt"), "hello world").unwrap();

        let tool = EditFileTool;
        let result = tool
            .execute(
                json!({
                    "path": "test.txt",
                    "old_text": "hello",
                    "new_text": "goodbye"
                }),
                workspace,
            )
            .await
            .unwrap();

        assert!(result.get("error").is_none());
        assert_eq!(result["success"].as_bool(), Some(true));
        assert_eq!(
            fs::read_to_string(workspace.join("test.txt")).unwrap(),
            "goodbye world"
        );
    }

    #[tokio::test]
    async fn test_edit_file_no_match() {
        let dir = tempdir().unwrap();
        let workspace = dir.path();

        fs::write(workspace.join("test.txt"), "hello world").unwrap();

        let tool = EditFileTool;
        let result = tool
            .execute(
                json!({
                    "path": "test.txt",
                    "old_text": "nonexistent",
                    "new_text": "replacement"
                }),
                workspace,
            )
            .await
            .unwrap();

        assert!(result.get("error").is_some());
        assert!(result["error"].as_str().unwrap().contains("no matches"));
    }

    #[tokio::test]
    async fn test_edit_file_multiple_matches() {
        let dir = tempdir().unwrap();
        let workspace = dir.path();

        fs::write(workspace.join("test.txt"), "hello hello hello").unwrap();

        let tool = EditFileTool;
        let result = tool
            .execute(
                json!({
                    "path": "test.txt",
                    "old_text": "hello",
                    "new_text": "goodbye"
                }),
                workspace,
            )
            .await
            .unwrap();

        assert!(result.get("error").is_some());
        assert!(result["error"].as_str().unwrap().contains("3 matches"));
    }

    #[tokio::test]
    async fn test_edit_file_returns_diff() {
        let dir = tempdir().unwrap();
        let workspace = dir.path();

        fs::write(workspace.join("test.txt"), "line1\nline2\nline3").unwrap();

        let tool = EditFileTool;
        let result = tool
            .execute(
                json!({
                    "path": "test.txt",
                    "old_text": "line2",
                    "new_text": "modified"
                }),
                workspace,
            )
            .await
            .unwrap();

        assert!(result.get("error").is_none());
        assert!(result.get("diff").is_some());
        let diff = result["diff"].as_str().unwrap();
        assert!(diff.contains("-line2"));
        assert!(diff.contains("+modified"));
    }

    // ========================================================================
    // delete_file tests
    // ========================================================================

    #[tokio::test]
    async fn test_delete_file_success() {
        let dir = tempdir().unwrap();
        let workspace = dir.path();

        fs::write(workspace.join("to_delete.txt"), "content").unwrap();
        assert!(workspace.join("to_delete.txt").exists());

        let tool = DeleteFileTool;
        let result = tool
            .execute(json!({"path": "to_delete.txt"}), workspace)
            .await
            .unwrap();

        assert!(result.get("error").is_none());
        assert_eq!(result["success"].as_bool(), Some(true));
        assert!(!workspace.join("to_delete.txt").exists());
    }

    #[tokio::test]
    async fn test_delete_file_not_found() {
        let dir = tempdir().unwrap();

        let tool = DeleteFileTool;
        let result = tool
            .execute(json!({"path": "nonexistent.txt"}), dir.path())
            .await
            .unwrap();

        assert!(result.get("error").is_some());
        assert!(result["error"].as_str().unwrap().contains("not found"));
    }

    #[tokio::test]
    async fn test_delete_file_is_directory() {
        let dir = tempdir().unwrap();
        let workspace = dir.path();

        fs::create_dir(workspace.join("subdir")).unwrap();

        let tool = DeleteFileTool;
        let result = tool
            .execute(json!({"path": "subdir"}), workspace)
            .await
            .unwrap();

        assert!(result.get("error").is_some());
        assert!(result["error"].as_str().unwrap().contains("directory"));
    }

    // ========================================================================
    // Path security tests
    // ========================================================================

    #[tokio::test]
    async fn test_path_traversal_blocked() {
        let dir = tempdir().unwrap();
        let workspace = dir.path();

        // Create a file outside the workspace
        let parent = workspace.parent().unwrap();
        fs::write(parent.join("outside.txt"), "secret").unwrap();

        let tool = ReadFileTool;
        let result = tool
            .execute(json!({"path": "../outside.txt"}), workspace)
            .await
            .unwrap();

        assert!(result.get("error").is_some());
        assert!(result["error"]
            .as_str()
            .unwrap()
            .contains("outside workspace"));
    }
}
