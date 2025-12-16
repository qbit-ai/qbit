//! Session persistence module for Qbit AI conversations.
//!
//! This module provides session archiving, conversation logs, and transcript export
//! capabilities by integrating with vtcode-core's session_archive system.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use rig::completion::{AssistantContent, Message};
use rig::message::UserContent;
use rig::one_or_many::OneOrMany;
use serde::{Deserialize, Serialize};

use crate::compat::session::{
    find_session_by_identifier, list_recent_sessions as list_sessions_internal, MessageRole,
    SessionArchive, SessionArchiveMetadata, SessionMessage,
};

/// Role of a message in the conversation (simplified for Qbit).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum QbitMessageRole {
    User,
    Assistant,
    System,
    Tool,
}

/// A simplified message format for Qbit sessions.
/// This provides a bridge between rig's Message type and vtcode's SessionMessage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QbitSessionMessage {
    pub role: QbitMessageRole,
    pub content: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_name: Option<String>,
}

impl QbitSessionMessage {
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: QbitMessageRole::User,
            content: content.into(),
            tool_call_id: None,
            tool_name: None,
        }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: QbitMessageRole::Assistant,
            content: content.into(),
            tool_call_id: None,
            tool_name: None,
        }
    }

    #[allow(dead_code)]
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: QbitMessageRole::System,
            content: content.into(),
            tool_call_id: None,
            tool_name: None,
        }
    }

    #[allow(dead_code)]
    pub fn tool_use(tool_name: impl Into<String>, result: impl Into<String>) -> Self {
        let tool_name = tool_name.into();
        Self {
            role: QbitMessageRole::Tool,
            content: result.into(),
            tool_call_id: None,
            tool_name: Some(tool_name),
        }
    }

    #[allow(dead_code)]
    pub fn tool_result(content: impl Into<String>, tool_call_id: impl Into<String>) -> Self {
        Self {
            role: QbitMessageRole::Tool,
            content: content.into(),
            tool_call_id: Some(tool_call_id.into()),
            tool_name: None,
        }
    }
}

/// Convert rig Message to QbitSessionMessage for persistence.
impl From<&Message> for QbitSessionMessage {
    fn from(message: &Message) -> Self {
        match message {
            Message::User { content } => {
                let text = content
                    .iter()
                    .filter_map(|c| match c {
                        rig::message::UserContent::Text(t) => Some(t.text.clone()),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
                Self::user(text)
            }
            Message::Assistant { content, .. } => {
                let text = content
                    .iter()
                    .filter_map(|c| match c {
                        rig::completion::AssistantContent::Text(t) => Some(t.text.clone()),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
                Self::assistant(text)
            }
        }
    }
}

impl QbitSessionMessage {
    /// Convert QbitSessionMessage back to rig Message for restoring sessions.
    /// Note: Tool messages are converted to assistant messages since rig's Message
    /// enum only supports User and Assistant variants for chat history.
    pub fn to_rig_message(&self) -> Option<Message> {
        match self.role {
            QbitMessageRole::User => Some(Message::User {
                content: OneOrMany::one(UserContent::Text(rig::message::Text {
                    text: self.content.clone(),
                })),
            }),
            QbitMessageRole::Assistant => Some(Message::Assistant {
                id: None,
                content: OneOrMany::one(AssistantContent::Text(rig::message::Text {
                    text: self.content.clone(),
                })),
            }),
            // System and Tool messages cannot be directly represented in rig's Message enum
            // for chat history, so we skip them (they were already processed)
            QbitMessageRole::System | QbitMessageRole::Tool => None,
        }
    }
}

/// Qbit session snapshot containing conversation data.
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QbitSessionSnapshot {
    /// Session metadata
    pub workspace_label: String,
    pub workspace_path: String,
    pub model: String,
    pub provider: String,

    /// Timestamps
    pub started_at: DateTime<Utc>,
    pub ended_at: DateTime<Utc>,

    /// Session statistics
    pub total_messages: usize,
    pub distinct_tools: Vec<String>,

    /// Human-readable transcript lines
    pub transcript: Vec<String>,

    /// Full message history
    pub messages: Vec<QbitSessionMessage>,

    /// Associated sidecar session ID (for context restoration)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sidecar_session_id: Option<String>,
}

/// Active session manager for creating and finalizing session archives.
pub struct QbitSessionManager {
    archive: Option<SessionArchive>,
    #[allow(dead_code)]
    workspace_label: String,
    #[allow(dead_code)]
    workspace_path: PathBuf,
    #[allow(dead_code)]
    model: String,
    #[allow(dead_code)]
    provider: String,
    messages: Vec<QbitSessionMessage>,
    tools_used: std::collections::HashSet<String>,
    transcript: Vec<String>,
    /// Associated sidecar session ID (for context restoration)
    sidecar_session_id: Option<String>,
}

impl QbitSessionManager {
    /// Create a new session manager.
    pub async fn new(
        workspace_path: PathBuf,
        model: impl Into<String>,
        provider: impl Into<String>,
    ) -> Result<Self> {
        let workspace_label = workspace_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("workspace")
            .to_string();

        let model = model.into();
        let provider = provider.into();

        let metadata = SessionArchiveMetadata::new(
            &workspace_label,
            workspace_path.display().to_string(),
            &model,
            &provider,
            "default",  // theme
            "standard", // reasoning_effort
        );

        let archive = SessionArchive::new(metadata)
            .await
            .context("Failed to create session archive")?;

        Ok(Self {
            archive: Some(archive),
            workspace_label,
            workspace_path,
            model,
            provider,
            messages: Vec::new(),
            tools_used: std::collections::HashSet::new(),
            transcript: Vec::new(),
            sidecar_session_id: None,
        })
    }

    /// Record a user message.
    pub fn add_user_message(&mut self, content: &str) {
        self.messages.push(QbitSessionMessage::user(content));
        self.transcript
            .push(format!("User: {}", truncate(content, 200)));
    }

    /// Record an assistant message.
    pub fn add_assistant_message(&mut self, content: &str) {
        self.messages.push(QbitSessionMessage::assistant(content));
        self.transcript
            .push(format!("Assistant: {}", truncate(content, 200)));
    }

    /// Record a tool use.
    #[allow(dead_code)]
    pub fn add_tool_use(&mut self, tool_name: &str, result: &str) {
        self.tools_used.insert(tool_name.to_string());
        self.messages
            .push(QbitSessionMessage::tool_use(tool_name, result));
        self.transcript
            .push(format!("Tool[{}]: {}", tool_name, truncate(result, 100)));
    }

    /// Convert rig Messages to session messages.
    #[allow(dead_code)]
    pub fn add_rig_messages(&mut self, messages: &[Message]) {
        for msg in messages {
            let qbit_msg = QbitSessionMessage::from(msg);
            match &qbit_msg.role {
                QbitMessageRole::User => {
                    self.transcript
                        .push(format!("User: {}", truncate(&qbit_msg.content, 200)));
                }
                QbitMessageRole::Assistant => {
                    self.transcript
                        .push(format!("Assistant: {}", truncate(&qbit_msg.content, 200)));
                }
                _ => {}
            }
            self.messages.push(qbit_msg);
        }
    }

    /// Save the current session state to disk without finalizing.
    /// This allows incremental saves after each message.
    ///
    /// Returns the path to the saved session file.
    pub fn save(&self) -> Result<PathBuf> {
        let archive = self.archive.as_ref().context("Session already finalized")?;

        // Convert QbitSessionMessages to vtcode SessionMessages
        let vtcode_messages: Vec<SessionMessage> = self
            .messages
            .iter()
            .map(|m| {
                let role = match m.role {
                    QbitMessageRole::User => vtcode_core::llm::provider::MessageRole::User,
                    QbitMessageRole::Assistant => {
                        vtcode_core::llm::provider::MessageRole::Assistant
                    }
                    QbitMessageRole::System => vtcode_core::llm::provider::MessageRole::System,
                    QbitMessageRole::Tool => vtcode_core::llm::provider::MessageRole::Tool,
                };
                SessionMessage::with_tool_call_id(role, &m.content, m.tool_call_id.clone())
            })
            .collect();

        let distinct_tools: Vec<String> = self.tools_used.iter().cloned().collect();

        let path = archive
            .finalize(
                self.transcript.clone(),
                self.messages.len(),
                distinct_tools,
                vtcode_messages,
            )
            .context("Failed to save session archive")?;

        // Save sidecar session ID to companion file if available
        if let Some(ref sidecar_id) = self.sidecar_session_id {
            if let Err(e) = Self::write_sidecar_session_id(&path, sidecar_id) {
                tracing::warn!("Failed to save sidecar session ID: {}", e);
            }
        }

        Ok(path)
    }

    /// Finalize the session and save to disk.
    /// After this, the session cannot be updated further.
    ///
    /// Returns the path to the saved session file.
    pub fn finalize(&mut self) -> Result<PathBuf> {
        let archive = self.archive.take().context("Session already finalized")?;

        // Convert QbitSessionMessages to vtcode SessionMessages
        let vtcode_messages: Vec<SessionMessage> = self
            .messages
            .iter()
            .map(|m| {
                let role = match m.role {
                    QbitMessageRole::User => vtcode_core::llm::provider::MessageRole::User,
                    QbitMessageRole::Assistant => {
                        vtcode_core::llm::provider::MessageRole::Assistant
                    }
                    QbitMessageRole::System => vtcode_core::llm::provider::MessageRole::System,
                    QbitMessageRole::Tool => vtcode_core::llm::provider::MessageRole::Tool,
                };
                SessionMessage::with_tool_call_id(role, &m.content, m.tool_call_id.clone())
            })
            .collect();

        let distinct_tools: Vec<String> = self.tools_used.iter().cloned().collect();

        let path = archive
            .finalize(
                self.transcript.clone(),
                self.messages.len(),
                distinct_tools,
                vtcode_messages,
            )
            .context("Failed to finalize session archive")?;

        // Save sidecar session ID to companion file if available
        if let Some(ref sidecar_id) = self.sidecar_session_id {
            if let Err(e) = Self::write_sidecar_session_id(&path, sidecar_id) {
                tracing::warn!("Failed to save sidecar session ID: {}", e);
            }
        }

        Ok(path)
    }

    /// Get the current message count.
    #[allow(dead_code)]
    pub fn message_count(&self) -> usize {
        self.messages.len()
    }

    /// Get the tools used in this session.
    #[allow(dead_code)]
    pub fn tools_used(&self) -> Vec<String> {
        self.tools_used.iter().cloned().collect()
    }

    /// Get the workspace path.
    #[allow(dead_code)]
    pub fn workspace_path(&self) -> &PathBuf {
        &self.workspace_path
    }

    /// Set the sidecar session ID for this AI session
    pub fn set_sidecar_session_id(&mut self, sidecar_session_id: String) {
        self.sidecar_session_id = Some(sidecar_session_id);
    }

    /// Write sidecar session ID to a companion file
    fn write_sidecar_session_id(session_path: &Path, sidecar_session_id: &str) -> Result<()> {
        // Create companion file with .sidecar extension
        let sidecar_meta_path = session_path.with_extension("sidecar");
        std::fs::write(&sidecar_meta_path, sidecar_session_id)
            .context("Failed to write sidecar session ID")?;
        Ok(())
    }

    /// Read sidecar session ID from a companion file
    fn read_sidecar_session_id(session_path: &Path) -> Option<String> {
        let sidecar_meta_path = session_path.with_extension("sidecar");
        if sidecar_meta_path.exists() {
            std::fs::read_to_string(&sidecar_meta_path).ok()
        } else {
            None
        }
    }
}

/// List recent sessions.
///
/// # Arguments
/// * `limit` - Maximum number of sessions to return (0 for all)
#[allow(dead_code)]
pub async fn list_recent_sessions(limit: usize) -> Result<Vec<SessionListingInfo>> {
    let listings = list_sessions_internal(limit).await?;

    Ok(listings
        .into_iter()
        .map(|listing| {
            let sidecar_meta = get_sidecar_session_meta(&listing.path);
            SessionListingInfo {
                identifier: listing.identifier(),
                path: listing.path.clone(),
                workspace_label: listing.snapshot.metadata.workspace_label.clone(),
                workspace_path: listing.snapshot.metadata.workspace_path.clone(),
                model: listing.snapshot.metadata.model.clone(),
                provider: listing.snapshot.metadata.provider.clone(),
                started_at: listing.snapshot.started_at,
                ended_at: listing.snapshot.ended_at,
                total_messages: listing.snapshot.total_messages,
                distinct_tools: listing.snapshot.distinct_tools.clone(),
                first_prompt_preview: listing.first_prompt_preview().map(|s| strip_xml_tags(&s)),
                first_reply_preview: listing.first_reply_preview().map(|s| strip_xml_tags(&s)),
                status: sidecar_meta.status,
                title: sidecar_meta.title,
            }
        })
        .collect())
}

/// Find a session by its identifier.
#[allow(dead_code)]
pub async fn find_session(identifier: &str) -> Result<Option<SessionListingInfo>> {
    let listing = find_session_by_identifier(identifier).await?;

    Ok(listing.map(|l| SessionListingInfo {
        identifier: l.identifier(),
        path: l.path.clone(),
        workspace_label: l.snapshot.metadata.workspace_label.clone(),
        workspace_path: l.snapshot.metadata.workspace_path.clone(),
        model: l.snapshot.metadata.model.clone(),
        provider: l.snapshot.metadata.provider.clone(),
        started_at: l.snapshot.started_at,
        ended_at: l.snapshot.ended_at,
        total_messages: l.snapshot.total_messages,
        distinct_tools: l.snapshot.distinct_tools.clone(),
        first_prompt_preview: l.first_prompt_preview().map(|s| strip_xml_tags(&s)),
        first_reply_preview: l.first_reply_preview().map(|s| strip_xml_tags(&s)),
        status: get_sidecar_session_meta(&l.path).status,
        title: get_sidecar_session_meta(&l.path).title,
    }))
}

/// Load a full session by identifier.
#[allow(dead_code)]
pub async fn load_session(identifier: &str) -> Result<Option<QbitSessionSnapshot>> {
    let listing = find_session_by_identifier(identifier).await?;

    Ok(listing.map(|l| {
        let messages = l
            .snapshot
            .messages
            .iter()
            .map(|m| {
                let role = match m.role {
                    MessageRole::User => QbitMessageRole::User,
                    MessageRole::Assistant => QbitMessageRole::Assistant,
                    MessageRole::System => QbitMessageRole::System,
                    MessageRole::Tool => QbitMessageRole::Tool,
                };
                QbitSessionMessage {
                    role,
                    content: m.content.as_text().to_string(),
                    tool_call_id: m.tool_call_id.clone(),
                    tool_name: None,
                }
            })
            .collect();

        // Read sidecar session ID from companion file
        let sidecar_session_id = QbitSessionManager::read_sidecar_session_id(&l.path);

        QbitSessionSnapshot {
            workspace_label: l.snapshot.metadata.workspace_label,
            workspace_path: l.snapshot.metadata.workspace_path,
            model: l.snapshot.metadata.model,
            provider: l.snapshot.metadata.provider,
            started_at: l.snapshot.started_at,
            ended_at: l.snapshot.ended_at,
            total_messages: l.snapshot.total_messages,
            distinct_tools: l.snapshot.distinct_tools,
            transcript: l.snapshot.transcript,
            messages,
            sidecar_session_id,
        }
    }))
}

/// Session listing information for display.
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionListingInfo {
    pub identifier: String,
    pub path: PathBuf,
    pub workspace_label: String,
    pub workspace_path: String,
    pub model: String,
    pub provider: String,
    pub started_at: DateTime<Utc>,
    pub ended_at: DateTime<Utc>,
    pub total_messages: usize,
    pub distinct_tools: Vec<String>,
    pub first_prompt_preview: Option<String>,
    pub first_reply_preview: Option<String>,
    /// Session status: "active", "completed", or "abandoned"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    /// LLM-generated session title
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
}

/// Truncate a string to a maximum length.
fn truncate(s: &str, max_len: usize) -> String {
    if s.chars().count() <= max_len {
        s.to_string()
    } else {
        let mut result: String = s.chars().take(max_len.saturating_sub(1)).collect();
        result.push('…');
        result
    }
}

/// Strip XML context tags from text.
/// Removes <context>...</context>, <cwd>...</cwd>, <session_id>...</session_id> tags.
fn strip_xml_tags(text: &str) -> String {
    let mut result = text.to_string();

    // List of tags to strip (with their content)
    let tags = ["context", "cwd", "session_id"];

    for tag in tags {
        let open_tag = format!("<{}>", tag);
        let close_tag = format!("</{}>", tag);

        // Remove tag and its content
        while let Some(start) = result.find(&open_tag) {
            if let Some(end_offset) = result[start..].find(&close_tag) {
                let end = start + end_offset + close_tag.len();
                result = format!("{}{}", &result[..start], &result[end..]);
            } else {
                // No closing tag found, just remove opening tag
                result = result.replace(&open_tag, "");
                break;
            }
        }
    }

    result.trim().to_string()
}

/// Sidecar session metadata extracted for display
struct SidecarSessionMeta {
    status: Option<String>,
    title: Option<String>,
}

/// Get metadata from the linked sidecar session for an AI session.
/// Returns status and title extracted from the sidecar session's state.md.
fn get_sidecar_session_meta(session_path: &Path) -> SidecarSessionMeta {
    // Read the sidecar session ID from the companion file
    let sidecar_meta_path = session_path.with_extension("sidecar");
    if !sidecar_meta_path.exists() {
        return SidecarSessionMeta {
            status: None,
            title: None,
        };
    }

    let sidecar_session_id = match std::fs::read_to_string(&sidecar_meta_path) {
        Ok(id) => id.trim().to_string(),
        Err(_) => {
            return SidecarSessionMeta {
                status: None,
                title: None,
            }
        }
    };

    // Get the sidecar sessions directory
    let sessions_dir = std::env::var("VT_SESSION_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join(".qbit")
                .join("sessions")
        });

    // Read the state.md file from the sidecar session
    let state_path = sessions_dir.join(&sidecar_session_id).join("state.md");
    if !state_path.exists() {
        return SidecarSessionMeta {
            status: None,
            title: None,
        };
    }

    let content = match std::fs::read_to_string(&state_path) {
        Ok(c) => c,
        Err(_) => {
            return SidecarSessionMeta {
                status: None,
                title: None,
            }
        }
    };

    // Parse YAML frontmatter to extract status and title
    if !content.starts_with("---\n") {
        return SidecarSessionMeta {
            status: None,
            title: None,
        };
    }

    let rest = &content[4..]; // Skip opening "---\n"
    let end_idx = match rest.find("\n---") {
        Some(idx) => idx,
        None => {
            return SidecarSessionMeta {
                status: None,
                title: None,
            }
        }
    };
    let yaml_content = &rest[..end_idx];

    let mut status = None;
    let mut title = None;

    // Simple extraction of fields
    for line in yaml_content.lines() {
        if line.starts_with("status:") {
            status = Some(line.trim_start_matches("status:").trim().to_string());
        } else if line.starts_with("title:") {
            title = Some(line.trim_start_matches("title:").trim().to_string());
        }
    }

    SidecarSessionMeta { status, title }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rig::message::Text;
    use serial_test::serial;
    use tempfile::TempDir;

    #[test]
    fn test_qbit_session_message_creation() {
        let user_msg = QbitSessionMessage::user("Hello");
        assert_eq!(user_msg.role, QbitMessageRole::User);
        assert_eq!(user_msg.content, "Hello");

        let assistant_msg = QbitSessionMessage::assistant("Hi there");
        assert_eq!(assistant_msg.role, QbitMessageRole::Assistant);
        assert_eq!(assistant_msg.content, "Hi there");
    }

    #[test]
    fn test_qbit_session_message_system() {
        let system_msg = QbitSessionMessage::system("You are a helpful assistant");
        assert_eq!(system_msg.role, QbitMessageRole::System);
        assert_eq!(system_msg.content, "You are a helpful assistant");
        assert!(system_msg.tool_call_id.is_none());
        assert!(system_msg.tool_name.is_none());
    }

    #[test]
    fn test_qbit_session_message_tool_result() {
        let tool_msg = QbitSessionMessage::tool_result("File contents here", "call_123");
        assert_eq!(tool_msg.role, QbitMessageRole::Tool);
        assert_eq!(tool_msg.content, "File contents here");
        assert_eq!(tool_msg.tool_call_id, Some("call_123".to_string()));
        assert!(tool_msg.tool_name.is_none());
    }

    #[test]
    fn test_truncate() {
        assert_eq!(truncate("short", 10), "short");
        assert_eq!(truncate("a longer string", 5), "a lo…");
        assert_eq!(truncate("", 10), "");
    }

    #[test]
    fn test_truncate_exact_length() {
        assert_eq!(truncate("12345", 5), "12345");
        assert_eq!(truncate("123456", 5), "1234…");
    }

    #[test]
    fn test_truncate_unicode() {
        // Unicode characters should be counted as single chars
        assert_eq!(truncate("héllo", 5), "héllo");
        assert_eq!(truncate("héllo world", 5), "héll…");
    }

    #[test]
    fn test_rig_message_conversion_user() {
        let rig_msg = Message::User {
            content: OneOrMany::one(UserContent::Text(Text {
                text: "Hello from user".to_string(),
            })),
        };

        let qbit_msg = QbitSessionMessage::from(&rig_msg);
        assert_eq!(qbit_msg.role, QbitMessageRole::User);
        assert_eq!(qbit_msg.content, "Hello from user");
    }

    #[test]
    fn test_rig_message_conversion_assistant() {
        let rig_msg = Message::Assistant {
            id: None,
            content: OneOrMany::one(AssistantContent::Text(Text {
                text: "Hello from assistant".to_string(),
            })),
        };

        let qbit_msg = QbitSessionMessage::from(&rig_msg);
        assert_eq!(qbit_msg.role, QbitMessageRole::Assistant);
        assert_eq!(qbit_msg.content, "Hello from assistant");
    }

    #[test]
    fn test_qbit_message_to_rig_user() {
        let qbit_msg = QbitSessionMessage::user("Test user message");
        let rig_msg = qbit_msg.to_rig_message();

        assert!(rig_msg.is_some());
        let rig_msg = rig_msg.unwrap();
        match rig_msg {
            Message::User { content } => {
                let text = content
                    .iter()
                    .filter_map(|c| match c {
                        UserContent::Text(t) => Some(t.text.clone()),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
                    .join("");
                assert_eq!(text, "Test user message");
            }
            _ => panic!("Expected User message"),
        }
    }

    #[test]
    fn test_qbit_message_to_rig_assistant() {
        let qbit_msg = QbitSessionMessage::assistant("Test assistant message");
        let rig_msg = qbit_msg.to_rig_message();

        assert!(rig_msg.is_some());
        let rig_msg = rig_msg.unwrap();
        match rig_msg {
            Message::Assistant { content, .. } => {
                let text = content
                    .iter()
                    .filter_map(|c| match c {
                        AssistantContent::Text(t) => Some(t.text.clone()),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
                    .join("");
                assert_eq!(text, "Test assistant message");
            }
            _ => panic!("Expected Assistant message"),
        }
    }

    #[test]
    fn test_qbit_message_to_rig_system_returns_none() {
        let qbit_msg = QbitSessionMessage::system("System prompt");
        assert!(qbit_msg.to_rig_message().is_none());
    }

    #[test]
    fn test_qbit_message_to_rig_tool_returns_none() {
        let qbit_msg = QbitSessionMessage::tool_result("Result", "call_id");
        assert!(qbit_msg.to_rig_message().is_none());
    }

    #[test]
    fn test_qbit_session_snapshot_serialization() {
        let snapshot = QbitSessionSnapshot {
            workspace_label: "test-workspace".to_string(),
            workspace_path: "/tmp/test".to_string(),
            model: "claude-3".to_string(),
            provider: "anthropic".to_string(),
            started_at: Utc::now(),
            ended_at: Utc::now(),
            total_messages: 2,
            distinct_tools: vec!["read_file".to_string(), "write_file".to_string()],
            transcript: vec!["User: Hello".to_string(), "Assistant: Hi".to_string()],
            messages: vec![
                QbitSessionMessage::user("Hello"),
                QbitSessionMessage::assistant("Hi"),
            ],
            sidecar_session_id: None,
        };

        let json = serde_json::to_string(&snapshot).expect("Failed to serialize");
        let deserialized: QbitSessionSnapshot =
            serde_json::from_str(&json).expect("Failed to deserialize");

        assert_eq!(deserialized.workspace_label, "test-workspace");
        assert_eq!(deserialized.total_messages, 2);
        assert_eq!(deserialized.messages.len(), 2);
        assert_eq!(deserialized.distinct_tools.len(), 2);
    }

    #[test]
    fn test_session_listing_info_serialization() {
        let info = SessionListingInfo {
            identifier: "session-test-123".to_string(),
            path: PathBuf::from("/tmp/sessions/session-test-123.json"),
            workspace_label: "my-project".to_string(),
            workspace_path: "/home/user/my-project".to_string(),
            model: "claude-3-opus".to_string(),
            provider: "anthropic".to_string(),
            started_at: Utc::now(),
            ended_at: Utc::now(),
            total_messages: 10,
            distinct_tools: vec!["bash".to_string()],
            first_prompt_preview: Some("Help me debug...".to_string()),
            first_reply_preview: Some("I'd be happy to help...".to_string()),
            status: Some("completed".to_string()),
            title: Some("Debug Authentication Bug".to_string()),
        };

        let json = serde_json::to_string(&info).expect("Failed to serialize");
        let deserialized: SessionListingInfo =
            serde_json::from_str(&json).expect("Failed to deserialize");

        assert_eq!(deserialized.identifier, "session-test-123");
        assert_eq!(deserialized.workspace_label, "my-project");
        assert_eq!(
            deserialized.first_prompt_preview,
            Some("Help me debug...".to_string())
        );
    }

    #[test]
    fn test_qbit_message_role_serialization() {
        // Test that roles serialize to lowercase as expected
        let user_msg = QbitSessionMessage::user("test");
        let json = serde_json::to_string(&user_msg).unwrap();
        assert!(json.contains("\"role\":\"user\""));

        let assistant_msg = QbitSessionMessage::assistant("test");
        let json = serde_json::to_string(&assistant_msg).unwrap();
        assert!(json.contains("\"role\":\"assistant\""));

        let system_msg = QbitSessionMessage::system("test");
        let json = serde_json::to_string(&system_msg).unwrap();
        assert!(json.contains("\"role\":\"system\""));

        let tool_msg = QbitSessionMessage::tool_result("test", "id");
        let json = serde_json::to_string(&tool_msg).unwrap();
        assert!(json.contains("\"role\":\"tool\""));
    }

    #[test]
    fn test_qbit_message_optional_fields_skip_when_none() {
        let msg = QbitSessionMessage::user("Hello");
        let json = serde_json::to_string(&msg).unwrap();

        // tool_call_id and tool_name should not appear when None
        assert!(!json.contains("tool_call_id"));
        assert!(!json.contains("tool_name"));
    }

    #[test]
    fn test_qbit_message_includes_tool_call_id_when_present() {
        let msg = QbitSessionMessage::tool_result("result", "call_abc");
        let json = serde_json::to_string(&msg).unwrap();

        assert!(json.contains("\"tool_call_id\":\"call_abc\""));
    }

    #[test]
    fn test_strip_xml_tags() {
        // Test stripping context tags
        let input = "<context>\n<cwd>/Users/test/project</cwd>\n<session_id>abc123</session_id>\n</context>\nActual user prompt here";
        let result = strip_xml_tags(input);
        assert_eq!(result, "Actual user prompt here");

        // Test with no tags
        let input = "Just a normal string";
        let result = strip_xml_tags(input);
        assert_eq!(result, "Just a normal string");

        // Test with partial tags (should still work)
        let input = "<context>Some content</context> More text";
        let result = strip_xml_tags(input);
        assert_eq!(result, "More text");

        // Test with nested content preserved outside tags
        let input = "Before <cwd>/path</cwd> After";
        let result = strip_xml_tags(input);
        assert_eq!(result, "Before  After");
    }

    // Note: The async tests that interact with the filesystem via vtcode-core's
    // session_archive are integration tests that depend on the VT_SESSION_DIR
    // environment variable. These tests are difficult to run in parallel because
    // they share global state. For comprehensive session persistence testing,
    // see the integration tests or run these with --test-threads=1.
    //
    // The tests below focus on unit-level functionality that doesn't require
    // filesystem isolation.

    #[tokio::test]
    #[serial]
    async fn test_session_manager_creation() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");

        // Set VT_SESSION_DIR for this test
        std::env::set_var("VT_SESSION_DIR", temp_dir.path());

        let manager =
            QbitSessionManager::new(temp_dir.path().to_path_buf(), "test-model", "test-provider")
                .await;

        assert!(manager.is_ok());
        let manager = manager.unwrap();
        assert_eq!(manager.message_count(), 0);
        assert!(manager.tools_used().is_empty());

        // Clean up
        std::env::remove_var("VT_SESSION_DIR");
    }

    #[tokio::test]
    #[serial]
    async fn test_session_manager_add_messages() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        std::env::set_var("VT_SESSION_DIR", temp_dir.path());

        let mut manager =
            QbitSessionManager::new(temp_dir.path().to_path_buf(), "test-model", "test-provider")
                .await
                .expect("Failed to create manager");

        manager.add_user_message("Hello, how are you?");
        assert_eq!(manager.message_count(), 1);

        manager.add_assistant_message("I'm doing well, thank you!");
        assert_eq!(manager.message_count(), 2);

        manager.add_tool_use("read_file", "File contents: hello world");
        assert_eq!(manager.message_count(), 3);
        assert!(manager.tools_used().contains(&"read_file".to_string()));

        std::env::remove_var("VT_SESSION_DIR");
    }

    #[tokio::test]
    #[serial]
    async fn test_session_manager_tools_tracking() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        std::env::set_var("VT_SESSION_DIR", temp_dir.path());

        let mut manager =
            QbitSessionManager::new(temp_dir.path().to_path_buf(), "test-model", "test-provider")
                .await
                .expect("Failed to create manager");

        manager.add_tool_use("read_file", "contents");
        manager.add_tool_use("write_file", "success");
        manager.add_tool_use("read_file", "more contents"); // Duplicate tool

        let tools = manager.tools_used();
        assert_eq!(tools.len(), 2); // Should dedupe
        assert!(tools.contains(&"read_file".to_string()));
        assert!(tools.contains(&"write_file".to_string()));

        std::env::remove_var("VT_SESSION_DIR");
    }

    #[tokio::test]
    #[serial]
    async fn test_list_empty_sessions_dir() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        std::env::set_var("VT_SESSION_DIR", temp_dir.path());

        let sessions = list_recent_sessions(10).await.expect("Failed to list");
        assert!(sessions.is_empty());

        std::env::remove_var("VT_SESSION_DIR");
    }

    #[tokio::test]
    #[serial]
    async fn test_list_recent_sessions_with_limit() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        std::env::set_var("VT_SESSION_DIR", temp_dir.path());

        // Create 5 sessions
        for i in 0..5 {
            let mut manager = QbitSessionManager::new(
                temp_dir.path().to_path_buf(),
                format!("model-{}", i),
                "provider",
            )
            .await
            .expect("Failed to create manager");

            manager.add_user_message(&format!("Message {}", i));
            manager.finalize().expect("Failed to finalize");
        }

        let sessions = list_recent_sessions(2).await.expect("Failed to list");
        assert_eq!(sessions.len(), 2);

        std::env::remove_var("VT_SESSION_DIR");
    }

    #[test]
    fn test_session_message_roundtrip() {
        // Test that messages survive serialization roundtrip
        let original = QbitSessionMessage {
            role: QbitMessageRole::Tool,
            content: "Tool result with special chars: <>&\"'".to_string(),
            tool_call_id: Some("call_123".to_string()),
            tool_name: Some("read_file".to_string()),
        };

        let json = serde_json::to_string(&original).unwrap();
        let restored: QbitSessionMessage = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.role, original.role);
        assert_eq!(restored.content, original.content);
        assert_eq!(restored.tool_call_id, original.tool_call_id);
        assert_eq!(restored.tool_name, original.tool_name);
    }

    #[tokio::test]
    #[serial]
    async fn test_session_finalization_creates_persisted_session() {
        // Test that finalizing a session creates a persistent file
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        std::env::set_var("VT_SESSION_DIR", temp_dir.path());

        // Create and populate a session
        let mut manager =
            QbitSessionManager::new(temp_dir.path().to_path_buf(), "test-model", "test-provider")
                .await
                .expect("Failed to create manager");

        manager.add_user_message("Test user message for finalization");
        manager.add_assistant_message("Test assistant response");

        // Finalize the session
        let finalized_path = manager.finalize().expect("Failed to finalize session");

        // Verify the file exists
        assert!(
            finalized_path.exists(),
            "Finalized session file should exist"
        );

        // Verify the file is in the temp directory
        assert!(
            finalized_path.starts_with(temp_dir.path()),
            "Session file should be in temp dir"
        );

        // Verify the file has expected content (JSON format)
        let content = std::fs::read_to_string(&finalized_path).expect("Failed to read session");
        assert!(
            content.contains("test-model"),
            "Session file should contain model name"
        );
        assert!(
            content.contains("test-provider"),
            "Session file should contain provider name"
        );
        // Check for message content or structure
        assert!(
            content.contains("messages") || content.contains("Test user message"),
            "Session file should contain messages data"
        );

        std::env::remove_var("VT_SESSION_DIR");
    }

    #[tokio::test]
    #[serial]
    async fn test_session_finalization_is_one_shot() {
        // Test that finalize() can only be called once - subsequent calls fail
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        std::env::set_var("VT_SESSION_DIR", temp_dir.path());

        let mut manager =
            QbitSessionManager::new(temp_dir.path().to_path_buf(), "test-model", "test-provider")
                .await
                .expect("Failed to create manager");

        manager.add_user_message("Test message");

        // First finalize should succeed
        let result1 = manager.finalize();
        assert!(result1.is_ok(), "First finalize should succeed");

        // Second finalize should fail (archive already taken)
        let result2 = manager.finalize();
        assert!(
            result2.is_err(),
            "Second finalize should fail - session already finalized"
        );

        std::env::remove_var("VT_SESSION_DIR");
    }

    #[tokio::test]
    #[serial]
    async fn test_session_save_allows_incremental_persistence() {
        // Test that save() can be called multiple times (unlike finalize)
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        std::env::set_var("VT_SESSION_DIR", temp_dir.path());

        let mut manager =
            QbitSessionManager::new(temp_dir.path().to_path_buf(), "test-model", "test-provider")
                .await
                .expect("Failed to create manager");

        manager.add_user_message("First message");

        // First save should succeed
        let path1 = manager.save().expect("First save should succeed");
        assert!(path1.exists());

        // Add more messages and save again
        manager.add_assistant_message("Response to first");
        manager.add_user_message("Second message");

        // Second save should also succeed
        let path2 = manager.save().expect("Second save should succeed");
        assert!(path2.exists());
        assert_eq!(path1, path2, "Save should write to the same file");

        // Finalize should still work after saves
        let final_path = manager
            .finalize()
            .expect("Finalize should work after saves");
        assert!(final_path.exists());

        std::env::remove_var("VT_SESSION_DIR");
    }
}
