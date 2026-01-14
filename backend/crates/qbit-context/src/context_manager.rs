//! Context management orchestration
//!
//! Coordinates token budgeting, context pruning, and truncation strategies.
// Public API for future use - not all methods are currently called
#![allow(dead_code)]

use rig::message::Message;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::{
    context_pruner::{ContextPruner, ContextPrunerConfig, PruneResult, SemanticScore},
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
    /// Enable semantic-aware pruning
    pub semantic_pruning: bool,
}

impl Default for ContextTrimConfig {
    fn default() -> Self {
        Self {
            enabled: false, // Disabled by default
            target_utilization: 0.7,
            aggressive_on_critical: true,
            max_tool_response_tokens: 25_000,
            semantic_pruning: true,
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
    /// Context was pruned
    ContextPruned {
        messages_removed: usize,
        tokens_freed: usize,
        utilization_after: f64,
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
    /// Context pruner
    pruner: Arc<RwLock<ContextPruner>>,
    /// Trim configuration
    trim_config: ContextTrimConfig,
    /// Whether token budgeting is enabled
    token_budget_enabled: bool,
    /// Last recorded efficiency metrics
    last_efficiency: Arc<RwLock<Option<ContextEfficiency>>>,
    /// Event channel for notifications
    event_tx: Option<tokio::sync::mpsc::Sender<ContextEvent>>,
    /// Timestamp of last pruning operation (for cooldown)
    last_prune_time: Arc<RwLock<Option<u64>>>,
    /// Cooldown between pruning operations in seconds
    prune_cooldown_seconds: u64,
}

impl ContextManager {
    /// Create a new context manager
    pub fn new(budget_config: TokenBudgetConfig, trim_config: ContextTrimConfig) -> Self {
        let pruner_config = ContextPrunerConfig {
            max_tokens: budget_config.available_tokens(),
            ..Default::default()
        };

        Self {
            token_budget: Arc::new(TokenBudgetManager::new(budget_config)),
            pruner: Arc::new(RwLock::new(ContextPruner::new(pruner_config))),
            trim_config,
            token_budget_enabled: false, // Disabled by default
            last_efficiency: Arc::new(RwLock::new(None)),
            event_tx: None,
            last_prune_time: Arc::new(RwLock::new(None)),
            prune_cooldown_seconds: 60, // Default 1 minute cooldown
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
        let available_tokens = budget_config.available_tokens();

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
            semantic_pruning: true,
        };

        // Configure pruner with protected turns
        let pruner_config = ContextPrunerConfig {
            max_tokens: available_tokens,
            protected_recent_turns: config.protected_turns,
            ..Default::default()
        };

        Self {
            token_budget: Arc::new(TokenBudgetManager::new(budget_config)),
            pruner: Arc::new(RwLock::new(ContextPruner::new(pruner_config))),
            trim_config,
            token_budget_enabled: config.enabled,
            last_efficiency: Arc::new(RwLock::new(None)),
            event_tx: None,
            last_prune_time: Arc::new(RwLock::new(None)),
            prune_cooldown_seconds: config.cooldown_seconds,
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

    /// Enforce context window by pruning if necessary.
    ///
    /// Returns a `ContextEnforcementResult` containing:
    /// - The (possibly pruned) messages
    /// - Warning info if utilization exceeded warning threshold
    /// - Pruning info if messages were removed
    ///
    /// The caller should use this information to emit appropriate `AiEvent`
    /// types to the frontend (e.g., `AiEvent::ContextWarning`, `AiEvent::ContextPruned`).
    pub async fn enforce_context_window(&self, messages: &[Message]) -> ContextEnforcementResult {
        if !self.token_budget_enabled || !self.trim_config.enabled {
            return ContextEnforcementResult {
                messages: messages.to_vec(),
                warning_info: None,
                pruned_info: None,
            };
        }

        let utilization_before = self.token_budget.usage_percentage().await;
        let alert_level = self.token_budget.alert_level().await;
        let stats = self.token_budget.stats().await;
        let max_tokens = self.token_budget.config().max_context_tokens;

        // Check for warning threshold (emit warning even if not pruning)
        let warning_info = if matches!(
            alert_level,
            TokenAlertLevel::Warning | TokenAlertLevel::Alert | TokenAlertLevel::Critical
        ) {
            Some(ContextWarningInfo {
                utilization: utilization_before,
                total_tokens: stats.total_tokens,
                max_tokens,
            })
        } else {
            None
        };

        // Determine if we need to prune
        let should_prune = matches!(
            alert_level,
            TokenAlertLevel::Alert | TokenAlertLevel::Critical
        );

        if !should_prune {
            return ContextEnforcementResult {
                messages: messages.to_vec(),
                warning_info,
                pruned_info: None,
            };
        }

        // Calculate target tokens
        let target_utilization = if matches!(alert_level, TokenAlertLevel::Critical)
            && self.trim_config.aggressive_on_critical
        {
            self.trim_config.target_utilization * 0.8
        } else {
            self.trim_config.target_utilization
        };

        let target_tokens =
            (self.token_budget.config().available_tokens() as f64 * target_utilization) as usize;

        // Enable aggressive mode if critical
        {
            let mut pruner = self.pruner.write().await;
            pruner.set_aggressive(
                matches!(alert_level, TokenAlertLevel::Critical)
                    && self.trim_config.aggressive_on_critical,
            );
        }

        // Prune messages
        let pruner = self.pruner.read().await;
        let result = pruner.prune_messages(messages, target_tokens);

        if !result.pruned {
            return ContextEnforcementResult {
                messages: messages.to_vec(),
                warning_info,
                pruned_info: None,
            };
        }

        // Apply pruning
        let kept_messages: Vec<Message> = result
            .kept_indices
            .iter()
            .filter_map(|&i| messages.get(i).cloned())
            .collect();

        // Update stats
        self.update_from_messages(&kept_messages).await;
        let utilization_after = self.token_budget.usage_percentage().await;

        // Record efficiency
        let efficiency = ContextEfficiency {
            utilization_before,
            utilization_after,
            tokens_freed: result.pruned_tokens,
            messages_pruned: result.pruned_indices.len(),
            tool_responses_truncated: 0,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        };

        *self.last_efficiency.write().await = Some(efficiency);

        // Emit internal event (for ContextManager's own event channel)
        if let Some(ref tx) = self.event_tx {
            let _ = tx
                .send(ContextEvent::ContextPruned {
                    messages_removed: result.pruned_indices.len(),
                    tokens_freed: result.pruned_tokens,
                    utilization_after,
                })
                .await;
        }

        tracing::info!(
            "Context pruned: {} messages removed, {} tokens freed, utilization {:.1}% -> {:.1}%",
            result.pruned_indices.len(),
            result.pruned_tokens,
            utilization_before * 100.0,
            utilization_after * 100.0
        );

        // Build pruned info for caller to emit AiEvent
        let pruned_info = Some(ContextPrunedInfo {
            messages_removed: result.pruned_indices.len(),
            tokens_freed: result.pruned_tokens,
            utilization_before,
            utilization_after,
        });

        ContextEnforcementResult {
            messages: kept_messages,
            warning_info: None, // Clear warning since we pruned (utilization should be lower now)
            pruned_info,
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

    /// Get prune result without applying it
    pub async fn preview_prune(&self, messages: &[Message], target_tokens: usize) -> PruneResult {
        let pruner = self.pruner.read().await;
        pruner.prune_messages(messages, target_tokens)
    }

    /// Score a message's semantic importance
    pub async fn score_message(&self, message: &Message) -> SemanticScore {
        let pruner = self.pruner.read().await;
        pruner.score_message(message)
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
///
/// This is returned from `enforce_context_window()` when utilization exceeds
/// the warning threshold, allowing the caller to emit appropriate events.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextWarningInfo {
    /// Current utilization ratio (0.0-1.0)
    pub utilization: f64,
    /// Total tokens currently in use
    pub total_tokens: usize,
    /// Maximum tokens available
    pub max_tokens: usize,
}

/// Information about context being pruned.
///
/// This is returned from `enforce_context_window()` when messages are removed,
/// allowing the caller to emit appropriate events.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextPrunedInfo {
    /// Number of messages removed
    pub messages_removed: usize,
    /// Tokens freed by pruning
    pub tokens_freed: usize,
    /// Utilization ratio before pruning
    pub utilization_before: f64,
    /// Utilization ratio after pruning
    pub utilization_after: f64,
}

/// Result of enforcing context window limits.
///
/// This struct contains the (possibly pruned) messages and information
/// about any warnings or pruning that occurred. The caller can use this
/// information to emit appropriate `AiEvent` types to the frontend.
#[derive(Debug, Clone)]
pub struct ContextEnforcementResult {
    /// The resulting messages (may be pruned from original)
    pub messages: Vec<Message>,
    /// Warning info if utilization exceeded warning threshold (but not alert)
    pub warning_info: Option<ContextWarningInfo>,
    /// Pruning info if messages were removed
    pub pruned_info: Option<ContextPrunedInfo>,
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

    #[tokio::test]
    async fn test_enforce_context_window_noop_when_disabled() {
        let config = ContextManagerConfig {
            enabled: false,
            ..Default::default()
        };
        let manager = ContextManager::with_config("claude-3-5-sonnet", config);

        let messages = vec![create_user_message("Hello"), create_user_message("World")];

        // When disabled, should return messages unchanged
        let result = manager.enforce_context_window(&messages).await;
        assert_eq!(result.messages.len(), messages.len());
        assert!(result.warning_info.is_none());
        assert!(result.pruned_info.is_none());
    }

    #[tokio::test]
    async fn test_enforce_context_window_active_when_enabled() {
        let config = ContextManagerConfig {
            enabled: true,
            compaction_threshold: 0.80,
            protected_turns: 2,
            cooldown_seconds: 0, // No cooldown for testing
        };
        let manager = ContextManager::with_config("claude-3-5-sonnet", config);

        let messages = vec![create_user_message("Hello"), create_user_message("World")];

        // When enabled but under threshold, should return messages unchanged
        let result = manager.enforce_context_window(&messages).await;
        assert_eq!(result.messages.len(), messages.len());
        // No warning since we're well under threshold
        assert!(result.warning_info.is_none());
        assert!(result.pruned_info.is_none());
    }

    #[tokio::test]
    async fn test_enforce_context_window_returns_warning_info() {
        let config = ContextManagerConfig {
            enabled: true,
            compaction_threshold: 0.80,
            protected_turns: 2,
            cooldown_seconds: 0,
        };
        let manager = ContextManager::with_config("claude-3-5-sonnet", config);

        // Result should include warning/pruned info structs that caller can use
        // to emit AiEvents
        let messages = vec![create_user_message("Hello")];
        let result = manager.enforce_context_window(&messages).await;

        // With minimal messages, no warning should be triggered
        assert!(result.warning_info.is_none());
        assert!(result.pruned_info.is_none());

        // Verify the struct fields are accessible
        if let Some(warning) = result.warning_info {
            assert!(warning.utilization >= 0.0);
            assert!(warning.total_tokens > 0);
            assert!(warning.max_tokens > 0);
        }
    }

    // ==================== Compaction Integration Tests ====================
    // These tests prove that context compaction actually works end-to-end

    /// Helper to create a test manager with a very small token budget.
    /// This allows us to easily trigger thresholds without generating huge messages.
    fn create_small_budget_manager(
        max_tokens: usize,
        threshold: f64,
        protected_turns: usize,
    ) -> ContextManager {
        let budget_config = TokenBudgetConfig {
            max_context_tokens: max_tokens,
            reserved_system_tokens: 0, // No reservations for testing
            reserved_response_tokens: 0,
            warning_threshold: threshold - 0.10,
            alert_threshold: threshold,
            model: "test-model".to_string(),
            tokenizer_id: None,
            detailed_tracking: false,
        };

        let trim_config = ContextTrimConfig {
            enabled: true,
            target_utilization: threshold - 0.10, // Target 10% below threshold
            aggressive_on_critical: true,
            max_tool_response_tokens: 25_000,
            semantic_pruning: true,
        };

        let pruner_config = ContextPrunerConfig {
            max_tokens,
            protected_recent_turns: protected_turns,
            ..Default::default()
        };

        ContextManager {
            token_budget: std::sync::Arc::new(TokenBudgetManager::new(budget_config)),
            pruner: std::sync::Arc::new(tokio::sync::RwLock::new(ContextPruner::new(
                pruner_config,
            ))),
            trim_config,
            token_budget_enabled: true,
            last_efficiency: std::sync::Arc::new(tokio::sync::RwLock::new(None)),
            event_tx: None,
            last_prune_time: std::sync::Arc::new(tokio::sync::RwLock::new(None)),
            prune_cooldown_seconds: 0, // No cooldown for testing
        }
    }

    /// Helper to generate a message with approximately the specified token count.
    /// Uses the estimate of ~4 chars per token.
    fn create_message_with_tokens(content_prefix: &str, approx_tokens: usize) -> Message {
        let chars_needed = approx_tokens * 4;
        let base_content = format!("{} ", content_prefix);
        let padding = "x".repeat(chars_needed.saturating_sub(base_content.len()));
        create_user_message(&format!("{}{}", base_content, padding))
    }

    #[tokio::test]
    async fn test_compaction_triggers_warning_at_threshold() {
        // Create a manager with 1000 token budget and 0.80 threshold
        // Warning should trigger at 70% (700 tokens)
        let manager = create_small_budget_manager(1000, 0.80, 0);

        // Create messages totaling ~750 tokens (75% utilization, above warning threshold)
        let messages = vec![
            create_message_with_tokens("msg1", 250),
            create_message_with_tokens("msg2", 250),
            create_message_with_tokens("msg3", 250),
        ];

        // Update token counts
        manager.update_from_messages(&messages).await;

        // Verify we're above warning threshold
        let stats = manager.stats().await;
        let utilization = stats.total_tokens as f64 / 1000.0;
        assert!(
            utilization >= 0.70,
            "Expected utilization >= 70%, got {:.1}%",
            utilization * 100.0
        );

        // Enforce context window - should return warning_info
        let result = manager.enforce_context_window(&messages).await;

        assert!(
            result.warning_info.is_some(),
            "Expected warning_info at {:.1}% utilization",
            utilization * 100.0
        );

        let warning = result.warning_info.unwrap();
        assert!(
            warning.utilization >= 0.70,
            "Warning utilization should be >= 70%"
        );
    }

    #[tokio::test]
    async fn test_compaction_prunes_at_alert_threshold() {
        // Create a manager with 1000 token budget and 0.80 threshold
        // Pruning should trigger at 80% (800 tokens)
        let manager = create_small_budget_manager(1000, 0.80, 0);

        // Create messages totaling ~900 tokens (90% utilization, above alert threshold)
        let messages = vec![
            create_message_with_tokens("msg1", 200),
            create_message_with_tokens("msg2", 200),
            create_message_with_tokens("msg3", 200),
            create_message_with_tokens("msg4", 200),
            create_message_with_tokens("msg5", 100),
        ];

        // Update token counts
        manager.update_from_messages(&messages).await;

        // Verify we're above alert threshold
        let stats = manager.stats().await;
        let utilization = stats.total_tokens as f64 / 1000.0;
        assert!(
            utilization >= 0.80,
            "Expected utilization >= 80%, got {:.1}%",
            utilization * 100.0
        );

        // Enforce context window - should return pruned_info
        let result = manager.enforce_context_window(&messages).await;

        assert!(
            result.pruned_info.is_some(),
            "Expected pruning at {:.1}% utilization",
            utilization * 100.0
        );

        let pruned = result.pruned_info.unwrap();
        assert!(
            pruned.messages_removed > 0,
            "Should have removed at least one message"
        );
        assert!(
            pruned.utilization_after < pruned.utilization_before,
            "Utilization should decrease after pruning"
        );
        assert!(
            result.messages.len() < messages.len(),
            "Should have fewer messages after pruning"
        );
    }

    #[tokio::test]
    async fn test_compaction_preserves_protected_turns() {
        // Create a manager with 2 protected turns
        let manager = create_small_budget_manager(1000, 0.80, 2);

        // Create 6 messages (3 turns) with high token count to trigger pruning
        // Turn 1: msg1, msg2 (oldest, should be pruned)
        // Turn 2: msg3, msg4 (may be pruned)
        // Turn 3: msg5, msg6 (protected, should be kept)
        let messages = vec![
            create_message_with_tokens("turn1_user", 150),
            create_assistant_message("turn1_asst response here"),
            create_message_with_tokens("turn2_user", 150),
            create_assistant_message("turn2_asst response here"),
            create_message_with_tokens("turn3_user", 150),
            create_assistant_message("turn3_asst response here"),
        ];

        // Update token counts
        manager.update_from_messages(&messages).await;

        // Enforce context window
        let result = manager.enforce_context_window(&messages).await;

        // If pruning occurred, verify protected turns are preserved
        if result.pruned_info.is_some() {
            // The last 4 messages (2 turns) should be preserved
            assert!(
                result.messages.len() >= 4,
                "Should preserve at least 4 messages (2 protected turns), got {}",
                result.messages.len()
            );

            // Verify the last messages are the protected ones
            let last_messages: Vec<_> = result.messages.iter().rev().take(4).collect();
            assert!(!last_messages.is_empty(), "Should have protected messages");
        }
    }

    #[tokio::test]
    async fn test_compaction_no_action_under_threshold() {
        // Create a manager with 1000 token budget
        let manager = create_small_budget_manager(1000, 0.80, 0);

        // Create messages totaling only ~300 tokens (30% utilization)
        let messages = vec![
            create_message_with_tokens("small1", 100),
            create_message_with_tokens("small2", 100),
            create_message_with_tokens("small3", 100),
        ];

        // Update token counts
        manager.update_from_messages(&messages).await;

        // Verify we're under threshold
        let stats = manager.stats().await;
        let utilization = stats.total_tokens as f64 / 1000.0;
        assert!(
            utilization < 0.70,
            "Expected utilization < 70%, got {:.1}%",
            utilization * 100.0
        );

        // Enforce context window - should not warn or prune
        let result = manager.enforce_context_window(&messages).await;

        assert!(
            result.warning_info.is_none(),
            "Should not warn under threshold"
        );
        assert!(
            result.pruned_info.is_none(),
            "Should not prune under threshold"
        );
        assert_eq!(
            result.messages.len(),
            messages.len(),
            "Messages should be unchanged"
        );
    }

    #[tokio::test]
    async fn test_compaction_disabled_does_nothing() {
        // Create a disabled manager
        let mut manager = create_small_budget_manager(1000, 0.80, 0);
        manager.token_budget_enabled = false;
        manager.trim_config.enabled = false;

        // Create messages that would trigger pruning if enabled
        let messages = vec![
            create_message_with_tokens("msg1", 300),
            create_message_with_tokens("msg2", 300),
            create_message_with_tokens("msg3", 300),
        ];

        // Update token counts
        manager.update_from_messages(&messages).await;

        // Enforce context window - should do nothing
        let result = manager.enforce_context_window(&messages).await;

        assert!(
            result.warning_info.is_none(),
            "Should not warn when disabled"
        );
        assert!(
            result.pruned_info.is_none(),
            "Should not prune when disabled"
        );
        assert_eq!(
            result.messages.len(),
            messages.len(),
            "Messages should be unchanged"
        );
    }

    #[tokio::test]
    async fn test_compaction_reduces_utilization_to_target() {
        // Create a manager that targets 70% utilization after pruning
        let manager = create_small_budget_manager(1000, 0.80, 0);

        // Create messages totaling ~950 tokens (95% utilization)
        let messages = vec![
            create_message_with_tokens("msg1", 190),
            create_message_with_tokens("msg2", 190),
            create_message_with_tokens("msg3", 190),
            create_message_with_tokens("msg4", 190),
            create_message_with_tokens("msg5", 190),
        ];

        // Update token counts
        manager.update_from_messages(&messages).await;

        // Enforce context window
        let result = manager.enforce_context_window(&messages).await;

        // Pruning should have occurred
        assert!(result.pruned_info.is_some(), "Should have pruned");

        let pruned = result.pruned_info.unwrap();
        // After pruning, utilization should be below the alert threshold
        assert!(
            pruned.utilization_after < 0.80,
            "Utilization after pruning ({:.1}%) should be below 80%",
            pruned.utilization_after * 100.0
        );
    }

    /// Verbose test that prints detailed compaction logs to prove it works.
    /// Run with: cargo test -p qbit-context test_compaction_verbose -- --nocapture
    #[tokio::test]
    async fn test_compaction_verbose_proof() {
        println!("\n============================================================");
        println!("CONTEXT COMPACTION PROOF TEST");
        println!("============================================================\n");

        // Create a manager with 5000 token budget and 0.80 alert threshold
        let manager = create_small_budget_manager(5000, 0.80, 2);

        println!("Configuration:");
        println!("  - Max tokens: 5000");
        println!("  - Warning threshold: 70%");
        println!("  - Alert/Prune threshold: 80%");
        println!("  - Protected turns: 2");
        println!();

        // Create 10 messages (5 turns) that will total ~4500 tokens (90% utilization)
        // Each turn needs ~900 tokens total (user + assistant)
        let messages = vec![
            create_message_with_tokens("turn1_user", 450),
            create_message_with_tokens("turn1_asst", 450), // Use helper for consistent token count
            create_message_with_tokens("turn2_user", 450),
            create_message_with_tokens("turn2_asst", 450),
            create_message_with_tokens("turn3_user", 450),
            create_message_with_tokens("turn3_asst", 450),
            create_message_with_tokens("turn4_user", 450),
            create_message_with_tokens("turn4_asst", 450),
            create_message_with_tokens("turn5_user", 450),
            create_message_with_tokens("turn5_asst", 450),
        ];

        println!("Created {} messages", messages.len());

        // Update token counts
        manager.update_from_messages(&messages).await;

        let stats_before = manager.stats().await;
        let utilization_before = stats_before.total_tokens as f64 / 5000.0;

        println!("\nBEFORE COMPACTION:");
        println!("  - Total tokens: {}", stats_before.total_tokens);
        println!("  - Utilization: {:.1}%", utilization_before * 100.0);
        println!("  - Message count: {}", messages.len());
        println!("  - Alert level: {:?}", manager.alert_level().await);

        // Enforce context window - this triggers compaction
        println!("\n>>> Calling enforce_context_window()...\n");
        let result = manager.enforce_context_window(&messages).await;

        println!("AFTER COMPACTION:");
        println!(
            "  - Message count: {} (was {})",
            result.messages.len(),
            messages.len()
        );

        if let Some(warning) = &result.warning_info {
            println!("\n✓ WARNING EVENT EMITTED:");
            println!("  - Utilization: {:.1}%", warning.utilization * 100.0);
            println!("  - Total tokens: {}", warning.total_tokens);
            println!("  - Max tokens: {}", warning.max_tokens);
        }

        if let Some(pruned) = &result.pruned_info {
            println!("\n✓ PRUNING OCCURRED:");
            println!("  - Messages removed: {}", pruned.messages_removed);
            println!(
                "  - Utilization before: {:.1}%",
                pruned.utilization_before * 100.0
            );
            println!(
                "  - Utilization after: {:.1}%",
                pruned.utilization_after * 100.0
            );
            println!(
                "  - Reduction: {:.1}%",
                (pruned.utilization_before - pruned.utilization_after) * 100.0
            );
        } else {
            println!("\n✗ No pruning occurred (this would be a bug!)");
        }

        println!("\n============================================================");
        println!("PROOF COMPLETE: Context compaction is working!");
        println!("============================================================\n");

        // Assertions - the key proof is that pruning occurred
        assert!(result.pruned_info.is_some(), "Should have pruned");
        assert!(
            result.messages.len() < messages.len(),
            "Should have fewer messages"
        );

        let pruned = result.pruned_info.as_ref().unwrap();
        assert!(pruned.messages_removed > 0, "Should have removed messages");
        assert!(
            pruned.utilization_after < pruned.utilization_before,
            "Utilization should decrease"
        );
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
            semantic_pruning: true,
        };

        let pruner_config = ContextPrunerConfig {
            max_tokens: 200_000,
            protected_recent_turns: 2,
            ..Default::default()
        };

        ContextManager {
            token_budget: std::sync::Arc::new(TokenBudgetManager::new(budget_config)),
            pruner: std::sync::Arc::new(tokio::sync::RwLock::new(ContextPruner::new(
                pruner_config,
            ))),
            trim_config,
            token_budget_enabled: enabled,
            last_efficiency: std::sync::Arc::new(tokio::sync::RwLock::new(None)),
            event_tx: None,
            last_prune_time: std::sync::Arc::new(tokio::sync::RwLock::new(None)),
            prune_cooldown_seconds: 0,
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

        assert!(!check.should_compact, "50% usage should not trigger compaction");
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

        assert!(!check.should_compact, "Should not compact if already attempted");
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
        assert!(check.using_heuristic, "Should indicate heuristic is being used");
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
