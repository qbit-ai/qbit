//! Context management orchestration
//!
//! Coordinates token budgeting, context compaction, and truncation strategies.
// Public API for future use - not all methods are currently called
#![allow(dead_code)]

use rig::message::Message;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::{
    token_budget::{TokenAlertLevel, TokenBudgetConfig, TokenBudgetManager, TokenUsageStats},
    token_trunc::{aggregate_tool_output, TruncationResult},
};

/// State tracking for context compaction.
#[derive(Debug, Clone, Default)]
pub struct CompactionState {
    /// Whether compaction has been attempted this turn
    pub attempted_this_turn: bool,
    /// Number of compactions performed this session
    pub compaction_count: u32,
    /// Last known input token count from provider
    pub last_input_tokens: Option<u64>,
    /// Whether we're using heuristic (no provider tokens available)
    pub using_heuristic: bool,
}

impl CompactionState {
    /// Create a new CompactionState with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Reset the turn-specific state (called at the start of each turn).
    pub fn reset_turn(&mut self) {
        self.attempted_this_turn = false;
    }

    /// Mark that compaction has been attempted this turn.
    pub fn mark_attempted(&mut self) {
        self.attempted_this_turn = true;
    }

    /// Increment the compaction count (called after successful compaction).
    pub fn increment_count(&mut self) {
        self.compaction_count += 1;
    }

    /// Update token count from provider response.
    pub fn update_tokens(&mut self, input_tokens: u64) {
        self.last_input_tokens = Some(input_tokens);
        self.using_heuristic = false;
    }

    /// Update token count using heuristic estimation (char_count / 4).
    pub fn update_tokens_heuristic(&mut self, char_count: usize) {
        self.last_input_tokens = Some((char_count / 4) as u64);
        self.using_heuristic = true;
    }
}

/// Result of checking whether compaction should occur.
#[derive(Debug, Clone)]
pub struct CompactionCheck {
    /// Whether compaction should be triggered
    pub should_compact: bool,
    /// Current token usage
    pub current_tokens: u64,
    /// Maximum tokens for the model
    pub max_tokens: usize,
    /// Threshold that was used (e.g., 0.80)
    pub threshold: f64,
    /// Whether tokens came from provider or heuristic
    pub using_heuristic: bool,
    /// Reason for the decision
    pub reason: String,
}

/// Configuration for context trimming behavior
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextTrimConfig {
    /// Enable automatic context trimming
    pub enabled: bool,
    /// Target utilization ratio (0.0-1.0) when trimming
    pub target_utilization: f64,
    /// Enable aggressive trimming when critically low on space
    pub aggressive_on_critical: bool,
    /// Maximum tool response tokens before truncation
    pub max_tool_response_tokens: usize,
}

impl Default for ContextTrimConfig {
    fn default() -> Self {
        Self {
            enabled: false, // Disabled by default
            target_utilization: 0.7,
            aggressive_on_critical: true,
            max_tool_response_tokens: 25_000,
        }
    }
}

/// High-level configuration for context management behavior.
///
/// This struct is designed to be easily constructed from application settings
/// (like `ContextSettings` from qbit-settings) without creating a dependency
/// between qbit-context and qbit-settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextManagerConfig {
    /// Enable context window management (pruning, truncation, token budgeting)
    pub enabled: bool,
    /// Context utilization threshold (0.0-1.0) at which pruning is triggered
    pub compaction_threshold: f64,
    /// Number of recent turns to protect from pruning
    pub protected_turns: usize,
    /// Minimum seconds between pruning operations (cooldown)
    pub cooldown_seconds: u64,
}

impl Default for ContextManagerConfig {
    fn default() -> Self {
        Self {
            enabled: true,              // Enabled by default
            compaction_threshold: 0.80, // Trigger at 80% utilization
            protected_turns: 2,         // Protect last 2 turns
            cooldown_seconds: 60,       // 1 minute cooldown
        }
    }
}

/// Efficiency metrics after context operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextEfficiency {
    /// Utilization before operation
    pub utilization_before: f64,
    /// Utilization after operation
    pub utilization_after: f64,
    /// Tokens freed
    pub tokens_freed: usize,
    /// Messages pruned
    pub messages_pruned: usize,
    /// Tool responses truncated
    pub tool_responses_truncated: usize,
    /// Timestamp of operation
    pub timestamp: u64,
}

/// Events emitted during context management
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ContextEvent {
    /// Warning threshold exceeded
    WarningThreshold {
        utilization: f64,
        total_tokens: usize,
        max_tokens: usize,
    },
    /// Alert threshold exceeded
    AlertThreshold {
        utilization: f64,
        total_tokens: usize,
        max_tokens: usize,
    },
    /// Tool response was truncated
    ToolResponseTruncated {
        original_tokens: usize,
        truncated_tokens: usize,
        tool_name: String,
    },
    /// Context window exceeded (critical)
    ContextExceeded {
        total_tokens: usize,
        max_tokens: usize,
    },
}

/// Central manager for context window management
#[derive(Debug)]
pub struct ContextManager {
    /// Token budget manager
    token_budget: Arc<TokenBudgetManager>,
    /// Trim configuration
    trim_config: ContextTrimConfig,
    /// Whether token budgeting is enabled
    token_budget_enabled: bool,
    /// Last recorded efficiency metrics
    last_efficiency: Arc<RwLock<Option<ContextEfficiency>>>,
    /// Event channel for notifications
    event_tx: Option<tokio::sync::mpsc::Sender<ContextEvent>>,
}

impl ContextManager {
    /// Create a new context manager
    pub fn new(budget_config: TokenBudgetConfig, trim_config: ContextTrimConfig) -> Self {
        Self {
            token_budget: Arc::new(TokenBudgetManager::new(budget_config)),
            trim_config,
            token_budget_enabled: false, // Disabled by default
            last_efficiency: Arc::new(RwLock::new(None)),
            event_tx: None,
        }
    }

    /// Create with high-level configuration.
    ///
    /// This constructor accepts a `ContextManagerConfig` which mirrors
    /// the settings from application configuration (e.g., `ContextSettings`).
    /// It properly enables both `token_budget_enabled` and `trim_config.enabled`
    /// based on the `config.enabled` setting.
    ///
    /// # Example
    /// ```
    /// use qbit_context::context_manager::{ContextManager, ContextManagerConfig};
    ///
    /// // Create with default config (enabled)
    /// let manager = ContextManager::with_config("claude-3-5-sonnet", ContextManagerConfig::default());
    /// assert!(manager.is_enabled());
    ///
    /// // Create with custom config
    /// let config = ContextManagerConfig {
    ///     enabled: true,
    ///     compaction_threshold: 0.75,
    ///     protected_turns: 3,
    ///     cooldown_seconds: 120,
    /// };
    /// let manager = ContextManager::with_config("claude-3-5-sonnet", config);
    /// ```
    pub fn with_config(model: &str, config: ContextManagerConfig) -> Self {
        let budget_config = TokenBudgetConfig::for_model(model);

        // Configure token budget thresholds based on compaction_threshold
        // Alert threshold is at compaction_threshold, warning is slightly below
        let mut budget_config = budget_config;
        budget_config.alert_threshold = config.compaction_threshold;
        budget_config.warning_threshold = (config.compaction_threshold - 0.10).max(0.50);

        // Configure trim settings
        let trim_config = ContextTrimConfig {
            enabled: config.enabled,
            target_utilization: config.compaction_threshold - 0.10, // Target 10% below threshold
            aggressive_on_critical: true,
            max_tool_response_tokens: 25_000,
        };

        Self {
            token_budget: Arc::new(TokenBudgetManager::new(budget_config)),
            trim_config,
            token_budget_enabled: config.enabled,
            last_efficiency: Arc::new(RwLock::new(None)),
            event_tx: None,
        }
    }

    /// Create with default configuration for a model
    pub fn for_model(model: &str) -> Self {
        Self::new(
            TokenBudgetConfig::for_model(model),
            ContextTrimConfig::default(),
        )
    }

    /// Create with default configuration for a model, with context management enabled.
    ///
    /// This is equivalent to `with_config(model, ContextManagerConfig::default())`
    /// and ensures both token budgeting and trimming are enabled.
    pub fn for_model_enabled(model: &str) -> Self {
        Self::with_config(model, ContextManagerConfig::default())
    }

    /// Set event channel for notifications
    pub fn set_event_channel(&mut self, tx: tokio::sync::mpsc::Sender<ContextEvent>) {
        self.event_tx = Some(tx);
    }

    /// Get reference to token budget manager
    pub fn token_budget(&self) -> Arc<TokenBudgetManager> {
        Arc::clone(&self.token_budget)
    }

    /// Get current trim configuration
    pub fn trim_config(&self) -> &ContextTrimConfig {
        &self.trim_config
    }

    /// Update trim configuration
    pub fn set_trim_config(&mut self, config: ContextTrimConfig) {
        self.trim_config = config;
    }

    /// Check if token budgeting is enabled
    pub fn is_enabled(&self) -> bool {
        self.token_budget_enabled
    }

    /// Enable/disable token budgeting
    pub fn set_enabled(&mut self, enabled: bool) {
        self.token_budget_enabled = enabled;
    }

    /// Get current token usage stats
    pub async fn stats(&self) -> TokenUsageStats {
        self.token_budget.stats().await
    }

    /// Get current alert level
    pub async fn alert_level(&self) -> TokenAlertLevel {
        self.token_budget.alert_level().await
    }

    /// Get utilization percentage
    pub async fn utilization(&self) -> f64 {
        self.token_budget.usage_percentage().await
    }

    /// Get remaining tokens
    pub async fn remaining_tokens(&self) -> usize {
        self.token_budget.remaining_tokens().await
    }

    /// Get last efficiency metrics
    pub async fn last_efficiency(&self) -> Option<ContextEfficiency> {
        self.last_efficiency.read().await.clone()
    }

    /// Reset token budget
    pub async fn reset(&self) {
        self.token_budget.reset().await;
        *self.last_efficiency.write().await = None;
    }

    /// Update budget from message history
    pub async fn update_from_messages(&self, messages: &[Message]) {
        let mut stats = TokenUsageStats::new();

        for message in messages {
            let tokens = TokenBudgetManager::estimate_tokens(&message_to_text(message));
            match message {
                Message::User { content } => {
                    // Check if this contains tool results
                    let has_tool_result = content
                        .iter()
                        .any(|c| matches!(c, rig::message::UserContent::ToolResult(_)));
                    if has_tool_result {
                        stats.tool_results_tokens += tokens;
                    } else {
                        stats.user_messages_tokens += tokens;
                    }
                }
                Message::Assistant { .. } => stats.assistant_messages_tokens += tokens,
            }
        }

        stats.total_tokens = stats.system_prompt_tokens
            + stats.user_messages_tokens
            + stats.assistant_messages_tokens
            + stats.tool_results_tokens;

        self.token_budget.set_stats(stats).await;

        // Check thresholds and emit events
        self.check_and_emit_alerts().await;
    }

    /// Check thresholds and emit alert events
    async fn check_and_emit_alerts(&self) {
        if let Some(ref tx) = self.event_tx {
            let alert_level = self.token_budget.alert_level().await;
            let stats = self.token_budget.stats().await;
            let utilization = self.token_budget.usage_percentage().await;
            let max_tokens = self.token_budget.config().max_context_tokens;

            let event = match alert_level {
                TokenAlertLevel::Critical => Some(ContextEvent::ContextExceeded {
                    total_tokens: stats.total_tokens,
                    max_tokens,
                }),
                TokenAlertLevel::Alert => Some(ContextEvent::AlertThreshold {
                    utilization,
                    total_tokens: stats.total_tokens,
                    max_tokens,
                }),
                TokenAlertLevel::Warning => Some(ContextEvent::WarningThreshold {
                    utilization,
                    total_tokens: stats.total_tokens,
                    max_tokens,
                }),
                TokenAlertLevel::Normal => None,
            };

            if let Some(event) = event {
                let _ = tx.send(event).await;
            }
        }
    }

    /// Truncate tool response if it exceeds limits
    pub async fn truncate_tool_response(&self, content: &str, tool_name: &str) -> TruncationResult {
        let result = aggregate_tool_output(content, self.trim_config.max_tool_response_tokens);

        if result.truncated {
            // Emit event
            if let Some(ref tx) = self.event_tx {
                let _ = tx
                    .send(ContextEvent::ToolResponseTruncated {
                        original_tokens: TokenBudgetManager::estimate_tokens(content),
                        truncated_tokens: TokenBudgetManager::estimate_tokens(&result.content),
                        tool_name: tool_name.to_string(),
                    })
                    .await;
            }

            tracing::debug!(
                "Tool response '{}' truncated: {} -> {} tokens",
                tool_name,
                TokenBudgetManager::estimate_tokens(content),
                TokenBudgetManager::estimate_tokens(&result.content)
            );
        }

        result
    }

    /// Check if there's room for a new message
    pub async fn can_add_message(&self, estimated_tokens: usize) -> bool {
        !self
            .token_budget
            .would_exceed_budget(estimated_tokens)
            .await
    }

    /// Get context summary for diagnostics
    pub async fn get_summary(&self) -> ContextSummary {
        let stats = self.token_budget.stats().await;
        let config = self.token_budget.config();

        ContextSummary {
            total_tokens: stats.total_tokens,
            max_tokens: config.max_context_tokens,
            available_tokens: config.available_tokens(),
            utilization: self.token_budget.usage_percentage().await,
            alert_level: self.token_budget.alert_level().await,
            system_prompt_tokens: stats.system_prompt_tokens,
            user_messages_tokens: stats.user_messages_tokens,
            assistant_messages_tokens: stats.assistant_messages_tokens,
            tool_results_tokens: stats.tool_results_tokens,
            warning_threshold: config.warning_threshold,
            alert_threshold: config.alert_threshold,
        }
    }

    /// Check if compaction should be triggered.
    ///
    /// This should be called between turns, before starting a new agent loop.
    ///
    /// # Arguments
    /// * `compaction_state` - The current compaction state
    /// * `model` - The model name (for looking up context limits)
    ///
    /// # Returns
    /// A CompactionCheck with the decision and context
    pub fn should_compact(
        &self,
        compaction_state: &CompactionState,
        model: &str,
    ) -> CompactionCheck {
        // Check if already attempted this turn
        if compaction_state.attempted_this_turn {
            return CompactionCheck {
                should_compact: false,
                current_tokens: compaction_state.last_input_tokens.unwrap_or(0),
                max_tokens: TokenBudgetConfig::for_model(model).max_context_tokens,
                threshold: self.token_budget.config().alert_threshold,
                using_heuristic: compaction_state.using_heuristic,
                reason: "Already attempted this turn".to_string(),
            };
        }

        // Check if context management is disabled
        if !self.token_budget_enabled {
            return CompactionCheck {
                should_compact: false,
                current_tokens: compaction_state.last_input_tokens.unwrap_or(0),
                max_tokens: TokenBudgetConfig::for_model(model).max_context_tokens,
                threshold: self.token_budget.config().alert_threshold,
                using_heuristic: compaction_state.using_heuristic,
                reason: "Context management disabled".to_string(),
            };
        }

        // Get current token count
        let current_tokens = compaction_state.last_input_tokens.unwrap_or(0);

        // Get model-specific max tokens
        let model_config = TokenBudgetConfig::for_model(model);
        let max_tokens = model_config.max_context_tokens;

        // Get threshold from config
        let threshold = self.token_budget.config().alert_threshold;

        // Calculate threshold in tokens
        let threshold_tokens = (max_tokens as f64 * threshold) as u64;

        // Determine if we should compact
        let should_compact = current_tokens >= threshold_tokens;

        // Build reason string
        let reason = if should_compact {
            format!(
                "Token usage {}% ({}/{}) exceeds threshold {}%",
                (current_tokens as f64 / max_tokens as f64 * 100.0) as u32,
                current_tokens,
                max_tokens,
                (threshold * 100.0) as u32
            )
        } else {
            format!(
                "Token usage {}% ({}/{}) below threshold {}%",
                (current_tokens as f64 / max_tokens as f64 * 100.0) as u32,
                current_tokens,
                max_tokens,
                (threshold * 100.0) as u32
            )
        };

        CompactionCheck {
            should_compact,
            current_tokens,
            max_tokens,
            threshold,
            using_heuristic: compaction_state.using_heuristic,
            reason,
        }
    }

    /// Check if context has exceeded the absolute limit (session is dead).
    pub fn is_context_exceeded(&self, compaction_state: &CompactionState, model: &str) -> bool {
        let current_tokens = compaction_state.last_input_tokens.unwrap_or(0);
        let model_config = TokenBudgetConfig::for_model(model);
        let max_context_tokens = model_config.max_context_tokens;

        current_tokens >= max_context_tokens as u64
    }
}

/// Summary of current context state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextSummary {
    pub total_tokens: usize,
    pub max_tokens: usize,
    pub available_tokens: usize,
    pub utilization: f64,
    pub alert_level: TokenAlertLevel,
    pub system_prompt_tokens: usize,
    pub user_messages_tokens: usize,
    pub assistant_messages_tokens: usize,
    pub tool_results_tokens: usize,
    pub warning_threshold: f64,
    pub alert_threshold: f64,
}

/// Information about a context warning threshold being exceeded.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextWarningInfo {
    /// Current utilization ratio (0.0-1.0)
    pub utilization: f64,
    /// Total tokens currently in use
    pub total_tokens: usize,
    /// Maximum tokens available
    pub max_tokens: usize,
}

/// Result of enforcing context window limits.
///
/// This struct contains the messages and information about any warnings that occurred.
/// The caller can use this information to emit appropriate `AiEvent` types to the frontend.
/// Note: Pruning has been replaced by compaction via the summarizer agent.
#[derive(Debug, Clone)]
pub struct ContextEnforcementResult {
    /// The messages (unchanged - pruning is no longer performed)
    pub messages: Vec<Message>,
    /// Warning info if utilization exceeded warning threshold
    pub warning_info: Option<ContextWarningInfo>,
}

/// Convert message to text for token estimation
fn message_to_text(message: &Message) -> String {
    use rig::completion::AssistantContent;
    use rig::message::UserContent;

    match message {
        Message::User { content } => content
            .iter()
            .map(|c| match c {
                UserContent::Text(t) => t.text.clone(),
                UserContent::Image(_) => "[image]".to_string(),
                UserContent::Document(_) => "[document]".to_string(),
                UserContent::ToolResult(result) => result
                    .content
                    .iter()
                    .map(|tc| format!("{:?}", tc))
                    .collect::<Vec<_>>()
                    .join("\n"),
                _ => "[media]".to_string(), // Audio, Video, etc.
            })
            .collect::<Vec<_>>()
            .join("\n"),
        Message::Assistant { content, .. } => content
            .iter()
            .map(|c| match c {
                AssistantContent::Text(t) => t.text.clone(),
                AssistantContent::ToolCall(call) => {
                    format!("[tool: {}]", call.function.name)
                }
                _ => "[reasoning]".to_string(), // Reasoning, etc.
            })
            .collect::<Vec<_>>()
            .join("\n"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rig::message::Text;
    use rig::one_or_many::OneOrMany;

    fn create_user_message(text: &str) -> Message {
        Message::User {
            content: OneOrMany::one(rig::message::UserContent::Text(Text {
                text: text.to_string(),
            })),
        }
    }

    fn create_assistant_message(text: &str) -> Message {
        Message::Assistant {
            id: None,
            content: OneOrMany::one(rig::message::AssistantContent::Text(Text {
                text: text.to_string(),
            })),
        }
    }

    #[tokio::test]
    async fn test_context_manager_creation() {
        let manager = ContextManager::for_model("claude-3-5-sonnet");
        // Context management is disabled by default
        assert!(!manager.is_enabled());
        assert_eq!(manager.alert_level().await, TokenAlertLevel::Normal);
    }

    #[tokio::test]
    async fn test_update_from_messages() {
        let manager = ContextManager::for_model("claude-3-5-sonnet");
        let messages = vec![
            create_user_message("Hello, how are you?"),
            create_user_message("I need help with something."),
        ];

        manager.update_from_messages(&messages).await;
        let stats = manager.stats().await;
        assert!(stats.user_messages_tokens > 0);
    }

    #[tokio::test]
    async fn test_tool_response_truncation() {
        let manager = ContextManager::new(
            TokenBudgetConfig::default(),
            ContextTrimConfig {
                max_tool_response_tokens: 10, // Very small for testing
                ..Default::default()
            },
        );

        // Create long content that exceeds MIN_TRUNCATION_LENGTH (100 chars) and is much larger than 10 tokens
        let long_content = "This is a very long tool response that contains a lot of text. \
            We need to ensure that it exceeds the minimum truncation length of 100 characters. \
            This additional text should push us well over that threshold and trigger actual truncation.";
        let result = manager
            .truncate_tool_response(long_content, "test_tool")
            .await;

        // With only 10 max tokens (~40 chars), this should be truncated
        assert!(result.truncated);
        assert!(result.result_chars < long_content.len());
    }

    #[tokio::test]
    async fn test_context_summary() {
        let manager = ContextManager::for_model("claude-3-5-sonnet");
        let summary = manager.get_summary().await;

        assert!(summary.max_tokens > 0);
        assert_eq!(summary.utilization, 0.0);
        assert_eq!(summary.alert_level, TokenAlertLevel::Normal);
    }

    // ==================== ContextManagerConfig Tests ====================

    #[test]
    fn test_context_manager_config_default() {
        let config = ContextManagerConfig::default();
        assert!(config.enabled);
        assert!((config.compaction_threshold - 0.80).abs() < f64::EPSILON);
        assert_eq!(config.protected_turns, 2);
        assert_eq!(config.cooldown_seconds, 60);
    }

    #[tokio::test]
    async fn test_with_config_enables_context_management() {
        let config = ContextManagerConfig::default();
        let manager = ContextManager::with_config("claude-3-5-sonnet", config);

        // Both flags should be enabled
        assert!(manager.is_enabled());
        assert!(manager.trim_config().enabled);
    }

    #[tokio::test]
    async fn test_with_config_disabled_results_in_noop() {
        let config = ContextManagerConfig {
            enabled: false,
            ..Default::default()
        };
        let manager = ContextManager::with_config("claude-3-5-sonnet", config);

        // Both flags should be disabled
        assert!(!manager.is_enabled());
        assert!(!manager.trim_config().enabled);
    }

    #[tokio::test]
    async fn test_with_config_sets_thresholds() {
        let config = ContextManagerConfig {
            enabled: true,
            compaction_threshold: 0.75,
            protected_turns: 3,
            cooldown_seconds: 120,
        };
        let manager = ContextManager::with_config("claude-3-5-sonnet", config);

        // Verify thresholds are set correctly
        let summary = manager.get_summary().await;
        assert!((summary.alert_threshold - 0.75).abs() < f64::EPSILON);
        // Warning threshold should be 0.10 below compaction_threshold
        assert!((summary.warning_threshold - 0.65).abs() < f64::EPSILON);
    }

    #[tokio::test]
    async fn test_for_model_enabled_enables_context_management() {
        let manager = ContextManager::for_model_enabled("claude-3-5-sonnet");

        // Both flags should be enabled
        assert!(manager.is_enabled());
        assert!(manager.trim_config().enabled);
    }

    #[tokio::test]
    async fn test_for_model_disabled_by_default() {
        let manager = ContextManager::for_model("claude-3-5-sonnet");

        // Both flags should be disabled with the original for_model()
        assert!(!manager.is_enabled());
        assert!(!manager.trim_config().enabled);
    }
}

#[cfg(test)]
mod compaction_tests {
    use super::*;

    /// Helper to create a context manager with specific settings for testing.
    fn create_test_manager(enabled: bool, alert_threshold: f64) -> ContextManager {
        let budget_config = TokenBudgetConfig {
            max_context_tokens: 200_000, // Claude context size
            reserved_system_tokens: 0,
            reserved_response_tokens: 0,
            warning_threshold: alert_threshold - 0.10,
            alert_threshold,
            model: "claude-3-5-sonnet".to_string(),
            tokenizer_id: None,
            detailed_tracking: false,
        };

        let trim_config = ContextTrimConfig {
            enabled,
            target_utilization: alert_threshold - 0.10,
            aggressive_on_critical: true,
            max_tool_response_tokens: 25_000,
        };

        ContextManager {
            token_budget: std::sync::Arc::new(TokenBudgetManager::new(budget_config)),
            trim_config,
            token_budget_enabled: enabled,
            last_efficiency: std::sync::Arc::new(tokio::sync::RwLock::new(None)),
            event_tx: None,
        }
    }

    #[test]
    fn test_should_compact_below_threshold() {
        // 50% usage should not trigger compaction (threshold is 80%)
        let manager = create_test_manager(true, 0.80);

        let mut state = CompactionState::new();
        // 50% of 200,000 = 100,000 tokens
        state.update_tokens(100_000);

        let check = manager.should_compact(&state, "claude-3-5-sonnet");

        assert!(
            !check.should_compact,
            "50% usage should not trigger compaction"
        );
        assert_eq!(check.current_tokens, 100_000);
        assert_eq!(check.max_tokens, 200_000);
        assert!((check.threshold - 0.80).abs() < f64::EPSILON);
        assert!(check.reason.contains("below threshold"));
    }

    #[test]
    fn test_should_compact_above_threshold() {
        // 85% usage should trigger compaction (threshold is 80%)
        let manager = create_test_manager(true, 0.80);

        let mut state = CompactionState::new();
        // 85% of 200,000 = 170,000 tokens
        state.update_tokens(170_000);

        let check = manager.should_compact(&state, "claude-3-5-sonnet");

        assert!(check.should_compact, "85% usage should trigger compaction");
        assert_eq!(check.current_tokens, 170_000);
        assert_eq!(check.max_tokens, 200_000);
        assert!((check.threshold - 0.80).abs() < f64::EPSILON);
        assert!(check.reason.contains("exceeds threshold"));
    }

    #[test]
    fn test_should_compact_already_attempted() {
        // Should not trigger if already attempted this turn
        let manager = create_test_manager(true, 0.80);

        let mut state = CompactionState::new();
        state.update_tokens(170_000); // 85% - would normally trigger
        state.mark_attempted(); // But we already tried this turn

        let check = manager.should_compact(&state, "claude-3-5-sonnet");

        assert!(
            !check.should_compact,
            "Should not compact if already attempted"
        );
        assert_eq!(check.reason, "Already attempted this turn");
    }

    #[test]
    fn test_should_compact_disabled() {
        // Should not trigger if context management is disabled
        let manager = create_test_manager(false, 0.80);

        let mut state = CompactionState::new();
        state.update_tokens(170_000); // 85% - would normally trigger

        let check = manager.should_compact(&state, "claude-3-5-sonnet");

        assert!(!check.should_compact, "Should not compact when disabled");
        assert_eq!(check.reason, "Context management disabled");
    }

    #[test]
    fn test_compaction_state_reset_turn() {
        // Verify reset_turn preserves last_input_tokens
        let mut state = CompactionState::new();
        state.update_tokens(150_000);
        state.mark_attempted();
        state.increment_count();

        assert!(state.attempted_this_turn);
        assert_eq!(state.last_input_tokens, Some(150_000));
        assert_eq!(state.compaction_count, 1);

        // Reset turn
        state.reset_turn();

        // attempted_this_turn should be reset
        assert!(!state.attempted_this_turn);
        // last_input_tokens should be preserved
        assert_eq!(state.last_input_tokens, Some(150_000));
        // compaction_count should be preserved
        assert_eq!(state.compaction_count, 1);
    }

    #[test]
    fn test_compaction_state_heuristic() {
        // Verify char/4 estimation
        let mut state = CompactionState::new();

        // 40,000 chars should estimate to ~10,000 tokens
        state.update_tokens_heuristic(40_000);

        assert_eq!(state.last_input_tokens, Some(10_000));
        assert!(state.using_heuristic);

        // Now update with provider tokens
        state.update_tokens(12_000);

        assert_eq!(state.last_input_tokens, Some(12_000));
        assert!(!state.using_heuristic);
    }

    #[test]
    fn test_is_context_exceeded() {
        // Verify boundary detection for absolute context limit
        let manager = create_test_manager(true, 0.80);

        // Just below limit (99.9%)
        let mut state = CompactionState::new();
        state.update_tokens(199_999);
        assert!(
            !manager.is_context_exceeded(&state, "claude-3-5-sonnet"),
            "199,999 tokens should not exceed 200,000 limit"
        );

        // Exactly at limit
        state.update_tokens(200_000);
        assert!(
            manager.is_context_exceeded(&state, "claude-3-5-sonnet"),
            "200,000 tokens should equal/exceed 200,000 limit"
        );

        // Above limit
        state.update_tokens(200_001);
        assert!(
            manager.is_context_exceeded(&state, "claude-3-5-sonnet"),
            "200,001 tokens should exceed 200,000 limit"
        );
    }

    #[test]
    fn test_is_context_exceeded_different_models() {
        let manager = create_test_manager(true, 0.80);

        // Test with GPT-4o (128k context)
        let mut state = CompactionState::new();
        state.update_tokens(127_999);
        assert!(
            !manager.is_context_exceeded(&state, "gpt-4o"),
            "127,999 should not exceed 128,000"
        );

        state.update_tokens(128_000);
        assert!(
            manager.is_context_exceeded(&state, "gpt-4o"),
            "128,000 should exceed 128,000 limit"
        );

        // Test with Gemini (1M context)
        state.update_tokens(999_999);
        assert!(
            !manager.is_context_exceeded(&state, "gemini-pro"),
            "999,999 should not exceed 1,000,000"
        );

        state.update_tokens(1_000_000);
        assert!(
            manager.is_context_exceeded(&state, "gemini-pro"),
            "1,000,000 should exceed 1,000,000 limit"
        );
    }

    #[test]
    fn test_should_compact_at_exact_threshold() {
        // Test behavior at exactly the threshold (edge case)
        let manager = create_test_manager(true, 0.80);

        let mut state = CompactionState::new();
        // Exactly 80% of 200,000 = 160,000 tokens
        state.update_tokens(160_000);

        let check = manager.should_compact(&state, "claude-3-5-sonnet");

        // At exactly threshold, should trigger compaction (>= comparison)
        assert!(
            check.should_compact,
            "Exactly at threshold should trigger compaction"
        );
    }

    #[test]
    fn test_should_compact_with_heuristic() {
        // Verify the heuristic flag is propagated correctly
        let manager = create_test_manager(true, 0.80);

        let mut state = CompactionState::new();
        state.update_tokens_heuristic(680_000); // 680k chars / 4 = 170k tokens (85%)

        let check = manager.should_compact(&state, "claude-3-5-sonnet");

        assert!(check.should_compact);
        assert!(
            check.using_heuristic,
            "Should indicate heuristic is being used"
        );
    }

    #[test]
    fn test_compaction_state_default() {
        let state = CompactionState::default();

        assert!(!state.attempted_this_turn);
        assert_eq!(state.compaction_count, 0);
        assert!(state.last_input_tokens.is_none());
        assert!(!state.using_heuristic);
    }

    #[test]
    fn test_compaction_check_fields() {
        // Verify all CompactionCheck fields are populated correctly
        let manager = create_test_manager(true, 0.75);

        let mut state = CompactionState::new();
        state.update_tokens(150_000); // 75% of 200k

        let check = manager.should_compact(&state, "claude-3-5-sonnet");

        assert!(check.should_compact);
        assert_eq!(check.current_tokens, 150_000);
        assert_eq!(check.max_tokens, 200_000);
        assert!((check.threshold - 0.75).abs() < f64::EPSILON);
        assert!(!check.using_heuristic);
        assert!(!check.reason.is_empty());
    }
}
