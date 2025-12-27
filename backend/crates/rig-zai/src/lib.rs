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
    OneOrMany,
    client::{CompletionClient, ProviderClient, VerifyClient, VerifyError},
    completion::{self, CompletionError, message, MessageError},
    http_client::{self, HttpClientExt},
    impl_conversion_traits,
    providers::openai::send_compatible_streaming_request,
    streaming::StreamingCompletionResponse,
};

use bytes::Bytes;
use http::Method;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tracing::{Instrument, info_span};

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

    fn from_val(input: rig::client::ProviderValue) -> Self {
        let rig::client::ProviderValue::Simple(api_key) = input else {
            panic!("Incorrect provider value type")
        };
        ClientBuilder::<T>::new(&api_key).build()
    }
}

impl<T> CompletionClient for Client<T>
where
    T: HttpClientExt + Clone + std::fmt::Debug + Default + Send + 'static,
{
    type CompletionModel = CompletionModel<T>;

    fn completion_model(&self, model: &str) -> Self::CompletionModel {
        CompletionModel::new(self.clone(), model)
    }
}

impl<T> VerifyClient for Client<T>
where
    T: HttpClientExt + Clone + std::fmt::Debug + Default + Send + 'static,
{
    async fn verify(&self) -> Result<(), VerifyError> {
        // Z.AI doesn't have a dedicated verification endpoint
        // We could make a minimal request, but for now we just return Ok
        Ok(())
    }
}

impl_conversion_traits!(
    AsTranscription,
    AsEmbeddings,
    AsImageGeneration,
    AsAudioGeneration for Client<T>
);

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
    pub content: String,
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
    pub role: Role,
    pub content: String,
}

#[derive(Deserialize, Debug, Serialize)]
pub struct Choice {
    pub index: usize,
    pub finish_reason: String,
    pub message: Message,
    pub delta: Delta,
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

impl TryFrom<CompletionResponse> for completion::CompletionResponse<CompletionResponse> {
    type Error = CompletionError;

    fn try_from(response: CompletionResponse) -> Result<Self, Self::Error> {
        let choice = response.choices.first().ok_or_else(|| {
            CompletionError::ResponseError("Response contained no choices".to_owned())
        })?;

        match &choice.message {
            Message {
                role: Role::Assistant,
                content,
            } => Ok(completion::CompletionResponse {
                choice: OneOrMany::one(content.clone().into()),
                usage: completion::Usage {
                    input_tokens: response.usage.prompt_tokens as u64,
                    output_tokens: response.usage.completion_tokens as u64,
                    total_tokens: response.usage.total_tokens as u64,
                },
                raw_response: response,
            }),
            _ => Err(CompletionError::ResponseError(
                "Response contained no assistant message".to_owned(),
            )),
        }
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
        if completion_request.tool_choice.is_some() {
            tracing::warn!("WARNING: `tool_choice` not supported on Z.AI GLM models");
        }

        // Build up the order of messages (context, chat_history, prompt)
        let mut partial_history = vec![];
        if let Some(docs) = completion_request.normalized_documents() {
            partial_history.push(docs);
        }
        partial_history.extend(completion_request.chat_history);

        // Initialize full history with preamble (or empty if non-existent)
        let mut full_history: Vec<Message> =
            completion_request
                .preamble
                .map_or_else(Vec::new, |preamble| {
                    vec![Message {
                        role: Role::System,
                        content: preamble,
                    }]
                });

        // Convert and extend the rest of the history
        full_history.extend(
            partial_history
                .into_iter()
                .map(message::Message::try_into)
                .collect::<Result<Vec<Message>, _>>()?,
        );

        // Compose request with thinking mode enabled for GLM-4.7
        // Z.AI thinking mode allows the model to reason before responding
        // See: https://docs.z.ai/guides/capabilities/thinking-mode
        let mut request = json!({
            "model": self.model,
            "messages": full_history,
            "temperature": completion_request.temperature,
        });

        // Enable thinking mode for GLM-4.7 (it's the default, but being explicit)
        // clear_thinking: false means we want "Preserved Thinking" - reasoning is kept in context
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
                    content: collapsed_content,
                }
            }

            message::Message::Assistant { content, .. } => {
                let collapsed_content = content
                    .into_iter()
                    .map(|content| {
                        Ok(match content {
                            message::AssistantContent::Text(message::Text { text }) => text,
                            _ => return Err(MessageError::ConversionError(
                                "Only text assistant message content is supported by Z.AI"
                                    .to_owned(),
                            )),
                        })
                    })
                    .collect::<Result<Vec<_>, _>>()?
                    .join("\n");

                Message {
                    role: Role::Assistant,
                    content: collapsed_content,
                }
            }
        })
    }
}

impl From<Message> for message::Message {
    fn from(message: Message) -> Self {
        match message.role {
            Role::User => message::Message::user(message.content),
            Role::Assistant => message::Message::assistant(message.content),
            // System messages get coerced into user messages for ease of error handling.
            // They should be handled on the outside of `Message` conversions via the preamble.
            Role::System => message::Message::user(message.content),
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
    type StreamingResponse = rig::providers::openai::StreamingCompletionResponse;

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

        request = merge_json(request, json!({"stream": true}));
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
        send_compatible_streaming_request(self.client.http_client.clone(), req)
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
        assert_eq!(message.content, "Hello, how can I help you?");
    }

    #[test]
    fn test_serialize_message() {
        let message = Message {
            role: Role::Assistant,
            content: "I am here to assist you.".to_string(),
        };

        let json_data = serde_json::to_string(&message).unwrap();
        let expected_json = r#"{"role":"assistant","content":"I am here to assist you."}"#;
        assert_eq!(json_data, expected_json);
    }

    #[test]
    fn test_message_to_message_conversion() {
        let user_message = message::Message::user("User message");
        let assistant_message = message::Message::assistant("Assistant message");

        let converted_user_message: Message = user_message.clone().try_into().unwrap();
        let converted_assistant_message: Message = assistant_message.clone().try_into().unwrap();

        assert_eq!(converted_user_message.role, Role::User);
        assert_eq!(converted_user_message.content, "User message");

        assert_eq!(converted_assistant_message.role, Role::Assistant);
        assert_eq!(converted_assistant_message.content, "Assistant message");

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
        let client_custom = Client::builder("test-api-key")
            .base_url(custom_url)
            .build();
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
}
