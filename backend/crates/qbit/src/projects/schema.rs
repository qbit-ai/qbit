//! Project configuration schema.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Configuration for a single project/codebase.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectConfig {
    /// Display name for the project
    pub name: String,

    /// Root path to the main project directory
    pub root_path: PathBuf,

    /// Optional directory where git worktrees are stored
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub worktrees_dir: Option<PathBuf>,

    /// Shell commands for common operations
    #[serde(default)]
    pub commands: ProjectCommands,

    /// Worktree initialization configuration
    #[serde(default)]
    pub worktree: WorktreeConfig,
}

/// Shell commands for common project operations.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProjectCommands {
    /// Command to run tests
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub test: Option<String>,

    /// Command to run linting
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lint: Option<String>,

    /// Command to build the project
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub build: Option<String>,

    /// Command to start the project
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start: Option<String>,
}

/// Configuration for worktree initialization.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WorktreeConfig {
    /// Script to run when initializing a new worktree (one command per line)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub init_script: Option<String>,
}
