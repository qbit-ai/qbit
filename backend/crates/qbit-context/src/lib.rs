//! Context window and token budget management for Qbit AI.
//!
//! This crate provides token counting, context compaction, and budget management
//! for managing LLM context windows.

pub mod context_manager;
pub mod token_budget;
pub mod token_trunc;

// Re-export main types
pub use context_manager::{
    CompactionCheck, CompactionState, ContextEnforcementResult, ContextEvent, ContextManager,
    ContextManagerConfig, ContextSummary, ContextTrimConfig, ContextWarningInfo,
};
pub use token_budget::{
    TokenAlertLevel, TokenBudgetConfig, TokenBudgetManager, TokenUsageStats,
    DEFAULT_MAX_CONTEXT_TOKENS, MAX_TOOL_RESPONSE_TOKENS,
};
pub use token_trunc::{
    aggregate_tool_output, truncate_by_chars, truncate_by_tokens, ContentType, TruncationResult,
};
