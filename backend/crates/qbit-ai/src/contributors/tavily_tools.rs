//! Tavily tools prompt contributor.
//!
//! Generates prompt sections documenting the Tavily-powered web tools
//! when they are available (and native web tools are not active).

use qbit_core::{PromptContext, PromptContributor, PromptPriority, PromptSection};

/// Contributor that adds Tavily web tool documentation to the system prompt.
///
/// This contributor activates when:
/// - `has_web_search` is true (Tavily tools are registered)
/// - `has_native_web_tools` is false (not using Claude's built-in web search)
///
/// The documentation helps the LLM understand the full capabilities of each
/// Tavily tool beyond the brief descriptions in ToolDefinition.
pub struct TavilyToolsContributor;

impl PromptContributor for TavilyToolsContributor {
    fn contribute(&self, ctx: &PromptContext) -> Option<Vec<PromptSection>> {
        // Only contribute for Tavily-based search (not native provider tools)
        if !ctx.has_web_search || ctx.has_native_web_tools {
            return None;
        }

        Some(vec![PromptSection::new(
            "tavily_tools",
            PromptPriority::Tools,
            TAVILY_TOOLS_DOCUMENTATION,
        )])
    }

    fn name(&self) -> &str {
        "TavilyToolsContributor"
    }
}

const TAVILY_TOOLS_DOCUMENTATION: &str = r#"## Tavily Web Tools

You have access to Tavily-powered web tools for searching, extracting, and analyzing web content.

### tavily_search
Search the web for information. Returns relevant results with titles, URLs, and content snippets.
- Use for current information, news, documentation, or facts beyond your training data
- Parameters: `query` (required), `max_results`, `search_depth` ("basic" or "advanced"), `topic`, `include_domains`, `exclude_domains`

### tavily_search_answer
Get an AI-generated answer synthesized from web search results.
- Best for direct questions that need a consolidated answer from multiple sources
- Returns both the answer and source citations
- Parameters: `query` (required)

### tavily_extract
Extract and parse content from specific URLs.
- Use to get full page content for deeper analysis when you have specific URLs
- Parameters: `urls` (required, array), `query` (optional focus), `extract_depth`, `format`

### tavily_crawl
Crawl a website starting from a URL, following links to extract content from multiple pages.
- Use for comprehensive site analysis or documentation gathering
- Parameters: `url` (required), `max_depth`, `max_breadth`, `limit`, `instructions`, `allow_external`

### tavily_map
Map the structure of a website, returning a list of discovered URLs.
- Use to discover site structure before crawling or extracting specific pages
- Parameters: `url` (required), `max_depth`, `max_breadth`, `limit`, `instructions`

### Best Practices
- Start with `tavily_search` for broad queries, then use `tavily_extract` for specific URLs
- Use `tavily_map` to discover site structure before targeted crawling
- Prefer `tavily_search_answer` when you need a synthesized response rather than raw results
- Always cite sources when presenting information from web results"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_contributes_when_tavily_active() {
        let contributor = TavilyToolsContributor;

        let ctx = PromptContext::new("anthropic", "claude-sonnet-4")
            .with_web_search(true)
            .with_native_web_tools(false);

        let sections = contributor.contribute(&ctx);
        assert!(sections.is_some());

        let sections = sections.unwrap();
        assert_eq!(sections.len(), 1);
        assert_eq!(sections[0].id, "tavily_tools");
        assert_eq!(sections[0].priority, PromptPriority::Tools);
        assert!(sections[0].content.contains("tavily_search"));
        assert!(sections[0].content.contains("tavily_extract"));
        assert!(sections[0].content.contains("tavily_crawl"));
    }

    #[test]
    fn test_does_not_contribute_when_native_web_tools() {
        let contributor = TavilyToolsContributor;

        // Native web tools take priority - don't show Tavily docs
        let ctx = PromptContext::new("anthropic", "claude-sonnet-4")
            .with_web_search(true)
            .with_native_web_tools(true);

        let sections = contributor.contribute(&ctx);
        assert!(sections.is_none());
    }

    #[test]
    fn test_does_not_contribute_when_no_web_search() {
        let contributor = TavilyToolsContributor;

        let ctx = PromptContext::new("anthropic", "claude-sonnet-4").with_web_search(false);

        let sections = contributor.contribute(&ctx);
        assert!(sections.is_none());
    }
}
