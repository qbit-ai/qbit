//! Workflow module for graph-flow based multi-agent orchestration.
//!
//! This module provides:
//! - Base workflow types (SubAgentTask, RouterTask, WorkflowRunner)
//! - State models for workflow graphs
//! - A registry for named workflow graphs
//! - The git_commit workflow implementation
//!
//! # Architecture
//!
//! Workflows use graph-flow for task orchestration:
//! - Each workflow is a graph of tasks
//! - Tasks communicate via shared Context
//! - Tasks can use LLM completions via WorkflowLlmExecutor
//!
//! # Example
//!
//! ```rust,ignore
//! use std::sync::Arc;
//! use ai::workflow::{git_commit, WorkflowRegistry, WorkflowLlmExecutor};
//!
//! // Create an executor (implements WorkflowLlmExecutor)
//! let executor: Arc<dyn WorkflowLlmExecutor> = /* ... */;
//!
//! // Create the workflow graph
//! let graph = git_commit::create_git_commit_workflow(executor);
//!
//! // Register it
//! let mut registry = WorkflowRegistry::new();
//! registry.register("git_commit", graph);
//! ```

pub mod git_commit;
pub mod models;
pub mod registry;
pub mod runner;

// Re-export base workflow types from runner
pub use runner::{
    patterns, AgentWorkflowBuilder, RouterTask, SubAgentExecutor, SubAgentTask, WorkflowRunner,
    WorkflowStatus, WorkflowStepResult, WorkflowStorage,
};

// Re-export state models
pub use models::{
    CommitPlan, FileChange, FileStatus, GitCommitResult, GitCommitState, WorkflowLlmExecutor,
    WorkflowStage,
};
pub use registry::WorkflowRegistry;

// Re-export git_commit workflow construction
pub use git_commit::create_git_commit_workflow;
