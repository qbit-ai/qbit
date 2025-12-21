//! Agent test harness for running evaluations.
//!
//! Provides `EvalRunner` which manages testbed setup, agent invocation,
//! and result capture.

use std::path::PathBuf;

use anyhow::Result;
use tempfile::TempDir;

/// Output captured from an agent run.
#[derive(Debug, Clone)]
pub struct AgentOutput {
    /// Final text response from the agent.
    pub response: String,
    /// Tool calls made during execution.
    pub tool_calls: Vec<ToolCall>,
    /// Files that were modified.
    pub files_modified: Vec<PathBuf>,
    /// Execution duration in milliseconds.
    pub duration_ms: u64,
}

/// A tool call made by the agent.
#[derive(Debug, Clone)]
pub struct ToolCall {
    /// Name of the tool.
    pub name: String,
    /// Input provided to the tool.
    pub input: serde_json::Value,
    /// Output from the tool.
    pub output: Option<String>,
    /// Whether the tool succeeded.
    pub success: bool,
}

/// Configuration for an eval run.
#[derive(Debug, Clone)]
pub struct EvalConfig {
    /// Model to use for the agent.
    pub model: String,
    /// Timeout in seconds.
    pub timeout_secs: u64,
    /// Whether to auto-approve tool calls.
    pub auto_approve: bool,
}

impl Default for EvalConfig {
    fn default() -> Self {
        Self {
            model: "claude-sonnet-4-20250514".to_string(),
            timeout_secs: 120,
            auto_approve: true,
        }
    }
}

/// Test harness for running agent evaluations.
pub struct EvalRunner {
    /// Temporary directory for the testbed.
    workspace: TempDir,
    /// Configuration for the run.
    #[allow(dead_code)]
    config: EvalConfig,
}

impl EvalRunner {
    /// Create a new eval runner with default config.
    pub fn new() -> Result<Self> {
        Self::with_config(EvalConfig::default())
    }

    /// Create a new eval runner with custom config.
    pub fn with_config(config: EvalConfig) -> Result<Self> {
        let workspace = TempDir::new()?;
        Ok(Self { workspace, config })
    }

    /// Get the workspace path.
    pub fn workspace_path(&self) -> PathBuf {
        self.workspace.path().to_path_buf()
    }

    /// Copy a testbed to the workspace.
    ///
    /// # Arguments
    /// * `testbed_name` - Name of the testbed (e.g., "rust-bug-fix")
    pub async fn setup_testbed(&self, testbed_name: &str) -> Result<PathBuf> {
        let testbed_path = self.workspace.path().join(testbed_name);

        // Get embedded testbed content
        let content = get_testbed_content(testbed_name)?;

        // Write files to workspace
        for (relative_path, file_content) in content {
            let full_path = testbed_path.join(&relative_path);
            if let Some(parent) = full_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(&full_path, file_content)?;
        }

        Ok(testbed_path)
    }

    /// Run a prompt against the agent in the current workspace.
    ///
    /// Uses the lightweight eval executor with Vertex Claude Haiku.
    pub async fn run_prompt(&self, prompt: &str) -> Result<AgentOutput> {
        let workspace = self.workspace_path();
        super::executor::execute_eval_prompt(&workspace, prompt).await
    }

    /// Clean up the workspace.
    pub fn cleanup(self) -> Result<()> {
        // TempDir handles cleanup on drop
        Ok(())
    }
}

/// Get embedded testbed content.
fn get_testbed_content(name: &str) -> Result<Vec<(String, String)>> {
    use crate::evals::scenarios;

    match name {
        "rust-bug-fix" => Ok(scenarios::bug_fix::testbed_files()),
        "rust-feature" => Ok(scenarios::feature_impl::testbed_files()),
        "rust-refactor" => Ok(scenarios::refactor::testbed_files()),
        "rust-understanding" => Ok(scenarios::code_understanding::testbed_files()),
        "rust-multi-step" => Ok(scenarios::multi_step::testbed_files()),
        _ => anyhow::bail!("Unknown testbed: {}", name),
    }
}
