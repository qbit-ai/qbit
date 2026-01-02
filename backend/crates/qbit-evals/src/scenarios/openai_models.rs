//! OpenAI model configuration evaluation scenarios.
//!
//! These scenarios test that each OpenAI model is correctly configured
//! by sending a simple "hello world" prompt and verifying a successful response.
//!
//! This is useful for:
//! - Validating API key configuration
//! - Testing model availability
//! - Verifying the OpenAI provider integration works end-to-end

use async_trait::async_trait;

use crate::metrics::{Metric, MetricResult};
use crate::outcome::EvalReport;
use crate::runner::{AgentOutput, EvalRunner};
use crate::scenarios::Scenario;

/// OpenAI models to test for basic connectivity.
/// These are unique model IDs (not counting reasoning effort variants).
pub const OPENAI_TEST_MODELS: &[(&str, &str)] = &[
    // GPT-5 series
    ("gpt-5.2", "GPT 5.2"),
    ("gpt-5.1", "GPT 5.1"),
    ("gpt-5", "GPT 5"),
    ("gpt-5-mini", "GPT 5 Mini"),
    ("gpt-5-nano", "GPT 5 Nano"),
    // GPT-4.1 series
    ("gpt-4.1", "GPT 4.1"),
    ("gpt-4.1-mini", "GPT 4.1 Mini"),
    ("gpt-4.1-nano", "GPT 4.1 Nano"),
    // GPT-4o series
    ("gpt-4o", "GPT 4o"),
    ("gpt-4o-mini", "GPT 4o Mini"),
    ("chatgpt-4o-latest", "ChatGPT 4o Latest"),
    // o-series reasoning models
    ("o4-mini", "o4 Mini"),
    ("o3", "o3"),
    ("o3-mini", "o3 Mini"),
    ("o1", "o1"),
    // Codex models (gpt-5.2-codex* not yet available)
    ("gpt-5.1-codex", "GPT 5.1 Codex"),
    ("gpt-5.1-codex-max", "GPT 5.1 Codex Max"),
    ("codex-mini-latest", "Codex Mini"),
];

/// OpenAI models that don't support the temperature parameter.
const NO_TEMPERATURE_MODELS: &[&str] = &[
    "o1",
    "o1-preview",
    "o3",
    "o3-mini",
    "o4-mini",
    "gpt-5",
    "gpt-5-mini",
    "gpt-5-nano",
    "gpt-5.1-codex",
    "gpt-5.1-codex-max",
    "codex-mini-latest",
];

/// Check if a model supports the temperature parameter.
fn supports_temperature(model_id: &str) -> bool {
    !NO_TEMPERATURE_MODELS.contains(&model_id)
}

/// Simple metric that checks if a response was received.
#[derive(Default)]
pub struct ResponseReceivedMetric;

impl ResponseReceivedMetric {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Metric for ResponseReceivedMetric {
    fn name(&self) -> &str {
        "response_received"
    }

    async fn evaluate(&self, ctx: &crate::metrics::EvalContext) -> anyhow::Result<MetricResult> {
        // Check that we got a non-empty response
        let response = &ctx.agent_output.response;

        if response.is_empty() {
            Ok(MetricResult::Fail {
                reason: "No response received from model".to_string(),
            })
        } else {
            Ok(MetricResult::Pass)
        }
    }
}

/// Scenario for testing a single OpenAI model.
pub struct OpenAiModelScenario {
    model_id: String,
    model_name: String,
}

impl OpenAiModelScenario {
    pub fn new(model_id: &str, model_name: &str) -> Self {
        Self {
            model_id: model_id.to_string(),
            model_name: model_name.to_string(),
        }
    }

    /// Get the model ID to test.
    pub fn model_id(&self) -> &str {
        &self.model_id
    }
}

#[async_trait]
impl Scenario for OpenAiModelScenario {
    fn name(&self) -> &str {
        // We return a static str, but for dynamic names we need to use Box::leak
        // This is fine for short-lived scenarios
        Box::leak(format!("openai-{}", self.model_id).into_boxed_str())
    }

    fn description(&self) -> &str {
        Box::leak(
            format!(
                "Test {} model configuration with hello world prompt",
                self.model_name
            )
            .into_boxed_str(),
        )
    }

    fn testbed(&self) -> &str {
        "minimal" // Use the minimal testbed (empty workspace)
    }

    fn prompt(&self) -> &str {
        "Say hello world. Keep your response brief."
    }

    fn system_prompt(&self) -> Option<&str> {
        Some("You are a helpful assistant. Respond briefly and concisely.")
    }

    fn metrics(&self) -> Vec<Box<dyn Metric>> {
        vec![Box::new(ResponseReceivedMetric::new())]
    }

    /// Custom run implementation that uses the specific OpenAI model.
    async fn run(&self, runner: &EvalRunner) -> anyhow::Result<EvalReport> {
        use crate::config::{EvalConfig, EvalProvider};
        use crate::runner::VerboseConfig;

        let start = std::time::Instant::now();

        // Setup minimal testbed
        let workspace = runner.setup_testbed(self.testbed()).await?;

        // Load OpenAI config
        let config = EvalConfig::load_for_provider(EvalProvider::OpenAi).await?;

        // Execute with specific model
        let agent_output = execute_with_openai_model(
            &workspace,
            self.prompt(),
            self.system_prompt(),
            &VerboseConfig::default(),
            &config,
            &self.model_id,
        )
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

/// Execute a prompt with a specific OpenAI model.
async fn execute_with_openai_model(
    _workspace: &std::path::Path,
    prompt: &str,
    system_prompt: Option<&str>,
    verbose_config: &crate::runner::VerboseConfig,
    config: &crate::config::EvalConfig,
    model_id: &str,
) -> anyhow::Result<AgentOutput> {
    use rig::client::CompletionClient;
    use rig::completion::{CompletionModel, CompletionRequest, Message};
    use rig::message::{Text, UserContent};
    use rig::one_or_many::OneOrMany;
    use rig::providers::openai as rig_openai;

    let openai_config = config
        .openai
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("OpenAI configuration not available"))?;

    let client: rig_openai::Client = rig_openai::Client::new(&openai_config.api_key)
        .map_err(|e| anyhow::anyhow!("Failed to create OpenAI client: {}", e))?;
    let model = client.completion_model(model_id);

    let start = std::time::Instant::now();

    // Print the user prompt if verbose
    if verbose_config.enabled {
        println!();
        println!("\x1b[36m━━━ User ({}) ━━━\x1b[0m", model_id);
        println!("{}", prompt);
    }

    // Build simple completion request (no tools for hello world test)
    let chat_history = vec![Message::User {
        content: OneOrMany::one(UserContent::Text(Text {
            text: prompt.to_string(),
        })),
    }];

    // Some models (o-series, codex, gpt-5 base) don't support temperature
    let temperature = if supports_temperature(model_id) {
        Some(0.3)
    } else {
        None
    };

    let request = CompletionRequest {
        preamble: system_prompt.map(|s| s.to_string()),
        chat_history: OneOrMany::many(chat_history.clone())
            .unwrap_or_else(|_| OneOrMany::one(chat_history[0].clone())),
        documents: vec![],
        tools: vec![], // No tools needed for hello world
        temperature,
        max_tokens: Some(256),
        tool_choice: None,
        additional_params: None,
    };

    let response = model.completion(request).await?;

    // Extract text response
    let mut response_text = String::new();
    for content in response.choice.iter() {
        if let rig::completion::AssistantContent::Text(text) = content {
            response_text.push_str(&text.text);
        }
    }

    if verbose_config.enabled {
        println!("\n\x1b[33m━━━ Agent ━━━\x1b[0m");
        println!("{}", response_text);
    }

    Ok(AgentOutput {
        response: response_text.trim().to_string(),
        tool_calls: vec![],
        files_modified: vec![],
        duration_ms: start.elapsed().as_millis() as u64,
        tokens_used: Some(response.usage.total_tokens as u32),
    })
}

/// Get all OpenAI model scenarios.
pub fn all_openai_model_scenarios() -> Vec<Box<dyn Scenario>> {
    OPENAI_TEST_MODELS
        .iter()
        .map(|(id, name)| Box::new(OpenAiModelScenario::new(id, name)) as Box<dyn Scenario>)
        .collect()
}

/// Testbed files for openai-models scenarios (minimal/empty).
pub fn testbed_files() -> Vec<(String, String)> {
    // Use the same minimal testbed as web_search
    vec![(
        "README.md".to_string(),
        "# OpenAI Model Test\n\nMinimal workspace for testing OpenAI model connectivity.\n"
            .to_string(),
    )]
}
