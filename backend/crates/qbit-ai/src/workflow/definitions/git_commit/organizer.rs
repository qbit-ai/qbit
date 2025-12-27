//! Organizer task for the git commit workflow.
//!
//! Groups file changes into logical commits based on their relationships.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use graph_flow::{Context, NextAction, Task, TaskResult};

use super::state::{CommitPlan, GitCommitState, WorkflowStage};
use super::STATE_KEY;
use crate::workflow::models::WorkflowLlmExecutor;

/// System prompt for the organizer agent.
const ORGANIZER_SYSTEM_PROMPT: &str = r#"You are a git commit organizer. Your task is to group file changes into logical commits.

Guidelines for organizing commits:
1. Group related changes together (same feature, same bugfix, etc.)
2. Keep commits atomic - each commit should represent one logical change
3. Separate concerns (don't mix features with refactoring)
4. Put infrastructure/config changes in their own commits
5. Order commits logically (dependencies first, features after)

Output your organization as JSON in this exact format:
```json
{
  "commit_plans": [
    {
      "message": "feat: add user authentication",
      "files": ["src/auth.rs", "src/middleware.rs"],
      "order": 1
    },
    {
      "message": "refactor: extract common utilities",
      "files": ["src/utils.rs"],
      "order": 2
    }
  ]
}
```

Use conventional commit format for messages:
- feat: new feature
- fix: bug fix
- refactor: code refactoring
- docs: documentation
- test: tests
- chore: maintenance

Be precise and ensure all files from the input are included in exactly one commit."#;

/// Organizes file changes into logical commit groups.
pub struct OrganizerTask {
    executor: Arc<dyn WorkflowLlmExecutor>,
}

impl OrganizerTask {
    /// Create a new organizer task with the given LLM executor.
    pub fn new(executor: Arc<dyn WorkflowLlmExecutor>) -> Self {
        Self { executor }
    }

    /// Parse the LLM response into commit plans.
    fn parse_response(&self, response: &str) -> Vec<CommitPlan> {
        // Try to extract JSON from the response
        let json_str = if let Some(start) = response.find("```json") {
            let start = start + 7;
            if let Some(end) = response[start..].find("```") {
                &response[start..start + end]
            } else {
                response
            }
        } else if let Some(start) = response.find('{') {
            if let Some(end) = response.rfind('}') {
                &response[start..=end]
            } else {
                response
            }
        } else {
            response
        };

        // Parse the JSON
        #[derive(serde::Deserialize)]
        struct OrganizerResponse {
            commit_plans: Vec<CommitPlanJson>,
        }

        #[derive(serde::Deserialize)]
        struct CommitPlanJson {
            message: String,
            files: Vec<String>,
            order: usize,
        }

        match serde_json::from_str::<OrganizerResponse>(json_str.trim()) {
            Ok(parsed) => {
                let mut plans: Vec<CommitPlan> = parsed
                    .commit_plans
                    .into_iter()
                    .map(|cp| CommitPlan {
                        message: cp.message,
                        files: cp.files,
                        order: cp.order,
                    })
                    .collect();

                // Sort by order
                plans.sort_by_key(|p| p.order);
                plans
            }
            Err(e) => {
                tracing::warn!("Failed to parse organizer response: {}", e);
                vec![]
            }
        }
    }
}

#[async_trait]
impl Task for OrganizerTask {
    fn id(&self) -> &str {
        "organizer"
    }

    async fn run(&self, context: Context) -> graph_flow::Result<TaskResult> {
        let start_time = std::time::Instant::now();

        // Emit step started event
        self.executor.emit_step_started("organizer", 2, 4);

        // Get current state
        let mut state: GitCommitState = context.get(STATE_KEY).await.unwrap_or_default();

        // Check if we have file changes to organize
        if state.file_changes.is_empty() {
            state.errors.push("No file changes to organize".to_string());
            context.set(STATE_KEY, state).await;
            let output = "No file changes to organize".to_string();
            self.executor.emit_step_completed(
                "organizer",
                Some(&output),
                start_time.elapsed().as_millis() as u64,
            );
            return Ok(TaskResult::new(
                Some(output),
                NextAction::GoTo("formatter".to_string()),
            ));
        }

        // Update stage
        state.stage = WorkflowStage::Organizing;
        context.set(STATE_KEY, state.clone()).await;

        // Build user prompt with file changes
        let file_changes_str = state
            .file_changes
            .iter()
            .map(|fc| {
                format!(
                    "- {} ({:?}): {} - {}",
                    fc.path,
                    fc.status,
                    fc.category.as_deref().unwrap_or("unknown"),
                    fc.diff_summary.as_deref().unwrap_or("no summary")
                )
            })
            .collect::<Vec<_>>()
            .join("\n");

        let user_prompt = format!(
            "Organize the following file changes into logical commits:\n\n{}",
            file_changes_str
        );

        // Call LLM
        let context_vars: HashMap<String, serde_json::Value> = HashMap::new();
        let response = match self
            .executor
            .complete(ORGANIZER_SYSTEM_PROMPT, &user_prompt, context_vars)
            .await
        {
            Ok(r) => r,
            Err(e) => {
                state.errors.push(format!("Organizer error: {}", e));
                context.set(STATE_KEY, state).await;
                let output = format!("Organization failed: {}", e);
                self.executor.emit_step_completed(
                    "organizer",
                    Some(&output),
                    start_time.elapsed().as_millis() as u64,
                );
                return Ok(TaskResult::new(
                    Some(output),
                    NextAction::GoTo("formatter".to_string()),
                ));
            }
        };

        // Parse response
        let commit_plans = self.parse_response(&response);

        if commit_plans.is_empty() {
            state.errors.push("Failed to organize commits".to_string());
        } else {
            state.commit_plans = commit_plans;
        }

        // Update state
        context.set(STATE_KEY, state.clone()).await;

        let output = format!("Organized into {} commits", state.commit_plans.len());
        self.executor.emit_step_completed(
            "organizer",
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
    use super::super::state::{FileChange, FileStatus};
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
    async fn test_organizer_groups_changes() {
        let executor = Arc::new(MockExecutor {
            response: r#"```json
{
  "commit_plans": [
    {
      "message": "feat: add authentication",
      "files": ["src/auth.rs"],
      "order": 1
    },
    {
      "message": "test: add auth tests",
      "files": ["tests/auth_test.rs"],
      "order": 2
    }
  ]
}
```"#
                .to_string(),
        });

        let task = OrganizerTask::new(executor);
        let context = Context::new();

        // Set up state with file changes
        let state = GitCommitState {
            file_changes: vec![
                FileChange {
                    path: "src/auth.rs".to_string(),
                    status: FileStatus::Added,
                    category: Some("feature".to_string()),
                    diff_summary: Some("Added auth module".to_string()),
                },
                FileChange {
                    path: "tests/auth_test.rs".to_string(),
                    status: FileStatus::Added,
                    category: Some("test".to_string()),
                    diff_summary: Some("Added auth tests".to_string()),
                },
            ],
            ..Default::default()
        };
        context.set(STATE_KEY, state).await;

        let result = task.run(context.clone()).await.unwrap();

        assert!(result.response.is_some());
        assert!(matches!(result.next_action, NextAction::ContinueAndExecute));

        let updated_state: GitCommitState = context.get(STATE_KEY).await.unwrap();
        assert_eq!(updated_state.commit_plans.len(), 2);
        assert_eq!(updated_state.commit_plans[0].order, 1);
        assert_eq!(updated_state.commit_plans[1].order, 2);
    }
}
