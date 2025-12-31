//! Sub-agent prompt contributor.
//!
//! Automatically generates documentation for available sub-agents
//! based on the current SubAgentRegistry state.

use std::sync::Arc;

use qbit_core::{PromptContext, PromptContributor, PromptPriority, PromptSection};
use qbit_sub_agents::SubAgentRegistry;
use tokio::sync::RwLock;

/// Contributor that generates documentation for available sub-agents.
///
/// This contributor reads from the SubAgentRegistry and generates
/// a prompt section listing all available sub-agents with their
/// descriptions and capabilities.
pub struct SubAgentPromptContributor {
    registry: Arc<RwLock<SubAgentRegistry>>,
}

impl SubAgentPromptContributor {
    /// Create a new sub-agent prompt contributor.
    pub fn new(registry: Arc<RwLock<SubAgentRegistry>>) -> Self {
        Self { registry }
    }

    /// Generate the sub-agent documentation synchronously.
    ///
    /// Note: This uses `blocking_read()` which should only be called
    /// from a synchronous context or during prompt building.
    fn generate_docs(&self) -> Option<String> {
        // Use try_read to avoid blocking if lock is held
        let registry = match self.registry.try_read() {
            Ok(guard) => guard,
            Err(_) => {
                tracing::warn!("Could not acquire SubAgentRegistry lock for prompt generation");
                return None;
            }
        };

        let agents: Vec<_> = registry.all().collect();
        if agents.is_empty() {
            return None;
        }

        let mut content = String::from("## Available Sub-Agents\n\n");
        content.push_str(
            "You can delegate tasks to specialized sub-agents using the corresponding tool.\n\n",
        );

        for agent in agents {
            content.push_str(&format!("### `sub_agent_{}`\n", agent.id));
            content.push_str(&format!("**{}**\n\n", agent.name));
            content.push_str(&format!("{}\n\n", agent.description));

            if !agent.allowed_tools.is_empty() {
                content.push_str("**Available tools**: ");
                content.push_str(&agent.allowed_tools.join(", "));
                content.push_str("\n\n");
            }
        }

        Some(content)
    }
}

impl PromptContributor for SubAgentPromptContributor {
    fn contribute(&self, ctx: &PromptContext) -> Option<Vec<PromptSection>> {
        // Only contribute if sub-agents are available
        if !ctx.has_sub_agents {
            return None;
        }

        self.generate_docs().map(|content| {
            vec![PromptSection::new(
                "sub_agents",
                PromptPriority::Tools,
                content,
            )]
        })
    }

    fn name(&self) -> &str {
        "SubAgentPromptContributor"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use qbit_sub_agents::SubAgentDefinition;

    fn create_test_registry() -> Arc<RwLock<SubAgentRegistry>> {
        let mut registry = SubAgentRegistry::new();

        registry.register(
            SubAgentDefinition::new(
                "code_analyzer",
                "Code Analyzer",
                "Analyzes code for patterns and issues",
                "You are a code analysis expert.",
            )
            .with_tools(vec!["read_file".to_string(), "grep_file".to_string()]),
        );

        registry.register(SubAgentDefinition::new(
            "code_writer",
            "Code Writer",
            "Writes and modifies code",
            "You are a code implementation expert.",
        ));

        Arc::new(RwLock::new(registry))
    }

    #[test]
    fn test_contributor_generates_docs() {
        let registry = create_test_registry();
        let contributor = SubAgentPromptContributor::new(registry);

        let ctx = PromptContext::new("anthropic", "claude-sonnet-4").with_sub_agents(true);

        let sections = contributor.contribute(&ctx);
        assert!(sections.is_some());

        let sections = sections.unwrap();
        assert_eq!(sections.len(), 1);
        assert_eq!(sections[0].id, "sub_agents");
        assert_eq!(sections[0].priority, PromptPriority::Tools);

        let content = &sections[0].content;
        assert!(content.contains("## Available Sub-Agents"));
        assert!(content.contains("sub_agent_code_analyzer"));
        assert!(content.contains("Code Analyzer"));
        assert!(content.contains("read_file, grep_file"));
    }

    #[test]
    fn test_contributor_skips_when_no_sub_agents() {
        let registry = create_test_registry();
        let contributor = SubAgentPromptContributor::new(registry);

        let ctx = PromptContext::new("anthropic", "claude-sonnet-4").with_sub_agents(false);

        let sections = contributor.contribute(&ctx);
        assert!(sections.is_none());
    }

    #[test]
    fn test_contributor_skips_empty_registry() {
        let registry = Arc::new(RwLock::new(SubAgentRegistry::new()));
        let contributor = SubAgentPromptContributor::new(registry);

        let ctx = PromptContext::new("anthropic", "claude-sonnet-4").with_sub_agents(true);

        let sections = contributor.contribute(&ctx);
        assert!(sections.is_none());
    }
}
