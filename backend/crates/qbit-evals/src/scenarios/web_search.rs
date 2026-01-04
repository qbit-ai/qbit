//! Web search evaluation scenario.
//!
//! Tests Claude's native web search capability (web_search_20250305).
//! This scenario verifies that the agent can use server-side web tools
//! to retrieve current information.
//!
//! Note: This scenario only supports Claude and OpenAI providers.
//! Z.AI does not support native web search capabilities.

use async_trait::async_trait;

use crate::config::EvalProvider;
use crate::metrics::{LlmJudgeMetric, Metric};
use crate::scenarios::Scenario;

/// Scenario: Use native web search to answer questions requiring current information.
pub struct WebSearchScenario;

#[async_trait]
impl Scenario for WebSearchScenario {
    fn name(&self) -> &str {
        "web-search"
    }

    fn description(&self) -> &str {
        "Use Claude's native web search to find current information"
    }

    fn testbed(&self) -> &str {
        "minimal"
    }

    fn prompt(&self) -> &str {
        // Ask a question that requires web search to answer accurately
        // This should trigger Claude's native web_search tool
        "Search the web to find and summarize the latest Rust programming language \
         release version and its key features. Include the version number and at \
         least 2-3 notable features or changes in this release."
    }

    fn system_prompt(&self) -> Option<&str> {
        // Override the default eval system prompt to encourage web search usage
        Some(
            r#"You are an AI assistant with access to web search capabilities.
When asked about current information, recent events, or topics that may have
changed since your training, use your web search tool to find accurate, up-to-date
information.

For this task:
1. Use web search to find the information requested
2. Provide a clear, factual summary based on the search results
3. Include specific details like version numbers or dates when available

Complete the task efficiently and provide accurate information."#,
        )
    }

    fn metrics(&self) -> Vec<Box<dyn Metric>> {
        vec![
            Box::new(LlmJudgeMetric::new(
                "contains_version",
                "Response contains a specific Rust version number (e.g., 1.XX.X format)",
                0.8,
            )),
            Box::new(LlmJudgeMetric::new(
                "contains_features",
                "Response describes at least 2 specific features or changes in the Rust release",
                0.7,
            )),
        ]
    }

    fn supports_provider(&self, provider: EvalProvider) -> bool {
        // Web search is only available for Claude and OpenAI
        // Z.AI does not support native web search capabilities
        !matches!(provider, EvalProvider::Zai)
    }
}

/// Testbed files for the web-search scenario (minimal - no files needed).
pub fn testbed_files() -> Vec<(String, String)> {
    vec![(
        "Cargo.toml".to_string(),
        r#"[package]
name = "web-search-testbed"
version = "0.1.0"
edition = "2021"

[dependencies]
"#
        .to_string(),
    )]
}
