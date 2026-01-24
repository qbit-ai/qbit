//! Built-in system hooks.
//!
//! This module defines all the default hooks that ship with the system.

use qbit_core::ToolName;

use super::hooks::{MessageHook, ToolHook};
use super::matcher::ToolMatcher;

/// Get all built-in message hooks.
pub fn message_hooks() -> Vec<MessageHook> {
    vec![
        // Currently no built-in message hooks.
        // Future hooks could include:
        // - Keyword detection for security-sensitive terms
        // - Pattern matching for common issues
    ]
}

/// Get all built-in tool hooks.
pub fn tool_hooks() -> Vec<ToolHook> {
    vec![plan_completion_hook()]
}

/// Hook that fires when all plan tasks are completed.
///
/// Reminds the agent to update documentation after completing a multi-step task.
fn plan_completion_hook() -> ToolHook {
    ToolHook::post(
        "plan_completion",
        ToolMatcher::tool(ToolName::UpdatePlan),
        |ctx| {
            if !is_plan_complete(ctx.result) {
                return None;
            }

            Some(
                "[Plan Complete - Documentation Check]

SKIP documentation updates for: bug fixes, refactors, minor tweaks, test changes, or any work that doesn't change external behavior or developer workflow.

For SIGNIFICANT changes only (new features, new commands, API changes, breaking changes):
- **Developer docs** (README.md, docs/*.md): commands, setup, APIs
- **Agent docs** (CLAUDE.md): code patterns, conventions, build commands

STOP CONDITIONS:
- Do NOT create new plan tasks after reading this message
- Do NOT call update_plan again
- If no docs need updating, respond to the user that the task is complete"
                    .to_string(),
            )
        },
    )
}

/// Check if the update_plan result indicates all tasks are completed.
fn is_plan_complete(value: &serde_json::Value) -> bool {
    value
        .get("success")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
        && value
            .get("summary")
            .map(|s| {
                let total = s.get("total").and_then(|v| v.as_u64()).unwrap_or(0);
                let completed = s.get("completed").and_then(|v| v.as_u64()).unwrap_or(0);
                total > 0 && total == completed
            })
            .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::system_hooks::context::PostToolContext;
    use serde_json::json;

    #[test]
    fn test_is_plan_complete_all_done() {
        let value = json!({
            "success": true,
            "summary": { "total": 3, "completed": 3, "in_progress": 0, "pending": 0 }
        });
        assert!(is_plan_complete(&value));
    }

    #[test]
    fn test_is_plan_complete_some_pending() {
        let value = json!({
            "success": true,
            "summary": { "total": 3, "completed": 2, "in_progress": 0, "pending": 1 }
        });
        assert!(!is_plan_complete(&value));
    }

    #[test]
    fn test_is_plan_complete_in_progress() {
        let value = json!({
            "success": true,
            "summary": { "total": 3, "completed": 2, "in_progress": 1, "pending": 0 }
        });
        assert!(!is_plan_complete(&value));
    }

    #[test]
    fn test_is_plan_complete_empty_plan() {
        let value = json!({
            "success": true,
            "summary": { "total": 0, "completed": 0, "in_progress": 0, "pending": 0 }
        });
        assert!(!is_plan_complete(&value)); // Empty plan is not "complete"
    }

    #[test]
    fn test_is_plan_complete_failed_update() {
        let value = json!({
            "success": false,
            "error": "something went wrong"
        });
        assert!(!is_plan_complete(&value));
    }

    #[test]
    fn test_is_plan_complete_malformed_response() {
        let value = json!({"foo": "bar"});
        assert!(!is_plan_complete(&value));
    }

    #[test]
    fn test_plan_completion_hook_fires() {
        let hook = plan_completion_hook();
        let args = json!({});
        let result = json!({
            "success": true,
            "summary": { "total": 2, "completed": 2, "in_progress": 0, "pending": 0 }
        });

        let ctx = PostToolContext::new("update_plan", &args, &result, true, 50, "s1");
        assert!(hook.matches_post(&ctx));

        let message = hook.execute_post(&ctx);
        assert!(message.is_some());
        assert!(message.unwrap().contains("Plan Complete"));
    }

    #[test]
    fn test_plan_completion_hook_does_not_fire_incomplete() {
        let hook = plan_completion_hook();
        let args = json!({});
        let result = json!({
            "success": true,
            "summary": { "total": 3, "completed": 2, "in_progress": 1, "pending": 0 }
        });

        let ctx = PostToolContext::new("update_plan", &args, &result, true, 50, "s1");
        assert!(hook.matches_post(&ctx)); // Matches the tool
        assert!(hook.execute_post(&ctx).is_none()); // But doesn't produce output
    }

    #[test]
    fn test_plan_completion_hook_wrong_tool() {
        let hook = plan_completion_hook();
        let args = json!({});
        let result = json!({
            "success": true,
            "summary": { "total": 2, "completed": 2, "in_progress": 0, "pending": 0 }
        });

        let ctx = PostToolContext::new("run_pty_cmd", &args, &result, true, 50, "s1");
        assert!(!hook.matches_post(&ctx));
    }

    #[test]
    fn test_builtin_hooks_loaded() {
        let message = message_hooks();
        let tool = tool_hooks();

        // Currently no message hooks, but tool hooks should have plan completion
        assert_eq!(message.len(), 0);
        assert_eq!(tool.len(), 1);
        assert_eq!(tool[0].name, "plan_completion");
    }
}
