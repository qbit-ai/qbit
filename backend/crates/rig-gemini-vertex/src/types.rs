//! Request and response types for Gemini Vertex AI API.

use serde::{Deserialize, Serialize};

/// Default max output tokens
pub const DEFAULT_MAX_TOKENS: u32 = 8192;

// ============================================================================
// Request Types
// ============================================================================

/// Request body for the Gemini Vertex AI API (generateContent)
#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct GenerateContentRequest {
    /// Contents of the conversation
    pub contents: Vec<Content>,
    /// System instruction (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_instruction: Option<Content>,
    /// Tools available to the model
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<Tool>>,
    /// Tool configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_config: Option<ToolConfig>,
    /// Safety settings
    #[serde(skip_serializing_if = "Option::is_none")]
    pub safety_settings: Option<Vec<SafetySetting>>,
    /// Generation configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generation_config: Option<GenerationConfig>,
}

/// Content in a conversation (user or model message)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Content {
    /// Role: "user" or "model"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    /// Parts of the content
    pub parts: Vec<Part>,
}

impl Content {
    /// Create a user message with text content
    pub fn user(text: impl Into<String>) -> Self {
        Self {
            role: Some("user".to_string()),
            parts: vec![Part::text(text)],
        }
    }

    /// Create a model message with text content
    pub fn model(text: impl Into<String>) -> Self {
        Self {
            role: Some("model".to_string()),
            parts: vec![Part::text(text)],
        }
    }

    /// Create a system instruction (no role needed)
    pub fn system(text: impl Into<String>) -> Self {
        Self {
            role: None,
            parts: vec![Part::text(text)],
        }
    }
}

/// A part of content (text, function call, function response, etc.)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Part {
    /// Text content
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    /// Inline data (for images, etc.)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inline_data: Option<Blob>,
    /// File data reference
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_data: Option<FileData>,
    /// Function call from the model
    #[serde(skip_serializing_if = "Option::is_none")]
    pub function_call: Option<FunctionCall>,
    /// Function response from the user
    #[serde(skip_serializing_if = "Option::is_none")]
    pub function_response: Option<FunctionResponse>,
    /// Thought content (for thinking models)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thought: Option<bool>,
    /// Thought signature (required for function calls after thinking)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thought_signature: Option<String>,
}

impl Part {
    /// Create a text part
    pub fn text(text: impl Into<String>) -> Self {
        Self {
            text: Some(text.into()),
            ..Default::default()
        }
    }

    /// Create a function call part
    pub fn function_call(name: impl Into<String>, args: serde_json::Value) -> Self {
        Self {
            function_call: Some(FunctionCall {
                name: name.into(),
                args,
            }),
            ..Default::default()
        }
    }

    /// Create a function response part
    pub fn function_response(name: impl Into<String>, response: serde_json::Value) -> Self {
        Self {
            function_response: Some(FunctionResponse {
                name: name.into(),
                response,
            }),
            ..Default::default()
        }
    }

    /// Create an inline data part (for images)
    pub fn inline_data(mime_type: impl Into<String>, data: impl Into<String>) -> Self {
        Self {
            inline_data: Some(Blob {
                mime_type: mime_type.into(),
                data: data.into(),
            }),
            ..Default::default()
        }
    }
}

/// Inline binary data (base64 encoded)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Blob {
    /// MIME type of the data
    pub mime_type: String,
    /// Base64 encoded data
    pub data: String,
}

/// File data reference (Cloud Storage URI or URL)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileData {
    /// MIME type of the file
    pub mime_type: String,
    /// File URI (gs:// or https://)
    pub file_uri: String,
}

/// Function call from the model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionCall {
    /// Function name
    pub name: String,
    /// Function arguments as JSON
    pub args: serde_json::Value,
}

/// Function response from the user
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionResponse {
    /// Function name
    pub name: String,
    /// Function result as JSON
    pub response: serde_json::Value,
}

/// Tool definition
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Tool {
    /// Function declarations
    #[serde(skip_serializing_if = "Option::is_none")]
    pub function_declarations: Option<Vec<FunctionDeclaration>>,
}

/// Function declaration for tool use
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FunctionDeclaration {
    /// Function name
    pub name: String,
    /// Function description
    pub description: String,
    /// Parameters schema in standard JSON Schema format.
    /// This field uses the Gemini API's `parametersJsonSchema` which accepts
    /// standard JSON Schema format (with lowercase type names like "string", "integer")
    /// rather than Google's custom Schema format (with uppercase TYPE names).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameters_json_schema: Option<serde_json::Value>,
}

/// Tool configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolConfig {
    /// Function calling config
    #[serde(skip_serializing_if = "Option::is_none")]
    pub function_calling_config: Option<FunctionCallingConfig>,
}

/// Function calling configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FunctionCallingConfig {
    /// Mode: "AUTO", "ANY", or "NONE"
    pub mode: String,
    /// Allowed function names (for "ANY" mode)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_function_names: Option<Vec<String>>,
}

/// Safety setting
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafetySetting {
    /// Harm category
    pub category: HarmCategory,
    /// Block threshold
    pub threshold: HarmBlockThreshold,
}

/// Harm categories
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum HarmCategory {
    HarmCategoryUnspecified,
    HarmCategoryHateSpeech,
    HarmCategoryDangerousContent,
    HarmCategoryHarassment,
    HarmCategorySexuallyExplicit,
}

/// Harm block thresholds
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum HarmBlockThreshold {
    HarmBlockThresholdUnspecified,
    BlockLowAndAbove,
    BlockMediumAndAbove,
    BlockOnlyHigh,
    BlockNone,
    Off,
}

/// Generation configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct GenerationConfig {
    /// Temperature for sampling
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    /// Top-p sampling
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    /// Top-k sampling
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_k: Option<i32>,
    /// Number of candidates to generate
    #[serde(skip_serializing_if = "Option::is_none")]
    pub candidate_count: Option<i32>,
    /// Maximum output tokens
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_output_tokens: Option<i32>,
    /// Stop sequences
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_sequences: Option<Vec<String>>,
    /// Response MIME type
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_mime_type: Option<String>,
    /// Response schema for structured output
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_schema: Option<serde_json::Value>,
    /// Thinking configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking_config: Option<ThinkingConfig>,
}

/// Thinking configuration for reasoning models
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThinkingConfig {
    /// Thinking budget in tokens
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking_budget: Option<i32>,
    /// Thinking level: "LOW" or "HIGH"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking_level: Option<String>,
}

impl ThinkingConfig {
    /// Create a thinking config with the specified budget
    pub fn with_budget(budget: i32) -> Self {
        Self {
            thinking_budget: Some(budget),
            thinking_level: None,
        }
    }

    /// Create a thinking config with the specified level
    pub fn with_level(level: impl Into<String>) -> Self {
        Self {
            thinking_budget: None,
            thinking_level: Some(level.into()),
        }
    }
}

// ============================================================================
// Response Types
// ============================================================================

/// Response from generateContent
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerateContentResponse {
    /// Generated candidates
    #[serde(default)]
    pub candidates: Vec<Candidate>,
    /// Usage metadata
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage_metadata: Option<UsageMetadata>,
    /// Model version used
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_version: Option<String>,
}

impl GenerateContentResponse {
    /// Extract text content from the first candidate
    pub fn text(&self) -> String {
        self.candidates
            .first()
            .map(|c| c.text())
            .unwrap_or_default()
    }

    /// Extract function calls from the first candidate
    pub fn function_calls(&self) -> Vec<&FunctionCall> {
        self.candidates
            .first()
            .map(|c| c.function_calls())
            .unwrap_or_default()
    }
}

/// A generated candidate
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Candidate {
    /// Content of the candidate
    pub content: Content,
    /// Finish reason
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finish_reason: Option<FinishReason>,
    /// Safety ratings
    #[serde(skip_serializing_if = "Option::is_none")]
    pub safety_ratings: Option<Vec<SafetyRating>>,
    /// Citation metadata
    #[serde(skip_serializing_if = "Option::is_none")]
    pub citation_metadata: Option<CitationMetadata>,
    /// Average log probability
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avg_logprobs: Option<f64>,
}

impl Candidate {
    /// Extract text content from this candidate
    pub fn text(&self) -> String {
        self.content
            .parts
            .iter()
            .filter_map(|p| p.text.as_ref())
            .cloned()
            .collect::<Vec<_>>()
            .join("")
    }

    /// Extract function calls from this candidate
    pub fn function_calls(&self) -> Vec<&FunctionCall> {
        self.content
            .parts
            .iter()
            .filter_map(|p| p.function_call.as_ref())
            .collect()
    }
}

/// Finish reason for generation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FinishReason {
    FinishReasonUnspecified,
    Stop,
    MaxTokens,
    Safety,
    Recitation,
    Blocklist,
    ProhibitedContent,
    Spii,
    MalformedFunctionCall,
    Other,
}

/// Safety rating for content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafetyRating {
    /// Harm category
    pub category: HarmCategory,
    /// Probability level
    pub probability: HarmProbability,
    /// Whether the content was blocked
    #[serde(default)]
    pub blocked: bool,
}

/// Harm probability levels
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum HarmProbability {
    HarmProbabilityUnspecified,
    Negligible,
    Low,
    Medium,
    High,
}

/// Citation metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CitationMetadata {
    /// Citations
    pub citations: Vec<Citation>,
}

/// A citation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Citation {
    /// Start index in the response
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_index: Option<i32>,
    /// End index in the response
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_index: Option<i32>,
    /// Source URI
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uri: Option<String>,
    /// Source title
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// License
    #[serde(skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,
}

/// Usage metadata
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct UsageMetadata {
    /// Prompt token count
    #[serde(default)]
    pub prompt_token_count: i32,
    /// Candidates token count
    #[serde(default)]
    pub candidates_token_count: i32,
    /// Total token count
    #[serde(default)]
    pub total_token_count: i32,
}

// ============================================================================
// Streaming Types
// ============================================================================

/// Streaming response chunk (same structure as GenerateContentResponse)
pub type StreamingChunk = GenerateContentResponse;
