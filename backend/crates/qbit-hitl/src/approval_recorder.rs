//! Approval recording and pattern learning for HITL.
//!
//! This module tracks tool approval decisions and learns patterns to enable
//! automatic approval for frequently-approved tools.
// Some public API items are for future pattern learning features
#![allow(dead_code)]

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

// Import core types from qbit-core
use qbit_core::hitl::{ApprovalPattern, RiskLevel, ToolApprovalConfig};

/// Request for tool approval.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalRequest {
    /// Unique ID for this request
    pub request_id: String,
    /// Name of the tool requesting approval
    pub tool_name: String,
    /// Tool arguments
    pub args: serde_json::Value,
    /// Current approval stats for this tool
    pub current_stats: Option<ApprovalPattern>,
    /// Whether this tool can potentially be auto-approved in the future
    pub can_learn: bool,
    /// Risk level of this tool
    pub risk_level: RiskLevel,
}

/// Persisted approval data.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ApprovalData {
    /// Version for future migrations
    version: u32,
    /// Approval patterns by tool name
    patterns: HashMap<String, ApprovalPattern>,
    /// Configuration
    config: ToolApprovalConfig,
}

impl Default for ApprovalData {
    fn default() -> Self {
        Self {
            version: 1,
            patterns: HashMap::new(),
            config: ToolApprovalConfig::default(),
        }
    }
}
/// Records and manages tool approval patterns.
///
/// Thread-safe wrapper around approval data with persistence.
pub struct ApprovalRecorder {
    /// Approval data (patterns and config)
    data: Arc<RwLock<ApprovalData>>,
    /// Path to the persistence file
    storage_path: PathBuf,
}

impl ApprovalRecorder {
    /// Create a new ApprovalRecorder with the given storage directory.
    pub async fn new(storage_dir: PathBuf) -> Self {
        let storage_path = storage_dir.join("approval_patterns.json");

        // Try to load existing data
        let data = if storage_path.exists() {
            match tokio::fs::read_to_string(&storage_path).await {
                Ok(contents) => serde_json::from_str(&contents).unwrap_or_default(),
                Err(e) => {
                    tracing::warn!("Failed to load approval data: {}", e);
                    ApprovalData::default()
                }
            }
        } else {
            ApprovalData::default()
        };

        Self {
            data: Arc::new(RwLock::new(data)),
            storage_path,
        }
    }

    /// Check if a tool should be auto-approved.
    ///
    /// Returns `true` if:
    /// - Tool is in the always_allow list, OR
    /// - Pattern learning is enabled AND the tool has enough approvals with high rate
    pub async fn should_auto_approve(&self, tool_name: &str) -> bool {
        let data = self.data.read().await;

        // Check always_allow list
        if data.config.always_allow.contains(&tool_name.to_string()) {
            return true;
        }

        // Check if tool is in always_require_approval list
        if data
            .config
            .always_require_approval
            .contains(&tool_name.to_string())
        {
            return false;
        }

        // Check if pattern learning is enabled
        if !data.config.pattern_learning_enabled {
            return false;
        }

        // Check the approval pattern
        if let Some(pattern) = data.patterns.get(tool_name) {
            // Check if explicitly marked as always_allow
            if pattern.always_allow {
                return true;
            }

            // Check if pattern qualifies
            pattern.qualifies_for_auto_approve(
                data.config.min_approvals,
                data.config.approval_threshold,
            )
        } else {
            false
        }
    }

    /// Record an approval decision.
    pub async fn record_approval(
        &self,
        tool_name: &str,
        approved: bool,
        reason: Option<String>,
        always_allow: bool,
    ) -> anyhow::Result<()> {
        let mut data = self.data.write().await;

        // Get or create pattern
        let pattern = data
            .patterns
            .entry(tool_name.to_string())
            .or_insert_with(|| ApprovalPattern::new(tool_name.to_string()));

        // Record the decision
        pattern.record_decision(approved, reason);

        // Handle always_allow
        if always_allow && approved {
            pattern.always_allow = true;
        }

        // Persist to disk
        drop(data);
        self.save().await
    }

    /// Get the approval pattern for a tool.
    pub async fn get_pattern(&self, tool_name: &str) -> Option<ApprovalPattern> {
        let data = self.data.read().await;
        data.patterns.get(tool_name).cloned()
    }

    /// Get all approval patterns.
    pub async fn get_all_patterns(&self) -> Vec<ApprovalPattern> {
        let data = self.data.read().await;
        data.patterns.values().cloned().collect()
    }

    /// Get the current configuration.
    pub async fn get_config(&self) -> ToolApprovalConfig {
        let data = self.data.read().await;
        data.config.clone()
    }

    /// Update the configuration.
    pub async fn set_config(&self, config: ToolApprovalConfig) -> anyhow::Result<()> {
        {
            let mut data = self.data.write().await;
            data.config = config;
        }
        self.save().await
    }

    /// Add a tool to the always_allow list.
    pub async fn add_always_allow(&self, tool_name: &str) -> anyhow::Result<()> {
        {
            let mut data = self.data.write().await;
            if !data.config.always_allow.contains(&tool_name.to_string()) {
                data.config.always_allow.push(tool_name.to_string());
            }
            // Also remove from always_require if present
            data.config
                .always_require_approval
                .retain(|t| t != tool_name);
        }
        self.save().await
    }

    /// Remove a tool from the always_allow list.
    pub async fn remove_always_allow(&self, tool_name: &str) -> anyhow::Result<()> {
        {
            let mut data = self.data.write().await;
            data.config.always_allow.retain(|t| t != tool_name);
            // Also clear the pattern's always_allow flag
            if let Some(pattern) = data.patterns.get_mut(tool_name) {
                pattern.always_allow = false;
            }
        }
        self.save().await
    }

    /// Reset all approval patterns (keep config).
    pub async fn reset_patterns(&self) -> anyhow::Result<()> {
        {
            let mut data = self.data.write().await;
            data.patterns.clear();
        }
        self.save().await
    }

    /// Save approval data to disk.
    async fn save(&self) -> anyhow::Result<()> {
        let data = self.data.read().await;

        // Ensure directory exists
        if let Some(parent) = self.storage_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        let json = serde_json::to_string_pretty(&*data)?;
        tokio::fs::write(&self.storage_path, json).await?;

        tracing::debug!("Saved approval patterns to {:?}", self.storage_path);
        Ok(())
    }

    /// Create an approval request for a tool.
    pub async fn create_request(
        &self,
        request_id: String,
        tool_name: &str,
        args: serde_json::Value,
    ) -> ApprovalRequest {
        let data = self.data.read().await;
        let pattern = data.patterns.get(tool_name).cloned();
        let can_learn = !data
            .config
            .always_require_approval
            .contains(&tool_name.to_string());
        let risk_level = RiskLevel::for_tool(tool_name);

        ApprovalRequest {
            request_id,
            tool_name: tool_name.to_string(),
            args,
            current_stats: pattern,
            can_learn,
            risk_level,
        }
    }

    /// Get a suggestion message if a tool is close to auto-approval threshold.
    pub async fn get_suggestion(&self, tool_name: &str) -> Option<String> {
        let data = self.data.read().await;

        if !data.config.pattern_learning_enabled {
            return None;
        }

        if let Some(pattern) = data.patterns.get(tool_name) {
            let rate = pattern.approval_rate();
            let approvals = pattern.approvals;
            let min = data.config.min_approvals;
            let threshold = data.config.approval_threshold;

            // Already qualifies
            if pattern.qualifies_for_auto_approve(min, threshold) {
                return None;
            }

            // Close to threshold - suggest
            if approvals >= 2 && rate >= 0.6 {
                let needed = min.saturating_sub(approvals);
                if needed > 0 {
                    return Some(format!(
                        "You've approved '{}' {} times ({:.0}% approval rate). {} more approval(s) needed for auto-approve.",
                        tool_name, approvals, rate * 100.0, needed
                    ));
                } else if rate < threshold {
                    return Some(format!(
                        "Tool '{}' has {} approvals but only {:.0}% approval rate. Need {:.0}% for auto-approve.",
                        tool_name, approvals, rate * 100.0, threshold * 100.0
                    ));
                }
            }
        }

        None
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
