//! Shell command execution tool: run_pty_cmd.
//!
//! This module provides shell command execution with proper PATH inheritance
//! from the user's shell configuration files (.zshrc, .bashrc, etc.).
//!
//! ## Streaming Support
//!
//! For long-running commands, use `execute_streaming` instead of `execute` to
//! receive output chunks as they arrive. This provides real-time feedback
//! without waiting for the command to complete.

use std::path::{Path, PathBuf};
use std::process::Stdio;

use anyhow::Result;
use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::mpsc;
use tracing::debug;

// Re-export the Tool trait from qbit-core for convenience
pub use qbit_core::Tool;

// ============================================================================
// Shell Detection
// ============================================================================

/// Supported shell types for PATH inheritance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ShellType {
    Zsh,
    Bash,
    Fish,
    Sh,
}

impl ShellType {
    /// Detect shell type from path.
    fn from_path(path: &Path) -> Self {
        let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        match file_name {
            "zsh" => ShellType::Zsh,
            "bash" => ShellType::Bash,
            "fish" => ShellType::Fish,
            _ => ShellType::Sh,
        }
    }

    /// Get the rc file path for this shell type.
    fn rc_file(&self, home: &Path) -> Option<PathBuf> {
        match self {
            ShellType::Zsh => Some(home.join(".zshrc")),
            ShellType::Bash => {
                // Bash uses .bashrc for interactive non-login shells
                // Check .bashrc first, then .bash_profile
                let bashrc = home.join(".bashrc");
                if bashrc.exists() {
                    Some(bashrc)
                } else {
                    let bash_profile = home.join(".bash_profile");
                    if bash_profile.exists() {
                        Some(bash_profile)
                    } else {
                        None
                    }
                }
            }
            ShellType::Fish => Some(home.join(".config/fish/config.fish")),
            ShellType::Sh => None,
        }
    }

    /// Build the command to execute with proper PATH loaded.
    ///
    /// The strategy is:
    /// 1. For zsh/bash: Source the rc file explicitly before running the command
    /// 2. For fish: Use fish -c with source command
    /// 3. For sh: Just run directly (no rc file)
    fn build_command(
        &self,
        shell_path: &Path,
        user_command: &str,
        home: &Path,
    ) -> (String, String) {
        match self {
            ShellType::Zsh => {
                let rc_file = home.join(".zshrc");
                if rc_file.exists() {
                    // Source .zshrc then run the command
                    // Use emulate sh -c to avoid issues with zsh-specific syntax in sourced file
                    let wrapped =
                        format!("source {} 2>/dev/null; {}", rc_file.display(), user_command);
                    (shell_path.to_string_lossy().to_string(), wrapped)
                } else {
                    (
                        shell_path.to_string_lossy().to_string(),
                        user_command.to_string(),
                    )
                }
            }
            ShellType::Bash => {
                // For bash, source .bashrc if it exists
                if let Some(rc_file) = self.rc_file(home) {
                    let wrapped =
                        format!("source {} 2>/dev/null; {}", rc_file.display(), user_command);
                    (shell_path.to_string_lossy().to_string(), wrapped)
                } else {
                    (
                        shell_path.to_string_lossy().to_string(),
                        user_command.to_string(),
                    )
                }
            }
            ShellType::Fish => {
                let rc_file = home.join(".config/fish/config.fish");
                if rc_file.exists() {
                    // Fish uses a different syntax for sourcing
                    let wrapped =
                        format!("source {} 2>/dev/null; {}", rc_file.display(), user_command);
                    (shell_path.to_string_lossy().to_string(), wrapped)
                } else {
                    (
                        shell_path.to_string_lossy().to_string(),
                        user_command.to_string(),
                    )
                }
            }
            ShellType::Sh => {
                // For sh, just run the command directly
                ("/bin/sh".to_string(), user_command.to_string())
            }
        }
    }
}

/// Get shell configuration.
///
/// Shell resolution order:
/// 1. `shell_override` parameter (from settings.toml `terminal.shell`)
/// 2. `$SHELL` environment variable
/// 3. Fall back to `/bin/sh`
///
/// Returns (shell_path, shell_type, home_dir)
fn get_shell_config(shell_override: Option<&str>) -> (PathBuf, ShellType, PathBuf) {
    let shell_path = shell_override
        .map(PathBuf::from)
        .or_else(|| std::env::var("SHELL").ok().map(PathBuf::from))
        .unwrap_or_else(|| PathBuf::from("/bin/sh"));

    let shell_type = ShellType::from_path(&shell_path);

    let home = std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/"));

    (shell_path, shell_type, home)
}

/// Default timeout in seconds for shell commands.
const DEFAULT_TIMEOUT_SECS: u64 = 120;

/// Maximum output size in bytes (10MB).
const MAX_OUTPUT_SIZE: usize = 10 * 1024 * 1024;

/// Get a string argument from JSON, returning an error if missing.
fn get_required_str<'a>(args: &'a Value, key: &str) -> Result<&'a str, Value> {
    args.get(key)
        .and_then(|v| v.as_str())
        .ok_or_else(|| json!({"error": format!("Missing required argument: {}", key)}))
}

/// Get an optional string argument from JSON.
fn get_optional_str<'a>(args: &'a Value, key: &str) -> Option<&'a str> {
    args.get(key).and_then(|v| v.as_str())
}

/// Get an optional integer argument from JSON.
fn get_optional_u64(args: &Value, key: &str) -> Option<u64> {
    args.get(key).and_then(|v| v.as_u64())
}

/// Resolve working directory relative to workspace.
fn resolve_cwd(cwd: Option<&str>, workspace: &Path) -> std::path::PathBuf {
    match cwd {
        Some(dir) => {
            let path = Path::new(dir);
            if path.is_absolute() {
                path.to_path_buf()
            } else {
                workspace.join(path)
            }
        }
        None => workspace.to_path_buf(),
    }
}

// ============================================================================
// Streaming Execution
// ============================================================================

/// Output chunk from a streaming command execution.
#[derive(Debug, Clone)]
pub struct OutputChunk {
    /// The output data (may contain ANSI codes).
    pub data: String,
    /// Which stream this came from.
    pub stream: OutputStream,
}

/// Which output stream a chunk came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputStream {
    Stdout,
    Stderr,
}

impl OutputStream {
    /// Convert to string representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            OutputStream::Stdout => "stdout",
            OutputStream::Stderr => "stderr",
        }
    }
}

/// Result of a streaming command execution.
#[derive(Debug, Clone)]
pub struct StreamingResult {
    /// Accumulated stdout.
    pub stdout: String,
    /// Accumulated stderr.
    pub stderr: String,
    /// Exit code.
    pub exit_code: i32,
    /// Whether the command timed out.
    pub timed_out: bool,
}

/// Flush interval for time-buffered output (100ms).
const FLUSH_INTERVAL_MS: u64 = 100;

/// Execute a shell command with streaming output.
///
/// This function is similar to `RunPtyCmdTool::execute` but emits output chunks
/// as they arrive via the provided channel, enabling real-time feedback for
/// long-running commands.
///
/// # Arguments
/// * `command` - The shell command to execute
/// * `cwd` - Optional working directory (relative to workspace)
/// * `timeout_secs` - Timeout in seconds
/// * `workspace` - Workspace root path
/// * `shell_override` - Optional shell path override
/// * `chunk_tx` - Channel sender for output chunks
///
/// # Returns
/// The final result with accumulated stdout/stderr and exit code.
pub async fn execute_streaming(
    command: &str,
    cwd: Option<&str>,
    timeout_secs: u64,
    workspace: &Path,
    shell_override: Option<&str>,
    chunk_tx: mpsc::Sender<OutputChunk>,
) -> Result<StreamingResult> {
    let working_dir = resolve_cwd(cwd, workspace);

    // Check if working directory exists
    if !working_dir.exists() {
        return Ok(StreamingResult {
            stdout: String::new(),
            stderr: format!("Working directory not found: {}", working_dir.display()),
            exit_code: 1,
            timed_out: false,
        });
    }

    // Determine shell and command to use
    let (shell, wrapped_command) = if cfg!(target_os = "windows") {
        ("cmd".to_string(), command.to_string())
    } else {
        let (shell_path, shell_type, home) = get_shell_config(shell_override);
        shell_type.build_command(&shell_path, command, &home)
    };

    let shell_arg = if cfg!(target_os = "windows") {
        "/c"
    } else {
        "-c"
    };

    tracing::info!(
        shell = %shell,
        original_command = %command,
        wrapped_command = %wrapped_command,
        "Executing shell command (streaming)"
    );

    // Create command
    let mut cmd = Command::new(&shell);
    cmd.arg(shell_arg)
        .arg(&wrapped_command)
        .current_dir(&working_dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .stdin(Stdio::null());

    // Set environment variables
    cmd.env("TERM", "xterm-256color");
    cmd.env("CLICOLOR", "1");
    cmd.env("CLICOLOR_FORCE", "1");

    // Spawn the process
    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => {
            return Ok(StreamingResult {
                stdout: String::new(),
                stderr: format!("Failed to spawn command: {}", e),
                exit_code: 1,
                timed_out: false,
            });
        }
    };

    let timeout_duration = tokio::time::Duration::from_secs(timeout_secs);
    let flush_interval = tokio::time::Duration::from_millis(FLUSH_INTERVAL_MS);

    // Take ownership of stdout/stderr
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();

    // Spawn tasks to read stdout and stderr with time-buffered output
    let chunk_tx_stdout = chunk_tx.clone();
    let stdout_handle = tokio::spawn(async move {
        let mut accumulated = String::new();
        tracing::debug!("stdout reader started");
        if let Some(stdout) = stdout {
            let mut reader = BufReader::new(stdout);
            let mut buffer = String::new();
            let mut last_flush = tokio::time::Instant::now();

            loop {
                let mut line = String::new();
                match tokio::time::timeout(flush_interval, reader.read_line(&mut line)).await {
                    Ok(Ok(0)) => {
                        // EOF - flush any remaining buffer
                        if !buffer.is_empty() {
                            tracing::info!("stdout EOF flush: {} bytes", buffer.len());
                            let _ = chunk_tx_stdout
                                .send(OutputChunk {
                                    data: buffer.clone(),
                                    stream: OutputStream::Stdout,
                                })
                                .await;
                        }
                        break;
                    }
                    Ok(Ok(_)) => {
                        buffer.push_str(&line);
                        accumulated.push_str(&line);

                        // Check if we should flush based on time
                        if last_flush.elapsed() >= flush_interval {
                            tracing::info!("stdout time flush: {} bytes", buffer.len());
                            let _ = chunk_tx_stdout
                                .send(OutputChunk {
                                    data: buffer.clone(),
                                    stream: OutputStream::Stdout,
                                })
                                .await;
                            buffer.clear();
                            last_flush = tokio::time::Instant::now();
                        }
                    }
                    Ok(Err(_)) => {
                        // Read error - flush buffer and break
                        if !buffer.is_empty() {
                            tracing::info!("stdout error flush: {} bytes", buffer.len());
                            let _ = chunk_tx_stdout
                                .send(OutputChunk {
                                    data: buffer.clone(),
                                    stream: OutputStream::Stdout,
                                })
                                .await;
                        }
                        break;
                    }
                    Err(_) => {
                        // Timeout - flush if we have data
                        if !buffer.is_empty() {
                            let _ = chunk_tx_stdout
                                .send(OutputChunk {
                                    data: buffer.clone(),
                                    stream: OutputStream::Stdout,
                                })
                                .await;
                            buffer.clear();
                            last_flush = tokio::time::Instant::now();
                        }
                    }
                }
            }
        }
        accumulated
    });

    let chunk_tx_stderr = chunk_tx;
    let stderr_handle = tokio::spawn(async move {
        let mut accumulated = String::new();
        if let Some(stderr) = stderr {
            let mut reader = BufReader::new(stderr);
            let mut buffer = String::new();
            let mut last_flush = tokio::time::Instant::now();

            loop {
                let mut line = String::new();
                match tokio::time::timeout(flush_interval, reader.read_line(&mut line)).await {
                    Ok(Ok(0)) => {
                        // EOF - flush any remaining buffer
                        if !buffer.is_empty() {
                            let _ = chunk_tx_stderr
                                .send(OutputChunk {
                                    data: buffer.clone(),
                                    stream: OutputStream::Stderr,
                                })
                                .await;
                        }
                        break;
                    }
                    Ok(Ok(_)) => {
                        buffer.push_str(&line);
                        accumulated.push_str(&line);

                        // Check if we should flush based on time
                        if last_flush.elapsed() >= flush_interval {
                            let _ = chunk_tx_stderr
                                .send(OutputChunk {
                                    data: buffer.clone(),
                                    stream: OutputStream::Stderr,
                                })
                                .await;
                            buffer.clear();
                            last_flush = tokio::time::Instant::now();
                        }
                    }
                    Ok(Err(_)) => {
                        // Read error - flush buffer and break
                        if !buffer.is_empty() {
                            let _ = chunk_tx_stderr
                                .send(OutputChunk {
                                    data: buffer.clone(),
                                    stream: OutputStream::Stderr,
                                })
                                .await;
                        }
                        break;
                    }
                    Err(_) => {
                        // Timeout - flush if we have data
                        if !buffer.is_empty() {
                            let _ = chunk_tx_stderr
                                .send(OutputChunk {
                                    data: buffer.clone(),
                                    stream: OutputStream::Stderr,
                                })
                                .await;
                            buffer.clear();
                            last_flush = tokio::time::Instant::now();
                        }
                    }
                }
            }
        }
        accumulated
    });

    // Wait for process with timeout
    let result = tokio::time::timeout(timeout_duration, async {
        let stdout_result = stdout_handle.await.unwrap_or_default();
        let stderr_result = stderr_handle.await.unwrap_or_default();
        let status = child.wait().await;
        (stdout_result, stderr_result, status)
    })
    .await;

    match result {
        Ok((stdout, stderr, status)) => {
            let exit_code = status.map(|s| s.code().unwrap_or(-1)).unwrap_or(-1);
            Ok(StreamingResult {
                stdout: truncate_output(stdout.as_bytes(), MAX_OUTPUT_SIZE),
                stderr: truncate_output(stderr.as_bytes(), MAX_OUTPUT_SIZE),
                exit_code,
                timed_out: false,
            })
        }
        Err(_) => {
            // Timeout - try to kill the process
            let _ = child.kill().await;
            Ok(StreamingResult {
                stdout: String::new(),
                stderr: format!("Command timed out after {} seconds", timeout_secs),
                exit_code: 124,
                timed_out: true,
            })
        }
    }
}

// ============================================================================
// run_pty_cmd
// ============================================================================

/// Tool for executing shell commands.
///
/// Shell resolution order:
/// 1. `shell_override` field (from settings.toml `terminal.shell`)
/// 2. `$SHELL` environment variable
/// 3. Fall back to `/bin/sh`
#[derive(Default)]
pub struct RunPtyCmdTool {
    /// Optional shell override from settings.
    /// When set, this takes priority over the $SHELL environment variable.
    shell_override: Option<String>,
}

impl RunPtyCmdTool {
    /// Create a new RunPtyCmdTool with default shell resolution.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a new RunPtyCmdTool with a shell override from settings.
    ///
    /// The shell override takes priority over the $SHELL environment variable.
    /// Pass `None` to use the default shell resolution order.
    pub fn with_shell(shell: Option<String>) -> Self {
        Self {
            shell_override: shell,
        }
    }
}

#[async_trait::async_trait]
impl Tool for RunPtyCmdTool {
    fn name(&self) -> &'static str {
        "run_pty_cmd"
    }

    fn description(&self) -> &'static str {
        "Execute a shell command and return the output. Commands run in a shell environment with access to common tools."
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "command": {
                    "type": "string",
                    "description": "Shell command to execute"
                },
                "cwd": {
                    "type": "string",
                    "description": "Working directory (relative to workspace)"
                },
                "timeout": {
                    "type": "integer",
                    "description": "Timeout in seconds (default: 120)"
                }
            },
            "required": ["command"]
        })
    }

    async fn execute(&self, args: Value, workspace: &Path) -> Result<Value> {
        let command_str = match get_required_str(&args, "command") {
            Ok(c) => c,
            Err(e) => return Ok(e),
        };

        let cwd = get_optional_str(&args, "cwd");
        let timeout_secs = get_optional_u64(&args, "timeout").unwrap_or(DEFAULT_TIMEOUT_SECS);

        let working_dir = resolve_cwd(cwd, workspace);

        // Check if working directory exists
        if !working_dir.exists() {
            return Ok(json!({
                "error": format!("Working directory not found: {}", working_dir.display()),
                "exit_code": 1
            }));
        }

        // Determine shell and command to use
        let (shell, wrapped_command) = if cfg!(target_os = "windows") {
            ("cmd".to_string(), command_str.to_string())
        } else {
            // Use the user's configured shell with proper PATH from rc files
            // Shell resolution order: 1) settings override, 2) $SHELL, 3) /bin/sh
            let (shell_path, shell_type, home) = get_shell_config(self.shell_override.as_deref());
            shell_type.build_command(&shell_path, command_str, &home)
        };

        let shell_arg = if cfg!(target_os = "windows") {
            "/c"
        } else {
            "-c"
        };

        debug!(
            shell = %shell,
            original_command = %command_str,
            wrapped_command = %wrapped_command,
            "Executing shell command"
        );

        // Create command
        let mut cmd = Command::new(&shell);
        cmd.arg(shell_arg)
            .arg(&wrapped_command)
            .current_dir(&working_dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .stdin(Stdio::null());

        // Set environment variables
        cmd.env("TERM", "xterm-256color");
        cmd.env("CLICOLOR", "1");
        cmd.env("CLICOLOR_FORCE", "1");

        // Spawn the process
        let mut child = match cmd.spawn() {
            Ok(c) => c,
            Err(e) => {
                return Ok(json!({
                    "error": format!("Failed to spawn command: {}", e),
                    "exit_code": 1
                }));
            }
        };

        // Read stdout and stderr with timeout
        let timeout_duration = tokio::time::Duration::from_secs(timeout_secs);

        let result = tokio::time::timeout(timeout_duration, async {
            let mut stdout_buf = Vec::new();
            let mut stderr_buf = Vec::new();

            // Take ownership of stdout/stderr
            if let Some(mut stdout) = child.stdout.take() {
                let _ = stdout.read_to_end(&mut stdout_buf).await;
            }
            if let Some(mut stderr) = child.stderr.take() {
                let _ = stderr.read_to_end(&mut stderr_buf).await;
            }

            // Wait for process to complete
            let status = child.wait().await;

            (stdout_buf, stderr_buf, status)
        })
        .await;

        match result {
            Ok((stdout_buf, stderr_buf, status)) => {
                // Truncate output if too large
                let stdout = truncate_output(&stdout_buf, MAX_OUTPUT_SIZE);
                let stderr = truncate_output(&stderr_buf, MAX_OUTPUT_SIZE);

                let exit_code = status.map(|s| s.code().unwrap_or(-1)).unwrap_or(-1);

                let mut response = json!({
                    "stdout": stdout,
                    "stderr": stderr,
                    "exit_code": exit_code,
                    "command": command_str
                });

                // Add working directory info
                if let Some(c) = cwd {
                    response["cwd"] = json!(c);
                }

                // Add error field if exit code is non-zero
                if exit_code != 0 {
                    response["error"] = json!(format!(
                        "Command exited with code {}: {}",
                        exit_code,
                        if stderr.is_empty() { &stdout } else { &stderr }
                    ));
                }

                Ok(response)
            }
            Err(_) => {
                // Timeout - try to kill the process
                let _ = child.kill().await;

                Ok(json!({
                    "error": format!("Command timed out after {} seconds", timeout_secs),
                    "exit_code": 124,  // Standard timeout exit code
                    "command": command_str,
                    "timeout": true
                }))
            }
        }
    }
}

/// Truncate output to a maximum size, preferring the end of the output.
fn truncate_output(buf: &[u8], max_size: usize) -> String {
    let content = String::from_utf8_lossy(buf);

    if content.len() <= max_size {
        return content.to_string();
    }

    // Truncate, keeping the end (most recent output is usually more relevant)
    let truncated_start = content.len() - max_size;
    format!(
        "[Output truncated, showing last {} bytes]\n{}",
        max_size,
        &content[truncated_start..]
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    // =========================================================================
    // Shell Detection Tests
    // =========================================================================

    #[test]
    fn test_shell_type_from_path_zsh() {
        assert_eq!(ShellType::from_path(Path::new("/bin/zsh")), ShellType::Zsh);
        assert_eq!(
            ShellType::from_path(Path::new("/usr/local/bin/zsh")),
            ShellType::Zsh
        );
        assert_eq!(
            ShellType::from_path(Path::new("/opt/homebrew/bin/zsh")),
            ShellType::Zsh
        );
    }

    #[test]
    fn test_shell_type_from_path_bash() {
        assert_eq!(
            ShellType::from_path(Path::new("/bin/bash")),
            ShellType::Bash
        );
        assert_eq!(
            ShellType::from_path(Path::new("/usr/bin/bash")),
            ShellType::Bash
        );
    }

    #[test]
    fn test_shell_type_from_path_fish() {
        assert_eq!(
            ShellType::from_path(Path::new("/usr/bin/fish")),
            ShellType::Fish
        );
        assert_eq!(
            ShellType::from_path(Path::new("/opt/homebrew/bin/fish")),
            ShellType::Fish
        );
    }

    #[test]
    fn test_shell_type_from_path_sh() {
        assert_eq!(ShellType::from_path(Path::new("/bin/sh")), ShellType::Sh);
        assert_eq!(ShellType::from_path(Path::new("/bin/dash")), ShellType::Sh);
        assert_eq!(ShellType::from_path(Path::new("/bin/tcsh")), ShellType::Sh);
    }

    #[test]
    fn test_shell_type_rc_file_zsh() {
        let home = PathBuf::from("/home/user");
        assert_eq!(
            ShellType::Zsh.rc_file(&home),
            Some(PathBuf::from("/home/user/.zshrc"))
        );
    }

    #[test]
    fn test_shell_type_rc_file_fish() {
        let home = PathBuf::from("/home/user");
        assert_eq!(
            ShellType::Fish.rc_file(&home),
            Some(PathBuf::from("/home/user/.config/fish/config.fish"))
        );
    }

    #[test]
    fn test_shell_type_rc_file_sh() {
        let home = PathBuf::from("/home/user");
        assert_eq!(ShellType::Sh.rc_file(&home), None);
    }

    #[test]
    fn test_build_command_zsh_with_rc() {
        let dir = tempdir().unwrap();
        let home = dir.path();
        std::fs::write(home.join(".zshrc"), "# zshrc").unwrap();

        let (shell, cmd) = ShellType::Zsh.build_command(Path::new("/bin/zsh"), "echo hello", home);

        assert_eq!(shell, "/bin/zsh");
        assert!(cmd.contains("source"));
        assert!(cmd.contains(".zshrc"));
        assert!(cmd.contains("echo hello"));
    }

    #[test]
    fn test_build_command_zsh_without_rc() {
        let dir = tempdir().unwrap();
        let home = dir.path();
        // No .zshrc file

        let (shell, cmd) = ShellType::Zsh.build_command(Path::new("/bin/zsh"), "echo hello", home);

        assert_eq!(shell, "/bin/zsh");
        assert_eq!(cmd, "echo hello");
    }

    #[test]
    fn test_build_command_bash_with_bashrc() {
        let dir = tempdir().unwrap();
        let home = dir.path();
        std::fs::write(home.join(".bashrc"), "# bashrc").unwrap();

        let (shell, cmd) =
            ShellType::Bash.build_command(Path::new("/bin/bash"), "echo hello", home);

        assert_eq!(shell, "/bin/bash");
        assert!(cmd.contains("source"));
        assert!(cmd.contains(".bashrc"));
        assert!(cmd.contains("echo hello"));
    }

    #[test]
    fn test_build_command_bash_with_bash_profile() {
        let dir = tempdir().unwrap();
        let home = dir.path();
        // No .bashrc, but .bash_profile exists
        std::fs::write(home.join(".bash_profile"), "# bash_profile").unwrap();

        let (shell, cmd) =
            ShellType::Bash.build_command(Path::new("/bin/bash"), "echo hello", home);

        assert_eq!(shell, "/bin/bash");
        assert!(cmd.contains("source"));
        assert!(cmd.contains(".bash_profile"));
        assert!(cmd.contains("echo hello"));
    }

    #[test]
    fn test_build_command_sh() {
        let dir = tempdir().unwrap();
        let home = dir.path();

        let (shell, cmd) = ShellType::Sh.build_command(Path::new("/bin/sh"), "echo hello", home);

        assert_eq!(shell, "/bin/sh");
        assert_eq!(cmd, "echo hello");
    }

    #[test]
    fn test_build_command_fish_with_config() {
        let dir = tempdir().unwrap();
        let home = dir.path();
        std::fs::create_dir_all(home.join(".config/fish")).unwrap();
        std::fs::write(home.join(".config/fish/config.fish"), "# fish config").unwrap();

        let (shell, cmd) =
            ShellType::Fish.build_command(Path::new("/usr/bin/fish"), "echo hello", home);

        assert_eq!(shell, "/usr/bin/fish");
        assert!(cmd.contains("source"));
        assert!(cmd.contains("config.fish"));
        assert!(cmd.contains("echo hello"));
    }

    // =========================================================================
    // Integration Tests
    // =========================================================================

    #[tokio::test]
    async fn test_run_pty_cmd_echo() {
        let dir = tempdir().unwrap();

        let tool = RunPtyCmdTool::new();
        let result = tool
            .execute(json!({"command": "echo hello"}), dir.path())
            .await
            .unwrap();

        assert_eq!(result["exit_code"].as_i64(), Some(0));
        assert!(result.get("error").is_none());
        assert!(result["stdout"].as_str().unwrap().contains("hello"));
    }

    #[tokio::test]
    async fn test_run_pty_cmd_exit_code() {
        let dir = tempdir().unwrap();

        let tool = RunPtyCmdTool::new();
        let result = tool
            .execute(json!({"command": "exit 42"}), dir.path())
            .await
            .unwrap();

        assert_eq!(result["exit_code"].as_i64(), Some(42));
        assert!(result.get("error").is_some());
    }

    #[tokio::test]
    async fn test_run_pty_cmd_with_cwd() {
        let dir = tempdir().unwrap();
        let workspace = dir.path();

        std::fs::create_dir(workspace.join("subdir")).unwrap();

        let tool = RunPtyCmdTool::new();
        let result = tool
            .execute(json!({"command": "pwd", "cwd": "subdir"}), workspace)
            .await
            .unwrap();

        assert_eq!(result["exit_code"].as_i64(), Some(0));
        assert!(result["stdout"].as_str().unwrap().contains("subdir"));
    }

    #[tokio::test]
    async fn test_run_pty_cmd_stderr() {
        let dir = tempdir().unwrap();

        let tool = RunPtyCmdTool::new();
        let result = tool
            .execute(json!({"command": "echo error >&2"}), dir.path())
            .await
            .unwrap();

        assert_eq!(result["exit_code"].as_i64(), Some(0));
        assert!(result["stderr"].as_str().unwrap().contains("error"));
    }

    #[tokio::test]
    async fn test_run_pty_cmd_timeout() {
        let dir = tempdir().unwrap();

        let tool = RunPtyCmdTool::new();
        let result = tool
            .execute(json!({"command": "sleep 10", "timeout": 1}), dir.path())
            .await
            .unwrap();

        assert!(result.get("error").is_some());
        assert!(result["error"].as_str().unwrap().contains("timed out"));
        assert_eq!(result["exit_code"].as_i64(), Some(124));
    }

    #[tokio::test]
    async fn test_run_pty_cmd_missing_command() {
        let dir = tempdir().unwrap();

        let tool = RunPtyCmdTool::new();
        let result = tool.execute(json!({}), dir.path()).await.unwrap();

        assert!(result.get("error").is_some());
        assert!(result["error"].as_str().unwrap().contains("Missing"));
    }

    #[tokio::test]
    async fn test_run_pty_cmd_invalid_cwd() {
        let dir = tempdir().unwrap();

        let tool = RunPtyCmdTool::new();
        let result = tool
            .execute(
                json!({"command": "echo test", "cwd": "nonexistent"}),
                dir.path(),
            )
            .await
            .unwrap();

        assert!(result.get("error").is_some());
        assert!(result["error"].as_str().unwrap().contains("not found"));
    }

    #[tokio::test]
    async fn test_run_pty_cmd_pipe() {
        let dir = tempdir().unwrap();

        let tool = RunPtyCmdTool::new();
        let result = tool
            .execute(
                json!({"command": "echo 'hello world' | grep hello"}),
                dir.path(),
            )
            .await
            .unwrap();

        assert_eq!(result["exit_code"].as_i64(), Some(0));
        assert!(result["stdout"].as_str().unwrap().contains("hello"));
    }

    #[tokio::test]
    async fn test_run_pty_cmd_multiline() {
        let dir = tempdir().unwrap();

        let tool = RunPtyCmdTool::new();
        let result = tool
            .execute(json!({"command": "echo line1 && echo line2"}), dir.path())
            .await
            .unwrap();

        assert_eq!(result["exit_code"].as_i64(), Some(0));
        let stdout = result["stdout"].as_str().unwrap();
        assert!(stdout.contains("line1"));
        assert!(stdout.contains("line2"));
    }

    #[test]
    fn test_truncate_output_short() {
        let content = b"short content";
        let result = truncate_output(content, 1000);
        assert_eq!(result, "short content");
    }

    #[test]
    fn test_truncate_output_long() {
        let content = b"a".repeat(1000);
        let result = truncate_output(&content, 100);
        assert!(result.contains("[Output truncated"));
        assert!(result.len() < 200); // Some overhead for the message
    }

    // =========================================================================
    // Shell Override Tests
    // =========================================================================

    #[test]
    fn test_get_shell_config_with_override() {
        // Test that shell_override takes priority
        let (shell_path, shell_type, _home) = get_shell_config(Some("/usr/local/bin/fish"));
        assert_eq!(shell_path.to_string_lossy(), "/usr/local/bin/fish");
        assert_eq!(shell_type, ShellType::Fish);
    }

    #[test]
    fn test_get_shell_config_with_zsh_override() {
        let (shell_path, shell_type, _home) = get_shell_config(Some("/opt/homebrew/bin/zsh"));
        assert_eq!(shell_path.to_string_lossy(), "/opt/homebrew/bin/zsh");
        assert_eq!(shell_type, ShellType::Zsh);
    }

    #[test]
    fn test_get_shell_config_with_bash_override() {
        let (shell_path, shell_type, _home) = get_shell_config(Some("/bin/bash"));
        assert_eq!(shell_path.to_string_lossy(), "/bin/bash");
        assert_eq!(shell_type, ShellType::Bash);
    }

    #[test]
    fn test_get_shell_config_none_falls_back_to_env_or_default() {
        // When shell_override is None, it should try $SHELL then fall back to /bin/sh
        let (shell_path, _shell_type, _home) = get_shell_config(None);
        // The result depends on the environment, but it should not be empty
        assert!(!shell_path.to_string_lossy().is_empty());
    }

    #[test]
    fn test_run_pty_cmd_tool_with_shell_override() {
        // Test that RunPtyCmdTool can be created with a shell override
        let tool = RunPtyCmdTool::with_shell(Some("/bin/zsh".to_string()));
        assert_eq!(tool.shell_override, Some("/bin/zsh".to_string()));
    }

    #[test]
    fn test_run_pty_cmd_tool_default() {
        // Test that default RunPtyCmdTool has no shell override
        let tool = RunPtyCmdTool::new();
        assert_eq!(tool.shell_override, None);
    }

    #[tokio::test]
    async fn test_run_pty_cmd_with_shell_override() {
        // Test that a command runs successfully with a shell override
        let dir = tempdir().unwrap();

        // Use /bin/sh as override (should be available on all Unix systems)
        let tool = RunPtyCmdTool::with_shell(Some("/bin/sh".to_string()));
        let result = tool
            .execute(json!({"command": "echo shell_override_test"}), dir.path())
            .await
            .unwrap();

        assert_eq!(result["exit_code"].as_i64(), Some(0));
        assert!(result["stdout"]
            .as_str()
            .unwrap()
            .contains("shell_override_test"));
    }
}
