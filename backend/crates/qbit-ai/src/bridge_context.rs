//! Context management extension for AgentBridge.
//!
//! This module contains methods for managing context window and token budgeting.

use super::agent_bridge::AgentBridge;
use qbit_context::token_budget::{TokenAlertLevel, TokenUsageStats};
use qbit_context::{ContextEnforcementResult, ContextSummary, ContextTrimConfig};

impl AgentBridge {
    // ========================================================================
    // Context Management Methods
    // ========================================================================

    /// Get current context summary.
    pub async fn get_context_summary(&self) -> ContextSummary {
        self.context_manager.get_summary().await
    }

    /// Get current token usage statistics.
    pub async fn get_token_usage_stats(&self) -> TokenUsageStats {
        self.context_manager.stats().await
    }

    /// Get current token alert level.
    pub async fn get_token_alert_level(&self) -> TokenAlertLevel {
        self.context_manager.alert_level().await
    }

    /// Get context utilization percentage.
    pub async fn get_context_utilization(&self) -> f64 {
        self.context_manager.utilization().await
    }

    /// Get remaining available tokens.
    pub async fn get_remaining_tokens(&self) -> usize {
        self.context_manager.remaining_tokens().await
    }

    /// Enforce context window limits by pruning old messages if needed.
    ///
    /// Returns the full enforcement result containing:
    /// - The (possibly pruned) messages
    /// - Warning info if utilization exceeded warning threshold
    /// - Pruning info if messages were removed
    ///
    /// The caller can use this to emit appropriate events.
    pub async fn enforce_context_window(&self) -> ContextEnforcementResult {
        let history = self.conversation_history.read().await;
        let result = self.context_manager.enforce_context_window(&history).await;
        drop(history);

        // Update history with pruned messages if any
        if result.pruned_info.is_some() {
            let mut history = self.conversation_history.write().await;
            *history = result.messages.clone();
        }

        result
    }

    /// Enforce context window and return the number of messages pruned (legacy API).
    ///
    /// This is a convenience method that returns just the count of pruned messages.
    /// For full control over warning/pruned events, use `enforce_context_window()` instead.
    pub async fn enforce_context_window_count(&self) -> usize {
        let result = self.enforce_context_window().await;
        result
            .pruned_info
            .map(|info| info.messages_removed)
            .unwrap_or(0)
    }

    /// Reset the context manager.
    pub async fn reset_context_manager(&self) {
        self.context_manager.reset().await;
    }

    /// Get the context trim configuration.
    pub fn get_context_trim_config(&self) -> ContextTrimConfig {
        self.context_manager.trim_config().clone()
    }

    /// Check if context management is enabled.
    pub fn is_context_management_enabled(&self) -> bool {
        self.context_manager.is_enabled()
    }
}
