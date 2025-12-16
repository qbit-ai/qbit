//! Session listing and lookup functionality.
//!
//! This module provides types and functions for listing and finding sessions.

use std::path::PathBuf;

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::archive::SessionArchiveMetadata;
use super::message::{MessageRole, SessionMessage};
use super::storage;

/// Full session snapshot that is serialized to disk.
///
/// This structure matches the JSON format of existing session files
/// for backwards compatibility.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSnapshot {
    /// Session metadata
    pub metadata: SessionArchiveMetadata,
    /// When the session started
    pub started_at: DateTime<Utc>,
    /// When the session ended
    pub ended_at: DateTime<Utc>,
    /// Total number of messages in the session
    pub total_messages: usize,
    /// List of unique tool names used
    pub distinct_tools: Vec<String>,
    /// Human-readable transcript lines
    pub transcript: Vec<String>,
    /// Full message history
    pub messages: Vec<SessionMessage>,
}

/// Session listing entry for display and lookup.
///
/// This provides metadata about a session without necessarily
/// loading the full message history.
#[derive(Debug, Clone)]
pub struct SessionListing {
    /// Path to the session file
    pub path: PathBuf,
    /// When the session started
    pub started_at: DateTime<Utc>,
    /// When the session ended
    pub ended_at: DateTime<Utc>,
    /// Full session snapshot (for accessing messages and metadata)
    pub snapshot: SessionSnapshot,
}

impl SessionListing {
    /// Create a listing from a snapshot and path.
    pub fn from_snapshot(snapshot: SessionSnapshot, path: PathBuf) -> Self {
        Self {
            started_at: snapshot.started_at,
            ended_at: snapshot.ended_at,
            path,
            snapshot,
        }
    }

    /// Get the session identifier (session_id from metadata).
    ///
    /// This matches the vtcode-core interface: `listing.identifier()`
    pub fn identifier(&self) -> String {
        self.snapshot.metadata.session_id.clone()
    }

    /// Get a preview of the first user prompt.
    ///
    /// Returns the content of the first User message, truncated if necessary.
    pub fn first_prompt_preview(&self) -> Option<String> {
        self.snapshot
            .messages
            .iter()
            .find(|m| m.role == MessageRole::User)
            .map(|m| {
                let text = m.content.as_text();
                if text.len() > 200 {
                    format!("{}...", &text[..200])
                } else {
                    text
                }
            })
    }

    /// Get a preview of the first assistant reply.
    ///
    /// Returns the content of the first Assistant message, truncated if necessary.
    pub fn first_reply_preview(&self) -> Option<String> {
        self.snapshot
            .messages
            .iter()
            .find(|m| m.role == MessageRole::Assistant)
            .map(|m| {
                let text = m.content.as_text();
                if text.len() > 200 {
                    format!("{}...", &text[..200])
                } else {
                    text
                }
            })
    }
}

/// Find a session by its identifier.
///
/// This is a drop-in replacement for `vtcode_core::utils::session_archive::find_session_by_identifier()`.
///
/// ## Arguments
/// * `identifier` - Session ID or prefix to search for
///
/// ## Returns
/// * `Ok(Some(listing))` - If a matching session is found
/// * `Ok(None)` - If no matching session exists
/// * `Err(_)` - If there was an error reading the sessions directory
///
/// ## Note
/// The vtcode-core version is async, but our implementation is synchronous.
/// We provide an async wrapper for interface compatibility.
pub async fn find_session_by_identifier(identifier: &str) -> Result<Option<SessionListing>> {
    storage::find_session(identifier)
}

/// List recent sessions.
///
/// This is a drop-in replacement for `vtcode_core::utils::session_archive::list_recent_sessions()`.
///
/// ## Arguments
/// * `limit` - Maximum number of sessions to return (0 for unlimited)
///
/// ## Returns
/// Sessions sorted by start time, most recent first.
///
/// ## Note
/// The vtcode-core version is async, but our implementation is synchronous.
/// We provide an async wrapper for interface compatibility.
pub async fn list_recent_sessions(limit: usize) -> Result<Vec<SessionListing>> {
    storage::list_sessions(limit)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use tempfile::TempDir;

    fn create_test_snapshot(workspace: &str, session_id: &str) -> SessionSnapshot {
        SessionSnapshot {
            metadata: SessionArchiveMetadata {
                session_id: session_id.to_string(),
                workspace_label: workspace.to_string(),
                workspace_path: format!("/tmp/{}", workspace),
                model: "test-model".to_string(),
                provider: "test-provider".to_string(),
                theme: "default".to_string(),
                reasoning_effort: "standard".to_string(),
            },
            started_at: Utc::now(),
            ended_at: Utc::now(),
            total_messages: 2,
            distinct_tools: vec![],
            transcript: vec!["User: Hello".to_string(), "Assistant: Hi".to_string()],
            messages: vec![
                SessionMessage::user("Hello, how are you?"),
                SessionMessage::assistant("I'm doing well, thank you for asking!"),
            ],
        }
    }

    // ==========================================================================
    // SessionSnapshot Tests
    // ==========================================================================

    mod snapshot {
        use super::*;

        #[test]
        fn serialization_roundtrip() {
            let snapshot = create_test_snapshot("roundtrip-test", "snap123");

            let json = serde_json::to_string(&snapshot).unwrap();
            let restored: SessionSnapshot = serde_json::from_str(&json).unwrap();

            assert_eq!(restored.metadata.session_id, "snap123");
            assert_eq!(restored.metadata.workspace_label, "roundtrip-test");
            assert_eq!(restored.messages.len(), 2);
        }

        #[test]
        fn deserializes_existing_format() {
            // This is an actual format from existing session files
            let json = r#"{
                "metadata": {
                    "workspace_label": "qbit",
                    "workspace_path": "/Users/xlyk/Code/qbit",
                    "model": "claude-opus-4-5@20251101",
                    "provider": "anthropic_vertex",
                    "theme": "default",
                    "reasoning_effort": "standard"
                },
                "started_at": "2025-12-14T08:43:35.012542Z",
                "ended_at": "2025-12-14T08:43:38.958506Z",
                "total_messages": 2,
                "distinct_tools": [],
                "transcript": [
                    "User: Hello",
                    "Assistant: Hi there"
                ],
                "messages": [
                    {
                        "role": "User",
                        "content": "Hello",
                        "tool_call_id": null
                    },
                    {
                        "role": "Assistant",
                        "content": "Hi there",
                        "tool_call_id": null
                    }
                ]
            }"#;

            let snapshot: SessionSnapshot = serde_json::from_str(json).unwrap();

            assert_eq!(snapshot.metadata.workspace_label, "qbit");
            assert_eq!(snapshot.metadata.model, "claude-opus-4-5@20251101");
            assert_eq!(snapshot.total_messages, 2);
            assert_eq!(snapshot.messages.len(), 2);
            assert_eq!(snapshot.messages[0].role, MessageRole::User);
            assert_eq!(snapshot.messages[0].content.as_text(), "Hello");
        }
    }

    // ==========================================================================
    // SessionListing Tests
    // ==========================================================================

    mod listing {
        use super::*;

        #[test]
        fn from_snapshot_copies_timestamps() {
            let snapshot = create_test_snapshot("listing-test", "list123");
            let path = PathBuf::from("/tmp/test.json");

            let listing = SessionListing::from_snapshot(snapshot.clone(), path);

            assert_eq!(listing.started_at, snapshot.started_at);
            assert_eq!(listing.ended_at, snapshot.ended_at);
        }

        #[test]
        fn identifier_returns_session_id() {
            let snapshot = create_test_snapshot("id-test", "myuniqueid123");
            let path = PathBuf::from("/tmp/test.json");

            let listing = SessionListing::from_snapshot(snapshot, path);

            assert_eq!(listing.identifier(), "myuniqueid123");
        }

        #[test]
        fn first_prompt_preview_finds_user_message() {
            let snapshot = create_test_snapshot("prompt-test", "prompt123");
            let path = PathBuf::from("/tmp/test.json");

            let listing = SessionListing::from_snapshot(snapshot, path);

            let preview = listing.first_prompt_preview();
            assert!(preview.is_some());
            assert!(preview.unwrap().contains("Hello"));
        }

        #[test]
        fn first_reply_preview_finds_assistant_message() {
            let snapshot = create_test_snapshot("reply-test", "reply123");
            let path = PathBuf::from("/tmp/test.json");

            let listing = SessionListing::from_snapshot(snapshot, path);

            let preview = listing.first_reply_preview();
            assert!(preview.is_some());
            assert!(preview.unwrap().contains("doing well"));
        }

        #[test]
        fn first_prompt_preview_truncates_long_messages() {
            let mut snapshot = create_test_snapshot("long-test", "long123");
            let long_content = "x".repeat(500);
            snapshot.messages = vec![SessionMessage::user(&long_content)];

            let listing = SessionListing::from_snapshot(snapshot, PathBuf::from("/tmp/test.json"));

            let preview = listing.first_prompt_preview().unwrap();
            assert!(preview.len() <= 203); // 200 + "..."
            assert!(preview.ends_with("..."));
        }

        #[test]
        fn first_prompt_preview_returns_none_when_no_user_message() {
            let mut snapshot = create_test_snapshot("no-user", "nouser123");
            snapshot.messages = vec![SessionMessage::assistant("Only assistant message")];

            let listing = SessionListing::from_snapshot(snapshot, PathBuf::from("/tmp/test.json"));

            assert!(listing.first_prompt_preview().is_none());
        }

        #[test]
        fn first_reply_preview_returns_none_when_no_assistant_message() {
            let mut snapshot = create_test_snapshot("no-assist", "noassist123");
            snapshot.messages = vec![SessionMessage::user("Only user message")];

            let listing = SessionListing::from_snapshot(snapshot, PathBuf::from("/tmp/test.json"));

            assert!(listing.first_reply_preview().is_none());
        }
    }

    // ==========================================================================
    // find_session_by_identifier Tests
    // ==========================================================================

    mod find_by_identifier {
        use super::*;

        #[tokio::test]
        #[serial]
        async fn finds_existing_session() {
            let temp = TempDir::new().unwrap();
            std::env::set_var("VT_SESSION_DIR", temp.path());

            // Create a session
            let snapshot = create_test_snapshot("find-workspace", "findme12345");
            storage::save_session(&temp.path().to_path_buf(), &snapshot).unwrap();

            // Find it
            let found = find_session_by_identifier("findme").await.unwrap();
            assert!(found.is_some());
            assert_eq!(found.unwrap().identifier(), "findme12345");

            std::env::remove_var("VT_SESSION_DIR");
        }

        #[tokio::test]
        #[serial]
        async fn returns_none_for_nonexistent() {
            let temp = TempDir::new().unwrap();
            std::env::set_var("VT_SESSION_DIR", temp.path());

            let found = find_session_by_identifier("doesnotexist").await.unwrap();
            assert!(found.is_none());

            std::env::remove_var("VT_SESSION_DIR");
        }
    }

    // ==========================================================================
    // list_recent_sessions Tests
    // ==========================================================================

    mod list_recent {
        use super::*;
        use std::thread;
        use std::time::Duration;

        #[tokio::test]
        #[serial]
        async fn returns_sessions_in_order() {
            let temp = TempDir::new().unwrap();
            std::env::set_var("VT_SESSION_DIR", temp.path());

            // Create sessions with delays
            for i in 0..3 {
                let mut snapshot =
                    create_test_snapshot(&format!("order-{}", i), &format!("ord{}", i));
                thread::sleep(Duration::from_millis(50));
                snapshot.started_at = Utc::now();
                storage::save_session(&temp.path().to_path_buf(), &snapshot).unwrap();
            }

            let sessions = list_recent_sessions(0).await.unwrap();

            // Verify descending order
            for i in 0..sessions.len() - 1 {
                assert!(
                    sessions[i].started_at >= sessions[i + 1].started_at,
                    "Sessions should be in descending order by start time"
                );
            }

            std::env::remove_var("VT_SESSION_DIR");
        }

        #[tokio::test]
        #[serial]
        async fn respects_limit() {
            let temp = TempDir::new().unwrap();
            std::env::set_var("VT_SESSION_DIR", temp.path());

            // Create 5 sessions
            for i in 0..5 {
                let snapshot = create_test_snapshot(&format!("limit-{}", i), &format!("lim{}", i));
                storage::save_session(&temp.path().to_path_buf(), &snapshot).unwrap();
            }

            let sessions = list_recent_sessions(3).await.unwrap();
            assert_eq!(sessions.len(), 3);

            std::env::remove_var("VT_SESSION_DIR");
        }

        #[tokio::test]
        #[serial]
        async fn unlimited_when_limit_is_zero() {
            let temp = TempDir::new().unwrap();
            std::env::set_var("VT_SESSION_DIR", temp.path());

            // Create 5 sessions
            for i in 0..5 {
                let snapshot = create_test_snapshot(&format!("zero-{}", i), &format!("zro{}", i));
                storage::save_session(&temp.path().to_path_buf(), &snapshot).unwrap();
            }

            let sessions = list_recent_sessions(0).await.unwrap();
            assert_eq!(sessions.len(), 5);

            std::env::remove_var("VT_SESSION_DIR");
        }
    }

    // ==========================================================================
    // Backwards Compatibility Tests
    // ==========================================================================

    mod backwards_compat {
        use super::*;

        #[test]
        fn loads_existing_session_format() {
            // This JSON matches exactly the format of existing session files
            let json = r#"{
                "metadata": {
                    "workspace_label": "qbit-go-testbed",
                    "workspace_path": "/Users/xlyk/Code/qbit-go-testbed",
                    "model": "claude-opus-4-5@20251101",
                    "provider": "anthropic_vertex",
                    "theme": "default",
                    "reasoning_effort": "standard"
                },
                "started_at": "2025-12-12T22:04:13.717221Z",
                "ended_at": "2025-12-12T22:04:24.832634Z",
                "total_messages": 2,
                "distinct_tools": [],
                "transcript": [
                    "User: Hello world",
                    "Assistant: Hi there!"
                ],
                "messages": [
                    {
                        "role": "User",
                        "content": "Hello world",
                        "tool_call_id": null
                    },
                    {
                        "role": "Assistant",
                        "content": "Hi there!",
                        "tool_call_id": null
                    }
                ]
            }"#;

            let snapshot: SessionSnapshot = serde_json::from_str(json).unwrap();

            assert_eq!(snapshot.metadata.workspace_label, "qbit-go-testbed");
            assert_eq!(
                snapshot.metadata.workspace_path,
                "/Users/xlyk/Code/qbit-go-testbed"
            );
            assert_eq!(snapshot.metadata.model, "claude-opus-4-5@20251101");
            assert_eq!(snapshot.metadata.provider, "anthropic_vertex");
            assert_eq!(snapshot.total_messages, 2);
            assert_eq!(snapshot.distinct_tools.len(), 0);
            assert_eq!(snapshot.transcript.len(), 2);
            assert_eq!(snapshot.messages.len(), 2);

            // Verify message parsing
            assert_eq!(snapshot.messages[0].role, MessageRole::User);
            assert_eq!(snapshot.messages[0].content.as_text(), "Hello world");
            assert!(snapshot.messages[0].tool_call_id.is_none());

            assert_eq!(snapshot.messages[1].role, MessageRole::Assistant);
            assert_eq!(snapshot.messages[1].content.as_text(), "Hi there!");
        }

        #[test]
        fn handles_message_with_tool_call_id() {
            let json = r#"{
                "role": "Tool",
                "content": "File read successfully: test content",
                "tool_call_id": "toolu_01ABC123"
            }"#;

            let msg: SessionMessage = serde_json::from_str(json).unwrap();

            assert_eq!(msg.role, MessageRole::Tool);
            assert_eq!(
                msg.content.as_text(),
                "File read successfully: test content"
            );
            assert_eq!(msg.tool_call_id, Some("toolu_01ABC123".to_string()));
        }
    }
}
