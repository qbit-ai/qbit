//! Application state for Tauri commands.
//!
//! This module is only compiled when the `tauri` feature is enabled.

use std::sync::Arc;

use crate::ai::commands::WorkflowState;
use crate::ai::AiState;
use crate::indexer::IndexerState;
use crate::pty::PtyManager;
use crate::settings::SettingsManager;
use crate::sidecar::{SidecarConfig, SidecarState};

pub struct AppState {
    pub pty_manager: Arc<PtyManager>,
    pub ai_state: AiState,
    pub workflow_state: Arc<WorkflowState>,
    pub indexer_state: Arc<IndexerState>,
    pub settings_manager: Arc<SettingsManager>,
    /// Sidecar configuration - used to create per-session SidecarState instances.
    pub sidecar_config: SidecarConfig,
    /// Global sidecar state for UI commands (status, session listing, etc.).
    /// NOTE: Agent bridges have their OWN SidecarState instances (created in configure_bridge)
    /// to enable per-session isolation and avoid blocking between tabs.
    pub sidecar_state: Arc<SidecarState>,
    /// Whether Langfuse tracing is active (enabled and properly configured).
    pub langfuse_active: bool,
}

impl AppState {
    /// Create a new AppState with all subsystems initialized.
    ///
    /// This is async because SettingsManager needs to load from disk.
    ///
    /// # Arguments
    /// * `langfuse_active` - Whether Langfuse tracing is enabled and properly configured.
    pub async fn new(langfuse_active: bool) -> Self {
        // Initialize settings manager first (needed by TavilyState in the future)
        let settings_manager = Arc::new(
            SettingsManager::new()
                .await
                .expect("Failed to initialize settings manager"),
        );

        // Ensure settings file exists (creates template on first run)
        if let Err(e) = settings_manager.ensure_settings_file().await {
            tracing::warn!("Failed to create settings template: {}", e);
        }

        // Load settings and create SidecarConfig from them
        let settings = settings_manager.get().await;
        let sidecar_config = SidecarConfig::from_qbit_settings(&settings.sidecar);
        tracing::debug!(
            "[app-state] Created sidecar config: enabled={}",
            sidecar_config.enabled
        );

        // Create global sidecar state for UI commands.
        // Note: Agent bridges create their OWN SidecarState instances for per-session isolation.
        let sidecar_state = Arc::new(SidecarState::with_config(sidecar_config.clone()));

        Self {
            pty_manager: Arc::new(PtyManager::new()),
            ai_state: AiState::new(),
            workflow_state: Arc::new(WorkflowState::new()),
            indexer_state: Arc::new(IndexerState::new()),
            settings_manager,
            sidecar_config,
            sidecar_state,
            langfuse_active,
        }
    }
}
