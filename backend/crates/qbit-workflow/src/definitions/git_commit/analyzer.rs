//! Analyzer task for the git commit workflow.
//!
//! Analyzes git status and diff output to categorize file changes.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use graph_flow::{Context, NextAction, Task, TaskResult};

use super::state::{FileChange, FileStatus, GitCommitState, WorkflowStage};
use super::STATE_KEY;
use crate::models::WorkflowLlmExecutor;

/// System prompt for the analyzer agent.
const ANALYZER_SYSTEM_PROMPT: &str = r#"You are a git change analyzer. Your task is to analyze git status and diff output to categorize file changes.

For each file, determine:
1. The file path
2. The status (Added, Modified, Deleted, Renamed, Untracked)
3. A category (e.g., "feature", "bugfix", "refactor", "docs", "test", "config", "deps")
4. A brief summary of what changed (from the diff)

Output your analysis as JSON in this exact format:
```json
{
  "file_changes": [
    {
      "path": "src/main.rs",
      "status": "Modified",
      "category": "feature",
      "diff_summary": "Added new error handling for database connections"
    }
  ]
}
```

Be precise and concise. Focus on the semantic meaning of changes, not just the file names."#;

/// Analyzes git status and diff to categorize file changes.
pub struct AnalyzerTask {
    executor: Arc<dyn WorkflowLlmExecutor>,
}

impl AnalyzerTask {
    /// Create a new analyzer task with the given LLM executor.
    pub fn new(executor: Arc<dyn WorkflowLlmExecutor>) -> Self {
        Self { executor }
    }

    /// Parse the LLM response into file changes.
    fn parse_response(&self, response: &str) -> Vec<FileChange> {
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
        struct AnalyzerResponse {
            file_changes: Vec<FileChangeJson>,
        }

        #[derive(serde::Deserialize)]
        struct FileChangeJson {
            path: String,
            status: String,
            category: Option<String>,
            diff_summary: Option<String>,
        }

        match serde_json::from_str::<AnalyzerResponse>(json_str.trim()) {
            Ok(parsed) => parsed
                .file_changes
                .into_iter()
                .map(|fc| FileChange {
                    path: fc.path,
                    status: match fc.status.to_lowercase().as_str() {
                        "added" | "a" => FileStatus::Added,
                        "modified" | "m" => FileStatus::Modified,
                        "deleted" | "d" => FileStatus::Deleted,
                        "renamed" | "r" => FileStatus::Renamed,
                        _ => FileStatus::Untracked,
                    },
                    category: fc.category,
                    diff_summary: fc.diff_summary,
                })
                .collect(),
            Err(e) => {
                tracing::warn!("Failed to parse analyzer response: {}", e);
                vec![]
            }
        }
    }
}

#[async_trait]
impl Task for AnalyzerTask {
    fn id(&self) -> &str {
        "analyzer"
    }

    async fn run(&self, context: Context) -> graph_flow::Result<TaskResult> {
        let start_time = std::time::Instant::now();

        // Emit step started event
        self.executor.emit_step_started("analyzer", 1, 4);

        // Get current state
        let mut state: GitCommitState = context.get(STATE_KEY).await.unwrap_or_default();

        // Update stage
        state.stage = WorkflowStage::Analyzing;
        context.set(STATE_KEY, state.clone()).await;

        // Build user prompt
        let user_prompt = format!(
            "Analyze the following git changes:\n\n## Git Status\n```\n{}\n```\n\n## Git Diff\n```\n{}\n```",
            state.git_status.as_deref().unwrap_or("No status available"),
            state.git_diff.as_deref().unwrap_or("No diff available")
        );

        // Call LLM
        let context_vars: HashMap<String, serde_json::Value> = HashMap::new();
        let response = match self
            .executor
            .complete(ANALYZER_SYSTEM_PROMPT, &user_prompt, context_vars)
            .await
        {
            Ok(r) => r,
            Err(e) => {
                state.errors.push(format!("Analyzer error: {}", e));
                context.set(STATE_KEY, state).await;
                let output = format!("Analysis failed: {}", e);
                self.executor.emit_step_completed(
                    "analyzer",
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
        let file_changes = self.parse_response(&response);

        if file_changes.is_empty() {
            state
                .errors
                .push("No file changes detected or failed to parse analysis".to_string());
        } else {
            state.file_changes = file_changes;
        }

        // Update state
        context.set(STATE_KEY, state.clone()).await;

        let output = format!("Analyzed {} file changes", state.file_changes.len());
        self.executor.emit_step_completed(
            "analyzer",
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
    use super::*;
    use std::sync::Mutex;

    /// Tracks step events emitted during test execution.
    #[derive(Debug, Default)]
    struct StepEventTracker {
        started: Vec<(String, usize, usize)>,
        completed: Vec<(String, Option<String>, u64)>,
    }

    struct MockExecutor {
        response: String,
        events: Arc<Mutex<StepEventTracker>>,
    }

    impl MockExecutor {
        fn new(response: &str) -> Self {
            Self {
                response: response.to_string(),
                events: Arc::new(Mutex::new(StepEventTracker::default())),
            }
        }

        fn with_events(response: &str, events: Arc<Mutex<StepEventTracker>>) -> Self {
            Self {
                response: response.to_string(),
                events,
            }
        }
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

        fn emit_step_started(&self, step_name: &str, step_index: usize, total_steps: usize) {
            let mut events = self.events.lock().unwrap();
            events
                .started
                .push((step_name.to_string(), step_index, total_steps));
        }

        fn emit_step_completed(&self, step_name: &str, output: Option<&str>, duration_ms: u64) {
            let mut events = self.events.lock().unwrap();
            events.completed.push((
                step_name.to_string(),
                output.map(|s| s.to_string()),
                duration_ms,
            ));
        }
    }

    #[tokio::test]
    async fn test_analyzer_parses_json_response() {
        let executor = Arc::new(MockExecutor::new(
            r#"```json
{
  "file_changes": [
    {
      "path": "src/main.rs",
      "status": "Modified",
      "category": "feature",
      "diff_summary": "Added error handling"
    }
  ]
}
```"#,
        ));

        let task = AnalyzerTask::new(executor);
        let context = Context::new();

        // Set up initial state
        let state = GitCommitState {
            git_status: Some("M  src/main.rs".to_string()),
            git_diff: Some("diff content".to_string()),
            ..Default::default()
        };
        context.set(STATE_KEY, state).await;

        let result = task.run(context.clone()).await.unwrap();

        assert!(result.response.is_some());
        assert!(matches!(result.next_action, NextAction::ContinueAndExecute));

        let updated_state: GitCommitState = context.get(STATE_KEY).await.unwrap();
        assert_eq!(updated_state.file_changes.len(), 1);
        assert_eq!(updated_state.file_changes[0].path, "src/main.rs");
        assert_eq!(updated_state.file_changes[0].status, FileStatus::Modified);
    }

    #[tokio::test]
    async fn test_analyzer_emits_step_events() {
        let events = Arc::new(Mutex::new(StepEventTracker::default()));
        let executor = Arc::new(MockExecutor::with_events(
            r#"```json
{
  "file_changes": [
    {
      "path": "src/main.rs",
      "status": "Modified",
      "category": "feature",
      "diff_summary": "Added error handling"
    }
  ]
}
```"#,
            events.clone(),
        ));

        let task = AnalyzerTask::new(executor);
        let context = Context::new();

        // Set up initial state
        let state = GitCommitState {
            git_status: Some("M  src/main.rs".to_string()),
            git_diff: Some("diff content".to_string()),
            ..Default::default()
        };
        context.set(STATE_KEY, state).await;

        let _result = task.run(context.clone()).await.unwrap();

        // Verify step events
        let events = events.lock().unwrap();

        assert_eq!(
            events.started.len(),
            1,
            "Expected exactly one step_started event"
        );
        assert_eq!(
            events.started[0].0, "analyzer",
            "Step name should be 'analyzer'"
        );
        assert_eq!(
            events.started[0].1, 1,
            "Step index should be 1 (second step)"
        );
        assert_eq!(events.started[0].2, 4, "Total steps should be 4");

        assert_eq!(
            events.completed.len(),
            1,
            "Expected exactly one step_completed event"
        );
        assert_eq!(
            events.completed[0].0, "analyzer",
            "Step name should be 'analyzer'"
        );
        assert!(events.completed[0].1.is_some(), "Output should be present");
    }
}
