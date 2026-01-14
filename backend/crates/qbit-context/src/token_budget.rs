//! Token budget management for AI context windows
//!
//! Implements token counting and budget allocation based on VTCode's design.
// Public API for future context management integration
#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Token usage for a single completion request
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TokenUsage {
    pub input_tokens: u64,
    pub output_tokens: u64,
}

impl TokenUsage {
    /// Create a new TokenUsage with specified input and output tokens
    pub fn new(input_tokens: u64, output_tokens: u64) -> Self {
        Self {
            input_tokens,
            output_tokens,
        }
    }

    /// Calculate total tokens (input + output)
    pub fn total(&self) -> u64 {
        self.input_tokens + self.output_tokens
    }
}

/// Maximum tokens allowed for tool responses before truncation
pub const MAX_TOOL_RESPONSE_TOKENS: usize = 25_000;

/// Default context window size (Claude 3.5 Sonnet)
pub const DEFAULT_MAX_CONTEXT_TOKENS: usize = 128_000;

/// Average characters per token for rough estimation
const CHARS_PER_TOKEN: f64 = 4.0;

/// Model-specific context window sizes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelContextLimits {
    // Claude models
    pub claude_3_5_sonnet: usize,
    pub claude_3_opus: usize,
    pub claude_3_haiku: usize,
    pub claude_4_sonnet: usize,
    pub claude_4_opus: usize,
    pub claude_4_5_opus: usize,
    pub claude_4_5_sonnet: usize,
    pub claude_4_5_haiku: usize,
    // OpenAI models
    pub gpt_4o: usize,
    pub gpt_4_turbo: usize,
    pub gpt_4_1: usize,
    pub gpt_5_1: usize,
    pub gpt_5_2: usize,
    pub o1: usize,
    pub o3: usize,
    // Google models
    pub gemini_pro: usize,
    pub gemini_flash: usize,
}

impl Default for ModelContextLimits {
    fn default() -> Self {
        Self {
            // Claude models: 200k context
            claude_3_5_sonnet: 200_000,
            claude_3_opus: 200_000,
            claude_3_haiku: 200_000,
            claude_4_sonnet: 200_000,
            claude_4_opus: 200_000,
            claude_4_5_opus: 200_000,
            claude_4_5_sonnet: 200_000,
            claude_4_5_haiku: 200_000,
            // OpenAI models
            gpt_4o: 128_000,
            gpt_4_turbo: 128_000,
            gpt_4_1: 1_047_576, // GPT-4.1 has ~1M context
            gpt_5_1: 1_047_576, // GPT-5.1 has ~1M context
            gpt_5_2: 1_047_576, // GPT-5.2 has ~1M context
            o1: 200_000,
            o3: 200_000,
            // Google models: 1M context
            gemini_pro: 1_000_000,
            gemini_flash: 1_000_000,
        }
    }
}

/// Configuration for token budget management
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenBudgetConfig {
    /// Maximum tokens allowed in context window
    pub max_context_tokens: usize,
    /// Threshold (0.0-1.0) at which to warn about token usage
    pub warning_threshold: f64,
    /// Threshold (0.0-1.0) at which to alert about token usage
    pub alert_threshold: f64,
    /// Model identifier for context-specific limits
    pub model: String,
    /// Optional custom tokenizer ID
    pub tokenizer_id: Option<String>,
    /// Enable detailed per-component token tracking
    pub detailed_tracking: bool,
    /// Reserved tokens for system prompt (subtracted from available budget)
    pub reserved_system_tokens: usize,
    /// Reserved tokens for assistant response
    pub reserved_response_tokens: usize,
}

impl Default for TokenBudgetConfig {
    fn default() -> Self {
        Self {
            max_context_tokens: DEFAULT_MAX_CONTEXT_TOKENS,
            warning_threshold: 0.75,
            alert_threshold: 0.85,
            model: "claude-3-5-sonnet".to_string(),
            tokenizer_id: None,
            detailed_tracking: false,
            reserved_system_tokens: 4_000,
            reserved_response_tokens: 8_192,
        }
    }
}

impl TokenBudgetConfig {
    /// Create config for a specific model
    pub fn for_model(model: &str) -> Self {
        let limits = ModelContextLimits::default();
        let model_lower = model.to_lowercase();
        let max_context = match model_lower.as_str() {
            // Claude models (check 4.5 before 4 to avoid false matches)
            m if m.contains("claude-3-5-sonnet") => limits.claude_3_5_sonnet,
            m if m.contains("claude-3-opus") => limits.claude_3_opus,
            m if m.contains("claude-3-haiku") => limits.claude_3_haiku,
            m if m.contains("claude-4-5-opus")
                || m.contains("claude-4.5-opus")
                || m.contains("claude-opus-4-5")
                || m.contains("claude-opus-4.5") =>
            {
                limits.claude_4_5_opus
            }
            m if m.contains("claude-4-5-sonnet")
                || m.contains("claude-4.5-sonnet")
                || m.contains("claude-sonnet-4-5")
                || m.contains("claude-sonnet-4.5") =>
            {
                limits.claude_4_5_sonnet
            }
            m if m.contains("claude-4-5-haiku")
                || m.contains("claude-4.5-haiku")
                || m.contains("claude-haiku-4-5")
                || m.contains("claude-haiku-4.5") =>
            {
                limits.claude_4_5_haiku
            }
            m if m.contains("claude-4-sonnet") || m.contains("claude-sonnet-4") => {
                limits.claude_4_sonnet
            }
            m if m.contains("claude-4-opus") || m.contains("claude-opus-4") => limits.claude_4_opus,
            // OpenAI GPT-5.x (check before gpt-4 to avoid false matches)
            m if m.contains("gpt-5.2") || m.contains("gpt-5-2") => limits.gpt_5_2,
            m if m.contains("gpt-5.1") || m.contains("gpt-5-1") => limits.gpt_5_1,
            // OpenAI GPT-4.1 (check before gpt-4 to avoid false matches)
            m if m.contains("gpt-4.1") || m.contains("gpt-4-1") => limits.gpt_4_1,
            // OpenAI GPT-4 variants
            m if m.contains("gpt-4o") => limits.gpt_4o,
            m if m.contains("gpt-4-turbo") => limits.gpt_4_turbo,
            // OpenAI o-series reasoning models
            m if m.contains("o3") => limits.o3,
            m if m.contains("o1") => limits.o1,
            // Google Gemini models (check specific variants before generic)
            m if m.contains("gemini") && m.contains("flash") => limits.gemini_flash,
            m if m.contains("gemini") && m.contains("pro") => limits.gemini_pro,
            m if m.contains("gemini") => limits.gemini_pro, // Default Gemini to Pro
            // Default fallback
            _ => DEFAULT_MAX_CONTEXT_TOKENS,
        };

        Self {
            max_context_tokens: max_context,
            model: model.to_string(),
            ..Default::default()
        }
    }

    /// Calculate available tokens after reservations
    pub fn available_tokens(&self) -> usize {
        self.max_context_tokens
            .saturating_sub(self.reserved_system_tokens)
            .saturating_sub(self.reserved_response_tokens)
    }
}

/// Statistics tracking token usage across different components
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TokenUsageStats {
    /// Total tokens currently in context
    pub total_tokens: usize,
    /// Tokens used by system prompt
    pub system_prompt_tokens: usize,
    /// Tokens used by user messages
    pub user_messages_tokens: usize,
    /// Tokens used by assistant messages
    pub assistant_messages_tokens: usize,
    /// Tokens used by tool results
    pub tool_results_tokens: usize,
    /// Tokens used by decision ledger/history
    pub decision_ledger_tokens: usize,
    /// Unix timestamp of last update
    pub timestamp: u64,
}

impl TokenUsageStats {
    /// Create new stats with current timestamp
    pub fn new() -> Self {
        Self {
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            ..Default::default()
        }
    }

    /// Reset all counters
    pub fn reset(&mut self) {
        *self = Self::new();
    }

    /// Calculate total excluding system prompt
    pub fn conversation_tokens(&self) -> usize {
        self.user_messages_tokens
            + self.assistant_messages_tokens
            + self.tool_results_tokens
            + self.decision_ledger_tokens
    }
}

/// Alert level for token usage
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TokenAlertLevel {
    /// Below warning threshold
    Normal,
    /// Above warning threshold but below alert
    Warning,
    /// Above alert threshold
    Alert,
    /// Context window exceeded
    Critical,
}

/// Manages token budget for a conversation
#[derive(Debug)]
pub struct TokenBudgetManager {
    config: TokenBudgetConfig,
    stats: Arc<RwLock<TokenUsageStats>>,
}

impl TokenBudgetManager {
    /// Create a new token budget manager
    pub fn new(config: TokenBudgetConfig) -> Self {
        Self {
            config,
            stats: Arc::new(RwLock::new(TokenUsageStats::new())),
        }
    }

    /// Create with default config
    pub fn default_for_model(model: &str) -> Self {
        Self::new(TokenBudgetConfig::for_model(model))
    }

    /// Get the current configuration
    pub fn config(&self) -> &TokenBudgetConfig {
        &self.config
    }

    /// Update configuration
    pub fn set_config(&mut self, config: TokenBudgetConfig) {
        self.config = config;
    }

    /// Get current token usage stats
    pub async fn stats(&self) -> TokenUsageStats {
        self.stats.read().await.clone()
    }

    /// Reset token usage stats
    pub async fn reset(&self) {
        let mut stats = self.stats.write().await;
        stats.reset();
    }

    /// Estimate tokens for text content
    pub fn estimate_tokens(text: &str) -> usize {
        // Simple estimation: ~4 characters per token
        // More accurate would use tiktoken or similar
        (text.len() as f64 / CHARS_PER_TOKEN).ceil() as usize
    }

    /// Calculate usage percentage (0.0 - 1.0+)
    pub async fn usage_percentage(&self) -> f64 {
        let stats = self.stats.read().await;
        stats.total_tokens as f64 / self.config.max_context_tokens as f64
    }

    /// Check if usage exceeds warning threshold
    pub async fn exceeds_warning(&self) -> bool {
        self.usage_percentage().await > self.config.warning_threshold
    }

    /// Check if usage exceeds alert threshold
    pub async fn exceeds_alert(&self) -> bool {
        self.usage_percentage().await > self.config.alert_threshold
    }

    /// Get current alert level
    pub async fn alert_level(&self) -> TokenAlertLevel {
        let usage = self.usage_percentage().await;
        if usage >= 1.0 {
            TokenAlertLevel::Critical
        } else if usage > self.config.alert_threshold {
            TokenAlertLevel::Alert
        } else if usage > self.config.warning_threshold {
            TokenAlertLevel::Warning
        } else {
            TokenAlertLevel::Normal
        }
    }

    /// Calculate remaining available tokens
    pub async fn remaining_tokens(&self) -> usize {
        let stats = self.stats.read().await;
        self.config
            .available_tokens()
            .saturating_sub(stats.total_tokens)
    }

    /// Update system prompt tokens
    pub async fn set_system_prompt_tokens(&self, tokens: usize) {
        let mut stats = self.stats.write().await;
        stats.system_prompt_tokens = tokens;
        self.update_total(&mut stats);
    }

    /// Add tokens for a user message
    pub async fn add_user_message(&self, tokens: usize) {
        let mut stats = self.stats.write().await;
        stats.user_messages_tokens += tokens;
        self.update_total(&mut stats);
    }

    /// Add tokens for an assistant message
    pub async fn add_assistant_message(&self, tokens: usize) {
        let mut stats = self.stats.write().await;
        stats.assistant_messages_tokens += tokens;
        self.update_total(&mut stats);
    }

    /// Add tokens for a tool result
    pub async fn add_tool_result(&self, tokens: usize) {
        let mut stats = self.stats.write().await;
        stats.tool_results_tokens += tokens;
        self.update_total(&mut stats);
    }

    /// Update total token count
    fn update_total(&self, stats: &mut TokenUsageStats) {
        stats.total_tokens = stats.system_prompt_tokens
            + stats.user_messages_tokens
            + stats.assistant_messages_tokens
            + stats.tool_results_tokens
            + stats.decision_ledger_tokens;
        stats.timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
    }

    /// Check if adding tokens would exceed budget
    pub async fn would_exceed_budget(&self, additional_tokens: usize) -> bool {
        let stats = self.stats.read().await;
        stats.total_tokens + additional_tokens > self.config.available_tokens()
    }

    /// Calculate how many tokens need to be pruned to fit new content
    pub async fn tokens_to_prune(&self, new_tokens: usize) -> usize {
        let stats = self.stats.read().await;
        let needed = stats.total_tokens + new_tokens;
        let available = self.config.available_tokens();
        needed.saturating_sub(available)
    }

    /// Set stats directly (useful for initialization from message history)
    pub async fn set_stats(&self, new_stats: TokenUsageStats) {
        let mut stats = self.stats.write().await;
        *stats = new_stats;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_token_estimation() {
        let text = "Hello, world!"; // 13 chars ≈ 4 tokens
        let estimate = TokenBudgetManager::estimate_tokens(text);
        assert!((3..=5).contains(&estimate));
    }

    #[tokio::test]
    async fn test_usage_tracking() {
        let manager = TokenBudgetManager::new(TokenBudgetConfig {
            max_context_tokens: 1000,
            ..Default::default()
        });

        manager.add_user_message(100).await;
        manager.add_assistant_message(200).await;
        manager.add_tool_result(50).await;

        let stats = manager.stats().await;
        assert_eq!(stats.total_tokens, 350);
        assert_eq!(stats.user_messages_tokens, 100);
        assert_eq!(stats.assistant_messages_tokens, 200);
        assert_eq!(stats.tool_results_tokens, 50);
    }

    #[tokio::test]
    async fn test_alert_levels() {
        let manager = TokenBudgetManager::new(TokenBudgetConfig {
            max_context_tokens: 1000,
            warning_threshold: 0.5,
            alert_threshold: 0.8,
            reserved_system_tokens: 0,
            reserved_response_tokens: 0,
            ..Default::default()
        });

        // Normal
        manager.add_user_message(400).await;
        assert_eq!(manager.alert_level().await, TokenAlertLevel::Normal);

        // Warning
        manager.add_user_message(200).await;
        assert_eq!(manager.alert_level().await, TokenAlertLevel::Warning);

        // Alert
        manager.add_user_message(300).await;
        assert_eq!(manager.alert_level().await, TokenAlertLevel::Alert);

        // Critical
        manager.add_user_message(200).await;
        assert_eq!(manager.alert_level().await, TokenAlertLevel::Critical);
    }

    #[tokio::test]
    async fn test_model_config() {
        let config = TokenBudgetConfig::for_model("claude-3-5-sonnet");
        assert_eq!(config.max_context_tokens, 200_000);

        let config = TokenBudgetConfig::for_model("unknown-model");
        assert_eq!(config.max_context_tokens, DEFAULT_MAX_CONTEXT_TOKENS);
    }

    #[test]
    fn test_model_context_limits_claude_4_5() {
        // Claude 4.5 Opus
        let config = TokenBudgetConfig::for_model("claude-opus-4-5-20251101");
        assert_eq!(config.max_context_tokens, 200_000);

        let config = TokenBudgetConfig::for_model("claude-4-5-opus");
        assert_eq!(config.max_context_tokens, 200_000);

        let config = TokenBudgetConfig::for_model("claude-4.5-opus");
        assert_eq!(config.max_context_tokens, 200_000);

        let config = TokenBudgetConfig::for_model("claude-opus-4.5");
        assert_eq!(config.max_context_tokens, 200_000);

        // Claude 4.5 Sonnet
        let config = TokenBudgetConfig::for_model("claude-sonnet-4-5-20251101");
        assert_eq!(config.max_context_tokens, 200_000);

        let config = TokenBudgetConfig::for_model("claude-4-5-sonnet");
        assert_eq!(config.max_context_tokens, 200_000);

        let config = TokenBudgetConfig::for_model("claude-4.5-sonnet");
        assert_eq!(config.max_context_tokens, 200_000);

        let config = TokenBudgetConfig::for_model("claude-sonnet-4.5");
        assert_eq!(config.max_context_tokens, 200_000);

        // Claude 4.5 Haiku
        let config = TokenBudgetConfig::for_model("claude-haiku-4-5-20251101");
        assert_eq!(config.max_context_tokens, 200_000);

        let config = TokenBudgetConfig::for_model("claude-4-5-haiku");
        assert_eq!(config.max_context_tokens, 200_000);

        let config = TokenBudgetConfig::for_model("claude-4.5-haiku");
        assert_eq!(config.max_context_tokens, 200_000);

        let config = TokenBudgetConfig::for_model("claude-haiku-4.5");
        assert_eq!(config.max_context_tokens, 200_000);
    }

    #[test]
    fn test_model_context_limits_gpt_5() {
        // GPT-5.1
        let config = TokenBudgetConfig::for_model("gpt-5.1");
        assert_eq!(config.max_context_tokens, 1_047_576);

        let config = TokenBudgetConfig::for_model("gpt-5-1");
        assert_eq!(config.max_context_tokens, 1_047_576);

        let config = TokenBudgetConfig::for_model("gpt-5.1-preview");
        assert_eq!(config.max_context_tokens, 1_047_576);

        // GPT-5.2
        let config = TokenBudgetConfig::for_model("gpt-5.2");
        assert_eq!(config.max_context_tokens, 1_047_576);

        let config = TokenBudgetConfig::for_model("gpt-5-2");
        assert_eq!(config.max_context_tokens, 1_047_576);

        let config = TokenBudgetConfig::for_model("gpt-5.2-preview");
        assert_eq!(config.max_context_tokens, 1_047_576);
    }

    // ==================== TokenUsage Tests ====================

    #[test]
    fn test_token_usage_new() {
        let usage = TokenUsage::new(1000, 500);
        assert_eq!(usage.input_tokens, 1000);
        assert_eq!(usage.output_tokens, 500);
    }

    #[test]
    fn test_token_usage_total() {
        let usage = TokenUsage::new(1500, 300);
        assert_eq!(usage.total(), 1800);
    }

    #[test]
    fn test_token_usage_default() {
        let usage = TokenUsage::default();
        assert_eq!(usage.input_tokens, 0);
        assert_eq!(usage.output_tokens, 0);
        assert_eq!(usage.total(), 0);
    }

    #[test]
    fn test_token_usage_accumulation() {
        // Simulates accumulating tokens across multiple LLM calls in a single agent turn
        let mut total = TokenUsage::default();

        // First LLM call
        let call1 = TokenUsage::new(5000, 200);
        total.input_tokens += call1.input_tokens;
        total.output_tokens += call1.output_tokens;

        // Second LLM call (after tool execution)
        let call2 = TokenUsage::new(5500, 150);
        total.input_tokens += call2.input_tokens;
        total.output_tokens += call2.output_tokens;

        // Third LLM call
        let call3 = TokenUsage::new(6000, 300);
        total.input_tokens += call3.input_tokens;
        total.output_tokens += call3.output_tokens;

        assert_eq!(total.input_tokens, 16500);
        assert_eq!(total.output_tokens, 650);
        assert_eq!(total.total(), 17150);
    }

    #[test]
    fn test_token_usage_serialization() {
        let usage = TokenUsage::new(12345, 6789);
        let json = serde_json::to_string(&usage).unwrap();
        assert!(json.contains("\"input_tokens\":12345"));
        assert!(json.contains("\"output_tokens\":6789"));

        // Deserialize back
        let parsed: TokenUsage = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.input_tokens, 12345);
        assert_eq!(parsed.output_tokens, 6789);
    }

    #[test]
    fn test_token_usage_large_values() {
        // Test with realistic large token counts (200k context window)
        let usage = TokenUsage::new(180_000, 4_000);
        assert_eq!(usage.total(), 184_000);
    }

    // ==================== Model Context Limits Tests ====================

    #[test]
    fn test_model_context_limits_gpt() {
        // GPT-4o
        let config = TokenBudgetConfig::for_model("gpt-4o");
        assert_eq!(config.max_context_tokens, 128_000);

        let config = TokenBudgetConfig::for_model("gpt-4o-2024-08-06");
        assert_eq!(config.max_context_tokens, 128_000);

        // GPT-4 Turbo
        let config = TokenBudgetConfig::for_model("gpt-4-turbo");
        assert_eq!(config.max_context_tokens, 128_000);

        let config = TokenBudgetConfig::for_model("gpt-4-turbo-preview");
        assert_eq!(config.max_context_tokens, 128_000);

        // GPT-4.1 (1M context)
        let config = TokenBudgetConfig::for_model("gpt-4.1");
        assert_eq!(config.max_context_tokens, 1_047_576);

        let config = TokenBudgetConfig::for_model("gpt-4-1");
        assert_eq!(config.max_context_tokens, 1_047_576);

        let config = TokenBudgetConfig::for_model("gpt-4.1-preview");
        assert_eq!(config.max_context_tokens, 1_047_576);
    }

    #[test]
    fn test_model_context_limits_gemini() {
        // Gemini Pro
        let config = TokenBudgetConfig::for_model("gemini-pro");
        assert_eq!(config.max_context_tokens, 1_000_000);

        let config = TokenBudgetConfig::for_model("gemini-1.5-pro");
        assert_eq!(config.max_context_tokens, 1_000_000);

        let config = TokenBudgetConfig::for_model("gemini-2.0-pro");
        assert_eq!(config.max_context_tokens, 1_000_000);

        // Gemini Flash
        let config = TokenBudgetConfig::for_model("gemini-flash");
        assert_eq!(config.max_context_tokens, 1_000_000);

        let config = TokenBudgetConfig::for_model("gemini-1.5-flash");
        assert_eq!(config.max_context_tokens, 1_000_000);

        let config = TokenBudgetConfig::for_model("gemini-2.0-flash");
        assert_eq!(config.max_context_tokens, 1_000_000);

        // Generic Gemini defaults to Pro
        let config = TokenBudgetConfig::for_model("gemini");
        assert_eq!(config.max_context_tokens, 1_000_000);

        let config = TokenBudgetConfig::for_model("gemini-1.5");
        assert_eq!(config.max_context_tokens, 1_000_000);
    }

    #[test]
    fn test_model_context_limits_o_series() {
        // o1 model
        let config = TokenBudgetConfig::for_model("o1");
        assert_eq!(config.max_context_tokens, 200_000);

        let config = TokenBudgetConfig::for_model("o1-preview");
        assert_eq!(config.max_context_tokens, 200_000);

        let config = TokenBudgetConfig::for_model("o1-mini");
        assert_eq!(config.max_context_tokens, 200_000);

        // o3 model
        let config = TokenBudgetConfig::for_model("o3");
        assert_eq!(config.max_context_tokens, 200_000);

        let config = TokenBudgetConfig::for_model("o3-mini");
        assert_eq!(config.max_context_tokens, 200_000);
    }
}
