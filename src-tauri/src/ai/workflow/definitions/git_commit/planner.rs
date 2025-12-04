//! Planner task for the git commit workflow.
//!
//! Generates the actual git commands for each planned commit.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use graph_flow::{Context, NextAction, Task, TaskResult};

use super::state::{GitCommitState, WorkflowStage};
use super::STATE_KEY;
use crate::ai::workflow::models::WorkflowLlmExecutor;

/// System prompt for the planner agent.
const PLANNER_SYSTEM_PROMPT: &str = r#"You are a git command generator. Your task is to generate the exact git commands needed to execute the planned commits.

Guidelines:
1. Use `git add` with specific file paths (not `git add .`)
2. Use proper quoting for commit messages
3. Handle special characters in file paths
4. Include any necessary setup commands (like git stash if needed)
5. Order commands correctly for dependency resolution

Output the commands as a bash script that can be executed:
```bash
# Commit 1: <commit message summary>
git add path/to/file1.rs
git add path/to/file2.rs
git commit -m "feat: the commit message"

# Commit 2: <commit message summary>
git add path/to/file3.rs
git commit -m "fix: another commit message"
```

Do not include any explanation, just the commands. Each commit should be separated by a blank line."#;

/// Generates git commands for the planned commits.
pub struct PlannerTask {
    executor: Arc<dyn WorkflowLlmExecutor>,
}

impl PlannerTask {
    /// Create a new planner task with the given LLM executor.
    pub fn new(executor: Arc<dyn WorkflowLlmExecutor>) -> Self {
        Self { executor }
    }

    /// Extract bash commands from the response.
    fn parse_response(&self, response: &str) -> Option<String> {
        // Try to extract bash code block
        if let Some(start) = response.find("```bash") {
            let start = start + 7;
            if let Some(end) = response[start..].find("```") {
                return Some(response[start..start + end].trim().to_string());
            }
        }

        // Try generic code block
        if let Some(start) = response.find("```") {
            let start = start + 3;
            // Skip language identifier if present
            let start = if let Some(newline) = response[start..].find('\n') {
                start + newline + 1
            } else {
                start
            };
            if let Some(end) = response[start..].find("```") {
                return Some(response[start..start + end].trim().to_string());
            }
        }

        // If no code block, check if response looks like commands
        let lines: Vec<&str> = response.lines().collect();
        if lines.iter().any(|l| l.starts_with("git ")) {
            return Some(response.trim().to_string());
        }

        None
    }
}

#[async_trait]
impl Task for PlannerTask {
    fn id(&self) -> &str {
        "planner"
    }

    async fn run(&self, context: Context) -> graph_flow::Result<TaskResult> {
        let start_time = std::time::Instant::now();

        // Emit step started event
        self.executor.emit_step_started("planner", 3, 4);

        // Get current state
        let mut state: GitCommitState = context.get(STATE_KEY).await.unwrap_or_default();

        // Check if we have commit plans
        if state.commit_plans.is_empty() {
            state.errors.push("No commit plans to execute".to_string());
            context.set(STATE_KEY, state).await;
            let output = "No commit plans to generate commands for".to_string();
            self.executor.emit_step_completed(
                "planner",
                Some(&output),
                start_time.elapsed().as_millis() as u64,
            );
            return Ok(TaskResult::new(
                Some(output),
                NextAction::ContinueAndExecute,
            ));
        }

        // Update stage
        state.stage = WorkflowStage::Planning;
        context.set(STATE_KEY, state.clone()).await;

        // Build user prompt with commit plans
        let commit_plans_str = state
            .commit_plans
            .iter()
            .map(|cp| {
                format!(
                    "Commit {} - \"{}\"\nFiles:\n{}",
                    cp.order,
                    cp.message,
                    cp.files
                        .iter()
                        .map(|f| format!("  - {}", f))
                        .collect::<Vec<_>>()
                        .join("\n")
                )
            })
            .collect::<Vec<_>>()
            .join("\n\n");

        let user_prompt = format!(
            "Generate git commands for the following commit plans:\n\n{}",
            commit_plans_str
        );

        // Call LLM
        let context_vars: HashMap<String, serde_json::Value> = HashMap::new();
        let response = match self
            .executor
            .complete(PLANNER_SYSTEM_PROMPT, &user_prompt, context_vars)
            .await
        {
            Ok(r) => r,
            Err(e) => {
                state.errors.push(format!("Planner error: {}", e));
                context.set(STATE_KEY, state).await;
                let output = format!("Planning failed: {}", e);
                self.executor.emit_step_completed(
                    "planner",
                    Some(&output),
                    start_time.elapsed().as_millis() as u64,
                );
                return Ok(TaskResult::new(
                    Some(output),
                    NextAction::ContinueAndExecute,
                ));
            }
        };

        // Parse response
        match self.parse_response(&response) {
            Some(commands) => {
                state.git_commands = Some(commands);
            }
            None => {
                state
                    .errors
                    .push("Failed to generate git commands".to_string());
                // Store raw response for debugging
                state.git_commands = Some(format!(
                    "# Raw response (parsing failed):\n# {}",
                    response.replace('\n', "\n# ")
                ));
            }
        }

        // Update state
        context.set(STATE_KEY, state).await;

        let output = "Generated git commands".to_string();
        self.executor.emit_step_completed(
            "planner",
            Some(&output),
            start_time.elapsed().as_millis() as u64,
        );

        Ok(TaskResult::new(
            Some(output),
            NextAction::ContinueAndExecute,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::super::state::CommitPlan;
    use super::*;

    struct MockExecutor {
        response: String,
    }

    #[async_trait]
    impl WorkflowLlmExecutor for MockExecutor {
        async fn complete(
            &self,
            _system_prompt: &str,
            _user_prompt: &str,
            _context: HashMap<String, serde_json::Value>,
        ) -> anyhow::Result<String> {
            Ok(self.response.clone())
        }
    }

    #[tokio::test]
    async fn test_planner_generates_commands() {
        let executor = Arc::new(MockExecutor {
            response: r#"```bash
# Commit 1: feat: add authentication
git add src/auth.rs
git commit -m "feat: add authentication"

# Commit 2: test: add auth tests
git add tests/auth_test.rs
git commit -m "test: add auth tests"
```"#
                .to_string(),
        });

        let task = PlannerTask::new(executor);
        let context = Context::new();

        // Set up state with commit plans
        let state = GitCommitState {
            commit_plans: vec![
                CommitPlan {
                    message: "feat: add authentication".to_string(),
                    files: vec!["src/auth.rs".to_string()],
                    order: 1,
                },
                CommitPlan {
                    message: "test: add auth tests".to_string(),
                    files: vec!["tests/auth_test.rs".to_string()],
                    order: 2,
                },
            ],
            ..Default::default()
        };
        context.set(STATE_KEY, state).await;

        let result = task.run(context.clone()).await.unwrap();

        assert!(result.response.is_some());
        assert!(matches!(result.next_action, NextAction::ContinueAndExecute));

        let updated_state: GitCommitState = context.get(STATE_KEY).await.unwrap();
        assert!(updated_state.git_commands.is_some());
        let commands = updated_state.git_commands.unwrap();
        assert!(commands.contains("git add src/auth.rs"));
        assert!(commands.contains("git commit -m"));
    }

    #[test]
    fn test_parse_bash_code_block() {
        let executor = Arc::new(MockExecutor {
            response: String::new(),
        });
        let task = PlannerTask::new(executor);

        let response =
            "Here are the commands:\n```bash\ngit add file.rs\ngit commit -m \"test\"\n```";
        let parsed = task.parse_response(response);

        assert!(parsed.is_some());
        let commands = parsed.unwrap();
        assert!(commands.contains("git add file.rs"));
    }
}
