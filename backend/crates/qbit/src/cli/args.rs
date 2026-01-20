//! CLI argument parsing using clap.
//!
//! Defines the command-line interface for qbit-cli.

use clap::Parser;
use std::path::PathBuf;

/// Qbit CLI - Headless interface for the Qbit AI agent
#[derive(Parser, Debug, Clone)]
#[command(name = "qbit-cli")]
#[command(version, about, long_about = None)]
pub struct Args {
    /// Working directory (default: current directory)
    #[arg(default_value = ".")]
    pub workspace: PathBuf,

    /// Execute a single prompt and exit
    #[arg(short = 'e', long, conflicts_with = "file")]
    pub execute: Option<String>,

    /// Execute prompts from a file (one per line) and exit
    #[arg(short = 'f', long, conflicts_with = "execute")]
    pub file: Option<PathBuf>,

    /// Override AI provider from settings
    ///
    /// Options: vertex_ai, openrouter, anthropic, openai
    #[arg(short = 'p', long)]
    pub provider: Option<String>,

    /// Override model from settings
    #[arg(short = 'm', long)]
    pub model: Option<String>,

    /// API key (overrides settings and env vars)
    #[arg(long, env = "QBIT_API_KEY")]
    pub api_key: Option<String>,

    /// Auto-approve all tool calls (DANGEROUS: for testing only)
    #[arg(long)]
    pub auto_approve: bool,

    /// Output events as JSON lines (for scripting/parsing)
    #[arg(long)]
    pub json: bool,

    /// Only output final response (suppress streaming)
    #[arg(long, short = 'q')]
    pub quiet: bool,

    /// Show verbose output (debug information)
    #[arg(short = 'v', long)]
    pub verbose: bool,

    /// Run evaluation scenarios
    #[cfg(feature = "evals")]
    #[arg(long, help = "Run evaluation scenarios")]
    pub eval: bool,

    /// Filter to specific scenario (e.g., "bug-fix")
    #[cfg(feature = "evals")]
    #[arg(long, help = "Run only this scenario")]
    pub scenario: Option<String>,

    /// List available scenarios
    #[cfg(feature = "evals")]
    #[arg(long, help = "List available scenarios")]
    pub list_scenarios: bool,

    /// Run eval scenarios in parallel (faster but interleaved output)
    #[cfg(feature = "evals")]
    #[arg(long, help = "Run scenarios in parallel")]
    pub parallel: bool,

    /// Maximum number of concurrent scenarios when running in parallel
    ///
    /// Limits resource usage (API rate limits, Docker containers, memory).
    /// Default: 4. Only applies when --parallel is used.
    #[cfg(feature = "evals")]
    #[arg(long, default_value = "4", help = "Max concurrent scenarios (default: 4)")]
    pub concurrency: usize,

    /// LLM provider for evals (default: vertex-claude)
    ///
    /// Options: vertex-claude, zai, openai
    #[cfg(feature = "evals")]
    #[arg(long, help = "LLM provider for evals (vertex-claude, zai, openai)")]
    pub eval_provider: Option<String>,

    /// Model to use for evals (overrides provider default)
    #[cfg(feature = "evals")]
    #[arg(long, help = "Model to use for evals (e.g., claude-sonnet-4-20250514)")]
    pub eval_model: Option<String>,

    /// Run OpenAI model connectivity tests
    ///
    /// Tests each OpenAI model with a simple hello world prompt
    /// to verify configuration and connectivity.
    #[cfg(feature = "evals")]
    #[arg(long, help = "Run OpenAI model connectivity tests")]
    pub openai_models: bool,

    /// Filter to specific OpenAI model (e.g., "gpt-5.1")
    #[cfg(feature = "evals")]
    #[arg(long, help = "Test only this OpenAI model")]
    pub openai_model: Option<String>,

    /// Run a benchmark suite (e.g., "humaneval")
    #[cfg(feature = "evals")]
    #[arg(long, help = "Run a benchmark suite (humaneval)")]
    pub benchmark: Option<String>,

    /// Filter to specific benchmark problems (e.g., "0-10" or "0,5,10")
    #[cfg(feature = "evals")]
    #[arg(long, help = "Filter to specific problems (e.g., 0-10)")]
    pub problems: Option<String>,

    /// List available benchmarks
    #[cfg(feature = "evals")]
    #[arg(long, help = "List available benchmarks")]
    pub list_benchmarks: bool,

    /// Run SWE-bench Lite benchmark (300 real GitHub issues)
    #[cfg(feature = "evals")]
    #[arg(long, help = "Run SWE-bench Lite benchmark")]
    pub swebench: bool,

    /// Filter to specific SWE-bench instance (e.g., "django__django-11133")
    #[cfg(feature = "evals")]
    #[arg(long, help = "Run specific SWE-bench instance")]
    pub instance: Option<String>,

    /// Use a persistent workspace directory instead of temp (for debugging)
    ///
    /// If the directory exists, it will be reused. This allows running tests
    /// separately from the agent with --test-only.
    #[cfg(feature = "evals")]
    #[arg(long, help = "Use persistent workspace directory")]
    pub workspace_dir: Option<PathBuf>,

    /// Skip agent execution, only run Docker tests on existing workspace
    ///
    /// Use with --workspace-dir to test changes to Docker execution without
    /// re-running the expensive agent step.
    #[cfg(feature = "evals")]
    #[arg(long, help = "Skip agent, run tests only (requires --workspace-dir)")]
    pub test_only: bool,

    /// Save detailed results for each instance to a directory
    ///
    /// Creates one JSON file per instance containing full transcript,
    /// test output, and metrics. Useful for post-hoc analysis.
    #[cfg(feature = "evals")]
    #[arg(long, help = "Save per-instance results to directory")]
    pub results_dir: Option<PathBuf>,

    /// Save eval results to a JSON file
    #[cfg(feature = "evals")]
    #[arg(long, help = "Save eval results to a JSON file")]
    pub output: Option<PathBuf>,

    /// Pretty print eval results summary (CI-friendly format)
    #[cfg(feature = "evals")]
    #[arg(long, help = "Pretty print eval results summary")]
    pub pretty: bool,

    /// Print the full agent transcript before results
    #[cfg(feature = "evals")]
    #[arg(long, help = "Print the full agent transcript before results")]
    pub transcript: bool,
}

impl Args {
    /// Resolve the workspace path to an absolute path.
    ///
    /// Priority:
    /// 1. QBIT_WORKSPACE environment variable (if set)
    /// 2. CLI argument (defaults to ".")
    ///
    /// Returns an error if the path does not exist or is not a directory.
    pub fn resolve_workspace(&self) -> anyhow::Result<PathBuf> {
        // Check QBIT_WORKSPACE env var first
        let workspace_path = if let Ok(env_workspace) = std::env::var("QBIT_WORKSPACE") {
            PathBuf::from(env_workspace)
        } else {
            self.workspace.clone()
        };

        let canonical = workspace_path.canonicalize().map_err(|e| {
            anyhow::anyhow!(
                "Workspace '{}' does not exist or is not accessible: {}",
                workspace_path.display(),
                e
            )
        })?;

        if !canonical.is_dir() {
            anyhow::bail!("Workspace '{}' is not a directory", canonical.display());
        }

        Ok(canonical)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_args_default_values() {
        let args = Args::parse_from(["qbit-cli"]);
        assert_eq!(args.workspace, PathBuf::from("."));
        assert!(!args.auto_approve);
        assert!(!args.json);
        assert!(!args.quiet);
        assert!(!args.verbose);
    }

    #[test]
    fn test_args_execute_flag() {
        let args = Args::parse_from(["qbit-cli", "-e", "Hello world"]);
        assert_eq!(args.execute, Some("Hello world".to_string()));
    }

    #[test]
    fn test_args_provider_and_model() {
        let args = Args::parse_from([
            "qbit-cli",
            "-p",
            "openrouter",
            "-m",
            "anthropic/claude-sonnet-4",
        ]);
        assert_eq!(args.provider, Some("openrouter".to_string()));
        assert_eq!(args.model, Some("anthropic/claude-sonnet-4".to_string()));
    }

    #[test]
    fn test_args_output_modes() {
        let args = Args::parse_from(["qbit-cli", "--json", "--quiet"]);
        assert!(args.json);
        assert!(args.quiet);
    }

    #[test]
    fn test_args_auto_approve() {
        let args = Args::parse_from(["qbit-cli", "--auto-approve"]);
        assert!(args.auto_approve);
    }
}
