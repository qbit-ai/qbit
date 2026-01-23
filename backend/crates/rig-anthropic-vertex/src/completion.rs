//! CompletionModel implementation for Anthropic on Vertex AI.

use rig::completion::{
    self, AssistantContent, CompletionError, CompletionRequest, CompletionResponse, Message,
    ToolDefinition, Usage,
};
use rig::one_or_many::OneOrMany;
use rig::streaming::{RawStreamingChoice, RawStreamingToolCall, StreamingCompletionResponse};
use serde::{Deserialize, Serialize};

use crate::client::Client;
use crate::streaming::StreamingResponse;
use crate::types::{
    self, CacheControl, CitationsConfig, ContentBlock, ImageSource, Role, ServerTool, SystemBlock,
    ThinkingConfig, ToolEntry, ANTHROPIC_VERSION, DEFAULT_MAX_TOKENS,
};

/// Beta header for web fetch feature
const WEB_FETCH_BETA: &str = "web-fetch-2025-09-10";

/// Configuration for native web search
#[derive(Debug, Clone)]
pub struct WebSearchConfig {
    /// Maximum number of searches per request
    pub max_uses: Option<u32>,
    /// Only include results from these domains
    pub allowed_domains: Option<Vec<String>>,
    /// Never include results from these domains
    pub blocked_domains: Option<Vec<String>>,
}

impl Default for WebSearchConfig {
    fn default() -> Self {
        Self {
            max_uses: Some(5),
            allowed_domains: None,
            blocked_domains: None,
        }
    }
}

/// Configuration for native web fetch
#[derive(Debug, Clone)]
pub struct WebFetchConfig {
    /// Maximum number of fetches per request
    pub max_uses: Option<u32>,
    /// Enable citations for fetched content
    pub citations_enabled: bool,
    /// Maximum content length in tokens
    pub max_content_tokens: Option<u32>,
}

impl Default for WebFetchConfig {
    fn default() -> Self {
        Self {
            max_uses: Some(10),
            citations_enabled: true,
            max_content_tokens: Some(100000),
        }
    }
}

/// Server tools configuration for Claude's native tools
#[derive(Debug, Clone, Default)]
pub struct ServerToolsConfig {
    /// Native web search configuration
    pub web_search: Option<WebSearchConfig>,
    /// Native web fetch configuration
    pub web_fetch: Option<WebFetchConfig>,
}

/// Default max tokens for different Claude models
fn default_max_tokens_for_model(model: &str) -> u32 {
    if model.contains("opus") {
        32000
    } else if model.contains("sonnet") || model.contains("haiku") {
        8192
    } else {
        DEFAULT_MAX_TOKENS
    }
}

/// Completion model for Anthropic Claude on Vertex AI.
#[derive(Clone)]
pub struct CompletionModel {
    client: Client,
    model: String,
    /// Optional thinking configuration for extended reasoning
    thinking: Option<ThinkingConfig>,
    /// Optional server tools configuration for native web tools
    server_tools: Option<ServerToolsConfig>,
}

impl CompletionModel {
    /// Create a new completion model.
    pub fn new(client: Client, model: String) -> Self {
        Self {
            client,
            model,
            thinking: None,
            server_tools: None,
        }
    }

    /// Enable extended thinking with the specified token budget.
    /// Note: When thinking is enabled, temperature is automatically set to 1.
    pub fn with_thinking(mut self, budget_tokens: u32) -> Self {
        self.thinking = Some(ThinkingConfig::new(budget_tokens));
        self
    }

    /// Enable extended thinking with default budget (10,000 tokens).
    pub fn with_default_thinking(mut self) -> Self {
        self.thinking = Some(ThinkingConfig::default_budget());
        self
    }

    /// Enable Claude's native web search tool with default configuration.
    pub fn with_web_search(mut self) -> Self {
        let config = self
            .server_tools
            .get_or_insert_with(ServerToolsConfig::default);
        config.web_search = Some(WebSearchConfig::default());
        self
    }

    /// Enable Claude's native web search tool with custom configuration.
    pub fn with_web_search_config(mut self, config: WebSearchConfig) -> Self {
        let server_config = self
            .server_tools
            .get_or_insert_with(ServerToolsConfig::default);
        server_config.web_search = Some(config);
        self
    }

    /// Enable Claude's native web fetch tool with default configuration.
    pub fn with_web_fetch(mut self) -> Self {
        let config = self
            .server_tools
            .get_or_insert_with(ServerToolsConfig::default);
        config.web_fetch = Some(WebFetchConfig::default());
        self
    }

    /// Enable Claude's native web fetch tool with custom configuration.
    pub fn with_web_fetch_config(mut self, config: WebFetchConfig) -> Self {
        let server_config = self
            .server_tools
            .get_or_insert_with(ServerToolsConfig::default);
        server_config.web_fetch = Some(config);
        self
    }

    /// Check if server tools are enabled (requires beta header)
    fn needs_web_fetch_beta(&self) -> bool {
        self.server_tools
            .as_ref()
            .map(|c| c.web_fetch.is_some())
            .unwrap_or(false)
    }

    /// Build server tools entries for the API request
    fn build_server_tools(&self) -> Vec<ToolEntry> {
        let mut tools = Vec::new();

        if let Some(ref config) = self.server_tools {
            if let Some(ref search) = config.web_search {
                tools.push(ToolEntry::Server(ServerTool::WebSearch {
                    name: "web_search".to_string(),
                    max_uses: search.max_uses,
                    allowed_domains: search.allowed_domains.clone(),
                    blocked_domains: search.blocked_domains.clone(),
                }));
            }

            if let Some(ref fetch) = config.web_fetch {
                tools.push(ToolEntry::Server(ServerTool::WebFetch {
                    name: "web_fetch".to_string(),
                    max_uses: fetch.max_uses,
                    citations: if fetch.citations_enabled {
                        Some(CitationsConfig { enabled: true })
                    } else {
                        None
                    },
                    max_content_tokens: fetch.max_content_tokens,
                }));
            }
        }

        tools
    }

    /// Get the model identifier.
    pub fn model(&self) -> &str {
        &self.model
    }

    /// Convert rig's Message to Anthropic message format.
    fn convert_message(msg: &Message) -> types::Message {
        match msg {
            Message::User { content } => {
                let blocks: Vec<ContentBlock> = content
                    .iter()
                    .filter_map(|c| {
                        use rig::message::UserContent;
                        match c {
                            UserContent::Text(text) => Some(ContentBlock::Text {
                                text: text.text.clone(),
                                cache_control: None,
                            }),
                            UserContent::Image(img) => {
                                // Extract base64 data from rig's Image type
                                use base64::Engine;
                                let data = match &img.data {
                                    rig::message::DocumentSourceKind::Base64(b64) => b64.clone(),
                                    rig::message::DocumentSourceKind::Url(_url) => {
                                        tracing::warn!("Image URLs not yet supported, skipping");
                                        return None;
                                    }
                                    rig::message::DocumentSourceKind::Raw(bytes) => {
                                        base64::engine::general_purpose::STANDARD.encode(bytes)
                                    }
                                    // Handle any future variants added to this non-exhaustive enum
                                    _ => {
                                        tracing::warn!("Unsupported image source kind, skipping");
                                        return None;
                                    }
                                };

                                let media_type = img
                                    .media_type
                                    .as_ref()
                                    .map(|mt| {
                                        use rig::message::ImageMediaType;
                                        match mt {
                                            ImageMediaType::PNG => "image/png",
                                            ImageMediaType::JPEG => "image/jpeg",
                                            ImageMediaType::GIF => "image/gif",
                                            ImageMediaType::WEBP => "image/webp",
                                            ImageMediaType::HEIC => "image/heic",
                                            ImageMediaType::HEIF => "image/heif",
                                            ImageMediaType::SVG => "image/svg+xml",
                                        }
                                        .to_string()
                                    })
                                    .unwrap_or_else(|| "image/png".to_string());

                                Some(ContentBlock::Image {
                                    source: ImageSource {
                                        source_type: "base64".to_string(),
                                        media_type,
                                        data,
                                    },
                                    cache_control: None,
                                })
                            }
                            UserContent::ToolResult(result) => Some(ContentBlock::ToolResult {
                                tool_use_id: result.id.clone(),
                                content: serde_json::to_string(&result.content).unwrap_or_default(),
                                is_error: None,
                                cache_control: None,
                            }),
                            // Skip other content types (Audio, Video, Document) not supported yet
                            _ => None,
                        }
                    })
                    .collect();

                types::Message {
                    role: Role::User,
                    content: if blocks.is_empty() {
                        vec![ContentBlock::Text {
                            text: String::new(),
                            cache_control: None,
                        }]
                    } else {
                        blocks
                    },
                }
            }
            Message::Assistant { content, .. } => {
                // When thinking is enabled, assistant messages must start with thinking blocks
                // Collect thinking blocks first, then other content
                let mut thinking_blocks: Vec<ContentBlock> = Vec::new();
                let mut other_blocks: Vec<ContentBlock> = Vec::new();

                for c in content.iter() {
                    match c {
                        AssistantContent::Text(text) => {
                            other_blocks.push(ContentBlock::Text {
                                text: text.text.clone(),
                                cache_control: None,
                            });
                        }
                        AssistantContent::ToolCall(tool_call) => {
                            // Ensure input is always a valid object (Anthropic API requirement)
                            let input = match &tool_call.function.arguments {
                                serde_json::Value::Object(_) => {
                                    tool_call.function.arguments.clone()
                                }
                                serde_json::Value::Null => serde_json::json!({}),
                                other => serde_json::json!({ "value": other }),
                            };
                            other_blocks.push(ContentBlock::ToolUse {
                                id: tool_call.id.clone(),
                                name: tool_call.function.name.clone(),
                                input,
                            });
                        }
                        AssistantContent::Reasoning(reasoning) => {
                            // Include thinking blocks for extended thinking mode
                            let thinking_text = reasoning.reasoning.join("");
                            if !thinking_text.is_empty() {
                                thinking_blocks.push(ContentBlock::Thinking {
                                    thinking: thinking_text,
                                    // Signature is required but we may not have it from history
                                    // Use empty string as placeholder (API may reject this)
                                    signature: reasoning.signature.clone().unwrap_or_default(),
                                });
                            }
                        }
                        AssistantContent::Image(_) => {
                            // Images in assistant content are not supported by Anthropic API
                            // Skip them silently
                        }
                    }
                }

                // Combine: thinking blocks first (required by API), then other content
                let mut blocks = thinking_blocks;
                blocks.append(&mut other_blocks);

                types::Message {
                    role: Role::Assistant,
                    content: if blocks.is_empty() {
                        vec![ContentBlock::Text {
                            text: String::new(),
                            cache_control: None,
                        }]
                    } else {
                        blocks
                    },
                }
            }
        }
    }

    /// Convert rig's ToolDefinition to Anthropic format as a ToolEntry.
    fn convert_tool(tool: &ToolDefinition) -> ToolEntry {
        ToolEntry::Function(types::ToolDefinition {
            name: tool.name.clone(),
            description: tool.description.clone(),
            input_schema: tool.parameters.clone(),
            cache_control: None,
        })
    }

    /// Build an Anthropic request from a rig CompletionRequest.
    fn build_request(&self, request: &CompletionRequest, stream: bool) -> types::CompletionRequest {
        // Convert chat history to messages
        let mut messages: Vec<types::Message> = request
            .chat_history
            .iter()
            .map(Self::convert_message)
            .collect();

        // Add normalized documents as user messages
        for doc in &request.documents {
            messages.push(types::Message {
                role: Role::User,
                content: vec![ContentBlock::Text {
                    text: format!("[Document: {}]\n{}", doc.id, doc.text),
                    cache_control: None,
                }],
            });
        }

        // Determine max tokens
        let mut max_tokens = request
            .max_tokens
            .map(|t| t as u32)
            .unwrap_or_else(|| default_max_tokens_for_model(&self.model));

        // When thinking is enabled, max_tokens must be greater than budget_tokens
        if let Some(ref thinking) = self.thinking {
            let min_required = thinking.budget_tokens + 1;
            if max_tokens <= thinking.budget_tokens {
                max_tokens = min_required.max(thinking.budget_tokens + 8192);
            }
        }

        // Convert function tools and add server tools
        let mut tool_entries: Vec<ToolEntry> =
            request.tools.iter().map(Self::convert_tool).collect();

        // Add cache_control to the last function tool for caching
        if !tool_entries.is_empty() {
            // Find the last function tool and add cache_control
            for entry in tool_entries.iter_mut().rev() {
                if let ToolEntry::Function(ref mut tool_def) = entry {
                    tool_def.cache_control = Some(CacheControl::ephemeral());
                    break;
                }
            }
        }

        // Add server tools (web_search, web_fetch) if configured
        tool_entries.extend(self.build_server_tools());

        let tools: Option<Vec<ToolEntry>> = if tool_entries.is_empty() {
            None
        } else {
            Some(tool_entries)
        };

        // When thinking is enabled, temperature must be 1
        let temperature = if self.thinking.is_some() {
            Some(1.0)
        } else {
            request.temperature.map(|t| t as f32)
        };

        // Log message content block statistics for debugging
        for (i, msg) in messages.iter().enumerate() {
            let image_count = msg
                .content
                .iter()
                .filter(|b| matches!(b, ContentBlock::Image { .. }))
                .count();
            let text_count = msg
                .content
                .iter()
                .filter(|b| matches!(b, ContentBlock::Text { .. }))
                .count();
            if image_count > 0 {
                tracing::info!(
                    "build_request: Message {} ({:?}) has {} text blocks, {} image blocks",
                    i,
                    msg.role,
                    text_count,
                    image_count
                );
            }
        }

        types::CompletionRequest {
            anthropic_version: ANTHROPIC_VERSION.to_string(),
            messages,
            max_tokens,
            system: request.preamble.as_ref().map(|preamble| {
                // Convert string to array format with cache_control enabled
                vec![SystemBlock::cached(preamble.clone())]
            }),
            temperature,
            top_p: None,
            top_k: None,
            stop_sequences: None,
            tools,
            stream: if stream { Some(true) } else { None },
            thinking: self.thinking.clone(),
        }
    }

    /// Convert Anthropic response to rig's CompletionResponse.
    fn convert_response(
        response: types::CompletionResponse,
    ) -> CompletionResponse<types::CompletionResponse> {
        use rig::message::{Reasoning, Text, ToolCall, ToolFunction};

        // IMPORTANT: When thinking is enabled, thinking blocks MUST come first
        // Separate thinking from other content to ensure correct ordering
        let mut thinking_content: Vec<AssistantContent> = vec![];
        let mut other_content: Vec<AssistantContent> = vec![];

        for block in response.content.iter() {
            match block {
                ContentBlock::Thinking {
                    thinking,
                    signature,
                } => {
                    // Convert to AssistantContent::Reasoning with signature
                    thinking_content.push(AssistantContent::Reasoning(
                        Reasoning::multi(vec![thinking.clone()])
                            .with_signature(Some(signature.clone())),
                    ));
                }
                ContentBlock::Text { text, .. } => {
                    other_content.push(AssistantContent::Text(Text { text: text.clone() }));
                }
                ContentBlock::ToolUse { id, name, input } => {
                    other_content.push(AssistantContent::ToolCall(ToolCall {
                        id: id.clone(),
                        call_id: None,
                        function: ToolFunction {
                            name: name.clone(),
                            arguments: input.clone(),
                        },
                        signature: None,
                        additional_params: None,
                    }));
                }
                _ => {}
            }
        }

        // Combine: thinking first, then other content
        thinking_content.append(&mut other_content);
        let choice = thinking_content;

        CompletionResponse {
            choice: OneOrMany::many(choice).unwrap_or_else(|_| {
                OneOrMany::one(AssistantContent::Text(Text {
                    text: String::new(),
                }))
            }),
            usage: Usage {
                input_tokens: response.usage.input_tokens as u64,
                output_tokens: response.usage.output_tokens as u64,
                total_tokens: (response.usage.input_tokens + response.usage.output_tokens) as u64,
            },
            raw_response: response,
        }
    }
}

/// Response type for streaming (wraps our streaming response)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StreamingCompletionResponseData {
    /// Accumulated text
    pub text: String,
    /// Token usage (filled at end)
    pub usage: Option<types::Usage>,
}

impl rig::completion::GetTokenUsage for StreamingCompletionResponseData {
    fn token_usage(&self) -> Option<Usage> {
        self.usage.as_ref().map(|u| Usage {
            input_tokens: u.input_tokens as u64,
            output_tokens: u.output_tokens as u64,
            total_tokens: (u.input_tokens + u.output_tokens) as u64,
        })
    }
}

impl completion::CompletionModel for CompletionModel {
    type Response = types::CompletionResponse;
    type StreamingResponse = StreamingCompletionResponseData;
    type Client = Client;

    fn make(client: &Self::Client, model: impl Into<String>) -> Self {
        Self::new(client.clone(), model.into())
    }

    async fn completion(
        &self,
        request: CompletionRequest,
    ) -> Result<CompletionResponse<Self::Response>, CompletionError> {
        let anthropic_request = self.build_request(&request, false);

        // Build URL for rawPredict (non-streaming)
        let url = self.client.endpoint_url(&self.model, "rawPredict");

        // Get headers with auth (include beta header if web_fetch is enabled)
        let beta = if self.needs_web_fetch_beta() {
            Some(WEB_FETCH_BETA)
        } else {
            None
        };
        let headers = self
            .client
            .build_headers_with_beta(beta)
            .await
            .map_err(|e| CompletionError::ProviderError(e.to_string()))?;

        // Make the request
        let response = self
            .client
            .http_client()
            .post(&url)
            .headers(headers)
            .json(&anthropic_request)
            .send()
            .await
            .map_err(|e| CompletionError::RequestError(Box::new(e)))?;

        // Check for errors
        if !response.status().is_success() {
            let status = response.status().as_u16();
            let body = response.text().await.unwrap_or_default();
            return Err(CompletionError::ProviderError(format!(
                "API error ({}): {}",
                status, body
            )));
        }

        // Parse response
        let body = response
            .text()
            .await
            .map_err(|e| CompletionError::RequestError(Box::new(e)))?;

        let anthropic_response: types::CompletionResponse = serde_json::from_str(&body)?;

        Ok(Self::convert_response(anthropic_response))
    }

    async fn stream(
        &self,
        request: CompletionRequest,
    ) -> Result<StreamingCompletionResponse<Self::StreamingResponse>, CompletionError> {
        let anthropic_request = self.build_request(&request, true);

        // Log request details
        tracing::debug!(
            "stream(): thinking={:?}, max_tokens={}, messages={}",
            anthropic_request.thinking.as_ref().map(|t| t.budget_tokens),
            anthropic_request.max_tokens,
            anthropic_request.messages.len()
        );

        // Build URL for streamRawPredict
        let url = self.client.endpoint_url(&self.model, "streamRawPredict");
        tracing::info!("stream(): POST {}", url);

        // Get headers with auth (include beta header if web_fetch is enabled)
        let beta = if self.needs_web_fetch_beta() {
            Some(WEB_FETCH_BETA)
        } else {
            None
        };
        let headers = self
            .client
            .build_headers_with_beta(beta)
            .await
            .map_err(|e| CompletionError::ProviderError(e.to_string()))?;

        // Make the request
        let response = self
            .client
            .http_client()
            .post(&url)
            .headers(headers)
            .json(&anthropic_request)
            .send()
            .await
            .map_err(|e| {
                tracing::error!("stream(): Request failed: {}", e);
                CompletionError::RequestError(Box::new(e))
            })?;

        let status = response.status();
        tracing::info!("stream(): Response status: {}", status);

        // Check for errors
        if !status.is_success() {
            let status_code = status.as_u16();
            let body = response.text().await.unwrap_or_default();
            tracing::error!("stream(): API error ({}): {}", status_code, body);
            return Err(CompletionError::ProviderError(format!(
                "API error ({}): {}",
                status_code, body
            )));
        }

        // Create streaming response
        tracing::info!(
            "stream(): Creating streaming response wrapper, status={}",
            status
        );
        let stream = StreamingResponse::new(response);

        // Convert to rig's streaming format
        use futures::StreamExt;

        let mapped_stream = stream.map(|chunk_result| {
            use crate::streaming::StreamChunk;

            chunk_result
                .map(|chunk| {
                    let raw_choice = match chunk {
                        StreamChunk::TextDelta { text, .. } => RawStreamingChoice::Message(text),
                        StreamChunk::ToolUseStart { id, name } => {
                            RawStreamingChoice::ToolCall(RawStreamingToolCall {
                                id: id.clone(),
                                call_id: Some(id),
                                name,
                                arguments: serde_json::json!({}), // Must be a valid object
                                signature: None,
                                additional_params: None,
                            })
                        }
                        StreamChunk::ToolInputDelta { partial_json } => {
                            RawStreamingChoice::ToolCallDelta {
                                id: String::new(),
                                content: rig::streaming::ToolCallDeltaContent::Delta(partial_json),
                            }
                        }
                        StreamChunk::Done { usage, .. } => {
                            // Return final response with usage info
                            RawStreamingChoice::FinalResponse(StreamingCompletionResponseData {
                                text: String::new(),
                                usage,
                            })
                        }
                        StreamChunk::Error { message } => {
                            // Can't return error directly, emit as message
                            RawStreamingChoice::Message(format!("[Error: {}]", message))
                        }
                        StreamChunk::ThinkingDelta { thinking } => {
                            // Emit thinking content using native reasoning type
                            RawStreamingChoice::Reasoning {
                                id: None,
                                reasoning: thinking,
                                signature: None,
                            }
                        }
                        StreamChunk::ThinkingSignature { signature } => {
                            // Emit signature as a Reasoning event (empty reasoning, signature set)
                            RawStreamingChoice::Reasoning {
                                id: None,
                                reasoning: String::new(),
                                signature: Some(signature),
                            }
                        }
                        // Server tool events - emit as tool calls for now
                        // The agentic loop will handle these specially
                        StreamChunk::ServerToolUseStart { id, name, input } => {
                            tracing::info!("Server tool started: {} ({})", name, id);
                            RawStreamingChoice::ToolCall(RawStreamingToolCall {
                                id: id.clone(),
                                call_id: Some(format!("server:{}", id)),
                                name,
                                arguments: input,
                                signature: None,
                                additional_params: None,
                            })
                        }
                        StreamChunk::WebSearchResult {
                            tool_use_id,
                            results,
                        } => {
                            // Emit as a special message that can be parsed by the agentic loop
                            tracing::info!("Web search results received for {}", tool_use_id);
                            RawStreamingChoice::Message(format!(
                                "[WEB_SEARCH_RESULT:{}:{}]",
                                tool_use_id,
                                serde_json::to_string(&results).unwrap_or_default()
                            ))
                        }
                        StreamChunk::WebFetchResult {
                            tool_use_id,
                            url,
                            content,
                        } => {
                            // Emit as a special message that can be parsed by the agentic loop
                            tracing::info!(
                                "Web fetch result received for {}: {}",
                                tool_use_id,
                                url
                            );
                            RawStreamingChoice::Message(format!(
                                "[WEB_FETCH_RESULT:{}:{}:{}]",
                                tool_use_id,
                                url,
                                serde_json::to_string(&content).unwrap_or_default()
                            ))
                        }
                    };
                    raw_choice
                })
                .map_err(|e| {
                    tracing::error!("map_to_raw: chunk error: {}", e);
                    CompletionError::ProviderError(e.to_string())
                })
        });

        tracing::info!("Returning StreamingCompletionResponse");
        Ok(StreamingCompletionResponse::stream(Box::pin(mapped_stream)))
    }
}

impl std::fmt::Debug for CompletionModel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CompletionModel")
            .field("model", &self.model)
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rig::message::{DocumentSourceKind, Image, ImageMediaType, Text, UserContent};
    use rig::one_or_many::OneOrMany;

    #[test]
    fn test_convert_message_with_image() {
        // Create a rig Message with text and image content
        let image = Image {
            data: DocumentSourceKind::Base64("iVBORw0KGgoAAAANSUhEUg==".to_string()),
            media_type: Some(ImageMediaType::PNG),
            detail: None,
            additional_params: None,
        };

        let content = vec![
            UserContent::Text(Text {
                text: "What is in this image?".to_string(),
            }),
            UserContent::Image(image),
        ];

        let msg = Message::User {
            content: OneOrMany::many(content).unwrap(),
        };

        // Convert to Anthropic format
        let converted = CompletionModel::convert_message(&msg);

        // Verify the conversion
        assert_eq!(converted.content.len(), 2, "Should have 2 content blocks");

        // Check text block
        match &converted.content[0] {
            ContentBlock::Text { text, .. } => {
                assert_eq!(text, "What is in this image?");
            }
            _ => panic!("Expected Text block at index 0"),
        }

        // Check image block
        match &converted.content[1] {
            ContentBlock::Image { source, .. } => {
                assert_eq!(source.source_type, "base64");
                assert_eq!(source.media_type, "image/png");
                assert_eq!(source.data, "iVBORw0KGgoAAAANSUhEUg==");
            }
            _ => panic!("Expected Image block at index 1"),
        }

        // Verify JSON serialization
        let json = serde_json::to_string_pretty(&converted).unwrap();
        println!("Converted message JSON:\n{}", json);

        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["role"], "user");
        assert_eq!(parsed["content"][0]["type"], "text");
        assert_eq!(parsed["content"][1]["type"], "image");
        assert_eq!(parsed["content"][1]["source"]["type"], "base64");
        assert_eq!(parsed["content"][1]["source"]["media_type"], "image/png");
    }

    #[test]
    fn test_convert_message_image_only() {
        // Test with only an image (no text)
        let image = Image {
            data: DocumentSourceKind::Base64("YWJjZGVm".to_string()),
            media_type: Some(ImageMediaType::JPEG),
            detail: None,
            additional_params: None,
        };

        let content = vec![UserContent::Image(image)];

        let msg = Message::User {
            content: OneOrMany::many(content).unwrap(),
        };

        let converted = CompletionModel::convert_message(&msg);

        assert_eq!(converted.content.len(), 1, "Should have 1 content block");

        match &converted.content[0] {
            ContentBlock::Image { source, .. } => {
                assert_eq!(source.source_type, "base64");
                assert_eq!(source.media_type, "image/jpeg");
                assert_eq!(source.data, "YWJjZGVm");
            }
            _ => panic!("Expected Image block"),
        }
    }
}
