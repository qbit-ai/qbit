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

    /// Build the prompt for the agent.
    fn build_prompt(instance: &SWEBenchInstance) -> String {
        let mut prompt = String::new();

        prompt.push_str("You are working on a software engineering task from the SWE-bench benchmark.\n\n");
        prompt.push_str("## Repository\n");
        prompt.push_str(&format!("- Repository: {}\n", instance.repo));
        prompt.push_str(&format!("- Version: {}\n\n", instance.version));

        prompt.push_str("## Problem Statement\n\n");
        prompt.push_str(&instance.problem_statement);
        prompt.push_str("\n\n");

        if let Some(hints) = &instance.hints_text {
            if !hints.is_empty() {
                prompt.push_str("## Hints\n\n");
                prompt.push_str(hints);
                prompt.push_str("\n\n");
            }
        }

        prompt.push_str("## Instructions\n\n");
        prompt.push_str("1. Explore the repository to understand the codebase structure\n");
        prompt.push_str("2. Identify the files that need to be modified to fix this issue\n");
        prompt.push_str("3. Make the necessary code changes to fix the issue\n");
        prompt.push_str("4. Ensure your changes don't break existing functionality\n\n");

        prompt.push_str("The repository is available at `/workspace/repo`. ");
        prompt.push_str("Make your changes directly to the files in this directory.\n");

        prompt
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
    /// 2. Run agent with problem_statement
    /// 3. Execute tests in Docker container
    /// 4. Evaluate metrics
    async fn run(&self, runner: &EvalRunner) -> Result<EvalReport> {
        let start = std::time::Instant::now();

        // Setup workspace with repository at base commit
        eprintln!(
            "  [1/4] Setting up workspace at commit {}...",
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

        // Run the agent
        eprintln!("  [2/4] Running agent...");
        let agent_output = runner
            .run_prompt(&workspace, self.prompt())
            .await
            .context("Agent execution failed")?;

        // Check what the agent modified
        let modified_files = repo_manager.modified_files(&repo_path).unwrap_or_default();
        eprintln!("  [3/4] Agent modified {} files", modified_files.len());

        // Run tests in Docker
        eprintln!("  [4/4] Running tests in Docker...");
        eprintln!("        Instance: {}", self.instance.instance_id);
        eprintln!("        FAIL_TO_PASS tests: {:?}", self.instance.fail_to_pass_tests());
        eprintln!("        PASS_TO_PASS tests: {} total", self.instance.pass_to_pass_tests().len());
        let docker = DockerExecutor::new()?;

        // Check Docker availability
        if !docker.is_available().await {
            return Ok(self.create_error_report(
                &agent_output,
                start.elapsed().as_millis() as u64,
                "Docker is not available. Please ensure Docker is running.",
            ));
        }

        // Execute tests
        let test_result = docker
            .run_tests(&self.instance, &repo_path)
            .await
            .context("Test execution failed")?;

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

            // Show failed FAIL_TO_PASS tests
            for result in &test_result.fail_to_pass_results {
                if !result.passed {
                    eprintln!("  │   ✗ {} (should have passed)", result.name);
                }
            }

            // Show regressed PASS_TO_PASS tests
            for result in &test_result.pass_to_pass_results {
                if !result.passed {
                    eprintln!("  │   ✗ {} (regression)", result.name);
                }
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
        assert!(prompt.contains("## Instructions"));
        assert!(prompt.contains("/workspace/repo"));
    }
}
