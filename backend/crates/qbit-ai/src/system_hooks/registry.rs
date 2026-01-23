//! Hook registry for system hooks.
//!
//! The registry manages all registered hooks and provides methods to run them.

use super::builtins;
use super::context::{MessageHookContext, PostToolContext, PreToolContext};
use super::hooks::{MessageHook, PreToolResult, ToolHook, ToolHookPhase};

/// Central registry for all system hooks.
///
/// The registry is initialized with built-in hooks and provides methods
/// to run hooks against various contexts.
pub struct HookRegistry {
    message_hooks: Vec<MessageHook>,
    tool_hooks: Vec<ToolHook>,
}

impl HookRegistry {
    /// Create a new registry with all built-in hooks.
    pub fn new() -> Self {
        let message_hooks = builtins::message_hooks();
        let tool_hooks = builtins::tool_hooks();

        tracing::info!(
            message_hooks = message_hooks.len(),
            tool_hooks = tool_hooks.len(),
            "System hook registry initialized"
        );

        Self {
            message_hooks,
            tool_hooks,
        }
    }

    /// Create an empty registry (for testing).
    pub fn empty() -> Self {
        Self {
            message_hooks: Vec::new(),
            tool_hooks: Vec::new(),
        }
    }

    /// Get the number of registered message hooks.
    pub fn message_hook_count(&self) -> usize {
        self.message_hooks.len()
    }

    /// Get the number of registered tool hooks.
    pub fn tool_hook_count(&self) -> usize {
        self.tool_hooks.len()
    }

    /// Run all matching message hooks and collect their outputs.
    ///
    /// Returns a vector of messages to inject (may be empty).
    pub fn run_message_hooks(&self, ctx: &MessageHookContext) -> Vec<String> {
        self.message_hooks
            .iter()
            .filter(|hook| hook.matches(ctx))
            .filter_map(|hook| {
                tracing::debug!(hook = %hook.name, "Running message hook");
                let output = hook.execute(ctx);
                if output.is_some() {
                    tracing::info!(
                        hook = %hook.name,
                        message_type = ?ctx.message_type,
                        "Message hook produced system reminder"
                    );
                }
                output
            })
            .collect()
    }

    /// Run all matching pre-tool hooks.
    ///
    /// If any hook blocks, returns `Block` immediately.
    /// Otherwise, collects all messages from `AllowWithMessage` results.
    pub fn run_pre_tool_hooks(&self, ctx: &PreToolContext) -> PreToolResult {
        let mut messages = Vec::new();

        for hook in self.tool_hooks.iter().filter(|h| h.matches_pre(ctx)) {
            tracing::debug!(hook = %hook.name, tool = %ctx.tool_name_raw, "Running pre-tool hook");

            if let Some(result) = hook.execute_pre(ctx) {
                match result {
                    PreToolResult::Block(reason) => {
                        tracing::info!(hook = %hook.name, reason = %reason, "Pre-tool hook blocked execution");
                        return PreToolResult::Block(reason);
                    }
                    PreToolResult::AllowWithMessage(msg) => {
                        tracing::info!(
                            hook = %hook.name,
                            tool = %ctx.tool_name_raw,
                            "Pre-tool hook produced system reminder"
                        );
                        messages.push(msg);
                    }
                    PreToolResult::Allow => {}
                }
            }
        }

        if messages.is_empty() {
            PreToolResult::Allow
        } else {
            // Combine all messages
            PreToolResult::AllowWithMessage(messages.join("\n\n"))
        }
    }

    /// Run all matching post-tool hooks and collect their outputs.
    ///
    /// Returns a vector of messages to inject (may be empty).
    pub fn run_post_tool_hooks(&self, ctx: &PostToolContext) -> Vec<String> {
        self.tool_hooks
            .iter()
            .filter(|hook| hook.matches_post(ctx))
            .filter_map(|hook| {
                tracing::debug!(hook = %hook.name, tool = %ctx.tool_name_raw, "Running post-tool hook");
                let output = hook.execute_post(ctx);
                if output.is_some() {
                    tracing::info!(
                        hook = %hook.name,
                        tool = %ctx.tool_name_raw,
                        "Post-tool hook produced system reminder"
                    );
                }
                output
            })
            .collect()
    }

    /// Get statistics about registered hooks.
    pub fn stats(&self) -> HookRegistryStats {
        let pre_tool_count = self
            .tool_hooks
            .iter()
            .filter(|h| h.phase == ToolHookPhase::Pre)
            .count();
        let post_tool_count = self
            .tool_hooks
            .iter()
            .filter(|h| h.phase == ToolHookPhase::Post)
            .count();

        HookRegistryStats {
            message_hooks: self.message_hooks.len(),
            pre_tool_hooks: pre_tool_count,
            post_tool_hooks: post_tool_count,
        }
    }
}

impl Default for HookRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for HookRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HookRegistry")
            .field("message_hooks", &self.message_hooks.len())
            .field("tool_hooks", &self.tool_hooks.len())
            .finish()
    }
}

/// Statistics about registered hooks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HookRegistryStats {
    /// Number of message hooks.
    pub message_hooks: usize,
    /// Number of pre-tool hooks.
    pub pre_tool_hooks: usize,
    /// Number of post-tool hooks.
    pub post_tool_hooks: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_empty_registry() {
        let registry = HookRegistry::empty();
        assert_eq!(registry.message_hook_count(), 0);
        assert_eq!(registry.tool_hook_count(), 0);
    }

    #[test]
    fn test_default_registry() {
        let registry = HookRegistry::new();
        // Should have at least the plan completion hook
        assert!(registry.tool_hook_count() >= 1);
    }

    #[test]
    fn test_run_message_hooks_empty() {
        let registry = HookRegistry::empty();
        let ctx = MessageHookContext::user_input("hello", "s1");
        let results = registry.run_message_hooks(&ctx);
        assert!(results.is_empty());
    }

    #[test]
    fn test_run_pre_tool_hooks_allow() {
        let registry = HookRegistry::empty();
        let args = json!({});
        let ctx = PreToolContext::new("read_file", &args, "s1");
        let result = registry.run_pre_tool_hooks(&ctx);
        assert!(result.is_allowed());
    }

    #[test]
    fn test_run_post_tool_hooks() {
        let registry = HookRegistry::new();
        let args = json!({});
        let result = json!({
            "success": true,
            "summary": { "total": 3, "completed": 3, "in_progress": 0, "pending": 0 }
        });
        let ctx = PostToolContext::new("update_plan", &args, &result, true, 50, "s1");

        let messages = registry.run_post_tool_hooks(&ctx);
        // Should have the plan completion message
        assert!(!messages.is_empty());
        assert!(messages[0].contains("Plan Complete"));
    }

    #[test]
    fn test_stats() {
        let registry = HookRegistry::new();
        let stats = registry.stats();
        // At minimum we have the plan completion post-tool hook
        assert!(stats.post_tool_hooks >= 1);
    }
}
