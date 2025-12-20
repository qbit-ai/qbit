//! Local session archive - drop-in replacement for `vtcode_core::utils::session_archive`.
//!
//! This module provides session persistence functionality for AI conversations.
//! It is designed as a drop-in replacement for vtcode-core's session_archive module,
//! maintaining exact interface compatibility for seamless migration.
//!
//! ## Interface Contract
//!
//! The following interfaces MUST be preserved for compatibility with `session.rs`:
//!
//! ```rust,ignore
//! // 1. Create archive (session.rs:218-220)
//! let archive = SessionArchive::new(metadata).await?;
//!
//! // 2. Create metadata (session.rs:209-216)
//! let metadata = SessionArchiveMetadata::new(
//!     &workspace_label,      // &str
//!     workspace_path,        // String
//!     &model,               // &str
//!     &provider,            // &str
//!     "default",            // theme: &str
//!     "standard",           // reasoning_effort: &str
//! );
//!
//! // 3. Finalize session (session.rs:305-312)
//! let path = archive.finalize(
//!     transcript,        // Vec<String>
//!     message_count,     // usize
//!     distinct_tools,    // Vec<String>
//!     messages,          // Vec<SessionMessage>
//! )?;
//!
//! // 4. Find session (session.rs:447, 470)
//! let listing = find_session_by_identifier(identifier).await?;
//! // Returns: Option<SessionListing>
//!
//! // 5. List sessions (session.rs:418)
//! let listings = list_recent_sessions(limit).await?;
//! // Returns: Vec<SessionListing>
//!
//! // 6. Message creation (session.rs:299, 344)
//! SessionMessage::with_tool_call_id(role, content, tool_call_id)
//!
//! // 7. Content access (session.rs:488)
//! message.content.as_text()
//! // Returns: String
//! ```
//!
//! ## Storage Format
//!
//! Sessions are stored as JSON files in `~/.qbit/sessions/` (or `$VT_SESSION_DIR`).
//! The format is backwards compatible with existing session files created by vtcode-core.
//!
//! ## Example
//!
//! ```rust,ignore
//! use crate::session::{
//!     SessionArchive, SessionArchiveMetadata, SessionMessage, MessageRole,
//!     find_session_by_identifier, list_recent_sessions,
//! };
//!
//! // Create a new session
//! let metadata = SessionArchiveMetadata::new(
//!     "my-project",
//!     "/path/to/my-project".to_string(),
//!     "claude-3-opus",
//!     "anthropic",
//!     "default",
//!     "standard",
//! );
//!
//! let archive = SessionArchive::new(metadata).await?;
//!
//! // Add messages
//! let messages = vec![
//!     SessionMessage::with_tool_call_id(MessageRole::User, "Hello!", None),
//!     SessionMessage::with_tool_call_id(MessageRole::Assistant, "Hi there!", None),
//! ];
//!
//! // Finalize and save
//! let path = archive.finalize(
//!     vec!["User: Hello!".into(), "Assistant: Hi there!".into()],
//!     2,
//!     vec![],
//!     messages,
//! )?;
//!
//! // Later, find the session
//! let listing = find_session_by_identifier(&session_id).await?;
//! ```

mod archive;
mod listing;
mod message;
mod storage;

// Re-export public types for drop-in compatibility
pub use archive::{SessionArchive, SessionArchiveMetadata};
pub use listing::{
    find_session_by_identifier, list_recent_sessions, SessionListing, SessionSnapshot,
};
pub use message::{MessageContent, MessageRole, SessionMessage};

// Also export storage utilities for testing/internal use
pub use storage::get_sessions_dir;

#[cfg(test)]
mod tests {
    //! Integration tests for the session module.
    //!
    //! These tests verify that the session module works correctly as a complete system.

    use super::*;
    use serial_test::serial;
    use tempfile::TempDir;

    /// Test the complete workflow: create -> add messages -> finalize -> find
    #[tokio::test]
    #[serial]
    async fn test_complete_workflow() {
        let temp = TempDir::new().unwrap();
        std::env::set_var("VT_SESSION_DIR", temp.path());

        // 1. Create metadata (matching session.rs:209-216)
        let metadata = SessionArchiveMetadata::new(
            "workflow-test",
            "/path/to/workspace".to_string(),
            "claude-3-opus",
            "anthropic",
            "default",
            "standard",
        );
        let session_id = metadata.session_id.clone();

        // 2. Create archive (matching session.rs:218-220)
        let archive = SessionArchive::new(metadata).await.unwrap();

        // 3. Create messages (matching session.rs:287-301)
        let messages = vec![
            SessionMessage::with_tool_call_id(MessageRole::User, "What is 2+2?", None),
            SessionMessage::with_tool_call_id(MessageRole::Assistant, "2+2 equals 4.", None),
            SessionMessage::with_tool_call_id(
                MessageRole::Tool,
                "Calculator result: 4",
                Some("tool_call_123".to_string()),
            ),
        ];

        let transcript = vec![
            "User: What is 2+2?".to_string(),
            "Assistant: 2+2 equals 4.".to_string(),
            "Tool: Calculator result: 4".to_string(),
        ];

        let distinct_tools = vec!["calculator".to_string()];

        // 4. Finalize (matching session.rs:305-312)
        let path = archive
            .finalize(transcript.clone(), 3, distinct_tools.clone(), messages)
            .unwrap();

        assert!(path.exists());

        // 5. Find the session (matching session.rs:447)
        let listing = find_session_by_identifier(&session_id).await.unwrap();
        assert!(listing.is_some());

        let listing = listing.unwrap();
        assert_eq!(listing.identifier(), session_id);
        assert_eq!(listing.snapshot.metadata.workspace_label, "workflow-test");
        assert_eq!(listing.snapshot.total_messages, 3);
        assert_eq!(listing.snapshot.distinct_tools, distinct_tools);
        assert_eq!(listing.snapshot.transcript, transcript);

        // 6. Verify message content access (matching session.rs:488)
        let first_msg = &listing.snapshot.messages[0];
        assert_eq!(first_msg.role, MessageRole::User);
        assert_eq!(first_msg.content.as_text(), "What is 2+2?");

        let tool_msg = &listing.snapshot.messages[2];
        assert_eq!(tool_msg.role, MessageRole::Tool);
        assert_eq!(tool_msg.content.as_text(), "Calculator result: 4");
        assert_eq!(tool_msg.tool_call_id, Some("tool_call_123".to_string()));

        std::env::remove_var("VT_SESSION_DIR");
    }

    /// Test listing multiple sessions
    #[tokio::test]
    #[serial]
    async fn test_list_multiple_sessions() {
        let temp = TempDir::new().unwrap();
        std::env::set_var("VT_SESSION_DIR", temp.path());

        // Create multiple sessions
        for i in 0..5 {
            let metadata = SessionArchiveMetadata::new(
                &format!("project-{}", i),
                format!("/path/to/project-{}", i),
                "model",
                "provider",
                "default",
                "standard",
            );

            let archive = SessionArchive::new(metadata).await.unwrap();

            let messages = vec![SessionMessage::user(&format!("Message {}", i))];

            archive
                .finalize(vec![format!("User: Message {}", i)], 1, vec![], messages)
                .unwrap();

            // Small delay to ensure different timestamps
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        }

        // List all sessions
        let all_sessions = list_recent_sessions(0).await.unwrap();
        assert_eq!(all_sessions.len(), 5);

        // List with limit
        let limited = list_recent_sessions(2).await.unwrap();
        assert_eq!(limited.len(), 2);

        // Verify order (most recent first)
        for i in 0..all_sessions.len() - 1 {
            assert!(all_sessions[i].started_at >= all_sessions[i + 1].started_at);
        }

        std::env::remove_var("VT_SESSION_DIR");
    }

    /// Test preview functionality
    #[tokio::test]
    #[serial]
    async fn test_session_previews() {
        let temp = TempDir::new().unwrap();
        std::env::set_var("VT_SESSION_DIR", temp.path());

        let metadata = SessionArchiveMetadata::new(
            "preview-test",
            "/path".to_string(),
            "model",
            "provider",
            "default",
            "standard",
        );
        let session_id = metadata.session_id.clone();

        let archive = SessionArchive::new(metadata).await.unwrap();

        let messages = vec![
            SessionMessage::user("What is the meaning of life?"),
            SessionMessage::assistant("The meaning of life is 42."),
        ];

        archive.finalize(vec![], 2, vec![], messages).unwrap();

        let listing = find_session_by_identifier(&session_id)
            .await
            .unwrap()
            .unwrap();

        let prompt_preview = listing.first_prompt_preview();
        assert!(prompt_preview.is_some());
        assert!(prompt_preview.unwrap().contains("meaning of life"));

        let reply_preview = listing.first_reply_preview();
        assert!(reply_preview.is_some());
        assert!(reply_preview.unwrap().contains("42"));

        std::env::remove_var("VT_SESSION_DIR");
    }

    /// Test backwards compatibility - can read existing session files
    #[tokio::test]
    #[serial]
    async fn test_backwards_compatibility_read() {
        let temp = TempDir::new().unwrap();
        std::env::set_var("VT_SESSION_DIR", temp.path());

        // Write a session file in the existing format
        let existing_json = r#"{
            "metadata": {
                "workspace_label": "legacy-project",
                "workspace_path": "/old/path",
                "model": "claude-2",
                "provider": "anthropic",
                "theme": "default",
                "reasoning_effort": "standard"
            },
            "started_at": "2025-01-01T00:00:00.000000Z",
            "ended_at": "2025-01-01T00:01:00.000000Z",
            "total_messages": 2,
            "distinct_tools": ["read_file"],
            "transcript": ["User: Hi", "Assistant: Hello"],
            "messages": [
                {"role": "User", "content": "Hi", "tool_call_id": null},
                {"role": "Assistant", "content": "Hello", "tool_call_id": null}
            ]
        }"#;

        let file_path = temp.path().join("session-legacy-test.json");
        std::fs::write(&file_path, existing_json).unwrap();

        // List should find it
        let sessions = list_recent_sessions(0).await.unwrap();
        assert_eq!(sessions.len(), 1);

        let session = &sessions[0];
        assert_eq!(session.snapshot.metadata.workspace_label, "legacy-project");
        assert_eq!(session.snapshot.total_messages, 2);
        assert_eq!(session.snapshot.distinct_tools, vec!["read_file"]);

        std::env::remove_var("VT_SESSION_DIR");
    }
}
