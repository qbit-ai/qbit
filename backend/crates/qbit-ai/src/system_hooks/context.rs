//! Hook context types for system hooks.
//!
//! These structs provide the context available to hook matchers and handlers.

use qbit_core::ToolName;
use serde_json::Value;

/// Type of message being processed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageType {
    /// User input message
    UserInput,
    /// Agent response message
    AgentResponse,
}

/// Context for message-based hooks.
///
/// Provided when matching/handling hooks that trigger on user messages
/// or agent responses.
#[derive(Debug)]
pub struct MessageHookContext<'a> {
    /// The message content
    pub content: &'a str,
    /// Whether this is user input or agent response
    pub message_type: MessageType,
    /// Current session ID
    pub session_id: &'a str,
}

impl<'a> MessageHookContext<'a> {
    /// Create a new message hook context.
    pub fn new(content: &'a str, message_type: MessageType, session_id: &'a str) -> Self {
        Self {
            content,
            message_type,
            session_id,
        }
    }

    /// Create context for user input.
    pub fn user_input(content: &'a str, session_id: &'a str) -> Self {
        Self::new(content, MessageType::UserInput, session_id)
    }

    /// Create context for agent response.
    pub fn agent_response(content: &'a str, session_id: &'a str) -> Self {
        Self::new(content, MessageType::AgentResponse, session_id)
    }
}

/// Context for pre-tool hooks (before execution).
///
/// Provided when matching/handling hooks that trigger before tool execution.
#[derive(Debug)]
pub struct PreToolContext<'a> {
    /// The tool being executed (None for dynamic/unknown tools)
    pub tool: Option<ToolName>,
    /// Raw tool name string (always available, even for dynamic tools)
    pub tool_name_raw: &'a str,
    /// Tool arguments
    pub args: &'a Value,
    /// Current session ID
    pub session_id: &'a str,
}

impl<'a> PreToolContext<'a> {
    /// Create a new pre-tool context.
    pub fn new(tool_name: &'a str, args: &'a Value, session_id: &'a str) -> Self {
        Self {
            tool: ToolName::from_str(tool_name),
            tool_name_raw: tool_name,
            args,
            session_id,
        }
    }

    /// Check if this is a sub-agent tool.
    pub fn is_sub_agent(&self) -> bool {
        ToolName::is_sub_agent_tool(self.tool_name_raw)
    }

    /// Get the sub-agent ID if this is a sub-agent tool.
    pub fn sub_agent_id(&self) -> Option<&str> {
        ToolName::sub_agent_id(self.tool_name_raw)
    }
}

/// Context for post-tool hooks (after execution).
///
/// Provided when matching/handling hooks that trigger after tool execution.
#[derive(Debug)]
pub struct PostToolContext<'a> {
    /// The tool that was executed (None for dynamic/unknown tools)
    pub tool: Option<ToolName>,
    /// Raw tool name string (always available, even for dynamic tools)
    pub tool_name_raw: &'a str,
    /// Tool arguments
    pub args: &'a Value,
    /// Tool execution result
    pub result: &'a Value,
    /// Whether the tool execution succeeded
    pub success: bool,
    /// Execution duration in milliseconds
    pub duration_ms: u64,
    /// Current session ID
    pub session_id: &'a str,
}

impl<'a> PostToolContext<'a> {
    /// Create a new post-tool context.
    pub fn new(
        tool_name: &'a str,
        args: &'a Value,
        result: &'a Value,
        success: bool,
        duration_ms: u64,
        session_id: &'a str,
    ) -> Self {
        Self {
            tool: ToolName::from_str(tool_name),
            tool_name_raw: tool_name,
            args,
            result,
            success,
            duration_ms,
            session_id,
        }
    }

    /// Check if this is a sub-agent tool.
    pub fn is_sub_agent(&self) -> bool {
        ToolName::is_sub_agent_tool(self.tool_name_raw)
    }

    /// Get the sub-agent ID if this is a sub-agent tool.
    pub fn sub_agent_id(&self) -> Option<&str> {
        ToolName::sub_agent_id(self.tool_name_raw)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_message_hook_context() {
        let ctx = MessageHookContext::user_input("hello world", "session-123");
        assert_eq!(ctx.content, "hello world");
        assert_eq!(ctx.message_type, MessageType::UserInput);
        assert_eq!(ctx.session_id, "session-123");

        let ctx = MessageHookContext::agent_response("I can help", "session-456");
        assert_eq!(ctx.message_type, MessageType::AgentResponse);
    }

    #[test]
    fn test_pre_tool_context() {
        let args = json!({"path": "/tmp/test.txt"});
        let ctx = PreToolContext::new("read_file", &args, "session-123");

        assert_eq!(ctx.tool, Some(ToolName::ReadFile));
        assert_eq!(ctx.tool_name_raw, "read_file");
        assert!(!ctx.is_sub_agent());
    }

    #[test]
    fn test_pre_tool_context_sub_agent() {
        let args = json!({"prompt": "analyze this"});
        let ctx = PreToolContext::new("sub_agent_coder", &args, "session-123");

        assert_eq!(ctx.tool, None); // sub-agent tools don't have a ToolName variant
        assert!(ctx.is_sub_agent());
        assert_eq!(ctx.sub_agent_id(), Some("coder"));
    }

    #[test]
    fn test_post_tool_context() {
        let args = json!({"path": "/tmp/test.txt"});
        let result = json!({"content": "file contents"});
        let ctx = PostToolContext::new("read_file", &args, &result, true, 150, "session-123");

        assert_eq!(ctx.tool, Some(ToolName::ReadFile));
        assert!(ctx.success);
        assert_eq!(ctx.duration_ms, 150);
    }
}
