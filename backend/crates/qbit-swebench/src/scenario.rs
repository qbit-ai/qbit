//! SWE-bench scenario implementation.
//!
//! Implements the Scenario trait for SWE-bench instances.

use anyhow::{Context, Result};
use async_trait::async_trait;
use qbit_evals::metrics::{EvalContext, Metric, MetricResult};
use qbit_evals::outcome::EvalReport;
use qbit_evals::runner::EvalRunner;
use qbit_evals::scenarios::Scenario;
use tracing::{debug, info};

use crate::docker::DockerExecutor;
use crate::metric::SWEBenchTestMetric;
use crate::repo::RepoManager;
use crate::tools::{
    clear_active_container, execute_swebench_test_tool, get_swebench_test_tool_definition,
    set_active_context, SWEBenchContext,
};
use crate::types::SWEBenchInstance;

/// Strip ANSI escape codes for display.
fn strip_ansi_for_display(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '\x1b' {
            if chars.peek() == Some(&'[') {
                chars.next();
                while let Some(&next) = chars.peek() {
                    chars.next();
                    if next.is_ascii_alphabetic() {
                        break;
                    }
                }
            }
        } else {
            result.push(c);
        }
    }
    result
}

/// Scenario for a single SWE-bench instance.
pub struct SWEBenchScenario {
    /// The SWE-bench instance
    instance: SWEBenchInstance,
    /// Formatted prompt for the agent
    formatted_prompt: String,
    /// Leaked name for static lifetime
    name: &'static str,
}

impl SWEBenchScenario {
    /// Create a new SWE-bench scenario from an instance.
    pub fn new(instance: SWEBenchInstance) -> Self {
        let formatted_prompt = Self::build_prompt(&instance);
        let name = Box::leak(instance.instance_id.clone().into_boxed_str());

        Self {
            instance,
            formatted_prompt,
            name,
        }
    }

    /// Build the base prompt for the agent (without test environment info).
    /// Note: This version uses a placeholder path - prefer build_prompt_with_workspace for actual runs.
    fn build_prompt(instance: &SWEBenchInstance) -> String {
        Self::build_prompt_with_workspace(instance, None, None)
    }

    /// Build the prompt for the agent with workspace path and optional Docker container.
    ///
    /// # Arguments
    /// * `instance` - The SWE-bench instance
    /// * `repo_path` - The actual host filesystem path to the repository root (where agent will work)
    /// * `container_name` - Optional Docker container name for running tests
    fn build_prompt_with_workspace(
        instance: &SWEBenchInstance,
        repo_path: Option<&std::path::Path>,
        container_name: Option<&str>,
    ) -> String {
        let hints_section = instance.hints_text
            .as_ref()
            .filter(|h| !h.is_empty())
            .map(|hints| format!("## Hints\n\n{}\n\n", hints))
            .unwrap_or_default();

        let test_env_section = container_name
            .map(|_| Self::build_test_environment_section(instance))
            .unwrap_or_default();

        // Use actual repo path if provided, otherwise use placeholder
        let repo_path_str = repo_path
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "/workspace/repo".to_string());

        // Get the fail_to_pass tests for the prompt
        let fail_to_pass_tests = instance.fail_to_pass_tests();
        let tests_section = if !fail_to_pass_tests.is_empty() {
            let tests_list = fail_to_pass_tests.iter()
                .map(|t| format!("- `{}`", t))
                .collect::<Vec<_>>()
                .join("\n");
            format!(
r#"## Tests That Must Pass

The following test(s) should PASS after your fix (they currently fail):

{}

**WORKFLOW**:
1. Run the failing test to see the error message and traceback
2. The traceback shows you EXACTLY which file and function has the bug
3. Fix that specific code - don't guess at a different location
"#, tests_list)
        } else {
            String::new()
        };

        format!(
r#"You are working on a software engineering task from the SWE-bench benchmark.

## Repository
- Repository: {repo}
- Version: {version}

## Problem Statement

{problem_statement}

{hints_section}{tests_section}
## Workflow

1. **RUN THE FAILING TEST FIRST** - You must see the actual error before doing anything else
2. **READ THE TRACEBACK** - It shows the exact file, function, and line where the error occurs
3. **FIX THAT SPECIFIC LOCATION** - Don't guess. The traceback tells you where to look
4. **Look for similar code** - The fix pattern often exists elsewhere in the same codebase
5. **Run the test again** - Verify your fix works
6. **Iterate** - If still failing, read the NEW error and adjust

## CRITICAL CONSTRAINTS

- **MINIMAL CHANGES**: Make the smallest possible fix. Most issues require changes to 1-3 files only.
- **🚫 NEVER MODIFY TEST FILES**: Do NOT create, modify, or touch ANY test files (files in `tests/`, `test_*.py`, or `*_test.py`). The test suite is FIXED and will be applied automatically. If you modify test files, your solution will FAIL.
- **NO REFACTORING**: Do not refactor, reorganize, or "improve" unrelated code.
- **PRESERVE BEHAVIOR**: Existing functionality must continue to work.
- **UNDERSTAND BEFORE CODING**: Read the error message and traceback carefully. The error tells you exactly where the problem is (which function, which line). Fix THAT code, not something else.

The repository is available at `{repo_path_str}`. Make your changes directly to the files in this directory.

{test_env_section}"#,
            repo = instance.repo,
            version = instance.version,
            problem_statement = instance.problem_statement,
            hints_section = hints_section,
            tests_section = tests_section,
            test_env_section = test_env_section,
            repo_path_str = repo_path_str,
        )
    }

    /// Build the test environment section of the prompt.
    ///
    /// Uses the `run_swebench_test` tool instead of docker exec to prevent
    /// agents from accessing git history which could leak fix commits.
    fn build_test_environment_section(instance: &SWEBenchInstance) -> String {
        let (test_cmd, _format) = instance.test_command();

        let examples = if test_cmd.contains("runtests.py") {
            // Django-specific examples
            r#"**Examples:**
```json
// Run a specific test module
{"test_path": "admin_views.tests"}

// Run a specific test class
{"test_path": "admin_views.tests.AdminViewBasicTest"}

// Run a specific test method
{"test_path": "admin_views.tests.AdminViewBasicTest.test_login"}

// Run tests matching a pattern
{"test_path": "--parallel 1 admin_views"}
```"#
        } else {
            // pytest-style examples (default)
            r#"**Examples:**
```json
// Run a specific test file
{"test_path": "tests/test_example.py"}

// Run a specific test function
{"test_path": "tests/test_example.py::test_function"}

// Run a specific test class
{"test_path": "tests/test_example.py::TestClass"}

// Run tests matching a pattern
{"test_path": "-k test_pattern"}

// Run with less verbose output
{"test_path": "tests/test_example.py", "verbose": false}
```"#
        };

        format!(
            r#"## Test Environment

A Docker container with the full test environment is available. **You MUST run tests to verify your changes.**

**Test runner for this repository:** `{test_cmd}`

### Running Tests

Use the `run_swebench_test` tool to execute tests:

```json
{{"test_path": "path/to/test"}}
```

{examples}

**IMPORTANT**: After making changes, run related tests to check for regressions. If tests fail, analyze the error and fix your code.
"#,
            test_cmd = test_cmd,
            examples = examples,
        )
    }

    /// Get the SWE-bench instance.
    pub fn instance(&self) -> &SWEBenchInstance {
        &self.instance
    }
}

impl From<SWEBenchInstance> for SWEBenchScenario {
    fn from(instance: SWEBenchInstance) -> Self {
        Self::new(instance)
    }
}

#[async_trait]
impl Scenario for SWEBenchScenario {
    fn name(&self) -> &str {
        self.name
    }

    fn description(&self) -> &str {
        "SWE-bench software engineering task"
    }

    fn testbed(&self) -> &str {
        "swebench"
    }

    fn prompt(&self) -> &str {
        &self.formatted_prompt
    }

    fn metrics(&self) -> Vec<Box<dyn Metric>> {
        // Metrics will be populated after Docker execution
        vec![Box::new(SWEBenchTestMetric::new())]
    }

    /// Run the SWE-bench scenario with custom workflow.
    ///
    /// 1. Clone repository at base_commit into temp workspace
    /// 2. Start Docker testbed container (so agent can run tests)
    /// 3. Run agent with problem_statement (agent can run tests via docker exec)
    /// 4. Execute final tests in Docker container (with test_patch applied)
    /// 5. Evaluate metrics
    async fn run(&self, runner: &EvalRunner) -> Result<EvalReport> {
        let start = std::time::Instant::now();

        // Setup workspace with repository at base commit
        eprintln!(
            "  [1/5] Setting up workspace at commit {}...",
            &self.instance.base_commit[..8.min(self.instance.base_commit.len())]
        );

        let workspace = runner.workspace_path().join(&self.instance.instance_id);
        std::fs::create_dir_all(&workspace)?;

        // Clone repository at the correct commit
        let repo_manager = RepoManager::new()?;
        let repo_path = repo_manager
            .setup_workspace(&self.instance, &workspace)
            .context("Failed to setup repository workspace")?;

        debug!("Repository ready at {}", repo_path.display());

        // Apply test patch so agent can run the failing tests
        // This adds the FAIL_TO_PASS tests to the repository
        if !self.instance.test_patch.is_empty() {
            eprintln!("        Applying test patch ({} bytes)...", self.instance.test_patch.len());
            let test_patch_path = repo_path.join(".swebench_test_patch.diff");
            std::fs::write(&test_patch_path, &self.instance.test_patch)
                .context("Failed to write test patch")?;

            // Try to apply the patch using git
            let apply_result = std::process::Command::new("git")
                .args(["apply", "--whitespace=nowarn", ".swebench_test_patch.diff"])
                .current_dir(&repo_path)
                .output();

            match apply_result {
                Ok(output) if output.status.success() => {
                    eprintln!("        Test patch applied successfully");
                }
                Ok(output) => {
                    // Try with patch command as fallback
                    let patch_result = std::process::Command::new("patch")
                        .args(["-p1", "--forward", "--ignore-whitespace"])
                        .stdin(std::process::Stdio::piped())
                        .current_dir(&repo_path)
                        .spawn()
                        .and_then(|mut child| {
                            use std::io::Write;
                            if let Some(stdin) = child.stdin.as_mut() {
                                stdin.write_all(self.instance.test_patch.as_bytes())?;
                            }
                            child.wait()
                        });

                    match patch_result {
                        Ok(status) if status.success() => {
                            eprintln!("        Test patch applied successfully (via patch)");
                        }
                        _ => {
                            debug!("git apply stderr: {}", String::from_utf8_lossy(&output.stderr));
                            eprintln!("        ⚠ Warning: Could not apply test patch, agent won't see failing tests");
                        }
                    }
                }
                Err(e) => {
                    debug!("Failed to run git apply: {}", e);
                    eprintln!("        ⚠ Warning: Could not apply test patch: {}", e);
                }
            }

            // Clean up the patch file
            let _ = std::fs::remove_file(&test_patch_path);
        }

        // Initialize Docker executor
        let docker = DockerExecutor::new()?;

        // Check Docker availability
        if !docker.is_available().await {
            // Create a minimal agent output for the error report
            let empty_output = qbit_evals::runner::AgentOutput {
                response: String::new(),
                tool_calls: vec![],
                files_modified: vec![],
                duration_ms: 0,
                tokens_used: None,
            };
            return Ok(self.create_error_report(
                &empty_output,
                start.elapsed().as_millis() as u64,
                "Docker is not available. Please ensure Docker is running.",
            ));
        }

        // Start testbed container so agent can run tests during its work
        eprintln!("  [2/5] Starting Docker testbed container...");
        let container_name = match docker.start_testbed_container(&self.instance, &workspace).await {
            Ok(name) => {
                eprintln!("        Container: {}", name);
                Some(name)
            }
            Err(e) => {
                let err_msg = e.to_string();
                if err_msg.contains("IMAGE_NOT_AVAILABLE") {
                    eprintln!("  ⚠ Skipping: Docker image not available for this instance");
                    let empty_output = qbit_evals::runner::AgentOutput {
                        response: String::new(),
                        tool_calls: vec![],
                        files_modified: vec![],
                        duration_ms: 0,
                        tokens_used: None,
                    };
                    return Ok(self.create_skip_report(
                        &empty_output,
                        start.elapsed().as_millis() as u64,
                        "Docker image not available for this instance (Epoch AI images don't cover all instances)",
                    ));
                }
                // Log warning but continue without container (agent won't be able to run tests)
                eprintln!("  ⚠ Warning: Could not start testbed container: {}", e);
                eprintln!("        Agent will not be able to run tests during work");
                None
            }
        };

        // Build prompt with actual workspace path and container info (if available)
        // Note: repo_path is the actual repo directory (workspace/repo/)
        // We tell the agent about this path and also use it as the working directory
        let prompt = Self::build_prompt_with_workspace(
            &self.instance,
            Some(&repo_path),  // Use repo_path, not workspace
            container_name.as_deref(),
        );

        // Run the agent (with access to testbed container for running tests)
        // Use repo_path as the workspace so agent file operations work from the repo root
        eprintln!("  [3/5] Running agent...");
        eprintln!("        Working directory: {}", repo_path.display());
        if container_name.is_some() {
            eprintln!("        Agent can run tests via: run_swebench_test tool");
        }

        // Set the active context so the run_swebench_test tool can use it.
        // This includes the container name and the correct test command for this repo.
        // This prevents the agent from using docker exec directly (which would
        // allow accessing git history containing the fix commits).
        if let Some(ref name) = container_name {
            let (test_cmd, _) = self.instance.test_command();
            set_active_context(Some(SWEBenchContext {
                container_name: name.clone(),
                test_command: test_cmd.to_string(),
                repo: self.instance.repo.clone(),
            }));
        }

        // Create the custom tool definition and executor for SWE-bench test runner
        let additional_tools = if container_name.is_some() {
            vec![get_swebench_test_tool_definition()]
        } else {
            vec![]
        };

        // Create a custom executor that handles the run_swebench_test tool
        let custom_executor: Option<qbit_ai::eval_support::CustomToolExecutor> =
            if container_name.is_some() {
                Some(std::sync::Arc::new(|tool_name: &str, args: &serde_json::Value| {
                    let tool_name = tool_name.to_string();
                    let args = args.clone();
                    Box::pin(async move {
                        if tool_name == "run_swebench_test" {
                            Some(execute_swebench_test_tool(&args).await)
                        } else {
                            None // Not handled by this executor
                        }
                    })
                }))
            } else {
                None
            };

        let agent_result = runner
            .run_prompt_with_tools(&repo_path, &prompt, additional_tools, custom_executor)
            .await;

        // Clear the active container regardless of success/failure
        clear_active_container();

        // Ensure we clean up the container even if agent fails
        let agent_output = match agent_result {
            Ok(output) => output,
            Err(e) => {
                // Stop container before returning error
                if let Some(ref name) = container_name {
                    let _ = docker.stop_container(name).await;
                }
                return Err(e.context("Agent execution failed"));
            }
        };

        // Check what the agent modified
        let modified_files = repo_manager.modified_files(&repo_path).unwrap_or_default();
        eprintln!("  [4/5] Agent modified {} files", modified_files.len());

        // Stop the testbed container (we'll start a fresh one for final tests)
        if let Some(ref name) = container_name {
            eprintln!("        Stopping testbed container...");
            let _ = docker.stop_container(name).await;
        }

        // Run final tests in Docker (with test_patch applied)
        eprintln!("  [5/5] Running final tests in Docker...");
        eprintln!("        Instance: {}", self.instance.instance_id);
        eprintln!("        FAIL_TO_PASS tests: {:?}", self.instance.fail_to_pass_tests());
        eprintln!("        PASS_TO_PASS tests: {} total", self.instance.pass_to_pass_tests().len());

        // Execute tests
        // Pass the parent workspace directory, not repo_path, because Docker mounts
        // workspace at /workspace and expects the repo at /workspace/repo
        let test_result = match docker.run_tests(&self.instance, &workspace).await {
            Ok(result) => result,
            Err(e) => {
                let err_msg = e.to_string();
                // Check if this is a missing image error - skip gracefully
                if err_msg.contains("IMAGE_NOT_AVAILABLE") {
                    eprintln!("  ⚠ Skipping: Docker image not available for this instance");
                    return Ok(self.create_skip_report(
                        &agent_output,
                        start.elapsed().as_millis() as u64,
                        "Docker image not available for this instance (Epoch AI images don't cover all instances)",
                    ));
                }
                return Err(e.context("Test execution failed"));
            }
        };

        info!(
            "Test results for {}: execution_success={}, exit_code={}, FAIL_TO_PASS={}/{}, PASS_TO_PASS={}/{}",
            self.instance.instance_id,
            test_result.execution_success,
            test_result.exit_code,
            test_result.fail_to_pass_count().0,
            test_result.fail_to_pass_count().1,
            test_result.pass_to_pass_count().0,
            test_result.pass_to_pass_count().1,
        );

        // Display test output when there are failures
        if !test_result.is_solved() {
            let (f2p_passed, f2p_total) = test_result.fail_to_pass_count();
            let (p2p_passed, p2p_total) = test_result.pass_to_pass_count();

            eprintln!("\n  ┌─ Test Results ─────────────────────────────────────");
            eprintln!("  │ FAIL_TO_PASS: {}/{} passing", f2p_passed, f2p_total);
            eprintln!("  │ PASS_TO_PASS: {}/{} passing (regressions: {})", p2p_passed, p2p_total, p2p_total - p2p_passed);

            // Show detailed parsing results for FAIL_TO_PASS tests
            eprintln!("  │");
            eprintln!("  │ FAIL_TO_PASS test details:");
            for result in &test_result.fail_to_pass_results {
                let status = if result.passed { "✓ PASSED" } else { "✗ FAILED" };
                eprintln!("  │   {} {}", status, result.name);
            }

            // Show failed FAIL_TO_PASS tests with error details
            for result in &test_result.fail_to_pass_results {
                if !result.passed {
                    eprintln!("  │   ✗ {} (should have passed)", result.name);
                    if let Some(ref error) = result.error {
                        if error != "Test did not pass" {
                            eprintln!("  │     └─ {}", error);
                        }
                    }
                }
            }

            // Show regressed PASS_TO_PASS tests with error details
            for result in &test_result.pass_to_pass_results {
                if !result.passed {
                    eprintln!("  │   ✗ {} (regression)", result.name);
                    if let Some(ref error) = result.error {
                        if error != "Test regression" {
                            eprintln!("  │     └─ {}", error);
                        }
                    }
                }
            }

            eprintln!("  └─────────────────────────────────────────────────────");

            // Check for common failure patterns and highlight them
            if test_result.stdout.contains("collected 0 items") {
                eprintln!("\n  ⚠ WARNING: No tests collected! This usually means:");
                eprintln!("    - Import error in the modified code");
                eprintln!("    - Syntax error in the modified code");
                eprintln!("    - The agent broke a required module");
            }
            if test_result.stdout.contains("ImportError") || test_result.stderr.contains("ImportError") {
                eprintln!("\n  ⚠ IMPORT ERROR detected - agent likely broke imports");
            }
            if test_result.stdout.contains("SyntaxError") || test_result.stderr.contains("SyntaxError") {
                eprintln!("\n  ⚠ SYNTAX ERROR detected - agent introduced invalid Python code");
            }

            // Show pytest result lines (the lines we parse for PASSED/FAILED status)
            eprintln!("\n  ┌─ Parsed Test Status Lines ────────────────────────────");
            let mut found_result_lines = false;
            for line in test_result.stdout.lines() {
                // Strip ANSI codes FIRST, then check for status keywords
                let clean_line = strip_ansi_for_display(line.trim());
                if clean_line.contains(" PASSED") || clean_line.contains(" FAILED") || clean_line.contains(" ERROR") {
                    eprintln!("  │ {}", clean_line);
                    found_result_lines = true;
                }
            }
            if !found_result_lines {
                eprintln!("  │ (no PASSED/FAILED/ERROR lines found in output!)");
            }
            eprintln!("  └─────────────────────────────────────────────────────");

            // Show truncated stdout/stderr for debugging
            if !test_result.stdout.is_empty() {
                eprintln!("\n  ┌─ Test Output (stdout) ─────────────────────────────");
                for line in test_result.stdout.lines().take(50) {
                    eprintln!("  │ {}", line);
                }
                if test_result.stdout.lines().count() > 50 {
                    eprintln!("  │ ... ({} more lines)", test_result.stdout.lines().count() - 50);
                }
                eprintln!("  └─────────────────────────────────────────────────────");
            }

            if !test_result.stderr.is_empty() {
                eprintln!("\n  ┌─ Test Output (stderr) ─────────────────────────────");
                for line in test_result.stderr.lines().take(30) {
                    eprintln!("  │ {}", line);
                }
                if test_result.stderr.lines().count() > 30 {
                    eprintln!("  │ ... ({} more lines)", test_result.stderr.lines().count() - 30);
                }
                eprintln!("  └─────────────────────────────────────────────────────");
            }
        }

        // Create report
        let mut report = EvalReport::new(
            self.name(),
            agent_output.clone(),
            start.elapsed().as_millis() as u64,
        );

        // Store test results as extra data for detailed JSON output
        let (f2p_passed, f2p_total) = test_result.fail_to_pass_count();
        let (p2p_passed, p2p_total) = test_result.pass_to_pass_count();
        report.set_extra_data(serde_json::json!({
            "instance_id": self.instance.instance_id,
            "repo": self.instance.repo,
            "version": self.instance.version,
            "base_commit": self.instance.base_commit,
            "test_execution": {
                "success": test_result.execution_success,
                "exit_code": test_result.exit_code,
                "duration_ms": test_result.duration_ms,
                "fail_to_pass": {
                    "passed": f2p_passed,
                    "total": f2p_total,
                    "tests": test_result.fail_to_pass_results.iter().map(|r| {
                        serde_json::json!({
                            "name": r.name,
                            "passed": r.passed,
                            "error": r.error,
                        })
                    }).collect::<Vec<_>>(),
                },
                "pass_to_pass": {
                    "passed": p2p_passed,
                    "total": p2p_total,
                    "regressions": p2p_total - p2p_passed,
                    "tests": test_result.pass_to_pass_results.iter().map(|r| {
                        serde_json::json!({
                            "name": r.name,
                            "passed": r.passed,
                            "error": r.error,
                        })
                    }).collect::<Vec<_>>(),
                },
                "stdout": test_result.stdout,
                "stderr": test_result.stderr,
            },
            "modified_files": modified_files.iter().map(|p| p.display().to_string()).collect::<Vec<_>>(),
        }));

        // Evaluate metrics with test results
        let ctx = EvalContext {
            workspace,
            agent_output,
            prompt: self.prompt().to_string(),
        };

        // Use the metric with actual test results
        let metric = SWEBenchTestMetric::new().with_result(test_result);
        let result = metric.evaluate(&ctx).await?;
        report.add_metric(metric.name(), result);

        Ok(report)
    }
}

impl SWEBenchScenario {
    /// Create an error report when something goes wrong.
    fn create_error_report(
        &self,
        agent_output: &qbit_evals::runner::AgentOutput,
        duration_ms: u64,
        error_message: &str,
    ) -> EvalReport {
        let mut report = EvalReport::new(self.name(), agent_output.clone(), duration_ms);

        report.add_metric(
            "swebench-tests",
            MetricResult::Fail {
                reason: error_message.to_string(),
            },
        );

        report
    }

    /// Create a skip report when an instance can't be evaluated.
    fn create_skip_report(
        &self,
        agent_output: &qbit_evals::runner::AgentOutput,
        duration_ms: u64,
        reason: &str,
    ) -> EvalReport {
        let mut report = EvalReport::new(self.name(), agent_output.clone(), duration_ms);

        report.add_metric(
            "swebench-tests",
            MetricResult::Skip {
                reason: reason.to_string(),
            },
        );

        report
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mock_instance() -> SWEBenchInstance {
        SWEBenchInstance {
            instance_id: "django__django-11133".to_string(),
            repo: "django/django".to_string(),
            base_commit: "abc123def456".to_string(),
            problem_statement: "HttpResponse doesn't handle memoryview objects".to_string(),
            patch: "".to_string(),
            test_patch: "".to_string(),
            fail_to_pass: "[\"test_memoryview\"]".to_string(),
            pass_to_pass: "[\"test_existing\"]".to_string(),
            version: "3.0".to_string(),
            environment_setup_commit: "def456".to_string(),
            hints_text: None,
            created_at: None,
        }
    }

    #[test]
    fn test_scenario_creation() {
        let instance = mock_instance();
        let scenario = SWEBenchScenario::new(instance);

        assert_eq!(scenario.name(), "django__django-11133");
        assert_eq!(scenario.description(), "SWE-bench software engineering task");
        assert_eq!(scenario.testbed(), "swebench");
        assert!(scenario.prompt().contains("django/django"));
        assert!(scenario.prompt().contains("HttpResponse"));
    }

    #[test]
    fn test_prompt_formatting() {
        let mut instance = mock_instance();
        instance.hints_text = Some("Try looking at the make_bytes method".to_string());

        let scenario = SWEBenchScenario::new(instance);
        let prompt = scenario.prompt();

        assert!(prompt.contains("## Repository"));
        assert!(prompt.contains("## Problem Statement"));
        assert!(prompt.contains("## Hints"));
        assert!(prompt.contains("## Workflow")); // Was renamed from "## Instructions"
        assert!(prompt.contains("/workspace/repo"));
    }
}
