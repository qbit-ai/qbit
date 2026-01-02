//! Dynamic system prompt composition.
//!
//! This module provides a trait-based system for dynamically composing
//! system prompts from multiple contributors (tools, features, providers).

/// Priority for ordering prompt sections.
///
/// Lower values appear earlier in the composed prompt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PromptPriority {
    /// Core identity and environment (0-99)
    Core = 0,
    /// Workflow and behavior rules (100-199)
    Workflow = 100,
    /// Tool documentation (200-299)
    Tools = 200,
    /// Feature-specific instructions (300-399)
    Features = 300,
    /// Provider-specific instructions (400-499)
    Provider = 400,
    /// Dynamic runtime context (500+)
    Context = 500,
}

/// Context passed to prompt contributors for conditional generation.
#[derive(Debug, Clone, Default)]
pub struct PromptContext {
    /// Current LLM provider (e.g., "anthropic", "openai", "vertex_ai")
    pub provider: String,
    /// Model name (e.g., "claude-sonnet-4-20250514")
    pub model: String,
    /// Available tool names (already assembled)
    pub available_tools: Vec<String>,
    /// Whether web search is available (Tavily or provider-specific)
    pub has_web_search: bool,
    /// Whether Claude's native web tools are enabled (web_search_20250305, web_fetch_20250910)
    /// These are server-side tools that Claude executes automatically
    pub has_native_web_tools: bool,
    /// Whether sub-agents are available (depth check passed)
    pub has_sub_agents: bool,
    /// Current workspace path
    pub workspace: Option<String>,
}

impl PromptContext {
    /// Create a new prompt context with the given provider and model.
    pub fn new(provider: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            provider: provider.into(),
            model: model.into(),
            ..Default::default()
        }
    }

    /// Set available tools.
    pub fn with_tools(mut self, tools: Vec<String>) -> Self {
        self.available_tools = tools;
        self
    }

    /// Set web search availability (Tavily or provider-specific).
    pub fn with_web_search(mut self, available: bool) -> Self {
        self.has_web_search = available;
        self
    }

    /// Set native web tools availability (Claude's web_search_20250305 and web_fetch_20250910).
    pub fn with_native_web_tools(mut self, available: bool) -> Self {
        self.has_native_web_tools = available;
        self
    }

    /// Set sub-agent availability.
    pub fn with_sub_agents(mut self, available: bool) -> Self {
        self.has_sub_agents = available;
        self
    }

    /// Set workspace path.
    pub fn with_workspace(mut self, path: impl Into<String>) -> Self {
        self.workspace = Some(path.into());
        self
    }
}

/// A section contributed to the system prompt.
#[derive(Debug, Clone)]
pub struct PromptSection {
    /// Section identifier (for debugging/logging)
    pub id: String,
    /// Priority for ordering
    pub priority: PromptPriority,
    /// The actual content to include
    pub content: String,
}

impl PromptSection {
    /// Create a new prompt section.
    pub fn new(
        id: impl Into<String>,
        priority: PromptPriority,
        content: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            priority,
            content: content.into(),
        }
    }
}

/// Trait for components that contribute to the system prompt.
///
/// Implementors can dynamically generate prompt sections based on the
/// current context (provider, available tools, features, etc.).
pub trait PromptContributor: Send + Sync {
    /// Generate prompt section(s) based on current context.
    ///
    /// Returns `None` if this contributor has nothing to add for the
    /// given context.
    fn contribute(&self, ctx: &PromptContext) -> Option<Vec<PromptSection>>;

    /// Returns a human-readable name for this contributor (for logging).
    fn name(&self) -> &str {
        std::any::type_name::<Self>()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestContributor;

    impl PromptContributor for TestContributor {
        fn contribute(&self, ctx: &PromptContext) -> Option<Vec<PromptSection>> {
            if ctx.has_web_search {
                Some(vec![PromptSection::new(
                    "test",
                    PromptPriority::Tools,
                    "Web search is available.",
                )])
            } else {
                None
            }
        }
    }

    #[test]
    fn test_priority_ordering() {
        assert!(PromptPriority::Core < PromptPriority::Workflow);
        assert!(PromptPriority::Workflow < PromptPriority::Tools);
        assert!(PromptPriority::Tools < PromptPriority::Features);
        assert!(PromptPriority::Features < PromptPriority::Provider);
        assert!(PromptPriority::Provider < PromptPriority::Context);
    }

    #[test]
    fn test_prompt_context_builder() {
        let ctx = PromptContext::new("anthropic", "claude-sonnet-4")
            .with_web_search(true)
            .with_sub_agents(true)
            .with_tools(vec!["read_file".to_string()]);

        assert_eq!(ctx.provider, "anthropic");
        assert_eq!(ctx.model, "claude-sonnet-4");
        assert!(ctx.has_web_search);
        assert!(ctx.has_sub_agents);
        assert_eq!(ctx.available_tools, vec!["read_file"]);
    }

    #[test]
    fn test_contributor_conditional() {
        let contributor = TestContributor;

        let ctx_with_search = PromptContext::new("test", "test").with_web_search(true);
        let ctx_without_search = PromptContext::new("test", "test").with_web_search(false);

        assert!(contributor.contribute(&ctx_with_search).is_some());
        assert!(contributor.contribute(&ctx_without_search).is_none());
    }
}
