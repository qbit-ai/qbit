//! CompletionModel implementation for Gemini on Vertex AI.

use rig::completion::{
    self, AssistantContent, CompletionError, CompletionRequest, CompletionResponse, Message,
    ToolDefinition, Usage,
};
use rig::one_or_many::OneOrMany;
use rig::streaming::{RawStreamingChoice, RawStreamingToolCall, StreamingCompletionResponse};
use serde::{Deserialize, Serialize};

use crate::client::Client;
use crate::types::{
    self, Content, FunctionDeclaration, GenerateContentRequest, GenerationConfig, Part,
    ThinkingConfig, Tool, DEFAULT_MAX_TOKENS,
};

/// Default max tokens for different Gemini models
fn default_max_tokens_for_model(model: &str) -> u32 {
    if model.contains("2.0") {
        // Gemini 2.0 models have 8K max output
        8192
    } else {
        // Gemini 2.5+ and 3.x have 64K max output
        DEFAULT_MAX_TOKENS
    }
}

/// Completion model for Gemini on Vertex AI.
#[derive(Clone)]
pub struct CompletionModel {
    client: Client,
    model: String,
    /// Optional thinking configuration for reasoning models
    thinking: Option<ThinkingConfig>,
}

impl CompletionModel {
    /// Create a new completion model.
    pub fn new(client: Client, model: String) -> Self {
        Self {
            client,
            model,
            thinking: None,
        }
    }

    /// Enable thinking mode with the specified token budget.
    pub fn with_thinking_budget(mut self, budget: i32) -> Self {
        self.thinking = Some(ThinkingConfig::with_budget(budget));
        self
    }

    /// Enable thinking mode with the specified level ("LOW" or "HIGH").
    pub fn with_thinking_level(mut self, level: impl Into<String>) -> Self {
        self.thinking = Some(ThinkingConfig::with_level(level));
        self
    }

    /// Get the model identifier.
    pub fn model(&self) -> &str {
        &self.model
    }

    /// Convert rig's Message to Gemini Content format.
    fn convert_message(msg: &Message) -> Content {
        match msg {
            Message::User { content } => {
                let parts: Vec<Part> = content
                    .iter()
                    .filter_map(|c| {
                        use rig::message::UserContent;
                        match c {
                            UserContent::Text(text) => Some(Part::text(&text.text)),
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

                                Some(Part::inline_data(media_type, data))
                            }
                            UserContent::ToolResult(result) => {
                                // Convert tool result to function response
                                let response = serde_json::json!({
                                    "result": result.content
                                });
                                Some(Part::function_response(&result.id, response))
                            }
                            // Skip other content types not supported yet
                            _ => None,
                        }
                    })
                    .collect();

                Content {
                    role: Some("user".to_string()),
                    parts: if parts.is_empty() {
                        vec![Part::text("")]
                    } else {
                        parts
                    },
                }
            }
            Message::Assistant { content, .. } => {
                let parts: Vec<Part> = content
                    .iter()
                    .filter_map(|c| match c {
                        AssistantContent::Text(text) => Some(Part::text(&text.text)),
                        AssistantContent::ToolCall(tool_call) => {
                            let mut part = Part::function_call(
                                &tool_call.function.name,
                                tool_call.function.arguments.clone(),
                            );
                            // Include thought signature if present (required for thinking models)
                            part.thought_signature = tool_call.signature.clone();
                            Some(part)
                        }
                        AssistantContent::Reasoning(_) => {
                            // Thinking content - skip for now as we can't reconstruct it
                            None
                        }
                        AssistantContent::Image(_) => {
                            // Images in assistant content are not supported
                            None
                        }
                    })
                    .collect();

                Content {
                    role: Some("model".to_string()),
                    parts: if parts.is_empty() {
                        vec![Part::text("")]
                    } else {
                        parts
                    },
                }
            }
        }
    }

    /// Convert rig's ToolDefinition to Gemini FunctionDeclaration.
    fn convert_tool(tool: &ToolDefinition) -> FunctionDeclaration {
        // Use parametersJsonSchema which accepts standard JSON Schema format
        // (with lowercase type names like "string", "integer")
        // rather than Google's custom Schema format (with uppercase TYPE names).
        let parameters = if tool.parameters.is_null()
            || tool.parameters == serde_json::json!({})
            || tool
                .parameters
                .as_object()
                .is_some_and(|obj| obj.is_empty())
        {
            // For functions with no parameters, we need to provide a minimal object schema
            Some(serde_json::json!({
                "type": "object",
                "properties": {},
                "additionalProperties": false
            }))
        } else {
            Some(tool.parameters.clone())
        };

        FunctionDeclaration {
            name: tool.name.clone(),
            description: tool.description.clone(),
            parameters_json_schema: parameters,
        }
    }

    /// Build a Gemini request from a rig CompletionRequest.
    fn build_request(&self, request: &CompletionRequest) -> GenerateContentRequest {
        // Convert chat history to contents
        let contents: Vec<Content> = request
            .chat_history
            .iter()
            .map(Self::convert_message)
            .collect();

        // Determine max tokens
        let max_output_tokens = request
            .max_tokens
            .map(|t| t as i32)
            .unwrap_or_else(|| default_max_tokens_for_model(&self.model) as i32);

        // Build generation config
        let generation_config = Some(GenerationConfig {
            temperature: request.temperature.map(|t| t as f32),
            top_p: None,
            top_k: None,
            candidate_count: None,
            max_output_tokens: Some(max_output_tokens),
            stop_sequences: None,
            response_mime_type: None,
            response_schema: None,
            thinking_config: self.thinking.clone(),
        });

        // Convert tools
        let tools = if request.tools.is_empty() {
            None
        } else {
            let function_declarations: Vec<FunctionDeclaration> =
                request.tools.iter().map(Self::convert_tool).collect();
            Some(vec![Tool {
                function_declarations: Some(function_declarations),
            }])
        };

        // Build system instruction
        let system_instruction = request
            .preamble
            .as_ref()
            .map(|preamble| Content::system(preamble.clone()));

        GenerateContentRequest {
            contents,
            system_instruction,
            tools,
            tool_config: None,
            safety_settings: None,
            generation_config,
        }
    }

    /// Convert Gemini response to rig's CompletionResponse.
    fn convert_response(
        response: types::GenerateContentResponse,
    ) -> CompletionResponse<types::GenerateContentResponse> {
        use rig::message::{Text, ToolCall, ToolFunction};

        let mut content: Vec<AssistantContent> = vec![];

        if let Some(candidate) = response.candidates.first() {
            for part in &candidate.content.parts {
                // Check for text
                if let Some(text) = &part.text {
                    if !text.is_empty() {
                        // Check if this is thinking content
                        if part.thought == Some(true) {
                            content.push(AssistantContent::Reasoning(
                                rig::message::Reasoning::multi(vec![text.clone()]),
                            ));
                        } else {
                            content.push(AssistantContent::Text(Text { text: text.clone() }));
                        }
                    }
                }

                // Check for function call
                if let Some(fc) = &part.function_call {
                    content.push(AssistantContent::ToolCall(ToolCall {
                        id: fc.name.clone(), // Gemini doesn't have separate IDs
                        call_id: None,
                        function: ToolFunction {
                            name: fc.name.clone(),
                            arguments: fc.args.clone(),
                        },
                        signature: None,
                        additional_params: None,
                    }));
                }
            }
        }

        // Get usage
        let usage = response
            .usage_metadata
            .as_ref()
            .map(|u| Usage {
                input_tokens: u.prompt_token_count as u64,
                output_tokens: u.candidates_token_count as u64,
                total_tokens: u.total_token_count as u64,
            })
            .unwrap_or_default();

        CompletionResponse {
            choice: OneOrMany::many(content).unwrap_or_else(|_| {
                OneOrMany::one(AssistantContent::Text(Text {
                    text: String::new(),
                }))
            }),
            usage,
            raw_response: response,
        }
    }
}

/// Response type for streaming
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StreamingCompletionResponseData {
    /// Accumulated text
    pub text: String,
    /// Token usage (filled at end)
    pub usage: Option<types::UsageMetadata>,
}

impl rig::completion::GetTokenUsage for StreamingCompletionResponseData {
    fn token_usage(&self) -> Option<Usage> {
        self.usage.as_ref().map(|u| Usage {
            input_tokens: u.prompt_token_count as u64,
            output_tokens: u.candidates_token_count as u64,
            total_tokens: u.total_token_count as u64,
        })
    }
}

impl completion::CompletionModel for CompletionModel {
    type Response = types::GenerateContentResponse;
    type StreamingResponse = StreamingCompletionResponseData;
    type Client = Client;

    fn make(client: &Self::Client, model: impl Into<String>) -> Self {
        Self::new(client.clone(), model.into())
    }

    async fn completion(
        &self,
        request: CompletionRequest,
    ) -> Result<CompletionResponse<Self::Response>, CompletionError> {
        let gemini_request = self.build_request(&request);

        // Build URL for generateContent (non-streaming)
        let url = self.client.endpoint_url(&self.model, "generateContent");

        // Get headers with auth
        let headers = self
            .client
            .build_headers()
            .await
            .map_err(|e| CompletionError::ProviderError(e.to_string()))?;

        // Make the request
        let response = self
            .client
            .http_client()
            .post(&url)
            .headers(headers)
            .json(&gemini_request)
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

        let gemini_response: types::GenerateContentResponse = serde_json::from_str(&body)?;

        Ok(Self::convert_response(gemini_response))
    }

    async fn stream(
        &self,
        request: CompletionRequest,
    ) -> Result<StreamingCompletionResponse<Self::StreamingResponse>, CompletionError> {
        let gemini_request = self.build_request(&request);

        // Build URL for streamGenerateContent with SSE
        let url = format!(
            "{}?alt=sse",
            self.client
                .endpoint_url(&self.model, "streamGenerateContent")
        );
        tracing::debug!("stream(): POST {}", url);

        // Get headers with auth
        let headers = self
            .client
            .build_headers()
            .await
            .map_err(|e| CompletionError::ProviderError(e.to_string()))?;

        // Make the request
        let response = self
            .client
            .http_client()
            .post(&url)
            .headers(headers)
            .json(&gemini_request)
            .send()
            .await
            .map_err(|e| {
                tracing::error!("stream(): Request failed: {}", e);
                CompletionError::RequestError(Box::new(e))
            })?;

        let status = response.status();
        tracing::debug!("stream(): Response status: {}", status);

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

        // Create streaming response using async_stream
        use crate::streaming::{create_stream, StreamChunk};
        use futures::StreamExt;

        let stream = create_stream(response);

        // Map to rig's streaming format
        let mapped_stream = stream.map(|chunk_result| {
            chunk_result
                .map(|chunk| match chunk {
                    StreamChunk::TextDelta { text, .. } => RawStreamingChoice::Message(text),
                    StreamChunk::FunctionCall {
                        name,
                        args,
                        signature,
                    } => RawStreamingChoice::ToolCall(RawStreamingToolCall {
                        id: name.clone(),
                        call_id: Some(name.clone()),
                        name,
                        arguments: args,
                        signature,
                        additional_params: None,
                    }),
                    StreamChunk::ThinkingDelta { thinking } => RawStreamingChoice::Reasoning {
                        id: None,
                        reasoning: thinking,
                        signature: None,
                    },
                    StreamChunk::Done { usage, .. } => {
                        RawStreamingChoice::FinalResponse(StreamingCompletionResponseData {
                            text: String::new(),
                            usage,
                        })
                    }
                })
                .map_err(|e| {
                    tracing::error!("stream map error: {}", e);
                    CompletionError::ProviderError(e.to_string())
                })
        });

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
