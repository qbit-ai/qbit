//! Client for the Z.AI API.

use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, AUTHORIZATION, CONTENT_TYPE};

use crate::completion::CompletionModel;
use crate::error::ZaiError;

/// Default base URL for Z.AI API
const DEFAULT_BASE_URL: &str = "https://api.z.ai/api/paas/v4";

/// Default source channel identifier
const DEFAULT_SOURCE_CHANNEL: &str = "rig-zai-sdk";

/// Client for the Z.AI API.
///
/// # Example
///
/// ```rust,no_run
/// use rig_zai_sdk::Client;
///
/// let client = Client::new("your-api-key");
/// let model = client.completion_model("glm-4-flash");
/// ```
#[derive(Clone)]
pub struct Client {
    /// HTTP client
    http_client: reqwest::Client,
    /// API key
    api_key: String,
    /// Base URL for the API
    base_url: String,
    /// Source channel identifier
    source_channel: String,
}

impl Client {
    /// Create a new client with the given API key.
    ///
    /// Uses the default base URL and source channel.
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            http_client: reqwest::Client::new(),
            api_key: api_key.into(),
            base_url: DEFAULT_BASE_URL.to_string(),
            source_channel: DEFAULT_SOURCE_CHANNEL.to_string(),
        }
    }

    /// Create a new client with custom configuration.
    ///
    /// # Arguments
    /// * `api_key` - API key for authentication
    /// * `base_url` - Custom base URL (if None, uses default)
    /// * `source_channel` - Custom source channel (if None, uses default)
    pub fn with_config(
        api_key: impl Into<String>,
        base_url: Option<String>,
        source_channel: Option<String>,
    ) -> Self {
        Self {
            http_client: reqwest::Client::new(),
            api_key: api_key.into(),
            base_url: base_url.unwrap_or_else(|| DEFAULT_BASE_URL.to_string()),
            source_channel: source_channel.unwrap_or_else(|| DEFAULT_SOURCE_CHANNEL.to_string()),
        }
    }

    /// Create a completion model for the given model ID.
    ///
    /// # Example
    /// ```rust,no_run
    /// use rig_zai_sdk::Client;
    ///
    /// let client = Client::new("your-api-key");
    /// let model = client.completion_model("glm-4-flash");
    /// ```
    pub fn completion_model(&self, model: impl Into<String>) -> CompletionModel {
        CompletionModel::new(self.clone(), model.into())
    }

    /// Get the API key.
    pub fn api_key(&self) -> &str {
        &self.api_key
    }

    /// Get the base URL.
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// Get the source channel.
    pub fn source_channel(&self) -> &str {
        &self.source_channel
    }

    /// Build the endpoint URL for a given path.
    pub(crate) fn endpoint_url(&self, path: &str) -> String {
        format!("{}{}", self.base_url.trim_end_matches('/'), path)
    }

    /// Build headers for API requests.
    pub(crate) fn build_headers(&self) -> Result<HeaderMap, ZaiError> {
        let mut headers = HeaderMap::new();

        // Authorization
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {}", self.api_key))
                .map_err(|e| ZaiError::Config(format!("Invalid API key: {}", e)))?,
        );

        // Content-Type
        headers.insert(
            CONTENT_TYPE,
            HeaderValue::from_static("application/json; charset=UTF-8"),
        );

        // Accept
        headers.insert(ACCEPT, HeaderValue::from_static("application/json"));

        // Source channel
        headers.insert(
            "x-source-channel",
            HeaderValue::from_str(&self.source_channel)
                .map_err(|e| ZaiError::Config(format!("Invalid source channel: {}", e)))?,
        );

        // Accept-Language (as per Python SDK for Z.AI)
        headers.insert("Accept-Language", HeaderValue::from_static("en-US,en"));

        Ok(headers)
    }

    /// Get the HTTP client.
    pub(crate) fn http_client(&self) -> &reqwest::Client {
        &self.http_client
    }
}

impl std::fmt::Debug for Client {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Client")
            .field("base_url", &self.base_url)
            .field("source_channel", &self.source_channel)
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_new() {
        let client = Client::new("test-api-key");
        assert_eq!(client.api_key(), "test-api-key");
        assert_eq!(client.base_url(), DEFAULT_BASE_URL);
        assert_eq!(client.source_channel(), DEFAULT_SOURCE_CHANNEL);
    }

    #[test]
    fn test_client_with_config() {
        let client = Client::with_config(
            "test-api-key",
            Some("https://custom.api.com".to_string()),
            Some("custom-channel".to_string()),
        );
        assert_eq!(client.base_url(), "https://custom.api.com");
        assert_eq!(client.source_channel(), "custom-channel");
    }

    #[test]
    fn test_endpoint_url() {
        let client = Client::new("test-api-key");
        let url = client.endpoint_url("/chat/completions");
        assert_eq!(url, format!("{}/chat/completions", DEFAULT_BASE_URL));
    }

    #[test]
    fn test_build_headers() {
        let client = Client::new("test-api-key");
        let headers = client.build_headers().unwrap();

        assert!(headers.contains_key(AUTHORIZATION));
        assert!(headers.contains_key(CONTENT_TYPE));
        assert!(headers.contains_key(ACCEPT));
        assert!(headers.contains_key("x-source-channel"));
        assert!(headers.contains_key("Accept-Language"));

        assert_eq!(headers.get(AUTHORIZATION).unwrap(), "Bearer test-api-key");
    }
}
