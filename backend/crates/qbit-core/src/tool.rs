//! Tool trait definition.
//!
//! This module defines the core `Tool` trait that all tool implementations must implement.
//! The trait is designed to be agnostic of the specific tool registry implementation.

use std::path::Path;

use anyhow::Result;
use serde_json::Value;

/// Trait for tool implementations.
///
/// All tools must be Send + Sync because ToolRegistry is wrapped in Arc<RwLock<>>.
///
/// ## Return Format Contract
///
/// The agentic loop determines success by checking:
/// 1. `exit_code` field (if present): non-zero means failure
/// 2. `error` field (if present): any value means failure
///
/// ```rust,ignore
/// // From agentic_loop.rs:429-436
/// let is_failure_by_exit_code = v.get("exit_code")
///     .and_then(|ec| ec.as_i64())
///     .map(|ec| ec != 0)
///     .unwrap_or(false);
/// let has_error_field = v.get("error").is_some();
/// let is_success = !is_failure_by_exit_code && !has_error_field;
/// ```
///
/// ### Success Format
/// - Return any JSON value (object, string, etc.)
/// - Do NOT include "error" field
/// - For shell commands, include "exit_code": 0
///
/// ### Failure Format
/// - Return JSON object with "error" field containing error message
/// - For shell commands, also include "exit_code": <non-zero>
#[async_trait::async_trait]
pub trait Tool: Send + Sync {
    /// Tool name (must match exactly what LLM requests)
    fn name(&self) -> &'static str;

    /// Tool description for LLM context
    fn description(&self) -> &'static str;

    /// JSON Schema for tool parameters
    fn parameters(&self) -> Value;

    /// Execute the tool with given arguments.
    ///
    /// ## Arguments
    /// - `args`: JSON value containing tool arguments
    /// - `workspace`: Path to the workspace root
    ///
    /// ## Returns
    /// - `Ok(Value)`: Tool result (success or failure JSON)
    /// - `Err(e)`: Unexpected error (will be converted to error JSON)
    ///
    /// Note: Tool implementations should return Ok(json!({"error": ...})) for
    /// expected failures, not Err. Reserve Err for truly unexpected conditions.
    async fn execute(&self, args: Value, workspace: &Path) -> Result<Value>;
}
