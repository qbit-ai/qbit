# System Hooks

The system hooks feature provides a mechanism for injecting contextual messages into the agent's conversation during execution. Hooks are triggered by predefined events (e.g., tool completions) and delivered as `<system-hook>` blocks.

## Overview

When certain conditions are met during an agent turn, the system can inject hooks to guide the AI's behavior. This is useful for:
- Prompting documentation updates after task completion
- Warning about potential issues
- Suggesting best practices at relevant moments

Hooks are:
- Collected during tool execution in a queue
- Batched into a single user message after all tool results
- Wrapped in `<system-hook>` XML tags for clear identification

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    Agent Turn Loop                          │
├─────────────────────────────────────────────────────────────┤
│  Initialize:                                                │
│    tool_results: Vec<UserContent>                           │
│    system_hooks: Vec<String>                                │
│                                                             │
│  For each tool call:                                        │
│    1. Execute tool → result                                 │
│    2. Push result to tool_results                           │
│    3. Run hook triggers:                                    │
│       - collect_system_hooks(tool_name, result)             │
│       - Extends system_hooks queue                          │
│                                                             │
│  After tool loop:                                           │
│    1. Push tool_results to chat_history                     │
│    2. If system_hooks not empty:                            │
│       Push formatted hooks as separate user message         │
└─────────────────────────────────────────────────────────────┘
```

## Built-in Triggers

### Plan Completion Hook

**Trigger**: `check_plan_completion_reminder`

**Fires when**: The `update_plan` tool is called and all tasks are marked as completed.

**Condition**:
```rust
tool_name == "update_plan"
  && result.success == true
  && result.summary.total > 0
  && result.summary.total == result.summary.completed
```

**Message**:
```
All tasks have been completed. Consider updating the following files with any new or updated information from this session:
- CLAUDE.md - Project conventions, commands, architecture changes
- README.md - User-facing documentation, setup instructions, feature descriptions
```

**Purpose**: Encourages the AI to update project documentation after completing a multi-step task, ensuring knowledge is captured.

## Implementation

### Key Files

| File | Purpose |
|------|---------|
| `backend/crates/qbit-ai/src/system_hooks.rs` | Trigger functions, formatting, collection |
| `backend/crates/qbit-ai/src/agentic_loop.rs` | Integration point (queue management) |

### Module Structure

```rust
// backend/crates/qbit-ai/src/system_hooks.rs

// Private helper functions
fn is_plan_complete(value: &Value) -> bool { ... }
fn check_plan_completion_reminder(tool_name: &str, result: &Value) -> Option<String> { ... }

// Public API
pub fn format_system_hooks(hooks: &[String]) -> String { ... }
pub fn collect_system_hooks(tool_name: &str, result: &Value) -> Vec<String> { ... }
```

### Integration in Agentic Loop

```rust
// backend/crates/qbit-ai/src/agentic_loop.rs

// 1. Import
use super::system_hooks::{collect_system_hooks, format_system_hooks};

// 2. Initialize queue alongside tool_results
let mut tool_results: Vec<UserContent> = vec![];
let mut system_hooks: Vec<String> = vec![];

// 3. After each tool execution
system_hooks.extend(collect_system_hooks(tool_name, &result.value));

// 4. After tool loop, push hooks as user message
if !system_hooks.is_empty() {
    chat_history.push(Message::User {
        content: OneOrMany::one(UserContent::Text(Text {
            text: format_system_hooks(&system_hooks),
        })),
    });
}
```

## Adding New Hook Triggers

Adding a new trigger is straightforward. Follow these steps:

### Step 1: Create the Trigger Function

Add a new function in `system_hooks.rs` with this signature:

```rust
fn check_your_trigger_name(tool_name: &str, result: &serde_json::Value) -> Option<String> {
    // Check conditions
    if !your_condition {
        return None;
    }

    // Return the hook message
    Some("Your hook message here".to_string())
}
```

**Parameters**:
- `tool_name`: The name of the tool that was executed (e.g., `"update_plan"`, `"run_pty_cmd"`)
- `result`: The JSON result returned by the tool execution

**Return**:
- `Some(String)` if the hook should fire
- `None` if conditions are not met

### Step 2: Register the Trigger

Add your function to the `triggers` array in `collect_system_hooks`:

```rust
pub fn collect_system_hooks(tool_name: &str, result: &serde_json::Value) -> Vec<String> {
    let triggers: &[fn(&str, &serde_json::Value) -> Option<String>] = &[
        check_plan_completion_reminder,
        check_your_trigger_name,  // Add here
    ];

    triggers
        .iter()
        .filter_map(|trigger| trigger(tool_name, result))
        .collect()
}
```

### Step 3: Add Tests

Add tests for your new trigger:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_your_trigger_fires() {
        let result = json!({ /* conditions that should trigger */ });
        let hooks = collect_system_hooks("relevant_tool", &result);
        assert_eq!(hooks.len(), 1);
        assert!(hooks[0].contains("expected text"));
    }

    #[test]
    fn test_your_trigger_does_not_fire() {
        let result = json!({ /* conditions that should NOT trigger */ });
        let hooks = collect_system_hooks("relevant_tool", &result);
        assert!(hooks.is_empty());
    }
}
```

### Example: Session Duration Hook

Here's a complete example of adding a hypothetical session duration trigger:

```rust
/// Trigger: Remind to save progress after long sessions
fn check_session_duration_hook(tool_name: &str, result: &serde_json::Value) -> Option<String> {
    // Only check on specific tools that indicate ongoing work
    if tool_name != "write_file" {
        return None;
    }

    // Check if session metadata indicates long duration
    let duration_minutes = result
        .get("session_metadata")
        .and_then(|m| m.get("duration_minutes"))
        .and_then(|d| d.as_u64())
        .unwrap_or(0);

    if duration_minutes < 30 {
        return None;
    }

    Some(
        "This session has been running for over 30 minutes. Consider:\n\
         - Committing your changes to avoid losing work\n\
         - Taking a break to review progress".to_string()
    )
}
```

## Modifying Existing Triggers

### Changing the Trigger Condition

Edit the conditional logic in the trigger function. For example, to make the plan completion hook also fire when there are pending tasks:

```rust
fn check_plan_completion_reminder(tool_name: &str, result: &serde_json::Value) -> Option<String> {
    if tool_name != "update_plan" {
        return None;
    }

    // Modified: Fire when 80% complete instead of 100%
    let summary = result.get("summary")?;
    let total = summary.get("total").and_then(|v| v.as_u64()).unwrap_or(0);
    let completed = summary.get("completed").and_then(|v| v.as_u64()).unwrap_or(0);

    if total == 0 || (completed as f64 / total as f64) < 0.8 {
        return None;
    }

    Some("Task plan is 80% complete...".to_string())
}
```

### Changing the Hook Message

Simply modify the string returned by `Some(...)`:

```rust
Some(
    "All tasks completed! Great work. Don't forget to:\n\
     - Update CLAUDE.md with any architectural decisions\n\
     - Add tests for new functionality\n\
     - Update the changelog".to_string()
)
```

### Disabling a Trigger

To temporarily disable a trigger, remove it from the `triggers` array:

```rust
let triggers: &[fn(&str, &serde_json::Value) -> Option<String>] = &[
    // check_plan_completion_reminder,  // Disabled
];
```

Or make the function always return `None`:

```rust
fn check_plan_completion_reminder(_tool_name: &str, _result: &serde_json::Value) -> Option<String> {
    None  // Disabled
}
```

## Output Format

Hooks are formatted with XML tags for clear identification:

**Single hook**:
```xml
<system-hook>
All tasks have been completed. Consider updating...
</system-hook>
```

**Multiple hooks**:
```xml
<system-hook>
First hook message.
</system-hook>

<system-hook>
Second hook message.
</system-hook>
```

## Future Extension Ideas

The system is designed to be easily extended. Potential future triggers:

| Trigger | Fires When | Purpose |
|---------|-----------|---------|
| Context utilization | Token usage > 80% | Warn about approaching limits |
| Security warning | Sensitive file modified | Remind about security review |
| Test reminder | Code files modified | Suggest running tests |
| Commit reminder | Multiple files changed | Suggest committing progress |
| Long output warning | Tool output truncated | Warn about missing data |

## Testing

Run the system hook tests:

```bash
cargo test -p qbit-ai system_hook
```

### Test Coverage

| Test | Verifies |
|------|----------|
| `test_is_plan_complete_all_done` | Detects complete plans |
| `test_is_plan_complete_some_pending` | Rejects incomplete plans |
| `test_is_plan_complete_in_progress` | Rejects in-progress plans |
| `test_is_plan_complete_empty_plan` | Rejects empty plans |
| `test_is_plan_complete_failed_update` | Handles failed updates |
| `test_is_plan_complete_malformed_response` | Handles malformed JSON |
| `test_format_system_hooks_single` | Formats single hook |
| `test_format_system_hooks_multiple` | Formats multiple hooks |
| `test_collect_system_hooks_plan_complete` | Collects on completion |
| `test_collect_system_hooks_plan_not_complete` | Skips incomplete |
| `test_collect_system_hooks_wrong_tool` | Skips wrong tools |
