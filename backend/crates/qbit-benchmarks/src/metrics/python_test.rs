//! Python test metric for evaluating code solutions.
//!
//! Runs Python test code against agent-generated solutions to verify correctness.

use std::time::Duration;

use anyhow::Result;
use async_trait::async_trait;
use qbit_evals::metrics::{EvalContext, Metric, MetricResult};
use tokio::process::Command;

/// Metric that runs Python test code to verify a solution.
///
/// This metric:
/// 1. Reads the agent's solution from `solution.py` in the workspace
/// 2. Combines it with the test code
/// 3. Runs Python and checks for success
pub struct PythonTestMetric {
    /// The test code (check function + assertions)
    test_code: String,
    /// Entry point function name
    entry_point: String,
    /// Timeout in seconds
    timeout_secs: u64,
}

impl PythonTestMetric {
    /// Create a new Python test metric.
    ///
    /// # Arguments
    /// * `test_code` - The test code containing assertion checks
    /// * `entry_point` - The name of the function being tested
    pub fn new(test_code: &str, entry_point: &str) -> Self {
        Self {
            test_code: test_code.to_string(),
            entry_point: entry_point.to_string(),
            timeout_secs: 10,
        }
    }

    /// Set the timeout in seconds (default: 10).
    pub fn with_timeout(mut self, secs: u64) -> Self {
        self.timeout_secs = secs;
        self
    }
}

#[async_trait]
impl Metric for PythonTestMetric {
    fn name(&self) -> &str {
        "python_test"
    }

    async fn evaluate(&self, ctx: &EvalContext) -> Result<MetricResult> {
        // 1. Read solution from workspace (agent wrote to solution.py)
        let solution_path = ctx.workspace.join("solution.py");
        let solution = match std::fs::read_to_string(&solution_path) {
            Ok(content) => content,
            Err(e) => {
                return Ok(MetricResult::Fail {
                    reason: format!("Failed to read solution.py: {}", e),
                });
            }
        };

        // 2. Check if solution is empty or just the placeholder
        let solution_trimmed = solution.trim();
        if solution_trimmed.is_empty()
            || solution_trimmed == "# Agent writes solution here"
            || !solution_trimmed.contains(&self.entry_point)
        {
            return Ok(MetricResult::Fail {
                reason: format!(
                    "Solution does not contain the required function '{}'",
                    self.entry_point
                ),
            });
        }

        // 3. Combine solution + test code
        // The test code contains a `check` function with assertions
        // We call check(entry_point) to run the tests
        let test_file = format!(
            "{}\n\n{}\n\ncheck({})\n",
            solution, self.test_code, self.entry_point
        );

        // 4. Write combined test file
        let test_path = ctx.workspace.join("test_solution.py");
        if let Err(e) = std::fs::write(&test_path, &test_file) {
            return Ok(MetricResult::Fail {
                reason: format!("Failed to write test file: {}", e),
            });
        }

        // 5. Run Python with timeout
        let output = tokio::time::timeout(
            Duration::from_secs(self.timeout_secs),
            Command::new("python3")
                .arg(&test_path)
                .current_dir(&ctx.workspace)
                .output(),
        )
        .await;

        match output {
            Ok(Ok(result)) if result.status.success() => Ok(MetricResult::Pass),
            Ok(Ok(result)) => {
                let stderr = String::from_utf8_lossy(&result.stderr);
                let stdout = String::from_utf8_lossy(&result.stdout);
                let combined = format!("{}\n{}", stdout, stderr);
                let reason = if combined.len() > 500 {
                    format!("{}...", &combined[..500])
                } else {
                    combined.to_string()
                };
                Ok(MetricResult::Fail { reason })
            }
            Ok(Err(e)) => Ok(MetricResult::Fail {
                reason: format!("Failed to execute Python: {}", e),
            }),
            Err(_) => Ok(MetricResult::Fail {
                reason: format!("Timeout after {} seconds", self.timeout_secs),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_python_test_metric_creation() {
        let metric = PythonTestMetric::new(
            "def check(candidate):\n    assert candidate(1, 2) == 3",
            "add",
        );
        assert_eq!(metric.name(), "python_test");
        assert_eq!(metric.timeout_secs, 10);
    }

    #[test]
    fn test_python_test_metric_with_timeout() {
        let metric = PythonTestMetric::new("", "test").with_timeout(30);
        assert_eq!(metric.timeout_secs, 30);
    }
}
