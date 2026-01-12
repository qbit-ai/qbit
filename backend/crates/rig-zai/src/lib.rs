//! Z.AI API client and Rig integration
//!
//! This crate provides integration with Z.AI's GLM models using the OpenAI-compatible
//! Coding Plan API endpoint. It implements rig-core's `CompletionModel` trait.
//!
//! # Example
//!
//! ```rust,no_run
//! use rig::client::CompletionClient;  // Trait for completion_model method
//! use rig_zai::Client;
//!
//! let client = Client::new("YOUR_API_KEY");
//!
//! // Use the default GLM-4.7 model
//! let glm_4_7 = client.completion_model(rig_zai::GLM_4_7);
//!
//! // Or the lightweight GLM-4.5-air model
//! let glm_4_5_air = client.completion_model(rig_zai::GLM_4_5_AIR);
//! ```
//!
//! # Environment Variables
//!
//! The client can be created from environment variables:
//! - `ZAI_API_KEY` - Your Z.AI API key (required)
//! - `ZAI_BASE_URL` - Custom base URL (optional, defaults to Coding Plan endpoint)

use rig::{
    client::{CompletionClient, ProviderClient},
    completion::{self, message, CompletionError, MessageError},
    http_client::sse::{Event, GenericEventSource},
    http_client::{self, HttpClientExt},
    message::ToolChoice,
    streaming::{self, RawStreamingChoice, RawStreamingToolCall, StreamingCompletionResponse},
    OneOrMany,
};

use async_stream::stream;
use bytes::Bytes;
use futures::StreamExt;
use http::Method;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use tracing::{info_span, Instrument};

// ================================================================
// JSON Utilities
// ================================================================

/// Merge two JSON objects. Keys in `b` override keys in `a`.
fn merge_json(a: Value, b: Value) -> Value {
    match (a, b) {
        (Value::Object(mut a_map), Value::Object(b_map)) => {
            b_map.into_iter().for_each(|(key, value)| {
                a_map.insert(key, value);
            });
            Value::Object(a_map)
        }
        (a, _) => a,
    }
}

// ================================================================
// Z.AI API Constants
// ================================================================

/// Z.AI Coding Plan API base URL
const ZAI_CODING_API_BASE_URL: &str = "https://api.z.ai/api/coding/paas/v4";

/// GLM-4.7 completion model - latest and most capable
pub const GLM_4_7: &str = "GLM-4.7";

/// GLM-4.5-air completion model - lightweight and faster
pub const GLM_4_5_AIR: &str = "GLM-4.5-air";

// ================================================================
// Client Builder
// ================================================================

pub struct ClientBuilder<'a, T = reqwest::Client> {
    api_key: &'a str,
    base_url: &'a str,
    http_client: T,
}

impl<'a, T> ClientBuilder<'a, T>
where
    T: Default,
{
    pub fn new(api_key: &'a str) -> Self {
        Self {
            api_key,
            base_url: ZAI_CODING_API_BASE_URL,
            http_client: Default::default(),
        }
    }
}

impl<'a, T> ClientBuilder<'a, T> {
    pub fn new_with_client(api_key: &'a str, http_client: T) -> Self {
        Self {
            api_key,
            base_url: ZAI_CODING_API_BASE_URL,
            http_client,
        }
    }

    /// Set a custom base URL (e.g., for the general API instead of coding API)
    pub fn base_url(mut self, base_url: &'a str) -> Self {
        self.base_url = base_url;
        self
    }

    pub fn with_client<U>(self, http_client: U) -> ClientBuilder<'a, U> {
        ClientBuilder {
            api_key: self.api_key,
            base_url: self.base_url,
            http_client,
        }
    }

    pub fn build(self) -> Client<T> {
        Client {
            base_url: self.base_url.to_string(),
            api_key: self.api_key.to_string(),
            http_client: self.http_client,
        }
    }
}

// ================================================================
// Z.AI Client
// ================================================================

#[derive(Clone)]
pub struct Client<T = reqwest::Client> {
    base_url: String,
    api_key: String,
    http_client: T,
}

impl<T> std::fmt::Debug for Client<T>
where
    T: std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Client")
            .field("base_url", &self.base_url)
            .field("http_client", &self.http_client)
            .field("api_key", &"<REDACTED>")
            .finish()
    }
}

impl<T> Client<T>
where
    T: HttpClientExt,
{
    fn req(
        &self,
        method: http_client::Method,
        path: &str,
    ) -> http_client::Result<http_client::Builder> {
        let url = format!("{}/{}", self.base_url, path.trim_start_matches('/'));
        let req = http_client::Builder::new().method(method).uri(url);

        http_client::with_bearer_auth(req, &self.api_key)
    }
}

impl Client<reqwest::Client> {
    pub fn builder(api_key: &str) -> ClientBuilder<'_, reqwest::Client> {
        ClientBuilder::new(api_key)
    }

    pub fn new(api_key: &str) -> Self {
        Self::builder(api_key).build()
    }

    pub fn from_env() -> Self {
        <Self as ProviderClient>::from_env()
    }
}

impl<T> ProviderClient for Client<T>
where
    T: HttpClientExt + Clone + std::fmt::Debug + Default + Send + 'static,
{
    type Input = String;

    /// Create a new Z.AI client from the `ZAI_API_KEY` environment variable.
    /// Optionally reads `ZAI_BASE_URL` for a custom endpoint.
    /// Panics if the API key environment variable is not set.
    fn from_env() -> Self {
        let api_key = std::env::var("ZAI_API_KEY").expect("ZAI_API_KEY not set");
        let base_url: Option<String> = std::env::var("ZAI_BASE_URL").ok();

        match base_url {
            Some(url) => ClientBuilder::<T>::new(&api_key).base_url(&url).build(),
            None => ClientBuilder::<T>::new(&api_key).build(),
        }
    }

    fn from_val(input: Self::Input) -> Self {
        ClientBuilder::<T>::new(&input).build()
    }
}

impl<T> CompletionClient for Client<T>
where
    T: HttpClientExt + Clone + std::fmt::Debug + Default + Send + 'static,
{
    type CompletionModel = CompletionModel<T>;

    fn completion_model(&self, model: impl Into<String>) -> Self::CompletionModel {
        CompletionModel::new(self.clone(), &model.into())
    }
}

// ================================================================
// API Response Types
// ================================================================

#[derive(Debug, Deserialize)]
struct ApiErrorResponse {
    message: String,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum ApiResponse<T> {
    Ok(T),
    Err(ApiErrorResponse),
}

// ================================================================
// Completion Types
// ================================================================

#[derive(Debug, Deserialize, Serialize)]
pub struct CompletionResponse {
    pub id: String,
    pub model: String,
    pub object: String,
    pub created: u64,
    #[serde(default)]
    pub choices: Vec<Choice>,
    pub usage: Usage,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Message {
    pub role: Role,
    #[serde(default)]
    pub content: Option<String>,
    /// Tool calls made by the assistant (for non-streaming responses)
    #[serde(default)]
    pub tool_calls: Vec<NonStreamingToolCall>,
}

/// Tool call in non-streaming response
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct NonStreamingToolCall {
    pub id: String,
    #[serde(rename = "type")]
    pub call_type: String,
    pub function: NonStreamingFunction,
}

/// Function details in non-streaming tool call
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct NonStreamingFunction {
    pub name: String,
    pub arguments: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    System,
    User,
    Assistant,
}

#[derive(Deserialize, Debug, Serialize)]
pub struct Delta {
    #[serde(default)]
    pub role: Option<Role>,
    #[serde(default)]
    pub content: Option<String>,
}

#[derive(Deserialize, Debug, Serialize)]
pub struct Choice {
    pub index: usize,
    #[serde(default)]
    pub finish_reason: Option<String>,
    #[serde(default)]
    pub message: Option<Message>,
    #[serde(default)]
    pub delta: Option<Delta>,
}

#[derive(Deserialize, Debug, Serialize, Clone)]
pub struct Usage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

impl std::fmt::Display for Usage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Prompt tokens: {}\nCompletion tokens: {} Total tokens: {}",
            self.prompt_tokens, self.completion_tokens, self.total_tokens
        )
    }
}

impl Usage {
    fn new() -> Self {
        Self {
            prompt_tokens: 0,
            completion_tokens: 0,
            total_tokens: 0,
        }
    }
}

// ================================================================
// Z.AI Streaming Types (with reasoning_content support)
// ================================================================

#[derive(Debug, Serialize, Deserialize, Clone)]
struct StreamingFunction {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    arguments: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct StreamingToolCall {
    index: usize,
    id: Option<String>,
    function: StreamingFunction,
}

/// Z.AI streaming delta with reasoning_content support
#[derive(Deserialize, Debug)]
struct StreamingDelta {
    #[serde(default)]
    content: Option<String>,
    /// Z.AI thinking/reasoning content (GLM-4.7 thinking mode)
    #[serde(default)]
    reasoning_content: Option<String>,
    #[serde(default)]
    tool_calls: Vec<StreamingToolCall>,
}

#[derive(Deserialize, Debug)]
struct StreamingChoice {
    delta: StreamingDelta,
    /// Finish reason (e.g., "stop", "tool_calls") - needed for deserialization
    #[serde(default)]
    #[allow(dead_code)]
    finish_reason: Option<String>,
}

#[derive(Deserialize, Debug)]
struct StreamingCompletionChunk {
    choices: Vec<StreamingChoice>,
    usage: Option<Usage>,
}

/// Z.AI streaming response with thinking support
#[derive(Clone, Serialize, Deserialize)]
pub struct ZaiStreamingResponse {
    pub usage: Usage,
}

impl completion::GetTokenUsage for ZaiStreamingResponse {
    fn token_usage(&self) -> Option<completion::Usage> {
        let mut usage = completion::Usage::new();
        usage.input_tokens = self.usage.prompt_tokens as u64;
        usage.output_tokens = self.usage.completion_tokens as u64;
        usage.total_tokens = self.usage.total_tokens as u64;
        Some(usage)
    }
}

/// Attempt to fix malformed JSON from Z.AI tool call arguments.
///
/// Z.AI sometimes returns JSON with unquoted string values like:
/// `{"path":.,"pattern":*}` instead of `{"path":".","pattern":"*"}`
///
/// This function tries to fix common issues by quoting unquoted values.
/// It tracks whether we're inside a string to avoid corrupting quoted content.
fn fix_malformed_json(s: &str) -> String {
    let mut result = String::with_capacity(s.len() * 2);
    let chars: Vec<char> = s.chars().collect();
    let len = chars.len();
    let mut i = 0;
    let mut in_string = false;
    let mut escape_next = false;

    while i < len {
        let c = chars[i];

        // Track string state
        if escape_next {
            escape_next = false;
            result.push(c);
            i += 1;
            continue;
        }

        if c == '\\' && in_string {
            escape_next = true;
            result.push(c);
            i += 1;
            continue;
        }

        if c == '"' {
            in_string = !in_string;
            result.push(c);
            i += 1;
            continue;
        }

        // If we're inside a string, just copy the character
        if in_string {
            result.push(c);
            i += 1;
            continue;
        }

        result.push(c);

        // After a colon (outside of strings), check if the value is unquoted
        if c == ':' {
            // Skip whitespace
            while i + 1 < len && chars[i + 1].is_whitespace() {
                i += 1;
                result.push(chars[i]);
            }

            if i + 1 < len {
                let next = chars[i + 1];
                // Check if next char starts a valid JSON value
                // Valid: " (string), { (object), [ (array), digit, -, t, f, n (true/false/null)
                let is_valid_start = next == '"'
                    || next == '{'
                    || next == '['
                    || next.is_ascii_digit()
                    || next == '-'
                    || next == 't'
                    || next == 'f'
                    || next == 'n';

                if !is_valid_start {
                    // This is an unquoted value - find where it ends
                    // It ends at the next `,"` or `,"key":` pattern or `}` or `]`
                    let start = i + 1;
                    let mut end = start;
                    let mut depth = 0;

                    while end < len {
                        let ec = chars[end];
                        if ec == '[' || ec == '{' {
                            depth += 1;
                        } else if ec == ']' || ec == '}' {
                            if depth == 0 {
                                break;
                            }
                            depth -= 1;
                        } else if ec == ',' && depth == 0 {
                            // Check if this looks like a key separator (followed by "key":)
                            // by looking ahead for a quote
                            let mut peek = end + 1;
                            while peek < len && chars[peek].is_whitespace() {
                                peek += 1;
                            }
                            if peek < len && chars[peek] == '"' {
                                // This comma is likely a separator, not part of the value
                                break;
                            }
                            // Otherwise, comma is part of the unquoted string value
                        }
                        end += 1;
                    }

                    // Extract the unquoted value
                    let value: String = chars[start..end].iter().collect();
                    let value = value.trim();

                    // Check if it's a JSON literal (boolean, null, or number)
                    let is_json_literal = value == "true"
                        || value == "false"
                        || value == "null"
                        || value.parse::<f64>().is_ok();

                    if is_json_literal {
                        result.push_str(value);
                    } else {
                        // Quote it as a string
                        let escaped = value.replace('\\', "\\\\").replace('"', "\\\"");
                        result.push('"');
                        result.push_str(&escaped);
                        result.push('"');
                    }
                    i = end - 1; // -1 because loop will increment
                }
            }
        }
        i += 1;
    }

    result
}

/// Send a Z.AI streaming request with reasoning_content support.
///
/// This is similar to OpenAI's send_compatible_streaming_request but handles
/// Z.AI's `reasoning_content` field for thinking mode.
async fn send_zai_streaming_request<T>(
    http_client: T,
    req: http::Request<Vec<u8>>,
) -> Result<streaming::StreamingCompletionResponse<ZaiStreamingResponse>, CompletionError>
where
    T: HttpClientExt + Clone + 'static,
{
    let mut event_source = GenericEventSource::new(http_client, req);

    let stream = stream! {
        let span = tracing::Span::current();
        let mut final_usage = Usage::new();

        // Track in-progress tool calls
        let mut tool_calls: HashMap<usize, (String, String, String)> = HashMap::new();

        let mut text_content = String::new();
        let mut reasoning_content = String::new();

        while let Some(event_result) = event_source.next().await {
            match event_result {
                Ok(Event::Open) => {
                    tracing::trace!("Z.AI SSE connection opened");
                    continue;
                }
                Ok(Event::Message(message)) => {
                    if message.data.trim().is_empty() || message.data == "[DONE]" {
                        continue;
                    }

                    // Trace log raw SSE data to see what Z.AI is sending
                    tracing::trace!(target: "rig_zai::streaming", "Raw SSE data: {}", message.data);

                    // Log raw SSE chunk to file (if API logging is enabled)
                    qbit_api_logger::API_LOGGER.log_sse_chunk("zai", &message.data);

                    let data = serde_json::from_str::<StreamingCompletionChunk>(&message.data);
                    let Ok(data) = data else {
                        let err = data.unwrap_err();
                        tracing::debug!("Couldn't parse Z.AI streaming chunk: {:?}", err);
                        continue;
                    };

                    if let Some(choice) = data.choices.first() {
                        let delta = &choice.delta;

                        // Handle reasoning/thinking content (Z.AI specific)
                        if let Some(reasoning) = &delta.reasoning_content {
                            if !reasoning.is_empty() {
                                reasoning_content += reasoning;
                                yield Ok(RawStreamingChoice::Reasoning {
                                    id: Some("zai-reasoning".to_string()),
                                    reasoning: reasoning.clone(),
                                    signature: None,
                                });
                            }
                        }

                        // Handle tool calls
                        if !delta.tool_calls.is_empty() {
                            for tool_call in &delta.tool_calls {
                                let function = tool_call.function.clone();

                                // Tool call with name = start or update tracking
                                if let Some(ref name) = function.name {
                                    if !name.is_empty() {
                                        let id = tool_call.id.clone().unwrap_or_default();
                                        // Start tracking with any initial arguments
                                        tool_calls.insert(
                                            tool_call.index,
                                            (id, name.clone(), function.arguments.clone()),
                                        );
                                        continue;
                                    }
                                }

                                // Tool call without name = continuation (accumulate arguments)
                                if !function.arguments.is_empty() {
                                    if let Some((id, name, arguments)) =
                                        tool_calls.get(&tool_call.index)
                                    {
                                        let new_arguments = &function.arguments;
                                        let arguments = format!("{arguments}{new_arguments}");
                                        tool_calls.insert(
                                            tool_call.index,
                                            (id.clone(), name.clone(), arguments),
                                        );
                                    } else {
                                        tracing::debug!("Partial tool call received but tool call was never started.");
                                    }
                                }
                            }
                        }

                        // Handle message content
                        if let Some(content) = &delta.content {
                            text_content += content;
                            yield Ok(RawStreamingChoice::Message(content.clone()));
                        }
                    }

                    // Usage updates
                    if let Some(usage) = data.usage {
                        final_usage = usage.clone();
                    }
                }
                Err(http_client::Error::StreamEnded) => {
                    break;
                }
                Err(error) => {
                    tracing::error!(?error, "Z.AI SSE error");
                    yield Err(CompletionError::ResponseError(error.to_string()));
                    break;
                }
            }
        }

        // Close event source
        event_source.close();

        // Capture tool calls count before consuming the HashMap
        let tool_calls_count = tool_calls.len();

        // Flush any tool calls that weren't fully yielded
        for (_idx, (id, name, arguments)) in tool_calls {
            // Try parsing directly first, then try fixing malformed JSON
            let parsed_args = match serde_json::from_str::<serde_json::Value>(&arguments) {
                Ok(args) => args,
                Err(original_err) => {
                    tracing::debug!(
                        target: "rig_zai::streaming",
                        "Original JSON parse failed for tool '{}': {} - Raw: {}",
                        name,
                        original_err,
                        &arguments[..arguments.len().min(200)]
                    );
                    // Try fixing malformed JSON (Z.AI sometimes returns unquoted strings)
                    let fixed = fix_malformed_json(&arguments);
                    match serde_json::from_str::<serde_json::Value>(&fixed) {
                        Ok(args) => {
                            tracing::debug!(
                                target: "rig_zai::streaming",
                                "Fixed malformed JSON in tool call arguments"
                            );
                            args
                        }
                        Err(e) => {
                            tracing::warn!(
                                target: "rig_zai::streaming",
                                "Failed to parse tool call arguments after fix: {} - Fixed: {}",
                                e,
                                &fixed[..fixed.len().min(200)]
                            );
                            continue;
                        }
                    }
                }
            };

            yield Ok(RawStreamingChoice::ToolCall(RawStreamingToolCall {
                id,
                call_id: None,
                name,
                arguments: parsed_args,
                signature: None,
                additional_params: None,
            }));
        }

        // Log summary
        tracing::info!(
            target: "rig_zai::streaming",
            "Z.AI stream complete: {} chars text, {} chars reasoning",
            text_content.len(),
            reasoning_content.len()
        );

        // Log stream end with response summary to file (if API logging is enabled)
        qbit_api_logger::API_LOGGER.log_response(
            "zai",
            "glm",
            &serde_json::json!({
                "text_content_len": text_content.len(),
                "reasoning_content_len": reasoning_content.len(),
                "tool_calls_count": tool_calls_count,
                "usage": {
                    "prompt_tokens": final_usage.prompt_tokens,
                    "completion_tokens": final_usage.completion_tokens,
                }
            }),
        );
        qbit_api_logger::API_LOGGER.log_stream_end("zai", "normal_completion");

        span.record("gen_ai.usage.input_tokens", final_usage.prompt_tokens);
        span.record("gen_ai.usage.output_tokens", final_usage.completion_tokens);

        yield Ok(RawStreamingChoice::FinalResponse(ZaiStreamingResponse {
            usage: final_usage,
        }));
    };

    Ok(streaming::StreamingCompletionResponse::stream(Box::pin(
        stream,
    )))
}

impl TryFrom<CompletionResponse> for completion::CompletionResponse<CompletionResponse> {
    type Error = CompletionError;

    fn try_from(response: CompletionResponse) -> Result<Self, Self::Error> {
        use rig::message::{AssistantContent, Text, ToolCall, ToolFunction};

        let choice = response.choices.first().ok_or_else(|| {
            CompletionError::ResponseError("Response contained no choices".to_owned())
        })?;

        let message = choice.message.as_ref().ok_or_else(|| {
            CompletionError::ResponseError("Response contained no message".to_owned())
        })?;

        if message.role != Role::Assistant {
            return Err(CompletionError::ResponseError(
                "Response contained no assistant message".to_owned(),
            ));
        }

        // Build content from text and tool calls
        let mut contents: Vec<AssistantContent> = Vec::new();

        // Add text content if present
        if let Some(ref text) = message.content {
            if !text.is_empty() {
                contents.push(AssistantContent::Text(Text { text: text.clone() }));
            }
        }

        // Add tool calls if present
        for tc in &message.tool_calls {
            let arguments: serde_json::Value = serde_json::from_str(&tc.function.arguments)
                .unwrap_or(serde_json::Value::Object(serde_json::Map::new()));

            contents.push(AssistantContent::ToolCall(ToolCall {
                id: tc.id.clone(),
                call_id: Some(tc.id.clone()),
                function: ToolFunction {
                    name: tc.function.name.clone(),
                    arguments,
                },
                signature: None,
                additional_params: None,
            }));
        }

        // If no content at all, add empty text
        if contents.is_empty() {
            contents.push(AssistantContent::Text(Text {
                text: String::new(),
            }));
        }

        let choice = if contents.len() == 1 {
            OneOrMany::one(contents.remove(0))
        } else {
            OneOrMany::many(contents).unwrap_or_else(|_| {
                OneOrMany::one(AssistantContent::Text(Text {
                    text: String::new(),
                }))
            })
        };

        Ok(completion::CompletionResponse {
            choice,
            usage: completion::Usage {
                input_tokens: response.usage.prompt_tokens as u64,
                output_tokens: response.usage.completion_tokens as u64,
                total_tokens: response.usage.total_tokens as u64,
            },
            raw_response: response,
        })
    }
}

// ================================================================
// Completion Model
// ================================================================

#[derive(Clone)]
pub struct CompletionModel<T> {
    client: Client<T>,
    pub model: String,
}

impl<T> CompletionModel<T> {
    pub fn new(client: Client<T>, model: &str) -> Self {
        Self {
            client,
            model: model.to_string(),
        }
    }

    fn create_completion_request(
        &self,
        completion_request: completion::CompletionRequest,
    ) -> Result<Value, CompletionError> {
        // Build up the order of messages (context, chat_history, prompt)
        let mut partial_history = vec![];
        if let Some(docs) = completion_request.normalized_documents() {
            partial_history.push(docs);
        }
        partial_history.extend(completion_request.chat_history);

        // Initialize full history with preamble (or empty if non-existent)
        let mut full_history: Vec<Value> =
            completion_request
                .preamble
                .map_or_else(Vec::new, |preamble| {
                    vec![json!({
                        "role": "system",
                        "content": preamble,
                    })]
                });

        // Convert messages to OpenAI-compatible JSON format
        for msg in partial_history {
            match msg {
                message::Message::User { content } => {
                    // Check if this is a tool result
                    let mut tool_results = vec![];
                    let mut text_parts = vec![];

                    for c in content.into_iter() {
                        match c {
                            message::UserContent::Text(message::Text { text }) => {
                                text_parts.push(text);
                            }
                            message::UserContent::ToolResult(result) => {
                                // Extract text from tool result content
                                let result_text = result
                                    .content
                                    .into_iter()
                                    .filter_map(|c| match c {
                                        message::ToolResultContent::Text(message::Text {
                                            text,
                                        }) => Some(text),
                                        _ => None,
                                    })
                                    .collect::<Vec<_>>()
                                    .join("\n");
                                tool_results
                                    .push((result.call_id.unwrap_or(result.id), result_text));
                            }
                            _ => {} // Skip other content types
                        }
                    }

                    // Add tool result messages first (OpenAI format)
                    for (tool_call_id, content) in tool_results {
                        full_history.push(json!({
                            "role": "tool",
                            "tool_call_id": tool_call_id,
                            "content": content,
                        }));
                    }

                    // Add regular user text if present
                    if !text_parts.is_empty() {
                        full_history.push(json!({
                            "role": "user",
                            "content": text_parts.join("\n"),
                        }));
                    }
                }
                message::Message::Assistant { content, .. } => {
                    let mut text_parts = vec![];
                    let mut tool_calls = vec![];
                    let mut reasoning_parts = vec![];

                    for c in content.into_iter() {
                        match c {
                            message::AssistantContent::Text(message::Text { text }) => {
                                text_parts.push(text);
                            }
                            message::AssistantContent::ToolCall(tc) => {
                                tool_calls.push(json!({
                                    "id": tc.call_id.unwrap_or(tc.id),
                                    "type": "function",
                                    "function": {
                                        "name": tc.function.name,
                                        "arguments": serde_json::to_string(&tc.function.arguments).unwrap_or_default(),
                                    }
                                }));
                            }
                            message::AssistantContent::Reasoning(reasoning) => {
                                // Collect reasoning content for preserved thinking
                                // Z.AI requires reasoning_content to be passed back unmodified
                                reasoning_parts.extend(reasoning.reasoning);
                            }
                            _ => {} // Skip other content types (e.g., Image)
                        }
                    }

                    // Build assistant message
                    let mut assistant_msg = json!({
                        "role": "assistant",
                        "content": if text_parts.is_empty() { Value::Null } else { json!(text_parts.join("\n")) },
                    });

                    if !tool_calls.is_empty() {
                        assistant_msg["tool_calls"] = json!(tool_calls);
                    }

                    // Include reasoning_content for preserved thinking mode
                    // This is required by Z.AI to maintain reasoning continuity across turns
                    if !reasoning_parts.is_empty() {
                        assistant_msg["reasoning_content"] = json!(reasoning_parts.join(""));
                    }

                    full_history.push(assistant_msg);
                }
            }
        }

        // Compose request with thinking mode enabled for Z.AI models
        // Z.AI thinking mode allows the model to reason before responding
        // See: https://docs.z.ai/guides/capabilities/thinking-mode
        let mut request = json!({
            "model": self.model,
            "messages": full_history,
            "temperature": completion_request.temperature,
        });

        // Enable thinking mode for GLM-4.7
        // - Thinking is on by default for GLM-4.7, but we're explicit here
        // - clear_thinking: false enables "Preserved Thinking" - reasoning is kept in context
        // - Preserved Thinking is enabled by default on Coding Plan endpoint but being explicit
        // - reasoning_content must be returned in assistant messages for multi-turn continuity
        // Note: GLM-4.5 supports interleaved thinking but explicit config may not be needed
        if self.model == GLM_4_7 {
            request = merge_json(
                request,
                json!({
                    "thinking": {
                        "type": "enabled",
                        "clear_thinking": false
                    }
                }),
            );
        }

        // Add tools in OpenAI-compatible format
        if !completion_request.tools.is_empty() {
            let tools: Vec<Value> = completion_request
                .tools
                .iter()
                .map(|tool| {
                    json!({
                        "type": "function",
                        "function": {
                            "name": tool.name,
                            "description": tool.description,
                            "parameters": tool.parameters
                        }
                    })
                })
                .collect();
            request = merge_json(request, json!({ "tools": tools }));
        }

        // Add tool choice (default to auto when tools are present)
        if let Some(tool_choice) =
            completion_request
                .tool_choice
                .clone()
                .or(if completion_request.tools.is_empty() {
                    None
                } else {
                    Some(ToolChoice::Auto)
                })
        {
            let tool_choice_value = match tool_choice {
                ToolChoice::Auto => json!("auto"),
                ToolChoice::None => json!("none"),
                ToolChoice::Required => json!("required"),
                ToolChoice::Specific { function_names } => {
                    if let Some(name) = function_names.first() {
                        json!({"type": "function", "function": {"name": name}})
                    } else {
                        json!("auto")
                    }
                }
            };
            request = merge_json(request, json!({ "tool_choice": tool_choice_value }));
        }

        let request = if let Some(ref params) = completion_request.additional_params {
            merge_json(request, params.clone())
        } else {
            request
        };

        Ok(request)
    }
}

// ================================================================
// Message Conversions
// ================================================================

impl TryFrom<message::Message> for Message {
    type Error = MessageError;

    fn try_from(message: message::Message) -> Result<Self, Self::Error> {
        Ok(match message {
            message::Message::User { content } => {
                let collapsed_content = content
                    .into_iter()
                    .map(|content| match content {
                        message::UserContent::Text(message::Text { text }) => Ok(text),
                        _ => Err(MessageError::ConversionError(
                            "Only text content is supported by Z.AI".to_owned(),
                        )),
                    })
                    .collect::<Result<Vec<_>, _>>()?
                    .join("\n");

                Message {
                    role: Role::User,
                    content: Some(collapsed_content),
                    tool_calls: vec![],
                }
            }

            message::Message::Assistant { content, .. } => {
                let collapsed_content = content
                    .into_iter()
                    .map(|content| {
                        Ok(match content {
                            message::AssistantContent::Text(message::Text { text }) => text,
                            _ => {
                                return Err(MessageError::ConversionError(
                                    "Only text assistant message content is supported by Z.AI"
                                        .to_owned(),
                                ))
                            }
                        })
                    })
                    .collect::<Result<Vec<_>, _>>()?
                    .join("\n");

                Message {
                    role: Role::Assistant,
                    content: Some(collapsed_content),
                    tool_calls: vec![],
                }
            }
        })
    }
}

impl From<Message> for message::Message {
    fn from(message: Message) -> Self {
        let content = message.content.unwrap_or_default();
        match message.role {
            Role::User => message::Message::user(content),
            Role::Assistant => message::Message::assistant(content),
            // System messages get coerced into user messages for ease of error handling.
            // They should be handled on the outside of `Message` conversions via the preamble.
            Role::System => message::Message::user(content),
        }
    }
}

// ================================================================
// CompletionModel trait implementation
// ================================================================

impl<T> completion::CompletionModel for CompletionModel<T>
where
    T: HttpClientExt + Clone + Default + std::fmt::Debug + Send + 'static,
{
    type Response = CompletionResponse;
    type StreamingResponse = ZaiStreamingResponse;
    type Client = Client<T>;

    fn make(client: &Self::Client, model: impl Into<String>) -> Self {
        Self::new(client.clone(), &model.into())
    }

    async fn completion(
        &self,
        completion_request: completion::CompletionRequest,
    ) -> Result<completion::CompletionResponse<CompletionResponse>, CompletionError> {
        let preamble = completion_request.preamble.clone();
        let request = self.create_completion_request(completion_request)?;

        let span = if tracing::Span::current().is_disabled() {
            info_span!(
                target: "rig::completions",
                "chat",
                gen_ai.operation.name = "chat",
                gen_ai.provider.name = "zai",
                gen_ai.request.model = self.model,
                gen_ai.system_instructions = preamble,
                gen_ai.response.id = tracing::field::Empty,
                gen_ai.response.model = tracing::field::Empty,
                gen_ai.usage.output_tokens = tracing::field::Empty,
                gen_ai.usage.input_tokens = tracing::field::Empty,
                gen_ai.input.messages = serde_json::to_string(&request.get("messages").unwrap()).unwrap(),
                gen_ai.output.messages = tracing::field::Empty,
            )
        } else {
            tracing::Span::current()
        };

        let body = serde_json::to_vec(&request)?;

        let req = self
            .client
            .req(Method::POST, "/chat/completions")?
            .header("Content-Type", "application/json")
            .body(body)
            .map_err(http_client::Error::from)?;

        let async_block = async move {
            let response = self.client.http_client.send::<_, Bytes>(req).await?;

            let status = response.status();
            let response_body = response.into_body().await?.to_vec();

            if status.is_success() {
                match serde_json::from_slice::<ApiResponse<CompletionResponse>>(&response_body)? {
                    ApiResponse::Ok(completion) => {
                        let span = tracing::Span::current();
                        span.record("gen_ai.usage.input_tokens", completion.usage.prompt_tokens);
                        span.record(
                            "gen_ai.usage.output_tokens",
                            completion.usage.completion_tokens,
                        );
                        span.record(
                            "gen_ai.output.messages",
                            serde_json::to_string(&completion.choices).unwrap(),
                        );
                        span.record("gen_ai.response.id", completion.id.to_string());
                        span.record("gen_ai.response.model_name", completion.model.to_string());
                        Ok(completion.try_into()?)
                    }
                    ApiResponse::Err(error) => Err(CompletionError::ProviderError(error.message)),
                }
            } else {
                Err(CompletionError::ProviderError(
                    String::from_utf8_lossy(&response_body).to_string(),
                ))
            }
        };

        async_block.instrument(span).await
    }

    async fn stream(
        &self,
        completion_request: completion::CompletionRequest,
    ) -> Result<StreamingCompletionResponse<Self::StreamingResponse>, CompletionError> {
        let preamble = completion_request.preamble.clone();
        let mut request = self.create_completion_request(completion_request)?;

        request = merge_json(
            request,
            json!({
                "stream": true,
                "tool_stream": true
            }),
        );

        // Debug log the full request to verify tools are included
        tracing::warn!(
            "Z.AI request tools count: {}, full request: {}",
            request
                .get("tools")
                .map(|t| t.as_array().map(|a| a.len()).unwrap_or(0))
                .unwrap_or(0),
            serde_json::to_string_pretty(&request).unwrap_or_default()
        );

        // Log raw request JSON to file (if API logging is enabled)
        qbit_api_logger::API_LOGGER.log_request("zai", &self.model, &request);

        let body = serde_json::to_vec(&request)?;

        let req = self
            .client
            .req(Method::POST, "/chat/completions")?
            .header("Content-Type", "application/json")
            .body(body)
            .map_err(http_client::Error::from)?;

        let span = if tracing::Span::current().is_disabled() {
            info_span!(
                target: "rig::completions",
                "chat_streaming",
                gen_ai.operation.name = "chat_streaming",
                gen_ai.provider.name = "zai",
                gen_ai.request.model = self.model,
                gen_ai.system_instructions = preamble,
                gen_ai.response.id = tracing::field::Empty,
                gen_ai.response.model = tracing::field::Empty,
                gen_ai.usage.output_tokens = tracing::field::Empty,
                gen_ai.usage.input_tokens = tracing::field::Empty,
                gen_ai.input.messages = serde_json::to_string(&request.get("messages").unwrap()).unwrap(),
                gen_ai.output.messages = tracing::field::Empty,
            )
        } else {
            tracing::Span::current()
        };
        send_zai_streaming_request(self.client.http_client.clone(), req)
            .instrument(span)
            .await
    }
}

// ================================================================
// Tests
// ================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_message() {
        let json_data = r#"
        {
            "role": "user",
            "content": "Hello, how can I help you?"
        }
        "#;

        let message: Message = serde_json::from_str(json_data).unwrap();
        assert_eq!(message.role, Role::User);
        assert_eq!(
            message.content,
            Some("Hello, how can I help you?".to_string())
        );
    }

    #[test]
    fn test_serialize_message() {
        let message = Message {
            role: Role::Assistant,
            content: Some("I am here to assist you.".to_string()),
            tool_calls: vec![],
        };

        let json_data = serde_json::to_string(&message).unwrap();
        assert!(json_data.contains(r#""role":"assistant""#));
        assert!(json_data.contains(r#""content":"I am here to assist you.""#));
    }

    #[test]
    fn test_message_to_message_conversion() {
        let user_message = message::Message::user("User message");
        let assistant_message = message::Message::assistant("Assistant message");

        let converted_user_message: Message = user_message.clone().try_into().unwrap();
        let converted_assistant_message: Message = assistant_message.clone().try_into().unwrap();

        assert_eq!(converted_user_message.role, Role::User);
        assert_eq!(
            converted_user_message.content,
            Some("User message".to_string())
        );

        assert_eq!(converted_assistant_message.role, Role::Assistant);
        assert_eq!(
            converted_assistant_message.content,
            Some("Assistant message".to_string())
        );

        let back_to_user_message: message::Message = converted_user_message.into();
        let back_to_assistant_message: message::Message = converted_assistant_message.into();

        assert_eq!(user_message, back_to_user_message);
        assert_eq!(assistant_message, back_to_assistant_message);
    }

    #[test]
    fn test_model_constants() {
        assert_eq!(GLM_4_7, "GLM-4.7");
        assert_eq!(GLM_4_5_AIR, "GLM-4.5-air");
    }

    #[test]
    fn test_client_builder() {
        let client = Client::builder("test-api-key").build();
        assert_eq!(client.base_url, ZAI_CODING_API_BASE_URL);

        let custom_url = "https://custom.endpoint.com";
        let client_custom = Client::builder("test-api-key").base_url(custom_url).build();
        assert_eq!(client_custom.base_url, custom_url);
    }

    #[test]
    fn test_merge_json() {
        let a = json!({"key1": "value1", "key2": "value2"});
        let b = json!({"key2": "new_value2", "key3": "value3"});
        let merged = merge_json(a, b);
        assert_eq!(merged["key1"], "value1");
        assert_eq!(merged["key2"], "new_value2");
        assert_eq!(merged["key3"], "value3");
    }

    #[test]
    fn test_tool_choice_defaults_to_auto_when_tools_present() {
        let model = CompletionModel::new(Client::builder("test-api-key").build(), GLM_4_7);

        let completion_request = completion::CompletionRequest {
            preamble: None,
            chat_history: OneOrMany::one(message::Message::user("Hello")),
            documents: vec![],
            tools: vec![completion::ToolDefinition {
                name: "my_tool".to_string(),
                description: "test tool".to_string(),
                parameters: json!({}),
            }],
            temperature: None,
            max_tokens: None,
            tool_choice: None,
            additional_params: None,
        };

        let request_value = model
            .create_completion_request(completion_request)
            .expect("request should serialize");

        let expected_auto: ToolChoice = serde_json::from_str("\"auto\"").unwrap();

        assert_eq!(
            request_value.get("tool_choice"),
            Some(&serde_json::to_value(expected_auto).unwrap())
        );
    }

    #[test]
    fn test_tool_choice_omitted_when_no_tools() {
        let model = CompletionModel::new(Client::builder("test-api-key").build(), GLM_4_7);

        let completion_request = completion::CompletionRequest {
            preamble: None,
            chat_history: OneOrMany::one(message::Message::user("Hello")),
            documents: vec![],
            tools: vec![],
            temperature: None,
            max_tokens: None,
            tool_choice: None,
            additional_params: None,
        };

        let request_value = model
            .create_completion_request(completion_request)
            .expect("request should serialize");

        assert!(request_value.get("tool_choice").is_none());
    }

    #[test]
    fn test_specific_tool_choice_serialization() {
        let model = CompletionModel::new(Client::builder("test-api-key").build(), GLM_4_7);

        let tool_choice = ToolChoice::Specific {
            function_names: vec!["my_tool".to_string()],
        };

        let completion_request = completion::CompletionRequest {
            preamble: None,
            chat_history: OneOrMany::one(message::Message::user("Hello")),
            documents: vec![],
            tools: vec![completion::ToolDefinition {
                name: "my_tool".to_string(),
                description: "test tool".to_string(),
                parameters: json!({}),
            }],
            temperature: None,
            max_tokens: None,
            tool_choice: Some(tool_choice.clone()),
            additional_params: None,
        };

        let request_value = model
            .create_completion_request(completion_request)
            .expect("request should serialize");

        assert_eq!(
            request_value.get("tool_choice"),
            Some(&json!({"type": "function", "function": {"name": "my_tool"}}))
        );
    }
}
