//! Core HITL (Human-in-the-Loop) types for tool approval management.
//!
//! These types are shared across multiple modules and define the data structures
//! for approval patterns, risk levels, and configuration.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Minimum number of approvals required before auto-approve is considered.
pub const HITL_AUTO_APPROVE_MIN_APPROVALS: u32 = 3;

/// Approval rate threshold for auto-approve (80%).
pub const HITL_AUTO_APPROVE_THRESHOLD: f64 = 0.8;

/// Risk level for a tool operation.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RiskLevel {
    /// Safe operations (read-only)
    Low,
    /// Operations that modify state but are recoverable
    Medium,
    /// Operations that can cause significant changes
    High,
    /// Destructive or irreversible operations
    Critical,
}

impl RiskLevel {
    /// Determine risk level for a tool based on its name.
    pub fn for_tool(tool_name: &str) -> Self {
        match tool_name {
            // Read-only operations
            "read_file" | "grep_file" | "list_files" => RiskLevel::Low,
            "indexer_search_code" | "indexer_search_files" | "indexer_analyze_file" => {
                RiskLevel::Low
            }
            "indexer_extract_symbols" | "indexer_get_metrics" | "indexer_detect_language" => {
                RiskLevel::Low
            }
            "debug_agent" | "analyze_agent" | "get_errors" => RiskLevel::Low,
            "list_skills" | "search_skills" | "load_skill" | "search_tools" => RiskLevel::Low,
            "update_plan" => RiskLevel::Low,
            "web_fetch" => RiskLevel::Low,

            // Write operations (recoverable)
            "write_file" | "create_file" | "edit_file" | "apply_patch" => RiskLevel::Medium,
            "save_skill" => RiskLevel::Medium,

            // Shell execution
            "run_command" | "run_pty_cmd" => RiskLevel::High,
            "create_pty_session" | "send_pty_input" => RiskLevel::High,

            // Destructive operations
            "delete_file" => RiskLevel::Critical,
            "execute_code" => RiskLevel::Critical,

            // Default for unknown tools
            _ => {
                // Sub-agents are medium risk
                if tool_name.starts_with("sub_agent_") {
                    RiskLevel::Medium
                } else {
                    RiskLevel::High
                }
            }
        }
    }
}

/// Approval pattern/statistics for a specific tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalPattern {
    /// Name of the tool
    pub tool_name: String,
    /// Total number of approval requests
    pub total_requests: u32,
    /// Number of approvals
    pub approvals: u32,
    /// Number of denials
    pub denials: u32,
    /// Whether this tool has been marked as "always allow"
    pub always_allow: bool,
    /// Last time this pattern was updated
    pub last_updated: DateTime<Utc>,
    /// Justifications provided (for auditing)
    pub justifications: Vec<String>,
}

impl ApprovalPattern {
    /// Create a new pattern for a tool.
    pub fn new(tool_name: String) -> Self {
        Self {
            tool_name,
            total_requests: 0,
            approvals: 0,
            denials: 0,
            always_allow: false,
            last_updated: Utc::now(),
            justifications: Vec::new(),
        }
    }

    /// Calculate the approval rate (0.0 - 1.0).
    pub fn approval_rate(&self) -> f64 {
        if self.total_requests == 0 {
            0.0
        } else {
            self.approvals as f64 / self.total_requests as f64
        }
    }

    /// Check if this pattern qualifies for auto-approval based on thresholds.
    pub fn qualifies_for_auto_approve(&self, min_approvals: u32, threshold: f64) -> bool {
        self.approvals >= min_approvals && self.approval_rate() >= threshold
    }

    /// Record an approval decision.
    pub fn record_decision(&mut self, approved: bool, reason: Option<String>) {
        self.total_requests += 1;
        if approved {
            self.approvals += 1;
        } else {
            self.denials += 1;
        }
        self.last_updated = Utc::now();

        if let Some(r) = reason {
            if !r.is_empty() {
                // Keep last 10 justifications for auditing
                if self.justifications.len() >= 10 {
                    self.justifications.remove(0);
                }
                self.justifications.push(r);
            }
        }
    }
}

/// User's decision on an approval request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalDecision {
    /// The request ID this decision is for
    pub request_id: String,
    /// Whether the tool was approved
    pub approved: bool,
    /// Optional reason/justification for the decision
    pub reason: Option<String>,
    /// Whether to remember this decision for future auto-approval
    pub remember: bool,
    /// Whether to always allow this specific tool
    pub always_allow: bool,
}

/// Configuration for tool approval behavior.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolApprovalConfig {
    /// Tools that are always allowed without approval
    pub always_allow: Vec<String>,
    /// Tools that always require approval (cannot be auto-approved)
    pub always_require_approval: Vec<String>,
    /// Whether pattern learning is enabled
    pub pattern_learning_enabled: bool,
    /// Minimum approvals before auto-approve
    pub min_approvals: u32,
    /// Approval rate threshold (0.0 - 1.0)
    pub approval_threshold: f64,
}

impl Default for ToolApprovalConfig {
    fn default() -> Self {
        Self {
            // Safe read-only tools
            always_allow: vec![
                "read_file".to_string(),
                "grep_file".to_string(),
                "list_files".to_string(),
                "indexer_search_code".to_string(),
                "indexer_search_files".to_string(),
                "indexer_analyze_file".to_string(),
                "indexer_extract_symbols".to_string(),
                "indexer_get_metrics".to_string(),
                "indexer_detect_language".to_string(),
                "debug_agent".to_string(),
                "analyze_agent".to_string(),
                "get_errors".to_string(),
                "list_skills".to_string(),
                "search_skills".to_string(),
                "load_skill".to_string(),
                "search_tools".to_string(),
            ],
            // Dangerous tools that should always require approval
            always_require_approval: vec![
                "delete_file".to_string(),
                "run_command".to_string(),
                "run_pty_cmd".to_string(),
                "execute_code".to_string(),
            ],
            pattern_learning_enabled: true,
            min_approvals: HITL_AUTO_APPROVE_MIN_APPROVALS,
            approval_threshold: HITL_AUTO_APPROVE_THRESHOLD,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_approval_pattern_rate() {
        let mut pattern = ApprovalPattern::new("test_tool".to_string());

        // No requests = 0% rate
        assert_eq!(pattern.approval_rate(), 0.0);

        // 3 approvals, 0 denials = 100%
        pattern.record_decision(true, None);
        pattern.record_decision(true, None);
        pattern.record_decision(true, None);
        assert_eq!(pattern.approval_rate(), 1.0);

        // 3 approvals, 1 denial = 75%
        pattern.record_decision(false, None);
        assert_eq!(pattern.approval_rate(), 0.75);
    }

    #[test]
    fn test_approval_pattern_qualification() {
        let mut pattern = ApprovalPattern::new("test_tool".to_string());

        // Not enough approvals
        pattern.record_decision(true, None);
        pattern.record_decision(true, None);
        assert!(!pattern.qualifies_for_auto_approve(3, 0.8));

        // Enough approvals but rate too low
        pattern.record_decision(true, None);
        pattern.record_decision(false, None);
        pattern.record_decision(false, None);
        // 3 approvals, 2 denials = 60%
        assert!(!pattern.qualifies_for_auto_approve(3, 0.8));

        // Meet both thresholds
        pattern.record_decision(true, None);
        pattern.record_decision(true, None);
        // 5 approvals, 2 denials = ~71%
        assert!(!pattern.qualifies_for_auto_approve(3, 0.8));

        pattern.record_decision(true, None);
        // 6 approvals, 2 denials = 75%
        assert!(!pattern.qualifies_for_auto_approve(3, 0.8));

        pattern.record_decision(true, None);
        // 7 approvals, 2 denials = ~78%
        assert!(!pattern.qualifies_for_auto_approve(3, 0.8));

        pattern.record_decision(true, None);
        // 8 approvals, 2 denials = 80%
        assert!(pattern.qualifies_for_auto_approve(3, 0.8));
    }

    #[test]
    fn test_risk_level_classification() {
        assert_eq!(RiskLevel::for_tool("read_file"), RiskLevel::Low);
        assert_eq!(RiskLevel::for_tool("write_file"), RiskLevel::Medium);
        assert_eq!(RiskLevel::for_tool("run_pty_cmd"), RiskLevel::High);
        assert_eq!(RiskLevel::for_tool("delete_file"), RiskLevel::Critical);
        assert_eq!(RiskLevel::for_tool("sub_agent_analyzer"), RiskLevel::Medium);
        assert_eq!(RiskLevel::for_tool("unknown_tool"), RiskLevel::High);
    }
}
