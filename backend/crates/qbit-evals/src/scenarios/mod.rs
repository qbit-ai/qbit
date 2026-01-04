//! Evaluation scenarios.
//!
//! Each scenario defines:
//! - A testbed (embedded files)
//! - A prompt for the agent
//! - Metrics to evaluate the result

pub mod ast_grep;
pub mod bug_fix;
pub mod code_understanding;
pub mod feature_impl;
pub mod multi_step;
pub mod multi_turn;
pub mod openai_models;
pub mod openai_web_search;
pub mod prompt_composition;
pub mod refactor;
pub mod web_search;

use anyhow::Result;
use async_trait::async_trait;

use crate::config::EvalProvider;
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

    /// Check if this scenario supports the given provider.
    ///
    /// Returns `true` by default. Scenarios that require specific provider
    /// capabilities (e.g., web search) should override this to return `false`
    /// for unsupported providers.
    fn supports_provider(&self, _provider: EvalProvider) -> bool {
        true
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

/// Get default scenarios (excludes optional/provider-specific scenarios).
///
/// Use this for normal eval runs. Optional scenarios like `openai-web-search`
/// are excluded but can still be run explicitly with `--scenario`.
pub fn default_scenarios() -> Vec<Box<dyn Scenario>> {
    vec![
        Box::new(bug_fix::BugFixScenario),
        Box::new(feature_impl::FeatureImplScenario),
        Box::new(refactor::RefactorScenario),
        Box::new(code_understanding::CodeUnderstandingScenario),
        Box::new(multi_step::MultiStepScenario),
        // Web search scenario (tests native web tools)
        Box::new(web_search::WebSearchScenario),
        // AST-grep scenarios (tests structural code search and replace)
        Box::new(ast_grep::AstGrepSearchScenario),
        Box::new(ast_grep::AstGrepReplaceScenario),
        // Prompt composition scenarios
        Box::new(prompt_composition::OutputFormatScenario),
        Box::new(prompt_composition::CodingConventionsScenario),
        Box::new(prompt_composition::ToolPreferenceScenario),
        Box::new(prompt_composition::BrevityInstructionScenario),
        Box::new(prompt_composition::NoInstructionsBaselineScenario),
        Box::new(prompt_composition::SubAgentAwarenessScenario),
        Box::new(prompt_composition::ProviderContextScenario),
        Box::new(prompt_composition::SpecificInstructionsScenario),
    ]
}

/// Get default scenarios filtered for a specific provider.
///
/// Excludes scenarios that don't support the given provider (e.g., web-search
/// is excluded for Z.AI since it only works with Claude and OpenAI).
pub fn default_scenarios_for_provider(provider: EvalProvider) -> Vec<Box<dyn Scenario>> {
    default_scenarios()
        .into_iter()
        .filter(|s| s.supports_provider(provider))
        .collect()
}

/// Get all available scenarios including optional ones.
///
/// Includes provider-specific scenarios like `openai-web-search` that require
/// specific provider configuration.
pub fn all_scenarios() -> Vec<Box<dyn Scenario>> {
    let mut scenarios = default_scenarios();
    // Optional scenarios (require specific provider configuration)
    scenarios.push(Box::new(openai_web_search::OpenAiWebSearchScenario));
    // Multi-turn scenarios (test conversation history and reasoning ID preservation)
    scenarios.push(Box::new(multi_turn::MultiTurnFileScenario));
    scenarios.push(Box::new(multi_turn::MultiTurnReasoningScenario));
    scenarios
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_web_search_excluded_for_zai() {
        let zai_scenarios = default_scenarios_for_provider(EvalProvider::Zai);
        let has_web_search = zai_scenarios.iter().any(|s| s.name() == "web-search");
        assert!(
            !has_web_search,
            "web-search scenario should be excluded for Z.AI provider"
        );
    }

    #[test]
    fn test_web_search_included_for_vertex_claude() {
        let vertex_scenarios = default_scenarios_for_provider(EvalProvider::VertexClaude);
        let has_web_search = vertex_scenarios.iter().any(|s| s.name() == "web-search");
        assert!(
            has_web_search,
            "web-search scenario should be included for Vertex Claude provider"
        );
    }

    #[test]
    fn test_web_search_included_for_openai() {
        let openai_scenarios = default_scenarios_for_provider(EvalProvider::OpenAi);
        let has_web_search = openai_scenarios.iter().any(|s| s.name() == "web-search");
        assert!(
            has_web_search,
            "web-search scenario should be included for OpenAI provider"
        );
    }

    #[test]
    fn test_zai_has_fewer_scenarios_than_claude() {
        let zai_count = default_scenarios_for_provider(EvalProvider::Zai).len();
        let claude_count = default_scenarios_for_provider(EvalProvider::VertexClaude).len();
        assert!(
            zai_count < claude_count,
            "Z.AI should have fewer scenarios ({}) than Claude ({}) due to web-search exclusion",
            zai_count,
            claude_count
        );
    }
}
