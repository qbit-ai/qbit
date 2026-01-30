//! CompletionModel implementation for Z.AI API.

use futures::StreamExt;
use rig::completion::{
    self, AssistantContent, CompletionError, CompletionRequest, CompletionResponse, Message,
    ToolDefinition, Usage,
};
use rig::message::{Reasoning, Text, ToolCall, ToolFunction, ToolResultContent, UserContent};
use rig::one_or_many::OneOrMany;
use rig::streaming::{
    RawStreamingChoice, RawStreamingToolCall, StreamingCompletionResponse, ToolCallDeltaContent,
};
use serde::{Deserialize, Serialize};

use crate::client::Client;
use crate::streaming::{StreamChunk, StreamingResponse};
use crate::text_tool_parser;
use crate::types;

/// Default max tokens for Z.AI models
const DEFAULT_MAX_TOKENS: u32 = 4096;

/// Completion model for Z.AI API.
#[derive(Clone)]
pub struct CompletionModel {
    client: Client,
    model: String,
}

impl CompletionModel {
    /// Create a new completion model.
    pub fn new(client: Client, model: String) -> Self {
        Self { client, model }
    }

    /// Get the model identifier.
    pub fn model(&self) -> &str {
        &self.model
    }

    /// Convert rig's Message to Z.AI message format.
    fn convert_message(msg: &Message) -> types::Message {
        match msg {
            Message::User { content } => {
                let text = extract_user_text(content);
                types::Message {
                    role: types::Role::User,
                    content: types::MessageContent::Text(text),
                    tool_calls: None,
                    tool_call_id: None,
                    name: None,
                }
            }
            Message::Assistant { content, .. } => {
                // Extract text and tool calls from assistant content
                let mut text_parts = Vec::new();
                let mut tool_calls = Vec::new();

                for c in content.iter() {
                    match c {
                        AssistantContent::Text(t) => {
                            text_parts.push(t.text.clone());
                        }
                        AssistantContent::ToolCall(tc) => {
                            tool_calls.push(types::ToolCall {
                                id: tc.id.clone(),
                                call_type: "function".to_string(),
                                function: types::FunctionCall {
                                    name: tc.function.name.clone(),
                                    arguments: serde_json::to_string(&tc.function.arguments)
                                        .unwrap_or_default(),
                                },
                            });
                        }
                        AssistantContent::Reasoning(r) => {
                            // Include reasoning as part of the text for context
                            text_parts.push(format!("[Reasoning]: {}", r.reasoning.join("")));
                        }
                        _ => {}
                    }
                }

                types::Message {
                    role: types::Role::Assistant,
                    content: types::MessageContent::Text(text_parts.join("\n")),
                    tool_calls: if tool_calls.is_empty() {
                        None
                    } else {
                        Some(tool_calls)
                    },
                    tool_call_id: None,
                    name: None,
                }
            }
        }
    }

    /// Convert a tool result from user content to a Z.AI tool message.
    fn convert_tool_result(
        tool_call_id: &str,
        content: &OneOrMany<ToolResultContent>,
    ) -> types::Message {
        let text: String = content
            .iter()
            .filter_map(|c| match c {
                ToolResultContent::Text(t) => Some(t.text.clone()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("\n");

        types::Message {
            role: types::Role::Tool,
            content: types::MessageContent::Text(text),
            tool_calls: None,
            tool_call_id: Some(tool_call_id.to_string()),
            name: None,
        }
    }

    /// Convert rig's ToolDefinition to Z.AI format.
    fn convert_tool(tool: &ToolDefinition) -> types::ToolDefinition {
        types::ToolDefinition {
            tool_type: "function".to_string(),
            function: types::FunctionDefinition {
                name: tool.name.clone(),
                description: tool.description.clone(),
                parameters: tool.parameters.clone(),
            },
        }
    }

    /// Build a Z.AI request from a rig CompletionRequest.
    fn build_request(&self, request: &CompletionRequest, stream: bool) -> types::CompletionRequest {
        let mut messages = Vec::new();

        // Add system prompt if present
        if let Some(ref preamble) = request.preamble {
            messages.push(types::Message::system(preamble.clone()));
        }

        // Convert chat history
        for msg in request.chat_history.iter() {
            // Check for tool results in user messages
            if let Message::User { content } = msg {
                for c in content.iter() {
                    if let UserContent::ToolResult(result) = c {
                        messages.push(Self::convert_tool_result(&result.id, &result.content));
                    }
                }
                // If there's also text content, add it as a user message
                let text = extract_user_text(content);
                if !text.is_empty()
                    && !content
                        .iter()
                        .all(|c| matches!(c, UserContent::ToolResult(_)))
                {
                    messages.push(types::Message::user(text));
                }
            } else {
                messages.push(Self::convert_message(msg));
            }
        }

        // Add documents as user messages
        for doc in &request.documents {
            messages.push(types::Message::user(format!(
                "[Document: {}]\n{}",
                doc.id, doc.text
            )));
        }

        // Convert tools
        let tools: Option<Vec<types::ToolDefinition>> = if request.tools.is_empty() {
            None
        } else {
            Some(request.tools.iter().map(Self::convert_tool).collect())
        };

        // Clamp temperature to (0.0, 1.0) open interval as per Z.AI requirements
        let temperature = request.temperature.map(|t| {
            let t = t as f32;
            if t <= 0.0 {
                0.01
            } else if t >= 1.0 {
                0.99
            } else {
                t
            }
        });

        types::CompletionRequest {
            model: self.model.clone(),
            messages,
            stream: if stream { Some(true) } else { None },
            temperature,
            top_p: None, // Could add from request.additional_params if needed
            max_tokens: Some(
                request
                    .max_tokens
                    .map(|t| t as u32)
                    .unwrap_or(DEFAULT_MAX_TOKENS),
            ),
            stop: None,
            seed: None,
            tools,
            tool_choice: None,
            thinking: Some(types::ThinkingConfig::enabled()), // Always enable thinking
            tool_stream: if stream { Some(true) } else { None }, // Enable tool streaming when streaming
        }
    }

    /// Convert Z.AI response to rig's CompletionResponse.
    fn convert_response(response: types::Completion) -> CompletionResponse<types::Completion> {
        let mut content: Vec<AssistantContent> = Vec::new();
        let mut pseudo_tool_call_counter = 0u32;

        // Get first choice
        if let Some(choice) = response.choices.first() {
            // Add reasoning content first if present, checking for pseudo-XML tool calls
            if let Some(ref reasoning) = choice.message.reasoning_content {
                if !reasoning.is_empty() {
                    // Check for pseudo-XML tool calls in the reasoning content
                    if text_tool_parser::contains_pseudo_xml_tool_calls(reasoning) {
                        let (parsed_calls, remaining_reasoning) =
                            text_tool_parser::parse_tool_calls_from_text(reasoning);

                        // Add remaining reasoning if any
                        if !remaining_reasoning.is_empty() {
                            content.push(AssistantContent::Reasoning(Reasoning::new(
                                &remaining_reasoning,
                            )));
                        }

                        // Convert parsed pseudo-XML tool calls to ToolCall content
                        for parsed in parsed_calls {
                            pseudo_tool_call_counter += 1;
                            let id = format!("pseudo_call_{}", pseudo_tool_call_counter);
                            tracing::info!(
                                "Extracted pseudo-XML tool call from reasoning content: {}",
                                parsed.name
                            );
                            content.push(AssistantContent::ToolCall(ToolCall {
                                id,
                                call_id: None,
                                function: ToolFunction {
                                    name: parsed.name,
                                    arguments: parsed.arguments,
                                },
                                signature: None,
                                additional_params: None,
                            }));
                        }
                    } else {
                        content.push(AssistantContent::Reasoning(Reasoning::new(reasoning)));
                    }
                }
            }

            // Add text content, checking for pseudo-XML tool calls
            if let Some(ref text) = choice.message.content {
                if !text.is_empty() {
                    // Check for pseudo-XML tool calls in the text
                    if text_tool_parser::contains_pseudo_xml_tool_calls(text) {
                        let (parsed_calls, remaining_text) =
                            text_tool_parser::parse_tool_calls_from_text(text);

                        // Add remaining text if any
                        if !remaining_text.is_empty() {
                            content.push(AssistantContent::Text(Text {
                                text: remaining_text,
                            }));
                        }

                        // Convert parsed pseudo-XML tool calls to ToolCall content
                        for parsed in parsed_calls {
                            pseudo_tool_call_counter += 1;
                            let id = format!("pseudo_call_{}", pseudo_tool_call_counter);
                            tracing::info!(
                                "Extracted pseudo-XML tool call from non-streaming response: {}",
                                parsed.name
                            );
                            content.push(AssistantContent::ToolCall(ToolCall {
                                id,
                                call_id: None,
                                function: ToolFunction {
                                    name: parsed.name,
                                    arguments: parsed.arguments,
                                },
                                signature: None,
                                additional_params: None,
                            }));
                        }
                    } else {
                        content.push(AssistantContent::Text(Text { text: text.clone() }));
                    }
                }
            }

            // Add tool calls from the structured API
            if let Some(ref tool_calls) = choice.message.tool_calls {
                for tc in tool_calls {
                    let arguments = qbit_json_repair::parse_tool_args(&tc.function.arguments);
                    content.push(AssistantContent::ToolCall(ToolCall {
                        id: tc.id.clone(),
                        call_id: None,
                        function: ToolFunction {
                            name: tc.function.name.clone(),
                            arguments,
                        },
                        signature: None,
                        additional_params: None,
                    }));
                }
            }
        }

        CompletionResponse {
            choice: OneOrMany::many(content).unwrap_or_else(|_| {
                OneOrMany::one(AssistantContent::Text(Text {
                    text: String::new(),
                }))
            }),
            usage: Usage {
                input_tokens: response.usage.prompt_tokens as u64,
                output_tokens: response.usage.completion_tokens as u64,
                total_tokens: response.usage.total_tokens as u64,
            },
            raw_response: response,
        }
    }
}

impl std::fmt::Debug for CompletionModel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CompletionModel")
            .field("model", &self.model)
            .finish_non_exhaustive()
    }
}

// ============================================================================
// StreamingResponseData
// ============================================================================

/// Data accumulated during streaming, returned as the final response.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct StreamingResponseData {
    /// Token usage statistics (populated at end of stream).
    pub usage: Option<StreamingUsage>,
}

/// Token usage for streaming responses.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StreamingUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

impl rig::completion::GetTokenUsage for StreamingResponseData {
    fn token_usage(&self) -> Option<Usage> {
        self.usage.as_ref().map(|u| Usage {
            input_tokens: u.prompt_tokens as u64,
            output_tokens: u.completion_tokens as u64,
            total_tokens: u.total_tokens as u64,
        })
    }
}

// ============================================================================
// CompletionModel Trait Implementation
// ============================================================================

impl completion::CompletionModel for CompletionModel {
    type Response = types::Completion;
    type StreamingResponse = StreamingResponseData;
    type Client = Client;

    fn make(client: &Self::Client, model: impl Into<String>) -> Self {
        Self::new(client.clone(), model.into())
    }

    async fn completion(
        &self,
        request: CompletionRequest,
    ) -> Result<CompletionResponse<Self::Response>, CompletionError> {
        let zai_request = self.build_request(&request, false);

        let url = self.client.endpoint_url("/chat/completions");
        let headers = self
            .client
            .build_headers()
            .map_err(|e| CompletionError::ProviderError(e.to_string()))?;

        tracing::debug!("Z.AI completion request to: {}", url);

        let response = self
            .client
            .http_client()
            .post(&url)
            .headers(headers)
            .json(&zai_request)
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

        let zai_response: types::Completion = serde_json::from_str(&body)?;

        Ok(Self::convert_response(zai_response))
    }

    async fn stream(
        &self,
        request: CompletionRequest,
    ) -> Result<StreamingCompletionResponse<Self::StreamingResponse>, CompletionError> {
        let zai_request = self.build_request(&request, true);

        let url = self.client.endpoint_url("/chat/completions");
        let headers = self
            .client
            .build_headers()
            .map_err(|e| CompletionError::ProviderError(e.to_string()))?;

        tracing::debug!("Z.AI streaming request to: {}", url);

        let response = self
            .client
            .http_client()
            .post(&url)
            .headers(headers)
            .json(&zai_request)
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

        // Create streaming response
        let stream = StreamingResponse::new(response);

        // Map to rig's streaming format
        let mapped_stream = stream.map(|chunk_result| {
            chunk_result
                .map(|chunk| match chunk {
                    StreamChunk::TextDelta { text } => RawStreamingChoice::Message(text),
                    StreamChunk::ReasoningDelta { reasoning } => RawStreamingChoice::Reasoning {
                        id: None,
                        reasoning,
                        signature: None,
                    },
                    StreamChunk::ToolCallStart { id, name, .. } => {
                        tracing::info!("Tool call started: {} ({})", name, id);
                        RawStreamingChoice::ToolCall(RawStreamingToolCall {
                            id: id.clone(),
                            call_id: Some(id),
                            name,
                            arguments: serde_json::json!({}),
                            signature: None,
                            additional_params: None,
                        })
                    }
                    StreamChunk::ToolCallDelta { arguments, .. } => {
                        RawStreamingChoice::ToolCallDelta {
                            id: String::new(),
                            content: ToolCallDeltaContent::Delta(arguments),
                        }
                    }
                    StreamChunk::ToolCallsComplete { tool_calls } => {
                        // Emit the first tool call as complete (rig handles one at a time)
                        if let Some(tc) = tool_calls.first() {
                            let arguments = qbit_json_repair::parse_tool_args(&tc.arguments);
                            RawStreamingChoice::ToolCall(RawStreamingToolCall {
                                id: tc.id.clone(),
                                call_id: Some(tc.id.clone()),
                                name: tc.name.clone(),
                                arguments,
                                signature: None,
                                additional_params: None,
                            })
                        } else {
                            RawStreamingChoice::Message(String::new())
                        }
                    }
                    StreamChunk::Done { usage } => {
                        RawStreamingChoice::FinalResponse(StreamingResponseData {
                            usage: usage.map(|u| StreamingUsage {
                                prompt_tokens: u.prompt_tokens,
                                completion_tokens: u.completion_tokens,
                                total_tokens: u.total_tokens,
                            }),
                        })
                    }
                    StreamChunk::Error { message } => {
                        RawStreamingChoice::Message(format!("[Error: {}]", message))
                    }
                    StreamChunk::Empty => RawStreamingChoice::Message(String::new()),
                })
                .map_err(|e| {
                    tracing::error!("Stream chunk error: {}", e);
                    CompletionError::ProviderError(e.to_string())
                })
        });

        Ok(StreamingCompletionResponse::stream(Box::pin(mapped_stream)))
    }
}

// ============================================================================
// Helpers
// ============================================================================

/// Extract text content from user message content.
fn extract_user_text(content: &OneOrMany<UserContent>) -> String {
    content
        .iter()
        .filter_map(|c| match c {
            UserContent::Text(text) => Some(text.text.clone()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_temperature_clamping() {
        // Test the temperature clamping logic directly
        let clamp = |t: f64| -> f32 {
            let t = t as f32;
            if t <= 0.0 {
                0.01
            } else if t >= 1.0 {
                0.99
            } else {
                t
            }
        };

        assert_eq!(clamp(0.0), 0.01);
        assert_eq!(clamp(1.0), 0.99);
        assert_eq!(clamp(0.7), 0.7);
        assert_eq!(clamp(-0.5), 0.01);
        assert_eq!(clamp(1.5), 0.99);
    }

    #[test]
    fn test_zai_request_defaults() {
        // Test that our request type has correct defaults
        let req = types::CompletionRequest::default();
        assert!(req.thinking.is_some());
        assert_eq!(req.thinking.as_ref().unwrap().thinking_type, "enabled");
        assert_eq!(req.stream, None);
        assert_eq!(req.tool_stream, None);
    }

    #[test]
    fn test_client_creation() {
        let client = Client::new("test-key");
        assert_eq!(client.api_key(), "test-key");
    }

    #[test]
    fn test_completion_model_creation() {
        let client = Client::new("test-key");
        let model = CompletionModel::new(client, "glm-4".to_string());
        assert_eq!(model.model(), "glm-4");
    }

    #[test]
    fn test_message_conversion() {
        // Test user message conversion
        let user_msg = types::Message::user("Hello");
        assert_eq!(user_msg.role, types::Role::User);
        match user_msg.content {
            types::MessageContent::Text(s) => assert_eq!(s, "Hello"),
            _ => panic!("Expected text content"),
        }

        // Test assistant message conversion
        let asst_msg = types::Message::assistant("Hi there");
        assert_eq!(asst_msg.role, types::Role::Assistant);
        match asst_msg.content {
            types::MessageContent::Text(s) => assert_eq!(s, "Hi there"),
            _ => panic!("Expected text content"),
        }

        // Test tool result message conversion
        let tool_msg = types::Message::tool_result("call_123", "Result data");
        assert_eq!(tool_msg.role, types::Role::Tool);
        assert_eq!(tool_msg.tool_call_id, Some("call_123".to_string()));
    }
}
