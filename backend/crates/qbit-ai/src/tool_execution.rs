//! Shared tool execution and routing.
//!
//! This module provides unified tool routing for all agent implementations,
//! eliminating duplication between main agent loops and sub-agent execution.
//!
//! # Tool Categories
//!
//! Tools are routed based on their name prefix:
//! - `web_fetch` - Web content fetching with readability extraction
//! - `web_search*`, `web_extract` - Tavily web search tools
//! - `update_plan` - Task planning updates
//! - `sub_agent_*` - Sub-agent delegation (main agent only)
//! - `run_command` - Alias for `run_pty_cmd`
//! - Everything else - Standard registry-based tools
//!
//! # Usage
//!
//! ```ignore
//! use qbit_ai::tool_execution::{route_tool_execution, ToolExecutionConfig, ToolSource};
//!
//! let config = ToolExecutionConfig {
//!     require_hitl: true,
//!     source: ToolSource::MainAgent,
//!     allow_sub_agents: true,
//! };
//!
//! let result = route_tool_execution(tool_name, &tool_args, &ctx, &config).await?;
//! ```

use std::sync::Arc;

use serde_json::Value;
use thiserror::Error;
use tokio::sync::RwLock;

use crate::indexer::IndexerState;
use crate::planner::PlanManager;
use qbit_core::ToolName;
use qbit_sub_agents::SubAgentRegistry;
use qbit_tools::ToolRegistry;

/// Configuration for tool execution behavior.
#[derive(Debug, Clone)]
pub struct ToolExecutionConfig {
    /// Whether HITL approval is required (false for trusted sub-agents).
    pub require_hitl: bool,
    /// Source identifier for logging and event emission.
    pub source: ToolSource,
    /// Whether sub-agent tools are allowed (false for sub-agents to prevent nesting).
    pub allow_sub_agents: bool,
}

impl Default for ToolExecutionConfig {
    fn default() -> Self {
        Self {
            require_hitl: true,
            source: ToolSource::MainAgent,
            allow_sub_agents: true,
        }
    }
}

/// Identifies the source of tool execution for logging and events.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolSource {
    /// Tool called from the main agent loop.
    MainAgent,
    /// Tool called from a sub-agent.
    SubAgent {
        /// Sub-agent identifier.
        name: String,
        /// Current nesting depth.
        depth: u32,
    },
}

impl ToolSource {
    /// Create a sub-agent source.
    pub fn sub_agent(name: impl Into<String>, depth: u32) -> Self {
        Self::SubAgent {
            name: name.into(),
            depth,
        }
    }

    /// Check if this is from the main agent.
    pub fn is_main_agent(&self) -> bool {
        matches!(self, Self::MainAgent)
    }
}

/// Result of successful tool execution with metadata.
#[derive(Debug, Clone)]
pub struct ToolExecutionResult {
    /// The result content (JSON serializable).
    pub content: Value,
    /// Whether the tool execution was successful.
    pub success: bool,
    /// Files modified by this tool execution (if any).
    pub files_modified: Vec<String>,
}

impl ToolExecutionResult {
    /// Create a successful result.
    pub fn success(content: Value) -> Self {
        Self {
            content,
            success: true,
            files_modified: vec![],
        }
    }

    /// Create a successful result with modified files.
    pub fn success_with_files(content: Value, files: Vec<String>) -> Self {
        Self {
            content,
            success: true,
            files_modified: files,
        }
    }

    /// Create a failure result.
    pub fn failure(content: Value) -> Self {
        Self {
            content,
            success: false,
            files_modified: vec![],
        }
    }

    /// Create an error result from a message.
    pub fn error(message: impl Into<String>) -> Self {
        Self {
            content: serde_json::json!({ "error": message.into() }),
            success: false,
            files_modified: vec![],
        }
    }
}

/// Errors that can occur during tool execution.
#[derive(Debug, Error)]
pub enum ToolExecutionError {
    /// Tool was not found in the registry.
    #[error("Tool not found: {0}")]
    ToolNotFound(String),

    /// Tool is not allowed in the current context.
    #[error("Tool not allowed: {0}")]
    ToolNotAllowed(String),

    /// Required state (indexer, tavily, etc.) is not initialized.
    #[error("Required state not initialized: {0}")]
    StateNotInitialized(String),

    /// Sub-agent not found in registry.
    #[error("Sub-agent not found: {0}")]
    SubAgentNotFound(String),

    /// Tool execution failed.
    #[error("Tool execution failed: {0}")]
    ExecutionFailed(String),

    /// Invalid tool arguments.
    #[error("Invalid arguments: {0}")]
    InvalidArguments(String),
}

/// Context providing access to tool execution dependencies.
///
/// This struct holds references to all the state and services needed
/// for tool execution, allowing the routing logic to be decoupled
/// from specific agent implementations.
pub struct ToolExecutionContext<'a> {
    /// Tool registry for standard tool execution.
    pub tool_registry: &'a Arc<RwLock<ToolRegistry>>,
    /// Sub-agent registry (only used if allow_sub_agents is true).
    pub sub_agent_registry: &'a Arc<RwLock<SubAgentRegistry>>,
    /// Indexer state for code search tools (optional).
    pub indexer_state: Option<&'a Arc<IndexerState>>,
    /// Plan manager for update_plan tool.
    pub plan_manager: &'a Arc<PlanManager>,
    /// Current workspace path.
    pub workspace: &'a Arc<RwLock<std::path::PathBuf>>,
}

/// Identifies which category a tool belongs to based on its name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolRoutingCategory {
    /// Web fetch tool (readability extraction).
    WebFetch,
    /// Plan update tool.
    UpdatePlan,
    /// Sub-agent delegation tool.
    SubAgent,
    /// Standard registry-based tool.
    Registry,
}

impl ToolRoutingCategory {
    /// Categorize a tool by its name.
    ///
    /// Uses `ToolName` for type-safe matching where possible,
    /// falling back to string prefix matching for dynamic tools.
    pub fn from_tool_name(name: &str) -> Self {
        // Try to parse as a known tool first
        if let Some(tool) = ToolName::from_str(name) {
            return Self::from_known_tool(tool);
        }

        // Handle dynamic tools by prefix
        if ToolName::is_sub_agent_tool(name) {
            Self::SubAgent
        } else {
            Self::Registry
        }
    }

    /// Categorize a known tool by its ToolName enum.
    pub fn from_known_tool(tool: ToolName) -> Self {
        match tool {
            // Indexer tools
            ToolName::IndexerSearchCode
            | ToolName::IndexerSearchFiles
            | ToolName::IndexerAnalyzeFile
            | ToolName::IndexerExtractSymbols
            | ToolName::IndexerGetMetrics
            | ToolName::IndexerDetectLanguage => Self::Registry,

            // Web fetch (special handling, not registry-based)
            ToolName::WebFetch => Self::WebFetch,

            // Plan update
            ToolName::UpdatePlan => Self::UpdatePlan,

            // Everything else goes through the registry
            _ => Self::Registry,
        }
    }
}

/// Route tool execution to the appropriate handler.
///
/// This is the main entry point for tool execution. It categorizes the tool
/// by name and delegates to the appropriate handler.
///
/// # Arguments
///
/// * `tool_name` - The name of the tool to execute.
/// * `tool_args` - The arguments to pass to the tool.
/// * `ctx` - Context providing access to tool dependencies.
/// * `config` - Configuration for tool execution behavior.
///
/// # Returns
///
/// Returns `Ok(ToolExecutionResult)` on success, or `Err(ToolExecutionError)` on failure.
///
/// # Tool Routing
///
/// Tools are routed based on their name prefix:
/// - `indexer_*` -> Indexer tools (code search, file analysis)
/// - `web_fetch` -> Web content fetching with readability
/// - `web_search*`, `web_extract` -> Tavily web search
/// - `update_plan` -> Task planning updates
/// - `sub_agent_*` -> Sub-agent delegation (if allowed)
/// - `run_command` -> Mapped to `run_pty_cmd`
/// - Everything else -> Registry-based execution
pub async fn route_tool_execution(
    tool_name: &str,
    tool_args: &Value,
    ctx: &ToolExecutionContext<'_>,
    config: &ToolExecutionConfig,
) -> Result<ToolExecutionResult, ToolExecutionError> {
    let category = ToolRoutingCategory::from_tool_name(tool_name);

    tracing::debug!(
        tool = %tool_name,
        category = ?category,
        source = ?config.source,
        "Routing tool execution"
    );

    match category {
        ToolRoutingCategory::WebFetch => execute_web_fetch_tool_routed(tool_name, tool_args).await,

        ToolRoutingCategory::UpdatePlan => {
            execute_plan_tool_routed(ctx.plan_manager, tool_args).await
        }

        ToolRoutingCategory::SubAgent => {
            if !config.allow_sub_agents {
                return Err(ToolExecutionError::ToolNotAllowed(format!(
                    "Sub-agent tools not allowed from {:?}",
                    config.source
                )));
            }
            // Sub-agent execution is a placeholder - actual execution requires model
            // and will be wired up when the full integration is done
            execute_sub_agent_placeholder(ctx.sub_agent_registry, tool_name, tool_args).await
        }

        ToolRoutingCategory::Registry => {
            execute_registry_tool(ctx.tool_registry, tool_name, tool_args).await
        }
    }
}

/// Execute a web fetch tool.
async fn execute_web_fetch_tool_routed(
    tool_name: &str,
    tool_args: &Value,
) -> Result<ToolExecutionResult, ToolExecutionError> {
    if ToolName::from_str(tool_name) != Some(ToolName::WebFetch) {
        return Err(ToolExecutionError::ToolNotFound(tool_name.to_string()));
    }

    // Placeholder - actual implementation will call the existing execute_web_fetch_tool
    tracing::debug!(tool = %tool_name, "Routing to web fetch tool executor");

    Ok(ToolExecutionResult::success(serde_json::json!({
        "_placeholder": true,
        "_tool": tool_name,
        "_args": tool_args,
        "_routed_to": "web_fetch"
    })))
}

/// Execute the update_plan tool.
async fn execute_plan_tool_routed(
    _plan_manager: &Arc<PlanManager>,
    tool_args: &Value,
) -> Result<ToolExecutionResult, ToolExecutionError> {
    // Placeholder - actual implementation will call the existing execute_plan_tool
    tracing::debug!("Routing to plan tool executor");

    Ok(ToolExecutionResult::success(serde_json::json!({
        "_placeholder": true,
        "_tool": "update_plan",
        "_args": tool_args,
        "_routed_to": "plan"
    })))
}

/// Placeholder for sub-agent execution.
///
/// Actual sub-agent execution requires the model and full context,
/// which will be wired up when integrating this module.
async fn execute_sub_agent_placeholder(
    sub_agent_registry: &Arc<RwLock<SubAgentRegistry>>,
    tool_name: &str,
    tool_args: &Value,
) -> Result<ToolExecutionResult, ToolExecutionError> {
    let agent_id = tool_name.strip_prefix("sub_agent_").ok_or_else(|| {
        ToolExecutionError::InvalidArguments("Invalid sub-agent tool name".to_string())
    })?;

    // Verify the sub-agent exists
    let registry = sub_agent_registry.read().await;
    if registry.get(agent_id).is_none() {
        return Err(ToolExecutionError::SubAgentNotFound(agent_id.to_string()));
    }

    // Placeholder - actual execution will be done when model is available
    tracing::debug!(agent_id = %agent_id, "Routing to sub-agent executor");

    Ok(ToolExecutionResult::success(serde_json::json!({
        "_placeholder": true,
        "_tool": tool_name,
        "_args": tool_args,
        "_routed_to": "sub_agent",
        "_agent_id": agent_id
    })))
}

/// Execute a standard registry-based tool.
async fn execute_registry_tool(
    tool_registry: &Arc<RwLock<ToolRegistry>>,
    tool_name: &str,
    tool_args: &Value,
) -> Result<ToolExecutionResult, ToolExecutionError> {
    // Map run_command to run_pty_cmd (run_command is a user-friendly alias)
    let effective_tool_name = match ToolName::from_str(tool_name) {
        Some(ToolName::RunCommand) => ToolName::RunPtyCmd.as_str(),
        _ => tool_name,
    };

    let registry = tool_registry.read().await;
    let result = registry
        .execute_tool(effective_tool_name, tool_args.clone())
        .await;

    match result {
        Ok(value) => {
            // Check for failure: exit_code != 0 OR presence of "error" field
            let is_failure_by_exit_code = value
                .get("exit_code")
                .and_then(|ec| ec.as_i64())
                .map(|ec| ec != 0)
                .unwrap_or(false);
            let has_error_field = value.get("error").is_some();
            let is_success = !is_failure_by_exit_code && !has_error_field;

            if is_success {
                Ok(ToolExecutionResult::success(value))
            } else {
                Ok(ToolExecutionResult::failure(value))
            }
        }
        Err(e) => Ok(ToolExecutionResult::error(e.to_string())),
    }
}

/// Normalize tool arguments for run_pty_cmd.
///
/// If the command is passed as an array, convert it to a space-joined string.
/// This prevents shell_words::join() from quoting metacharacters like &&, ||, |, etc.
pub fn normalize_run_pty_cmd_args(mut args: Value) -> Value {
    if let Some(obj) = args.as_object_mut() {
        if let Some(command) = obj.get_mut("command") {
            if let Some(arr) = command.as_array() {
                // Convert array to space-joined string
                let cmd_str: String = arr
                    .iter()
                    .filter_map(|v| v.as_str())
                    .collect::<Vec<_>>()
                    .join(" ");
                *command = Value::String(cmd_str);
            }
        }
    }
    args
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_tool_category_from_name() {
        assert_eq!(
            ToolRoutingCategory::from_tool_name("indexer_search_code"),
            ToolRoutingCategory::Registry
        );
        assert_eq!(
            ToolRoutingCategory::from_tool_name("indexer_analyze_file"),
            ToolRoutingCategory::Registry
        );
        assert_eq!(
            ToolRoutingCategory::from_tool_name("web_fetch"),
            ToolRoutingCategory::WebFetch
        );
        // web_search and other Tavily tools now go through Registry
        assert_eq!(
            ToolRoutingCategory::from_tool_name("web_search"),
            ToolRoutingCategory::Registry
        );
        assert_eq!(
            ToolRoutingCategory::from_tool_name("web_search_answer"),
            ToolRoutingCategory::Registry
        );
        assert_eq!(
            ToolRoutingCategory::from_tool_name("web_extract"),
            ToolRoutingCategory::Registry
        );
        assert_eq!(
            ToolRoutingCategory::from_tool_name("web_crawl"),
            ToolRoutingCategory::Registry
        );
        assert_eq!(
            ToolRoutingCategory::from_tool_name("web_map"),
            ToolRoutingCategory::Registry
        );
        assert_eq!(
            ToolRoutingCategory::from_tool_name("update_plan"),
            ToolRoutingCategory::UpdatePlan
        );
        assert_eq!(
            ToolRoutingCategory::from_tool_name("sub_agent_coder"),
            ToolRoutingCategory::SubAgent
        );
        assert_eq!(
            ToolRoutingCategory::from_tool_name("sub_agent_researcher"),
            ToolRoutingCategory::SubAgent
        );
        assert_eq!(
            ToolRoutingCategory::from_tool_name("read_file"),
            ToolRoutingCategory::Registry
        );
        assert_eq!(
            ToolRoutingCategory::from_tool_name("run_pty_cmd"),
            ToolRoutingCategory::Registry
        );
        assert_eq!(
            ToolRoutingCategory::from_tool_name("run_command"),
            ToolRoutingCategory::Registry
        );
    }

    #[test]
    fn test_tool_source_is_main_agent() {
        assert!(ToolSource::MainAgent.is_main_agent());
        assert!(!ToolSource::sub_agent("coder", 1).is_main_agent());
    }

    #[test]
    fn test_tool_execution_result_constructors() {
        let success = ToolExecutionResult::success(json!({"result": "ok"}));
        assert!(success.success);
        assert!(success.files_modified.is_empty());

        let with_files = ToolExecutionResult::success_with_files(
            json!({"result": "ok"}),
            vec!["file1.rs".to_string()],
        );
        assert!(with_files.success);
        assert_eq!(with_files.files_modified, vec!["file1.rs"]);

        let failure = ToolExecutionResult::failure(json!({"error": "failed"}));
        assert!(!failure.success);

        let error = ToolExecutionResult::error("Something went wrong");
        assert!(!error.success);
        assert_eq!(error.content["error"], "Something went wrong");
    }

    #[test]
    fn test_tool_execution_config_default() {
        let config = ToolExecutionConfig::default();
        assert!(config.require_hitl);
        assert_eq!(config.source, ToolSource::MainAgent);
        assert!(config.allow_sub_agents);
    }

    #[test]
    fn test_normalize_run_pty_cmd_args_array() {
        let args = json!({
            "command": ["cd", "/path", "&&", "pwd"],
            "cwd": "."
        });
        let normalized = normalize_run_pty_cmd_args(args);
        assert_eq!(normalized["command"].as_str().unwrap(), "cd /path && pwd");
        assert_eq!(normalized["cwd"].as_str().unwrap(), ".");
    }

    #[test]
    fn test_normalize_run_pty_cmd_args_string() {
        let args = json!({
            "command": "cd /path && pwd",
            "cwd": "."
        });
        let normalized = normalize_run_pty_cmd_args(args);
        assert_eq!(normalized["command"].as_str().unwrap(), "cd /path && pwd");
    }

    #[test]
    fn test_normalize_run_pty_cmd_args_pipe() {
        let args = json!({
            "command": ["ls", "-la", "|", "grep", "foo"]
        });
        let normalized = normalize_run_pty_cmd_args(args);
        assert_eq!(normalized["command"].as_str().unwrap(), "ls -la | grep foo");
    }

    #[test]
    fn test_tool_execution_error_display() {
        let err = ToolExecutionError::ToolNotFound("unknown_tool".to_string());
        assert_eq!(err.to_string(), "Tool not found: unknown_tool");

        let err = ToolExecutionError::SubAgentNotFound("missing_agent".to_string());
        assert_eq!(err.to_string(), "Sub-agent not found: missing_agent");
    }
}
