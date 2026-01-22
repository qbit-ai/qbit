//! SWE-bench scenario implementation.
//!
//! Implements the Scenario trait for SWE-bench instances.

use anyhow::{Context, Result};
use async_trait::async_trait;
use qbit_evals::metrics::{Metric, MetricResult};
use qbit_evals::outcome::EvalReport;
use qbit_evals::runner::EvalRunner;
use qbit_evals::scenarios::Scenario;
use tracing::{debug, info};

use crate::docker::DockerExecutor;
use crate::harness::{is_swebench_available, run_fallback_evaluation, run_official_harness};
use crate::metric::SWEBenchTestMetric;
use crate::repo::RepoManager;
use crate::tools::{
    clear_active_container, execute_swebench_test_tool, get_swebench_test_tool_definition,
    set_active_context, SWEBenchContext,
};
use crate::types::SWEBenchInstance;

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
        _repo_path: Option<&std::path::Path>,
        container_name: Option<&str>,
    ) -> String {
        let hints_section = instance
            .hints_text
            .as_ref()
            .filter(|h| !h.is_empty())
            .map(|hints| format!("## Hints\n\n{}\n\n", hints))
            .unwrap_or_default();

        let test_section = container_name
            .map(|_| Self::build_test_section(instance))
            .unwrap_or_default();

        // Get the fail_to_pass tests for the prompt
        let fail_to_pass_tests = instance.fail_to_pass_tests();
        let tests_to_fix = if !fail_to_pass_tests.is_empty() {
            let tests_list = fail_to_pass_tests
                .iter()
                .map(|t| format!("- `{}`", t))
                .collect::<Vec<_>>()
                .join("\n");
            format!(
                r#"## Tests to Fix

The following tests currently fail and must pass after your fix:

{}

"#,
                tests_list
            )
        } else {
            String::new()
        };

        format!(
            r#"You are fixing a software engineering issue from the SWE-bench benchmark.

## Repository
- Repository: {repo}
- Version: {version}

## Problem Statement

{problem_statement}

{hints_section}{tests_to_fix}## Success Criteria

You are done when:
1. All tests listed above pass
2. No other tests regress (existing functionality preserved)
3. Only necessary files were modified

## Approach

1. **Run the failing test** - Use `run_swebench_test` to see the actual error and traceback
2. **Locate the bug** - The traceback shows the exact file, function, and line
3. **Make a minimal fix** - Change only what's necessary (typically 1-3 files)
4. **Verify** - Run the test again to confirm it passes
5. **Check for regressions** - Ensure you haven't broken other tests

## Constraints

- Do not modify test files (they are read-only)
- Do not refactor or "improve" unrelated code
- Preserve all existing functionality

{test_section}"#,
            repo = instance.repo,
            version = instance.version,
            problem_statement = instance.problem_statement,
            hints_section = hints_section,
            tests_to_fix = tests_to_fix,
            test_section = test_section,
        )
    }

    /// Build the test section of the prompt.
    ///
    /// Uses the `run_swebench_test` tool instead of docker exec to prevent
    /// agents from accessing git history which could leak fix commits.
    fn build_test_section(instance: &SWEBenchInstance) -> String {
        let is_django = instance.repo == "django/django";

        let examples = if is_django {
            r#"
For Django, use dotted module paths:
- Module: `{"test_path": "admin_views.tests"}`
- Method: `{"test_path": "admin_views.tests.AdminViewBasicTest.test_login"}`"#
        } else {
            ""
        };

        format!(
            r#"## Running Tests

Use relative paths from the repository root for all file operations.

Test with `run_swebench_test`:
```json
{{"test_path": "tests/test_example.py::test_function"}}
```

Other patterns:
- Full file: `{{"test_path": "tests/test_example.py"}}`
- Pattern match: `{{"test_path": "-k test_pattern"}}`
{examples}
Your changes are automatically synced to the test environment.
"#,
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
            eprintln!(
                "        Applying test patch ({} bytes)...",
                self.instance.test_patch.len()
            );
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
                            debug!(
                                "git apply stderr: {}",
                                String::from_utf8_lossy(&output.stderr)
                            );
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

        // Protect test files by making them read-only
        // This prevents the agent from modifying test files (which is forbidden)
        match repo_manager.protect_test_files(&repo_path) {
            Ok(count) if count > 0 => {
                eprintln!("        Protected {} test files (read-only)", count);
            }
            Err(e) => {
                debug!("Failed to protect test files: {}", e);
            }
            _ => {}
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
        let container_name = match docker
            .start_testbed_container(&self.instance, &workspace)
            .await
        {
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

        // Apply test patch to /testbed inside the container so agent can run FAIL_TO_PASS tests
        // This is necessary because:
        // 1. The test patch adds new tests that verify the fix
        // 2. We exclude test files from syncing (to prevent agent from modifying tests)
        // 3. So we need to apply the test patch directly to /testbed
        if let Some(ref name) = container_name {
            if !self.instance.test_patch.is_empty() {
                eprintln!("        Applying test patch to container /testbed...");
                match docker
                    .apply_test_patch_to_container(name, &self.instance.test_patch)
                    .await
                {
                    Ok(_) => {
                        eprintln!("        Test patch applied to /testbed");
                    }
                    Err(e) => {
                        eprintln!(
                            "        ⚠ Warning: Failed to apply test patch to /testbed: {}",
                            e
                        );
                        eprintln!("        Agent may not be able to run FAIL_TO_PASS tests");
                    }
                }
            }
        }

        // Build prompt with actual workspace path and container info (if available)
        // Note: repo_path is the actual repo directory (workspace/repo/)
        // We tell the agent about this path and also use it as the working directory
        let prompt = Self::build_prompt_with_workspace(
            &self.instance,
            Some(&repo_path), // Use repo_path, not workspace
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
        // Use verbose_test_command() so agent can see full tracebacks when debugging.
        if let Some(ref name) = container_name {
            let test_cmd = self.instance.verbose_test_command();
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
                Some(std::sync::Arc::new(
                    |tool_name: &str, args: &serde_json::Value| {
                        let tool_name = tool_name.to_string();
                        let args = args.clone();
                        Box::pin(async move {
                            if tool_name == "run_swebench_test" {
                                Some(execute_swebench_test_tool(&args).await)
                            } else {
                                None // Not handled by this executor
                            }
                        })
                    },
                ))
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

        // Run final tests to verify the agent's solution
        eprintln!("  [5/5] Running final evaluation...");
        eprintln!("        Instance: {}", self.instance.instance_id);
        eprintln!(
            "        FAIL_TO_PASS tests: {:?}",
            self.instance.fail_to_pass_tests()
        );
        eprintln!(
            "        PASS_TO_PASS tests: {} total",
            self.instance.pass_to_pass_tests().len()
        );

        // Try official SWE-bench harness first, fall back to our Docker executor
        let harness_result = if is_swebench_available() {
            eprintln!("        Using official SWE-bench harness");
            match run_official_harness(&self.instance, &workspace, "qbit-agent").await {
                Ok(result) => result,
                Err(e) => {
                    eprintln!("        ⚠ Official harness failed: {}, using fallback", e);
                    run_fallback_evaluation(&self.instance, &workspace).await?
                }
            }
        } else {
            eprintln!("        Using fallback Docker evaluation (swebench package not installed)");
            match run_fallback_evaluation(&self.instance, &workspace).await {
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
            }
        };

        // Log result
        info!(
            "Evaluation result for {}: resolved={}, completed={}",
            self.instance.instance_id, harness_result.resolved, harness_result.completed,
        );

        // Display result
        if harness_result.resolved {
            eprintln!("\n  ┌─ Evaluation Result ─────────────────────────────────");
            eprintln!("  │ ✓ RESOLVED - All tests pass");
            eprintln!("  └─────────────────────────────────────────────────────");
        } else {
            eprintln!("\n  ┌─ Evaluation Result ─────────────────────────────────");
            eprintln!("  │ ✗ NOT RESOLVED");
            if let Some(ref error) = harness_result.error {
                eprintln!("  │   Error: {}", error);
            }
            eprintln!("  └─────────────────────────────────────────────────────");

            // Show truncated output for debugging
            if !harness_result.output.is_empty() {
                eprintln!("\n  ┌─ Evaluation Output ─────────────────────────────────");
                for line in harness_result.output.lines().take(50) {
                    eprintln!("  │ {}", line);
                }
                if harness_result.output.lines().count() > 50 {
                    eprintln!(
                        "  │ ... ({} more lines)",
                        harness_result.output.lines().count() - 50
                    );
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

        // Store evaluation result as extra data
        report.set_extra_data(serde_json::json!({
            "instance_id": self.instance.instance_id,
            "repo": self.instance.repo,
            "version": self.instance.version,
            "base_commit": self.instance.base_commit,
            "evaluation": {
                "resolved": harness_result.resolved,
                "completed": harness_result.completed,
                "error": harness_result.error,
                "output_length": harness_result.output.len(),
            },
            "modified_files": modified_files.iter().map(|p| p.display().to_string()).collect::<Vec<_>>(),
        }));

        // Add metric based on harness result
        let metric_result = if harness_result.resolved {
            MetricResult::Pass
        } else if !harness_result.completed {
            MetricResult::Fail {
                reason: harness_result
                    .error
                    .unwrap_or_else(|| "Evaluation did not complete".to_string()),
            }
        } else {
            MetricResult::Fail {
                reason: harness_result
                    .error
                    .unwrap_or_else(|| "Tests did not pass".to_string()),
            }
        };
        report.add_metric("swebench-tests", metric_result);

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
        assert_eq!(
            scenario.description(),
            "SWE-bench software engineering task"
        );
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
        assert!(prompt.contains("## Approach"));
        assert!(prompt.contains("## Success Criteria"));
    }
}
