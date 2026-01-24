//! Request and response types for the Z.AI API.
//!
//! These types match the Z.AI API specification as documented in the Python SDK.

use serde::{Deserialize, Serialize};

// ============================================================================
// Request Types
// ============================================================================

/// Role in a conversation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    System,
    User,
    Assistant,
    Tool,
}

/// Content part for multi-modal messages
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentPart {
    /// Text content
    Text { text: String },
    /// Image content (base64 or URL)
    #[serde(rename = "image_url")]
    ImageUrl { image_url: ImageUrl },
}

/// Image URL structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageUrl {
    /// URL or base64 data URI
    pub url: String,
    /// Optional detail level
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

/// Message content - can be a string or array of content parts
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MessageContent {
    /// Simple text content
    Text(String),
    /// Multi-modal content parts
    Parts(Vec<ContentPart>),
}

/// A message in the conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// Role of the message sender
    pub role: Role,
    /// Message content
    pub content: MessageContent,
    /// Tool calls made by the assistant (only for assistant role)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
    /// Tool call ID (only for tool role)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    /// Name of the tool (only for tool role)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

impl Message {
    /// Create a system message
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: Role::System,
            content: MessageContent::Text(content.into()),
            tool_calls: None,
            tool_call_id: None,
            name: None,
        }
    }

    /// Create a user message
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: Role::User,
            content: MessageContent::Text(content.into()),
            tool_calls: None,
            tool_call_id: None,
            name: None,
        }
    }

    /// Create an assistant message
    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: Role::Assistant,
            content: MessageContent::Text(content.into()),
            tool_calls: None,
            tool_call_id: None,
            name: None,
        }
    }

    /// Create a tool result message
    pub fn tool_result(tool_call_id: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            role: Role::Tool,
            content: MessageContent::Text(content.into()),
            tool_calls: None,
            tool_call_id: Some(tool_call_id.into()),
            name: None,
        }
    }
}

/// Tool call from the assistant
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    /// Unique ID for the tool call
    pub id: String,
    /// Type of the tool call (always "function")
    #[serde(rename = "type")]
    pub call_type: String,
    /// Function details
    pub function: FunctionCall,
}

/// Function call details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionCall {
    /// Function name
    pub name: String,
    /// Function arguments as JSON string
    pub arguments: String,
}

/// Thinking/reasoning configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThinkingConfig {
    /// Type of thinking: "enabled" or "disabled"
    #[serde(rename = "type")]
    pub thinking_type: String,
}

impl ThinkingConfig {
    /// Create an enabled thinking config
    pub fn enabled() -> Self {
        Self {
            thinking_type: "enabled".to_string(),
        }
    }

    /// Create a disabled thinking config
    #[allow(dead_code)]
    pub fn disabled() -> Self {
        Self {
            thinking_type: "disabled".to_string(),
        }
    }
}

/// Tool definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    /// Type of the tool (always "function")
    #[serde(rename = "type")]
    pub tool_type: String,
    /// Function definition
    pub function: FunctionDefinition,
}

/// Function definition for tools
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionDefinition {
    /// Function name
    pub name: String,
    /// Function description
    pub description: String,
    /// Parameters schema (JSON Schema)
    pub parameters: serde_json::Value,
}

/// Chat completion request
#[derive(Debug, Clone, Serialize)]
pub struct CompletionRequest {
    /// Model identifier
    pub model: String,
    /// Messages in the conversation
    pub messages: Vec<Message>,
    /// Whether to stream the response
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
    /// Sampling temperature (0.0 to 1.0)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    /// Top-p sampling
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    /// Maximum tokens to generate
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    /// Stop sequences
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop: Option<Vec<String>>,
    /// Random seed for reproducibility
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed: Option<i64>,
    /// Tools available to the model
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<ToolDefinition>>,
    /// Tool choice strategy
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<serde_json::Value>,
    /// Enable thinking/reasoning (always enabled for this SDK)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking: Option<ThinkingConfig>,
    /// Enable tool streaming (always true for streaming requests)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_stream: Option<bool>,
}

impl Default for CompletionRequest {
    fn default() -> Self {
        Self {
            model: String::new(),
            messages: Vec::new(),
            stream: None,
            temperature: None,
            top_p: None,
            max_tokens: None,
            stop: None,
            seed: None,
            tools: None,
            tool_choice: None,
            thinking: Some(ThinkingConfig::enabled()), // Always enable thinking
            tool_stream: None,
        }
    }
}

// ============================================================================
// Response Types (Non-streaming)
// ============================================================================

/// Prompt tokens details
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PromptTokensDetails {
    /// Number of tokens reused from cache
    #[serde(default)]
    pub cached_tokens: u32,
}

/// Completion tokens details
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CompletionTokensDetails {
    /// Number of tokens used for reasoning
    #[serde(default)]
    pub reasoning_tokens: u32,
}

/// Token usage information
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Usage {
    /// Tokens in the prompt
    pub prompt_tokens: u32,
    /// Detailed prompt token breakdown
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_tokens_details: Option<PromptTokensDetails>,
    /// Tokens in the completion
    pub completion_tokens: u32,
    /// Detailed completion token breakdown
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completion_tokens_details: Option<CompletionTokensDetails>,
    /// Total tokens used
    pub total_tokens: u32,
}

/// Completion message from the assistant
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionMessage {
    /// Message content
    #[serde(default)]
    pub content: Option<String>,
    /// Role (always "assistant")
    pub role: String,
    /// Reasoning content (thinking)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_content: Option<String>,
    /// Tool calls made by the assistant
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
}

/// A completion choice
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionChoice {
    /// Index of this choice
    pub index: u32,
    /// Reason the completion finished
    pub finish_reason: String,
    /// The completion message
    pub message: CompletionMessage,
}

/// Non-streaming completion response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Completion {
    /// Unique ID for the completion
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// Model used
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// Creation timestamp
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created: Option<i64>,
    /// Completion choices
    pub choices: Vec<CompletionChoice>,
    /// Token usage
    pub usage: Usage,
    /// Request ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
}

// ============================================================================
// Streaming Response Types
// ============================================================================

/// Tool call delta in streaming response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChoiceDeltaToolCall {
    /// Index of this tool call (for accumulation)
    pub index: u32,
    /// Tool call ID (only in first delta for this index)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// Function details
    #[serde(skip_serializing_if = "Option::is_none")]
    pub function: Option<ChoiceDeltaFunction>,
    /// Type of tool call
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "type")]
    pub call_type: Option<String>,
}

/// Function delta in streaming response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChoiceDeltaFunction {
    /// Function name (only in first delta)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Partial arguments (accumulated across deltas)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<String>,
}

/// Delta content in streaming response
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ChoiceDelta {
    /// Content delta
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    /// Role (only in first delta)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    /// Reasoning content delta (thinking)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_content: Option<String>,
    /// Tool call deltas
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ChoiceDeltaToolCall>>,
}

/// Streaming choice
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamingChoice {
    /// Delta content
    pub delta: ChoiceDelta,
    /// Finish reason (only in final chunk)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finish_reason: Option<String>,
    /// Index of this choice
    pub index: u32,
}

/// Streaming completion chunk
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionChunk {
    /// Unique ID for the completion
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// Choices in this chunk
    pub choices: Vec<StreamingChoice>,
    /// Creation timestamp
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created: Option<i64>,
    /// Model used
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// Token usage (only in final chunk)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<Usage>,
}

// ============================================================================
// Error Response
// ============================================================================

/// API error response
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct ApiError {
    /// Error message
    pub message: String,
    /// Error type
    #[serde(rename = "type")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_type: Option<String>,
    /// Error code
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
}

/// Error wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct ErrorResponse {
    /// The error details
    pub error: ApiError,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_serialization() {
        let msg = Message::user("Hello, world!");
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"role\":\"user\""));
        assert!(json.contains("\"content\":\"Hello, world!\""));
    }

    #[test]
    fn test_tool_call_serialization() {
        let tool_call = ToolCall {
            id: "call_123".to_string(),
            call_type: "function".to_string(),
            function: FunctionCall {
                name: "get_weather".to_string(),
                arguments: "{\"location\": \"NYC\"}".to_string(),
            },
        };
        let json = serde_json::to_string(&tool_call).unwrap();
        assert!(json.contains("\"id\":\"call_123\""));
        assert!(json.contains("\"name\":\"get_weather\""));
    }

    #[test]
    fn test_completion_request_defaults() {
        let req = CompletionRequest::default();
        assert!(req.thinking.is_some());
        assert_eq!(req.thinking.as_ref().unwrap().thinking_type, "enabled");
    }

    #[test]
    fn test_streaming_chunk_deserialization() {
        let json = r#"{
            "id": "chunk_123",
            "choices": [{
                "delta": {"content": "Hello"},
                "index": 0
            }],
            "created": 1234567890,
            "model": "glm-4"
        }"#;
        let chunk: ChatCompletionChunk = serde_json::from_str(json).unwrap();
        assert_eq!(chunk.id, Some("chunk_123".to_string()));
        assert_eq!(chunk.choices.len(), 1);
        assert_eq!(chunk.choices[0].delta.content, Some("Hello".to_string()));
    }

    #[test]
    fn test_reasoning_content_deserialization() {
        let json = r#"{
            "id": "chunk_456",
            "choices": [{
                "delta": {"reasoning_content": "Let me think..."},
                "index": 0
            }],
            "model": "glm-4"
        }"#;
        let chunk: ChatCompletionChunk = serde_json::from_str(json).unwrap();
        assert_eq!(
            chunk.choices[0].delta.reasoning_content,
            Some("Let me think...".to_string())
        );
    }
}
