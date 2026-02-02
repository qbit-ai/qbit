//! CompletionModel implementation for OpenAI Responses API.

use async_openai::config::OpenAIConfig;
use async_openai::types::responses::{
    CreateResponse, EasyInputContent, EasyInputMessage, FunctionCallOutput,
    FunctionCallOutputItemParam, FunctionTool, FunctionToolCall, ImageDetail, InputContent,
    InputImageContent, InputItem, InputParam, InputTextContent, Item, MessageType, OutputItem,
    OutputMessageContent, OutputStatus, Reasoning, ReasoningEffort as OAReasoningEffort,
    ReasoningItem, ReasoningSummary, Response, ResponseStreamEvent, Role, Summary, SummaryPart,
    Tool,
};
use async_openai::Client as OpenAIClient;
use futures::StreamExt;
use rig::completion::{
    self, AssistantContent, CompletionError, CompletionRequest, CompletionResponse, Message,
    ToolDefinition,
};
use rig::message::{Text, ToolCall, ToolFunction, UserContent};
use rig::one_or_many::OneOrMany;
use rig::streaming::{
    RawStreamingChoice, RawStreamingToolCall, StreamingCompletionResponse, ToolCallDeltaContent,
};
use serde::{Deserialize, Serialize};

use crate::error::OpenAiResponsesError;

// ============================================================================
// Client
// ============================================================================

/// Wrapper around async-openai client for creating completion models.
#[derive(Clone)]
pub struct Client {
    inner: OpenAIClient<OpenAIConfig>,
}

impl Client {
    /// Create a new client with the given API key.
    pub fn new(api_key: impl Into<String>) -> Self {
        let config = OpenAIConfig::new().with_api_key(api_key);
        Self {
            inner: OpenAIClient::with_config(config),
        }
    }

    /// Create a new client with a custom base URL (e.g., for Azure OpenAI).
    pub fn with_base_url(api_key: impl Into<String>, base_url: impl Into<String>) -> Self {
        let config = OpenAIConfig::new()
            .with_api_key(api_key)
            .with_api_base(base_url);
        Self {
            inner: OpenAIClient::with_config(config),
        }
    }

    /// Create a completion model for the given model name.
    pub fn completion_model(&self, model: impl Into<String>) -> CompletionModel {
        CompletionModel::new(self.clone(), model.into())
    }
}

impl std::fmt::Debug for Client {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Client").finish_non_exhaustive()
    }
}

// ============================================================================
// ReasoningEffort
// ============================================================================

/// Reasoning effort level for OpenAI reasoning models.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ReasoningEffort {
    /// Low reasoning effort - faster but less thorough.
    Low,
    /// Medium reasoning effort - balanced.
    #[default]
    Medium,
    /// High reasoning effort - slower but more thorough.
    High,
}

impl From<ReasoningEffort> for OAReasoningEffort {
    fn from(effort: ReasoningEffort) -> Self {
        match effort {
            ReasoningEffort::Low => OAReasoningEffort::Low,
            ReasoningEffort::Medium => OAReasoningEffort::Medium,
            ReasoningEffort::High => OAReasoningEffort::High,
        }
    }
}

// ============================================================================
// CompletionModel
// ============================================================================

/// Completion model for OpenAI Responses API with explicit reasoning support.
#[derive(Clone)]
pub struct CompletionModel {
    client: Client,
    model: String,
    reasoning_effort: Option<ReasoningEffort>,
}

impl CompletionModel {
    /// Create a new completion model.
    pub fn new(client: Client, model: String) -> Self {
        Self {
            client,
            model,
            reasoning_effort: None,
        }
    }

    /// Set the reasoning effort level for reasoning models.
    pub fn with_reasoning_effort(mut self, effort: ReasoningEffort) -> Self {
        self.reasoning_effort = Some(effort);
        self
    }

    /// Get the model name.
    pub fn model(&self) -> &str {
        &self.model
    }

    /// Build an OpenAI Responses API request from a rig CompletionRequest.
    fn build_request(
        &self,
        request: &CompletionRequest,
    ) -> Result<CreateResponse, OpenAiResponsesError> {
        // Convert chat history to input items using EasyInputMessage
        let mut input_items: Vec<InputItem> = Vec::new();

        for msg in request.chat_history.iter() {
            match msg {
                Message::User { content } => {
                    let user_items = convert_user_content(content);
                    input_items.extend(user_items);
                }
                Message::Assistant { content, .. } => {
                    let assistant_items = convert_assistant_content_to_items(content);
                    input_items.extend(assistant_items);
                }
            }
        }

        // Add the current prompt/preamble as a system/developer message
        if let Some(preamble) = &request.preamble {
            input_items.insert(
                0,
                InputItem::EasyMessage(EasyInputMessage {
                    r#type: MessageType::Message,
                    role: Role::Developer,
                    content: EasyInputContent::Text(preamble.clone()),
                }),
            );
        }

        // Build the input
        let input = if input_items.is_empty() {
            InputParam::Text(String::new())
        } else if input_items.len() == 1 {
            // For single user text message, we can use simple text input
            if let InputItem::EasyMessage(msg) = &input_items[0] {
                if matches!(msg.role, Role::User) {
                    if let EasyInputContent::Text(text) = &msg.content {
                        InputParam::Text(text.clone())
                    } else {
                        InputParam::Items(input_items)
                    }
                } else {
                    InputParam::Items(input_items)
                }
            } else {
                InputParam::Items(input_items)
            }
        } else {
            InputParam::Items(input_items)
        };

        // Convert tools
        let tools: Option<Vec<Tool>> = if request.tools.is_empty() {
            None
        } else {
            Some(request.tools.iter().map(convert_tool_definition).collect())
        };

        // Build reasoning config if enabled
        let reasoning = self.reasoning_effort.map(|effort| Reasoning {
            effort: Some(effort.into()),
            summary: Some(ReasoningSummary::Auto),
        });

        // Build the request
        // Note: Reasoning models (o1, o3, o4, gpt-5.x) don't support temperature
        let temperature = if crate::is_reasoning_model(&self.model) {
            if request.temperature.is_some() {
                tracing::debug!(
                    "Ignoring temperature parameter for reasoning model {}",
                    self.model
                );
            }
            None
        } else {
            request.temperature.map(|t| t as f32)
        };

        Ok(CreateResponse {
            model: Some(self.model.clone()),
            input,
            tools,
            reasoning,
            temperature,
            max_output_tokens: request.max_tokens.map(|t| t as u32),
            ..Default::default()
        })
    }

    /// Convert an OpenAI Response to a rig CompletionResponse.
    fn convert_response(response: Response) -> CompletionResponse<Response> {
        let mut content: Vec<AssistantContent> = Vec::new();

        // Extract content from output items
        for output in &response.output {
            match output {
                OutputItem::Message(msg) => {
                    for c in &msg.content {
                        match c {
                            OutputMessageContent::OutputText(text_output) => {
                                content.push(AssistantContent::Text(Text {
                                    text: text_output.text.clone(),
                                }));
                            }
                            OutputMessageContent::Refusal(refusal) => {
                                content.push(AssistantContent::Text(Text {
                                    text: format!("[Refusal]: {}", refusal.refusal),
                                }));
                            }
                        }
                    }
                }
                OutputItem::Reasoning(reasoning) => {
                    // Extract reasoning texts from summary, preserving each part separately
                    // This ensures proper round-tripping when the reasoning is sent back to OpenAI
                    let reasoning_parts: Vec<String> = reasoning
                        .summary
                        .iter()
                        .map(|SummaryPart::SummaryText(st)| st.text.clone())
                        .collect();

                    // Also check the content field if present (populated with reasoning.encrypted_content include)
                    let content_parts: Vec<String> = reasoning
                        .content
                        .as_ref()
                        .map(|c| c.iter().map(|rtc| rtc.text.clone()).collect())
                        .unwrap_or_default();

                    // Combine: prefer content if available, otherwise use summary
                    let all_parts = if !content_parts.is_empty() {
                        content_parts
                    } else {
                        reasoning_parts
                    };

                    if !all_parts.is_empty() {
                        // Create Reasoning with multi() to preserve structure
                        content.push(AssistantContent::Reasoning(
                            rig::message::Reasoning::multi(all_parts)
                                .with_id(reasoning.id.clone()),
                        ));
                    }
                }
                OutputItem::FunctionCall(fc) => {
                    let arguments = qbit_json_repair::parse_tool_args(&fc.arguments);
                    // fc.id is Option<String>, use empty string as fallback
                    let id = fc.id.clone().unwrap_or_default();
                    content.push(AssistantContent::ToolCall(ToolCall {
                        id,
                        call_id: Some(fc.call_id.clone()),
                        function: ToolFunction {
                            name: fc.name.clone(),
                            arguments,
                        },
                        signature: None,
                        additional_params: None,
                    }));
                }
                _ => {}
            }
        }

        // Extract usage
        let usage = response.usage.as_ref().map(|u| rig::completion::Usage {
            input_tokens: u.input_tokens as u64,
            output_tokens: u.output_tokens as u64,
            total_tokens: u.total_tokens as u64,
        });

        CompletionResponse {
            choice: OneOrMany::many(content).unwrap_or_else(|_| {
                OneOrMany::one(AssistantContent::Text(Text {
                    text: String::new(),
                }))
            }),
            usage: usage.unwrap_or(rig::completion::Usage {
                input_tokens: 0,
                output_tokens: 0,
                total_tokens: 0,
            }),
            raw_response: response,
        }
    }
}

impl std::fmt::Debug for CompletionModel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CompletionModel")
            .field("model", &self.model)
            .field("reasoning_effort", &self.reasoning_effort)
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
    pub usage: Option<Usage>,
}

/// Token usage for streaming responses.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Usage {
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub total_tokens: u32,
}

impl rig::completion::GetTokenUsage for StreamingResponseData {
    fn token_usage(&self) -> Option<rig::completion::Usage> {
        self.usage.as_ref().map(|u| rig::completion::Usage {
            input_tokens: u.input_tokens as u64,
            output_tokens: u.output_tokens as u64,
            total_tokens: u.total_tokens as u64,
        })
    }
}

// ============================================================================
// CompletionModel Trait Implementation
// ============================================================================

impl completion::CompletionModel for CompletionModel {
    type Response = Response;
    type StreamingResponse = StreamingResponseData;
    type Client = Client;

    fn make(client: &Self::Client, model: impl Into<String>) -> Self {
        Self::new(client.clone(), model.into())
    }

    async fn completion(
        &self,
        request: CompletionRequest,
    ) -> Result<CompletionResponse<Self::Response>, CompletionError> {
        let openai_request = self.build_request(&request)?;

        let response = self
            .client
            .inner
            .responses()
            .create(openai_request)
            .await
            .map_err(|e| CompletionError::ProviderError(e.to_string()))?;

        Ok(Self::convert_response(response))
    }

    async fn stream(
        &self,
        request: CompletionRequest,
    ) -> Result<StreamingCompletionResponse<Self::StreamingResponse>, CompletionError> {
        let openai_request = self.build_request(&request)?;

        tracing::debug!("Starting OpenAI Responses stream for model: {}", self.model);

        let stream = self
            .client
            .inner
            .responses()
            .create_stream(openai_request)
            .await
            .map_err(|e| CompletionError::ProviderError(e.to_string()))?;

        // Map async-openai events to rig-core RawStreamingChoice
        let mapped = stream.filter_map(|result| async move {
            match result {
                Ok(event) => map_stream_event(event).map(Ok),
                Err(e) => {
                    tracing::error!("OpenAI stream error: {}", e);
                    Some(Ok(RawStreamingChoice::Message(format!("[Error: {}]", e))))
                }
            }
        });

        Ok(StreamingCompletionResponse::stream(Box::pin(mapped)))
    }
}

// ============================================================================
// Event Mapping
// ============================================================================

/// Map an async-openai ResponseStreamEvent to a rig-core RawStreamingChoice.
///
/// This is the core function that ensures reasoning events are explicitly
/// separated from text events.
fn map_stream_event(
    event: ResponseStreamEvent,
) -> Option<RawStreamingChoice<StreamingResponseData>> {
    match event {
        // Text deltas → Message
        ResponseStreamEvent::ResponseOutputTextDelta(e) => {
            tracing::trace!("Text delta: {} chars", e.delta.len());
            Some(RawStreamingChoice::Message(e.delta))
        }

        // Reasoning summary deltas → ReasoningDelta (EXPLICIT separation!)
        ResponseStreamEvent::ResponseReasoningSummaryTextDelta(e) => {
            tracing::trace!("Reasoning summary delta: {} chars", e.delta.len());
            Some(RawStreamingChoice::ReasoningDelta {
                id: Some(e.item_id),
                reasoning: e.delta,
            })
        }

        // Reasoning text deltas → ReasoningDelta
        ResponseStreamEvent::ResponseReasoningTextDelta(e) => {
            tracing::trace!("Reasoning text delta: {} chars", e.delta.len());
            Some(RawStreamingChoice::ReasoningDelta {
                id: Some(e.item_id),
                reasoning: e.delta,
            })
        }

        // Function call argument deltas → ToolCallDelta
        ResponseStreamEvent::ResponseFunctionCallArgumentsDelta(e) => {
            tracing::trace!("Function call args delta: {} chars", e.delta.len());
            Some(RawStreamingChoice::ToolCallDelta {
                id: e.item_id,
                content: ToolCallDeltaContent::Delta(e.delta),
            })
        }

        // Output item added - check for function calls
        ResponseStreamEvent::ResponseOutputItemAdded(e) => {
            if let OutputItem::FunctionCall(fc) = e.item {
                tracing::info!("Function call started: {}", fc.name);
                // fc.id is Option<String>, use empty string as fallback
                let id = fc.id.clone().unwrap_or_default();
                Some(RawStreamingChoice::ToolCall(RawStreamingToolCall {
                    id,
                    call_id: Some(fc.call_id),
                    name: fc.name,
                    arguments: serde_json::json!({}),
                    signature: None,
                    additional_params: None,
                }))
            } else {
                None
            }
        }

        // Response completed → FinalResponse with usage
        ResponseStreamEvent::ResponseCompleted(e) => {
            tracing::info!("Response completed");
            let usage = e.response.usage.map(|u| Usage {
                input_tokens: u.input_tokens,
                output_tokens: u.output_tokens,
                total_tokens: u.total_tokens,
            });
            Some(RawStreamingChoice::FinalResponse(StreamingResponseData {
                usage,
            }))
        }

        // Errors - ResponseErrorEvent has code, message, param fields
        ResponseStreamEvent::ResponseError(e) => {
            tracing::error!(
                "OpenAI response error: code={:?}, message={:?}",
                e.code,
                e.message
            );
            Some(RawStreamingChoice::Message(format!(
                "[Error: {:?} - {:?}]",
                e.code, e.message
            )))
        }

        // Response failed
        ResponseStreamEvent::ResponseFailed(e) => {
            tracing::error!("OpenAI response failed: {:?}", e.response.status);
            Some(RawStreamingChoice::Message(format!(
                "[Response failed: {:?}]",
                e.response.status
            )))
        }

        // Refusal deltas
        ResponseStreamEvent::ResponseRefusalDelta(e) => {
            tracing::warn!("Refusal delta received");
            Some(RawStreamingChoice::Message(format!(
                "[Refusal] {}",
                e.delta
            )))
        }

        // Lifecycle events we don't need to emit as content
        ResponseStreamEvent::ResponseCreated(_)
        | ResponseStreamEvent::ResponseInProgress(_)
        | ResponseStreamEvent::ResponseIncomplete(_)
        | ResponseStreamEvent::ResponseQueued(_)
        | ResponseStreamEvent::ResponseOutputItemDone(_)
        | ResponseStreamEvent::ResponseContentPartAdded(_)
        | ResponseStreamEvent::ResponseContentPartDone(_)
        | ResponseStreamEvent::ResponseOutputTextDone(_)
        | ResponseStreamEvent::ResponseRefusalDone(_)
        | ResponseStreamEvent::ResponseReasoningSummaryPartAdded(_)
        | ResponseStreamEvent::ResponseReasoningSummaryPartDone(_)
        | ResponseStreamEvent::ResponseReasoningSummaryTextDone(_)
        | ResponseStreamEvent::ResponseReasoningTextDone(_)
        | ResponseStreamEvent::ResponseFunctionCallArgumentsDone(_) => None,

        // Other events (web search, file search, MCP, etc.) - log and skip
        other => {
            tracing::debug!("Unhandled OpenAI stream event: {:?}", other);
            None
        }
    }
}

// ============================================================================
// Conversion Helpers
// ============================================================================

/// Convert user content to OpenAI InputItems, handling text, images, and tool results.
///
/// For text and images, returns an EasyInputMessage.
/// For tool results, returns structured Item::FunctionCallOutput.
fn convert_user_content(content: &OneOrMany<UserContent>) -> Vec<InputItem> {
    use base64::Engine;

    let mut has_images = false;
    let mut input_parts: Vec<InputContent> = Vec::new();
    let mut result_items: Vec<InputItem> = Vec::new();

    /// Helper to flush pending text/image content into an EasyInputMessage
    fn flush_pending(
        parts: &mut Vec<InputContent>,
        has_img: bool,
        result_items: &mut Vec<InputItem>,
    ) {
        if parts.is_empty() {
            return;
        }

        let content = if has_img {
            EasyInputContent::ContentList(parts.clone())
        } else {
            // For text-only, join all text parts
            let text = parts
                .iter()
                .filter_map(|p| {
                    if let InputContent::InputText(t) = p {
                        Some(t.text.clone())
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>()
                .join("\n");
            EasyInputContent::Text(text)
        };

        result_items.push(InputItem::EasyMessage(EasyInputMessage {
            r#type: MessageType::Message,
            role: Role::User,
            content,
        }));

        parts.clear();
    }

    for c in content.iter() {
        match c {
            UserContent::Text(text) => {
                if !text.text.is_empty() {
                    input_parts.push(InputContent::InputText(InputTextContent {
                        text: text.text.clone(),
                    }));
                }
            }
            UserContent::Image(img) => {
                // Convert rig Image to OpenAI InputImageContent
                let image_url = match &img.data {
                    rig::message::DocumentSourceKind::Base64(b64) => {
                        // Already base64, construct data URL
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
                            })
                            .unwrap_or("image/png");
                        format!("data:{};base64,{}", media_type, b64)
                    }
                    rig::message::DocumentSourceKind::Url(url) => {
                        // Direct URL
                        url.clone()
                    }
                    rig::message::DocumentSourceKind::Raw(bytes) => {
                        // Raw bytes, encode to base64
                        let b64 = base64::engine::general_purpose::STANDARD.encode(bytes);
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
                            })
                            .unwrap_or("image/png");
                        format!("data:{};base64,{}", media_type, b64)
                    }
                    // Handle any future variants added to this non-exhaustive enum
                    _ => {
                        tracing::warn!("Unsupported image source kind, skipping");
                        continue;
                    }
                };

                // Convert rig ImageDetail to async-openai ImageDetail
                let detail = img
                    .detail
                    .as_ref()
                    .map(|d| {
                        use rig::message::ImageDetail as RigImageDetail;
                        match d {
                            RigImageDetail::Auto => ImageDetail::Auto,
                            RigImageDetail::High => ImageDetail::High,
                            RigImageDetail::Low => ImageDetail::Low,
                        }
                    })
                    .unwrap_or(ImageDetail::Auto);

                input_parts.push(InputContent::InputImage(InputImageContent {
                    detail,
                    file_id: None,
                    image_url: Some(image_url),
                }));
                has_images = true;
            }
            UserContent::ToolResult(result) => {
                // Flush any pending text/image content before adding tool result
                flush_pending(&mut input_parts, has_images, &mut result_items);
                has_images = false;

                // Extract text from tool result content
                let result_text = result
                    .content
                    .iter()
                    .filter_map(|item| {
                        if let rig::message::ToolResultContent::Text(t) = item {
                            Some(t.text.clone())
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>()
                    .join("\n");

                // Use structured FunctionCallOutput with proper call_id linkage
                // Note: The `id` field is generated by the API for output items, not expected on input.
                let call_id = result.call_id.clone().unwrap_or_else(|| result.id.clone());
                result_items.push(InputItem::Item(Item::FunctionCallOutput(
                    FunctionCallOutputItemParam {
                        call_id,
                        output: FunctionCallOutput::Text(result_text),
                        id: None,
                        status: Some(OutputStatus::Completed),
                    },
                )));
            }
            // Skip other content types (Audio, Video, Document) not supported yet
            _ => {
                tracing::debug!("Skipping unsupported user content type");
            }
        }
    }

    // Flush any remaining text/image content
    flush_pending(&mut input_parts, has_images, &mut result_items);

    result_items
}

/// Convert assistant content to OpenAI InputItems, handling text, tool calls, and reasoning.
///
/// Returns structured items for tool calls (Item::FunctionCall), reasoning (Item::Reasoning),
/// and text (EasyInputMessage).
///
/// IMPORTANT: For reasoning models (GPT-5, o-series), reasoning items must be passed back
/// with tool call outputs. See: https://platform.openai.com/docs/guides/function-calling
fn convert_assistant_content_to_items(content: &OneOrMany<AssistantContent>) -> Vec<InputItem> {
    let mut items: Vec<InputItem> = Vec::new();
    let mut text_parts: Vec<String> = Vec::new();

    /// Helper to flush pending text content into an EasyInputMessage
    fn flush_text(text_parts: &mut Vec<String>, items: &mut Vec<InputItem>) {
        if !text_parts.is_empty() {
            let combined_text = text_parts.join("\n");
            items.push(InputItem::EasyMessage(EasyInputMessage {
                r#type: MessageType::Message,
                role: Role::Assistant,
                content: EasyInputContent::Text(combined_text),
            }));
            text_parts.clear();
        }
    }

    for c in content.iter() {
        match c {
            AssistantContent::Text(text) => {
                text_parts.push(text.text.clone());
            }
            AssistantContent::ToolCall(tc) => {
                // Flush any pending text before adding tool call
                flush_text(&mut text_parts, &mut items);

                // Emit structured tool call
                // Note: The `id` field is generated by the API for output items, not expected on input.
                // We only need `call_id` which links to the function_call_output.
                let arguments = serde_json::to_string(&tc.function.arguments)
                    .unwrap_or_else(|_| "{}".to_string());
                let call_id = tc.call_id.clone().unwrap_or_else(|| tc.id.clone());
                items.push(InputItem::Item(Item::FunctionCall(FunctionToolCall {
                    arguments,
                    call_id,
                    name: tc.function.name.clone(),
                    id: None,
                    status: Some(OutputStatus::Completed),
                })));
            }
            AssistantContent::Reasoning(reasoning) => {
                // Flush any pending text before adding reasoning
                flush_text(&mut text_parts, &mut items);

                // For reasoning models, we MUST include reasoning items in the conversation.
                // Convert rig Reasoning to OpenAI ReasoningItem.
                let id = reasoning.id.clone().unwrap_or_else(|| {
                    // Generate a unique ID if not provided
                    format!("rs_{:x}", std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_nanos())
                });

                // Convert reasoning text to summary parts
                let summary: Vec<SummaryPart> = reasoning
                    .reasoning
                    .iter()
                    .map(|text| SummaryPart::SummaryText(Summary { text: text.clone() }))
                    .collect();

                items.push(InputItem::Item(Item::Reasoning(ReasoningItem {
                    id,
                    summary,
                    content: None,
                    encrypted_content: None,
                    status: Some(OutputStatus::Completed),
                })));
            }
            _ => {
                // Skip other content types
            }
        }
    }

    // Flush any remaining text
    flush_text(&mut text_parts, &mut items);

    items
}

/// Extract only pure text content from assistant message content.
/// Tool calls and reasoning are skipped.
#[allow(dead_code)]
fn extract_assistant_text_only(content: &OneOrMany<AssistantContent>) -> String {
    content
        .iter()
        .filter_map(|c| match c {
            AssistantContent::Text(text) => Some(text.text.clone()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Convert a rig ToolDefinition to an async-openai Tool.
fn convert_tool_definition(tool: &ToolDefinition) -> Tool {
    Tool::Function(FunctionTool {
        name: tool.name.clone(),
        description: Some(tool.description.clone()),
        parameters: Some(tool.parameters.clone()),
        strict: Some(true),
    })
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reasoning_effort_conversion() {
        assert!(matches!(
            OAReasoningEffort::from(ReasoningEffort::Low),
            OAReasoningEffort::Low
        ));
        assert!(matches!(
            OAReasoningEffort::from(ReasoningEffort::Medium),
            OAReasoningEffort::Medium
        ));
        assert!(matches!(
            OAReasoningEffort::from(ReasoningEffort::High),
            OAReasoningEffort::High
        ));
    }

    #[test]
    fn test_convert_user_content_text_only() {
        let content = OneOrMany::one(UserContent::Text(Text {
            text: "Hello, world!".to_string(),
        }));
        let result = convert_user_content(&content);
        assert_eq!(result.len(), 1);
        match &result[0] {
            InputItem::EasyMessage(msg) => {
                assert_eq!(msg.role, Role::User);
                match &msg.content {
                    EasyInputContent::Text(text) => assert_eq!(text, "Hello, world!"),
                    _ => panic!("Expected Text content"),
                }
            }
            _ => panic!("Expected EasyMessage"),
        }
    }

    #[test]
    fn test_convert_user_content_with_image() {
        use rig::message::{DocumentSourceKind, Image, ImageMediaType};

        let content = OneOrMany::many(vec![
            UserContent::Text(Text {
                text: "What's in this image?".to_string(),
            }),
            UserContent::Image(Image {
                data: DocumentSourceKind::Base64("dGVzdA==".to_string()),
                media_type: Some(ImageMediaType::PNG),
                detail: None,
                additional_params: None,
            }),
        ])
        .unwrap();
        let result = convert_user_content(&content);
        assert_eq!(result.len(), 1);
        match &result[0] {
            InputItem::EasyMessage(msg) => {
                assert_eq!(msg.role, Role::User);
                match &msg.content {
                    EasyInputContent::ContentList(parts) => {
                        assert_eq!(parts.len(), 2);
                        match &parts[0] {
                            InputContent::InputText(t) => {
                                assert_eq!(t.text, "What's in this image?")
                            }
                            _ => panic!("Expected InputText"),
                        }
                        match &parts[1] {
                            InputContent::InputImage(img) => {
                                assert!(img
                                    .image_url
                                    .as_ref()
                                    .unwrap()
                                    .starts_with("data:image/png;base64,"));
                            }
                            _ => panic!("Expected InputImage"),
                        }
                    }
                    _ => panic!("Expected ContentList"),
                }
            }
            _ => panic!("Expected EasyMessage"),
        }
    }

    #[test]
    fn test_extract_assistant_text_only() {
        let content = OneOrMany::one(AssistantContent::Text(Text {
            text: "Hello from assistant!".to_string(),
        }));
        assert_eq!(
            extract_assistant_text_only(&content),
            "Hello from assistant!"
        );
    }

    #[test]
    fn test_convert_user_content_with_tool_result() {
        use rig::message::{ToolResult, ToolResultContent};

        let content = OneOrMany::one(UserContent::ToolResult(ToolResult {
            id: "result_123".to_string(),
            call_id: Some("call_abc".to_string()),
            content: OneOrMany::one(ToolResultContent::Text(Text {
                text: "Tool execution result".to_string(),
            })),
        }));
        let result = convert_user_content(&content);

        // Should produce a structured FunctionCallOutput, not text
        assert_eq!(result.len(), 1);
        match &result[0] {
            InputItem::Item(Item::FunctionCallOutput(output)) => {
                // Verify call_id is properly linked
                assert_eq!(output.call_id, "call_abc");
                // Verify the output text
                match &output.output {
                    FunctionCallOutput::Text(text) => {
                        assert_eq!(text, "Tool execution result");
                    }
                    _ => panic!("Expected Text output"),
                }
            }
            _ => panic!("Expected Item::FunctionCallOutput"),
        }
    }

    #[test]
    fn test_convert_assistant_content_with_tool_call() {
        let content = OneOrMany::one(AssistantContent::ToolCall(ToolCall {
            id: "tool_123".to_string(),
            call_id: Some("call_xyz".to_string()),
            function: ToolFunction {
                name: "read_file".to_string(),
                arguments: serde_json::json!({"path": "test.txt"}),
            },
            signature: None,
            additional_params: None,
        }));
        let result = convert_assistant_content_to_items(&content);

        // Should produce a structured FunctionCall, not text
        assert_eq!(result.len(), 1);
        match &result[0] {
            InputItem::Item(Item::FunctionCall(fc)) => {
                assert_eq!(fc.name, "read_file");
                assert_eq!(fc.call_id, "call_xyz");
                // Arguments should be serialized as JSON string
                assert!(fc.arguments.contains("test.txt"));
            }
            _ => panic!("Expected Item::FunctionCall"),
        }
    }

    #[test]
    fn test_convert_assistant_content_with_reasoning() {
        // Use multi() constructor for multiple reasoning items
        let reasoning = rig::message::Reasoning::multi(vec![
            "First, I need to consider...".to_string(),
            "Then, I should analyze...".to_string(),
        ])
        .with_id("rs_test123".to_string());
        let content = OneOrMany::one(AssistantContent::Reasoning(reasoning));
        let result = convert_assistant_content_to_items(&content);

        // Should produce a structured Reasoning item
        assert_eq!(result.len(), 1);
        match &result[0] {
            InputItem::Item(Item::Reasoning(reasoning)) => {
                assert_eq!(reasoning.id, "rs_test123");
                assert_eq!(reasoning.summary.len(), 2);
                // Check that summaries contain the reasoning text
                match &reasoning.summary[0] {
                    SummaryPart::SummaryText(s) => {
                        assert_eq!(s.text, "First, I need to consider...");
                    }
                }
                match &reasoning.summary[1] {
                    SummaryPart::SummaryText(s) => {
                        assert_eq!(s.text, "Then, I should analyze...");
                    }
                }
            }
            _ => panic!("Expected Item::Reasoning"),
        }
    }
}