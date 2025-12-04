//! Registry for workflow graphs.

use std::collections::HashMap;
use std::sync::Arc;

use graph_flow::Graph;

/// Registry of named workflow graphs.
pub struct WorkflowRegistry {
    workflows: HashMap<String, Arc<Graph>>,
}

impl WorkflowRegistry {
    /// Create a new empty registry.
    pub fn new() -> Self {
        Self {
            workflows: HashMap::new(),
        }
    }

    /// Register a workflow graph.
    pub fn register(&mut self, name: impl Into<String>, graph: Arc<Graph>) {
        self.workflows.insert(name.into(), graph);
    }

    /// Get a workflow by name.
    pub fn get(&self, name: &str) -> Option<Arc<Graph>> {
        self.workflows.get(name).cloned()
    }

    /// List all registered workflow names.
    pub fn list(&self) -> Vec<String> {
        self.workflows.keys().cloned().collect()
    }

    /// Check if a workflow exists.
    pub fn contains(&self, name: &str) -> bool {
        self.workflows.contains_key(name)
    }
}

impl Default for WorkflowRegistry {
    fn default() -> Self {
        Self::new()
    }
}
