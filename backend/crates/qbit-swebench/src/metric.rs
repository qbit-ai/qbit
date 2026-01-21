//! SWE-bench test metrics.
//!
//! Provides metrics for evaluating SWE-bench test results.

use anyhow::Result;
use async_trait::async_trait;
use qbit_evals::metrics::{EvalContext, Metric, MetricResult};

use crate::types::TestExecutionResult;

/// Metric for evaluating FAIL_TO_PASS tests.
///
/// These are tests that should fail before the fix and pass after.
pub struct FailToPassMetric {
    /// Test execution result (set after Docker execution)
    result: Option<TestExecutionResult>,
}

impl FailToPassMetric {
    /// Create a new FAIL_TO_PASS metric.
    pub fn new() -> Self {
        Self { result: None }
    }

    /// Set the test execution result.
    pub fn with_result(mut self, result: TestExecutionResult) -> Self {
        self.result = Some(result);
        self
    }
}

impl Default for FailToPassMetric {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Metric for FailToPassMetric {
    fn name(&self) -> &str {
        "fail-to-pass"
    }

    async fn evaluate(&self, _ctx: &EvalContext) -> Result<MetricResult> {
        let result = match &self.result {
            Some(r) => r,
            None => {
                return Ok(MetricResult::Fail {
                    reason: "No test execution result available".to_string(),
                })
            }
        };

        if !result.execution_success && result.exit_code == -1 {
            return Ok(MetricResult::Fail {
                reason: "Test execution timed out or failed to start".to_string(),
            });
        }

        let (passed, total) = result.fail_to_pass_count();

        if total == 0 {
            return Ok(MetricResult::Skip {
                reason: "No FAIL_TO_PASS tests defined".to_string(),
            });
        }

        if passed == total {
            Ok(MetricResult::Pass)
        } else if passed > 0 {
            Ok(MetricResult::Score {
                value: passed as f64,
                max: total as f64,
            })
        } else {
            Ok(MetricResult::Fail {
                reason: format!("All {} FAIL_TO_PASS tests still failing", total),
            })
        }
    }
}

/// Metric for evaluating PASS_TO_PASS tests (no regressions).
///
/// These are tests that should pass both before and after the fix.
pub struct PassToPassMetric {
    /// Test execution result (set after Docker execution)
    result: Option<TestExecutionResult>,
}

impl PassToPassMetric {
    /// Create a new PASS_TO_PASS metric.
    pub fn new() -> Self {
        Self { result: None }
    }

    /// Set the test execution result.
    pub fn with_result(mut self, result: TestExecutionResult) -> Self {
        self.result = Some(result);
        self
    }
}

impl Default for PassToPassMetric {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Metric for PassToPassMetric {
    fn name(&self) -> &str {
        "pass-to-pass"
    }

    async fn evaluate(&self, _ctx: &EvalContext) -> Result<MetricResult> {
        let result = match &self.result {
            Some(r) => r,
            None => {
                return Ok(MetricResult::Fail {
                    reason: "No test execution result available".to_string(),
                })
            }
        };

        if !result.execution_success && result.exit_code == -1 {
            return Ok(MetricResult::Fail {
                reason: "Test execution timed out or failed to start".to_string(),
            });
        }

        let (passed, total) = result.pass_to_pass_count();

        if total == 0 {
            return Ok(MetricResult::Skip {
                reason: "No PASS_TO_PASS tests defined".to_string(),
            });
        }

        let regressions = total - passed;

        if regressions == 0 {
            Ok(MetricResult::Pass)
        } else {
            Ok(MetricResult::Fail {
                reason: format!("{} tests regressed", regressions),
            })
        }
    }
}

/// Combined metric for SWE-bench evaluation.
///
/// Passes only if all FAIL_TO_PASS tests pass and no PASS_TO_PASS tests regress.
pub struct SWEBenchTestMetric {
    /// Test execution result (set after Docker execution)
    result: Option<TestExecutionResult>,
}

impl SWEBenchTestMetric {
    /// Create a new SWE-bench test metric.
    pub fn new() -> Self {
        Self { result: None }
    }

    /// Set the test execution result.
    pub fn with_result(mut self, result: TestExecutionResult) -> Self {
        self.result = Some(result);
        self
    }
}

impl Default for SWEBenchTestMetric {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Metric for SWEBenchTestMetric {
    fn name(&self) -> &str {
        "swebench-tests"
    }

    async fn evaluate(&self, _ctx: &EvalContext) -> Result<MetricResult> {
        let result = match &self.result {
            Some(r) => r,
            None => {
                return Ok(MetricResult::Fail {
                    reason: "No test execution result available".to_string(),
                })
            }
        };

        if !result.execution_success && result.exit_code == -1 {
            return Ok(MetricResult::Fail {
                reason: "Test execution timed out or failed to start".to_string(),
            });
        }

        if result.is_solved() {
            return Ok(MetricResult::Pass);
        }

        let (f2p_passed, f2p_total) = result.fail_to_pass_count();
        let (p2p_passed, p2p_total) = result.pass_to_pass_count();

        let regressions = p2p_total - p2p_passed;

        if regressions > 0 {
            Ok(MetricResult::Fail {
                reason: format!(
                    "{} regressions, {}/{} FAIL_TO_PASS tests passing",
                    regressions, f2p_passed, f2p_total
                ),
            })
        } else if f2p_passed < f2p_total {
            Ok(MetricResult::Score {
                value: f2p_passed as f64,
                max: f2p_total as f64,
            })
        } else {
            Ok(MetricResult::Fail {
                reason: "Unknown failure".to_string(),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::TestResult;
    use qbit_evals::runner::AgentOutput;
    use std::path::PathBuf;

    fn mock_eval_context() -> EvalContext {
        EvalContext {
            workspace: PathBuf::from("/tmp/test"),
            agent_output: AgentOutput {
                response: "Test response".to_string(),
                tool_calls: vec![],
                files_modified: vec![],
                duration_ms: 1000,
                tokens_used: None,
            },
            prompt: "Test prompt".to_string(),
        }
    }

    #[tokio::test]
    async fn test_fail_to_pass_all_passing() {
        let result = TestExecutionResult {
            execution_success: true,
            exit_code: 0,
            stdout: String::new(),
            stderr: String::new(),
            fail_to_pass_results: vec![
                TestResult {
                    name: "test1".to_string(),
                    passed: true,
                    error: None,
                    duration_ms: None,
                },
                TestResult {
                    name: "test2".to_string(),
                    passed: true,
                    error: None,
                    duration_ms: None,
                },
            ],
            pass_to_pass_results: vec![],
            duration_ms: 1000,
        };

        let metric = FailToPassMetric::new().with_result(result);
        let ctx = mock_eval_context();
        let metric_result = metric.evaluate(&ctx).await.unwrap();

        assert!(matches!(metric_result, MetricResult::Pass));
    }

    #[tokio::test]
    async fn test_fail_to_pass_partial() {
        let result = TestExecutionResult {
            execution_success: true,
            exit_code: 1,
            stdout: String::new(),
            stderr: String::new(),
            fail_to_pass_results: vec![
                TestResult {
                    name: "test1".to_string(),
                    passed: true,
                    error: None,
                    duration_ms: None,
                },
                TestResult {
                    name: "test2".to_string(),
                    passed: false,
                    error: Some("Failed".to_string()),
                    duration_ms: None,
                },
            ],
            pass_to_pass_results: vec![],
            duration_ms: 1000,
        };

        let metric = FailToPassMetric::new().with_result(result);
        let ctx = mock_eval_context();
        let metric_result = metric.evaluate(&ctx).await.unwrap();

        assert!(matches!(
            metric_result,
            MetricResult::Score {
                value: 1.0,
                max: 2.0
            }
        ));
    }

    #[tokio::test]
    async fn test_pass_to_pass_regression() {
        let result = TestExecutionResult {
            execution_success: true,
            exit_code: 1,
            stdout: String::new(),
            stderr: String::new(),
            fail_to_pass_results: vec![],
            pass_to_pass_results: vec![
                TestResult {
                    name: "test1".to_string(),
                    passed: true,
                    error: None,
                    duration_ms: None,
                },
                TestResult {
                    name: "test2".to_string(),
                    passed: false,
                    error: Some("Regression".to_string()),
                    duration_ms: None,
                },
            ],
            duration_ms: 1000,
        };

        let metric = PassToPassMetric::new().with_result(result);
        let ctx = mock_eval_context();
        let metric_result = metric.evaluate(&ctx).await.unwrap();

        assert!(matches!(metric_result, MetricResult::Fail { .. }));
    }

    #[tokio::test]
    async fn test_swebench_metric_solved() {
        let result = TestExecutionResult {
            execution_success: true,
            exit_code: 0,
            stdout: String::new(),
            stderr: String::new(),
            fail_to_pass_results: vec![TestResult {
                name: "test1".to_string(),
                passed: true,
                error: None,
                duration_ms: None,
            }],
            pass_to_pass_results: vec![TestResult {
                name: "test2".to_string(),
                passed: true,
                error: None,
                duration_ms: None,
            }],
            duration_ms: 1000,
        };

        let metric = SWEBenchTestMetric::new().with_result(result);
        let ctx = mock_eval_context();
        let metric_result = metric.evaluate(&ctx).await.unwrap();

        assert!(matches!(metric_result, MetricResult::Pass));
    }
}
