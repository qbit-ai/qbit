//! Isolated conversation summarizer agent.
//!
//! This module provides Tauri command wrappers for the conversation summarizer
//! in qbit-ai. The actual summarization logic is implemented in qbit-ai::summarizer.

use tauri::State;

use super::ai_session_not_initialized_error;
use crate::state::AppState;

// Re-export types from qbit-ai for Tauri command compatibility
pub use qbit_ai::SummaryResponse;

/// Tauri command to generate a conversation summary.
///
/// This command is primarily for testing the summarizer from the frontend.
/// In production, the summarizer is called internally by the context compaction system.
///
/// # Arguments
/// * `state` - The application state
/// * `session_id` - The session ID to use for the LLM client
/// * `conversation` - The conversation transcript to summarize
///
/// # Returns
/// A SummaryResponse containing the structured summary
#[tauri::command]
pub async fn generate_conversation_summary(
    state: State<'_, AppState>,
    session_id: String,
    conversation: String,
) -> Result<SummaryResponse, String> {
    // Get Arc clone and release map lock immediately
    let bridge = state
        .ai_state
        .get_session_bridge(&session_id)
        .await
        .ok_or_else(|| ai_session_not_initialized_error(&session_id))?;

    // Access the LLM client
    let client = bridge.client().clone();
    let client_guard = client.read().await;

    // Generate the summary using qbit-ai's summarizer
    qbit_ai::generate_summary_with_config(&client_guard, &conversation)
        .await
        .map_err(|e| format!("Failed to generate summary: {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;
    use qbit_ai::SUMMARIZER_SYSTEM_PROMPT;

    #[test]
    fn test_summarizer_system_prompt_not_empty() {
        assert!(!SUMMARIZER_SYSTEM_PROMPT.is_empty());
        assert!(SUMMARIZER_SYSTEM_PROMPT.contains("## Original Request"));
        assert!(SUMMARIZER_SYSTEM_PROMPT.contains("## Current State"));
        assert!(SUMMARIZER_SYSTEM_PROMPT.contains("## Key Decisions"));
        assert!(SUMMARIZER_SYSTEM_PROMPT.contains("## Pending Work"));
        assert!(SUMMARIZER_SYSTEM_PROMPT.contains("## Important Context"));
    }
}
