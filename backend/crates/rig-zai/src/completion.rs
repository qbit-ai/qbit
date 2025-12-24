//! Completion model implementation for Z.AI.

use crate::client::Client;
use crate::streaming::{process_streaming_response, StreamChunk, StreamingResponse};
use crate::types::{
    CompletionRequest, CompletionResponse, FunctionDefinition, Message, Role, ThinkingConfig, Tool,
};
use crate::ZaiError;
use rig::completion::{
    self, AssistantContent, CompletionError, CompletionRequest as RigCompletionRequest,
    CompletionResponse as RigCompletionResponse, Usage,
};
use rig::message::{Message as RigMessage, Reasoning, Text, ToolCall, ToolFunction, UserContent};
use rig::one_or_many::OneOrMany;
use rig::streaming::{RawStreamingChoice, StreamingCompletionResponse};
use serde::{Deserialize, Serialize};

/// Z.AI completion model
#[derive(Clone)]
pub struct CompletionModel {
    client: Client,
    model: String,
    thinking_enabled: bool,
}

impl CompletionModel {
    /// Create a new completion model.
    pub fn new(client: Client, model: &str) -> Self {
        // Enable thinking by default for GLM-4.7
        let thinking_enabled = model.contains("GLM-4.7") || model.contains("glm-4.7");
        Self {
            client,
            model: model.to_string(),
            thinking_enabled,
        }
    }

    /// Enable or disable thinking mode.
    pub fn with_thinking(mut self, enabled: bool) -> Self {
        self.thinking_enabled = enabled;
        self
    }

    /// Build the API request from a rig CompletionRequest.
    fn build_request(&self, request: &RigCompletionRequest, stream: bool) -> CompletionRequest {
        let mut messages = Vec::new();

        // Add system prompt if present
        if let Some(ref preamble) = request.preamble {
            messages.push(Message {
                role: Role::System,
                content: Some(preamble.clone()),
                name: None,
                tool_calls: None,
                tool_call_id: None,
                reasoning_content: None,
            });
        }

        // Convert chat history
        for msg in request.chat_history.iter() {
            messages.push(convert_rig_message(msg));
        }

        // Convert tools
        let tools: Option<Vec<Tool>> = if request.tools.is_empty() {
            None
        } else {
            Some(
                request
                    .tools
                    .iter()
                    .map(|t| Tool {
                        tool_type: "function".to_string(),
                        function: FunctionDefinition {
                            name: t.name.clone(),
                            description: Some(t.description.clone()),
                            parameters: Some(t.parameters.clone()),
                        },
                    })
                    .collect(),
            )
        };

        CompletionRequest {
            model: self.model.clone(),
            messages,
            max_tokens: request.max_tokens.map(|t| t as u32),
            temperature: request.temperature.map(|t| t as f32),
            stream: Some(stream),
            thinking: if self.thinking_enabled {
                Some(ThinkingConfig::enabled())
            } else {
                None
            },
            tools,
            tool_choice: None,
        }
    }

    /// Make a streaming request to Z.AI API.
    async fn stream_request(&self, request: CompletionRequest) -> Result<StreamingResponse, ZaiError> {
        let url = format!("{}/chat/completions", self.client.base_url);

        // Log the full request JSON for debugging
        if let Ok(json) = serde_json::to_string_pretty(&request) {
            tracing::info!(
                "Z.AI streaming request to {}:\n{}",
                url,
                json
            );
        }

        tracing::debug!(
            "Z.AI streaming request to {}: model={}, thinking={:?}",
            url,
            request.model,
            request.thinking
        );

        let response = self
            .client
            .http_client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.client.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| ZaiError::RequestError(e.to_string()))?;

        tracing::info!(
            "Z.AI response status: {}, content-type: {:?}",
            response.status(),
            response.headers().get("content-type")
        );

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            tracing::error!("Z.AI API error: {} - {}", status, text);
            return Err(ZaiError::ApiError {
                status: status.as_u16(),
                message: text,
            });
        }

        Ok(process_streaming_response(response))
    }
}

/// Streaming response data for Z.AI
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StreamingResponseData {
    /// Accumulated text
    pub text: String,
}

impl rig::completion::GetTokenUsage for StreamingResponseData {
    fn token_usage(&self) -> Option<Usage> {
        // Z.AI doesn't provide token usage in streaming mode
        None
    }
}

impl completion::CompletionModel for CompletionModel {
    type Response = String;
    type StreamingResponse = StreamingResponseData;

    async fn completion(
        &self,
        request: RigCompletionRequest,
    ) -> Result<RigCompletionResponse<Self::Response>, CompletionError> {
        let api_request = self.build_request(&request, false);
        let url = format!("{}/chat/completions", self.client.base_url);

        let response = self
            .client
            .http_client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.client.api_key))
            .header("Content-Type", "application/json")
            .json(&api_request)
            .send()
            .await
            .map_err(|e| CompletionError::ProviderError(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(CompletionError::ProviderError(
                format!("Z.AI API error {}: {}", status, text),
            ));
        }

        let api_response: CompletionResponse = response
            .json()
            .await
            .map_err(|e| CompletionError::ProviderError(e.to_string()))?;

        let choice = api_response
            .choices
            .first()
            .ok_or_else(|| CompletionError::ProviderError(
                "No choices in response".to_string(),
            ))?;

        let content = choice.message.content.clone().unwrap_or_default();

        // Build assistant content
        let mut assistant_content: Vec<AssistantContent> = Vec::new();

        // Add reasoning content if present
        if let Some(ref reasoning) = choice.message.reasoning_content {
            if !reasoning.is_empty() {
                assistant_content.push(AssistantContent::Reasoning(
                    Reasoning::multi(vec![reasoning.clone()])
                ));
            }
        }

        // Add text content
        if !content.is_empty() {
            assistant_content.push(AssistantContent::Text(Text { text: content.clone() }));
        }

        // Add tool calls if present
        if let Some(ref tool_calls) = choice.message.tool_calls {
            for tc in tool_calls {
                assistant_content.push(AssistantContent::ToolCall(ToolCall {
                    id: tc.id.clone(),
                    call_id: Some(tc.id.clone()),
                    function: ToolFunction {
                        name: tc.function.name.clone(),
                        arguments: serde_json::from_str(&tc.function.arguments)
                            .unwrap_or(serde_json::Value::Null),
                    },
                }));
            }
        }

        // Default to empty text if no content
        if assistant_content.is_empty() {
            assistant_content.push(AssistantContent::Text(Text { text: String::new() }));
        }

        Ok(RigCompletionResponse {
            choice: OneOrMany::many(assistant_content)
                .unwrap_or_else(|_| OneOrMany::one(AssistantContent::Text(Text { text: String::new() }))),
            usage: Usage {
                input_tokens: api_response.usage.as_ref().map(|u| u.prompt_tokens as u64).unwrap_or(0),
                output_tokens: api_response.usage.as_ref().map(|u| u.completion_tokens as u64).unwrap_or(0),
                total_tokens: api_response.usage.as_ref().map(|u| u.total_tokens as u64).unwrap_or(0),
            },
            raw_response: content,
        })
    }

    async fn stream(
        &self,
        request: RigCompletionRequest,
    ) -> Result<StreamingCompletionResponse<Self::StreamingResponse>, CompletionError> {
        let api_request = self.build_request(&request, true);
        let stream = self
            .stream_request(api_request)
            .await
            .map_err(|e| CompletionError::ProviderError(e.to_string()))?;

        // Map our stream chunks to rig's RawStreamingChoice format
        use futures::StreamExt;

        let mapped_stream = stream.map(|result| {
            result
                .map(|chunk| {
                    let raw_choice = match chunk {
                        StreamChunk::Reasoning(ref reasoning) => {
                            tracing::info!("Z.AI: emitting Reasoning chunk: {} chars", reasoning.len());
                            RawStreamingChoice::Reasoning {
                                id: None,
                                reasoning: reasoning.clone(),
                                signature: None,
                            }
                        }
                        StreamChunk::Text(ref text) => {
                            tracing::debug!("Z.AI: emitting Text chunk: {} chars", text.len());
                            RawStreamingChoice::Message(text.clone())
                        }
                        StreamChunk::ToolCallStart { ref id, ref name, .. } => {
                            tracing::info!("Z.AI: emitting ToolCall: {} - {}", id, name);
                            RawStreamingChoice::ToolCall {
                                id: id.clone(),
                                call_id: Some(id.clone()),
                                name: name.clone(),
                                arguments: serde_json::json!({}),
                            }
                        }
                        StreamChunk::ToolCallDelta { ref id, ref delta } => {
                            tracing::debug!("Z.AI: emitting ToolCallDelta: {}", id);
                            RawStreamingChoice::ToolCallDelta {
                                id: id.clone(),
                                delta: delta.clone(),
                            }
                        }
                        StreamChunk::Done => {
                            tracing::info!("Z.AI: emitting FinalResponse");
                            RawStreamingChoice::FinalResponse(StreamingResponseData {
                                text: String::new(),
                            })
                        }
                    };
                    raw_choice
                })
                .map_err(|e| CompletionError::ProviderError(e.to_string()))
        });

        Ok(StreamingCompletionResponse::stream(Box::pin(mapped_stream)))
    }
}

/// Convert a rig Message to a Z.AI Message.
fn convert_rig_message(msg: &RigMessage) -> Message {
    match msg {
        RigMessage::User { content } => {
            let text = extract_user_content_text(content);
            Message {
                role: Role::User,
                content: Some(text),
                name: None,
                tool_calls: None,
                tool_call_id: None,
                reasoning_content: None,
            }
        }
        RigMessage::Assistant { content, .. } => {
            let (text, tool_calls) = extract_assistant_content(content);
            Message {
                role: Role::Assistant,
                content: if text.is_empty() { None } else { Some(text) },
                name: None,
                tool_calls,
                tool_call_id: None,
                reasoning_content: None,
            }
        }
    }
}

/// Extract text from user content.
fn extract_user_content_text(content: &OneOrMany<UserContent>) -> String {
    content
        .iter()
        .filter_map(|c| match c {
            UserContent::Text(text) => Some(text.text.clone()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Extract text and tool calls from assistant content.
fn extract_assistant_content(
    content: &OneOrMany<AssistantContent>,
) -> (String, Option<Vec<crate::types::ToolCall>>) {
    let mut text_parts = Vec::new();
    let mut tool_calls = Vec::new();

    for c in content.iter() {
        match c {
            AssistantContent::Text(text) => {
                text_parts.push(text.text.clone());
            }
            AssistantContent::ToolCall(tc) => {
                tool_calls.push(crate::types::ToolCall {
                    id: tc.id.clone(),
                    call_type: "function".to_string(),
                    function: crate::types::ToolFunction {
                        name: tc.function.name.clone(),
                        arguments: tc.function.arguments.to_string(),
                    },
                });
            }
            _ => {}
        }
    }

    let text = text_parts.join("");
    let tool_calls = if tool_calls.is_empty() {
        None
    } else {
        Some(tool_calls)
    };

    (text, tool_calls)
}
