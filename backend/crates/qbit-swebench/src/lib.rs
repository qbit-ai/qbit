//! SWE-bench Lite integration for Qbit AI agent evaluation.
//!
//! This crate provides integration with the SWE-bench Lite benchmark,
//! a collection of 300 real GitHub issues from Python repositories
//! used to evaluate AI agents on software engineering tasks.
//!
//! # Benchmark Overview
//!
//! SWE-bench Lite is a curated subset of the full SWE-bench dataset,
//! containing 300 instances that are:
//! - More reliably solvable
//! - Better documented
//! - Faster to evaluate
//!
//! Each instance consists of:
//! - A GitHub issue from a real repository
//! - A base commit to start from
//! - A gold patch (the actual fix)
//! - Test cases to verify the fix
//!
//! # Usage
//!
//! ```bash
//! # Run all SWE-bench Lite instances
//! cargo run --features evals,cli --bin qbit-cli -- --swebench
//!
//! # Run a specific instance
//! cargo run --features evals,cli --bin qbit-cli -- --swebench --instance django__django-11133
//!
//! # Run first 10 instances
//! cargo run --features evals,cli --bin qbit-cli -- --swebench --problems 0-9
//! ```
//!
//! # Prerequisites
//!
//! - Docker installed and running
//! - ~20GB disk space for repositories and images
//! - Network access for initial downloads
//!
//! # Architecture
//!
//! ```text
//! DatasetLoader     - Downloads/caches SWE-bench Lite from HuggingFace
//! RepoManager       - Clones repositories, manages worktrees
//! DockerExecutor    - Runs tests in isolated containers
//! SWEBenchScenario  - Implements the Scenario trait for evaluation
//! ```

pub mod docker;
pub mod harness;
pub mod loader;
pub mod metric;
pub mod repo;
pub mod scenario;
pub mod tools;
pub mod types;

pub use docker::DockerExecutor;
pub use harness::{
    is_swebench_available, run_fallback_evaluation, run_official_harness, HarnessResult,
};
pub use loader::{parse_instance_filter, DatasetLoader, InstanceFilter};
pub use metric::{FailToPassMetric, PassToPassMetric, SWEBenchTestMetric};
pub use repo::RepoManager;
pub use scenario::SWEBenchScenario;
pub use tools::{
    clear_active_container, execute_swebench_test_tool, get_active_container, get_active_context,
    get_swebench_test_tool_definition, is_swebench_tool, set_active_container, set_active_context,
    SWEBenchContext,
};
pub use types::{SWEBenchInstance, SWEBenchResult, TestExecutionResult, TestResult, TestRunner};

use anyhow::Result;
use qbit_evals::scenarios::Scenario;

/// Number of instances in SWE-bench Lite.
pub const SWEBENCH_LITE_COUNT: usize = 300;

/// Benchmark name for CLI identification.
pub const BENCHMARK_NAME: &str = "swebench";

/// Benchmark description.
pub const BENCHMARK_DESCRIPTION: &str =
    "300 real GitHub issues from Python repositories (SWE-bench Lite)";

/// Get all SWE-bench Lite scenarios.
///
/// Downloads the dataset if not cached locally.
pub async fn all_scenarios() -> Result<Vec<Box<dyn Scenario>>> {
    let loader = DatasetLoader::new()?;
    let instances = loader.load_lite().await?;

    Ok(instances
        .into_iter()
        .map(|i| Box::new(SWEBenchScenario::from(i)) as Box<dyn Scenario>)
        .collect())
}

/// Get scenarios for a range of instances.
///
/// # Arguments
/// * `filter` - Filter string (e.g., "0-10", "django__django-11133", "django/django")
pub async fn scenarios_for_filter(filter: &str) -> Result<Vec<Box<dyn Scenario>>> {
    let loader = DatasetLoader::new()?;
    let instances = loader.load_lite().await?;

    let filter = parse_instance_filter(filter);
    let filtered = filter.apply(instances);

    Ok(filtered
        .into_iter()
        .map(|i| Box::new(SWEBenchScenario::from(i)) as Box<dyn Scenario>)
        .collect())
}

/// Get a specific scenario by instance ID.
///
/// # Arguments
/// * `instance_id` - Instance ID (e.g., "django__django-11133")
pub async fn get_scenario(instance_id: &str) -> Result<Option<Box<dyn Scenario>>> {
    let loader = DatasetLoader::new()?;
    let instance = loader.load_instance(instance_id).await.ok();

    Ok(instance.map(|i| Box::new(SWEBenchScenario::from(i)) as Box<dyn Scenario>))
}

/// Get benchmark scenarios for the CLI.
///
/// This function is called from the CLI eval module.
///
/// # Arguments
/// * `problems` - Optional problem filter (e.g., "0-10" or "django__django-11133")
pub async fn get_benchmark_scenarios(problems: Option<&str>) -> Result<Vec<Box<dyn Scenario>>> {
    if let Some(filter) = problems {
        scenarios_for_filter(filter).await
    } else {
        all_scenarios().await
    }
}

/// List available instance repositories.
pub async fn list_repos() -> Result<Vec<String>> {
    let loader = DatasetLoader::new()?;
    loader.list_repos().await
}

/// Get dataset statistics.
pub async fn stats() -> Result<loader::DatasetStats> {
    let loader = DatasetLoader::new()?;
    loader.stats().await
}

/// Check if Docker is available for test execution.
pub async fn check_docker() -> Result<bool> {
    let executor = DockerExecutor::new()?;
    Ok(executor.is_available().await)
}

/// Get the list of benchmark info for CLI.
pub fn benchmark_info() -> (&'static str, &'static str, usize) {
    (BENCHMARK_NAME, BENCHMARK_DESCRIPTION, SWEBENCH_LITE_COUNT)
}

/// Run Docker tests only on an existing workspace (skip agent execution).
///
/// This is useful for debugging Docker test execution without re-running the agent.
///
/// # Arguments
/// * `instance_id` - Instance ID (e.g., "django__django-11133")
/// * `workspace_dir` - Path to the workspace directory containing the repo
///
/// # Returns
/// Test execution result
pub async fn run_tests_only(
    instance_id: &str,
    workspace_dir: &std::path::Path,
) -> Result<TestExecutionResult> {
    use tracing::info;

    // Load the instance
    let loader = DatasetLoader::new()?;
    let instance = loader.load_instance(instance_id).await?;

    eprintln!("Running tests only for {}", instance_id);
    eprintln!("  Workspace: {}", workspace_dir.display());
    eprintln!("  FAIL_TO_PASS tests: {:?}", instance.fail_to_pass_tests());
    eprintln!(
        "  PASS_TO_PASS tests: {} total",
        instance.pass_to_pass_tests().len()
    );

    // Run Docker tests
    let docker = DockerExecutor::new()?;

    if !docker.is_available().await {
        anyhow::bail!("Docker is not available. Please ensure Docker is running.");
    }

    let test_result = docker.run_tests(&instance, workspace_dir).await?;

    info!(
        "Test results for {}: FAIL_TO_PASS={}/{}, PASS_TO_PASS={}/{}",
        instance_id,
        test_result.fail_to_pass_count().0,
        test_result.fail_to_pass_count().1,
        test_result.pass_to_pass_count().0,
        test_result.pass_to_pass_count().1,
    );

    // Print test results
    let (f2p_passed, f2p_total) = test_result.fail_to_pass_count();
    let (p2p_passed, p2p_total) = test_result.pass_to_pass_count();

    eprintln!("\n  ┌─ Test Results ─────────────────────────────────────");
    eprintln!("  │ FAIL_TO_PASS: {}/{} passing", f2p_passed, f2p_total);
    eprintln!(
        "  │ PASS_TO_PASS: {}/{} passing (regressions: {})",
        p2p_passed,
        p2p_total,
        p2p_total - p2p_passed
    );

    // Show failed tests
    for result in &test_result.fail_to_pass_results {
        if !result.passed {
            eprintln!("  │   ✗ {} (should have passed)", result.name);
        }
    }
    for result in &test_result.pass_to_pass_results {
        if !result.passed {
            eprintln!("  │   ✗ {} (regression)", result.name);
        }
    }
    eprintln!("  └─────────────────────────────────────────────────────");

    // Show test output
    if !test_result.stdout.is_empty() {
        eprintln!("\n  ┌─ Test Output (stdout) ─────────────────────────────");
        for line in test_result.stdout.lines().take(100) {
            eprintln!("  │ {}", line);
        }
        if test_result.stdout.lines().count() > 100 {
            eprintln!(
                "  │ ... ({} more lines)",
                test_result.stdout.lines().count() - 100
            );
        }
        eprintln!("  └─────────────────────────────────────────────────────");
    }

    if !test_result.stderr.is_empty() {
        eprintln!("\n  ┌─ Test Output (stderr) ─────────────────────────────");
        for line in test_result.stderr.lines().take(50) {
            eprintln!("  │ {}", line);
        }
        if test_result.stderr.lines().count() > 50 {
            eprintln!(
                "  │ ... ({} more lines)",
                test_result.stderr.lines().count() - 50
            );
        }
        eprintln!("  └─────────────────────────────────────────────────────");
    }

    // Print summary
    let status = if test_result.is_solved() {
        "\x1b[32mPASS\x1b[0m"
    } else {
        "\x1b[31mFAIL\x1b[0m"
    };
    eprintln!("\nResult: {} ({}ms)", status, test_result.duration_ms);

    Ok(test_result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_benchmark_info() {
        let (name, desc, count) = benchmark_info();
        assert_eq!(name, "swebench");
        assert!(desc.contains("SWE-bench"));
        assert_eq!(count, 300);
    }

    #[test]
    fn test_filter_parsing() {
        // Index range
        let filter = parse_instance_filter("0-5");
        assert!(matches!(filter, InstanceFilter::ByIndex(_)));

        // Instance ID
        let filter = parse_instance_filter("django__django-11133");
        assert!(matches!(filter, InstanceFilter::ById(_)));

        // Repository
        let filter = parse_instance_filter("django/django");
        assert!(matches!(filter, InstanceFilter::ByRepo(_)));
    }
}
