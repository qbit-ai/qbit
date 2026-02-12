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

    /// Optional model override (provider_name, model_name).
    /// When set, this sub-agent uses a different model than the main agent.
    /// None = inherit the main agent's model.
    pub model_override: Option<(String, String)>,

    /// Overall timeout for the entire sub-agent execution in seconds.
    /// None = no timeout. Default: 600 (10 minutes).
    pub timeout_secs: Option<u64>,

    /// Idle timeout - max seconds without any progress (LLM chunk, tool result).
    /// None = no idle timeout. Default: 180 (3 minutes).
    pub idle_timeout_secs: Option<u64>,

    /// Optional prompt generation system prompt. When set, the executor makes an LLM call
    /// using this as the system prompt and the task/context as the user message to generate
    /// the sub-agent's system prompt before execution. The definition's `system_prompt`
    /// field is used as a fallback if prompt generation fails.
    /// When `None`, the `system_prompt` is used directly (default for specialized agents).
    pub prompt_template: Option<String>,
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
            model_override: None,
            timeout_secs: Some(600),
            idle_timeout_secs: Some(180),
            prompt_template: None,
        }
    }

    /// Set allowed tools for this sub-agent
    pub fn with_tools(mut self, tools: Vec<String>) -> Self {
        self.allowed_tools = tools;
        self
    }

    /// Set a prompt generation system prompt. When set, the executor uses this as the
    /// system prompt in an LLM call (with task/context as user message) to generate
    /// an optimized system prompt for the sub-agent before execution.
    pub fn with_prompt_template(mut self, template: impl Into<String>) -> Self {
        self.prompt_template = Some(template.into());
        self
    }

    /// Set maximum iterations
    pub fn with_max_iterations(mut self, max: usize) -> Self {
        self.max_iterations = max;
        self
    }

    /// Set overall timeout in seconds
    pub fn with_timeout(mut self, secs: u64) -> Self {
        self.timeout_secs = Some(secs);
        self
    }

    /// Set idle timeout in seconds
    pub fn with_idle_timeout(mut self, secs: u64) -> Self {
        self.idle_timeout_secs = Some(secs);
        self
    }

    /// Set model override for this sub-agent (builder pattern)
    pub fn with_model_override(
        mut self,
        provider: impl Into<String>,
        model: impl Into<String>,
    ) -> Self {
        self.model_override = Some((provider.into(), model.into()));
        self
    }

    /// Set model override at runtime
    pub fn set_model_override(&mut self, provider: impl Into<String>, model: impl Into<String>) {
        self.model_override = Some((provider.into(), model.into()));
    }

    /// Clear model override (will inherit main agent's model)
    pub fn clear_model_override(&mut self) {
        self.model_override = None;
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

    /// Get a mutable reference to a sub-agent by ID
    pub fn get_mut(&mut self, id: &str) -> Option<&mut SubAgentDefinition> {
        self.agents.get_mut(id)
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
        assert!(agent.model_override.is_none()); // default
        assert_eq!(agent.timeout_secs, Some(600)); // default: 10 minutes
        assert_eq!(agent.idle_timeout_secs, Some(180)); // default: 3 minutes
        assert!(agent.prompt_template.is_none()); // default: no prompt generation
    }

    #[test]
    fn test_sub_agent_definition_with_prompt_template() {
        let agent = SubAgentDefinition::new("test", "Test", "desc", "prompt")
            .with_prompt_template("Generate a prompt for: {task}");
        assert_eq!(
            agent.prompt_template,
            Some("Generate a prompt for: {task}".to_string())
        );
    }

    #[test]
    fn test_sub_agent_definition_without_prompt_template() {
        let agent = SubAgentDefinition::new("test", "Test", "desc", "prompt");
        assert!(agent.prompt_template.is_none());
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

    #[test]
    fn test_sub_agent_definition_with_model_override() {
        let agent = SubAgentDefinition::new("test", "Test", "desc", "prompt")
            .with_model_override("openai", "gpt-4o");

        assert_eq!(
            agent.model_override,
            Some(("openai".to_string(), "gpt-4o".to_string()))
        );
    }

    #[test]
    fn test_sub_agent_definition_set_and_clear_model_override() {
        let mut agent = SubAgentDefinition::new("test", "Test", "desc", "prompt");

        // Initially no override
        assert!(agent.model_override.is_none());

        // Set override
        agent.set_model_override("anthropic", "claude-sonnet-4");
        assert_eq!(
            agent.model_override,
            Some(("anthropic".to_string(), "claude-sonnet-4".to_string()))
        );

        // Clear override
        agent.clear_model_override();
        assert!(agent.model_override.is_none());
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
