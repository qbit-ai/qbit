# Step 4: Compaction Trigger Logic

**Goal:** Update `ContextManager` to detect when compaction should be triggered based on token usage. Use provider-returned token counts, with a char/4 heuristic fallback.

**Outcome:** After this step, we can call `should_compact()` to determine if compaction is needed, based on actual token usage vs. model limits.

---

## Prerequisites

- Steps 1-3 completed (transcript + summarizer infrastructure ready)
- Understanding of existing `ContextManager` and `TokenBudgetManager`

## Files to Modify

| File | Action | Description |
|------|--------|-------------|
| `backend/crates/qbit-context/src/context_manager.rs` | Modify | Add compaction trigger logic |
| `backend/crates/qbit-context/src/token_budget.rs` | Modify | Extend model limits, add helper methods |
| `backend/crates/qbit-ai/src/agentic_loop.rs` | Modify | Track token usage from LLM responses |

---

## Task Breakdown

### 4.1 Extend ModelContextLimits with more models

**File:** `backend/crates/qbit-context/src/token_budget.rs`

```rust
// Update ModelContextLimits struct:

/// Model-specific context window sizes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelContextLimits {
    // Claude models
    pub claude_3_5_sonnet: usize,
    pub claude_3_opus: usize,
    pub claude_3_haiku: usize,
    pub claude_4_sonnet: usize,
    pub claude_4_opus: usize,
    // OpenAI models
    pub gpt_4o: usize,
    pub gpt_4_turbo: usize,
    pub gpt_4_1: usize,
    pub o1: usize,
    pub o3: usize,
    // Google models
    pub gemini_pro: usize,
    pub gemini_flash: usize,
}

impl Default for ModelContextLimits {
    fn default() -> Self {
        Self {
            // Claude models - 200k context
            claude_3_5_sonnet: 200_000,
            claude_3_opus: 200_000,
            claude_3_haiku: 200_000,
            claude_4_sonnet: 200_000,
            claude_4_opus: 200_000,
            // OpenAI models
            gpt_4o: 128_000,
            gpt_4_turbo: 128_000,
            gpt_4_1: 1_047_576,  // GPT-4.1 has ~1M context
            o1: 200_000,
            o3: 200_000,
            // Google models - 1M+ context
            gemini_pro: 1_000_000,
            gemini_flash: 1_000_000,
        }
    }
}

// Update TokenBudgetConfig::for_model():

impl TokenBudgetConfig {
    /// Create config for a specific model
    pub fn for_model(model: &str) -> Self {
        let limits = ModelContextLimits::default();
        let max_context = match model {
            // Claude models
            m if m.contains("claude-3-5-sonnet") => limits.claude_3_5_sonnet,
            m if m.contains("claude-3-opus") => limits.claude_3_opus,
            m if m.contains("claude-3-haiku") => limits.claude_3_haiku,
            m if m.contains("claude-4-sonnet") || m.contains("claude-sonnet-4") => {
                limits.claude_4_sonnet
            }
            m if m.contains("claude-4-opus") || m.contains("claude-opus-4") => limits.claude_4_opus,
            m if m.contains("claude-4-5") || m.contains("claude-haiku-4") => limits.claude_4_sonnet,
            
            // OpenAI models
            m if m.contains("gpt-4o") => limits.gpt_4o,
            m if m.contains("gpt-4-turbo") => limits.gpt_4_turbo,
            m if m.contains("gpt-4.1") || m.contains("gpt-4-1") => limits.gpt_4_1,
            m if m.contains("o1") => limits.o1,
            m if m.contains("o3") => limits.o3,
            
            // Gemini models
            m if m.contains("gemini") && m.contains("pro") => limits.gemini_pro,
            m if m.contains("gemini") && m.contains("flash") => limits.gemini_flash,
            m if m.contains("gemini") => limits.gemini_pro, // Default Gemini
            
            // Default fallback
            _ => DEFAULT_MAX_CONTEXT_TOKENS,
        };

        Self {
            max_context_tokens: max_context,
            model: model.to_string(),
            ..Default::default()
        }
    }
}
```

**Tests:**
```rust
#[test]
fn test_model_context_limits_gpt() {
    let config = TokenBudgetConfig::for_model("gpt-4o");
    assert_eq!(config.max_context_tokens, 128_000);
    
    let config = TokenBudgetConfig::for_model("gpt-4.1");
    assert_eq!(config.max_context_tokens, 1_047_576);
}

#[test]
fn test_model_context_limits_gemini() {
    let config = TokenBudgetConfig::for_model("gemini-1.5-pro");
    assert_eq!(config.max_context_tokens, 1_000_000);
    
    let config = TokenBudgetConfig::for_model("gemini-flash");
    assert_eq!(config.max_context_tokens, 1_000_000);
}

#[test]
fn test_model_context_limits_o_series() {
    let config = TokenBudgetConfig::for_model("o1-preview");
    assert_eq!(config.max_context_tokens, 200_000);
    
    let config = TokenBudgetConfig::for_model("o3-mini");
    assert_eq!(config.max_context_tokens, 200_000);
}
```

### 4.2 Add CompactionState and trigger detection

**File:** `backend/crates/qbit-context/src/context_manager.rs`

```rust
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
    /// Create a new compaction state
    pub fn new() -> Self {
        Self::default()
    }

    /// Reset the per-turn state (called at start of new turn)
    pub fn reset_turn(&mut self) {
        self.attempted_this_turn = false;
    }

    /// Mark that compaction was attempted this turn
    pub fn mark_attempted(&mut self) {
        self.attempted_this_turn = true;
    }

    /// Increment compaction count (after successful compaction)
    pub fn increment_count(&mut self) {
        self.compaction_count += 1;
    }

    /// Update token count from provider response
    pub fn update_tokens(&mut self, input_tokens: u64) {
        self.last_input_tokens = Some(input_tokens);
        self.using_heuristic = false;
    }

    /// Update token count using heuristic (when provider doesn't return usage)
    pub fn update_tokens_heuristic(&mut self, char_count: usize) {
        // Approximate: ~4 characters per token
        let estimated_tokens = (char_count / 4) as u64;
        self.last_input_tokens = Some(estimated_tokens);
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
```

### 4.3 Add compaction trigger method to ContextManager

**File:** `backend/crates/qbit-context/src/context_manager.rs`

```rust
impl ContextManager {
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
        // If already attempted this turn, don't try again
        if compaction_state.attempted_this_turn {
            return CompactionCheck {
                should_compact: false,
                current_tokens: compaction_state.last_input_tokens.unwrap_or(0),
                max_tokens: self.token_budget.config().max_context_tokens,
                threshold: self.token_budget.config().alert_threshold,
                using_heuristic: compaction_state.using_heuristic,
                reason: "Already attempted this turn".to_string(),
            };
        }

        // If context management is disabled, don't compact
        if !self.token_budget_enabled {
            return CompactionCheck {
                should_compact: false,
                current_tokens: compaction_state.last_input_tokens.unwrap_or(0),
                max_tokens: self.token_budget.config().max_context_tokens,
                threshold: self.token_budget.config().alert_threshold,
                using_heuristic: compaction_state.using_heuristic,
                reason: "Context management disabled".to_string(),
            };
        }

        // Get token counts
        let current_tokens = compaction_state.last_input_tokens.unwrap_or(0);
        
        // Look up model-specific limits
        let model_config = TokenBudgetConfig::for_model(model);
        let max_tokens = model_config.max_context_tokens;
        let threshold = self.token_budget.config().alert_threshold; // compaction_threshold
        
        // Calculate threshold
        let threshold_tokens = (max_tokens as f64 * threshold) as u64;
        
        let should_compact = current_tokens >= threshold_tokens;
        
        let reason = if should_compact {
            format!(
                "Token usage {} exceeds threshold {} ({}% of {})",
                current_tokens,
                threshold_tokens,
                (threshold * 100.0) as u32,
                max_tokens
            )
        } else {
            format!(
                "Token usage {} below threshold {} ({}% of {})",
                current_tokens,
                threshold_tokens,
                (threshold * 100.0) as u32,
                max_tokens
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
    pub fn is_context_exceeded(
        &self,
        compaction_state: &CompactionState,
        model: &str,
    ) -> bool {
        let current_tokens = compaction_state.last_input_tokens.unwrap_or(0);
        let model_config = TokenBudgetConfig::for_model(model);
        current_tokens as usize >= model_config.max_context_tokens
    }
}

#[cfg(test)]
mod compaction_tests {
    use super::*;

    #[test]
    fn test_should_compact_below_threshold() {
        let config = ContextManagerConfig {
            enabled: true,
            compaction_threshold: 0.80,
            ..Default::default()
        };
        let manager = ContextManager::with_config("claude-3-5-sonnet", config);
        
        let mut state = CompactionState::new();
        state.update_tokens(100_000); // 50% of 200k
        
        let check = manager.should_compact(&state, "claude-3-5-sonnet");
        
        assert!(!check.should_compact);
        assert_eq!(check.current_tokens, 100_000);
        assert_eq!(check.max_tokens, 200_000);
    }

    #[test]
    fn test_should_compact_above_threshold() {
        let config = ContextManagerConfig {
            enabled: true,
            compaction_threshold: 0.80,
            ..Default::default()
        };
        let manager = ContextManager::with_config("claude-3-5-sonnet", config);
        
        let mut state = CompactionState::new();
        state.update_tokens(170_000); // 85% of 200k
        
        let check = manager.should_compact(&state, "claude-3-5-sonnet");
        
        assert!(check.should_compact);
        assert_eq!(check.current_tokens, 170_000);
    }

    #[test]
    fn test_should_compact_already_attempted() {
        let config = ContextManagerConfig {
            enabled: true,
            compaction_threshold: 0.80,
            ..Default::default()
        };
        let manager = ContextManager::with_config("claude-3-5-sonnet", config);
        
        let mut state = CompactionState::new();
        state.update_tokens(170_000);
        state.mark_attempted();
        
        let check = manager.should_compact(&state, "claude-3-5-sonnet");
        
        assert!(!check.should_compact);
        assert!(check.reason.contains("Already attempted"));
    }

    #[test]
    fn test_should_compact_disabled() {
        let config = ContextManagerConfig {
            enabled: false,
            ..Default::default()
        };
        let manager = ContextManager::with_config("claude-3-5-sonnet", config);
        
        let mut state = CompactionState::new();
        state.update_tokens(190_000); // 95%
        
        let check = manager.should_compact(&state, "claude-3-5-sonnet");
        
        assert!(!check.should_compact);
        assert!(check.reason.contains("disabled"));
    }

    #[test]
    fn test_compaction_state_reset_turn() {
        let mut state = CompactionState::new();
        state.update_tokens(100_000);
        state.mark_attempted();
        
        assert!(state.attempted_this_turn);
        
        state.reset_turn();
        
        assert!(!state.attempted_this_turn);
        assert_eq!(state.last_input_tokens, Some(100_000)); // Tokens preserved
    }

    #[test]
    fn test_compaction_state_heuristic() {
        let mut state = CompactionState::new();
        
        // 400,000 characters ≈ 100,000 tokens
        state.update_tokens_heuristic(400_000);
        
        assert_eq!(state.last_input_tokens, Some(100_000));
        assert!(state.using_heuristic);
    }

    #[test]
    fn test_is_context_exceeded() {
        let config = ContextManagerConfig::default();
        let manager = ContextManager::with_config("claude-3-5-sonnet", config);
        
        let mut state = CompactionState::new();
        state.update_tokens(199_000);
        assert!(!manager.is_context_exceeded(&state, "claude-3-5-sonnet"));
        
        state.update_tokens(200_001);
        assert!(manager.is_context_exceeded(&state, "claude-3-5-sonnet"));
    }
}
```

### 4.4 Update agentic_loop to track token usage

**File:** `backend/crates/qbit-ai/src/agentic_loop.rs`

Find where LLM responses are processed and extract token usage:

```rust
// Add to LoopContext or wherever state is tracked:
compaction_state: CompactionState,

// After each LLM call completes (where TokenUsage is available):
if let Some(usage) = response.get_token_usage() {
    ctx.compaction_state.update_tokens(usage.input_tokens);
    tracing::debug!(
        "Token usage: {} input, {} output",
        usage.input_tokens,
        usage.output_tokens
    );
} else {
    // Fallback: estimate from message content
    let total_chars: usize = messages.iter()
        .map(|m| estimate_message_chars(m))
        .sum();
    ctx.compaction_state.update_tokens_heuristic(total_chars);
    tracing::debug!(
        "Token usage (heuristic): ~{} estimated from {} chars",
        total_chars / 4,
        total_chars
    );
}

// At the start of each turn:
ctx.compaction_state.reset_turn();

// Helper function:
fn estimate_message_chars(message: &Message) -> usize {
    // Rough character count for a message
    match message {
        Message::User { content } => {
            content.iter().map(|c| match c {
                UserContent::Text(t) => t.text.len(),
                UserContent::ToolResult(r) => {
                    r.content.iter().map(|tc| format!("{:?}", tc).len()).sum()
                }
                _ => 100, // Estimate for images, etc.
            }).sum()
        }
        Message::Assistant { content, .. } => {
            content.iter().map(|c| match c {
                AssistantContent::Text(t) => t.text.len(),
                AssistantContent::ToolCall(call) => {
                    call.function.name.len() + 
                    serde_json::to_string(&call.function.arguments)
                        .map(|s| s.len())
                        .unwrap_or(100)
                }
                _ => 100,
            }).sum()
        }
    }
}
```

### 4.5 Export new types from lib.rs

**File:** `backend/crates/qbit-context/src/lib.rs`

```rust
pub use context_manager::{
    // ... existing exports ...
    CompactionState,
    CompactionCheck,
};
```

---

## Verification

### Run Tests
```bash
cd backend
cargo test -p qbit-context compaction
cargo test -p qbit-context model_context_limits
```

### Manual Verification
1. Start a session and check logs for token usage tracking
2. Verify different models get correct context limits
3. Test that `should_compact()` returns expected results

### Integration Check
```bash
cd backend
cargo test
```

---

## Definition of Done

- [ ] `ModelContextLimits` extended with GPT, Gemini, o-series models
- [ ] `TokenBudgetConfig::for_model()` updated with new match arms
- [ ] `CompactionState` struct implemented
- [ ] `should_compact()` method implemented on ContextManager
- [ ] `is_context_exceeded()` method implemented
- [ ] Token usage extracted from LLM responses in agentic_loop
- [ ] Heuristic fallback works when provider doesn't return usage
- [ ] All tests pass
- [ ] Existing tests still pass

---

## Notes

- This step only adds the detection logic - actual compaction happens in Step 5
- The `attempted_this_turn` flag prevents compaction loops
- Token usage comes from provider when available, falls back to char/4 heuristic
- Different models have very different context limits (128k to 1M+)
- The check is designed to be called between turns, not during tool execution
