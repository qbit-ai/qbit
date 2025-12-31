//! Provider-specific built-in tools contributor.
//!
//! Generates prompt sections for provider-specific capabilities
//! like Anthropic's built-in web search or OpenAI's code interpreter.

use qbit_core::{PromptContext, PromptContributor, PromptPriority, PromptSection};

/// Contributor that adds provider-specific tool instructions.
///
/// Different LLM providers have different built-in capabilities:
/// - Anthropic: web_search (built into Claude)
/// - OpenAI: code_interpreter, file_search (assistants API)
/// - etc.
///
/// This contributor detects the active provider and adds relevant
/// instructions for using provider-specific features.
pub struct ProviderBuiltinToolsContributor;

impl PromptContributor for ProviderBuiltinToolsContributor {
    fn contribute(&self, ctx: &PromptContext) -> Option<Vec<PromptSection>> {
        let content = match ctx.provider.as_str() {
            // Anthropic (direct API or via Vertex AI)
            "anthropic" | "anthropic_vertex" | "vertex_ai" => {
                if ctx.has_web_search {
                    Some(ANTHROPIC_WEB_SEARCH_INSTRUCTIONS.to_string())
                } else {
                    None
                }
            }

            // OpenAI with Responses API
            "openai" | "openai_responses" => {
                let mut sections = Vec::new();

                if ctx.has_web_search {
                    sections.push(OPENAI_WEB_SEARCH_INSTRUCTIONS.to_string());
                }

                // OpenAI may have code interpreter in the future
                // if ctx.available_tools.contains(&"code_interpreter".to_string()) {
                //     sections.push(OPENAI_CODE_INTERPRETER_INSTRUCTIONS.to_string());
                // }

                if sections.is_empty() {
                    None
                } else {
                    Some(sections.join("\n\n"))
                }
            }

            // Gemini
            "gemini" => {
                if ctx.has_web_search {
                    Some(GEMINI_WEB_SEARCH_INSTRUCTIONS.to_string())
                } else {
                    None
                }
            }

            // Other providers - no special instructions
            _ => None,
        };

        content.map(|c| {
            vec![PromptSection::new(
                "provider_builtin_tools",
                PromptPriority::Provider,
                c,
            )]
        })
    }

    fn name(&self) -> &str {
        "ProviderBuiltinToolsContributor"
    }
}

// =============================================================================
// Provider-specific instruction templates
// =============================================================================

const ANTHROPIC_WEB_SEARCH_INSTRUCTIONS: &str = r#"## Web Search (Anthropic Built-in)

You have access to web search capabilities. When searching:
- Use specific, targeted queries for best results
- Cite sources when presenting information from search results
- Search results are automatically fetched and included in context"#;

const OPENAI_WEB_SEARCH_INSTRUCTIONS: &str = r#"## Web Search (OpenAI Built-in)

You have access to web search via the Responses API. When searching:
- Formulate clear, specific queries
- Results are integrated into your response context
- Always cite sources when using web information"#;

const GEMINI_WEB_SEARCH_INSTRUCTIONS: &str = r#"## Web Search (Gemini Grounding)

You have access to Google Search grounding. When using search:
- Queries are automatically grounded with real-time web information
- Cite sources when presenting factual claims
- Use search for current events and recent information"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_anthropic_web_search() {
        let contributor = ProviderBuiltinToolsContributor;

        let ctx = PromptContext::new("anthropic", "claude-sonnet-4").with_web_search(true);

        let sections = contributor.contribute(&ctx);
        assert!(sections.is_some());

        let sections = sections.unwrap();
        assert_eq!(sections.len(), 1);
        assert_eq!(sections[0].priority, PromptPriority::Provider);
        assert!(sections[0].content.contains("Anthropic Built-in"));
    }

    #[test]
    fn test_anthropic_no_web_search() {
        let contributor = ProviderBuiltinToolsContributor;

        let ctx = PromptContext::new("anthropic", "claude-sonnet-4").with_web_search(false);

        let sections = contributor.contribute(&ctx);
        assert!(sections.is_none());
    }

    #[test]
    fn test_vertex_ai_provider() {
        let contributor = ProviderBuiltinToolsContributor;

        let ctx = PromptContext::new("vertex_ai", "claude-sonnet-4").with_web_search(true);

        let sections = contributor.contribute(&ctx);
        assert!(sections.is_some());
        assert!(sections.unwrap()[0].content.contains("Anthropic Built-in"));
    }

    #[test]
    fn test_openai_web_search() {
        let contributor = ProviderBuiltinToolsContributor;

        let ctx = PromptContext::new("openai", "gpt-4").with_web_search(true);

        let sections = contributor.contribute(&ctx);
        assert!(sections.is_some());
        assert!(sections.unwrap()[0].content.contains("OpenAI Built-in"));
    }

    #[test]
    fn test_gemini_web_search() {
        let contributor = ProviderBuiltinToolsContributor;

        let ctx = PromptContext::new("gemini", "gemini-pro").with_web_search(true);

        let sections = contributor.contribute(&ctx);
        assert!(sections.is_some());
        assert!(sections.unwrap()[0].content.contains("Gemini Grounding"));
    }

    #[test]
    fn test_unknown_provider() {
        let contributor = ProviderBuiltinToolsContributor;

        let ctx = PromptContext::new("unknown", "some-model").with_web_search(true);

        let sections = contributor.contribute(&ctx);
        assert!(sections.is_none());
    }
}
