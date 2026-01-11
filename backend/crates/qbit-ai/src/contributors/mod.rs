//! Prompt contributors for dynamic system prompt composition.
//!
//! Each contributor implements the `PromptContributor` trait and provides
//! context-aware prompt sections.

mod provider_tools;
mod sub_agents;
mod tavily_tools;

pub use provider_tools::ProviderBuiltinToolsContributor;
pub use sub_agents::SubAgentPromptContributor;
pub use tavily_tools::TavilyToolsContributor;

use std::sync::Arc;

use qbit_core::PromptContributor;
use qbit_sub_agents::SubAgentRegistry;
use tokio::sync::RwLock;

/// Create the default set of prompt contributors.
pub fn create_default_contributors(
    sub_agent_registry: Arc<RwLock<SubAgentRegistry>>,
) -> Vec<Arc<dyn PromptContributor>> {
    vec![
        Arc::new(SubAgentPromptContributor::new(sub_agent_registry)),
        Arc::new(ProviderBuiltinToolsContributor),
        Arc::new(TavilyToolsContributor),
    ]
}
