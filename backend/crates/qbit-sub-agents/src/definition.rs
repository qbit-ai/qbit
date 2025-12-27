//! Sub-agent definitions, context, and registry.
//!
//! This module provides the infrastructure for:
//! - Defining specialized sub-agents with custom system prompts and tool restrictions
//! - Managing state and context between agents
//! - Registering and retrieving sub-agent definitions

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// Context passed to a sub-agent during execution
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SubAgentContext {
    /// The original user request that triggered the workflow
    pub original_request: String,

    /// Summary of conversation history for context awareness
    pub conversation_summary: Option<String>,

    /// Variables passed from parent agent's state
    pub variables: HashMap<String, serde_json::Value>,

    /// Current depth in the agent hierarchy (to prevent infinite recursion)
    pub depth: usize,
}

/// Result returned by a sub-agent after execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubAgentResult {
    /// ID of the sub-agent that produced this result
    pub agent_id: String,

    /// The agent's response text
    pub response: String,

    /// Updated context (may include new variables)
    pub context: SubAgentContext,

    /// Whether the sub-agent completed successfully
    pub success: bool,

    /// Execution duration in milliseconds
    pub duration_ms: u64,

    /// Files modified by this sub-agent during execution
    #[serde(default)]
    pub files_modified: Vec<String>,
}

/// Definition of a specialized sub-agent
#[derive(Clone, Debug)]
pub struct SubAgentDefinition {
    /// Unique identifier for this sub-agent
    pub id: String,

    /// Human-readable name
    pub name: String,

    /// Description for the parent agent to understand when to invoke this sub-agent
    pub description: String,

    /// System prompt that defines this sub-agent's role and capabilities
    pub system_prompt: String,

    /// List of tool names this sub-agent is allowed to use (empty = all tools)
    pub allowed_tools: Vec<String>,

    /// Maximum iterations for this sub-agent's tool loop
    pub max_iterations: usize,
}

impl SubAgentDefinition {
    /// Create a new sub-agent definition
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        description: impl Into<String>,
        system_prompt: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            description: description.into(),
            system_prompt: system_prompt.into(),
            allowed_tools: Vec::new(),
            max_iterations: 50,
        }
    }

    /// Set allowed tools for this sub-agent
    pub fn with_tools(mut self, tools: Vec<String>) -> Self {
        self.allowed_tools = tools;
        self
    }

    /// Set maximum iterations
    pub fn with_max_iterations(mut self, max: usize) -> Self {
        self.max_iterations = max;
        self
    }
}

/// Registry of available sub-agents
#[derive(Default)]
pub struct SubAgentRegistry {
    agents: HashMap<String, SubAgentDefinition>,
}

impl SubAgentRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            agents: HashMap::new(),
        }
    }

    /// Get a sub-agent by ID
    pub fn get(&self, id: &str) -> Option<&SubAgentDefinition> {
        self.agents.get(id)
    }

    /// Get all registered sub-agents
    pub fn all(&self) -> impl Iterator<Item = &SubAgentDefinition> {
        self.agents.values()
    }

    /// Get count of registered sub-agents
    #[allow(dead_code)] // Used in tests
    pub fn len(&self) -> usize {
        self.agents.len()
    }

    /// Check if registry is empty
    #[allow(dead_code)] // Used in tests
    pub fn is_empty(&self) -> bool {
        self.agents.is_empty()
    }

    /// Register a sub-agent in the registry
    pub fn register(&mut self, agent: SubAgentDefinition) {
        self.agents.insert(agent.id.clone(), agent);
    }

    /// Register multiple sub-agents at once
    pub fn register_multiple(&mut self, agents: Vec<SubAgentDefinition>) {
        for agent in agents {
            self.register(agent);
        }
    }
}

/// Maximum recursion depth to prevent infinite sub-agent loops
pub const MAX_AGENT_DEPTH: usize = 5;

#[cfg(test)]
mod tests {
    use super::*;

    // ===========================================
    // SubAgentDefinition Tests
    // ===========================================

    #[test]
    fn test_sub_agent_definition_new() {
        let agent = SubAgentDefinition::new(
            "test_agent",
            "Test Agent",
            "A test agent for unit tests",
            "You are a test agent.",
        );

        assert_eq!(agent.id, "test_agent");
        assert_eq!(agent.name, "Test Agent");
        assert_eq!(agent.description, "A test agent for unit tests");
        assert_eq!(agent.system_prompt, "You are a test agent.");
        assert!(agent.allowed_tools.is_empty());
        assert_eq!(agent.max_iterations, 50); // default
    }

    #[test]
    fn test_sub_agent_definition_with_tools() {
        let agent = SubAgentDefinition::new("test", "Test", "desc", "prompt")
            .with_tools(vec!["read_file".to_string(), "write_file".to_string()]);

        assert_eq!(agent.allowed_tools.len(), 2);
        assert!(agent.allowed_tools.contains(&"read_file".to_string()));
        assert!(agent.allowed_tools.contains(&"write_file".to_string()));
    }

    #[test]
    fn test_sub_agent_definition_with_max_iterations() {
        let agent =
            SubAgentDefinition::new("test", "Test", "desc", "prompt").with_max_iterations(100);

        assert_eq!(agent.max_iterations, 100);
    }

    #[test]
    fn test_sub_agent_definition_builder_chain() {
        let agent = SubAgentDefinition::new("chained", "Chained Agent", "desc", "prompt")
            .with_tools(vec!["tool1".to_string()])
            .with_max_iterations(25);

        assert_eq!(agent.id, "chained");
        assert_eq!(agent.allowed_tools, vec!["tool1".to_string()]);
        assert_eq!(agent.max_iterations, 25);
    }

    // ===========================================
    // SubAgentRegistry Tests
    // ===========================================

    #[test]
    fn test_registry_new_is_empty() {
        let registry = SubAgentRegistry::new();
        assert!(registry.is_empty());
        assert_eq!(registry.len(), 0);
    }

    #[test]
    fn test_registry_default_is_empty() {
        let registry = SubAgentRegistry::default();
        assert!(registry.is_empty());
    }

    #[test]
    fn test_registry_get_nonexistent() {
        let registry = SubAgentRegistry::new();
        assert!(registry.get("nonexistent").is_none());
    }

    // ===========================================
    // SubAgentContext Tests
    // ===========================================

    #[test]
    fn test_context_default() {
        let context = SubAgentContext::default();
        assert_eq!(context.original_request, "");
        assert!(context.conversation_summary.is_none());
        assert!(context.variables.is_empty());
        assert_eq!(context.depth, 0);
    }

    #[test]
    fn test_context_with_values() {
        let mut variables = HashMap::new();
        variables.insert("key".to_string(), serde_json::json!("value"));

        let context = SubAgentContext {
            original_request: "Do something".to_string(),
            conversation_summary: Some("Previous context".to_string()),
            variables,
            depth: 2,
        };

        assert_eq!(context.original_request, "Do something");
        assert_eq!(
            context.conversation_summary,
            Some("Previous context".to_string())
        );
        assert_eq!(
            context.variables.get("key").unwrap(),
            &serde_json::json!("value")
        );
        assert_eq!(context.depth, 2);
    }

    // ===========================================
    // SubAgentResult Tests
    // ===========================================

    #[test]
    fn test_result_construction() {
        let result = SubAgentResult {
            agent_id: "test_agent".to_string(),
            response: "Task completed".to_string(),
            context: SubAgentContext::default(),
            success: true,
            duration_ms: 1500,
            files_modified: vec!["main.go".to_string()],
        };

        assert_eq!(result.agent_id, "test_agent");
        assert_eq!(result.response, "Task completed");
        assert!(result.success);
        assert_eq!(result.duration_ms, 1500);
        assert_eq!(result.files_modified, vec!["main.go".to_string()]);
    }

    // ===========================================
    // Constants Tests
    // ===========================================

    #[test]
    fn test_max_agent_depth() {
        assert_eq!(MAX_AGENT_DEPTH, 5);
    }
}
