//! Code correctness metric - verifies code compiles and tests pass.

use std::process::Command;

use anyhow::Result;
use async_trait::async_trait;

use super::{EvalContext, Metric, MetricResult};

/// Metric that verifies code correctness by running a shell command.
pub struct CodeCorrectnessMetric {
    /// Name of this metric instance.
    name: String,
    /// Command to run (e.g., "cargo check", "cargo test").
    command: String,
    /// Arguments to the command.
    args: Vec<String>,
}

impl CodeCorrectnessMetric {
    /// Create a new metric that runs `cargo check`.
    pub fn cargo_check() -> Self {
        Self {
            name: "cargo_check".to_string(),
            command: "cargo".to_string(),
            args: vec!["check".to_string()],
        }
    }

    /// Create a new metric that runs `cargo test`.
    pub fn cargo_test() -> Self {
        Self {
            name: "cargo_test".to_string(),
            command: "cargo".to_string(),
            args: vec!["test".to_string()],
        }
    }

    /// Create a new metric that runs `cargo build`.
    pub fn cargo_build() -> Self {
        Self {
            name: "cargo_build".to_string(),
            command: "cargo".to_string(),
            args: vec!["build".to_string()],
        }
    }

    /// Create a custom metric with arbitrary command.
    pub fn custom(name: impl Into<String>, command: impl Into<String>, args: Vec<String>) -> Self {
        Self {
            name: name.into(),
            command: command.into(),
            args,
        }
    }
}

#[async_trait]
impl Metric for CodeCorrectnessMetric {
    fn name(&self) -> &str {
        &self.name
    }

    async fn evaluate(&self, ctx: &EvalContext) -> Result<MetricResult> {
        let output = Command::new(&self.command)
            .args(&self.args)
            .current_dir(&ctx.workspace)
            .output()?;

        if output.status.success() {
            Ok(MetricResult::Pass)
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let reason = if stderr.len() > 500 {
                format!("{}...", &stderr[..500])
            } else {
                stderr.to_string()
            };
            Ok(MetricResult::Fail { reason })
        }
    }
}
