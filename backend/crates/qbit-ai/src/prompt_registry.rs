//! Registry for prompt contributors.
//!
//! Collects and aggregates prompt sections from multiple contributors
//! to build a dynamically composed system prompt.

use std::sync::Arc;

use qbit_core::{PromptContext, PromptContributor, PromptSection};

/// Registry for prompt contributors.
///
/// Collects contributions from registered components and builds
/// a composed prompt string sorted by priority.
#[derive(Default)]
pub struct PromptContributorRegistry {
    contributors: Vec<Arc<dyn PromptContributor>>,
}

impl PromptContributorRegistry {
    /// Create a new empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a prompt contributor.
    pub fn register(&mut self, contributor: Arc<dyn PromptContributor>) {
        tracing::debug!("Registering prompt contributor: {}", contributor.name());
        self.contributors.push(contributor);
    }

    /// Register multiple prompt contributors.
    pub fn register_multiple(&mut self, contributors: Vec<Arc<dyn PromptContributor>>) {
        for contributor in contributors {
            self.register(contributor);
        }
    }

    /// Collect all contributions for the given context.
    ///
    /// Returns sections sorted by priority (lower priority values first).
    pub fn collect(&self, ctx: &PromptContext) -> Vec<PromptSection> {
        let mut sections: Vec<PromptSection> = self
            .contributors
            .iter()
            .filter_map(|c| {
                let result = c.contribute(ctx);
                if let Some(ref sections) = result {
                    tracing::trace!(
                        "Contributor {} produced {} section(s)",
                        c.name(),
                        sections.len()
                    );
                }
                result
            })
            .flatten()
            .collect();

        // Sort by priority (stable sort preserves registration order for same priority)
        sections.sort_by_key(|s| s.priority);

        tracing::debug!(
            "Collected {} prompt sections from {} contributors",
            sections.len(),
            self.contributors.len()
        );

        sections
    }

    /// Build complete prompt string from contributions.
    ///
    /// Sections are joined with double newlines for readability.
    pub fn build_prompt(&self, ctx: &PromptContext) -> String {
        self.collect(ctx)
            .into_iter()
            .map(|s| s.content)
            .collect::<Vec<_>>()
            .join("\n\n")
    }

    /// Returns the number of registered contributors.
    pub fn len(&self) -> usize {
        self.contributors.len()
    }

    /// Returns true if no contributors are registered.
    pub fn is_empty(&self) -> bool {
        self.contributors.is_empty()
    }
}

impl std::fmt::Debug for PromptContributorRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PromptContributorRegistry")
            .field("contributor_count", &self.contributors.len())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use qbit_core::PromptPriority;

    struct HighPriorityContributor;
    struct LowPriorityContributor;
    struct ConditionalContributor;

    impl PromptContributor for HighPriorityContributor {
        fn contribute(&self, _ctx: &PromptContext) -> Option<Vec<PromptSection>> {
            Some(vec![PromptSection::new(
                "high",
                PromptPriority::Core,
                "High priority content.",
            )])
        }

        fn name(&self) -> &str {
            "HighPriorityContributor"
        }
    }

    impl PromptContributor for LowPriorityContributor {
        fn contribute(&self, _ctx: &PromptContext) -> Option<Vec<PromptSection>> {
            Some(vec![PromptSection::new(
                "low",
                PromptPriority::Context,
                "Low priority content.",
            )])
        }

        fn name(&self) -> &str {
            "LowPriorityContributor"
        }
    }

    impl PromptContributor for ConditionalContributor {
        fn contribute(&self, ctx: &PromptContext) -> Option<Vec<PromptSection>> {
            if ctx.has_web_search {
                Some(vec![PromptSection::new(
                    "conditional",
                    PromptPriority::Tools,
                    "Web search available.",
                )])
            } else {
                None
            }
        }

        fn name(&self) -> &str {
            "ConditionalContributor"
        }
    }

    #[test]
    fn test_registry_ordering() {
        let mut registry = PromptContributorRegistry::new();

        // Register in reverse priority order
        registry.register(Arc::new(LowPriorityContributor));
        registry.register(Arc::new(HighPriorityContributor));

        let ctx = PromptContext::default();
        let sections = registry.collect(&ctx);

        assert_eq!(sections.len(), 2);
        // Should be sorted by priority, not registration order
        assert_eq!(sections[0].id, "high");
        assert_eq!(sections[1].id, "low");
    }

    #[test]
    fn test_registry_conditional() {
        let mut registry = PromptContributorRegistry::new();
        registry.register(Arc::new(ConditionalContributor));

        let ctx_with_search = PromptContext::new("test", "test").with_web_search(true);
        let ctx_without_search = PromptContext::new("test", "test").with_web_search(false);

        assert_eq!(registry.collect(&ctx_with_search).len(), 1);
        assert_eq!(registry.collect(&ctx_without_search).len(), 0);
    }

    #[test]
    fn test_build_prompt() {
        let mut registry = PromptContributorRegistry::new();
        registry.register(Arc::new(HighPriorityContributor));
        registry.register(Arc::new(LowPriorityContributor));

        let ctx = PromptContext::default();
        let prompt = registry.build_prompt(&ctx);

        assert!(prompt.contains("High priority content."));
        assert!(prompt.contains("Low priority content."));
        // High should come before low
        assert!(prompt.find("High").unwrap() < prompt.find("Low").unwrap());
    }
}
