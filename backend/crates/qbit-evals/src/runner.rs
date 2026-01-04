//! Agent test harness for running evaluations.
//!
//! Provides `EvalRunner` which manages testbed setup, agent invocation,
//! and result capture.

use std::path::PathBuf;

use anyhow::Result;
use tempfile::TempDir;

use crate::config::EvalProvider;

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
            model: "claude-sonnet-4-5@20250929".to_string(),
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
    /// LLM provider to use.
    provider: EvalProvider,
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

    /// Create a new eval runner with a specific provider.
    pub fn new_with_provider(provider: EvalProvider) -> Result<Self> {
        Self::with_config_and_provider(EvalRunConfig::default(), VerboseConfig::default(), provider)
    }

    /// Create a new eval runner with verbose output and a specific provider.
    pub fn new_verbose_with_provider(verbose: bool, provider: EvalProvider) -> Result<Self> {
        let verbose_config = if verbose {
            VerboseConfig::stdout()
        } else {
            VerboseConfig::default()
        };
        Self::with_config_and_provider(EvalRunConfig::default(), verbose_config, provider)
    }

    /// Create a new eval runner with log file and a specific provider.
    pub fn new_with_log_file_and_provider(
        log_file: PathBuf,
        provider: EvalProvider,
    ) -> Result<Self> {
        Self::with_config_and_provider(
            EvalRunConfig::default(),
            VerboseConfig::to_file(log_file),
            provider,
        )
    }

    /// Create a new eval runner with custom config (uses default provider).
    pub fn with_config(config: EvalRunConfig, verbose_config: VerboseConfig) -> Result<Self> {
        Self::with_config_and_provider(config, verbose_config, EvalProvider::default())
    }

    /// Create a new eval runner with custom config and provider.
    pub fn with_config_and_provider(
        config: EvalRunConfig,
        verbose_config: VerboseConfig,
        provider: EvalProvider,
    ) -> Result<Self> {
        let workspace = TempDir::new()?;
        Ok(Self {
            workspace,
            config,
            verbose_config,
            provider,
        })
    }

    /// Get the workspace path.
    pub fn workspace_path(&self) -> PathBuf {
        self.workspace.path().to_path_buf()
    }

    /// Get the provider being used.
    pub fn provider(&self) -> EvalProvider {
        self.provider
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
    /// Uses the configured LLM provider for execution.
    ///
    /// # Arguments
    /// * `workspace` - The workspace directory where the agent should operate
    /// * `prompt` - The prompt to give to the agent
    pub async fn run_prompt(
        &self,
        workspace: &std::path::Path,
        prompt: &str,
    ) -> Result<AgentOutput> {
        self.run_prompt_with_system(workspace, prompt, None).await
    }

    /// Run a prompt with a custom system prompt.
    ///
    /// # Arguments
    /// * `workspace` - The workspace directory where the agent should operate
    /// * `prompt` - The prompt to give to the agent
    /// * `system_prompt` - Optional custom system prompt (uses default if None)
    pub async fn run_prompt_with_system(
        &self,
        workspace: &std::path::Path,
        prompt: &str,
        system_prompt: Option<&str>,
    ) -> Result<AgentOutput> {
        crate::executor::execute_eval_prompt_with_options(
            workspace,
            prompt,
            system_prompt,
            &self.verbose_config,
            self.provider,
        )
        .await
    }

    /// Run a multi-turn conversation against the agent.
    ///
    /// This is critical for testing reasoning ID preservation across turns,
    /// which is required for OpenAI Responses API compatibility.
    ///
    /// # Arguments
    /// * `workspace` - The workspace directory where the agent should operate
    /// * `prompts` - The sequence of prompts for each turn
    pub async fn run_multi_turn(
        &self,
        workspace: &std::path::Path,
        prompts: &[&str],
    ) -> Result<crate::executor::MultiTurnAgentOutput> {
        crate::executor::execute_multi_turn_eval(
            workspace,
            prompts,
            &self.verbose_config,
            self.provider,
        )
        .await
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
        "rust-prompt-test" => Ok(scenarios::prompt_composition::testbed_files()),
        "minimal" => Ok(scenarios::web_search::testbed_files()),
        "openai-models" => Ok(scenarios::openai_models::testbed_files()),
        "empty" => Ok(scenarios::multi_turn::empty_testbed()),
        "js-ast-grep" => Ok(scenarios::ast_grep::testbed_files()),
        _ => anyhow::bail!("Unknown testbed: {}", name),
    }
}
