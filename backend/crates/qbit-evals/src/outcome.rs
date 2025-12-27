//! Evaluation outcome types and reporting.

use std::io::Write;

use crate::metrics::MetricResult;
use crate::runner::AgentOutput;

/// Outcome of a single metric evaluation.
#[derive(Debug, Clone)]
pub struct MetricOutcome {
    /// Name of the metric.
    pub name: String,
    /// Result of the evaluation.
    pub result: MetricResult,
}

/// Report for a single scenario evaluation.
#[derive(Debug, Clone)]
pub struct EvalReport {
    /// Name of the scenario.
    pub scenario: String,
    /// Whether the scenario passed overall.
    pub passed: bool,
    /// Individual metric outcomes.
    pub metrics: Vec<MetricOutcome>,
    /// Duration of the evaluation in milliseconds.
    pub duration_ms: u64,
    /// Agent output from the run.
    pub agent_output: AgentOutput,
}

impl EvalReport {
    /// Create a new eval report.
    pub fn new(scenario: impl Into<String>, agent_output: AgentOutput, duration_ms: u64) -> Self {
        Self {
            scenario: scenario.into(),
            passed: true,
            metrics: Vec::new(),
            duration_ms,
            agent_output,
        }
    }

    /// Add a metric outcome and update passed status.
    pub fn add_metric(&mut self, name: impl Into<String>, result: MetricResult) {
        let passed = result.passed();
        self.metrics.push(MetricOutcome {
            name: name.into(),
            result,
        });
        if !passed {
            self.passed = false;
        }
    }

    /// Print a summary to the terminal.
    pub fn print_summary<W: Write>(&self, w: &mut W) -> std::io::Result<()> {
        let status = if self.passed { "PASS" } else { "FAIL" };
        let status_color = if self.passed { "\x1b[32m" } else { "\x1b[31m" };
        let reset = "\x1b[0m";

        writeln!(
            w,
            "\n{}{}{} {} ({}ms)",
            status_color, status, reset, self.scenario, self.duration_ms
        )?;

        for metric in &self.metrics {
            let (icon, color) = match &metric.result {
                MetricResult::Pass => ("✓", "\x1b[32m"),
                MetricResult::Fail { .. } => ("✗", "\x1b[31m"),
                MetricResult::Score { value, max } => {
                    if *value >= *max * 0.7 {
                        ("●", "\x1b[32m")
                    } else {
                        ("●", "\x1b[33m")
                    }
                }
                MetricResult::Skip { .. } => ("○", "\x1b[90m"),
            };

            write!(w, "  {}{}{} {}", color, icon, reset, metric.name)?;

            match &metric.result {
                MetricResult::Fail { reason } => {
                    let short_reason = if reason.len() > 60 {
                        format!("{}...", &reason[..60])
                    } else {
                        reason.clone()
                    };
                    writeln!(w, ": {}", short_reason)?;
                }
                MetricResult::Score { value, max } => {
                    writeln!(w, ": {:.1}/{:.1}", value, max)?;
                }
                MetricResult::Skip { reason } => {
                    writeln!(w, ": {}", reason)?;
                }
                MetricResult::Pass => {
                    writeln!(w)?;
                }
            }
        }

        Ok(())
    }

    /// Convert to JSON for CI integration.
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "scenario": self.scenario,
            "passed": self.passed,
            "duration_ms": self.duration_ms,
            "metrics": self.metrics.iter().map(|m| {
                let (status, details) = match &m.result {
                    MetricResult::Pass => ("pass", None),
                    MetricResult::Fail { reason } => ("fail", Some(reason.clone())),
                    MetricResult::Score { value, max } => {
                        return serde_json::json!({
                            "name": m.name,
                            "status": "score",
                            "value": value,
                            "max": max,
                        });
                    }
                    MetricResult::Skip { reason } => ("skip", Some(reason.clone())),
                };
                serde_json::json!({
                    "name": m.name,
                    "status": status,
                    "details": details,
                })
            }).collect::<Vec<_>>(),
        })
    }
}

/// Aggregate report for multiple scenarios.
#[derive(Debug, Default)]
pub struct EvalSummary {
    /// Individual scenario reports.
    pub reports: Vec<EvalReport>,
    /// Total duration in milliseconds.
    pub total_duration_ms: u64,
}

impl EvalSummary {
    /// Add a report to the summary.
    pub fn add(&mut self, report: EvalReport) {
        self.total_duration_ms += report.duration_ms;
        self.reports.push(report);
    }

    /// Count of passed scenarios.
    pub fn passed_count(&self) -> usize {
        self.reports.iter().filter(|r| r.passed).count()
    }

    /// Count of failed scenarios.
    pub fn failed_count(&self) -> usize {
        self.reports.iter().filter(|r| !r.passed).count()
    }

    /// Overall pass rate.
    pub fn pass_rate(&self) -> f64 {
        if self.reports.is_empty() {
            0.0
        } else {
            self.passed_count() as f64 / self.reports.len() as f64
        }
    }

    /// Print aggregate summary.
    pub fn print_summary<W: Write>(&self, w: &mut W) -> std::io::Result<()> {
        writeln!(w, "\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━")?;
        writeln!(
            w,
            "Results: {}/{} passed ({:.0}%)",
            self.passed_count(),
            self.reports.len(),
            self.pass_rate() * 100.0
        )?;
        writeln!(w, "Duration: {}ms", self.total_duration_ms)?;
        writeln!(w, "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━")?;
        Ok(())
    }

    /// Convert to JSON.
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "total": self.reports.len(),
            "passed": self.passed_count(),
            "failed": self.failed_count(),
            "pass_rate": self.pass_rate(),
            "total_duration_ms": self.total_duration_ms,
            "scenarios": self.reports.iter().map(|r| r.to_json()).collect::<Vec<_>>(),
        })
    }
}
