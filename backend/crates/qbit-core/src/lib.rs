//! Core types and traits for the Qbit application.
//!
//! This crate provides the foundation types used across all other qbit crates.
//! It has ZERO internal crate dependencies and only depends on external libraries.
//!
//! ## Architecture Principle
//!
//! qbit-core sits at the bottom of the dependency hierarchy:
//! - Layer 1 (Foundation): qbit-core ← YOU ARE HERE
//! - Layer 2 (Infrastructure): qbit-settings, qbit-runtime
//! - Layer 3 (Domain): qbit-tools, qbit-pty, etc.
//! - Layer 4 (Application): qbit (main crate)

// Module declarations (will be populated in next steps)
pub mod events;
pub mod message;
pub mod runtime;
pub mod session;
pub mod tool;
pub mod tool_name;

pub mod hitl;
pub mod plan;
pub mod prompt;
pub mod utils;

// Re-exports
pub use events::*; // Re-export all event types
pub use hitl::{
    ApprovalDecision, ApprovalPattern, RiskLevel, ToolApprovalConfig,
    HITL_AUTO_APPROVE_MIN_APPROVALS, HITL_AUTO_APPROVE_THRESHOLD,
};
pub use message::{PromptPart, PromptPayload};
pub use plan::{PlanStep, PlanSummary, StepStatus, TaskPlan, MAX_PLAN_STEPS, MIN_PLAN_STEPS};
pub use prompt::{
    PromptContext, PromptContributor, PromptMatchedSkill, PromptPriority, PromptSection,
    PromptSkillInfo,
};
pub use runtime::{ApprovalResult, QbitRuntime, RuntimeError, RuntimeEvent};
pub use session::{
    find_session_by_identifier, list_recent_sessions, MessageContent, MessageRole, SessionArchive,
    SessionArchiveMetadata, SessionListing, SessionMessage, SessionSnapshot,
};
pub use tool::Tool;
pub use tool_name::{ToolCategory, ToolName};
