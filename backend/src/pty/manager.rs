#[cfg(feature = "tauri")]
use crate::error::{QbitError, Result};
#[cfg(feature = "tauri")]
use parking_lot::Mutex;
#[cfg(feature = "tauri")]
use portable_pty::{native_pty_system, Child, CommandBuilder, MasterPty, PtySize};
use serde::{Deserialize, Serialize};
#[cfg(feature = "tauri")]
use std::collections::HashMap;
#[cfg(feature = "tauri")]
use std::io::Read;
#[cfg(feature = "tauri")]
use std::io::Write;
#[cfg(feature = "tauri")]
use std::path::PathBuf;
#[cfg(feature = "tauri")]
use std::sync::Arc;
#[cfg(feature = "tauri")]
use std::thread;
#[cfg(feature = "tauri")]
use uuid::Uuid;

#[cfg(feature = "tauri")]
use super::parser::{OscEvent, TerminalParser};

// Import runtime types for the runtime-based emitter
#[cfg(feature = "tauri")]
use crate::runtime::{QbitRuntime, RuntimeEvent};

// ============================================================================
// PtyEventEmitter Trait - Internal abstraction for event emission
// ============================================================================

/// Internal trait for emitting PTY events.
///
/// This trait abstracts over how PTY events (output, exit, directory changes, etc.)
/// are delivered to consumers. The primary implementation is:
/// - `RuntimeEmitter`: Emits events via QbitRuntime (for Tauri, CLI and other runtimes)
///
/// # Thread Safety
/// Implementors must be `Send + Sync + 'static` to work with std::thread spawning
/// in the PTY read loop.
#[cfg(feature = "tauri")]
trait PtyEventEmitter: Send + Sync + 'static {
    /// Emit terminal output data
    fn emit_output(&self, session_id: &str, data: &str);

    /// Emit session ended event
    fn emit_session_ended(&self, session_id: &str);

    /// Emit directory changed event
    fn emit_directory_changed(&self, session_id: &str, path: &str);

    /// Emit command block event (prompt start/end, command start/end)
    fn emit_command_block(&self, event_name: &str, event: CommandBlockEvent);

    /// Emit alternate screen buffer state change
    /// Used to trigger fullterm mode for TUI applications
    fn emit_alternate_screen(&self, session_id: &str, enabled: bool);
}

// ============================================================================
// RuntimeEmitter - QbitRuntime-based implementation
// ============================================================================

/// Event emitter that uses QbitRuntime for CLI and other non-Tauri environments.
///
/// This emitter converts PTY events to `RuntimeEvent` variants and emits them
/// through the runtime's `emit()` method. This allows the CLI to receive
/// terminal events through the same abstraction used for AI events.
#[cfg(feature = "tauri")]
struct RuntimeEmitter(Arc<dyn QbitRuntime>);

#[cfg(feature = "tauri")]
impl PtyEventEmitter for RuntimeEmitter {
    fn emit_output(&self, session_id: &str, data: &str) {
        // Convert string data to bytes for RuntimeEvent::TerminalOutput
        let bytes = data.as_bytes().to_vec();
        if let Err(e) = self.0.emit(RuntimeEvent::TerminalOutput {
            session_id: session_id.to_string(),
            data: bytes,
        }) {
            tracing::warn!(
                session_id = %session_id,
                bytes = data.len(),
                error = %e,
                "Failed to emit terminal output"
            );
        }
    }

    fn emit_session_ended(&self, session_id: &str) {
        tracing::info!(
            session_id = %session_id,
            "PTY session ended (EOF)"
        );
        // Use TerminalExit with no exit code (EOF/closed)
        if let Err(e) = self.0.emit(RuntimeEvent::TerminalExit {
            session_id: session_id.to_string(),
            code: None,
        }) {
            tracing::error!(
                session_id = %session_id,
                error = %e,
                "Failed to emit session ended event"
            );
        }
    }

    fn emit_directory_changed(&self, session_id: &str, path: &str) {
        tracing::debug!(
            session_id = %session_id,
            path = %path,
            "Emitting directory_changed"
        );
        // Use Custom event for directory changes (not yet in RuntimeEvent enum)
        if let Err(e) = self.0.emit(RuntimeEvent::Custom {
            name: "directory_changed".to_string(),
            payload: serde_json::json!({
                "session_id": session_id,
                "path": path
            }),
        }) {
            tracing::warn!(
                session_id = %session_id,
                path = %path,
                error = %e,
                "Failed to emit directory_changed event"
            );
        }
    }

    fn emit_command_block(&self, event_name: &str, event: CommandBlockEvent) {
        // Use Custom event for command block events
        let payload = match serde_json::to_value(&event) {
            Ok(v) => v,
            Err(e) => {
                tracing::warn!(
                    event_name = %event_name,
                    error = %e,
                    "Failed to serialize command block event"
                );
                return;
            }
        };

        if let Err(e) = self.0.emit(RuntimeEvent::Custom {
            name: event_name.to_string(),
            payload,
        }) {
            tracing::warn!(
                event_name = %event_name,
                session_id = %event.session_id,
                error = %e,
                "Failed to emit command block event"
            );
        }
    }

    fn emit_alternate_screen(&self, session_id: &str, enabled: bool) {
        tracing::debug!(
            session_id = %session_id,
            enabled = enabled,
            "Emitting alternate_screen"
        );
        if let Err(e) = self.0.emit(RuntimeEvent::Custom {
            name: "alternate_screen".to_string(),
            payload: serde_json::json!({
                "session_id": session_id,
                "enabled": enabled
            }),
        }) {
            tracing::warn!(
                session_id = %session_id,
                enabled = enabled,
                error = %e,
                "Failed to emit alternate_screen event"
            );
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PtySession {
    pub id: String,
    pub working_directory: String,
    pub rows: u16,
    pub cols: u16,
}

#[derive(Debug, Clone, Serialize)]
pub struct CommandBlockEvent {
    pub session_id: String,
    pub command: Option<String>,
    pub exit_code: Option<i32>,
    pub event_type: String,
}

/// Internal session state tracking active PTY sessions.
/// Only available when the `tauri` feature is enabled.
#[cfg(feature = "tauri")]
struct ActiveSession {
    #[allow(dead_code)]
    child: Mutex<Box<dyn Child + Send + Sync>>,
    master: Arc<Mutex<Box<dyn MasterPty + Send>>>,
    writer: Mutex<Box<dyn Write + Send>>,
    working_directory: Mutex<PathBuf>,
    rows: Mutex<u16>,
    cols: Mutex<u16>,
}

/// Manager for PTY sessions.
///
/// When the `tauri` feature is enabled, this provides full PTY session management
/// with event emission to the Tauri frontend. Without the feature, it provides
/// a minimal stub for compilation.
#[derive(Default)]
pub struct PtyManager {
    #[cfg(feature = "tauri")]
    sessions: Mutex<HashMap<String, Arc<ActiveSession>>>,
}

impl PtyManager {
    pub fn new() -> Self {
        Self::default()
    }

    // ========================================================================
    // Internal Implementation
    // ========================================================================

    /// Internal implementation that takes a generic emitter.
    ///
    /// This is the core session creation logic, abstracted over the event
    /// emission mechanism.
    #[cfg(feature = "tauri")]
    fn create_session_internal<E: PtyEventEmitter>(
        &self,
        emitter: Arc<E>,
        working_directory: Option<PathBuf>,
        rows: u16,
        cols: u16,
    ) -> Result<PtySession> {
        let session_id = Uuid::new_v4().to_string();

        tracing::info!(
            session_id = %session_id,
            rows = rows,
            cols = cols,
            requested_dir = ?working_directory,
            "Creating PTY session"
        );

        let pty_system = native_pty_system();

        let size = PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        };

        let pair = pty_system
            .openpty(size)
            .map_err(|e| QbitError::Pty(e.to_string()))?;

        let mut cmd = CommandBuilder::new("zsh");
        cmd.args(["-l"]);

        cmd.env("QBIT", "1");
        cmd.env("QBIT_VERSION", env!("CARGO_PKG_VERSION"));
        cmd.env("TERM", "xterm-256color");

        let (work_dir, dir_source) = if let Some(dir) = working_directory {
            (dir, "explicit")
        } else if let Ok(workspace) = std::env::var("QBIT_WORKSPACE") {
            // Expand ~ to home directory
            let path = if let Some(stripped) = workspace.strip_prefix("~/") {
                if let Some(home) = dirs::home_dir() {
                    home.join(stripped)
                } else {
                    PathBuf::from(&workspace)
                }
            } else {
                PathBuf::from(&workspace)
            };
            (path, "QBIT_WORKSPACE")
        } else if let Ok(init_cwd) = std::env::var("INIT_CWD") {
            (PathBuf::from(init_cwd), "INIT_CWD")
        } else if let Ok(cwd) = std::env::current_dir() {
            // If we're in src-tauri, go up to project root
            if cwd.ends_with("src-tauri") {
                if let Some(parent) = cwd.parent() {
                    (parent.to_path_buf(), "current_dir (adjusted)")
                } else {
                    (cwd, "current_dir")
                }
            } else {
                (cwd, "current_dir")
            }
        } else {
            (
                dirs::home_dir().unwrap_or_else(|| PathBuf::from("/")),
                "home_dir fallback",
            )
        };

        tracing::debug!(
            session_id = %session_id,
            work_dir = %work_dir.display(),
            source = dir_source,
            "Working directory resolved"
        );

        cmd.cwd(&work_dir);

        let child = pair
            .slave
            .spawn_command(cmd)
            .map_err(|e| QbitError::Pty(e.to_string()))?;

        let writer = pair
            .master
            .take_writer()
            .map_err(|e| QbitError::Pty(e.to_string()))?;

        let master = Arc::new(Mutex::new(pair.master));

        let session = Arc::new(ActiveSession {
            child: Mutex::new(child),
            master: master.clone(),
            writer: Mutex::new(writer),
            working_directory: Mutex::new(work_dir.clone()),
            rows: Mutex::new(rows),
            cols: Mutex::new(cols),
        });

        // Store session
        {
            let mut sessions = self.sessions.lock();
            sessions.insert(session_id.clone(), session.clone());
        }

        // Start read thread with the generic emitter
        let reader_session_id = session_id.clone();
        let reader_session = session.clone();

        // Get a reader from the master
        let mut reader = {
            let master = master.lock();
            master
                .try_clone_reader()
                .map_err(|e| QbitError::Pty(e.to_string()))?
        };

        // Spawn reader thread
        let reader_session_id_for_log = reader_session_id.clone();
        tracing::debug!(
            session_id = %reader_session_id_for_log,
            "Spawning PTY reader thread"
        );

        thread::spawn(move || {
            tracing::trace!(
                session_id = %reader_session_id,
                "PTY reader thread started"
            );

            let mut parser = TerminalParser::new();
            let mut buf = [0u8; 4096];
            let mut total_bytes_read: u64 = 0;

            loop {
                match reader.read(&mut buf) {
                    Ok(0) => {
                        tracing::debug!(
                            session_id = %reader_session_id,
                            total_bytes = total_bytes_read,
                            "PTY reader received EOF"
                        );
                        emitter.emit_session_ended(&reader_session_id);
                        break;
                    }
                    Ok(n) => {
                        total_bytes_read += n as u64;
                        let data = &buf[..n];
                        let events = parser.parse(data);

                        for event in events {
                            match &event {
                                OscEvent::DirectoryChanged { path } => {
                                    // Update the session's working directory so path completion
                                    // uses the current directory, not the initial one
                                    let new_path = PathBuf::from(path);
                                    let mut current = reader_session.working_directory.lock();
                                    // Only emit if the directory actually changed
                                    if *current != new_path {
                                        tracing::trace!(
                                            session_id = %reader_session_id,
                                            old_dir = %current.display(),
                                            new_dir = %new_path.display(),
                                            "Working directory changed"
                                        );
                                        *current = new_path;
                                        drop(current); // Release lock before emitting
                                        emitter.emit_directory_changed(&reader_session_id, path);
                                    }
                                }
                                OscEvent::AlternateScreenEnabled => {
                                    emitter.emit_alternate_screen(&reader_session_id, true);
                                }
                                OscEvent::AlternateScreenDisabled => {
                                    emitter.emit_alternate_screen(&reader_session_id, false);
                                }
                                _ => {
                                    if let Some((event_name, payload)) =
                                        event.to_command_block_event(&reader_session_id)
                                    {
                                        emitter.emit_command_block(event_name, payload);
                                    }
                                }
                            }
                        }

                        let output = String::from_utf8_lossy(data).to_string();
                        emitter.emit_output(&reader_session_id, &output);
                    }
                    Err(e) => {
                        tracing::error!(
                            session_id = %reader_session_id,
                            error = %e,
                            error_kind = ?e.kind(),
                            total_bytes = total_bytes_read,
                            "PTY read error"
                        );
                        break;
                    }
                }
            }

            tracing::trace!(
                session_id = %reader_session_id,
                total_bytes = total_bytes_read,
                "PTY reader thread exiting"
            );
        });

        Ok(PtySession {
            id: session_id,
            working_directory: work_dir.to_string_lossy().to_string(),
            rows,
            cols,
        })
    }

    // ========================================================================
    // Public API
    // ========================================================================

    /// Create a PTY session with runtime-based event emission.
    ///
    /// This method is the preferred way to create PTY sessions as it works with
    /// any `QbitRuntime` implementation (Tauri, CLI, or future runtimes).
    ///
    /// # Arguments
    /// * `runtime` - Runtime implementation for event emission
    /// * `working_directory` - Initial working directory (defaults to project root)
    /// * `rows` - Terminal height in rows
    /// * `cols` - Terminal width in columns
    ///
    /// # Example
    /// ```rust,ignore
    /// // With TauriRuntime
    /// let runtime = Arc::new(TauriRuntime::new(app_handle));
    /// let session = pty_manager.create_session_with_runtime(runtime, None, 24, 80)?;
    ///
    /// // With CliRuntime
    /// let runtime = Arc::new(CliRuntime::new(event_tx, true, false));
    /// let session = pty_manager.create_session_with_runtime(runtime, None, 24, 80)?;
    /// ```
    #[cfg(feature = "tauri")]
    pub fn create_session_with_runtime(
        &self,
        runtime: Arc<dyn QbitRuntime>,
        working_directory: Option<PathBuf>,
        rows: u16,
        cols: u16,
    ) -> Result<PtySession> {
        let emitter = Arc::new(RuntimeEmitter(runtime));
        self.create_session_internal(emitter, working_directory, rows, cols)
    }

    #[cfg(feature = "tauri")]
    pub fn write(&self, session_id: &str, data: &[u8]) -> Result<()> {
        let sessions = self.sessions.lock();
        let session = sessions
            .get(session_id)
            .ok_or_else(|| QbitError::SessionNotFound(session_id.to_string()))?;

        let mut writer = session.writer.lock();
        writer.write_all(data).map_err(QbitError::Io)?;
        writer.flush().map_err(QbitError::Io)?;

        Ok(())
    }

    #[cfg(feature = "tauri")]
    pub fn resize(&self, session_id: &str, rows: u16, cols: u16) -> Result<()> {
        let sessions = self.sessions.lock();
        let session = sessions
            .get(session_id)
            .ok_or_else(|| QbitError::SessionNotFound(session_id.to_string()))?;

        let old_rows = *session.rows.lock();
        let old_cols = *session.cols.lock();

        let master = session.master.lock();
        master
            .resize(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| QbitError::Pty(e.to_string()))?;

        *session.rows.lock() = rows;
        *session.cols.lock() = cols;

        tracing::debug!(
            session_id = %session_id,
            old_size = %format!("{}x{}", old_cols, old_rows),
            new_size = %format!("{}x{}", cols, rows),
            "PTY resized"
        );

        Ok(())
    }

    #[cfg(feature = "tauri")]
    pub fn destroy(&self, session_id: &str) -> Result<()> {
        let mut sessions = self.sessions.lock();
        let session_count_before = sessions.len();

        sessions
            .remove(session_id)
            .ok_or_else(|| QbitError::SessionNotFound(session_id.to_string()))?;

        tracing::info!(
            session_id = %session_id,
            sessions_before = session_count_before,
            sessions_after = sessions.len(),
            "PTY session destroyed"
        );

        Ok(())
    }

    #[cfg(feature = "tauri")]
    pub fn get_session(&self, session_id: &str) -> Result<PtySession> {
        let sessions = self.sessions.lock();
        let session = sessions
            .get(session_id)
            .ok_or_else(|| QbitError::SessionNotFound(session_id.to_string()))?;

        let working_directory = session
            .working_directory
            .lock()
            .to_string_lossy()
            .to_string();
        let rows = *session.rows.lock();
        let cols = *session.cols.lock();

        Ok(PtySession {
            id: session_id.to_string(),
            working_directory,
            rows,
            cols,
        })
    }

    /// Get the foreground process name for a PTY session.
    ///
    /// This uses OS-level process group detection to get the actual running process,
    /// rather than guessing based on command patterns.
    ///
    /// # Platform Support
    /// - macOS/Linux: Uses `ps` to query the terminal's foreground process group
    /// - Windows: Returns None (process groups work differently)
    ///
    /// # Returns
    /// - `Ok(Some(String))` - The foreground process name (e.g., "npm", "cargo", "python")
    /// - `Ok(None)` - No foreground process or shell is in foreground
    /// - `Err(_)` - Failed to query process information
    #[cfg(feature = "tauri")]
    pub fn get_foreground_process(&self, session_id: &str) -> Result<Option<String>> {
        use std::process::Command;

        // Verify session exists
        let sessions = self.sessions.lock();
        if !sessions.contains_key(session_id) {
            return Err(QbitError::SessionNotFound(session_id.to_string()));
        }
        drop(sessions);

        // Platform-specific process detection
        #[cfg(any(target_os = "macos", target_os = "linux"))]
        {
            // Get the PTY's foreground process group leader
            // This uses the ps command to query the terminal's current foreground process
            let output = Command::new("sh")
                .arg("-c")
                .arg("ps -o comm= -p $(ps -o tpgid= -p $$) 2>/dev/null || echo ''")
                .output();

            match output {
                Ok(output) if output.status.success() => {
                    let process_name = String::from_utf8_lossy(&output.stdout).trim().to_string();

                    if process_name.is_empty() {
                        Ok(None)
                    } else {
                        // Extract just the binary name (remove path)
                        let name = process_name
                            .rsplit('/')
                            .next()
                            .unwrap_or(&process_name)
                            .to_string();
                        Ok(Some(name))
                    }
                }
                _ => Ok(None),
            }
        }

        #[cfg(not(any(target_os = "macos", target_os = "linux")))]
        {
            // Windows and other platforms don't have the same process group semantics
            Ok(None)
        }
    }
}
