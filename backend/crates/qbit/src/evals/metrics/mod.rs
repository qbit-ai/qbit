//! Evaluation metrics for assessing agent performance.
//!
//! Provides various metric types:
//! - `CodeCorrectnessMetric`: Verifies code compiles/tests pass
//! - `FileStateMetric`: Checks file existence and content
//! - `LlmJudgeMetric`: Uses LLM to judge output quality
//! - `LlmScoreMetric`: Uses LLM to score output on a scale
//! - `TokenTrackingMetric`: Tracks and validates token usage

mod code_correctness;
mod file_state;
mod llm_judge;
pub mod token_tracking;

pub use code_correctness::CodeCorrectnessMetric;
pub use file_state::FileStateMetric;
pub use llm_judge::{LlmJudgeMetric, LlmScoreMetric};
pub use token_tracking::TokenTrackingMetric;

use std::path::PathBuf;

use anyhow::Result;
use async_trait::async_trait;

use crate::evals::runner::AgentOutput;

/// Result of evaluating a metric.
#[derive(Debug, Clone)]
pub enum MetricResult {
    /// Metric passed.
    Pass,
    /// Metric failed with a reason.
    Fail { reason: String },
    /// Metric returned a score.
    Score { value: f64, max: f64 },
    /// Metric was skipped.
    Skip { reason: String },
}

impl MetricResult {
    /// Check if the metric passed.
    ///
    /// Skip results are treated as passed (neutral) to not affect the overall score.
    pub fn passed(&self) -> bool {
        match self {
            MetricResult::Pass => true,
            MetricResult::Score { value, max } => *value >= *max * 0.7,
            MetricResult::Skip { .. } => true, // Skipped metrics are neutral
            MetricResult::Fail { .. } => false,
        }
    }
}

/// Context provided to metrics during evaluation.
pub struct EvalContext {
    /// Path to the workspace.
    pub workspace: PathBuf,
    /// Output from the agent run.
    pub agent_output: AgentOutput,
    /// Original prompt given to the agent.
    pub prompt: String,
}

/// Trait for evaluation metrics.
#[async_trait]
pub trait Metric: Send + Sync {
    /// Name of the metric.
    fn name(&self) -> &str;

    /// Evaluate the metric.
    async fn evaluate(&self, ctx: &EvalContext) -> Result<MetricResult>;
}
