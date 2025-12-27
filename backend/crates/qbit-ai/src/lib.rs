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

pub mod agent_bridge;
pub mod agent_mode;
pub mod agentic_loop;
mod bridge_context;
mod bridge_hitl;
mod bridge_policy;
mod bridge_session;
pub mod context_manager;
pub mod context_pruner;
pub mod hitl;
pub mod llm_client;
pub mod loop_detection;
pub mod memory_file;
pub mod session;
pub mod sub_agent;
pub mod sub_agent_executor;
pub mod system_prompt;
pub mod token_budget;
pub mod token_trunc;
pub mod tool_definitions;
pub mod tool_executors;
pub mod tool_policy;
pub mod workflow;

// External service integrations
pub mod tavily;
pub mod web_fetch;

// Public API types
pub use agent_mode::AgentMode;
pub use context_manager::{ContextEvent, ContextManager, ContextSummary, ContextTrimConfig};
pub use context_pruner::{ContextPruner, ContextPrunerConfig, PruneResult, SemanticScore};
pub use hitl::{ApprovalRecorder, ApprovalRequest};
pub use loop_detection::{
    LoopDetectionResult, LoopDetector, LoopDetectorStats, LoopProtectionConfig,
};
pub use session::{QbitMessageRole, QbitSessionMessage, QbitSessionSnapshot, SessionListingInfo};
pub use sub_agent::{SubAgentContext, SubAgentDefinition, SubAgentRegistry, SubAgentResult};
pub use token_budget::{
    TokenAlertLevel, TokenBudgetConfig, TokenBudgetManager, TokenUsageStats,
    DEFAULT_MAX_CONTEXT_TOKENS, MAX_TOOL_RESPONSE_TOKENS,
};
pub use token_trunc::{
    aggregate_tool_output, truncate_by_chars, truncate_by_tokens, ContentType, TruncationResult,
};
pub use tool_definitions::{
    get_all_tool_definitions_with_config, get_tool_definitions_for_preset,
    get_tool_definitions_with_config, ToolConfig, ToolPreset,
};
pub use tool_policy::{
    PolicyConstraintResult, ToolConstraints, ToolPolicy, ToolPolicyConfig, ToolPolicyManager,
};
pub use workflow::{
    create_default_registry, register_builtin_workflows, GitCommitResult, GitCommitState,
    GitCommitWorkflow, WorkflowDefinition, WorkflowInfo, WorkflowLlmExecutor, WorkflowRegistry,
    WorkflowRunner, WorkflowStatus, WorkflowStepResult, WorkflowStorage,
};
