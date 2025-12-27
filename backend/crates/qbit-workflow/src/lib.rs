#![allow(dead_code)] // Workflow system is implemented but not yet integrated
//! Graph-based workflow execution for Qbit AI.
//!
//! This crate provides a workflow system for multi-step AI tasks using graph-flow.
//!
//! # Architecture
//!
//! Workflows use graph-flow for task orchestration:
//! - Each workflow implements the `WorkflowDefinition` trait
//! - Workflows are registered by name in a `WorkflowRegistry`
//! - The `WorkflowRunner` handles session-based execution
//! - Tasks communicate via shared Context
//!
//! # Adding a New Workflow
//!
//! 1. Create a new module in `definitions/`
//! 2. Implement `WorkflowDefinition` trait
//! 3. Register in `definitions::register_builtin_workflows()`
//!
//! # Example
//!
//! ```rust,ignore
//! use qbit_workflow::{definitions, WorkflowRunner};
//!
//! // Create registry with built-in workflows
//! let registry = definitions::create_default_registry();
//!
//! // Get a workflow by name
//! let workflow = registry.get("git_commit").unwrap();
//!
//! // Start execution
//! let executor: Arc<dyn WorkflowLlmExecutor> = /* ... */;
//! let graph = workflow.build_graph(executor);
//! let runner = WorkflowRunner::new_in_memory(graph);
//! ```

pub mod definitions;
mod models;
mod registry;
mod runner;

// Re-export core types
pub use models::{
    StartWorkflowResponse, WorkflowAgentConfig, WorkflowAgentResult, WorkflowDefinition,
    WorkflowInfo, WorkflowLlmExecutor, WorkflowStateResponse, WorkflowStepResponse,
    WorkflowToolCall,
};
pub use registry::WorkflowRegistry;
pub use runner::{
    AgentWorkflowBuilder, RouterTask, SubAgentDefinition, SubAgentExecutor, SubAgentTask,
    WorkflowRunner, WorkflowStatus, WorkflowStepResult, WorkflowStorage,
};

// Re-export workflow definitions for convenience
pub use definitions::git_commit::{GitCommitResult, GitCommitState, GitCommitWorkflow};
pub use definitions::{create_default_registry, register_builtin_workflows};
