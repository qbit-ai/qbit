//! Human-in-the-Loop (HITL) module for tool approval management.
//!
//! This module provides:
//! - `ApprovalRecorder`: Tracks approval patterns for tools
//! - `ApprovalPattern`: Statistics for a specific tool
//! - Pattern learning: Auto-approve tools with high approval rates
//!
//! Based on the VTCode implementation pattern.

mod approval_recorder;

// Re-export core types from qbit-core
pub use qbit_core::hitl::{
    ApprovalDecision, ApprovalPattern, RiskLevel, ToolApprovalConfig,
    HITL_AUTO_APPROVE_MIN_APPROVALS, HITL_AUTO_APPROVE_THRESHOLD,
};

// Re-export local implementation types
pub use approval_recorder::{ApprovalRecorder, ApprovalRequest};
