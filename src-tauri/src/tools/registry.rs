//! Tool registry implementation.
//!
//! Drop-in replacement for vtcode_core::tools::ToolRegistry.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use serde_json::Value;

use super::directory_ops::{GrepFileTool, ListDirectoryTool, ListFilesTool};
use super::file_ops::{CreateFileTool, DeleteFileTool, EditFileTool, ReadFileTool, WriteFileTool};
use super::shell::RunPtyCmdTool;
use super::traits::Tool;
use super::ToolError;

/// Tool registry that manages and executes tools.
///
/// This struct provides the same interface as vtcode_core::tools::ToolRegistry
/// to enable drop-in replacement.
///
/// ## Thread Safety
///
/// ToolRegistry is designed to be wrapped in `Arc<RwLock<ToolRegistry>>` for
/// concurrent access. All registered tools implement Send + Sync.
pub struct ToolRegistry {
    tools: HashMap<String, Arc<dyn Tool>>,
    workspace: PathBuf,
}

impl ToolRegistry {
    /// Create a new ToolRegistry for the given workspace.
    ///
    /// This signature matches vtcode_core::tools::ToolRegistry::new().
    ///
    /// ## Arguments
    /// - `workspace`: Path to the workspace root. All file operations are
    ///   restricted to this directory and its subdirectories.
    pub async fn new(workspace: PathBuf) -> Self {
        let mut tools: HashMap<String, Arc<dyn Tool>> = HashMap::new();

        // Register all tools
        let tool_list: Vec<Arc<dyn Tool>> = vec![
            // File operations
            Arc::new(ReadFileTool),
            Arc::new(WriteFileTool),
            Arc::new(CreateFileTool),
            Arc::new(EditFileTool),
            Arc::new(DeleteFileTool),
            // Directory operations
            Arc::new(ListFilesTool),
            Arc::new(ListDirectoryTool),
            Arc::new(GrepFileTool),
            // Shell
            Arc::new(RunPtyCmdTool),
        ];

        for tool in tool_list {
            tools.insert(tool.name().to_string(), tool);
        }

        Self { tools, workspace }
    }

    /// Execute a tool by name with the given arguments.
    ///
    /// This signature matches vtcode_core::tools::ToolRegistry::execute_tool().
    ///
    /// ## Return Format
    ///
    /// Returns JSON with optional `error` and `exit_code` fields for failure detection.
    /// The agentic loop determines success by:
    /// - No `error` field present
    /// - No non-zero `exit_code` field (for shell commands)
    ///
    /// ## Arguments
    /// - `name`: Tool name to execute
    /// - `args`: JSON arguments for the tool
    ///
    /// ## Returns
    /// - `Ok(Value)`: Tool result (may contain error field for tool-level failures)
    /// - `Err(e)`: Unknown tool or unexpected execution error
    pub async fn execute_tool(&mut self, name: &str, args: Value) -> Result<Value> {
        let tool = self
            .tools
            .get(name)
            .ok_or_else(|| ToolError::UnknownTool(name.to_string()))?;

        // Clone the Arc to avoid holding the borrow
        let tool = Arc::clone(tool);
        tool.execute(args, &self.workspace).await
    }

    /// List all available tool names.
    ///
    /// This signature matches vtcode_core::tools::ToolRegistry::available_tools().
    pub fn available_tools(&self) -> Vec<String> {
        self.tools.keys().cloned().collect()
    }

    /// Get the workspace path.
    pub fn workspace(&self) -> &PathBuf {
        &self.workspace
    }

    /// Update the workspace path.
    pub fn set_workspace(&mut self, workspace: PathBuf) {
        self.workspace = workspace;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_registry_creation() {
        let dir = tempdir().unwrap();
        let registry = ToolRegistry::new(dir.path().to_path_buf()).await;

        // Should have all expected tools
        let tools = registry.available_tools();
        assert!(tools.contains(&"read_file".to_string()));
        assert!(tools.contains(&"write_file".to_string()));
        assert!(tools.contains(&"create_file".to_string()));
        assert!(tools.contains(&"edit_file".to_string()));
        assert!(tools.contains(&"delete_file".to_string()));
        assert!(tools.contains(&"list_files".to_string()));
        assert!(tools.contains(&"list_directory".to_string()));
        assert!(tools.contains(&"grep_file".to_string()));
        assert!(tools.contains(&"run_pty_cmd".to_string()));
    }

    #[tokio::test]
    async fn test_unknown_tool_returns_error() {
        let dir = tempdir().unwrap();
        let mut registry = ToolRegistry::new(dir.path().to_path_buf()).await;

        let result = registry.execute_tool("nonexistent_tool", json!({})).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Unknown tool"));
    }

    #[tokio::test]
    async fn test_read_file_success_format() {
        let dir = tempdir().unwrap();
        let workspace = dir.path().to_path_buf();

        // Create test file
        std::fs::write(workspace.join("test.txt"), "hello world").unwrap();

        let mut registry = ToolRegistry::new(workspace).await;
        let result = registry
            .execute_tool("read_file", json!({"path": "test.txt"}))
            .await
            .unwrap();

        // Verify success format: no error field, has content
        assert!(
            result.get("error").is_none(),
            "Success should not have error field"
        );
        assert!(
            result.get("content").is_some(),
            "Success should have content"
        );
        assert_eq!(result["content"].as_str().unwrap(), "hello world");
    }

    #[tokio::test]
    async fn test_read_file_failure_format() {
        let dir = tempdir().unwrap();
        let workspace = dir.path().to_path_buf();

        let mut registry = ToolRegistry::new(workspace).await;
        let result = registry
            .execute_tool("read_file", json!({"path": "nonexistent.txt"}))
            .await
            .unwrap();

        // Verify failure format: has error field
        assert!(
            result.get("error").is_some(),
            "Failure must have error field"
        );
    }

    #[tokio::test]
    async fn test_run_pty_cmd_success_exit_code() {
        let dir = tempdir().unwrap();
        let workspace = dir.path().to_path_buf();

        let mut registry = ToolRegistry::new(workspace).await;
        let result = registry
            .execute_tool("run_pty_cmd", json!({"command": "echo hello"}))
            .await
            .unwrap();

        // Success: exit_code should be 0
        assert_eq!(result.get("exit_code").and_then(|v| v.as_i64()), Some(0));
        assert!(result.get("error").is_none());
    }

    #[tokio::test]
    async fn test_run_pty_cmd_failure_exit_code() {
        let dir = tempdir().unwrap();
        let workspace = dir.path().to_path_buf();

        let mut registry = ToolRegistry::new(workspace).await;
        let result = registry
            .execute_tool("run_pty_cmd", json!({"command": "exit 1"}))
            .await
            .unwrap();

        // Failure: exit_code should be non-zero
        let exit_code = result.get("exit_code").and_then(|v| v.as_i64());
        assert!(exit_code.is_some());
        assert_ne!(exit_code.unwrap(), 0);
    }

    #[tokio::test]
    async fn test_write_file_creates_file() {
        let dir = tempdir().unwrap();
        let workspace = dir.path().to_path_buf();

        let mut registry = ToolRegistry::new(workspace.clone()).await;
        let result = registry
            .execute_tool(
                "write_file",
                json!({"path": "new_file.txt", "content": "test content"}),
            )
            .await
            .unwrap();

        // Verify success
        assert!(result.get("error").is_none());
        assert_eq!(result["success"].as_bool(), Some(true));

        // Verify file was created
        let content = std::fs::read_to_string(workspace.join("new_file.txt")).unwrap();
        assert_eq!(content, "test content");
    }

    #[tokio::test]
    async fn test_create_file_fails_if_exists() {
        let dir = tempdir().unwrap();
        let workspace = dir.path().to_path_buf();

        // Create existing file
        std::fs::write(workspace.join("existing.txt"), "existing content").unwrap();

        let mut registry = ToolRegistry::new(workspace).await;
        let result = registry
            .execute_tool(
                "create_file",
                json!({"path": "existing.txt", "content": "new content"}),
            )
            .await
            .unwrap();

        // Should fail with error
        assert!(result.get("error").is_some());
        assert!(result["error"].as_str().unwrap().contains("exists"));
    }
}
