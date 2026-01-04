//! Tool implementations for AST-grep search and replace.
//!
//! These tools implement the `qbit_core::Tool` trait for integration
//! with the Qbit tool registry.

use std::path::Path;

use anyhow::Result;
use qbit_core::Tool;
use serde_json::{json, Value};

use crate::{replace, search};

/// Get a required string argument from JSON.
fn get_required_str<'a>(args: &'a Value, key: &str) -> Result<&'a str, Value> {
    args.get(key)
        .and_then(|v| v.as_str())
        .ok_or_else(|| json!({"error": format!("Missing required argument: {}", key)}))
}

/// Get an optional string argument from JSON.
fn get_optional_str<'a>(args: &'a Value, key: &str) -> Option<&'a str> {
    args.get(key).and_then(|v| v.as_str())
}

// ============================================================================
// ast_grep (search)
// ============================================================================

/// AST-grep search tool for finding code patterns.
pub struct AstGrepTool;

#[async_trait::async_trait]
impl Tool for AstGrepTool {
    fn name(&self) -> &'static str {
        "ast_grep"
    }

    fn description(&self) -> &'static str {
        "Search code using AST patterns. Unlike regex, this understands code structure. \
         Use meta-variables like $VAR to match any expression. \
         Examples: 'fn $NAME($$$ARGS) { $$$BODY }' matches Rust functions, \
         'console.log($MSG)' matches JS logging calls. \
         Pattern must include complete syntactic structures."
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "pattern": {
                    "type": "string",
                    "description": "AST pattern to search for. Use $VAR for single nodes, $$$VAR for multiple nodes. Must be a complete syntactic structure."
                },
                "path": {
                    "type": "string",
                    "description": "File or directory to search (relative to workspace). Defaults to current directory."
                },
                "language": {
                    "type": "string",
                    "enum": ["rust", "typescript", "javascript", "python", "go", "java", "c", "cpp"],
                    "description": "Language for pattern parsing. Auto-detected from file extension if not specified."
                }
            },
            "required": ["pattern"]
        })
    }

    async fn execute(&self, args: Value, workspace: &Path) -> Result<Value> {
        let pattern = match get_required_str(&args, "pattern") {
            Ok(p) => p,
            Err(e) => return Ok(e),
        };

        let path = get_optional_str(&args, "path");
        let language = get_optional_str(&args, "language");

        match search(workspace, pattern, path, language) {
            Ok(result) => Ok(json!({
                "matches": result.matches.iter().map(|m| json!({
                    "file": m.file,
                    "line": m.line,
                    "column": m.column,
                    "text": m.text,
                    "end_line": m.end_line,
                    "end_column": m.end_column
                })).collect::<Vec<_>>(),
                "count": result.matches.len(),
                "files_searched": result.files_searched
            })),
            Err(e) => Ok(json!({"error": e.to_string()})),
        }
    }
}

// ============================================================================
// ast_grep_replace
// ============================================================================

/// AST-grep replace tool for structural code refactoring.
///
/// Note: This tool modifies files and should require HITL approval.
pub struct AstGrepReplaceTool;

#[async_trait::async_trait]
impl Tool for AstGrepReplaceTool {
    fn name(&self) -> &'static str {
        "ast_grep_replace"
    }

    fn description(&self) -> &'static str {
        "Replace code patterns using AST-aware rewriting. \
         Captured meta-variables from the pattern can be used in the replacement. \
         Example: pattern='console.log($MSG)' replacement='logger.info($MSG)' \
         transforms logging calls."
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "pattern": {
                    "type": "string",
                    "description": "AST pattern to match. Use $VAR for captures."
                },
                "replacement": {
                    "type": "string",
                    "description": "Replacement template. Use captured $VAR names from pattern."
                },
                "path": {
                    "type": "string",
                    "description": "File or directory to modify (relative to workspace)."
                },
                "language": {
                    "type": "string",
                    "enum": ["rust", "typescript", "javascript", "python", "go", "java", "c", "cpp"],
                    "description": "Language for pattern parsing. Auto-detected if not specified."
                }
            },
            "required": ["pattern", "replacement", "path"]
        })
    }

    async fn execute(&self, args: Value, workspace: &Path) -> Result<Value> {
        let pattern = match get_required_str(&args, "pattern") {
            Ok(p) => p,
            Err(e) => return Ok(e),
        };

        let replacement_str = match get_required_str(&args, "replacement") {
            Ok(r) => r,
            Err(e) => return Ok(e),
        };

        let path = match get_required_str(&args, "path") {
            Ok(p) => p,
            Err(e) => return Ok(e),
        };

        let language = get_optional_str(&args, "language");

        match replace(workspace, pattern, replacement_str, path, language) {
            Ok(result) => Ok(json!({
                "files_modified": result.files_modified,
                "replacements_count": result.replacements_count,
                "changes": result.changes.iter().map(|c| json!({
                    "file": c.file,
                    "line": c.line,
                    "original": c.original,
                    "replacement": c.replacement
                })).collect::<Vec<_>>()
            })),
            Err(e) => Ok(json!({"error": e.to_string()})),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_ast_grep_tool_search() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("test.js"), "console.log('hello');").unwrap();

        let tool = AstGrepTool;
        let result = tool
            .execute(
                json!({
                    "pattern": "console.log($MSG)",
                    "path": "test.js",
                    "language": "javascript"
                }),
                tmp.path(),
            )
            .await
            .unwrap();

        assert!(result.get("error").is_none());
        assert_eq!(result["count"].as_i64().unwrap(), 1);
    }

    #[tokio::test]
    async fn test_ast_grep_tool_missing_pattern() {
        let tmp = TempDir::new().unwrap();

        let tool = AstGrepTool;
        let result = tool.execute(json!({}), tmp.path()).await.unwrap();

        assert!(result.get("error").is_some());
        assert!(result["error"].as_str().unwrap().contains("pattern"));
    }

    #[tokio::test]
    async fn test_ast_grep_replace_tool() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("test.js"), "console.log('hello');").unwrap();

        let tool = AstGrepReplaceTool;
        let result = tool
            .execute(
                json!({
                    "pattern": "console.log($MSG)",
                    "replacement": "logger.info($MSG)",
                    "path": "test.js",
                    "language": "javascript"
                }),
                tmp.path(),
            )
            .await
            .unwrap();

        assert!(result.get("error").is_none());
        assert_eq!(result["replacements_count"].as_i64().unwrap(), 1);

        // Verify file was modified
        let content = fs::read_to_string(tmp.path().join("test.js")).unwrap();
        assert_eq!(content, "logger.info('hello');");
    }

    #[tokio::test]
    async fn test_ast_grep_replace_tool_missing_args() {
        let tmp = TempDir::new().unwrap();

        let tool = AstGrepReplaceTool;

        // Missing pattern
        let result = tool
            .execute(json!({"replacement": "foo", "path": "."}), tmp.path())
            .await
            .unwrap();
        assert!(result["error"].as_str().unwrap().contains("pattern"));

        // Missing replacement
        let result = tool
            .execute(json!({"pattern": "foo", "path": "."}), tmp.path())
            .await
            .unwrap();
        assert!(result["error"].as_str().unwrap().contains("replacement"));

        // Missing path
        let result = tool
            .execute(json!({"pattern": "foo", "replacement": "bar"}), tmp.path())
            .await
            .unwrap();
        assert!(result["error"].as_str().unwrap().contains("path"));
    }

    #[tokio::test]
    async fn test_ast_grep_tool_directory_search() {
        let tmp = TempDir::new().unwrap();
        fs::create_dir(tmp.path().join("src")).unwrap();
        fs::write(tmp.path().join("src/a.js"), "console.log('a');").unwrap();
        fs::write(tmp.path().join("src/b.js"), "console.log('b');").unwrap();

        let tool = AstGrepTool;
        let result = tool
            .execute(
                json!({
                    "pattern": "console.log($MSG)",
                    "path": "src",
                    "language": "javascript"
                }),
                tmp.path(),
            )
            .await
            .unwrap();

        assert!(result.get("error").is_none());
        assert_eq!(result["count"].as_i64().unwrap(), 2);
    }
}
