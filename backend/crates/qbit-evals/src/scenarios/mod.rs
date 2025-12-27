//! Evaluation scenarios.
//!
//! Each scenario defines:
//! - A testbed (embedded files)
//! - A prompt for the agent
//! - Metrics to evaluate the result

pub mod bug_fix;
pub mod code_understanding;
pub mod feature_impl;
pub mod multi_step;
pub mod refactor;

use anyhow::Result;
use async_trait::async_trait;

use crate::metrics::Metric;
use crate::outcome::EvalReport;
use crate::runner::EvalRunner;

/// Trait for evaluation scenarios.
#[async_trait]
pub trait Scenario: Send + Sync {
    /// Name of the scenario.
    fn name(&self) -> &str;

    /// Description of what this scenario tests.
    fn description(&self) -> &str;

    /// Name of the testbed to use.
    fn testbed(&self) -> &str;

    /// Prompt to give to the agent.
    fn prompt(&self) -> &str;

    /// Metrics to evaluate the result.
    fn metrics(&self) -> Vec<Box<dyn Metric>>;

    /// Run the scenario and return a report.
    async fn run(&self, runner: &EvalRunner) -> Result<EvalReport> {
        let start = std::time::Instant::now();

        // Setup testbed
        let workspace = runner.setup_testbed(self.testbed()).await?;

        // Run agent
        let agent_output = runner.run_prompt(self.prompt()).await?;

        // Create report
        let mut report = EvalReport::new(
            self.name(),
            agent_output.clone(),
            start.elapsed().as_millis() as u64,
        );

        // Evaluate metrics
        let ctx = crate::metrics::EvalContext {
            workspace,
            agent_output,
            prompt: self.prompt().to_string(),
        };

        for metric in self.metrics() {
            let result = metric.evaluate(&ctx).await?;
            report.add_metric(metric.name(), result);
        }

        Ok(report)
    }
}

/// Get all available scenarios.
pub fn all_scenarios() -> Vec<Box<dyn Scenario>> {
    vec![
        Box::new(bug_fix::BugFixScenario),
        Box::new(feature_impl::FeatureImplScenario),
        Box::new(refactor::RefactorScenario),
        Box::new(code_understanding::CodeUnderstandingScenario),
        Box::new(multi_step::MultiStepScenario),
    ]
}

/// Get a scenario by name.
pub fn get_scenario(name: &str) -> Option<Box<dyn Scenario>> {
    all_scenarios().into_iter().find(|s| s.name() == name)
}
