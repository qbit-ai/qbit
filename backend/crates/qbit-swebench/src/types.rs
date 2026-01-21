//! Core types for SWE-bench integration.
//!
//! Defines the data structures for SWE-bench instances and test results.

use serde::{Deserialize, Serialize};

/// A single SWE-bench instance representing a GitHub issue.
///
/// Each instance contains all the information needed to reproduce and evaluate
/// a software engineering task from a real GitHub issue.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SWEBenchInstance {
    /// Unique identifier for this instance (e.g., "django__django-11133")
    pub instance_id: String,

    /// Repository in owner/name format (e.g., "django/django")
    pub repo: String,

    /// Base commit hash to checkout before applying the fix
    pub base_commit: String,

    /// The problem description from the GitHub issue
    pub problem_statement: String,

    /// The gold patch that fixes the issue (for reference, not shown to agent)
    pub patch: String,

    /// Additional test patch to apply for evaluation
    pub test_patch: String,

    /// JSON array of test names that should fail before fix and pass after
    #[serde(rename = "FAIL_TO_PASS")]
    pub fail_to_pass: String,

    /// JSON array of test names that should pass both before and after
    #[serde(rename = "PASS_TO_PASS")]
    pub pass_to_pass: String,

    /// Version of the software (e.g., "3.0")
    pub version: String,

    /// Commit hash for environment setup (may differ from base_commit)
    pub environment_setup_commit: String,

    /// Optional hints text (available in some variants)
    #[serde(default)]
    pub hints_text: Option<String>,

    /// Created at timestamp (optional)
    #[serde(default)]
    pub created_at: Option<String>,
}

impl SWEBenchInstance {
    /// Parse the FAIL_TO_PASS field into a list of test names.
    pub fn fail_to_pass_tests(&self) -> Vec<String> {
        serde_json::from_str(&self.fail_to_pass).unwrap_or_default()
    }

    /// Parse the PASS_TO_PASS field into a list of test names.
    pub fn pass_to_pass_tests(&self) -> Vec<String> {
        serde_json::from_str(&self.pass_to_pass).unwrap_or_default()
    }

    /// Get the repository owner (e.g., "django" from "django/django")
    pub fn repo_owner(&self) -> &str {
        self.repo.split('/').next().unwrap_or(&self.repo)
    }

    /// Get the repository name (e.g., "django" from "django/django")
    pub fn repo_name(&self) -> &str {
        self.repo.split('/').nth(1).unwrap_or(&self.repo)
    }

    /// Get the short name for display (e.g., "django-11133")
    pub fn short_name(&self) -> &str {
        // instance_id is like "django__django-11133", we want "django-11133"
        self.instance_id
            .split("__")
            .nth(1)
            .unwrap_or(&self.instance_id)
    }

    /// Get the Docker image tag for this instance (Epoch AI optimized).
    ///
    /// Uses Epoch AI's optimized images which are ~10x smaller than official.
    pub fn docker_image(&self) -> String {
        #[cfg(target_arch = "aarch64")]
        let arch = "arm64";
        #[cfg(not(target_arch = "aarch64"))]
        let arch = "x86_64";

        format!(
            "ghcr.io/epoch-research/swe-bench.eval.{}.{}",
            arch, self.instance_id
        )
    }

    /// Get alternative Docker image sources to try if primary is unavailable.
    ///
    /// Tries native architecture first, then cross-architecture (via emulation),
    /// then falls back to official DockerHub images.
    pub fn docker_image_alternatives(&self) -> Vec<String> {
        #[cfg(target_arch = "aarch64")]
        let (native_arch, emulated_arch) = ("arm64", "x86_64");
        #[cfg(not(target_arch = "aarch64"))]
        let (native_arch, emulated_arch) = ("x86_64", "arm64");

        vec![
            // Primary: Epoch AI optimized images (native architecture)
            format!(
                "ghcr.io/epoch-research/swe-bench.eval.{}.{}",
                native_arch, self.instance_id
            ),
            // Fallback 1: Epoch AI images (emulated architecture - slower but works)
            format!(
                "ghcr.io/epoch-research/swe-bench.eval.{}.{}",
                emulated_arch, self.instance_id
            ),
            // Fallback 2: Official SWE-bench images on DockerHub
            format!("swebench/sweb.eval.{}", self.instance_id),
        ]
    }

    /// Get the test runner command for this repository.
    ///
    /// Different repositories use different test frameworks:
    /// - Django: `./tests/runtests.py`
    /// - Most others: `python -m pytest`
    ///
    /// Returns (command_prefix, test_arg_format) where test_arg_format indicates
    /// how to pass test names to the runner.
    pub fn test_command(&self) -> (&'static str, TestArgFormat) {
        match self.repo.as_str() {
            "django/django" => (
                "./tests/runtests.py --verbosity 2",
                TestArgFormat::DjangoStyle,
            ),
            "matplotlib/matplotlib" => (
                "python -m pytest -xvs",
                TestArgFormat::PytestStyle,
            ),
            "sympy/sympy" => (
                "python -m pytest -xvs",
                TestArgFormat::PytestStyle,
            ),
            // Default to pytest for most repositories
            _ => (
                "python -m pytest -xvs",
                TestArgFormat::PytestStyle,
            ),
        }
    }

    /// Get the full test command for running a specific test.
    pub fn build_test_command(&self, test_path: &str) -> String {
        let (cmd, format) = self.test_command();
        match format {
            TestArgFormat::PytestStyle => format!("{} {}", cmd, test_path),
            TestArgFormat::DjangoStyle => {
                // Django tests are specified as dotted module paths
                // e.g., "admin_views.tests.AdminViewBasicTest.test_login"
                // But the agent might provide file paths, so we handle both
                let test_arg = if test_path.ends_with(".py") || test_path.contains('/') {
                    // Convert file path to Django test format
                    // tests/admin_views/tests.py::TestClass::test_method
                    // -> admin_views.tests.TestClass.test_method
                    test_path
                        .trim_start_matches("tests/")
                        .replace('/', ".")
                        .replace(".py::", ".")
                        .replace(".py", "")
                        .replace("::", ".")
                } else {
                    test_path.to_string()
                };
                format!("{} {}", cmd, test_arg)
            }
        }
    }
}

/// How test arguments are formatted for different test runners.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TestArgFormat {
    /// pytest style: `path/to/test.py::TestClass::test_method`
    PytestStyle,
    /// Django style: `module.tests.TestClass.test_method`
    DjangoStyle,
}

/// Result of executing tests in a Docker container.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestExecutionResult {
    /// Whether the test execution completed successfully (not whether tests passed)
    pub execution_success: bool,

    /// Exit code from the test runner
    pub exit_code: i32,

    /// Standard output from the test runner
    pub stdout: String,

    /// Standard error from the test runner
    pub stderr: String,

    /// Tests that were expected to fail and now pass
    pub fail_to_pass_results: Vec<TestResult>,

    /// Tests that should continue to pass
    pub pass_to_pass_results: Vec<TestResult>,

    /// Execution time in milliseconds
    pub duration_ms: u64,
}

impl TestExecutionResult {
    /// Check if all FAIL_TO_PASS tests now pass.
    pub fn fail_to_pass_success(&self) -> bool {
        !self.fail_to_pass_results.is_empty()
            && self.fail_to_pass_results.iter().all(|r| r.passed)
    }

    /// Check if all PASS_TO_PASS tests still pass (no regressions).
    pub fn pass_to_pass_success(&self) -> bool {
        self.pass_to_pass_results.iter().all(|r| r.passed)
    }

    /// Check if the instance is fully solved (all tests pass).
    pub fn is_solved(&self) -> bool {
        self.fail_to_pass_success() && self.pass_to_pass_success()
    }

    /// Get the number of FAIL_TO_PASS tests that now pass.
    pub fn fail_to_pass_count(&self) -> (usize, usize) {
        let passed = self.fail_to_pass_results.iter().filter(|r| r.passed).count();
        (passed, self.fail_to_pass_results.len())
    }

    /// Get the number of PASS_TO_PASS tests that still pass.
    pub fn pass_to_pass_count(&self) -> (usize, usize) {
        let passed = self.pass_to_pass_results.iter().filter(|r| r.passed).count();
        (passed, self.pass_to_pass_results.len())
    }
}

/// Result of a single test.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestResult {
    /// Name of the test
    pub name: String,

    /// Whether the test passed
    pub passed: bool,

    /// Error message if the test failed
    pub error: Option<String>,

    /// Execution time in milliseconds (if available)
    pub duration_ms: Option<u64>,
}

/// Evaluation result for a SWE-bench instance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SWEBenchResult {
    /// All tests pass - instance fully solved
    Solved,

    /// Some FAIL_TO_PASS tests pass, no regressions
    Partial {
        /// Number of FAIL_TO_PASS tests that pass
        fail_to_pass_passed: usize,
        /// Total number of FAIL_TO_PASS tests
        fail_to_pass_total: usize,
    },

    /// Tests failed or caused regressions
    Failed {
        /// Reason for failure
        reason: String,
        /// Number of FAIL_TO_PASS tests that pass
        fail_to_pass_passed: usize,
        /// Total number of FAIL_TO_PASS tests
        fail_to_pass_total: usize,
        /// Number of PASS_TO_PASS tests that regressed
        regressions: usize,
    },

    /// Error during evaluation (Docker, timeout, etc.)
    Error {
        /// Error message
        message: String,
    },
}

impl SWEBenchResult {
    /// Check if the instance was fully solved.
    pub fn is_solved(&self) -> bool {
        matches!(self, SWEBenchResult::Solved)
    }

    /// Get the pass rate for FAIL_TO_PASS tests.
    pub fn fail_to_pass_rate(&self) -> f64 {
        match self {
            SWEBenchResult::Solved => 1.0,
            SWEBenchResult::Partial {
                fail_to_pass_passed,
                fail_to_pass_total,
            } => {
                if *fail_to_pass_total == 0 {
                    0.0
                } else {
                    *fail_to_pass_passed as f64 / *fail_to_pass_total as f64
                }
            }
            SWEBenchResult::Failed {
                fail_to_pass_passed,
                fail_to_pass_total,
                ..
            } => {
                if *fail_to_pass_total == 0 {
                    0.0
                } else {
                    *fail_to_pass_passed as f64 / *fail_to_pass_total as f64
                }
            }
            SWEBenchResult::Error { .. } => 0.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_instance_parsing() {
        let json = r#"{
            "instance_id": "django__django-11133",
            "repo": "django/django",
            "base_commit": "abc123",
            "problem_statement": "Test problem",
            "patch": "diff --git ...",
            "test_patch": "diff --git ...",
            "FAIL_TO_PASS": "[\"test_foo\", \"test_bar\"]",
            "PASS_TO_PASS": "[\"test_existing\"]",
            "version": "3.0",
            "environment_setup_commit": "def456"
        }"#;

        let instance: SWEBenchInstance = serde_json::from_str(json).unwrap();
        assert_eq!(instance.instance_id, "django__django-11133");
        assert_eq!(instance.repo_owner(), "django");
        assert_eq!(instance.repo_name(), "django");
        assert_eq!(instance.short_name(), "django-11133");
        assert_eq!(instance.fail_to_pass_tests(), vec!["test_foo", "test_bar"]);
        assert_eq!(instance.pass_to_pass_tests(), vec!["test_existing"]);
    }

    #[test]
    fn test_docker_image() {
        let instance = SWEBenchInstance {
            instance_id: "django__django-11133".to_string(),
            repo: "django/django".to_string(),
            base_commit: "abc123".to_string(),
            problem_statement: "Test".to_string(),
            patch: "".to_string(),
            test_patch: "".to_string(),
            fail_to_pass: "[]".to_string(),
            pass_to_pass: "[]".to_string(),
            version: "3.0".to_string(),
            environment_setup_commit: "def456".to_string(),
            hints_text: None,
            created_at: None,
        };

        let image = instance.docker_image();
        assert!(image.starts_with("ghcr.io/epoch-research/swe-bench.eval."));
        assert!(image.ends_with(".django__django-11133"));
    }

    #[test]
    fn test_execution_result() {
        let result = TestExecutionResult {
            execution_success: true,
            exit_code: 0,
            stdout: "".to_string(),
            stderr: "".to_string(),
            fail_to_pass_results: vec![
                TestResult {
                    name: "test1".to_string(),
                    passed: true,
                    error: None,
                    duration_ms: Some(100),
                },
                TestResult {
                    name: "test2".to_string(),
                    passed: true,
                    error: None,
                    duration_ms: Some(50),
                },
            ],
            pass_to_pass_results: vec![TestResult {
                name: "test3".to_string(),
                passed: true,
                error: None,
                duration_ms: Some(75),
            }],
            duration_ms: 225,
        };

        assert!(result.fail_to_pass_success());
        assert!(result.pass_to_pass_success());
        assert!(result.is_solved());
        assert_eq!(result.fail_to_pass_count(), (2, 2));
        assert_eq!(result.pass_to_pass_count(), (1, 1));
    }
}
