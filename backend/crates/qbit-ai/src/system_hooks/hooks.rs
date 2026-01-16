//! Hook definitions for system hooks.
//!
//! This module defines the hook structs that combine matchers with handlers.

use super::context::{MessageHookContext, MessageType, PostToolContext, PreToolContext};
use super::matcher::{MessageMatcher, ToolMatcher};

/// Result of a pre-tool hook evaluation.
#[derive(Debug, Clone)]
pub enum PreToolResult {
    /// Allow the tool to execute normally.
    Allow,
    /// Allow the tool but inject a message after execution.
    AllowWithMessage(String),
    /// Block the tool execution with a reason.
    Block(String),
}

impl PreToolResult {
    /// Check if this result allows tool execution.
    pub fn is_allowed(&self) -> bool {
        !matches!(self, Self::Block(_))
    }

    /// Get the message to inject, if any.
    pub fn message(&self) -> Option<&str> {
        match self {
            Self::AllowWithMessage(msg) => Some(msg),
            _ => None,
        }
    }

    /// Get the block reason, if blocked.
    pub fn block_reason(&self) -> Option<&str> {
        match self {
            Self::Block(reason) => Some(reason),
            _ => None,
        }
    }
}

/// Handler type for message hooks.
pub type MessageHookHandler = Box<dyn Fn(&MessageHookContext) -> Option<String> + Send + Sync>;

/// Handler type for pre-tool hooks.
pub type PreToolHookHandler = Box<dyn Fn(&PreToolContext) -> PreToolResult + Send + Sync>;

/// Handler type for post-tool hooks.
pub type PostToolHookHandler = Box<dyn Fn(&PostToolContext) -> Option<String> + Send + Sync>;

/// A hook that fires on user messages or agent responses.
pub struct MessageHook {
    /// Unique name for this hook (for logging/debugging).
    pub name: String,
    /// The matcher that determines when this hook fires.
    pub matcher: MessageMatcher,
    /// Which message type(s) this hook applies to.
    pub target: MessageType,
    /// The handler that produces the hook output.
    pub handler: MessageHookHandler,
    /// Whether this hook is enabled.
    pub enabled: bool,
}

impl MessageHook {
    /// Create a new message hook.
    pub fn new(
        name: impl Into<String>,
        target: MessageType,
        matcher: MessageMatcher,
        handler: impl Fn(&MessageHookContext) -> Option<String> + Send + Sync + 'static,
    ) -> Self {
        Self {
            name: name.into(),
            matcher,
            target,
            handler: Box::new(handler),
            enabled: true,
        }
    }

    /// Check if this hook matches the given context.
    pub fn matches(&self, ctx: &MessageHookContext) -> bool {
        self.enabled && ctx.message_type == self.target && self.matcher.matches(ctx)
    }

    /// Execute the hook handler.
    pub fn execute(&self, ctx: &MessageHookContext) -> Option<String> {
        (self.handler)(ctx)
    }
}

impl std::fmt::Debug for MessageHook {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MessageHook")
            .field("name", &self.name)
            .field("matcher", &self.matcher)
            .field("target", &self.target)
            .field("enabled", &self.enabled)
            .finish_non_exhaustive()
    }
}

/// Phase when a tool hook fires.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolHookPhase {
    /// Before tool execution.
    Pre,
    /// After tool execution.
    Post,
}

/// A hook that fires before or after tool execution.
pub struct ToolHook {
    /// Unique name for this hook (for logging/debugging).
    pub name: String,
    /// Whether this hook fires pre or post execution.
    pub phase: ToolHookPhase,
    /// The matcher that determines when this hook fires.
    pub matcher: ToolMatcher,
    /// The handler (stored as enum to handle both pre and post).
    handler: ToolHookHandlerInner,
    /// Whether this hook is enabled.
    pub enabled: bool,
}

enum ToolHookHandlerInner {
    Pre(PreToolHookHandler),
    Post(PostToolHookHandler),
}

impl ToolHook {
    /// Create a new pre-tool hook.
    pub fn pre(
        name: impl Into<String>,
        matcher: ToolMatcher,
        handler: impl Fn(&PreToolContext) -> PreToolResult + Send + Sync + 'static,
    ) -> Self {
        Self {
            name: name.into(),
            phase: ToolHookPhase::Pre,
            matcher,
            handler: ToolHookHandlerInner::Pre(Box::new(handler)),
            enabled: true,
        }
    }

    /// Create a new post-tool hook.
    pub fn post(
        name: impl Into<String>,
        matcher: ToolMatcher,
        handler: impl Fn(&PostToolContext) -> Option<String> + Send + Sync + 'static,
    ) -> Self {
        Self {
            name: name.into(),
            phase: ToolHookPhase::Post,
            matcher,
            handler: ToolHookHandlerInner::Post(Box::new(handler)),
            enabled: true,
        }
    }

    /// Check if this hook matches the given pre-tool context.
    pub fn matches_pre(&self, ctx: &PreToolContext) -> bool {
        self.enabled && self.phase == ToolHookPhase::Pre && self.matcher.matches_pre(ctx)
    }

    /// Check if this hook matches the given post-tool context.
    pub fn matches_post(&self, ctx: &PostToolContext) -> bool {
        self.enabled && self.phase == ToolHookPhase::Post && self.matcher.matches_post(ctx)
    }

    /// Execute the pre-tool handler.
    ///
    /// Returns `None` if this is not a pre-tool hook.
    pub fn execute_pre(&self, ctx: &PreToolContext) -> Option<PreToolResult> {
        match &self.handler {
            ToolHookHandlerInner::Pre(handler) => Some(handler(ctx)),
            ToolHookHandlerInner::Post(_) => None,
        }
    }

    /// Execute the post-tool handler.
    ///
    /// Returns `None` if this is not a post-tool hook or if the handler returns None.
    pub fn execute_post(&self, ctx: &PostToolContext) -> Option<String> {
        match &self.handler {
            ToolHookHandlerInner::Post(handler) => handler(ctx),
            ToolHookHandlerInner::Pre(_) => None,
        }
    }
}

impl std::fmt::Debug for ToolHook {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ToolHook")
            .field("name", &self.name)
            .field("phase", &self.phase)
            .field("matcher", &self.matcher)
            .field("enabled", &self.enabled)
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_pre_tool_result() {
        let allow = PreToolResult::Allow;
        assert!(allow.is_allowed());
        assert!(allow.message().is_none());
        assert!(allow.block_reason().is_none());

        let allow_msg = PreToolResult::AllowWithMessage("Note this".into());
        assert!(allow_msg.is_allowed());
        assert_eq!(allow_msg.message(), Some("Note this"));

        let block = PreToolResult::Block("Not allowed".into());
        assert!(!block.is_allowed());
        assert_eq!(block.block_reason(), Some("Not allowed"));
    }

    #[test]
    fn test_message_hook() {
        let hook = MessageHook::new(
            "test_hook",
            MessageType::UserInput,
            MessageMatcher::keyword("help"),
            |_ctx| Some("Help detected".into()),
        );

        let ctx = MessageHookContext::user_input("I need help", "s1");
        assert!(hook.matches(&ctx));
        assert_eq!(hook.execute(&ctx), Some("Help detected".into()));

        let ctx = MessageHookContext::agent_response("I need help", "s1");
        assert!(!hook.matches(&ctx)); // wrong message type
    }

    #[test]
    fn test_tool_hook_pre() {
        use qbit_core::ToolName;

        let hook = ToolHook::pre(
            "warn_before_write",
            ToolMatcher::tool(ToolName::WriteFile),
            |_ctx| PreToolResult::AllowWithMessage("About to write file".into()),
        );

        let args = json!({});
        let ctx = PreToolContext::new("write_file", &args, "s1");
        assert!(hook.matches_pre(&ctx));

        let result = hook.execute_pre(&ctx).unwrap();
        assert!(result.is_allowed());
        assert_eq!(result.message(), Some("About to write file"));
    }

    #[test]
    fn test_tool_hook_post() {
        use qbit_core::ToolName;

        let hook = ToolHook::post(
            "notify_after_plan",
            ToolMatcher::tool(ToolName::UpdatePlan),
            |ctx| {
                if ctx.success {
                    Some("Plan updated successfully".into())
                } else {
                    None
                }
            },
        );

        let args = json!({});
        let result = json!({"success": true});
        let ctx = PostToolContext::new("update_plan", &args, &result, true, 50, "s1");

        assert!(hook.matches_post(&ctx));
        assert_eq!(
            hook.execute_post(&ctx),
            Some("Plan updated successfully".into())
        );
    }
}
