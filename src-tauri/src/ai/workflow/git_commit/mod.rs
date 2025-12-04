//! Git commit workflow implementation.
//!
//! A multi-agent workflow that:
//! 1. Analyzes git status and diff output
//! 2. Organizes changes into logical commits
//! 3. Generates git commands for each commit
//!
//! This workflow uses graph-flow for orchestration and supports
//! step-by-step execution with human-in-the-loop capabilities.

mod analyzer;
mod organizer;
mod planner;

pub use analyzer::AnalyzerTask;
pub use organizer::OrganizerTask;
pub use planner::PlannerTask;

use std::sync::Arc;

use async_trait::async_trait;
use graph_flow::{Context, GraphBuilder, NextAction, Task, TaskResult};

use super::models::{GitCommitState, WorkflowStage};
use super::WorkflowLlmExecutor;

/// State key for storing GitCommitState in graph-flow Context.
pub const STATE_KEY: &str = "git_commit_state";

/// Initialize task - sets up the workflow state.
pub struct InitializeTask;

#[async_trait]
impl Task for InitializeTask {
    fn id(&self) -> &str {
        "initialize"
    }

    async fn run(&self, context: Context) -> graph_flow::Result<TaskResult> {
        // Get initial input from context
        let git_status: Option<String> = context.get("git_status_input").await;
        let git_diff: Option<String> = context.get("git_diff_input").await;

        // Create initial state
        let state = GitCommitState {
            git_status,
            git_diff,
            file_changes: vec![],
            commit_plans: vec![],
            git_commands: None,
            errors: vec![],
            stage: WorkflowStage::Initialized,
        };

        // Store state in context
        context.set(STATE_KEY, state).await;

        Ok(TaskResult::new(
            Some("Workflow initialized".to_string()),
            NextAction::ContinueAndExecute,
        ))
    }
}

/// Formatter task - formats the final output.
pub struct FormatterTask;

#[async_trait]
impl Task for FormatterTask {
    fn id(&self) -> &str {
        "formatter"
    }

    async fn run(&self, context: Context) -> graph_flow::Result<TaskResult> {
        let state: GitCommitState = context
            .get(STATE_KEY)
            .await
            .unwrap_or_default();

        // Format the output
        let output = if state.errors.is_empty() {
            if let Some(ref commands) = state.git_commands {
                format!(
                    "## Git Commit Plan\n\n{} commits planned:\n\n{}\n\n### Commands\n```bash\n{}\n```",
                    state.commit_plans.len(),
                    state
                        .commit_plans
                        .iter()
                        .enumerate()
                        .map(|(i, plan)| format!(
                            "{}. **{}**\n   Files: {}",
                            i + 1,
                            plan.message,
                            plan.files.join(", ")
                        ))
                        .collect::<Vec<_>>()
                        .join("\n"),
                    commands
                )
            } else {
                "No git commands generated".to_string()
            }
        } else {
            format!(
                "## Errors\n\n{}",
                state.errors.join("\n")
            )
        };

        // Update state to completed
        let mut final_state = state;
        final_state.stage = WorkflowStage::Completed;
        context.set(STATE_KEY, final_state).await;

        Ok(TaskResult::new(Some(output), NextAction::End))
    }
}

/// Create the git commit workflow graph.
///
/// Graph structure:
/// ```text
/// Initialize -> Analyzer -> Organizer -> Planner -> Formatter
/// ```
pub fn create_git_commit_workflow(
    executor: Arc<dyn WorkflowLlmExecutor>,
) -> Arc<graph_flow::Graph> {
    let initialize = Arc::new(InitializeTask);
    let analyzer = Arc::new(AnalyzerTask::new(executor.clone()));
    let organizer = Arc::new(OrganizerTask::new(executor.clone()));
    let planner = Arc::new(PlannerTask::new(executor));
    let formatter = Arc::new(FormatterTask);

    let graph = GraphBuilder::new("git_commit")
        .add_task(initialize.clone())
        .add_task(analyzer.clone())
        .add_task(organizer.clone())
        .add_task(planner.clone())
        .add_task(formatter.clone())
        .add_edge(initialize.id(), analyzer.id())
        .add_edge(analyzer.id(), organizer.id())
        .add_edge(organizer.id(), planner.id())
        .add_edge(planner.id(), formatter.id())
        .build();

    Arc::new(graph)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    struct MockExecutor;

    #[async_trait]
    impl WorkflowLlmExecutor for MockExecutor {
        async fn complete(
            &self,
            _system_prompt: &str,
            _user_prompt: &str,
            _context: HashMap<String, serde_json::Value>,
        ) -> anyhow::Result<String> {
            Ok("Mock response".to_string())
        }
    }

    #[tokio::test]
    async fn test_initialize_task() {
        let task = InitializeTask;
        let context = Context::new();

        context.set("git_status_input", "M  file.txt".to_string()).await;
        context.set("git_diff_input", "diff content".to_string()).await;

        let result = task.run(context.clone()).await.unwrap();

        assert!(result.response.is_some());
        assert!(matches!(result.next_action, NextAction::ContinueAndExecute));

        let state: GitCommitState = context.get(STATE_KEY).await.unwrap();
        assert_eq!(state.stage, WorkflowStage::Initialized);
    }

    #[tokio::test]
    async fn test_workflow_graph_creation() {
        let executor = Arc::new(MockExecutor);
        let graph = create_git_commit_workflow(executor);

        // Verify graph was created
        assert!(graph.has_task("initialize"));
        assert!(graph.has_task("analyzer"));
        assert!(graph.has_task("organizer"));
        assert!(graph.has_task("planner"));
        assert!(graph.has_task("formatter"));
    }
}
