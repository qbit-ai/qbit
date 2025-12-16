//! Session message types for AI conversation persistence.
//!
//! This module provides drop-in replacements for vtcode-core's message types:
//! - `MessageRole` - replaces `vtcode_core::llm::provider::MessageRole`
//! - `MessageContent` - wrapper type for message content with `as_text()` method
//! - `SessionMessage` - replaces `vtcode_core::utils::session_archive::SessionMessage`

use serde::{Deserialize, Serialize};

/// Message role enum - drop-in replacement for `vtcode_core::llm::provider::MessageRole`.
///
/// IMPORTANT: The serialization format uses PascalCase ("User", "Assistant", etc.)
/// to maintain backwards compatibility with existing session files.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MessageRole {
    User,
    Assistant,
    System,
    Tool,
}

/// Message content wrapper providing the `as_text()` method.
///
/// This type handles both simple string content and structured content
/// for backwards compatibility with existing session files.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum MessageContent {
    /// Simple text content (most common)
    Text(String),
    /// Structured content with explicit text field
    Structured { text: String },
}

impl MessageContent {
    /// Extract text content.
    ///
    /// This matches the vtcode-core interface: `message.content.as_text()`
    pub fn as_text(&self) -> String {
        match self {
            MessageContent::Text(s) => s.clone(),
            MessageContent::Structured { text } => text.clone(),
        }
    }

    /// Create from a string slice.
    pub fn from_text(s: &str) -> Self {
        MessageContent::Text(s.to_string())
    }
}

impl From<String> for MessageContent {
    fn from(s: String) -> Self {
        MessageContent::Text(s)
    }
}

impl From<&str> for MessageContent {
    fn from(s: &str) -> Self {
        MessageContent::Text(s.to_string())
    }
}

/// Session message - drop-in replacement for `vtcode_core::utils::session_archive::SessionMessage`.
///
/// ## Interface Contract
///
/// The following interface MUST be preserved for compatibility:
///
/// ```rust,ignore
/// // Creation with tool_call_id (session.rs:299, 344)
/// SessionMessage::with_tool_call_id(role, content, tool_call_id)
///
/// // Content access (session.rs:488)
/// message.content.as_text()
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMessage {
    /// Message role (User, Assistant, System, Tool)
    pub role: MessageRole,
    /// Message content
    pub content: MessageContent,
    /// Optional tool call ID for tool messages
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

impl SessionMessage {
    /// Create a message with an optional tool call ID.
    ///
    /// This is the primary factory method used by session.rs.
    ///
    /// ## Signature matches
    /// `vtcode_core::utils::session_archive::SessionMessage::with_tool_call_id()`
    pub fn with_tool_call_id(
        role: MessageRole,
        content: &str,
        tool_call_id: Option<String>,
    ) -> Self {
        Self {
            role,
            content: MessageContent::Text(content.to_string()),
            tool_call_id,
        }
    }

    /// Create a simple message without tool call ID.
    pub fn new(role: MessageRole, content: &str) -> Self {
        Self::with_tool_call_id(role, content, None)
    }

    /// Create a user message.
    pub fn user(content: &str) -> Self {
        Self::new(MessageRole::User, content)
    }

    /// Create an assistant message.
    pub fn assistant(content: &str) -> Self {
        Self::new(MessageRole::Assistant, content)
    }

    /// Create a system message.
    pub fn system(content: &str) -> Self {
        Self::new(MessageRole::System, content)
    }

    /// Create a tool message with tool call ID.
    pub fn tool(content: &str, tool_call_id: impl Into<String>) -> Self {
        Self::with_tool_call_id(MessageRole::Tool, content, Some(tool_call_id.into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==========================================================================
    // MessageRole Tests
    // ==========================================================================

    mod message_role {
        use super::*;

        #[test]
        fn serializes_to_pascal_case() {
            // This is critical for backwards compatibility with existing session files
            assert_eq!(
                serde_json::to_string(&MessageRole::User).unwrap(),
                "\"User\""
            );
            assert_eq!(
                serde_json::to_string(&MessageRole::Assistant).unwrap(),
                "\"Assistant\""
            );
            assert_eq!(
                serde_json::to_string(&MessageRole::System).unwrap(),
                "\"System\""
            );
            assert_eq!(
                serde_json::to_string(&MessageRole::Tool).unwrap(),
                "\"Tool\""
            );
        }

        #[test]
        fn deserializes_from_pascal_case() {
            assert_eq!(
                serde_json::from_str::<MessageRole>("\"User\"").unwrap(),
                MessageRole::User
            );
            assert_eq!(
                serde_json::from_str::<MessageRole>("\"Assistant\"").unwrap(),
                MessageRole::Assistant
            );
            assert_eq!(
                serde_json::from_str::<MessageRole>("\"System\"").unwrap(),
                MessageRole::System
            );
            assert_eq!(
                serde_json::from_str::<MessageRole>("\"Tool\"").unwrap(),
                MessageRole::Tool
            );
        }

        #[test]
        fn all_variants_are_copy() {
            let role = MessageRole::User;
            let _copy1 = role;
            let _copy2 = role; // Should compile since MessageRole is Copy
        }
    }

    // ==========================================================================
    // MessageContent Tests
    // ==========================================================================

    mod message_content {
        use super::*;

        #[test]
        fn as_text_returns_text_content() {
            let content = MessageContent::Text("Hello, world!".to_string());
            assert_eq!(content.as_text(), "Hello, world!");
        }

        #[test]
        fn as_text_returns_structured_text() {
            let content = MessageContent::Structured {
                text: "Structured content".to_string(),
            };
            assert_eq!(content.as_text(), "Structured content");
        }

        #[test]
        fn serializes_text_as_plain_string() {
            let content = MessageContent::Text("plain text".to_string());
            assert_eq!(serde_json::to_string(&content).unwrap(), "\"plain text\"");
        }

        #[test]
        fn deserializes_from_plain_string() {
            let content: MessageContent = serde_json::from_str("\"plain text\"").unwrap();
            assert_eq!(content.as_text(), "plain text");
        }

        #[test]
        fn deserializes_from_structured_object() {
            let content: MessageContent =
                serde_json::from_str(r#"{"text": "structured"}"#).unwrap();
            assert_eq!(content.as_text(), "structured");
        }

        #[test]
        fn from_string_creates_text_variant() {
            let content: MessageContent = "from string".to_string().into();
            assert_eq!(content.as_text(), "from string");
        }

        #[test]
        fn from_str_creates_text_variant() {
            let content: MessageContent = "from str".into();
            assert_eq!(content.as_text(), "from str");
        }

        #[test]
        fn handles_special_characters() {
            let content = MessageContent::Text("Hello <tag> & \"quotes\"".to_string());
            let json = serde_json::to_string(&content).unwrap();
            let restored: MessageContent = serde_json::from_str(&json).unwrap();
            assert_eq!(restored.as_text(), "Hello <tag> & \"quotes\"");
        }

        #[test]
        fn handles_unicode() {
            let content = MessageContent::Text("Hello, world!".to_string());
            let json = serde_json::to_string(&content).unwrap();
            let restored: MessageContent = serde_json::from_str(&json).unwrap();
            assert_eq!(restored.as_text(), "Hello, world!");
        }

        #[test]
        fn handles_empty_string() {
            let content = MessageContent::Text(String::new());
            assert_eq!(content.as_text(), "");
        }

        #[test]
        fn handles_multiline_content() {
            let content = MessageContent::Text("line1\nline2\nline3".to_string());
            let json = serde_json::to_string(&content).unwrap();
            let restored: MessageContent = serde_json::from_str(&json).unwrap();
            assert_eq!(restored.as_text(), "line1\nline2\nline3");
        }
    }

    // ==========================================================================
    // SessionMessage Tests
    // ==========================================================================

    mod session_message {
        use super::*;

        #[test]
        fn with_tool_call_id_creates_message() {
            let msg = SessionMessage::with_tool_call_id(
                MessageRole::User,
                "Hello",
                Some("id_123".into()),
            );

            assert_eq!(msg.role, MessageRole::User);
            assert_eq!(msg.content.as_text(), "Hello");
            assert_eq!(msg.tool_call_id, Some("id_123".to_string()));
        }

        #[test]
        fn with_tool_call_id_none() {
            let msg = SessionMessage::with_tool_call_id(MessageRole::Assistant, "Response", None);

            assert_eq!(msg.role, MessageRole::Assistant);
            assert_eq!(msg.content.as_text(), "Response");
            assert_eq!(msg.tool_call_id, None);
        }

        #[test]
        fn new_creates_message_without_tool_call_id() {
            let msg = SessionMessage::new(MessageRole::System, "System prompt");

            assert_eq!(msg.role, MessageRole::System);
            assert_eq!(msg.content.as_text(), "System prompt");
            assert_eq!(msg.tool_call_id, None);
        }

        #[test]
        fn user_creates_user_message() {
            let msg = SessionMessage::user("User input");
            assert_eq!(msg.role, MessageRole::User);
            assert_eq!(msg.content.as_text(), "User input");
        }

        #[test]
        fn assistant_creates_assistant_message() {
            let msg = SessionMessage::assistant("Assistant response");
            assert_eq!(msg.role, MessageRole::Assistant);
            assert_eq!(msg.content.as_text(), "Assistant response");
        }

        #[test]
        fn system_creates_system_message() {
            let msg = SessionMessage::system("System instructions");
            assert_eq!(msg.role, MessageRole::System);
            assert_eq!(msg.content.as_text(), "System instructions");
        }

        #[test]
        fn tool_creates_tool_message_with_id() {
            let msg = SessionMessage::tool("Tool result", "tool_call_456");
            assert_eq!(msg.role, MessageRole::Tool);
            assert_eq!(msg.content.as_text(), "Tool result");
            assert_eq!(msg.tool_call_id, Some("tool_call_456".to_string()));
        }

        #[test]
        fn serialization_includes_all_fields() {
            let msg = SessionMessage::tool("result", "call_id");
            let json = serde_json::to_string(&msg).unwrap();

            assert!(json.contains("\"role\":\"Tool\""));
            assert!(json.contains("\"content\":\"result\""));
            assert!(json.contains("\"tool_call_id\":\"call_id\""));
        }

        #[test]
        fn serialization_skips_none_tool_call_id() {
            let msg = SessionMessage::user("Hello");
            let json = serde_json::to_string(&msg).unwrap();

            assert!(!json.contains("tool_call_id"));
        }

        #[test]
        fn roundtrip_serialization() {
            let original = SessionMessage::with_tool_call_id(
                MessageRole::Tool,
                "File contents: test data",
                Some("call_789".to_string()),
            );

            let json = serde_json::to_string(&original).unwrap();
            let restored: SessionMessage = serde_json::from_str(&json).unwrap();

            assert_eq!(restored.role, original.role);
            assert_eq!(restored.content.as_text(), original.content.as_text());
            assert_eq!(restored.tool_call_id, original.tool_call_id);
        }

        #[test]
        fn deserializes_existing_session_format() {
            // This test verifies compatibility with existing session files
            let json = r#"{
                "role": "User",
                "content": "<context>\n<cwd>/Users/test</cwd>\n</context>\n\nHello",
                "tool_call_id": null
            }"#;

            let msg: SessionMessage = serde_json::from_str(json).unwrap();
            assert_eq!(msg.role, MessageRole::User);
            assert!(msg.content.as_text().contains("Hello"));
            assert!(msg.tool_call_id.is_none());
        }

        #[test]
        fn deserializes_without_tool_call_id_field() {
            // Some messages may not have tool_call_id field at all
            let json = r#"{
                "role": "Assistant",
                "content": "Response text"
            }"#;

            let msg: SessionMessage = serde_json::from_str(json).unwrap();
            assert_eq!(msg.role, MessageRole::Assistant);
            assert_eq!(msg.content.as_text(), "Response text");
            assert!(msg.tool_call_id.is_none());
        }
    }
}
