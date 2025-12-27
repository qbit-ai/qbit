//! Shell command execution tool: run_pty_cmd.

use std::path::Path;
use std::process::Stdio;

use anyhow::Result;
use serde_json::{json, Value};
use tokio::io::AsyncReadExt;
use tokio::process::Command;

// Re-export the Tool trait from qbit-core for convenience
pub use qbit_core::Tool;

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
// run_pty_cmd
// ============================================================================

/// Tool for executing shell commands.
pub struct RunPtyCmdTool;

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

        // Determine shell to use
        let shell = if cfg!(target_os = "windows") {
            "cmd"
        } else {
            "/bin/sh"
        };

        let shell_arg = if cfg!(target_os = "windows") {
            "/c"
        } else {
            "-c"
        };

        // Create command
        let mut cmd = Command::new(shell);
        cmd.arg(shell_arg)
            .arg(command_str)
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

    #[tokio::test]
    async fn test_run_pty_cmd_echo() {
        let dir = tempdir().unwrap();

        let tool = RunPtyCmdTool;
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

        let tool = RunPtyCmdTool;
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

        let tool = RunPtyCmdTool;
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

        let tool = RunPtyCmdTool;
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

        let tool = RunPtyCmdTool;
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

        let tool = RunPtyCmdTool;
        let result = tool.execute(json!({}), dir.path()).await.unwrap();

        assert!(result.get("error").is_some());
        assert!(result["error"].as_str().unwrap().contains("Missing"));
    }

    #[tokio::test]
    async fn test_run_pty_cmd_invalid_cwd() {
        let dir = tempdir().unwrap();

        let tool = RunPtyCmdTool;
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

        let tool = RunPtyCmdTool;
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

        let tool = RunPtyCmdTool;
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
}
