//! Directory operation tools: list_files, list_directory, grep_file.

use std::fs;
use std::path::Path;

use anyhow::Result;
use ignore::WalkBuilder;
use serde_json::{json, Value};

use super::traits::Tool;

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

/// Get an optional boolean argument from JSON.
fn get_optional_bool(args: &Value, key: &str) -> Option<bool> {
    args.get(key).and_then(|v| v.as_bool())
}

/// Resolve a path relative to workspace.
fn resolve_path(path_str: &str, workspace: &Path) -> std::path::PathBuf {
    let path = Path::new(path_str);
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        workspace.join(path)
    }
}

/// Check if a resolved path is within the workspace.
fn is_within_workspace(resolved: &Path, workspace: &Path) -> bool {
    match (resolved.canonicalize(), workspace.canonicalize()) {
        (Ok(r), Ok(w)) => r.starts_with(w),
        _ => false,
    }
}

// ============================================================================
// list_files
// ============================================================================

/// Tool for listing files matching a glob pattern.
pub struct ListFilesTool;

#[async_trait::async_trait]
impl Tool for ListFilesTool {
    fn name(&self) -> &'static str {
        "list_files"
    }

    fn description(&self) -> &'static str {
        "List files matching a glob pattern. Respects .gitignore by default."
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Directory to search (relative to workspace, default: root)"
                },
                "pattern": {
                    "type": "string",
                    "description": "Glob pattern to match files (e.g., '*.rs', '**/*.ts')"
                },
                "recursive": {
                    "type": "boolean",
                    "description": "Search recursively (default: true)"
                }
            },
            "required": []
        })
    }

    async fn execute(&self, args: Value, workspace: &Path) -> Result<Value> {
        let path_str = get_optional_str(&args, "path").unwrap_or(".");
        let pattern = get_optional_str(&args, "pattern");
        let recursive = get_optional_bool(&args, "recursive").unwrap_or(true);

        let search_dir = resolve_path(path_str, workspace);

        // Check if directory exists
        if !search_dir.exists() {
            return Ok(json!({"error": format!("Directory not found: {}", path_str)}));
        }

        if !search_dir.is_dir() {
            return Ok(json!({"error": format!("Path is not a directory: {}", path_str)}));
        }

        // Check if within workspace
        if !is_within_workspace(&search_dir, workspace) {
            return Ok(json!({"error": format!("Path is outside workspace: {}", path_str)}));
        }

        // Build glob pattern matcher if provided
        let glob_matcher = pattern.and_then(|p| glob::Pattern::new(p).ok());

        // Walk the directory
        let mut files: Vec<String> = Vec::new();
        let max_depth = if recursive { None } else { Some(1) };

        let walker = WalkBuilder::new(&search_dir)
            .max_depth(max_depth)
            .hidden(false) // Don't ignore hidden files
            .git_ignore(true) // Respect .gitignore
            .git_global(true)
            .git_exclude(true)
            .build();

        for entry in walker.flatten() {
            let path = entry.path();

            // Skip directories
            if path.is_dir() {
                continue;
            }

            // Get path relative to workspace
            let relative = match path.strip_prefix(workspace) {
                Ok(r) => r.to_string_lossy().to_string(),
                Err(_) => continue,
            };

            // Apply glob pattern if provided
            if let Some(ref matcher) = glob_matcher {
                let file_name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();

                // Match against filename or full path
                if !matcher.matches(&file_name) && !matcher.matches(&relative) {
                    continue;
                }
            }

            files.push(relative);
        }

        // Sort for consistent output
        files.sort();

        Ok(json!({
            "files": files,
            "count": files.len(),
            "path": path_str
        }))
    }
}

// ============================================================================
// list_directory
// ============================================================================

/// Tool for listing directory contents.
pub struct ListDirectoryTool;

#[async_trait::async_trait]
impl Tool for ListDirectoryTool {
    fn name(&self) -> &'static str {
        "list_directory"
    }

    fn description(&self) -> &'static str {
        "List the contents of a directory with file/directory type indicators."
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Directory path (relative to workspace)"
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

        let dir_path = resolve_path(path_str, workspace);

        // Check if exists
        if !dir_path.exists() {
            return Ok(json!({"error": format!("Directory not found: {}", path_str)}));
        }

        // Check if it's a directory
        if !dir_path.is_dir() {
            return Ok(json!({"error": format!("Path is not a directory: {}", path_str)}));
        }

        // Check if within workspace
        if !is_within_workspace(&dir_path, workspace) {
            return Ok(json!({"error": format!("Path is outside workspace: {}", path_str)}));
        }

        // Read directory contents
        let entries = match fs::read_dir(&dir_path) {
            Ok(e) => e,
            Err(e) => return Ok(json!({"error": format!("Failed to read directory: {}", e)})),
        };

        let mut items: Vec<Value> = Vec::new();

        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            let entry_path = entry.path();
            let is_dir = entry_path.is_dir();
            let is_symlink = entry_path.is_symlink();

            let entry_type = if is_symlink {
                "symlink"
            } else if is_dir {
                "directory"
            } else {
                "file"
            };

            // Get file size for files
            let size = if !is_dir {
                entry_path.metadata().ok().map(|m| m.len())
            } else {
                None
            };

            let mut item = json!({
                "name": name,
                "type": entry_type
            });

            if let Some(s) = size {
                item["size"] = json!(s);
            }

            items.push(item);
        }

        // Sort by name
        items.sort_by(|a, b| {
            let a_name = a["name"].as_str().unwrap_or("");
            let b_name = b["name"].as_str().unwrap_or("");
            a_name.cmp(b_name)
        });

        Ok(json!({
            "entries": items,
            "count": items.len(),
            "path": path_str
        }))
    }
}

// ============================================================================
// grep_file
// ============================================================================

/// Tool for searching file contents with regex.
pub struct GrepFileTool;

#[async_trait::async_trait]
impl Tool for GrepFileTool {
    fn name(&self) -> &'static str {
        "grep_file"
    }

    fn description(&self) -> &'static str {
        "Search file contents using regex pattern. Returns matching lines with context."
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "pattern": {
                    "type": "string",
                    "description": "Regex pattern to search for"
                },
                "path": {
                    "type": "string",
                    "description": "File or directory to search (default: workspace root)"
                },
                "include": {
                    "type": "string",
                    "description": "Glob pattern to filter files (e.g., '*.rs')"
                }
            },
            "required": ["pattern"]
        })
    }

    async fn execute(&self, args: Value, workspace: &Path) -> Result<Value> {
        let pattern_str = match get_required_str(&args, "pattern") {
            Ok(p) => p,
            Err(e) => return Ok(e),
        };

        let path_str = get_optional_str(&args, "path").unwrap_or(".");
        let include = get_optional_str(&args, "include");

        // Compile regex
        let regex = match regex::Regex::new(pattern_str) {
            Ok(r) => r,
            Err(e) => return Ok(json!({"error": format!("Invalid regex pattern: {}", e)})),
        };

        let search_path = resolve_path(path_str, workspace);

        // Check if path exists
        if !search_path.exists() {
            return Ok(json!({"error": format!("Path not found: {}", path_str)}));
        }

        // Check if within workspace
        if !is_within_workspace(&search_path, workspace) {
            return Ok(json!({"error": format!("Path is outside workspace: {}", path_str)}));
        }

        // Build include pattern matcher
        let include_matcher = include.and_then(|p| glob::Pattern::new(p).ok());

        let mut matches: Vec<Value> = Vec::new();
        let max_matches = 1000; // Limit results

        // If it's a file, search just that file
        if search_path.is_file() {
            if let Some(file_matches) = search_file(&search_path, &regex, workspace) {
                matches.extend(file_matches);
            }
        } else {
            // Walk directory
            let walker = WalkBuilder::new(&search_path)
                .hidden(false)
                .git_ignore(true)
                .git_global(true)
                .git_exclude(true)
                .build();

            for entry in walker.flatten() {
                if matches.len() >= max_matches {
                    break;
                }

                let path = entry.path();
                if !path.is_file() {
                    continue;
                }

                // Apply include filter
                if let Some(ref matcher) = include_matcher {
                    let file_name = path
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_default();
                    if !matcher.matches(&file_name) {
                        continue;
                    }
                }

                if let Some(file_matches) = search_file(path, &regex, workspace) {
                    for m in file_matches {
                        if matches.len() >= max_matches {
                            break;
                        }
                        matches.push(m);
                    }
                }
            }
        }

        let truncated = matches.len() >= max_matches;

        Ok(json!({
            "matches": matches,
            "count": matches.len(),
            "pattern": pattern_str,
            "truncated": truncated
        }))
    }
}

/// Search a single file for regex matches.
fn search_file(path: &Path, regex: &regex::Regex, workspace: &Path) -> Option<Vec<Value>> {
    // Read file content
    let content = fs::read_to_string(path).ok()?;

    // Skip binary-looking files
    if content.contains('\0') {
        return None;
    }

    let relative_path = path
        .strip_prefix(workspace)
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| path.to_string_lossy().to_string());

    let mut results: Vec<Value> = Vec::new();

    for (line_num, line) in content.lines().enumerate() {
        if regex.is_match(line) {
            results.push(json!({
                "file": relative_path,
                "line": line_num + 1,
                "content": line.trim(),
                "match": true
            }));
        }
    }

    if results.is_empty() {
        None
    } else {
        Some(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    // ========================================================================
    // list_files tests
    // ========================================================================

    #[tokio::test]
    async fn test_list_files_basic() {
        let dir = tempdir().unwrap();
        let workspace = dir.path();

        fs::write(workspace.join("file1.txt"), "content").unwrap();
        fs::write(workspace.join("file2.txt"), "content").unwrap();
        fs::create_dir(workspace.join("subdir")).unwrap();
        fs::write(workspace.join("subdir/file3.txt"), "content").unwrap();

        let tool = ListFilesTool;
        let result = tool.execute(json!({}), workspace).await.unwrap();

        assert!(result.get("error").is_none());
        let files = result["files"].as_array().unwrap();
        assert_eq!(files.len(), 3);
    }

    #[tokio::test]
    async fn test_list_files_with_pattern() {
        let dir = tempdir().unwrap();
        let workspace = dir.path();

        fs::write(workspace.join("file.txt"), "content").unwrap();
        fs::write(workspace.join("file.rs"), "content").unwrap();
        fs::write(workspace.join("file.js"), "content").unwrap();

        let tool = ListFilesTool;
        let result = tool
            .execute(json!({"pattern": "*.rs"}), workspace)
            .await
            .unwrap();

        assert!(result.get("error").is_none());
        let files = result["files"].as_array().unwrap();
        assert_eq!(files.len(), 1);
        assert!(files[0].as_str().unwrap().ends_with(".rs"));
    }

    #[tokio::test]
    async fn test_list_files_non_recursive() {
        let dir = tempdir().unwrap();
        let workspace = dir.path();

        fs::write(workspace.join("top.txt"), "content").unwrap();
        fs::create_dir(workspace.join("subdir")).unwrap();
        fs::write(workspace.join("subdir/nested.txt"), "content").unwrap();

        let tool = ListFilesTool;
        let result = tool
            .execute(json!({"recursive": false}), workspace)
            .await
            .unwrap();

        assert!(result.get("error").is_none());
        let files = result["files"].as_array().unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].as_str().unwrap(), "top.txt");
    }

    #[tokio::test]
    async fn test_list_files_directory_not_found() {
        let dir = tempdir().unwrap();

        let tool = ListFilesTool;
        let result = tool
            .execute(json!({"path": "nonexistent"}), dir.path())
            .await
            .unwrap();

        assert!(result.get("error").is_some());
        assert!(result["error"].as_str().unwrap().contains("not found"));
    }

    // ========================================================================
    // list_directory tests
    // ========================================================================

    #[tokio::test]
    async fn test_list_directory_basic() {
        let dir = tempdir().unwrap();
        let workspace = dir.path();

        fs::write(workspace.join("file.txt"), "content").unwrap();
        fs::create_dir(workspace.join("subdir")).unwrap();

        let tool = ListDirectoryTool;
        let result = tool.execute(json!({"path": "."}), workspace).await.unwrap();

        assert!(result.get("error").is_none());
        let entries = result["entries"].as_array().unwrap();
        assert_eq!(entries.len(), 2);

        // Check types are correct
        let file_entry = entries.iter().find(|e| e["name"] == "file.txt").unwrap();
        assert_eq!(file_entry["type"].as_str().unwrap(), "file");

        let dir_entry = entries.iter().find(|e| e["name"] == "subdir").unwrap();
        assert_eq!(dir_entry["type"].as_str().unwrap(), "directory");
    }

    #[tokio::test]
    async fn test_list_directory_includes_size() {
        let dir = tempdir().unwrap();
        let workspace = dir.path();

        fs::write(workspace.join("file.txt"), "hello world").unwrap();

        let tool = ListDirectoryTool;
        let result = tool.execute(json!({"path": "."}), workspace).await.unwrap();

        assert!(result.get("error").is_none());
        let entries = result["entries"].as_array().unwrap();
        let file_entry = &entries[0];
        assert!(file_entry.get("size").is_some());
        assert_eq!(file_entry["size"].as_u64().unwrap(), 11);
    }

    #[tokio::test]
    async fn test_list_directory_not_found() {
        let dir = tempdir().unwrap();

        let tool = ListDirectoryTool;
        let result = tool
            .execute(json!({"path": "nonexistent"}), dir.path())
            .await
            .unwrap();

        assert!(result.get("error").is_some());
    }

    // ========================================================================
    // grep_file tests
    // ========================================================================

    #[tokio::test]
    async fn test_grep_file_basic() {
        let dir = tempdir().unwrap();
        let workspace = dir.path();

        fs::write(
            workspace.join("test.txt"),
            "hello world\ngoodbye world\nhello again",
        )
        .unwrap();

        let tool = GrepFileTool;
        let result = tool
            .execute(json!({"pattern": "hello", "path": "test.txt"}), workspace)
            .await
            .unwrap();

        assert!(result.get("error").is_none());
        let matches = result["matches"].as_array().unwrap();
        assert_eq!(matches.len(), 2);
        assert_eq!(matches[0]["line"].as_i64().unwrap(), 1);
        assert_eq!(matches[1]["line"].as_i64().unwrap(), 3);
    }

    #[tokio::test]
    async fn test_grep_file_regex() {
        let dir = tempdir().unwrap();
        let workspace = dir.path();

        fs::write(workspace.join("test.txt"), "foo123\nbar456\nfoo789").unwrap();

        let tool = GrepFileTool;
        let result = tool
            .execute(json!({"pattern": "foo\\d+", "path": "test.txt"}), workspace)
            .await
            .unwrap();

        assert!(result.get("error").is_none());
        let matches = result["matches"].as_array().unwrap();
        assert_eq!(matches.len(), 2);
    }

    #[tokio::test]
    async fn test_grep_file_directory_search() {
        let dir = tempdir().unwrap();
        let workspace = dir.path();

        fs::write(workspace.join("file1.txt"), "match here").unwrap();
        fs::write(workspace.join("file2.txt"), "no result").unwrap();
        fs::create_dir(workspace.join("subdir")).unwrap();
        fs::write(workspace.join("subdir/file3.txt"), "another match").unwrap();

        let tool = GrepFileTool;
        let result = tool
            .execute(json!({"pattern": "match"}), workspace)
            .await
            .unwrap();

        assert!(result.get("error").is_none());
        let matches = result["matches"].as_array().unwrap();
        assert_eq!(matches.len(), 2);
    }

    #[tokio::test]
    async fn test_grep_file_with_include_filter() {
        let dir = tempdir().unwrap();
        let workspace = dir.path();

        fs::write(workspace.join("file.txt"), "match").unwrap();
        fs::write(workspace.join("file.rs"), "match").unwrap();
        fs::write(workspace.join("file.js"), "match").unwrap();

        let tool = GrepFileTool;
        let result = tool
            .execute(json!({"pattern": "match", "include": "*.rs"}), workspace)
            .await
            .unwrap();

        assert!(result.get("error").is_none());
        let matches = result["matches"].as_array().unwrap();
        assert_eq!(matches.len(), 1);
        assert!(matches[0]["file"].as_str().unwrap().ends_with(".rs"));
    }

    #[tokio::test]
    async fn test_grep_file_invalid_regex() {
        let dir = tempdir().unwrap();

        let tool = GrepFileTool;
        let result = tool
            .execute(json!({"pattern": "[invalid"}), dir.path())
            .await
            .unwrap();

        assert!(result.get("error").is_some());
        assert!(result["error"].as_str().unwrap().contains("Invalid regex"));
    }

    #[tokio::test]
    async fn test_grep_file_no_matches() {
        let dir = tempdir().unwrap();
        let workspace = dir.path();

        fs::write(workspace.join("test.txt"), "hello world").unwrap();

        let tool = GrepFileTool;
        let result = tool
            .execute(
                json!({"pattern": "nonexistent", "path": "test.txt"}),
                workspace,
            )
            .await
            .unwrap();

        assert!(result.get("error").is_none());
        let matches = result["matches"].as_array().unwrap();
        assert_eq!(matches.len(), 0);
    }
}
