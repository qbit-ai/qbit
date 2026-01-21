//! SWE-bench specific tools for agent execution.
//!
//! These tools are only available during SWE-bench evaluations and provide
//! restricted access to the Docker test environment. This prevents agents from
//! accessing git history or other information that could leak answers.

use std::cell::RefCell;

use anyhow::{Context, Result};
use rig::completion::ToolDefinition;
use serde_json::json;

/// Context for the active SWE-bench container.
#[derive(Clone)]
pub struct SWEBenchContext {
    /// Container name for docker exec
    pub container_name: String,
    /// Test command prefix (e.g., "python -m pytest -xvs" or "./tests/runtests.py")
    pub test_command: String,
    /// Repository name (e.g., "django/django")
    pub repo: String,
}

thread_local! {
    /// Thread-local storage for the active SWE-bench context.
    /// Set by SWEBenchScenario::run() before agent execution.
    static ACTIVE_CONTEXT: RefCell<Option<SWEBenchContext>> = const { RefCell::new(None) };
}

/// Set the active SWE-bench context for the current thread.
///
/// Called by SWEBenchScenario before running the agent.
pub fn set_active_context(ctx: Option<SWEBenchContext>) {
    ACTIVE_CONTEXT.with(|cell| {
        *cell.borrow_mut() = ctx;
    });
}

/// Get the active SWE-bench context for the current thread.
pub fn get_active_context() -> Option<SWEBenchContext> {
    ACTIVE_CONTEXT.with(|cell| cell.borrow().clone())
}

/// Set the active container (convenience wrapper for backward compatibility).
pub fn set_active_container(name: Option<String>) {
    if let Some(name) = name {
        // When called with just a container name, use default pytest command
        set_active_context(Some(SWEBenchContext {
            container_name: name,
            test_command: "python -m pytest -xvs".to_string(),
            repo: "unknown".to_string(),
        }));
    } else {
        set_active_context(None);
    }
}

/// Get the active container name for the current thread.
pub fn get_active_container() -> Option<String> {
    get_active_context().map(|ctx| ctx.container_name)
}

/// Clear the active container/context.
///
/// Called by SWEBenchScenario after agent execution.
pub fn clear_active_container() {
    set_active_context(None);
}

/// Get the tool definition for the SWE-bench test runner.
///
/// This tool allows the agent to run tests in the Docker container
/// without giving it direct access to docker exec (which would allow
/// accessing git history containing the fix commits).
pub fn get_swebench_test_tool_definition() -> ToolDefinition {
    ToolDefinition {
        name: "run_swebench_test".to_string(),
        description: "Run tests in the SWE-bench Docker test environment. \
            Use this to verify your code changes. \
            The appropriate test runner for the repository is used automatically \
            (pytest for most repos, Django's test runner for Django, etc.). \
            You can run specific test files, test functions, or test patterns."
            .to_string(),
        parameters: sanitize_schema(json!({
            "type": "object",
            "properties": {
                "test_path": {
                    "type": "string",
                    "description": "The test to run. Can be:\n\
                        - A test file path (e.g., 'tests/test_example.py')\n\
                        - A specific test (e.g., 'tests/test_example.py::test_function')\n\
                        - A test class (e.g., 'tests/test_example.py::TestClass')\n\
                        - A pattern with -k (e.g., '-k test_memoryview')"
                },
                "verbose": {
                    "type": "boolean",
                    "description": "Whether to use verbose output (-xvs flags). Defaults to true."
                }
            },
            "required": ["test_path"]
        })),
    }
}

/// Execute the SWE-bench test tool.
///
/// Runs tests in the active Docker container using the appropriate test runner
/// for the repository. Only allows running tests, not arbitrary commands.
/// Includes automatic fallback to alternative test runners if the primary fails.
///
/// # Arguments
/// * `args` - Tool arguments containing `test_path` and optional `verbose`
///
/// # Returns
/// * `(json_result, success_flag)` - The test output and whether it succeeded
pub async fn execute_swebench_test_tool(args: &serde_json::Value) -> (serde_json::Value, bool) {
    // Get the active context
    let ctx = match get_active_context() {
        Some(ctx) => ctx,
        None => {
            return (
                json!({
                    "error": "No active SWE-bench container. This tool is only available during SWE-bench evaluations."
                }),
                false,
            );
        }
    };

    let container_name = ctx.container_name.clone();

    // Extract arguments
    let test_path = match args.get("test_path").and_then(|v| v.as_str()) {
        Some(path) => path,
        None => {
            return (
                json!({
                    "error": "Missing required argument: test_path"
                }),
                false,
            );
        }
    };

    let verbose = args
        .get("verbose")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);

    // Validate the test path to prevent command injection
    if let Err(e) = validate_test_path(test_path) {
        return (
            json!({
                "error": format!("Invalid test path: {}", e)
            }),
            false,
        );
    }

    // Build the test command using the repository-specific test runner
    let test_cmd = build_test_command(&ctx, test_path, verbose);

    // Run the command in the container
    let (stdout, stderr, exit_code) = match run_in_container(&container_name, &test_cmd).await {
        Ok(result) => result,
        Err(e) => {
            return (
                json!({
                    "error": format!("Failed to run tests: {}", e)
                }),
                false,
            );
        }
    };

    // Check if we need to try a fallback test runner
    let combined_output = format!("{}\n{}", stdout, stderr);
    let needs_fallback = is_test_runner_missing(&combined_output, exit_code);

    if needs_fallback {
        // Try alternative test runner
        let fallback_cmd = build_fallback_test_command(&ctx, test_path, verbose);

        match run_in_container(&container_name, &fallback_cmd).await {
            Ok((fb_stdout, fb_stderr, fb_exit_code)) => {
                let success = fb_exit_code == 0;
                let output = if fb_stderr.is_empty() {
                    format!(
                        "[Primary test runner unavailable, using fallback]\n\n{}",
                        fb_stdout
                    )
                } else {
                    format!(
                        "[Primary test runner unavailable, using fallback]\n\n{}\n\nSTDERR:\n{}",
                        fb_stdout, fb_stderr
                    )
                };

                return (
                    json!({
                        "output": truncate_output(&output, 50000),
                        "exit_code": fb_exit_code,
                        "success": success,
                        "used_fallback": true
                    }),
                    success,
                );
            }
            Err(e) => {
                return (
                    json!({
                        "error": format!("Both primary and fallback test runners failed: {}", e),
                        "primary_output": truncate_output(&combined_output, 10000)
                    }),
                    false,
                );
            }
        }
    }

    // Primary command succeeded or failed for reasons other than missing runner
    let success = exit_code == 0;
    let output = if stderr.is_empty() {
        stdout
    } else {
        format!("{}\n\nSTDERR:\n{}", stdout, stderr)
    };

    (
        json!({
            "output": truncate_output(&output, 50000),
            "exit_code": exit_code,
            "success": success
        }),
        success,
    )
}

/// Build the primary test command based on repository context.
fn build_test_command(ctx: &SWEBenchContext, test_path: &str, verbose: bool) -> String {
    if ctx.test_command.contains("pytest") {
        // For pytest, we can add verbose flags
        let verbose_flags = if verbose { "-xvs" } else { "-x" };
        format!(
            "cd /workspace/repo && {} {} {}",
            ctx.test_command
                .trim_end_matches("-xvs")
                .trim_end_matches("-x")
                .trim(),
            verbose_flags,
            test_path
        )
    } else if ctx.test_command.contains("runtests") {
        // Django test runner
        format!(
            "cd /workspace/repo && {} {}",
            ctx.test_command,
            convert_to_django_format(test_path)
        )
    } else {
        // Generic test runner
        format!("cd /workspace/repo && {} {}", ctx.test_command, test_path)
    }
}

/// Build a fallback test command when primary fails.
fn build_fallback_test_command(ctx: &SWEBenchContext, test_path: &str, verbose: bool) -> String {
    if ctx.test_command.contains("pytest") {
        // Primary was pytest, try Django test runner
        format!(
            "cd /workspace/repo && ./tests/runtests.py --verbosity 2 {}",
            convert_to_django_format(test_path)
        )
    } else {
        // Primary was Django or other, try pytest
        let verbose_flags = if verbose { "-xvs" } else { "-x" };
        format!(
            "cd /workspace/repo && python -m pytest {} {}",
            verbose_flags, test_path
        )
    }
}

/// Convert a test path to Django format if it's in pytest format.
fn convert_to_django_format(test_path: &str) -> String {
    if test_path.contains("::") {
        // Convert pytest format to Django format
        // pytest: tests/test_foo.py::TestClass::test_method
        // Django: test_foo.TestClass.test_method
        let without_tests = test_path.trim_start_matches("tests/");
        without_tests
            .replace('/', ".")
            .replace(".py::", ".")
            .replace("::", ".")
    } else if test_path.starts_with("-k ") {
        // -k patterns work with pytest, for Django we need module names
        // Keep as-is since we can't easily convert
        test_path.to_string()
    } else {
        test_path.to_string()
    }
}

/// Check if the test runner is missing based on output and exit code.
fn is_test_runner_missing(output: &str, exit_code: i64) -> bool {
    // Exit code 127 indicates command not found
    if exit_code == 127 {
        return true;
    }

    // Check for common "not found" error messages
    let missing_indicators = [
        "No module named pytest",
        "No module named 'pytest'",
        "pytest: not found",
        "pytest not found",
        "command not found: pytest",
        "/bin/bash: pytest: command not found",
        "ModuleNotFoundError: No module named 'pytest'",
        // Django test runner errors
        "No module named django",
        "No module named 'django'",
        "./tests/runtests.py: No such file or directory",
        "runtests.py: not found",
    ];

    for indicator in &missing_indicators {
        if output.contains(indicator) {
            return true;
        }
    }

    false
}

/// Validate a test path to prevent command injection.
///
/// Only allows:
/// - Alphanumeric characters
/// - Underscores, hyphens, dots
/// - Forward slashes (path separators)
/// - Colons (for pytest test selection)
/// - Brackets (for parameterized tests)
/// - Spaces (for -k patterns, but limited)
fn validate_test_path(path: &str) -> Result<()> {
    // Check for shell metacharacters that could be used for injection
    let forbidden_chars = [
        '`', '$', ';', '&', '|', '>', '<', '!', '\\', '\n', '\r', '\'', '"',
    ];

    for c in forbidden_chars {
        if path.contains(c) {
            anyhow::bail!("Forbidden character '{}' in test path", c);
        }
    }

    // Check for command substitution patterns
    if path.contains("$(") || path.contains("${") {
        anyhow::bail!("Command substitution not allowed");
    }

    // Check for path traversal outside testbed
    if path.contains("..") {
        anyhow::bail!("Path traversal not allowed");
    }

    // Limit length to prevent buffer overflow attacks
    if path.len() > 1000 {
        anyhow::bail!("Test path too long (max 1000 characters)");
    }

    Ok(())
}

/// Run a command in the Docker container.
///
/// Uses bollard to execute the command and capture output.
async fn run_in_container(container_name: &str, command: &str) -> Result<(String, String, i64)> {
    use bollard::exec::{CreateExecOptions, StartExecResults};
    use bollard::Docker;
    use futures::StreamExt;

    let docker = Docker::connect_with_local_defaults().context("Failed to connect to Docker")?;

    // Create exec instance
    let full_command = format!(
        "source /opt/miniconda3/etc/profile.d/conda.sh && conda activate testbed && {}",
        command
    );
    let exec_options = CreateExecOptions {
        attach_stdout: Some(true),
        attach_stderr: Some(true),
        cmd: Some(vec!["bash", "-c", &full_command]),
        ..Default::default()
    };

    let exec = docker
        .create_exec(container_name, exec_options)
        .await
        .context("Failed to create exec")?;

    // Start exec and capture output
    let mut stdout = String::new();
    let mut stderr = String::new();

    match docker.start_exec(&exec.id, None).await? {
        StartExecResults::Attached { mut output, .. } => {
            while let Some(Ok(msg)) = output.next().await {
                match msg {
                    bollard::container::LogOutput::StdOut { message } => {
                        stdout.push_str(&String::from_utf8_lossy(&message));
                    }
                    bollard::container::LogOutput::StdErr { message } => {
                        stderr.push_str(&String::from_utf8_lossy(&message));
                    }
                    _ => {}
                }
            }
        }
        StartExecResults::Detached => {
            anyhow::bail!("Exec started in detached mode unexpectedly");
        }
    }

    // Get exit code
    let inspect = docker.inspect_exec(&exec.id).await?;
    let exit_code = inspect.exit_code.unwrap_or(-1);

    Ok((stdout, stderr, exit_code))
}

/// Truncate output to a maximum length.
fn truncate_output(output: &str, max_len: usize) -> String {
    if output.len() <= max_len {
        output.to_string()
    } else {
        format!(
            "{}...\n\n[Output truncated, {} bytes total]",
            &output[..max_len],
            output.len()
        )
    }
}

/// Sanitize JSON schema for LLM compatibility (simplified version).
fn sanitize_schema(mut schema: serde_json::Value) -> serde_json::Value {
    if let Some(obj) = schema.as_object_mut() {
        obj.insert(
            "additionalProperties".to_string(),
            serde_json::Value::Bool(false),
        );
    }
    schema
}

/// Check if a tool name is the SWE-bench test tool.
pub fn is_swebench_tool(tool_name: &str) -> bool {
    tool_name == "run_swebench_test"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_test_path_valid() {
        // Valid paths
        assert!(validate_test_path("tests/test_example.py").is_ok());
        assert!(validate_test_path("tests/test_example.py::test_function").is_ok());
        assert!(validate_test_path("tests/test_example.py::TestClass::test_method").is_ok());
        assert!(validate_test_path("-k test_pattern").is_ok());
        assert!(validate_test_path("tests/test_foo.py[param1]").is_ok());
    }

    #[test]
    fn test_validate_test_path_invalid() {
        // Command injection attempts
        assert!(validate_test_path("tests/test.py; rm -rf /").is_err());
        assert!(validate_test_path("tests/test.py && cat /etc/passwd").is_err());
        assert!(validate_test_path("tests/test.py | grep secret").is_err());
        assert!(validate_test_path("$(whoami)").is_err());
        assert!(validate_test_path("tests/`id`/test.py").is_err());
        assert!(validate_test_path("tests/../../../etc/passwd").is_err());
    }

    #[test]
    fn test_container_thread_local() {
        // Set container
        set_active_container(Some("test-container".to_string()));
        assert_eq!(get_active_container(), Some("test-container".to_string()));

        // Clear container
        clear_active_container();
        assert_eq!(get_active_container(), None);
    }

    #[test]
    fn test_tool_definition() {
        let def = get_swebench_test_tool_definition();
        assert_eq!(def.name, "run_swebench_test");
        assert!(def.description.contains("pytest"));
    }

    #[test]
    fn test_convert_to_django_format() {
        // Pytest format to Django format
        assert_eq!(
            convert_to_django_format("tests/test_foo.py::TestClass::test_method"),
            "test_foo.TestClass.test_method"
        );

        // Already in Django-ish format
        assert_eq!(
            convert_to_django_format("test_foo.TestClass.test_method"),
            "test_foo.TestClass.test_method"
        );

        // -k pattern stays as-is
        assert_eq!(
            convert_to_django_format("-k test_pattern"),
            "-k test_pattern"
        );
    }

    #[test]
    fn test_is_test_runner_missing() {
        // Pytest missing
        assert!(is_test_runner_missing("No module named pytest", 1));
        assert!(is_test_runner_missing(
            "ModuleNotFoundError: No module named 'pytest'",
            1
        ));
        assert!(is_test_runner_missing(
            "/bin/bash: pytest: command not found",
            127
        ));

        // Django runner missing
        assert!(is_test_runner_missing(
            "./tests/runtests.py: No such file or directory",
            1
        ));

        // Exit code 127 always indicates missing command
        assert!(is_test_runner_missing("", 127));

        // Normal test failures shouldn't trigger fallback
        assert!(!is_test_runner_missing("FAILED test_foo.py::test_bar", 1));
        assert!(!is_test_runner_missing("AssertionError: expected True", 1));
        assert!(!is_test_runner_missing("1 passed, 2 failed", 1));
    }

    #[test]
    fn test_build_test_command_pytest() {
        let ctx = SWEBenchContext {
            container_name: "test".to_string(),
            test_command: "python -m pytest -xvs".to_string(),
            repo: "astropy/astropy".to_string(),
        };

        let cmd = build_test_command(&ctx, "tests/test_foo.py", true);
        assert!(cmd.contains("python -m pytest"));
        assert!(cmd.contains("-xvs"));
        assert!(cmd.contains("tests/test_foo.py"));
    }

    #[test]
    fn test_build_test_command_django() {
        let ctx = SWEBenchContext {
            container_name: "test".to_string(),
            test_command: "./tests/runtests.py --verbosity 2".to_string(),
            repo: "django/django".to_string(),
        };

        let cmd = build_test_command(&ctx, "admin_views.tests", true);
        assert!(cmd.contains("./tests/runtests.py"));
        assert!(cmd.contains("admin_views.tests"));
    }

    #[test]
    fn test_build_fallback_test_command() {
        // When pytest is primary, fallback should be Django
        let pytest_ctx = SWEBenchContext {
            container_name: "test".to_string(),
            test_command: "python -m pytest -xvs".to_string(),
            repo: "astropy/astropy".to_string(),
        };

        let fallback = build_fallback_test_command(&pytest_ctx, "tests/test_foo.py", true);
        assert!(fallback.contains("./tests/runtests.py"));

        // When Django is primary, fallback should be pytest
        let django_ctx = SWEBenchContext {
            container_name: "test".to_string(),
            test_command: "./tests/runtests.py --verbosity 2".to_string(),
            repo: "django/django".to_string(),
        };

        let fallback = build_fallback_test_command(&django_ctx, "admin_views.tests", true);
        assert!(fallback.contains("python -m pytest"));
    }
}
