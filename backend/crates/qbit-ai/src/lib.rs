//! AI Agent Orchestration for Qbit
//!
//! This crate provides the core AI agent system including:
//! - Agent bridge and lifecycle management
//! - Agentic loop execution
//! - Context management and pruning
//! - Token budget management
//! - Tool execution and policy enforcement
//! - HITL (Human-in-the-Loop) approval system
//! - Loop detection and protection
//! - Sub-agent execution
//! - Workflow execution (graph-flow based multi-step workflows)
//!
//! # Architecture
//!
//! This is a **Layer 3 (Domain)** crate:
//! - Depends on: qbit-core, qbit-settings, qbit-tools
//! - Used by: qbit (main application via Tauri commands)
//!
//! # Core Components
//!
//! - **AgentBridge**: Main interface for agent lifecycle and interactions
//! - **AgenticLoop**: Core agentic execution loop
//! - **LlmClient**: LLM provider abstraction (Anthropic, OpenAI, Gemini, etc.)
//! - **ContextManager**: Manages conversation context and trimming
//! - **TokenBudgetManager**: Tracks token usage and budget limits
//! - **ToolExecutors**: Executes AI tools (file ops, shell, etc.)
//! - **ApprovalRecorder**: HITL approval tracking and auto-approval
//! - **LoopDetector**: Detects and prevents infinite agent loops
//! - **WorkflowRunner**: Executes multi-step graph-based workflows

// Core modules (kept in this crate)
pub mod agent_bridge;
pub mod agent_mode;
pub mod agentic_loop;
mod bridge_context;
mod bridge_hitl;
mod bridge_policy;
mod bridge_session;
pub mod llm_client;
pub mod memory_file;
pub mod system_prompt;
pub mod tool_definitions;
pub mod tool_execution;
pub mod tool_executors;
pub mod tool_provider_impl;

// Prompt composition system
pub mod contributors;
pub mod prompt_registry;

// Test utilities (only available in test builds)
#[cfg(test)]
pub mod test_utils;

// Public API types from this crate
pub use agent_mode::AgentMode;
pub use prompt_registry::PromptContributorRegistry;
pub use tool_definitions::{
    get_all_tool_definitions_with_config, get_tool_definitions_for_preset,
    get_tool_definitions_with_config, ToolConfig, ToolPreset,
};
pub use tool_execution::{
    normalize_run_pty_cmd_args, route_tool_execution, ToolCategory, ToolExecutionConfig,
    ToolExecutionContext, ToolExecutionError, ToolExecutionResult, ToolSource,
};
pub use tool_provider_impl::DefaultToolProvider;
