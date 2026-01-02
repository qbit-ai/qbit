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
pub mod openai_models;
pub mod prompt_composition;
pub mod refactor;
pub mod web_search;

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

    /// Optional custom system prompt for this scenario.
    /// Returns `None` to use the default eval system prompt.
    fn system_prompt(&self) -> Option<&str> {
        None
    }

    /// Run the scenario and return a report.
    async fn run(&self, runner: &EvalRunner) -> Result<EvalReport> {
        let start = std::time::Instant::now();

        // Setup testbed
        let workspace = runner.setup_testbed(self.testbed()).await?;

        // Run agent in the testbed workspace (with optional custom system prompt)
        let agent_output = runner
            .run_prompt_with_system(&workspace, self.prompt(), self.system_prompt())
            .await?;

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
        // Web search scenario (tests native web tools)
        Box::new(web_search::WebSearchScenario),
        // Prompt composition scenarios
        Box::new(prompt_composition::OutputFormatScenario),
        Box::new(prompt_composition::CodingConventionsScenario),
        Box::new(prompt_composition::ToolPreferenceScenario),
        Box::new(prompt_composition::BrevityInstructionScenario),
        Box::new(prompt_composition::NoInstructionsBaselineScenario),
        Box::new(prompt_composition::SubAgentAwarenessScenario),
        Box::new(prompt_composition::ProviderContextScenario),
        Box::new(prompt_composition::SpecificInstructionsScenario),
        Box::new(prompt_composition::ConflictingInstructionsScenario),
    ]
}

/// Get a scenario by name.
pub fn get_scenario(name: &str) -> Option<Box<dyn Scenario>> {
    all_scenarios().into_iter().find(|s| s.name() == name)
}

/// Get all OpenAI model scenarios.
pub fn openai_model_scenarios() -> Vec<Box<dyn Scenario>> {
    openai_models::all_openai_model_scenarios()
}

/// Get an OpenAI model scenario by model ID.
pub fn get_openai_model_scenario(model_id: &str) -> Option<Box<dyn Scenario>> {
    openai_models::OPENAI_TEST_MODELS
        .iter()
        .find(|(id, _)| *id == model_id)
        .map(|(id, name)| {
            Box::new(openai_models::OpenAiModelScenario::new(id, name)) as Box<dyn Scenario>
        })
}

/// List available OpenAI models for testing.
pub fn list_openai_models() -> Vec<(&'static str, &'static str)> {
    openai_models::OPENAI_TEST_MODELS.to_vec()
}
