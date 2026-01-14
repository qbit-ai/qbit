# Step 5: Hard Reset Mechanism

**Goal:** Wire up the complete compaction flow in `agentic_loop.rs`: detect threshold, call summarizer, clear messages, update system prompt with summary.

**Outcome:** After this step, when context exceeds the threshold, the conversation is automatically compacted and continues with the summary.

---

## Prerequisites

- Step 1 completed (transcript writer)
- Step 2 completed (summarizer agent)
- Step 3 completed (summarizer input builder)
- Step 4 completed (compaction trigger logic)

## Files to Modify

| File | Action | Description |
|------|--------|-------------|
| `backend/crates/qbit-ai/src/agentic_loop.rs` | Modify | Add compaction flow between turns |
| `backend/crates/qbit-ai/src/system_prompt.rs` | Modify | Add helper for summary section |
| `backend/crates/qbit-ai/src/agent_bridge.rs` | Modify | Store summary for system prompt |

---

## Task Breakdown

### 5.1 Add system prompt helper for continuation summary

**File:** `backend/crates/qbit-ai/src/system_prompt.rs`

```rust
/// The continuation section template for compacted sessions.
const CONTINUATION_TEMPLATE: &str = r#"
## Continuation

This is a continuation of a previous session. The conversation so far has been compacted into the summary below. Use this context to continue assisting the user without asking redundant questions.

<summary>
{summary}
</summary>
"#;

/// Append a continuation summary section to a system prompt.
///
/// The summary is placed at the end of the system prompt to ensure
/// it's the most recent context the model sees.
pub fn append_continuation_summary(base_prompt: &str, summary: &str) -> String {
    let continuation = CONTINUATION_TEMPLATE.replace("{summary}", summary);
    format!("{}\n{}", base_prompt.trim_end(), continuation)
}

/// Check if a system prompt already has a continuation section.
pub fn has_continuation_section(prompt: &str) -> bool {
    prompt.contains("## Continuation") && prompt.contains("<summary>")
}

/// Replace an existing continuation section with a new summary.
///
/// If no continuation section exists, appends one.
pub fn update_continuation_summary(prompt: &str, summary: &str) -> String {
    if has_continuation_section(prompt) {
        // Find and replace the existing section
        if let Some(start) = prompt.find("## Continuation") {
            let base = prompt[..start].trim_end();
            return append_continuation_summary(base, summary);
        }
    }
    append_continuation_summary(prompt, summary)
}

#[cfg(test)]
mod continuation_tests {
    use super::*;

    #[test]
    fn test_append_continuation_summary() {
        let base = "You are a helpful assistant.";
        let summary = "User asked to fix a bug in auth.rs.";
        
        let result = append_continuation_summary(base, summary);
        
        assert!(result.starts_with(base));
        assert!(result.contains("## Continuation"));
        assert!(result.contains("<summary>"));
        assert!(result.contains(summary));
        assert!(result.contains("</summary>"));
    }

    #[test]
    fn test_has_continuation_section() {
        let with_section = "Base prompt\n## Continuation\n<summary>test</summary>";
        let without = "Just a regular prompt.";
        
        assert!(has_continuation_section(with_section));
        assert!(!has_continuation_section(without));
    }

    #[test]
    fn test_update_continuation_summary_replaces_existing() {
        let prompt = "Base prompt\n## Continuation\n<summary>old summary</summary>";
        let new_summary = "new updated summary";
        
        let result = update_continuation_summary(prompt, new_summary);
        
        assert!(result.contains(new_summary));
        assert!(!result.contains("old summary"));
        // Should only have one continuation section
        assert_eq!(result.matches("## Continuation").count(), 1);
    }

    #[test]
    fn test_update_continuation_summary_appends_if_missing() {
        let prompt = "Base prompt only";
        let summary = "new summary";
        
        let result = update_continuation_summary(prompt, summary);
        
        assert!(result.contains("## Continuation"));
        assert!(result.contains(summary));
    }
}
```

### 5.2 Add summary storage to AgentBridge

**File:** `backend/crates/qbit-ai/src/agent_bridge.rs`

```rust
// Add to AgentBridge struct:
/// Current continuation summary (from compaction)
continuation_summary: Arc<RwLock<Option<String>>>,

// Initialize in new():
continuation_summary: Arc::new(RwLock::new(None)),

// Add methods:

/// Set the continuation summary (after compaction).
pub async fn set_continuation_summary(&self, summary: String) {
    *self.continuation_summary.write().await = Some(summary);
}

/// Get the current continuation summary.
pub async fn get_continuation_summary(&self) -> Option<String> {
    self.continuation_summary.read().await.clone()
}

/// Clear the continuation summary.
pub async fn clear_continuation_summary(&self) {
    *self.continuation_summary.write().await = None;
}

/// Build the system prompt, including continuation summary if present.
pub async fn build_system_prompt_with_continuation(&self) -> String {
    let base_prompt = self.build_system_prompt().await;
    
    if let Some(summary) = self.get_continuation_summary().await {
        system_prompt::update_continuation_summary(&base_prompt, &summary)
    } else {
        base_prompt
    }
}
```

### 5.3 Create compaction orchestrator function

**File:** `backend/crates/qbit-ai/src/agentic_loop.rs`

Create a dedicated function for the compaction flow:

```rust
use crate::transcript::{build_summarizer_input, save_summarizer_input, save_summary};

/// Result of a compaction attempt.
#[derive(Debug)]
pub struct CompactionResult {
    /// Whether compaction succeeded
    pub success: bool,
    /// The generated summary (if successful)
    pub summary: Option<String>,
    /// Path to the saved summary artifact
    pub summary_path: Option<PathBuf>,
    /// Error message (if failed)
    pub error: Option<String>,
    /// Whether context has exceeded limits (session is dead)
    pub context_exceeded: bool,
}

/// Perform context compaction.
///
/// This function:
/// 1. Reads the transcript
/// 2. Formats it for the summarizer
/// 3. Calls the summarizer
/// 4. Saves artifacts
/// 5. Returns the summary for the caller to use
///
/// The caller is responsible for:
/// - Clearing the message history
/// - Updating the system prompt
/// - Emitting events
async fn perform_compaction(
    session_id: &str,
    session_dir: &Path,
    client: &LlmClient,
    summarizer_model: Option<&str>,
    model_factory: Option<&LlmClientFactory>,
) -> CompactionResult {
    // Step 1: Build summarizer input from transcript
    let summarizer_input = match build_summarizer_input(session_dir, session_id) {
        Ok(input) => input,
        Err(e) => {
            return CompactionResult {
                success: false,
                summary: None,
                summary_path: None,
                error: Some(format!("Failed to read transcript: {}", e)),
                context_exceeded: false,
            };
        }
    };

    // Step 2: Save summarizer input artifact
    if let Err(e) = save_summarizer_input(session_dir, session_id, &summarizer_input) {
        tracing::warn!("Failed to save summarizer input artifact: {}", e);
        // Continue anyway - artifact saving is not critical
    }

    // Step 3: Call summarizer
    let summary_response = match generate_summary_with_config(
        client,
        summarizer_model,
        &summarizer_input,
        model_factory,
    ).await {
        Ok(response) => response,
        Err(e) => {
            return CompactionResult {
                success: false,
                summary: None,
                summary_path: None,
                error: Some(format!("Summarizer failed: {}", e)),
                context_exceeded: false,
            };
        }
    };

    // Step 4: Save summary artifact
    let summary_path = match save_summary(session_dir, session_id, &summary_response.summary) {
        Ok(path) => Some(path),
        Err(e) => {
            tracing::warn!("Failed to save summary artifact: {}", e);
            None
        }
    };

    CompactionResult {
        success: true,
        summary: Some(summary_response.summary),
        summary_path,
        error: None,
        context_exceeded: false,
    }
}

/// Check and perform compaction if needed.
///
/// This should be called between turns (before starting a new agent loop).
/// Returns true if compaction was performed.
async fn maybe_compact(
    ctx: &mut LoopContext<'_>,
    bridge: &AgentBridge,
    messages: &mut Vec<Message>,
) -> Option<CompactionResult> {
    let model = bridge.model_name().await;
    
    // Check if compaction is needed
    let check = ctx.context_manager.should_compact(&ctx.compaction_state, &model);
    
    if !check.should_compact {
        tracing::debug!("Compaction check: {}", check.reason);
        return None;
    }

    tracing::info!(
        "Triggering compaction: {} tokens ({:.1}% of {} limit)",
        check.current_tokens,
        (check.current_tokens as f64 / check.max_tokens as f64) * 100.0,
        check.max_tokens
    );

    // Mark as attempted before we start
    ctx.compaction_state.mark_attempted();

    // Get session info
    let session_id = bridge.session_id().await;
    let transcript_dir = get_transcript_dir(); // VT_TRANSCRIPT_DIR helper

    // Get LLM client and settings
    let client_guard = bridge.client().read().await;
    let client = match client_guard.as_ref() {
        Some(c) => c,
        None => {
            return Some(CompactionResult {
                success: false,
                summary: None,
                summary_path: None,
                error: Some("LLM client not initialized".to_string()),
                context_exceeded: ctx.context_manager.is_context_exceeded(&ctx.compaction_state, &model),
            });
        }
    };

    let settings = bridge.settings_manager().map(|sm| sm.blocking_get());
    let summarizer_model = settings.as_ref().and_then(|s| s.ai.summarizer_model.as_deref());

    // Perform compaction
    let result = perform_compaction(
        &session_id,
        &transcript_dir,
        client,
        summarizer_model,
        bridge.model_factory().map(|f| f.as_ref()),
    ).await;

    if result.success {
        if let Some(ref summary) = result.summary {
            // Clear message history
            messages.clear();
            
            // Set continuation summary on bridge
            bridge.set_continuation_summary(summary.clone()).await;
            
            // Reset compaction state for new conversation
            ctx.compaction_state.reset_turn();
            ctx.compaction_state.increment_count();
            ctx.compaction_state.last_input_tokens = None; // Reset token count
            
            tracing::info!(
                "Compaction successful. Session {} compacted (count: {})",
                session_id,
                ctx.compaction_state.compaction_count
            );
        }
    } else {
        // Check if context is exceeded (session is dead)
        let context_exceeded = ctx.context_manager.is_context_exceeded(&ctx.compaction_state, &model);
        
        return Some(CompactionResult {
            context_exceeded,
            ..result
        });
    }

    Some(result)
}

/// Get the transcript directory from environment.
fn get_transcript_dir() -> PathBuf {
    std::env::var("VT_TRANSCRIPT_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            dirs::home_dir()
                .unwrap_or_default()
                .join(".qbit/transcripts")
        })
}
```

### 5.4 Integrate compaction into the main loop

**File:** `backend/crates/qbit-ai/src/agentic_loop.rs`

Find the main agent loop and add compaction check:

```rust
// In the main execute function, before starting each turn:

pub async fn execute(
    ctx: &mut LoopContext<'_>,
    bridge: &AgentBridge,
    // ... other params
) -> Result<AgentResponse> {
    let mut messages: Vec<Message> = Vec::new();
    
    // ... existing setup ...

    loop {
        // === COMPACTION CHECK (between turns) ===
        // Reset per-turn state
        ctx.compaction_state.reset_turn();
        
        // Check and perform compaction if needed
        if let Some(result) = maybe_compact(ctx, bridge, &mut messages).await {
            if result.success {
                // Emit success event
                let _ = ctx.event_tx.send(AiEvent::ContextCompacted {
                    tokens_before: ctx.compaction_state.last_input_tokens.unwrap_or(0),
                    trigger_reason: if ctx.compaction_state.using_heuristic {
                        "heuristic".to_string()
                    } else {
                        "provider_usage".to_string()
                    },
                    model: bridge.model_name().await,
                    transcript_path: None, // Could add if needed
                    summary_path: result.summary_path.map(|p| p.to_string_lossy().to_string()),
                    compaction_count: ctx.compaction_state.compaction_count,
                }).await;
            } else {
                // Emit failure event
                let _ = ctx.event_tx.send(AiEvent::ContextCompactionFailed {
                    error: result.error.unwrap_or_else(|| "Unknown error".to_string()),
                    context_exceeded: result.context_exceeded,
                    tokens_current: ctx.compaction_state.last_input_tokens.unwrap_or(0),
                    max_tokens: TokenBudgetConfig::for_model(&bridge.model_name().await).max_context_tokens as u64,
                }).await;
                
                if result.context_exceeded {
                    // Session is dead - return error
                    return Err(anyhow::anyhow!(
                        "Context limit exceeded and compaction failed. Please start a new session."
                    ));
                }
            }
        }
        // === END COMPACTION CHECK ===

        // Build system prompt (now includes continuation summary if present)
        let system_prompt = bridge.build_system_prompt_with_continuation().await;

        // ... rest of the existing loop ...
    }
}
```

### 5.5 Add tests for the compaction flow

**File:** `backend/crates/qbit-ai/src/agentic_loop.rs` (or separate test file)

```rust
#[cfg(test)]
mod compaction_integration_tests {
    use super::*;
    use tempfile::TempDir;

    // Mock/stub tests for the compaction flow
    
    #[test]
    fn test_get_transcript_dir_from_env() {
        std::env::set_var("VT_TRANSCRIPT_DIR", "/tmp/test-transcripts");
        let dir = get_transcript_dir();
        assert_eq!(dir, PathBuf::from("/tmp/test-transcripts"));
        std::env::remove_var("VT_TRANSCRIPT_DIR");
    }

    #[test]
    fn test_get_transcript_dir_default() {
        std::env::remove_var("VT_TRANSCRIPT_DIR");
        let dir = get_transcript_dir();
        assert!(dir.to_string_lossy().contains(".qbit/transcripts"));
    }

    #[tokio::test]
    async fn test_compaction_clears_messages() {
        // This would be an integration test with mocked LLM
        // For now, just verify the structure compiles
    }
}
```

### 5.6 Update LoopContext to include compaction state

**File:** `backend/crates/qbit-ai/src/agentic_loop.rs`

```rust
// Add to LoopContext struct (or wherever loop state is defined):

pub struct LoopContext<'a> {
    // ... existing fields ...
    
    /// Context manager for token budgeting and compaction
    pub context_manager: &'a ContextManager,
    
    /// Compaction state for this session
    pub compaction_state: CompactionState,
}

// Initialize in the execute function:
let mut ctx = LoopContext {
    // ... existing fields ...
    context_manager: bridge.context_manager(),
    compaction_state: CompactionState::new(),
};
```

---

## Verification

### Run Tests
```bash
cd backend
cargo test -p qbit-ai compaction
cargo test -p qbit-ai continuation
```

### Manual Testing

1. **Simulate high token usage:**
   - Start a session with a model that has a lower limit (or temporarily lower the threshold)
   - Have a long conversation with many tool calls
   - Verify compaction triggers when threshold is reached

2. **Verify message clearing:**
   - After compaction, check that message history is empty
   - Next response should not reference pre-compaction conversation details

3. **Verify summary in prompt:**
   - After compaction, check logs for system prompt
   - Should contain `## Continuation` section with summary

4. **Check artifacts:**
   - Look in `~/.qbit/transcripts/{session_id}/` for `summarizer-input-*.md` and `summary-*.md` files

### Integration Check
```bash
cd backend
cargo test
```

---

## Definition of Done

- [ ] `append_continuation_summary()` helper implemented
- [ ] `update_continuation_summary()` handles replacement
- [ ] `AgentBridge` stores and retrieves continuation summary
- [ ] `build_system_prompt_with_continuation()` includes summary
- [ ] `perform_compaction()` orchestrates the full flow
- [ ] `maybe_compact()` checks threshold and triggers compaction
- [ ] Message history is cleared on successful compaction
- [ ] Compaction state is properly tracked
- [ ] Artifacts (input + summary) are saved
- [ ] Events are emitted (success and failure) - Note: actual event types added in Step 6
- [ ] All tests pass
- [ ] Existing tests still pass

---

## Notes

- The compaction happens synchronously between turns - UI should show a loading state
- If compaction fails and context is exceeded, the session becomes unusable
- The continuation summary replaces any previous summary (supports multiple compactions)
- Token counts reset after compaction since the context is now just the summary
- This step prepares for Step 6 which adds the actual event types
