//! Gatherer task for the git commit workflow.
//!
//! This task runs git commands to gather the current repository state
//! without requiring any user input.

use std::sync::Arc;

use async_trait::async_trait;
use graph_flow::{Context, NextAction, Task, TaskResult};

use super::state::{GitCommitState, WorkflowStage};
use super::STATE_KEY;
use crate::workflow::models::{WorkflowAgentConfig, WorkflowLlmExecutor};

/// System prompt for the gatherer agent.
const GATHERER_SYSTEM_PROMPT: &str = r#"You are a git data gatherer. Your task is to run git commands to collect information about the current repository state.

You have access to the `run_pty_cmd` tool to execute shell commands.

Your task is to:
1. Run `git status` to see what files have changed
2. Run `git diff` to see the actual changes (for staged files)
3. Run `git diff --cached` to see staged changes
4. Optionally run `git log -3 --oneline` to see recent commits for context

After gathering this information, respond with a JSON summary in this format:
```json
{
  "git_status": "output from git status",
  "git_diff": "combined output from git diff and git diff --cached"
}
```

Be thorough but efficient. Only gather the information needed to understand what needs to be committed."#;

/// Gathers git repository state by running commands.
pub struct GathererTask {
    executor: Arc<dyn WorkflowLlmExecutor>,
}

impl GathererTask {
    /// Create a new gatherer task with the given LLM executor.
    pub fn new(executor: Arc<dyn WorkflowLlmExecutor>) -> Self {
        Self { executor }
    }

    /// Parse the gatherer response to extract git data.
    fn parse_response(&self, response: &str) -> (Option<String>, Option<String>) {
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

        #[derive(serde::Deserialize)]
        struct GathererResponse {
            git_status: Option<String>,
            git_diff: Option<String>,
        }

        match serde_json::from_str::<GathererResponse>(json_str.trim()) {
            Ok(parsed) => (parsed.git_status, parsed.git_diff),
            Err(e) => {
                tracing::warn!("Failed to parse gatherer response: {}", e);
                // Try to extract raw output if JSON parsing fails
                (None, None)
            }
        }
    }
}

#[async_trait]
impl Task for GathererTask {
    fn id(&self) -> &str {
        "gatherer"
    }

    async fn run(&self, context: Context) -> graph_flow::Result<TaskResult> {
        let start_time = std::time::Instant::now();

        // Get current state (or create default)
        let mut state: GitCommitState = context.get(STATE_KEY).await.unwrap_or_default();

        // If we already have git data (from input), skip gathering
        // Note: We manually emit step events here since run_agent won't be called
        if state.git_status.is_some() && state.git_diff.is_some() {
            tracing::debug!("Git data already available, skipping gathering");
            self.executor.emit_step_started("gatherer", 0, 4);
            let output = "Git data already available".to_string();
            self.executor.emit_step_completed(
                "gatherer",
                Some(&output),
                start_time.elapsed().as_millis() as u64,
            );
            return Ok(TaskResult::new(
                Some(output),
                NextAction::ContinueAndExecute,
            ));
        }

        // Update stage
        state.stage = WorkflowStage::Initialized;
        context.set(STATE_KEY, state.clone()).await;

        // Configure the agent to gather git data
        // emit_events=true enables both step events AND tool events from run_agent
        let config = WorkflowAgentConfig::new(
            GATHERER_SYSTEM_PROMPT,
            "Gather the current git repository state by running git status and git diff commands. \
             Report the results in JSON format.",
        )
        .with_tools(vec!["run_pty_cmd"])
        .with_max_iterations(10)
        .with_emit_events(true)
        .with_step("gatherer", 0);

        // Run the agent - this handles step started/completed events automatically
        let result = match self.executor.run_agent(config).await {
            Ok(r) => r,
            Err(e) => {
                state.errors.push(format!("Gatherer error: {}", e));
                context.set(STATE_KEY, state).await;
                // Note: run_agent emits step completed on error, so no manual call needed
                return Ok(TaskResult::new(
                    Some(format!("Failed to gather git data: {}", e)),
                    NextAction::GoTo("formatter".to_string()),
                ));
            }
        };

        // Parse the response
        let (git_status, git_diff) = self.parse_response(&result.response);

        if git_status.is_none() && git_diff.is_none() {
            // If parsing failed, try to use the raw response
            // The agent might have just dumped the output
            state
                .errors
                .push("Could not parse git data from response".to_string());
        } else {
            state.git_status = git_status;
            state.git_diff = git_diff;
        }

        // Update state
        context.set(STATE_KEY, state.clone()).await;

        let status_preview = state
            .git_status
            .as_ref()
            .map(|s| s.chars().take(100).collect::<String>())
            .unwrap_or_else(|| "No status".to_string());

        // Note: run_agent already emits step completed, so no manual call needed
        Ok(TaskResult::new(
            Some(format!("Gathered git data: {status_preview}...")),
            NextAction::ContinueAndExecute,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::sync::Mutex;

    /// Tracks step events emitted during test execution.
    #[derive(Debug, Default)]
    struct StepEventTracker {
        started: Vec<(String, usize, usize)>, // (step_name, step_index, total_steps)
        completed: Vec<(String, Option<String>, u64)>, // (step_name, output, duration_ms)
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

        async fn run_agent(
            &self,
            _config: WorkflowAgentConfig,
        ) -> anyhow::Result<crate::workflow::models::WorkflowAgentResult> {
            Ok(crate::workflow::models::WorkflowAgentResult {
                response: self.response.clone(),
                tool_history: vec![],
                iterations: 1,
                tokens_used: None,
                completed: true,
                error: None,
            })
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
    async fn test_gatherer_parses_json_response() {
        let executor = Arc::new(MockExecutor::new(
            r#"```json
{
  "git_status": "M  src/main.rs",
  "git_diff": "diff content here"
}
```"#,
        ));

        let task = GathererTask::new(executor);
        let context = Context::new();

        // Set up initial empty state
        let state = GitCommitState::default();
        context.set(STATE_KEY, state).await;

        let result = task.run(context.clone()).await.unwrap();

        assert!(result.response.is_some());
        assert!(matches!(result.next_action, NextAction::ContinueAndExecute));

        let updated_state: GitCommitState = context.get(STATE_KEY).await.unwrap();
        assert!(updated_state.git_status.is_some());
        assert!(updated_state.git_diff.is_some());
    }

    #[tokio::test]
    async fn test_gatherer_skips_when_data_exists() {
        let executor = Arc::new(MockExecutor::new("should not be called"));

        let task = GathererTask::new(executor);
        let context = Context::new();

        // Set up state with existing data
        let state = GitCommitState {
            git_status: Some("M  file.rs".to_string()),
            git_diff: Some("diff content".to_string()),
            ..Default::default()
        };
        context.set(STATE_KEY, state).await;

        let result = task.run(context.clone()).await.unwrap();

        assert!(result.response.unwrap().contains("already available"));
    }

    // Note: test_gatherer_emits_step_events was removed because in the normal path,
    // step events are emitted by run_agent (which requires the real BridgeLlmExecutor).
    // The skip path test below covers the manual step event emission.

    #[tokio::test]
    async fn test_gatherer_emits_step_events_when_skipping() {
        // Create shared event tracker
        let events = Arc::new(Mutex::new(StepEventTracker::default()));
        let executor = Arc::new(MockExecutor::with_events(
            "should not be called",
            events.clone(),
        ));

        let task = GathererTask::new(executor);
        let context = Context::new();

        // Set up state with existing data (to trigger skip path)
        let state = GitCommitState {
            git_status: Some("M  file.rs".to_string()),
            git_diff: Some("diff content".to_string()),
            ..Default::default()
        };
        context.set(STATE_KEY, state).await;

        // Run the task
        let _result = task.run(context.clone()).await.unwrap();

        // Verify step events were emitted even when skipping
        let events = events.lock().unwrap();

        // Should still emit start event
        assert_eq!(
            events.started.len(),
            1,
            "Expected step_started event even when skipping"
        );
        assert_eq!(events.started[0].0, "gatherer");

        // Should still emit complete event
        assert_eq!(
            events.completed.len(),
            1,
            "Expected step_completed event even when skipping"
        );
        assert_eq!(events.completed[0].0, "gatherer");
        assert!(events.completed[0]
            .1
            .as_ref()
            .unwrap()
            .contains("already available"));
    }
}
