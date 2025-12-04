//! State types for the git commit workflow.

use serde::{Deserialize, Serialize};

/// State for the git commit workflow.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GitCommitState {
    /// Raw git status output
    pub git_status: Option<String>,
    /// Raw git diff output
    pub git_diff: Option<String>,
    /// Categorized file changes
    pub file_changes: Vec<FileChange>,
    /// Organized commit plans
    pub commit_plans: Vec<CommitPlan>,
    /// Final git commands script
    pub git_commands: Option<String>,
    /// Error messages if any
    pub errors: Vec<String>,
    /// Current workflow stage
    pub stage: WorkflowStage,
}

/// A single file change with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileChange {
    pub path: String,
    pub status: FileStatus,
    pub category: Option<String>,
    pub diff_summary: Option<String>,
}

/// Git file status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum FileStatus {
    Added,
    Modified,
    Deleted,
    Renamed,
    Untracked,
}

/// A planned commit with files and message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitPlan {
    pub message: String,
    pub files: Vec<String>,
    pub order: usize,
}

/// Workflow execution stage
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub enum WorkflowStage {
    #[default]
    Initialized,
    Analyzing,
    Organizing,
    Planning,
    Completed,
    Failed,
}

/// Input for starting a git commit workflow.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitCommitInput {
    pub git_status: String,
    pub git_diff: String,
}

/// Final result of git commit workflow
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitCommitResult {
    pub success: bool,
    pub commit_plans: Vec<CommitPlan>,
    pub git_commands: Option<String>,
    pub errors: Vec<String>,
}

impl From<GitCommitState> for GitCommitResult {
    fn from(state: GitCommitState) -> Self {
        Self {
            success: state.errors.is_empty() && state.git_commands.is_some(),
            commit_plans: state.commit_plans,
            git_commands: state.git_commands,
            errors: state.errors,
        }
    }
}
