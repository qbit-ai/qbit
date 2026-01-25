//! Tool implementations for Tavily web search.
//!
//! These tools implement the `qbit_core::Tool` trait for integration
//! with the Qbit tool registry.

use std::path::Path;
use std::sync::Arc;

use anyhow::Result;
use qbit_core::Tool;
use serde_json::{json, Value};

use crate::tavily::TavilyState;

/// Get a required string argument from JSON.
fn get_required_str<'a>(args: &'a Value, key: &str) -> Result<&'a str, Value> {
    args.get(key)
        .and_then(|v| v.as_str())
        .ok_or_else(|| json!({"error": format!("Missing required argument: {}", key)}))
}

/// Get an optional string argument from JSON.
#[allow(dead_code)]
fn get_optional_str<'a>(args: &'a Value, key: &str) -> Option<&'a str> {
    args.get(key).and_then(|v| v.as_str())
}

/// Get an optional integer argument from JSON.
fn get_optional_usize(args: &Value, key: &str) -> Option<usize> {
    args.get(key).and_then(|v| v.as_u64()).map(|n| n as usize)
}

/// Get an optional u32 argument from JSON.
fn get_optional_u32(args: &Value, key: &str) -> Option<u32> {
    args.get(key).and_then(|v| v.as_u64()).map(|n| n as u32)
}

// ============================================================================
// web_search
// ============================================================================

/// Web search tool using Tavily API.
pub struct WebSearchTool {
    tavily: Arc<TavilyState>,
}

impl WebSearchTool {
    /// Create a new WebSearchTool with the given TavilyState.
    pub fn new(tavily: Arc<TavilyState>) -> Self {
        Self { tavily }
    }
}

#[async_trait::async_trait]
impl Tool for WebSearchTool {
    fn name(&self) -> &'static str {
        "tavily_search"
    }

    fn description(&self) -> &'static str {
        "Search the web for information. Returns relevant results with titles, URLs, and content snippets. \
         Use this when you need current information, news, documentation, or facts beyond your training data."
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "The search query"
                },
                "max_results": {
                    "type": "integer",
                    "description": "Maximum number of results to return (default: 5)"
                },
                "search_depth": {
                    "type": "string",
                    "enum": ["basic", "advanced"],
                    "description": "Search depth: 'basic' for quick results, 'advanced' for comprehensive search (default: basic)"
                },
                "topic": {
                    "type": "string",
                    "description": "Search topic category like 'general', 'news', etc."
                },
                "include_domains": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "List of domains to include in search results"
                },
                "exclude_domains": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "List of domains to exclude from search results"
                }
            },
            "required": ["query"]
        })
    }

    async fn execute(&self, args: Value, _workspace: &Path) -> Result<Value> {
        let query = match get_required_str(&args, "query") {
            Ok(q) => q,
            Err(e) => return Ok(e),
        };

        let max_results = get_optional_usize(&args, "max_results");

        match self.tavily.search(query, max_results).await {
            Ok(results) => Ok(json!({
                "query": results.query,
                "results": results.results.iter().map(|r| json!({
                    "title": r.title,
                    "url": r.url,
                    "content": r.content,
                    "score": r.score
                })).collect::<Vec<_>>(),
                "answer": results.answer,
                "count": results.results.len()
            })),
            Err(e) => Ok(json!({"error": e.to_string()})),
        }
    }
}

// ============================================================================
// web_search_answer
// ============================================================================

/// Web search tool that returns an AI-generated answer using Tavily API.
pub struct WebSearchAnswerTool {
    tavily: Arc<TavilyState>,
}

impl WebSearchAnswerTool {
    /// Create a new WebSearchAnswerTool with the given TavilyState.
    pub fn new(tavily: Arc<TavilyState>) -> Self {
        Self { tavily }
    }
}

#[async_trait::async_trait]
impl Tool for WebSearchAnswerTool {
    fn name(&self) -> &'static str {
        "tavily_search_answer"
    }

    fn description(&self) -> &'static str {
        "Get an AI-generated answer from web search results. \
         Best for direct questions that need a synthesized answer from multiple sources."
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "The question to answer"
                }
            },
            "required": ["query"]
        })
    }

    async fn execute(&self, args: Value, _workspace: &Path) -> Result<Value> {
        let query = match get_required_str(&args, "query") {
            Ok(q) => q,
            Err(e) => return Ok(e),
        };

        match self.tavily.answer(query).await {
            Ok(result) => Ok(json!({
                "query": result.query,
                "answer": result.answer,
                "sources": result.sources.iter().map(|r| json!({
                    "title": r.title,
                    "url": r.url,
                    "content": r.content,
                    "score": r.score
                })).collect::<Vec<_>>()
            })),
            Err(e) => Ok(json!({"error": e.to_string()})),
        }
    }
}

// ============================================================================
// web_extract
// ============================================================================

/// Web content extraction tool using Tavily API.
pub struct WebExtractTool {
    tavily: Arc<TavilyState>,
}

impl WebExtractTool {
    /// Create a new WebExtractTool with the given TavilyState.
    pub fn new(tavily: Arc<TavilyState>) -> Self {
        Self { tavily }
    }
}

#[async_trait::async_trait]
impl Tool for WebExtractTool {
    fn name(&self) -> &'static str {
        "tavily_extract"
    }

    fn description(&self) -> &'static str {
        "Extract and parse content from specific URLs. \
         Use this to get the full content of web pages for deeper analysis."
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "urls": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "List of URLs to extract content from"
                },
                "query": {
                    "type": "string",
                    "description": "Optional query to focus extraction on specific information"
                },
                "extract_depth": {
                    "type": "string",
                    "enum": ["basic", "advanced"],
                    "description": "Extraction depth: 'basic' for quick extraction, 'advanced' for comprehensive extraction"
                },
                "format": {
                    "type": "string",
                    "enum": ["markdown", "text"],
                    "description": "Output format for extracted content (default: markdown)"
                }
            },
            "required": ["urls"]
        })
    }

    async fn execute(&self, args: Value, _workspace: &Path) -> Result<Value> {
        let urls: Vec<String> = args
            .get("urls")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();

        if urls.is_empty() {
            return Ok(json!({"error": "Missing required argument: urls"}));
        }

        match self.tavily.extract(urls).await {
            Ok(results) => Ok(json!({
                "results": results.results.iter().map(|r| json!({
                    "url": r.url,
                    "content": r.raw_content
                })).collect::<Vec<_>>(),
                "failed_urls": results.failed_urls,
                "count": results.results.len()
            })),
            Err(e) => Ok(json!({"error": e.to_string()})),
        }
    }
}

// ============================================================================
// web_crawl
// ============================================================================

/// Web crawling tool using Tavily API.
pub struct WebCrawlTool {
    tavily: Arc<TavilyState>,
}

impl WebCrawlTool {
    /// Create a new WebCrawlTool with the given TavilyState.
    pub fn new(tavily: Arc<TavilyState>) -> Self {
        Self { tavily }
    }
}

#[async_trait::async_trait]
impl Tool for WebCrawlTool {
    fn name(&self) -> &'static str {
        "tavily_crawl"
    }

    fn description(&self) -> &'static str {
        "Crawl a website starting from a URL, following links to extract content from multiple pages. \
         Use for comprehensive site analysis."
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "url": {
                    "type": "string",
                    "description": "Base URL to start crawling from"
                },
                "max_depth": {
                    "type": "integer",
                    "description": "Maximum crawl depth (levels of links to follow)"
                },
                "max_breadth": {
                    "type": "integer",
                    "description": "Maximum pages to crawl per level"
                },
                "limit": {
                    "type": "integer",
                    "description": "Maximum total pages to crawl"
                },
                "instructions": {
                    "type": "string",
                    "description": "Natural language instructions for what to focus on during crawling"
                },
                "allow_external": {
                    "type": "boolean",
                    "description": "Whether to follow external links outside the base domain"
                }
            },
            "required": ["url"]
        })
    }

    async fn execute(&self, args: Value, _workspace: &Path) -> Result<Value> {
        let url = match get_required_str(&args, "url") {
            Ok(u) => u.to_string(),
            Err(e) => return Ok(e),
        };

        let max_depth = get_optional_u32(&args, "max_depth");

        match self.tavily.crawl(url, max_depth).await {
            Ok(results) => Ok(json!({
                "results": results.results.iter().map(|r| json!({
                    "url": r.url,
                    "content": r.raw_content
                })).collect::<Vec<_>>(),
                "failed_urls": results.failed_urls,
                "count": results.results.len()
            })),
            Err(e) => Ok(json!({"error": e.to_string()})),
        }
    }
}

// ============================================================================
// web_map
// ============================================================================

/// Website structure mapping tool using Tavily API.
pub struct WebMapTool {
    tavily: Arc<TavilyState>,
}

impl WebMapTool {
    /// Create a new WebMapTool with the given TavilyState.
    pub fn new(tavily: Arc<TavilyState>) -> Self {
        Self { tavily }
    }
}

#[async_trait::async_trait]
impl Tool for WebMapTool {
    fn name(&self) -> &'static str {
        "tavily_map"
    }

    fn description(&self) -> &'static str {
        "Map the structure of a website, returning a list of URLs found. \
         Use to discover site structure before crawling or extracting specific pages."
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "url": {
                    "type": "string",
                    "description": "Base URL to map"
                },
                "max_depth": {
                    "type": "integer",
                    "description": "Maximum depth to explore"
                },
                "max_breadth": {
                    "type": "integer",
                    "description": "Maximum links to follow per level"
                },
                "limit": {
                    "type": "integer",
                    "description": "Maximum URLs to return"
                },
                "instructions": {
                    "type": "string",
                    "description": "Natural language instructions for what to focus on during mapping"
                }
            },
            "required": ["url"]
        })
    }

    async fn execute(&self, args: Value, _workspace: &Path) -> Result<Value> {
        let url = match get_required_str(&args, "url") {
            Ok(u) => u.to_string(),
            Err(e) => return Ok(e),
        };

        let max_depth = get_optional_u32(&args, "max_depth");

        match self.tavily.map(url, max_depth).await {
            Ok(results) => Ok(json!({
                "urls": results.urls,
                "base_url": results.base_url,
                "count": results.urls.len()
            })),
            Err(e) => Ok(json!({"error": e.to_string()})),
        }
    }
}

// ============================================================================
// Helper functions for tool registration
// ============================================================================

/// Create all Tavily tools with shared state.
/// Tools are registered even if API key is missing; errors occur at execution time.
pub fn create_tavily_tools(tavily: Arc<TavilyState>) -> Vec<std::sync::Arc<dyn Tool>> {
    vec![
        std::sync::Arc::new(WebSearchTool::new(tavily.clone())),
        std::sync::Arc::new(WebSearchAnswerTool::new(tavily.clone())),
        std::sync::Arc::new(WebExtractTool::new(tavily.clone())),
        std::sync::Arc::new(WebCrawlTool::new(tavily.clone())),
        std::sync::Arc::new(WebMapTool::new(tavily)),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_web_search_tool_metadata() {
        let tavily = Arc::new(TavilyState::new());
        let tool = WebSearchTool::new(tavily);

        assert_eq!(tool.name(), "tavily_search");
        assert!(!tool.description().is_empty());

        let params = tool.parameters();
        assert_eq!(params["type"], "object");
        assert!(params["properties"]["query"].is_object());
        assert!(params["required"]
            .as_array()
            .unwrap()
            .contains(&json!("query")));
    }

    #[test]
    fn test_web_search_answer_tool_metadata() {
        let tavily = Arc::new(TavilyState::new());
        let tool = WebSearchAnswerTool::new(tavily);

        assert_eq!(tool.name(), "tavily_search_answer");
        assert!(!tool.description().is_empty());
    }

    #[test]
    fn test_web_extract_tool_metadata() {
        let tavily = Arc::new(TavilyState::new());
        let tool = WebExtractTool::new(tavily);

        assert_eq!(tool.name(), "tavily_extract");
        assert!(!tool.description().is_empty());

        let params = tool.parameters();
        assert!(params["properties"]["urls"].is_object());
    }

    #[test]
    fn test_web_crawl_tool_metadata() {
        let tavily = Arc::new(TavilyState::new());
        let tool = WebCrawlTool::new(tavily);

        assert_eq!(tool.name(), "tavily_crawl");
        assert!(!tool.description().is_empty());

        let params = tool.parameters();
        assert!(params["properties"]["url"].is_object());
    }

    #[test]
    fn test_web_map_tool_metadata() {
        let tavily = Arc::new(TavilyState::new());
        let tool = WebMapTool::new(tavily);

        assert_eq!(tool.name(), "tavily_map");
        assert!(!tool.description().is_empty());

        let params = tool.parameters();
        assert!(params["properties"]["url"].is_object());
    }

    #[test]
    fn test_create_tavily_tools_always_returns_all_tools() {
        let tavily = Arc::new(TavilyState::from_api_key(None));
        let tools = create_tavily_tools(tavily);

        let names: Vec<String> = tools.iter().map(|t| t.name().to_string()).collect();
        assert!(names.contains(&"tavily_search".to_string()));
        assert!(names.contains(&"tavily_search_answer".to_string()));
        assert!(names.contains(&"tavily_extract".to_string()));
        assert!(names.contains(&"tavily_crawl".to_string()));
        assert!(names.contains(&"tavily_map".to_string()));
    }
}
