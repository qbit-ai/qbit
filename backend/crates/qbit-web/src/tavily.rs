//! Tavily web search integration
//!
//! Provides web search capabilities for the AI agent using Tavily's API.
//! Supports configuration via settings file with environment variable fallback.

use anyhow::Result;
use serde::{Deserialize, Serialize};

const TAVILY_BASE_URL: &str = "https://api.tavily.com";

/// Manages the Tavily API key state and HTTP client
pub struct TavilyState {
    /// The API key (None if not configured)
    api_key: Option<String>,
    /// HTTP client for API calls
    client: reqwest::Client,
}

impl TavilyState {
    /// Create a new TavilyState with an optional API key.
    pub fn from_api_key(api_key: Option<String>) -> Self {
        if api_key.is_some() {
            tracing::info!("Tavily web search tools enabled");
        } else {
            tracing::debug!(
                "Tavily API key not configured, web search will fail at execution time"
            );
        }

        Self {
            api_key,
            client: reqwest::Client::new(),
        }
    }

    /// Create a new TavilyState, checking for TAVILY_API_KEY from environment.
    /// This is the legacy constructor for backward compatibility.
    #[deprecated(note = "Use from_api_key instead")]
    pub fn new() -> Self {
        let api_key = std::env::var("TAVILY_API_KEY")
            .ok()
            .filter(|k| !k.is_empty());
        Self::from_api_key(api_key)
    }

    /// Get the API key
    fn get_api_key(&self) -> Result<&str> {
        self.api_key.as_deref().ok_or_else(|| {
            anyhow::anyhow!(
                "Tavily API key not configured. Set api_keys.tavily in ~/.qbit/settings.toml"
            )
        })
    }

    /// Helper method to make POST requests to Tavily API
    async fn post_json<TReq: Serialize, TResp: for<'de> Deserialize<'de>>(
        &self,
        endpoint: &str,
        req: &TReq,
    ) -> Result<TResp> {
        let api_key = self.get_api_key()?;
        let url = format!("{}{}", TAVILY_BASE_URL, endpoint);

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(req)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to send request: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!(
                "API request failed with status {}: {}",
                status,
                error_text
            ));
        }

        response
            .json()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to parse response: {}", e))
    }

    /// Perform a web search
    pub async fn search(&self, query: &str, max_results: Option<usize>) -> Result<SearchResults> {
        let request = TavilySearchRequest {
            api_key: self.get_api_key()?.to_string(),
            query: query.to_string(),
            search_depth: Some("basic".to_string()),
            chunks_per_source: None,
            max_results: max_results.map(|n| n as u32),
            topic: None,
            time_range: None,
            start_date: None,
            end_date: None,
            include_answer: Some(true),
            include_raw_content: Some(false),
            include_images: Some(false),
            include_image_descriptions: None,
            include_favicon: None,
            include_domains: None,
            exclude_domains: None,
            country: None,
            auto_parameters: None,
            include_usage: None,
        };

        let response: TavilySearchResponse = self.post_json("/search", &request).await?;

        Ok(SearchResults {
            query: response.query,
            results: response
                .results
                .into_iter()
                .map(|r| SearchResult {
                    title: r.title,
                    url: r.url,
                    content: r.content,
                    score: r.score,
                })
                .collect(),
            answer: response.answer,
        })
    }

    /// Get an AI-generated answer for a query (search with answer included)
    pub async fn answer(&self, query: &str) -> Result<AnswerResult> {
        let request = TavilySearchRequest {
            api_key: self.get_api_key()?.to_string(),
            query: query.to_string(),
            search_depth: Some("advanced".to_string()),
            chunks_per_source: None,
            max_results: Some(5),
            topic: None,
            time_range: None,
            start_date: None,
            end_date: None,
            include_answer: Some(true),
            include_raw_content: Some(false),
            include_images: Some(false),
            include_image_descriptions: None,
            include_favicon: None,
            include_domains: None,
            exclude_domains: None,
            country: None,
            auto_parameters: None,
            include_usage: None,
        };

        let response: TavilySearchResponse = self.post_json("/search", &request).await?;

        Ok(AnswerResult {
            query: response.query,
            answer: response.answer.unwrap_or_default(),
            sources: response
                .results
                .into_iter()
                .take(5)
                .map(|r| SearchResult {
                    title: r.title,
                    url: r.url,
                    content: r.content,
                    score: r.score,
                })
                .collect(),
        })
    }

    /// Extract content from URLs using the real /extract endpoint
    pub async fn extract(&self, urls: Vec<String>) -> Result<ExtractResults> {
        let request = TavilyExtractRequest {
            api_key: self.get_api_key()?.to_string(),
            urls: TavilyUrls::Array(urls),
            query: None,
            chunks_per_source: None,
            extract_depth: None,
            include_images: None,
            include_favicon: None,
            format: None,
            timeout: None,
            include_usage: None,
        };

        let response: TavilyExtractResponse = self.post_json("/extract", &request).await?;

        Ok(ExtractResults {
            results: response
                .results
                .into_iter()
                .map(|r| ExtractResult {
                    url: r.url,
                    raw_content: r.raw_content,
                })
                .collect(),
            failed_urls: response.failed_results.into_iter().map(|f| f.url).collect(),
        })
    }

    /// Crawl a website and extract content
    pub async fn crawl(&self, url: String, max_depth: Option<u32>) -> Result<CrawlResults> {
        let request = TavilyCrawlRequest {
            api_key: self.get_api_key()?.to_string(),
            url,
            instructions: None,
            chunks_per_source: None,
            max_depth,
            max_breadth: None,
            limit: None,
            select_paths: None,
            select_domains: None,
            exclude_paths: None,
            exclude_domains: None,
            allow_external: None,
            include_images: None,
            extract_depth: None,
            format: None,
            include_favicon: None,
            timeout: None,
            include_usage: None,
        };

        let response: TavilyCrawlResponse = self.post_json("/crawl", &request).await?;

        Ok(CrawlResults {
            results: response
                .results
                .into_iter()
                .map(|r| CrawlResult {
                    url: r.url,
                    raw_content: r.raw_content,
                })
                .collect(),
            failed_urls: response.failed_results.into_iter().map(|f| f.url).collect(),
        })
    }

    /// Map a website's structure
    pub async fn map(&self, url: String, max_depth: Option<u32>) -> Result<MapResults> {
        let request = TavilyMapRequest {
            api_key: self.get_api_key()?.to_string(),
            url: url.clone(),
            instructions: None,
            max_depth,
            max_breadth: None,
            limit: None,
            select_paths: None,
            select_domains: None,
            exclude_paths: None,
            exclude_domains: None,
            allow_external: None,
            timeout: None,
            include_usage: None,
        };

        let response: TavilyMapResponse = self.post_json("/map", &request).await?;

        Ok(MapResults {
            urls: response.urls,
            base_url: response.base_url,
        })
    }
}

impl Default for TavilyState {
    fn default() -> Self {
        Self::from_api_key(None)
    }
}

// ============================================================================
// Request Types
// ============================================================================

#[derive(Debug, Serialize)]
struct TavilySearchRequest {
    api_key: String,
    query: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    search_depth: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    chunks_per_source: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_results: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    topic: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    time_range: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    start_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    end_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    include_answer: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    include_raw_content: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    include_images: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    include_image_descriptions: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    include_favicon: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    include_domains: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    exclude_domains: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    country: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    auto_parameters: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    include_usage: Option<bool>,
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
#[allow(dead_code)] // Single variant kept for API completeness
enum TavilyUrls {
    Single(String),
    Array(Vec<String>),
}

#[derive(Debug, Serialize)]
struct TavilyExtractRequest {
    api_key: String,
    urls: TavilyUrls,
    #[serde(skip_serializing_if = "Option::is_none")]
    query: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    chunks_per_source: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    extract_depth: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    include_images: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    include_favicon: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    format: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    timeout: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    include_usage: Option<bool>,
}

#[derive(Debug, Serialize)]
struct TavilyCrawlRequest {
    api_key: String,
    url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    instructions: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    chunks_per_source: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_depth: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_breadth: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    limit: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    select_paths: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    select_domains: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    exclude_paths: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    exclude_domains: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    allow_external: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    include_images: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    extract_depth: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    format: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    include_favicon: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    timeout: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    include_usage: Option<bool>,
}

#[derive(Debug, Serialize)]
struct TavilyMapRequest {
    api_key: String,
    url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    instructions: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_depth: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_breadth: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    limit: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    select_paths: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    select_domains: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    exclude_paths: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    exclude_domains: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    allow_external: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    timeout: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    include_usage: Option<bool>,
}

// ============================================================================
// Response Types (Internal - from Tavily API)
// Fields marked dead_code are kept for API completeness and debugging
// ============================================================================

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct TavilySearchResponse {
    query: String,
    #[serde(default)]
    answer: Option<String>,
    results: Vec<TavilySearchResult>,
    #[serde(default)]
    images: Vec<String>,
    #[serde(default)]
    usage: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct TavilySearchResult {
    title: String,
    url: String,
    content: String,
    score: f64,
    #[serde(default)]
    raw_content: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TavilyExtractResponse {
    results: Vec<TavilyExtractResult>,
    #[serde(default)]
    failed_results: Vec<TavilyFailedResult>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct TavilyExtractResult {
    url: String,
    raw_content: String,
    #[serde(default)]
    images: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct TavilyCrawlResponse {
    results: Vec<TavilyCrawlResult>,
    #[serde(default)]
    failed_results: Vec<TavilyFailedResult>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct TavilyCrawlResult {
    url: String,
    raw_content: String,
    #[serde(default)]
    images: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct TavilyMapResponse {
    urls: Vec<String>,
    base_url: String,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct TavilyFailedResult {
    url: String,
    #[serde(default)]
    error: Option<String>,
}

// ============================================================================
// Public Result Types (for backward compatibility)
// ============================================================================

/// A single search result
#[derive(Debug, Clone)]
pub struct SearchResult {
    pub title: String,
    pub url: String,
    pub content: String,
    pub score: f64,
}

/// Search results container
#[derive(Debug)]
pub struct SearchResults {
    pub query: String,
    pub results: Vec<SearchResult>,
    pub answer: Option<String>,
}

/// Answer result with sources
#[derive(Debug)]
pub struct AnswerResult {
    pub query: String,
    pub answer: String,
    pub sources: Vec<SearchResult>,
}

/// A single extracted URL result
#[derive(Debug, Clone)]
pub struct ExtractResult {
    pub url: String,
    pub raw_content: String,
}

/// Extract results container
#[derive(Debug)]
pub struct ExtractResults {
    pub results: Vec<ExtractResult>,
    pub failed_urls: Vec<String>,
}

/// A single crawled URL result
#[derive(Debug, Clone)]
pub struct CrawlResult {
    pub url: String,
    pub raw_content: String,
}

/// Crawl results container
#[derive(Debug)]
pub struct CrawlResults {
    pub results: Vec<CrawlResult>,
    pub failed_urls: Vec<String>,
}

/// Map results container
#[derive(Debug)]
pub struct MapResults {
    pub urls: Vec<String>,
    pub base_url: String,
}
