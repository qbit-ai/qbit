//! Types for instrumented tracing functions.
//!
//! This module provides common types used by instrumented async functions
//! for LLM streaming, tool execution, and agent operations.

use qbit_context::token_budget::TokenUsage;

/// Result of streaming an LLM completion.
///
/// This struct captures all the data accumulated during a streaming LLM call,
/// including the response text, reasoning content, tool calls, and token usage.
///
/// # Example
///
/// ```rust,ignore
/// use qbit_tracing::StreamCompletionResult;
///
/// #[tracing::instrument(/* ... */)]
/// async fn stream_llm_completion(...) -> Result<StreamCompletionResult<ToolCall>> {
///     let mut result = StreamCompletionResult::default();
///
///     while let Some(chunk) = stream.next().await {
///         // Process chunks, accumulate into result
///         result.text.push_str(&chunk.text);
///     }
///
///     // Record on span
///     tracing::Span::current().record("gen_ai.usage.input_tokens", result.usage.input_tokens as i64);
///
///     Ok(result)
/// }
/// ```
#[derive(Debug, Clone, Default)]
pub struct StreamCompletionResult<T = ()> {
    /// Accumulated response text from the model.
    pub text: String,

    /// Accumulated thinking/reasoning content (for models with extended thinking).
    pub thinking: String,

    /// Thinking signature for Anthropic models (required for history).
    pub thinking_signature: Option<String>,

    /// Reasoning ID for OpenAI Responses API (rs_... IDs).
    pub thinking_id: Option<String>,

    /// Tool calls requested by the model.
    pub tool_calls: Vec<T>,

    /// Whether the model requested any tool calls.
    pub has_tool_calls: bool,

    /// Token usage statistics.
    pub usage: TokenUsage,

    /// The finish reason from the model.
    pub finish_reason: Option<String>,
}

impl<T> StreamCompletionResult<T> {
    /// Create a new empty result.
    pub fn new() -> Self
    where
        T: Default,
        Self: Default,
    {
        Self::default()
    }

    /// Check if the result contains any text content.
    pub fn has_text(&self) -> bool {
        !self.text.is_empty()
    }

    /// Check if the result contains any thinking content.
    pub fn has_thinking(&self) -> bool {
        !self.thinking.is_empty()
    }

    /// Get the total number of tokens used.
    pub fn total_tokens(&self) -> u64 {
        self.usage.input_tokens + self.usage.output_tokens
    }

    /// Get a truncated preview of the text for logging.
    pub fn text_preview(&self, max_len: usize) -> String {
        crate::helpers::truncate_string(&self.text, max_len)
    }

    /// Get a truncated preview of the thinking for logging.
    pub fn thinking_preview(&self, max_len: usize) -> String {
        crate::helpers::truncate_string(&self.thinking, max_len)
    }
}

/// Result of executing a tool.
///
/// This struct captures the outcome of a tool execution, including
/// the result value and success status.
#[derive(Debug, Clone)]
pub struct ToolExecutionResult {
    /// The result value from the tool.
    pub value: serde_json::Value,

    /// Whether the tool execution was successful.
    pub success: bool,

    /// Error message if the tool failed.
    pub error: Option<String>,

    /// Execution duration in milliseconds.
    pub duration_ms: Option<u64>,
}

impl ToolExecutionResult {
    /// Create a successful result.
    pub fn success(value: serde_json::Value) -> Self {
        Self {
            value,
            success: true,
            error: None,
            duration_ms: None,
        }
    }

    /// Create a failed result.
    pub fn failure(error: impl Into<String>) -> Self {
        Self {
            value: serde_json::Value::Null,
            success: false,
            error: Some(error.into()),
            duration_ms: None,
        }
    }

    /// Set the execution duration.
    pub fn with_duration(mut self, duration_ms: u64) -> Self {
        self.duration_ms = Some(duration_ms);
        self
    }

    /// Get a truncated preview of the result value for logging.
    pub fn value_preview(&self, max_len: usize) -> String {
        crate::helpers::truncate_json(&self.value, max_len)
    }
}

impl Default for ToolExecutionResult {
    fn default() -> Self {
        Self::success(serde_json::Value::Null)
    }
}

/// Configuration for session/thread tracing.
///
/// Use this to configure the root span for an agent session.
#[derive(Debug, Clone, Default)]
pub struct SessionConfig {
    /// Unique session identifier for grouping traces.
    pub session_id: String,

    /// Human-readable session name.
    pub session_name: Option<String>,

    /// Workspace path for the session.
    pub workspace: Option<String>,

    /// Additional metadata tags.
    pub tags: Vec<String>,
}

impl SessionConfig {
    /// Create a new session config with the given ID.
    pub fn new(session_id: impl Into<String>) -> Self {
        Self {
            session_id: session_id.into(),
            ..Default::default()
        }
    }

    /// Set the session name.
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.session_name = Some(name.into());
        self
    }

    /// Set the workspace path.
    pub fn with_workspace(mut self, workspace: impl Into<String>) -> Self {
        self.workspace = Some(workspace.into());
        self
    }

    /// Add a tag.
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    /// Get tags as comma-separated string for LangSmith.
    pub fn tags_string(&self) -> String {
        self.tags.join(",")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stream_completion_result_default() {
        let result: StreamCompletionResult<()> = StreamCompletionResult::default();
        assert!(result.text.is_empty());
        assert!(!result.has_tool_calls);
        assert_eq!(result.total_tokens(), 0);
    }

    #[test]
    fn test_stream_completion_result_with_data() {
        let mut result: StreamCompletionResult<String> = StreamCompletionResult::default();
        result.text = "Hello, world!".to_string();
        result.thinking = "Let me think...".to_string();
        result.usage.input_tokens = 100;
        result.usage.output_tokens = 50;
        result.tool_calls.push("read_file".to_string());
        result.has_tool_calls = true;

        assert!(result.has_text());
        assert!(result.has_thinking());
        assert_eq!(result.total_tokens(), 150);
        assert_eq!(result.tool_calls.len(), 1);
    }

    #[test]
    fn test_tool_execution_result_success() {
        let result = ToolExecutionResult::success(serde_json::json!({"ok": true}));
        assert!(result.success);
        assert!(result.error.is_none());
    }

    #[test]
    fn test_tool_execution_result_failure() {
        let result = ToolExecutionResult::failure("File not found");
        assert!(!result.success);
        assert_eq!(result.error, Some("File not found".to_string()));
    }

    #[test]
    fn test_session_config() {
        let config = SessionConfig::new("session-123")
            .with_name("Test Session")
            .with_workspace("/path/to/project")
            .with_tag("test")
            .with_tag("development");

        assert_eq!(config.session_id, "session-123");
        assert_eq!(config.session_name, Some("Test Session".to_string()));
        assert_eq!(config.tags_string(), "test,development");
    }
}
