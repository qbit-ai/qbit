pub mod ai;
pub mod cli_output;
pub mod compat;
mod error;
pub mod history;
mod indexer;
mod mcp;
mod models;
mod projects;
mod pty;
pub mod runtime;
mod settings;
mod sidecar;
mod state;
pub mod telemetry;
pub mod tools;
mod window_state;

// CLI module (for headless mode)
pub mod cli;

// Tauri commands module (for GUI mode)
mod commands;

use crate::history::{HistoryConfig, HistoryManager};
use ai::{
    add_tool_always_allow, cancel_workflow, clear_ai_conversation, clear_ai_conversation_session,
    disable_full_auto_mode, disable_loop_detection, enable_full_auto_mode, enable_loop_detection,
    execute_ai_tool, export_ai_session_transcript, finalize_ai_session, find_ai_session,
    generate_commit_message, get_agent_mode, get_ai_conversation_length,
    get_ai_conversation_length_session, get_api_request_stats, get_approval_patterns,
    get_available_tools, get_context_summary, get_context_trim_config, get_context_utilization,
    get_hitl_config, get_loop_detector_stats, get_loop_protection_config, get_openai_api_key,
    get_openrouter_api_key, get_plan, get_project_settings, get_remaining_tokens,
    get_session_ai_config, get_sub_agent_model, get_token_alert_level, get_token_usage_stats,
    get_tool_approval_pattern, get_tool_policy, get_tool_policy_config, get_vertex_ai_config,
    get_vision_capabilities, get_workflow_state, init_ai_agent, init_ai_agent_openai,
    init_ai_agent_unified, init_ai_agent_vertex, init_ai_session, is_ai_initialized,
    is_ai_session_initialized, is_ai_session_persistence_enabled, is_context_management_enabled,
    is_full_auto_mode_enabled, is_loop_detection_enabled, list_ai_sessions, list_sub_agents,
    list_workflow_sessions, list_workflows, load_ai_session, load_env_file,
    remove_tool_always_allow, reset_approval_patterns, reset_context_manager, reset_loop_detector,
    reset_tool_policies, respond_to_tool_approval, restore_ai_session, retry_compaction,
    run_workflow_to_completion, save_project_agent_mode, save_project_model, send_ai_prompt,
    send_ai_prompt_session, send_ai_prompt_with_attachments, set_agent_mode,
    set_ai_session_persistence, set_hitl_config, set_loop_protection_config, set_sub_agent_model,
    set_tool_policy, set_tool_policy_config, shutdown_ai_agent, shutdown_ai_session,
    signal_frontend_ready, start_workflow, step_workflow, update_ai_workspace,
};
use commands::*;
use indexer::{
    add_indexed_codebase, create_git_worktree, detect_memory_files, get_all_indexed_files,
    get_indexed_file_count, get_indexer_workspace, index_directory, index_file, init_indexer,
    is_indexer_initialized, list_git_branches, list_indexed_codebases, list_projects_for_home,
    list_recent_directories, migrate_codebase_index, reindex_codebase, remove_indexed_codebase,
    search_code, search_files, shutdown_indexer, update_codebase_memory_file,
};
use mcp::{
    mcp_connect, mcp_disconnect, mcp_get_config, mcp_has_project_config, mcp_is_project_trusted,
    mcp_list_servers, mcp_list_tools, mcp_trust_project_config,
};
use models::commands::{
    get_available_models, get_model_by_id, get_model_capabilities_command, get_providers,
};
use projects::commands::{
    delete_project_config, get_project_config, list_project_configs, save_project,
};
use settings::{
    get_setting, get_settings, get_settings_path, get_telemetry_stats, get_window_state,
    is_langfuse_active, reload_settings, reset_settings, save_window_state, set_setting,
    settings_file_exists, update_settings,
};
use sidecar::{
    // L3: Artifact commands
    sidecar_apply_all_artifacts,
    // L2: Patch commands
    sidecar_apply_all_patches,
    sidecar_apply_artifact,
    sidecar_apply_patch,
    sidecar_current_session,
    sidecar_discard_artifact,
    sidecar_discard_patch,
    sidecar_end_session,
    sidecar_get_applied_artifacts,
    sidecar_get_applied_patches,
    sidecar_get_artifact,
    sidecar_get_config,
    sidecar_get_current_pending_artifacts,
    sidecar_get_current_staged_patches,
    sidecar_get_injectable_context,
    sidecar_get_patch,
    sidecar_get_pending_artifacts,
    sidecar_get_session_log,
    sidecar_get_session_meta,
    sidecar_get_session_state,
    sidecar_get_staged_patches,
    sidecar_initialize,
    sidecar_list_sessions,
    sidecar_preview_artifact,
    sidecar_regenerate_artifacts,
    sidecar_regenerate_patch,
    sidecar_resume_session,
    sidecar_set_config,
    sidecar_shutdown,
    sidecar_start_session,
    sidecar_status,
    sidecar_update_patch_message,
};
use state::AppState;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::{Emitter, Manager};
use tokio::sync::RwLock;

/// Tauri application entry point for GUI mode
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run_gui() {
    // Parse CLI arguments for workspace directory
    // Usage: qbit [path] or pnpm tauri dev -- [path]
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 {
        let path_arg = &args[1];
        // Skip if it looks like a flag (starts with -)
        if !path_arg.starts_with('-') {
            // Expand ~ to home directory
            let workspace = if path_arg.starts_with("~/") {
                dirs::home_dir()
                    .map(|home| home.join(&path_arg[2..]))
                    .unwrap_or_else(|| std::path::PathBuf::from(path_arg))
            } else {
                std::path::PathBuf::from(path_arg)
            };
            std::env::set_var("QBIT_WORKSPACE", &workspace);
        }
    }

    // Install rustls crypto provider (required for rustls 0.23+)
    // This must be done before any TLS operations (e.g., reqwest)
    let _ = rustls::crypto::ring::default_provider().install_default();

    // Load .env file from the project root (if it exists)
    // This loads env vars before anything else needs them
    if let Err(e) = dotenvy::dotenv() {
        // Only warn if file doesn't exist - other errors should be reported
        if !matches!(e, dotenvy::Error::Io(_)) {
            eprintln!("Warning: Failed to load .env file: {}", e);
        }
    }

    // Set session directory to ~/.qbit/sessions
    // This env var is read by qbit-core's session_archive module
    if std::env::var_os("VT_SESSION_DIR").is_none() {
        if let Some(home) = dirs::home_dir() {
            let qbit_sessions = home.join(".qbit").join("sessions");
            std::env::set_var("VT_SESSION_DIR", &qbit_sessions);
        }
    }

    // Load settings and initialize telemetry using Tauri's async runtime.
    // IMPORTANT: We use tauri::async_runtime::block_on instead of creating our own
    // Tokio runtime. This ensures the BatchSpanProcessor's background tasks run on
    // the same runtime that Tauri uses, so they continue executing during the app's
    // lifetime. Previously, we created a separate runtime that went idle after
    // initialization, causing spans to never be flushed to Langfuse.
    let (_telemetry_guard, app_state) = tauri::async_runtime::block_on(async {
        // Load settings to configure telemetry
        let settings_manager = Arc::new(
            settings::SettingsManager::new()
                .await
                .expect("Failed to initialize settings manager"),
        );

        // Read settings for telemetry configuration
        let (langfuse_config, log_level) = {
            let settings = settings_manager.get().await;
            let langfuse = telemetry::LangfuseConfig::from_settings(&settings.telemetry.langfuse);
            let level = settings.advanced.log_level.to_string();
            (langfuse, level)
        };

        // Initialize tracing with optional Langfuse export
        // Must be done within Tokio runtime for async batch processor
        let (telemetry_guard, langfuse_active, telemetry_stats) =
            match telemetry::init_tracing(langfuse_config, &log_level, &[]) {
                Ok(guard) => {
                    let active = guard.langfuse_active;
                    let stats = guard.stats.clone();
                    (Some(guard), active, stats)
                }
                Err(e) => {
                    // Fall back to basic tracing if OpenTelemetry setup fails
                    eprintln!("Warning: Failed to initialize OpenTelemetry: {}", e);
                    let _ = tracing_subscriber::fmt()
                        .with_env_filter(
                            tracing_subscriber::EnvFilter::from_default_env()
                                .add_directive("qbit=debug".parse().unwrap()),
                        )
                        .try_init();
                    (None, false, None)
                }
            };

        // Create AppState using the same SettingsManager (no redundant disk read)
        let app_state =
            AppState::with_settings_manager(settings_manager, langfuse_active, telemetry_stats)
                .await;

        (telemetry_guard, app_state)
    });

    // Initialize HistoryManager in the background (deferred to avoid blocking startup)
    let history_manager: Arc<RwLock<Option<HistoryManager>>> = Arc::new(RwLock::new(None));
    {
        let history_manager = history_manager.clone();
        tauri::async_runtime::spawn(async move {
            match HistoryManager::new(HistoryConfig::default()) {
                Ok(manager) => {
                    *history_manager.write().await = Some(manager);
                    tracing::debug!("HistoryManager initialized in background");
                }
                Err(e) => {
                    tracing::warn!("Failed to initialize HistoryManager: {}", e);
                }
            }
        });
    }

    // Ensure settings file exists in the background (creates template on first run)
    {
        let settings_manager = app_state.settings_manager.clone();
        tauri::async_runtime::spawn(async move {
            if let Err(e) = settings_manager.ensure_settings_file().await {
                tracing::warn!("Failed to create settings template: {}", e);
            }
        });
    }

    async fn persist_window_state_from_window(window: &tauri::Window) {
        let scale_factor = window.scale_factor().unwrap_or(1.0);

        let size = window
            .inner_size()
            .map(|size| size.to_logical::<f64>(scale_factor))
            .unwrap_or_else(|_| tauri::LogicalSize::new(0.0, 0.0));

        let position = window
            .outer_position()
            .ok()
            .map(|p| p.to_logical::<f64>(scale_factor));

        let is_maximized = window.is_maximized().unwrap_or(false);

        let normalized = window_state::normalize_persisted_window_state(
            size.width,
            size.height,
            position.map(|p| p.x),
            position.map(|p| p.y),
            is_maximized,
        );

        static LOGGED: AtomicBool = AtomicBool::new(false);

        let state = window.app_handle().state::<AppState>();
        let mut settings = state.settings_manager.get().await;

        settings.ui.window.width = normalized.width;
        settings.ui.window.height = normalized.height;
        settings.ui.window.x = normalized.x;
        settings.ui.window.y = normalized.y;
        settings.ui.window.maximized = normalized.maximized;

        if !LOGGED.swap(true, Ordering::SeqCst) {
            tracing::debug!(
                settings_path = %state.settings_manager.path().display(),
                width = settings.ui.window.width,
                height = settings.ui.window.height,
                x = ?settings.ui.window.x,
                y = ?settings.ui.window.y,
                maximized = settings.ui.window.maximized,
                "Persisting window state"
            );
        }

        if let Err(e) = state.settings_manager.update(settings).await {
            tracing::debug!(error = %e, "Failed to persist window state");
        }
    }

    async fn persist_window_state_from_webview_window(window: &tauri::WebviewWindow) {
        let scale_factor = window.scale_factor().unwrap_or(1.0);

        let size = window
            .inner_size()
            .map(|size| size.to_logical::<f64>(scale_factor))
            .unwrap_or_else(|_| tauri::LogicalSize::new(0.0, 0.0));

        let position = window
            .outer_position()
            .ok()
            .map(|p| p.to_logical::<f64>(scale_factor));

        let is_maximized = window.is_maximized().unwrap_or(false);

        let normalized = window_state::normalize_persisted_window_state(
            size.width,
            size.height,
            position.map(|p| p.x),
            position.map(|p| p.y),
            is_maximized,
        );

        static LOGGED: AtomicBool = AtomicBool::new(false);

        let state = window.app_handle().state::<AppState>();
        let mut settings = state.settings_manager.get().await;

        settings.ui.window.width = normalized.width;
        settings.ui.window.height = normalized.height;
        settings.ui.window.x = normalized.x;
        settings.ui.window.y = normalized.y;
        settings.ui.window.maximized = normalized.maximized;

        if !LOGGED.swap(true, Ordering::SeqCst) {
            tracing::debug!(
                settings_path = %state.settings_manager.path().display(),
                width = settings.ui.window.width,
                height = settings.ui.window.height,
                x = ?settings.ui.window.x,
                y = ?settings.ui.window.y,
                maximized = settings.ui.window.maximized,
                "Persisting window state (exit)"
            );
        }

        if let Err(e) = state.settings_manager.update(settings).await {
            tracing::debug!(error = %e, "Failed to persist window state");
        }
    }

    async fn restore_window_state_on_startup(app_handle: &tauri::AppHandle) {
        let window = app_handle
            .get_webview_window("main")
            .or_else(|| app_handle.webview_windows().values().next().cloned());

        let Some(window) = window else {
            return;
        };

        let state = app_handle.state::<AppState>();
        let settings = state.settings_manager.get().await;
        let ws = settings.ui.window;

        // Clamp to current monitor to avoid off-screen/oversized restores.
        let scale_factor = window.scale_factor().unwrap_or(1.0);
        let monitor_rect = match window.current_monitor() {
            Ok(Some(monitor)) => {
                let monitor_pos = monitor.position().to_logical::<f64>(scale_factor);
                let monitor_size = monitor.size().to_logical::<f64>(scale_factor);
                Some(window_state::MonitorRect {
                    x: monitor_pos.x,
                    y: monitor_pos.y,
                    width: monitor_size.width,
                    height: monitor_size.height,
                })
            }
            _ => None,
        };

        let Some(action) = window_state::compute_restore_action(&ws, monitor_rect) else {
            return;
        };

        match action {
            window_state::RestoreAction::Maximize => {
                let _ = window.maximize();
            }
            window_state::RestoreAction::Bounds {
                width,
                height,
                x,
                y,
            } => {
                let _ =
                    window.set_size(tauri::Size::Logical(tauri::LogicalSize::new(width, height)));
                if let (Some(x), Some(y)) = (x, y) {
                    let _ = window
                        .set_position(tauri::Position::Logical(tauri::LogicalPosition::new(x, y)));
                }
            }
        }
    }

    async fn persist_window_state_on_exit(app_handle: &tauri::AppHandle) {
        let window = app_handle
            .get_webview_window("main")
            .or_else(|| app_handle.webview_windows().values().next().cloned());

        let Some(window) = window else {
            return;
        };

        persist_window_state_from_webview_window(&window).await;
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_notification::init())
        .manage(app_state)
        .manage(history_manager)
        .manage(Arc::new(FileWatcherState::new()))
        .on_window_event(|window, event| {
            // Persist window bounds continuously (debounced) so dev restarts and Cmd+Q are reliable.
            static SAVE_SEQ: AtomicU64 = AtomicU64::new(0);

            match event {
                tauri::WindowEvent::Moved(_) | tauri::WindowEvent::Resized(_) => {
                    let seq = SAVE_SEQ.fetch_add(1, Ordering::SeqCst) + 1;
                    let window = window.clone();
                    tauri::async_runtime::spawn(async move {
                        tokio::time::sleep(std::time::Duration::from_millis(400)).await;
                        if SAVE_SEQ.load(Ordering::SeqCst) != seq {
                            return;
                        }
                        persist_window_state_from_window(&window).await;
                    });
                }
                _ => {}
            }
        })
        .setup(|app| {
            // Auto-initialize sidecar at startup
            let state = app.state::<AppState>();
            let sidecar_state = state.sidecar_state.clone();

            let app_handle = app.handle().clone();
            sidecar_state.set_app_handle(app_handle);

            // Spawn async initialization (settings access is async)
            let sidecar_settings = state.settings_manager.clone();
            tauri::async_runtime::spawn(async move {
                let settings = sidecar_settings.get().await;

                if !settings.sidecar.enabled {
                    tracing::debug!(
                        "[tauri-setup] Sidecar disabled in settings, skipping initialization"
                    );
                    return;
                }

                // Get workspace path from QBIT_WORKSPACE env var, or fall back to current_dir
                let workspace = std::env::var("QBIT_WORKSPACE")
                    .ok()
                    .map(|p| {
                        // Expand ~ to home directory
                        if p.starts_with("~/") {
                            dirs::home_dir()
                                .map(|home| home.join(&p[2..]))
                                .unwrap_or_else(|| std::path::PathBuf::from(&p))
                        } else {
                            std::path::PathBuf::from(&p)
                        }
                    })
                    .unwrap_or_else(|| {
                        std::env::current_dir()
                            .unwrap_or_else(|_| dirs::home_dir().unwrap_or_default())
                    });

                tracing::info!(
                    "[tauri-setup] Initializing sidecar for workspace: {:?}",
                    workspace
                );

                // Initialize sidecar
                if let Err(e) = sidecar_state.initialize(workspace).await {
                    tracing::warn!("[tauri-setup] Failed to initialize sidecar: {}", e);
                } else {
                    tracing::info!("[tauri-setup] Sidecar initialized successfully");
                }
            });

            // Build command index in the background (for auto input mode)
            {
                let command_index = state.command_index.clone();
                tauri::async_runtime::spawn_blocking(move || {
                    command_index.build();
                });
            }

            // Initialize MCP servers in the background (non-blocking)
            {
                let mcp_manager_slot = state.mcp_manager.clone();
                let app_handle = app.handle().clone();

                tauri::async_runtime::spawn(async move {
                    // Determine workspace path
                    let workspace = std::env::var("QBIT_WORKSPACE")
                        .ok()
                        .map(|p| {
                            if p.starts_with("~/") {
                                dirs::home_dir()
                                    .map(|home| home.join(&p[2..]))
                                    .unwrap_or_else(|| std::path::PathBuf::from(&p))
                            } else {
                                std::path::PathBuf::from(&p)
                            }
                        })
                        .unwrap_or_else(|| {
                            std::env::current_dir()
                                .unwrap_or_else(|_| dirs::home_dir().unwrap_or_default())
                        });

                    tracing::info!(
                        "[mcp] Starting background MCP initialization for workspace: {:?}",
                        workspace
                    );

                    // Emit "initializing" event
                    let _ = app_handle.emit(
                        "mcp-event",
                        serde_json::json!({
                            "type": "initializing",
                            "message": "Connecting to MCP servers..."
                        }),
                    );

                    // Load config
                    let config = match qbit_mcp::load_mcp_config(&workspace) {
                        Ok(c) => c,
                        Err(e) => {
                            tracing::warn!("[mcp] Failed to load MCP config: {}", e);
                            let _ = app_handle.emit(
                                "mcp-event",
                                serde_json::json!({
                                    "type": "error",
                                    "message": format!("Failed to load MCP config: {}", e)
                                }),
                            );
                            return;
                        }
                    };

                    if config.mcp_servers.is_empty() {
                        tracing::debug!("[mcp] No MCP servers configured, skipping initialization");
                        // Store empty manager so commands don't get "not initialized" errors
                        let manager = Arc::new(qbit_mcp::McpManager::new(
                            std::collections::HashMap::new(),
                        ));
                        *mcp_manager_slot.write().await = Some(manager);
                        let _ = app_handle.emit(
                            "mcp-event",
                            serde_json::json!({
                                "type": "ready",
                                "message": "No MCP servers configured",
                                "serverCount": 0,
                                "toolCount": 0
                            }),
                        );
                        return;
                    }

                    let server_count = config.mcp_servers.len();
                    let manager = Arc::new(qbit_mcp::McpManager::new(config.mcp_servers));

                    // Connect to all enabled servers (this is the slow part)
                    if let Err(e) = manager.connect_all().await {
                        tracing::warn!("[mcp] Some MCP servers failed to connect: {}", e);
                        // Non-fatal: continue with whatever connected
                    }

                    // Count tools from connected servers
                    let tool_count = manager
                        .list_tools()
                        .await
                        .map(|tools| tools.len())
                        .unwrap_or(0);

                    // Store the global manager
                    *mcp_manager_slot.write().await = Some(Arc::clone(&manager));

                    tracing::info!(
                        "[mcp] Background MCP initialization complete: {} servers, {} tools",
                        server_count,
                        tool_count
                    );

                    // Emit completion event to frontend
                    let _ = app_handle.emit(
                        "mcp-event",
                        serde_json::json!({
                            "type": "ready",
                            "message": format!("MCP ready: {} tools from {} servers", tool_count, server_count),
                            "serverCount": server_count,
                            "toolCount": tool_count
                        }),
                    );

                    // Refresh MCP tools on any bridges that were already created
                    // (e.g., if a session was initialized before MCP finished loading)
                    let app_state = app_handle.state::<AppState>();
                    let bridges = app_state.ai_state.bridges.read().await;
                    for (session_id, bridge) in bridges.iter() {
                        crate::ai::commands::setup_bridge_mcp_tools(bridge, &app_state).await;
                        tracing::debug!(
                            "[mcp] Refreshed MCP tools for session {} after background init",
                            session_id
                        );
                    }
                });
            }

            // Restore window state as early as possible (Rust-side, reliable on Cmd+Q/dev).
            let app_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                restore_window_state_on_startup(&app_handle).await;
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // PTY commands
            pty_create,
            pty_write,
            pty_resize,
            pty_destroy,
            pty_get_session,
            pty_get_foreground_process,
            // Completion commands
            list_path_completions,
            // Input classification (auto mode)
            classify_input,
            // Shell integration commands
            shell_integration_status,
            shell_integration_install,
            shell_integration_uninstall,
            get_git_branch,
            // Git commands
            git_status,
            git_diff,
            git_diff_staged,
            git_stage,
            git_unstage,
            git_commit,
            git_push,
            git_delete_worktree,
            // AI commands
            init_ai_agent,
            init_ai_agent_vertex,
            init_ai_agent_openai,
            init_ai_agent_unified,
            send_ai_prompt,
            execute_ai_tool,
            get_available_tools,
            list_sub_agents,
            get_sub_agent_model,
            set_sub_agent_model,
            shutdown_ai_agent,
            is_ai_initialized,
            // Isolated commit writer agent
            generate_commit_message,
            // Session-specific AI commands
            init_ai_session,
            shutdown_ai_session,
            is_ai_session_initialized,
            get_session_ai_config,
            send_ai_prompt_session,
            send_ai_prompt_with_attachments,
            get_vision_capabilities,
            clear_ai_conversation_session,
            get_ai_conversation_length_session,
            signal_frontend_ready,
            // Provider config commands
            get_openrouter_api_key,
            get_openai_api_key,
            get_project_settings,
            save_project_model,
            get_vertex_ai_config,
            load_env_file,
            update_ai_workspace,
            clear_ai_conversation,
            get_ai_conversation_length,
            // Session persistence commands
            list_ai_sessions,
            find_ai_session,
            load_ai_session,
            export_ai_session_transcript,
            set_ai_session_persistence,
            is_ai_session_persistence_enabled,
            finalize_ai_session,
            restore_ai_session,
            // HITL commands
            get_approval_patterns,
            get_tool_approval_pattern,
            get_hitl_config,
            set_hitl_config,
            add_tool_always_allow,
            remove_tool_always_allow,
            reset_approval_patterns,
            respond_to_tool_approval,
            // Tool policy commands
            get_tool_policy_config,
            set_tool_policy_config,
            get_tool_policy,
            set_tool_policy,
            reset_tool_policies,
            enable_full_auto_mode,
            disable_full_auto_mode,
            is_full_auto_mode_enabled,
            // Agent mode commands
            get_agent_mode,
            set_agent_mode,
            save_project_agent_mode,
            // Debug commands
            get_api_request_stats,
            // Plan management commands
            get_plan,
            // Context management commands
            get_context_summary,
            get_token_usage_stats,
            get_token_alert_level,
            get_context_utilization,
            get_remaining_tokens,
            reset_context_manager,
            get_context_trim_config,
            is_context_management_enabled,
            retry_compaction,
            // Loop protection commands
            get_loop_protection_config,
            set_loop_protection_config,
            get_loop_detector_stats,
            is_loop_detection_enabled,
            disable_loop_detection,
            enable_loop_detection,
            reset_loop_detector,
            // Indexer commands
            init_indexer,
            is_indexer_initialized,
            get_indexer_workspace,
            get_indexed_file_count,
            get_all_indexed_files,
            index_file,
            index_directory,
            search_code,
            search_files,
            shutdown_indexer,
            // Codebase management commands
            list_indexed_codebases,
            add_indexed_codebase,
            remove_indexed_codebase,
            reindex_codebase,
            migrate_codebase_index,
            update_codebase_memory_file,
            detect_memory_files,
            // Home view commands
            list_projects_for_home,
            list_recent_directories,
            // Worktree management commands
            list_git_branches,
            create_git_worktree,
            // Project config commands
            save_project,
            delete_project_config,
            list_project_configs,
            get_project_config,
            // Prompt commands
            list_prompts,
            read_prompt,
            // Skill commands
            list_skills,
            read_skill,
            read_skill_body,
            list_skill_files,
            read_skill_file,
            // File commands
            list_workspace_files,
            list_directory,
            read_workspace_file,
            write_workspace_file,
            stat_workspace_file,
            read_file_as_base64,
            watch_file,
            unwatch_file,
            unwatch_all_files,
            // Theme commands
            list_themes,
            read_theme,
            save_theme,
            delete_theme,
            save_theme_asset,
            get_theme_asset_path,
            // History commands
            add_command_history,
            add_prompt_history,
            load_history,
            search_history,
            clear_history,
            // Workflow commands (generic)
            list_workflows,
            start_workflow,
            step_workflow,
            run_workflow_to_completion,
            get_workflow_state,
            list_workflow_sessions,
            cancel_workflow,
            // Settings commands
            get_settings,
            update_settings,
            get_setting,
            set_setting,
            reset_settings,
            settings_file_exists,
            get_settings_path,
            reload_settings,
            save_window_state,
            get_window_state,
            is_langfuse_active,
            get_telemetry_stats,
            // Model registry commands
            get_available_models,
            get_model_by_id,
            get_model_capabilities_command,
            get_providers,
            // Sidecar commands (simplified markdown-based)
            sidecar_status,
            sidecar_initialize,
            sidecar_start_session,
            sidecar_end_session,
            sidecar_current_session,
            sidecar_resume_session,
            sidecar_get_session_state,
            sidecar_get_session_log,
            sidecar_get_injectable_context,
            sidecar_get_session_meta,
            sidecar_list_sessions,
            sidecar_get_config,
            sidecar_set_config,
            sidecar_shutdown,
            // L2: Staged patches (git format-patch style)
            sidecar_get_staged_patches,
            sidecar_get_applied_patches,
            sidecar_get_patch,
            sidecar_discard_patch,
            sidecar_get_current_staged_patches,
            sidecar_apply_patch,
            sidecar_apply_all_patches,
            sidecar_regenerate_patch,
            sidecar_update_patch_message,
            // L3: Project artifacts (auto-maintained docs)
            sidecar_get_pending_artifacts,
            sidecar_get_applied_artifacts,
            sidecar_get_artifact,
            sidecar_discard_artifact,
            sidecar_preview_artifact,
            sidecar_get_current_pending_artifacts,
            sidecar_apply_artifact,
            sidecar_apply_all_artifacts,
            sidecar_regenerate_artifacts,
            write_frontend_log,
            // MCP (Model Context Protocol) commands
            mcp_list_servers,
            mcp_list_tools,
            mcp_get_config,
            mcp_is_project_trusted,
            mcp_trust_project_config,
            mcp_has_project_config,
            mcp_connect,
            mcp_disconnect,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app_handle, event| {
            // Ensure we save window bounds on Cmd+Q / app quit, even if the frontend
            // doesn't get a chance to flush its debounced state.
            static EXITING: AtomicBool = AtomicBool::new(false);

            if let tauri::RunEvent::ExitRequested { api, .. } = event {
                if EXITING.swap(true, Ordering::SeqCst) {
                    return;
                }

                api.prevent_exit();
                let handle = app_handle.clone();
                tauri::async_runtime::spawn(async move {
                    persist_window_state_on_exit(&handle).await;
                    handle.exit(0);
                });
            }
        });
}
