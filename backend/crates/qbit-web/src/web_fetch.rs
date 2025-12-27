//! Custom web fetch with readability-based content extraction
//!
//! Fetches web pages and extracts the main content using Mozilla's readability algorithm.

use anyhow::Result;
use reqwest::Client;
use std::io::Cursor;
use std::time::Duration;
use url::Url;

/// Web fetch client with content extraction
pub struct WebFetcher {
    client: Client,
}

impl WebFetcher {
    /// Create a new WebFetcher
    pub fn new() -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .unwrap_or_else(|_| Client::new());

        Self { client }
    }

    /// Fetch a single URL and extract its main content
    pub async fn fetch(&self, url: &str) -> Result<FetchResult> {
        // Fetch the page
        let response = self
            .client
            .get(url)
            .header(
                "User-Agent",
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
            )
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to fetch {}: {}", url, e))?;

        let html = response
            .text()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to read response body: {}", e))?;

        // Extract content using readability
        let content = extract_content(&html, url)?;

        Ok(FetchResult {
            url: url.to_string(),
            content,
        })
    }
}

impl Default for WebFetcher {
    fn default() -> Self {
        Self::new()
    }
}

/// Extract main content from HTML using readability algorithm
fn extract_content(html: &str, url: &str) -> Result<String> {
    // First, try the readability algorithm
    let parsed_url = match Url::parse(url) {
        Ok(u) => u,
        Err(e) => {
            tracing::debug!("Failed to parse URL {}: {}", url, e);
            // Fallback if URL parsing fails
            return Ok(extract_text_fallback(html));
        }
    };

    // Create a cursor from the HTML string
    let mut cursor = Cursor::new(html.as_bytes());

    // Extract content using readability
    match readability::extractor::extract(&mut cursor, &parsed_url) {
        Ok(product) => {
            tracing::debug!("Readability extraction successful for {}", url);
            // Return the extracted text content (cleaned of HTML)
            if product.text.is_empty() {
                // If text is empty, fallback to content (which is HTML)
                Ok(extract_text_fallback(&product.content))
            } else {
                Ok(product.text)
            }
        }
        Err(e) => {
            tracing::debug!(
                "Readability extraction failed for {}: {}, falling back to simple extraction",
                url,
                e
            );
            // Fallback to simple text extraction
            Ok(extract_text_fallback(html))
        }
    }
}

/// Fallback text extraction if readability fails
/// Strips basic HTML tags and returns text content
fn extract_text_fallback(html: &str) -> String {
    // Simple HTML tag stripping
    let mut result = String::new();
    let mut in_tag = false;

    for char in html.chars() {
        match char {
            '<' => {
                in_tag = true;
            }
            '>' => {
                in_tag = false;
                result.push(' ');
            }
            _ if !in_tag => {
                result.push(char);
            }
            _ => {
                // Inside a tag, skip
            }
        }
    }

    // Clean up whitespace and remove multiple spaces
    result.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Result of fetching a single URL
#[derive(Debug, Clone)]
pub struct FetchResult {
    pub url: String,
    pub content: String,
}
