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
    /// Prompts sent to the agent (for multi-turn scenarios).
    pub prompts: Vec<String>,
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
            prompts: Vec::new(),
        }
    }

    /// Create a new eval report with prompts (for multi-turn scenarios).
    pub fn new_with_prompts(
        scenario: impl Into<String>,
        agent_output: AgentOutput,
        duration_ms: u64,
        prompts: Vec<String>,
    ) -> Self {
        Self {
            scenario: scenario.into(),
            passed: true,
            metrics: Vec::new(),
            duration_ms,
            agent_output,
            prompts,
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

    /// Calculate the metric pass rate.
    pub fn metric_pass_rate(&self) -> f64 {
        if self.metrics.is_empty() {
            return 0.0;
        }
        let passed = self.metrics.iter().filter(|m| m.result.passed()).count();
        passed as f64 / self.metrics.len() as f64
    }

    /// Recalculate passed status using a threshold.
    ///
    /// This allows providers like Z.AI to pass with 80% of metrics passing
    /// instead of requiring 100%.
    pub fn apply_pass_threshold(&mut self, threshold: f64) {
        self.passed = self.metric_pass_rate() >= threshold;
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

    /// Print a CI-friendly formatted summary with clear pass/fail indicators.
    pub fn print_ci_summary<W: Write>(&self, w: &mut W, provider: &str) -> std::io::Result<()> {
        writeln!(
            w,
            "═══════════════════════════════════════════════════════════════"
        )?;
        writeln!(w, "                    EVAL RESULTS SUMMARY")?;
        writeln!(
            w,
            "═══════════════════════════════════════════════════════════════"
        )?;
        writeln!(w)?;
        writeln!(w, "Provider: {}", provider)?;
        writeln!(
            w,
            "Total: {} | Passed: {} | Failed: {} | Pass Rate: {:.0}%",
            self.reports.len(),
            self.passed_count(),
            self.failed_count(),
            self.pass_rate() * 100.0
        )?;
        writeln!(w)?;
        writeln!(
            w,
            "───────────────────────────────────────────────────────────────"
        )?;
        writeln!(w, "SCENARIOS:")?;
        writeln!(
            w,
            "───────────────────────────────────────────────────────────────"
        )?;

        for report in &self.reports {
            let icon = if report.passed { "✓" } else { "✗" };
            writeln!(w, "  {} {}", icon, report.scenario)?;
        }
        writeln!(w)?;

        // Show details for failed scenarios
        let failed: Vec<_> = self.reports.iter().filter(|r| !r.passed).collect();
        if !failed.is_empty() {
            writeln!(
                w,
                "═══════════════════════════════════════════════════════════════"
            )?;
            writeln!(w, "                    FAILED SCENARIO DETAILS")?;
            writeln!(
                w,
                "═══════════════════════════════════════════════════════════════"
            )?;

            for report in failed {
                writeln!(w)?;
                writeln!(w, "[{}]", report.scenario)?;
                writeln!(w, "  Metrics:")?;
                for metric in &report.metrics {
                    let status_str = match &metric.result {
                        crate::metrics::MetricResult::Pass => "pass".to_string(),
                        crate::metrics::MetricResult::Fail { reason } => {
                            format!("fail ({})", reason)
                        }
                        crate::metrics::MetricResult::Score { value, max } => {
                            format!("score ({:.1}/{:.1})", value, max)
                        }
                        crate::metrics::MetricResult::Skip { reason } => {
                            format!("skip ({})", reason)
                        }
                    };
                    writeln!(w, "    - {}: {}", metric.name, status_str)?;
                }
            }
        }

        Ok(())
    }
}
