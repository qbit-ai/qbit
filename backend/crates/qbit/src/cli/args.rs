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

    /// LLM provider for evals (default: vertex-claude)
    ///
    /// Options: vertex-claude, zai, openai
    #[cfg(feature = "evals")]
    #[arg(long, help = "LLM provider for evals (vertex-claude, zai, openai)")]
    pub eval_provider: Option<String>,

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

    /// Save eval results to a JSON file
    #[cfg(feature = "evals")]
    #[arg(long, help = "Save eval results to a JSON file")]
    pub output: Option<PathBuf>,

    /// Pretty print eval results summary (CI-friendly format)
    #[cfg(feature = "evals")]
    #[arg(long, help = "Pretty print eval results summary")]
    pub pretty: bool,
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
