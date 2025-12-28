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
    /// Total tokens used (input + output).
    pub tokens_used: Option<u32>,
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
pub struct EvalRunConfig {
    /// Model to use for the agent.
    pub model: String,
    /// Timeout in seconds.
    pub timeout_secs: u64,
    /// Whether to auto-approve tool calls.
    pub auto_approve: bool,
}

impl Default for EvalRunConfig {
    fn default() -> Self {
        Self {
            model: "claude-haiku-4-5@20251001".to_string(),
            timeout_secs: 120,
            auto_approve: true,
        }
    }
}

/// Verbose output configuration.
#[derive(Debug, Clone, Default)]
pub struct VerboseConfig {
    /// Whether verbose output is enabled.
    pub enabled: bool,
    /// Optional file path to write verbose output to (instead of stdout).
    pub log_file: Option<PathBuf>,
}

impl VerboseConfig {
    /// Create a config for stdout verbose output.
    pub fn stdout() -> Self {
        Self {
            enabled: true,
            log_file: None,
        }
    }

    /// Create a config for file-based verbose output.
    pub fn to_file(path: PathBuf) -> Self {
        Self {
            enabled: true,
            log_file: Some(path),
        }
    }
}

/// Test harness for running agent evaluations.
pub struct EvalRunner {
    /// Temporary directory for the testbed.
    workspace: TempDir,
    /// Configuration for the run.
    #[allow(dead_code)]
    config: EvalRunConfig,
    /// Verbose output configuration.
    verbose_config: VerboseConfig,
}

impl EvalRunner {
    /// Create a new eval runner with default config.
    pub fn new() -> Result<Self> {
        Self::with_config(EvalRunConfig::default(), VerboseConfig::default())
    }

    /// Create a new eval runner with verbose output to stdout.
    pub fn new_verbose(verbose: bool) -> Result<Self> {
        let verbose_config = if verbose {
            VerboseConfig::stdout()
        } else {
            VerboseConfig::default()
        };
        Self::with_config(EvalRunConfig::default(), verbose_config)
    }

    /// Create a new eval runner with verbose output to a file.
    pub fn new_with_log_file(log_file: PathBuf) -> Result<Self> {
        Self::with_config(EvalRunConfig::default(), VerboseConfig::to_file(log_file))
    }

    /// Create a new eval runner with custom config.
    pub fn with_config(config: EvalRunConfig, verbose_config: VerboseConfig) -> Result<Self> {
        let workspace = TempDir::new()?;
        Ok(Self {
            workspace,
            config,
            verbose_config,
        })
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

    /// Run a prompt against the agent in the specified workspace.
    ///
    /// Uses the lightweight eval executor with Vertex Claude Haiku.
    ///
    /// # Arguments
    /// * `workspace` - The workspace directory where the agent should operate
    /// * `prompt` - The prompt to give to the agent
    pub async fn run_prompt(
        &self,
        workspace: &std::path::Path,
        prompt: &str,
    ) -> Result<AgentOutput> {
        crate::executor::execute_eval_prompt(workspace, prompt, &self.verbose_config).await
    }

    /// Clean up the workspace.
    pub fn cleanup(self) -> Result<()> {
        // TempDir handles cleanup on drop
        Ok(())
    }
}

/// Get embedded testbed content.
fn get_testbed_content(name: &str) -> Result<Vec<(String, String)>> {
    use crate::scenarios;

    match name {
        "rust-bug-fix" => Ok(scenarios::bug_fix::testbed_files()),
        "rust-feature" => Ok(scenarios::feature_impl::testbed_files()),
        "rust-refactor" => Ok(scenarios::refactor::testbed_files()),
        "rust-understanding" => Ok(scenarios::code_understanding::testbed_files()),
        "rust-multi-step" => Ok(scenarios::multi_step::testbed_files()),
        _ => anyhow::bail!("Unknown testbed: {}", name),
    }
}
