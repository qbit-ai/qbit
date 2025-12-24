//! Z.AI API client implementation.

use crate::completion::CompletionModel;

/// Z.AI Coding Plan API base URL
pub const ZAI_API_BASE_URL: &str = "https://api.z.ai/api/coding/paas/v4";

/// Z.AI API client
#[derive(Clone)]
pub struct Client {
    pub(crate) http_client: reqwest::Client,
    pub(crate) api_key: String,
    pub(crate) base_url: String,
}

impl Client {
    /// Create a new Z.AI client with the given API key.
    ///
    /// Uses the Coding Plan API endpoint by default.
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            http_client: reqwest::Client::new(),
            api_key: api_key.into(),
            base_url: ZAI_API_BASE_URL.to_string(),
        }
    }

    /// Create a new Z.AI client with a custom base URL.
    pub fn with_base_url(api_key: impl Into<String>, base_url: impl Into<String>) -> Self {
        Self {
            http_client: reqwest::Client::new(),
            api_key: api_key.into(),
            base_url: base_url.into(),
        }
    }

    /// Create a completion model for the given model ID.
    ///
    /// # Arguments
    /// * `model` - Model ID (e.g., "GLM-4.7", "GLM-4.5-air")
    ///
    /// # Example
    /// ```ignore
    /// let client = Client::new("your-api-key");
    /// let model = client.completion_model("GLM-4.7");
    /// ```
    pub fn completion_model(&self, model: &str) -> CompletionModel {
        CompletionModel::new(self.clone(), model)
    }
}
