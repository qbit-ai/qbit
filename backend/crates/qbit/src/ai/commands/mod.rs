// Commands module for AI agent interaction.
//
// This module provides Tauri command handlers for the AI agent system,
// organized into logical submodules for maintainability.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use super::agent_bridge::AgentBridge;
use crate::state::AppState;
use qbit_core::runtime::QbitRuntime;

pub mod config;
pub mod context;
pub mod core;
pub mod hitl;
pub mod loop_detection;
pub mod mode;
pub mod plan;
pub mod policy;
pub mod session;
pub mod workflow;

// Re-export all commands for easier access
pub use config::*;
pub use context::*;
pub use core::*;
pub use hitl::*;
pub use loop_detection::*;
pub use mode::*;
pub use plan::*;
pub use policy::*;
pub use session::*;
pub use workflow::*;

/// Shared AI state supporting multiple per-session agents.
/// Uses tokio RwLock for async compatibility with AgentBridge methods.
pub struct AiState {
    /// Map of session_id -> AgentBridge for per-tab AI isolation.
    pub bridges: Arc<RwLock<HashMap<String, AgentBridge>>>,
    /// Legacy single bridge for backwards compatibility during migration.
    /// TODO: Remove once all commands use session-specific bridges.
    pub bridge: Arc<RwLock<Option<AgentBridge>>>,
    /// Runtime abstraction for event emission and approval handling.
    /// Stored here for later phases when AgentBridge will use it directly.
    /// Currently created during init but the existing event_tx path is used.
    pub runtime: Arc<RwLock<Option<Arc<dyn QbitRuntime>>>>,
}

impl Default for AiState {
    fn default() -> Self {
        Self {
            bridges: Arc::new(RwLock::new(HashMap::new())),
            bridge: Arc::new(RwLock::new(None)),
            runtime: Arc::new(RwLock::new(None)),
        }
    }
}

/// Error message for uninitialized AI agent.
pub const AI_NOT_INITIALIZED_ERROR: &str = "AI agent not initialized. Call init_ai_agent first.";

/// Error message for session without AI agent.
pub fn ai_session_not_initialized_error(session_id: &str) -> String {
    format!(
        "AI agent not initialized for session '{}'. Call init_ai_session first.",
        session_id
    )
}

impl AiState {
    pub fn new() -> Self {
        Self::default()
    }

    // ========== Session-specific bridge methods ==========

    /// Get a read guard to the bridges map.
    pub async fn get_bridges(
        &self,
    ) -> tokio::sync::RwLockReadGuard<'_, HashMap<String, AgentBridge>> {
        self.bridges.read().await
    }

    /// Get a write guard to the bridges map.
    #[allow(dead_code)]
    pub async fn get_bridges_mut(
        &self,
    ) -> tokio::sync::RwLockWriteGuard<'_, HashMap<String, AgentBridge>> {
        self.bridges.write().await
    }

    /// Check if a session has an initialized AI agent.
    pub async fn has_session_bridge(&self, session_id: &str) -> bool {
        self.bridges.read().await.contains_key(session_id)
    }

    /// Execute a closure with access to a session's bridge reference.
    ///
    /// Returns an error if the session has no initialized bridge.
    #[allow(dead_code)]
    pub async fn with_session_bridge<F, T>(&self, session_id: &str, f: F) -> Result<T, String>
    where
        F: FnOnce(&AgentBridge) -> T,
    {
        let guard = self.bridges.read().await;
        let bridge = guard
            .get(session_id)
            .ok_or_else(|| ai_session_not_initialized_error(session_id))?;
        Ok(f(bridge))
    }

    /// Execute an async closure with access to a session's bridge reference.
    ///
    /// Returns an error if the session has no initialized bridge.
    #[allow(dead_code)]
    pub async fn with_session_bridge_async<F, Fut, T>(
        &self,
        session_id: &str,
        f: F,
    ) -> Result<T, String>
    where
        F: FnOnce(&AgentBridge) -> Fut,
        Fut: std::future::Future<Output = T>,
    {
        let guard = self.bridges.read().await;
        let bridge = guard
            .get(session_id)
            .ok_or_else(|| ai_session_not_initialized_error(session_id))?;
        Ok(f(bridge).await)
    }

    /// Insert a bridge for a session.
    pub async fn insert_session_bridge(&self, session_id: String, bridge: AgentBridge) {
        self.bridges.write().await.insert(session_id, bridge);
    }

    /// Remove and return the bridge for a session.
    pub async fn remove_session_bridge(&self, session_id: &str) -> Option<AgentBridge> {
        self.bridges.write().await.remove(session_id)
    }

    // ========== Legacy single bridge methods (for backwards compatibility) ==========

    /// Get a read guard to the legacy bridge, returning an error if not initialized.
    ///
    /// DEPRECATED: Use with_session_bridge instead.
    /// This helper reduces boilerplate in command handlers by providing
    /// a consistent way to access the bridge with proper error handling.
    pub async fn get_bridge(
        &self,
    ) -> Result<tokio::sync::RwLockReadGuard<'_, Option<AgentBridge>>, String> {
        let guard = self.bridge.read().await;
        if guard.is_none() {
            return Err(AI_NOT_INITIALIZED_ERROR.to_string());
        }
        Ok(guard)
    }

    /// Execute a closure with access to the legacy bridge reference.
    ///
    /// DEPRECATED: Use with_session_bridge instead.
    /// This helper eliminates the two-step pattern of `get_bridge().await?.as_ref().unwrap()`.
    /// Only use for synchronous operations. For async operations, use `get_bridge()` directly.
    pub async fn with_bridge<F, T>(&self, f: F) -> Result<T, String>
    where
        F: FnOnce(&AgentBridge) -> T,
    {
        let guard = self.bridge.read().await;
        let bridge = guard.as_ref().ok_or(AI_NOT_INITIALIZED_ERROR)?;
        Ok(f(bridge))
    }
}

/// Configure the agent bridge with shared services from AppState.
///
/// This also looks up and sets the memory file path for project instructions
/// based on the workspace path and indexed codebases in settings.
pub async fn configure_bridge(bridge: &mut AgentBridge, state: &AppState) {
    bridge.set_pty_manager(state.pty_manager.clone());
    bridge.set_indexer_state(state.indexer_state.clone());
    bridge.set_tavily_state(state.tavily_state.clone());
    // NOTE: Workflow state is no longer part of qbit-ai's AgentBridge
    // It's managed directly in the qbit crate's WorkflowState
    bridge.set_sidecar_state(state.sidecar_state.clone());
    bridge.set_settings_manager(state.settings_manager.clone());

    // Look up memory file from codebase settings based on workspace path
    let workspace_path = bridge.workspace().read().await;
    let settings = state.settings_manager.get().await;

    // Find matching codebase and get memory file
    let memory_file_path = find_memory_file_for_workspace(&workspace_path, &settings.codebases);
    drop(workspace_path);

    if let Some(ref path) = memory_file_path {
        tracing::info!(
            "[agent] Using memory file from codebase settings: {}",
            path.display()
        );
    }
    bridge.set_memory_file_path(memory_file_path).await;
}

/// Find the memory file path for a workspace by matching against indexed codebases.
pub(crate) fn find_memory_file_for_workspace(
    workspace_path: &std::path::Path,
    codebases: &[crate::settings::schema::CodebaseConfig],
) -> Option<std::path::PathBuf> {
    // Helper to expand ~ to home directory
    fn expand_home_dir(path: &str) -> std::path::PathBuf {
        if path.starts_with("~/") {
            dirs::home_dir()
                .map(|home| home.join(&path[2..]))
                .unwrap_or_else(|| std::path::PathBuf::from(path))
        } else {
            std::path::PathBuf::from(path)
        }
    }

    // Canonicalize workspace path for comparison
    let workspace_canonical = workspace_path.canonicalize().ok()?;

    // Find matching codebase
    for config in codebases {
        let codebase_path = expand_home_dir(&config.path);
        if let Ok(codebase_canonical) = codebase_path.canonicalize() {
            // Check if workspace is the codebase or a subdirectory
            if workspace_canonical == codebase_canonical
                || workspace_canonical.starts_with(&codebase_canonical)
            {
                // Found matching codebase
                if let Some(ref memory_file) = config.memory_file {
                    // Return just the filename - it will be resolved relative to workspace
                    return Some(std::path::PathBuf::from(memory_file));
                }
                // Codebase found but no memory file configured
                return None;
            }
        }
    }

    // No matching codebase found
    None
}
