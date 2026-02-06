//! Context management extension for AgentBridge.
//!
//! This module contains methods for managing context window and token budgeting.
//! Note: Pruning has been replaced by compaction in the summarizer agent.

use super::agent_bridge::AgentBridge;
use qbit_context::token_budget::{TokenAlertLevel, TokenUsageStats};
use qbit_context::{ContextSummary, ContextTrimConfig};
use qbit_core::events::AiEvent;

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

    /// Retry context compaction manually.
    ///
    /// This reads the transcript, generates a summary, and replaces the conversation history.
    /// Emits CompactionStarted/Completed/Failed events like the automatic compaction path.
    pub async fn retry_compaction(&self) -> Result<(), String> {
        use crate::agentic_loop::{
            apply_compaction, get_artifacts_dir, get_summaries_dir, get_transcript_dir,
        };

        let session_id = self
            .event_session_id
            .as_deref()
            .ok_or_else(|| "No session ID available".to_string())?;

        let messages_before = self.conversation_history.read().await.len();

        // Estimate current tokens
        let tokens_before = {
            let compaction_state = self.compaction_state.read().await;
            compaction_state.last_input_tokens.unwrap_or(0)
        };

        // Emit started event
        self.emit_event(AiEvent::CompactionStarted {
            tokens_before,
            messages_before,
        });

        let transcript_dir = get_transcript_dir();
        let artifacts_dir = get_artifacts_dir();
        let summaries_dir = get_summaries_dir();

        // Build summarizer input from transcript
        let summarizer_input =
            crate::transcript::build_summarizer_input(&transcript_dir, session_id)
                .await
                .map_err(|e| {
                    let error = format!("Failed to build summarizer input: {}", e);
                    self.emit_event(AiEvent::CompactionFailed {
                        tokens_before,
                        messages_before,
                        error: error.clone(),
                        summarizer_input: None,
                    });
                    error
                })?;

        // Save summarizer input for debugging
        let _ =
            crate::transcript::save_summarizer_input(&artifacts_dir, session_id, &summarizer_input);

        // Generate summary
        let client = self.client.read().await;
        let summary_result = crate::summarizer::generate_summary(&client, &summarizer_input).await;
        drop(client);

        let summary = match summary_result {
            Ok(response) => response.summary,
            Err(e) => {
                let error = format!("Summarizer failed: {}", e);
                self.emit_event(AiEvent::CompactionFailed {
                    tokens_before,
                    messages_before,
                    error: error.clone(),
                    summarizer_input: Some(summarizer_input),
                });
                return Err(error);
            }
        };

        // Save summary for debugging
        let _ = crate::transcript::save_summary(&summaries_dir, session_id, &summary);

        // Apply compaction to chat history
        let mut chat_history = self.conversation_history.write().await;
        let _messages_removed = apply_compaction(&mut chat_history, &summary);
        let messages_after = chat_history.len();
        drop(chat_history);

        // Update context manager
        let history = self.conversation_history.read().await;
        self.context_manager.update_from_messages(&history).await;
        drop(history);

        // Reset compaction state
        {
            let mut compaction_state = self.compaction_state.write().await;
            compaction_state.increment_count();
        }

        // Emit success event
        let summary_length = summary.len();
        self.emit_event(AiEvent::CompactionCompleted {
            tokens_before,
            messages_before,
            messages_after,
            summary_length,
            summary: Some(summary),
            summarizer_input: Some(summarizer_input),
        });

        Ok(())
    }
}
