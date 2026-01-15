//! Request and response types for Anthropic Vertex AI API.

use serde::{Deserialize, Serialize};

/// Anthropic API version for Vertex AI
pub const ANTHROPIC_VERSION: &str = "vertex-2023-10-16";

/// Maximum tokens default
pub const DEFAULT_MAX_TOKENS: u32 = 4096;

/// Configuration for extended thinking (reasoning) mode.
/// When enabled, the model will show its reasoning process before responding.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThinkingConfig {
    /// Must be "enabled" to activate extended thinking
    #[serde(rename = "type")]
    pub thinking_type: String,
    /// Token budget for thinking (must be >= 1024)
    pub budget_tokens: u32,
}

impl ThinkingConfig {
    /// Create a new thinking configuration with the specified budget.
    /// Budget must be at least 1024 tokens.
    pub fn new(budget_tokens: u32) -> Self {
        Self {
            thinking_type: "enabled".to_string(),
            budget_tokens: budget_tokens.max(1024),
        }
    }

    /// Create a thinking config with a default budget of 10,000 tokens
    pub fn default_budget() -> Self {
        Self::new(10_000)
    }
}

/// Cache control configuration for prompt caching.
/// When set, marks content as cacheable with the specified type.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheControl {
    /// The cache type. Currently only "ephemeral" is supported.
    #[serde(rename = "type")]
    pub cache_type: String,
}

impl CacheControl {
    /// Create an ephemeral cache control marker.
    /// Cached content has a 5-minute TTL, refreshed on each hit.
    pub fn ephemeral() -> Self {
        Self {
            cache_type: "ephemeral".to_string(),
        }
    }
}

/// A block in the system prompt array.
/// Required for prompt caching - the single-string format does not support cache_control.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemBlock {
    /// Block type (always "text" for system prompts)
    #[serde(rename = "type")]
    pub block_type: String,
    /// The text content
    pub text: String,
    /// Optional cache control marker
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_control: Option<CacheControl>,
}

impl SystemBlock {
    /// Create a new text system block without caching.
    pub fn text(content: impl Into<String>) -> Self {
        Self {
            block_type: "text".to_string(),
            text: content.into(),
            cache_control: None,
        }
    }

    /// Create a new text system block with ephemeral caching.
    pub fn cached(content: impl Into<String>) -> Self {
        Self {
            block_type: "text".to_string(),
            text: content.into(),
            cache_control: Some(CacheControl::ephemeral()),
        }
    }
}

/// Content block in a message
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentBlock {
    /// Text content
    Text {
        text: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        cache_control: Option<CacheControl>,
    },
    /// Image content (base64 encoded)
    Image {
        source: ImageSource,
        #[serde(skip_serializing_if = "Option::is_none")]
        cache_control: Option<CacheControl>,
    },
    /// Tool use request from the model
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
    },
    /// Tool result from execution
    ToolResult {
        tool_use_id: String,
        content: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        is_error: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        cache_control: Option<CacheControl>,
    },
    /// Thinking/reasoning content from extended thinking mode
    Thinking {
        thinking: String,
        /// Signature for verification (provided by API)
        signature: String,
    },
    /// Server tool use (Claude's native web_search/web_fetch)
    /// These are initiated by Claude and executed server-side
    ServerToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
    },
    /// Web search tool result from Claude's native web search
    WebSearchToolResult {
        tool_use_id: String,
        content: serde_json::Value, // WebSearchToolResultContent
    },
    /// Web fetch tool result from Claude's native web fetch
    WebFetchToolResult {
        tool_use_id: String,
        content: serde_json::Value, // WebFetchToolResultContent
    },
}

/// Image source for image content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageSource {
    #[serde(rename = "type")]
    pub source_type: String,
    pub media_type: String,
    pub data: String,
}

/// Role in a conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    User,
    Assistant,
}

/// A message in the conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: Role,
    pub content: Vec<ContentBlock>,
}

impl Message {
    /// Create a user message with text content
    pub fn user(text: impl Into<String>) -> Self {
        Self {
            role: Role::User,
            content: vec![ContentBlock::Text {
                text: text.into(),
                cache_control: None,
            }],
        }
    }

    /// Create an assistant message with text content
    pub fn assistant(text: impl Into<String>) -> Self {
        Self {
            role: Role::Assistant,
            content: vec![ContentBlock::Text {
                text: text.into(),
                cache_control: None,
            }],
        }
    }
}

/// Tool definition for the API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
    /// Optional cache control marker for caching tool definitions
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_control: Option<CacheControl>,
}

/// Request body for the Anthropic Vertex AI API
#[derive(Debug, Clone, Serialize)]
pub struct CompletionRequest {
    /// Anthropic API version
    pub anthropic_version: String,
    /// Messages in the conversation
    pub messages: Vec<Message>,
    /// Maximum tokens to generate
    pub max_tokens: u32,
    /// System prompt as array of blocks (required for caching).
    /// If None, no system prompt is sent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<Vec<SystemBlock>>,
    /// Temperature for sampling (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    /// Top-p sampling (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    /// Top-k sampling (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_k: Option<u32>,
    /// Stop sequences (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_sequences: Option<Vec<String>>,
    /// Tools available to the model (optional)
    /// Can contain both function tools and server tools (web_search, web_fetch)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<ToolEntry>>,
    /// Whether to stream the response
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
    /// Extended thinking configuration (optional)
    /// When enabled, temperature must be 1 and budget_tokens >= 1024
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking: Option<ThinkingConfig>,
}

impl Default for CompletionRequest {
    fn default() -> Self {
        Self {
            anthropic_version: ANTHROPIC_VERSION.to_string(),
            messages: Vec::new(),
            max_tokens: DEFAULT_MAX_TOKENS,
            system: None,
            temperature: None,
            top_p: None,
            top_k: None,
            stop_sequences: None,
            tools: None,
            stream: None,
            thinking: None,
        }
    }
}

/// Usage statistics in the response
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Usage {
    /// Input tokens (may be missing in message_delta events)
    #[serde(default)]
    pub input_tokens: u32,
    pub output_tokens: u32,
    /// Tokens used to create new cache entries
    #[serde(default)]
    pub cache_creation_input_tokens: u32,
    /// Tokens read from cache (cache hit)
    #[serde(default)]
    pub cache_read_input_tokens: u32,
}

/// Stop reason for completion
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StopReason {
    EndTurn,
    MaxTokens,
    StopSequence,
    ToolUse,
}

/// Response from the Anthropic Vertex AI API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionResponse {
    /// Unique ID for the response
    pub id: String,
    /// Type of response (always "message")
    #[serde(rename = "type")]
    pub response_type: String,
    /// Role (always "assistant")
    pub role: String,
    /// Content blocks
    pub content: Vec<ContentBlock>,
    /// Model that generated the response
    pub model: String,
    /// Reason the model stopped generating
    pub stop_reason: Option<StopReason>,
    /// Stop sequence that triggered stopping (if applicable)
    pub stop_sequence: Option<String>,
    /// Token usage statistics
    pub usage: Usage,
}

impl CompletionResponse {
    /// Extract text content from the response
    pub fn text(&self) -> String {
        self.content
            .iter()
            .filter_map(|block| match block {
                ContentBlock::Text { text, .. } => Some(text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("")
    }

    /// Extract tool use blocks from the response
    pub fn tool_uses(&self) -> Vec<(&str, &str, &serde_json::Value)> {
        self.content
            .iter()
            .filter_map(|block| match block {
                ContentBlock::ToolUse { id, name, input } => {
                    Some((id.as_str(), name.as_str(), input))
                }
                _ => None,
            })
            .collect()
    }

    /// Extract thinking/reasoning content from the response
    pub fn thinking(&self) -> Option<&str> {
        self.content.iter().find_map(|block| match block {
            ContentBlock::Thinking { thinking, .. } => Some(thinking.as_str()),
            _ => None,
        })
    }
}

/// Streaming event types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StreamEvent {
    /// Initial message start event
    MessageStart { message: StreamMessageStart },
    /// Content block started
    ContentBlockStart {
        index: usize,
        content_block: ContentBlock,
    },
    /// Delta for content block
    ContentBlockDelta { index: usize, delta: ContentDelta },
    /// Content block finished
    ContentBlockStop { index: usize },
    /// Final message delta with usage
    MessageDelta {
        delta: MessageDeltaContent,
        usage: Usage,
    },
    /// Message complete
    MessageStop,
    /// Ping event (keep-alive)
    Ping,
    /// Error event
    Error { error: StreamError },
}

/// Message start in streaming
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamMessageStart {
    pub id: String,
    #[serde(rename = "type")]
    pub message_type: String,
    pub role: String,
    pub model: String,
    pub usage: Usage,
}

/// Content delta in streaming
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentDelta {
    TextDelta {
        text: String,
    },
    InputJsonDelta {
        partial_json: String,
    },
    /// Thinking content delta (streamed reasoning)
    ThinkingDelta {
        thinking: String,
    },
    /// Signature delta for thinking blocks
    SignatureDelta {
        signature: String,
    },
}

/// Message delta content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageDeltaContent {
    pub stop_reason: Option<StopReason>,
    pub stop_sequence: Option<String>,
}

/// Error in streaming
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamError {
    #[serde(rename = "type")]
    pub error_type: String,
    pub message: String,
}

// ============================================================================
// Server Tools (Claude Native Web Tools)
// ============================================================================

/// Configuration for citations in web fetch results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CitationsConfig {
    pub enabled: bool,
}

/// Server tool definitions for Claude's native tools.
/// These use a type-based format instead of the function-based format.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ServerTool {
    /// Native web search tool (web_search_20250305)
    #[serde(rename = "web_search_20250305")]
    WebSearch {
        name: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        max_uses: Option<u32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        allowed_domains: Option<Vec<String>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        blocked_domains: Option<Vec<String>>,
    },
    /// Native web fetch tool (web_fetch_20250910)
    #[serde(rename = "web_fetch_20250910")]
    WebFetch {
        name: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        max_uses: Option<u32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        citations: Option<CitationsConfig>,
        #[serde(skip_serializing_if = "Option::is_none")]
        max_content_tokens: Option<u32>,
    },
}

/// Union type for the tools array in API requests.
/// Can contain both function tools and server tools.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ToolEntry {
    /// Traditional function-based tool definition
    Function(ToolDefinition),
    /// Server-side tool (web_search, web_fetch)
    Server(ServerTool),
}

// ============================================================================
// Server Tool Result Types
// ============================================================================

/// Web search result from Claude's native web search
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSearchResult {
    /// URL of the search result
    pub url: String,
    /// Title of the page
    pub title: String,
    /// Encrypted content (must be passed back for citations)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub encrypted_content: Option<String>,
    /// When the page was last updated
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_age: Option<String>,
}

/// Document source in web fetch result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebFetchDocumentSource {
    #[serde(rename = "type")]
    pub source_type: String,
    pub media_type: String,
    pub data: String,
}

/// Document content in web fetch result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebFetchDocument {
    #[serde(rename = "type")]
    pub doc_type: String,
    pub source: WebFetchDocumentSource,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub citations: Option<CitationsConfig>,
}

/// Web fetch result content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebFetchResultContent {
    #[serde(rename = "type")]
    pub content_type: String,
    pub url: String,
    pub content: WebFetchDocument,
    pub retrieved_at: String,
}

/// Web search tool result content (can be results or error)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum WebSearchToolResultContent {
    /// Successful search results
    Results(Vec<WebSearchResult>),
    /// Error response
    Error(WebToolError),
}

/// Web fetch tool result content (can be result or error)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum WebFetchToolResultContent {
    /// Successful fetch result
    Result(WebFetchResultContent),
    /// Error response
    Error(WebToolError),
}

/// Error from web tools
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebToolError {
    #[serde(rename = "type")]
    pub error_type: String,
    pub error_code: String,
}

/// Citation from web search or fetch
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebCitation {
    #[serde(rename = "type")]
    pub citation_type: String,
    pub url: String,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub encrypted_index: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cited_text: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_image_content_block_serialization() {
        let img = ContentBlock::Image {
            source: ImageSource {
                source_type: "base64".to_string(),
                media_type: "image/png".to_string(),
                data: "iVBORw0KGgoAAAANSUhEUg==".to_string(),
            },
            cache_control: None,
        };

        let json = serde_json::to_string(&img).unwrap();
        println!("Image ContentBlock JSON: {}", json);

        // Verify the structure matches Anthropic's expected format
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["type"], "image");
        assert_eq!(parsed["source"]["type"], "base64");
        assert_eq!(parsed["source"]["media_type"], "image/png");
        assert!(parsed["source"]["data"].as_str().is_some());
    }

    #[test]
    fn test_message_with_image_serialization() {
        let msg = Message {
            role: Role::User,
            content: vec![
                ContentBlock::Text {
                    text: "What is in this image?".to_string(),
                    cache_control: None,
                },
                ContentBlock::Image {
                    source: ImageSource {
                        source_type: "base64".to_string(),
                        media_type: "image/jpeg".to_string(),
                        data: "base64data".to_string(),
                    },
                    cache_control: None,
                },
            ],
        };

        let json = serde_json::to_string_pretty(&msg).unwrap();
        println!("Message with image JSON:\n{}", json);

        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["role"], "user");
        assert!(parsed["content"].is_array());
        assert_eq!(parsed["content"][0]["type"], "text");
        assert_eq!(parsed["content"][1]["type"], "image");
        assert_eq!(parsed["content"][1]["source"]["type"], "base64");
    }
}
