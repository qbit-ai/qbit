use crate::error::{PtyError, Result};

use parking_lot::Mutex;

use portable_pty::{native_pty_system, Child, CommandBuilder, MasterPty, PtySize};
use serde::{Deserialize, Serialize};

use std::collections::HashMap;

use std::io::Read;

use std::io::Write;

use std::path::PathBuf;

use std::sync::Arc;

use std::thread;

use uuid::Uuid;

use super::parser::{OscEvent, TerminalParser};

use super::shell::{detect_shell, ShellIntegration};

// Import runtime types for the runtime-based emitter

use qbit_core::runtime::{QbitRuntime, RuntimeEvent};

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
trait PtyEventEmitter: Send + Sync + 'static {
    /// Emit terminal output data
    fn emit_output(&self, session_id: &str, data: &str);

    /// Emit session ended event
    fn emit_session_ended(&self, session_id: &str);

    /// Emit directory changed event
    fn emit_directory_changed(&self, session_id: &str, path: &str);

    /// Emit virtual environment changed event
    fn emit_virtual_env_changed(&self, session_id: &str, name: Option<&str>);

    /// Emit command block event (prompt start/end, command start/end)
    fn emit_command_block(&self, event_name: &str, event: CommandBlockEvent);

    /// Emit alternate screen buffer state change
    /// Used to trigger fullterm mode for TUI applications
    fn emit_alternate_screen(&self, session_id: &str, enabled: bool);

    /// Emit synchronized output mode change (DEC 2026)
    /// Used to batch terminal updates atomically to prevent flickering
    fn emit_synchronized_output(&self, session_id: &str, enabled: bool);
}

// ============================================================================
// RuntimeEmitter - QbitRuntime-based implementation
// ============================================================================

/// Event emitter that uses QbitRuntime for CLI and other non-Tauri environments.
///
/// This emitter converts PTY events to `RuntimeEvent` variants and emits them
/// through the runtime's `emit()` method. This allows the CLI to receive
/// terminal events through the same abstraction used for AI events.
struct RuntimeEmitter(Arc<dyn QbitRuntime>);

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

    fn emit_virtual_env_changed(&self, session_id: &str, name: Option<&str>) {
        tracing::debug!(
            session_id = %session_id,
            name = ?name,
            "Emitting virtual_env_changed"
        );
        if let Err(e) = self.0.emit(RuntimeEvent::Custom {
            name: "virtual_env_changed".to_string(),
            payload: serde_json::json!({
                "session_id": session_id,
                "name": name
            }),
        }) {
            tracing::warn!(
                session_id = %session_id,
                name = ?name,
                error = %e,
                "Failed to emit virtual_env_changed event"
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
        tracing::trace!(
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

    fn emit_synchronized_output(&self, session_id: &str, enabled: bool) {
        tracing::debug!(
            session_id = %session_id,
            enabled = enabled,
            "Emitting synchronized_output"
        );
        if let Err(e) = self.0.emit(RuntimeEvent::Custom {
            name: "synchronized_output".to_string(),
            payload: serde_json::json!({
                "session_id": session_id,
                "enabled": enabled
            }),
        }) {
            tracing::warn!(
                session_id = %session_id,
                enabled = enabled,
                error = %e,
                "Failed to emit synchronized_output event"
            );
        }
    }
}

// ============================================================================
// UTF-8 Buffer Handling - Prevents corruption of multi-byte characters at
// buffer boundaries when reading PTY output
// ============================================================================

/// Buffer for holding incomplete UTF-8 sequences between PTY reads.
/// Max UTF-8 char is 4 bytes, so we buffer up to 3 trailing bytes.
struct Utf8IncompleteBuffer {
    bytes: [u8; 3],
    len: u8,
}

impl Utf8IncompleteBuffer {
    fn new() -> Self {
        Self {
            bytes: [0; 3],
            len: 0,
        }
    }

    fn has_pending(&self) -> bool {
        self.len > 0
    }

    fn as_slice(&self) -> &[u8] {
        &self.bytes[..self.len as usize]
    }

    fn clear(&mut self) {
        self.len = 0;
    }

    fn store(&mut self, bytes: &[u8]) {
        let len = bytes.len().min(3);
        self.bytes[..len].copy_from_slice(&bytes[..len]);
        self.len = len as u8;
    }
}

/// Find boundary where valid complete UTF-8 ends.
/// Returns the index up to which the data is valid UTF-8.
fn find_valid_utf8_boundary(data: &[u8]) -> usize {
    if data.is_empty() {
        return 0;
    }

    // Check last 1-3 bytes for incomplete sequences
    for check_len in 1..=3.min(data.len()) {
        let start_idx = data.len() - check_len;
        if is_incomplete_utf8_start(&data[start_idx..]) {
            return start_idx;
        }
    }

    // Verify entire buffer
    match std::str::from_utf8(data) {
        Ok(_) => data.len(),
        Err(e) => e.valid_up_to(),
    }
}

/// Check if bytes are start of incomplete UTF-8 sequence.
fn is_incomplete_utf8_start(bytes: &[u8]) -> bool {
    if bytes.is_empty() {
        return false;
    }

    let expected_len = match bytes[0] {
        b if b & 0b1000_0000 == 0 => 1,           // ASCII
        b if b & 0b1110_0000 == 0b1100_0000 => 2, // 2-byte
        b if b & 0b1111_0000 == 0b1110_0000 => 3, // 3-byte
        b if b & 0b1111_1000 == 0b1111_0000 => 4, // 4-byte
        _ => return false,                        // Invalid lead or continuation byte
    };

    if bytes.len() >= expected_len {
        return false; // Complete sequence
    }

    // Verify remaining bytes are valid continuation bytes
    bytes[1..].iter().all(|&b| b & 0b1100_0000 == 0b1000_0000)
}

/// Process bytes into valid UTF-8, buffering incomplete sequences.
fn process_utf8_with_buffer(buf: &mut Utf8IncompleteBuffer, data: &[u8]) -> String {
    if !buf.has_pending() {
        let valid_len = find_valid_utf8_boundary(data);
        if valid_len < data.len() {
            buf.store(&data[valid_len..]);
        }
        return String::from_utf8_lossy(&data[..valid_len]).to_string();
    }

    // Combine pending + new data
    let mut combined = Vec::with_capacity(buf.as_slice().len() + data.len());
    combined.extend_from_slice(buf.as_slice());
    combined.extend_from_slice(data);
    buf.clear();

    let valid_len = find_valid_utf8_boundary(&combined);
    if valid_len < combined.len() {
        buf.store(&combined[valid_len..]);
    }
    String::from_utf8_lossy(&combined[..valid_len]).to_string()
}

/// Messages sent from the PTY reader thread to the output emitter thread.
///
/// The reader thread sends raw output bytes through this channel so the emitter
/// thread can coalesce bursts of small reads into batched Tauri IPC events.
enum OutputMessage {
    Data(Vec<u8>),
    Eof,
}

#[allow(dead_code)] // Used by Tauri feature
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PtySession {
    pub id: String,
    pub working_directory: String,
    pub rows: u16,
    pub cols: u16,
}

#[allow(dead_code)] // Used by Tauri feature
#[derive(Debug, Clone, Serialize)]
pub struct CommandBlockEvent {
    pub session_id: String,
    pub command: Option<String>,
    pub exit_code: Option<i32>,
    pub event_type: String,
}

/// Internal session state tracking active PTY sessions.
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
            .map_err(|e| PtyError::Pty(e.to_string()))?;

        // Detect shell from environment (settings integration can be added later)
        let shell_env = std::env::var("SHELL").ok();
        let shell_info = detect_shell(None, shell_env.as_deref());

        tracing::info!(
            "Spawning shell: {} (detected type: {:?})",
            shell_info.path.display(),
            shell_info.shell_type()
        );

        let mut cmd = CommandBuilder::new(shell_info.path.to_str().unwrap_or("/bin/sh"));

        // Set up shell integration (ZDOTDIR for zsh, --rcfile for bash, etc.)
        // This injects OSC 133 sequences automatically without requiring config file edits
        let integration = ShellIntegration::setup(shell_info.shell_type());

        // For shells with integration that provides custom args (like bash --rcfile),
        // use those instead of the default login args
        let shell_args = integration.as_ref().map(|i| i.shell_args());
        if let Some(ref args) = shell_args {
            if !args.is_empty() {
                tracing::debug!(
                    session_id = %session_id,
                    args = ?args,
                    "Using integration shell args"
                );
                for arg in args {
                    cmd.arg(arg);
                }
            } else {
                cmd.args(shell_info.login_args());
            }
        } else {
            cmd.args(shell_info.login_args());
        }

        cmd.env("QBIT", "1");
        cmd.env("QBIT_VERSION", env!("CARGO_PKG_VERSION"));
        cmd.env("TERM", "xterm-256color");
        // Note: Set QBIT_DEBUG=1 to enable shell integration debug output

        // Set integration environment variables
        if let Some(integration) = integration {
            for (key, value) in integration.env_vars() {
                tracing::debug!(
                    session_id = %session_id,
                    key = %key,
                    value = %value,
                    "Setting shell integration env var"
                );
                cmd.env(key, value);
            }
        }

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
            // If cwd is root "/", fall through to home_dir - this happens when launched from Finder
            if cwd.as_os_str() == "/" {
                (
                    dirs::home_dir().unwrap_or_else(|| PathBuf::from("/")),
                    "home_dir (cwd was root)",
                )
            // If we're in src-tauri, go up to project root
            } else if cwd.ends_with("src-tauri") {
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
            .map_err(|e| PtyError::Pty(e.to_string()))?;

        let writer = pair
            .master
            .take_writer()
            .map_err(|e| PtyError::Pty(e.to_string()))?;

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
                .map_err(|e| PtyError::Pty(e.to_string()))?
        };

        // Channel for passing raw output bytes from the reader thread to the emitter thread.
        // This allows the emitter thread to coalesce bursts of small reads into batched
        // Tauri IPC events (targeting ~60 fps / 16 ms batching window).
        let (output_tx, output_rx) = std::sync::mpsc::channel::<OutputMessage>();

        // Clone emitter for the output emitter thread (reader thread keeps the original).
        let emitter_for_output = emitter.clone();
        let output_session_id = session_id.clone();

        // Spawn reader thread
        let reader_session_id_for_log = reader_session_id.clone();
        tracing::trace!(
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
            // Note: utf8_buffer moved to emitter thread — UTF-8 boundary handling happens there.

            loop {
                match reader.read(&mut buf) {
                    Ok(0) => {
                        tracing::debug!(
                            session_id = %reader_session_id,
                            total_bytes = total_bytes_read,
                            "PTY reader received EOF"
                        );
                        // Signal EOF to emitter thread; it will flush pending UTF-8 bytes
                        // and emit session_ended.
                        let _ = output_tx.send(OutputMessage::Eof);
                        break;
                    }
                    Ok(n) => {
                        total_bytes_read += n as u64;
                        let data = &buf[..n];

                        // Parse and filter: only Output region bytes are returned
                        // Prompt (A→B) and Input (B→C) regions are suppressed
                        let parse_result = parser.parse_filtered(data);

                        // Log parsed OSC events at trace level
                        if !parse_result.events.is_empty() {
                            tracing::trace!(
                                session_id = %reader_session_id,
                                event_count = parse_result.events.len(),
                                events = ?parse_result.events,
                                "Parsed OSC events"
                            );
                        }

                        // Semantic events are emitted directly from the reader thread.
                        // The corresponding output bytes for the same reads are queued in
                        // the channel. Delivery ordering of semantic vs. output events via
                        // Tauri IPC was never strictly guaranteed, so this is acceptable.
                        for event in parse_result.events {
                            match &event {
                                OscEvent::DirectoryChanged { path } => {
                                    // Update the session's working directory so path completion
                                    // uses the current directory, not the initial one
                                    let new_path = PathBuf::from(path);
                                    let mut current = reader_session.working_directory.lock();
                                    // Only emit if the directory actually changed
                                    if *current != new_path {
                                        // DEBUG: Log with more context to trace directory changes
                                        tracing::warn!(
                                            session_id = %reader_session_id,
                                            old_dir = %current.display(),
                                            new_dir = %new_path.display(),
                                            "[cwd-debug] PTY manager emitting directory_changed event"
                                        );
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
                                OscEvent::VirtualEnvChanged { name } => {
                                    // Emit virtual environment change to frontend
                                    emitter.emit_virtual_env_changed(
                                        &reader_session_id,
                                        name.as_deref(),
                                    );
                                }
                                OscEvent::AlternateScreenEnabled => {
                                    emitter.emit_alternate_screen(&reader_session_id, true);
                                }
                                OscEvent::AlternateScreenDisabled => {
                                    emitter.emit_alternate_screen(&reader_session_id, false);
                                }
                                OscEvent::SynchronizedOutputEnabled => {
                                    emitter.emit_synchronized_output(&reader_session_id, true);
                                }
                                OscEvent::SynchronizedOutputDisabled => {
                                    emitter.emit_synchronized_output(&reader_session_id, false);
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

                        // Send raw output bytes to the emitter thread for coalescing.
                        // UTF-8 boundary handling happens in the emitter thread.
                        if !parse_result.output.is_empty() {
                            let _ = output_tx.send(OutputMessage::Data(parse_result.output));
                        }
                    }
                    Err(e) => {
                        tracing::error!(
                            session_id = %reader_session_id,
                            error = %e,
                            error_kind = ?e.kind(),
                            total_bytes = total_bytes_read,
                            "PTY read error"
                        );
                        let _ = output_tx.send(OutputMessage::Eof);
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

        // Spawn output emitter thread.
        //
        // Receives raw output bytes from the reader thread and coalesces bursts into
        // batched emit calls. TUI apps doing full-screen redraws produce many small reads
        // per frame; without coalescing these become a flood of Tauri IPC events that
        // saturate the bridge. The 16 ms timeout targets ~60 fps.
        thread::spawn(move || {
            let mut utf8_buffer = Utf8IncompleteBuffer::new();
            let mut coalesce_buf: Vec<u8> = Vec::with_capacity(16 * 1024);
            let timeout = std::time::Duration::from_millis(16);

            loop {
                match output_rx.recv_timeout(timeout) {
                    Ok(OutputMessage::Data(bytes)) => {
                        coalesce_buf.extend_from_slice(&bytes);

                        // Drain all immediately-queued messages without blocking,
                        // coalescing them into a single emit call.
                        loop {
                            match output_rx.try_recv() {
                                Ok(OutputMessage::Data(more)) => {
                                    coalesce_buf.extend_from_slice(&more);
                                }
                                Ok(OutputMessage::Eof) => {
                                    // Flush coalesced bytes, then emit session_ended.
                                    let output =
                                        process_utf8_with_buffer(&mut utf8_buffer, &coalesce_buf);
                                    if !output.is_empty() {
                                        emitter_for_output.emit_output(&output_session_id, &output);
                                    }
                                    if utf8_buffer.has_pending() {
                                        let remaining =
                                            String::from_utf8_lossy(utf8_buffer.as_slice())
                                                .to_string();
                                        if !remaining.is_empty() {
                                            emitter_for_output
                                                .emit_output(&output_session_id, &remaining);
                                        }
                                    }
                                    emitter_for_output.emit_session_ended(&output_session_id);
                                    return;
                                }
                                Err(_) => break,
                            }
                        }

                        // Emit the coalesced batch.
                        let output = process_utf8_with_buffer(&mut utf8_buffer, &coalesce_buf);
                        if !output.is_empty() {
                            emitter_for_output.emit_output(&output_session_id, &output);
                        }
                        coalesce_buf.clear();
                    }
                    Ok(OutputMessage::Eof) => {
                        // Flush any incomplete UTF-8 sequence, then signal session end.
                        if utf8_buffer.has_pending() {
                            let remaining =
                                String::from_utf8_lossy(utf8_buffer.as_slice()).to_string();
                            if !remaining.is_empty() {
                                emitter_for_output.emit_output(&output_session_id, &remaining);
                            }
                        }
                        emitter_for_output.emit_session_ended(&output_session_id);
                        return;
                    }
                    Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                        // Idle — nothing to flush.
                    }
                    Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                        // Reader thread exited without sending an explicit Eof message.
                        emitter_for_output.emit_session_ended(&output_session_id);
                        return;
                    }
                }
            }
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

    pub fn write(&self, session_id: &str, data: &[u8]) -> Result<()> {
        let sessions = self.sessions.lock();
        let session = sessions
            .get(session_id)
            .ok_or_else(|| PtyError::SessionNotFound(session_id.to_string()))?;

        let mut writer = session.writer.lock();
        writer.write_all(data).map_err(PtyError::Io)?;
        writer.flush().map_err(PtyError::Io)?;

        Ok(())
    }

    pub fn resize(&self, session_id: &str, rows: u16, cols: u16) -> Result<()> {
        let sessions = self.sessions.lock();
        let session = sessions
            .get(session_id)
            .ok_or_else(|| PtyError::SessionNotFound(session_id.to_string()))?;

        let old_rows = *session.rows.lock();
        let old_cols = *session.cols.lock();

        // Skip resize if dimensions haven't changed
        if old_rows == rows && old_cols == cols {
            return Ok(());
        }

        let master = session.master.lock();
        master
            .resize(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| PtyError::Pty(e.to_string()))?;

        *session.rows.lock() = rows;
        *session.cols.lock() = cols;

        tracing::trace!(
            session_id = %session_id,
            old_size = %format!("{}x{}", old_cols, old_rows),
            new_size = %format!("{}x{}", cols, rows),
            "PTY resized"
        );

        Ok(())
    }

    pub fn destroy(&self, session_id: &str) -> Result<()> {
        let mut sessions = self.sessions.lock();
        let session_count_before = sessions.len();

        sessions
            .remove(session_id)
            .ok_or_else(|| PtyError::SessionNotFound(session_id.to_string()))?;

        tracing::info!(
            session_id = %session_id,
            sessions_before = session_count_before,
            sessions_after = sessions.len(),
            "PTY session destroyed"
        );

        Ok(())
    }

    pub fn get_session(&self, session_id: &str) -> Result<PtySession> {
        let sessions = self.sessions.lock();
        let session = sessions
            .get(session_id)
            .ok_or_else(|| PtyError::SessionNotFound(session_id.to_string()))?;

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
    pub fn get_foreground_process(&self, session_id: &str) -> Result<Option<String>> {
        use std::process::Command;

        // Verify session exists
        let sessions = self.sessions.lock();
        if !sessions.contains_key(session_id) {
            return Err(PtyError::SessionNotFound(session_id.to_string()));
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
