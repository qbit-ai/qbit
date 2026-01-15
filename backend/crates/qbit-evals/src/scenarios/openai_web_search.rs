//! OpenAI web search evaluation scenario.
//!
//! Tests OpenAI's native web_search_preview tool capability.
//! This scenario verifies that the agent can use OpenAI's server-side web search
//! to retrieve current information.

use async_trait::async_trait;

use crate::color;
use crate::config::{EvalConfig, EvalProvider};
use crate::metrics::{EvalContext, LlmJudgeMetric, Metric, MetricResult};
use crate::outcome::EvalReport;
use crate::runner::{AgentOutput, EvalRunner, VerboseConfig};
use crate::scenarios::Scenario;

/// Scenario: Use OpenAI's native web search to answer questions requiring current information.
pub struct OpenAiWebSearchScenario;

#[async_trait]
impl Scenario for OpenAiWebSearchScenario {
    fn name(&self) -> &str {
        "openai-web-search"
    }

    fn description(&self) -> &str {
        "Use OpenAI's native web_search_preview tool to find current information"
    }

    fn testbed(&self) -> &str {
        "minimal"
    }

    fn prompt(&self) -> &str {
        // Ask a question that requires web search to answer accurately
        // This should trigger OpenAI's web_search_preview tool
        "Search the web to find and summarize the latest news or developments \
         about OpenAI. Include at least 2-3 recent announcements, product updates, \
         or news items from the past few months."
    }

    fn system_prompt(&self) -> Option<&str> {
        Some(
            r#"You are an AI assistant with access to web search capabilities.
When asked about current information, recent events, or topics that may have
changed since your training, use your web search tool to find accurate, up-to-date
information.

For this task:
1. Use web search to find the information requested
2. Provide a clear, factual summary based on the search results
3. Include specific details like dates when available

Complete the task efficiently and provide accurate information."#,
        )
    }

    fn metrics(&self) -> Vec<Box<dyn Metric>> {
        vec![
            Box::new(LlmJudgeMetric::new(
                "contains_news_items",
                "Response contains at least 2 specific news items, announcements, or updates about OpenAI",
                0.8,
            )),
            Box::new(WebSearchToolUsedMetric),
        ]
    }

    /// Custom run implementation that uses OpenAI provider with web search enabled.
    async fn run(&self, runner: &EvalRunner) -> anyhow::Result<EvalReport> {
        let start = std::time::Instant::now();

        // Setup minimal testbed
        let workspace = runner.setup_testbed(self.testbed()).await?;

        // Load OpenAI config
        let config = EvalConfig::load_for_provider(EvalProvider::OpenAi).await?;

        // Execute with OpenAI model and web search enabled
        let agent_output = execute_with_openai_web_search(
            &workspace,
            self.prompt(),
            self.system_prompt(),
            &VerboseConfig::default(),
            &config,
        )
        .await?;

        // Create report
        let mut report = EvalReport::new(
            self.name(),
            agent_output.clone(),
            start.elapsed().as_millis() as u64,
        );

        // Evaluate metrics
        let ctx = EvalContext {
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

/// Metric that checks if web search was used (based on response characteristics).
pub struct WebSearchToolUsedMetric;

#[async_trait]
impl Metric for WebSearchToolUsedMetric {
    fn name(&self) -> &str {
        "web_search_used"
    }

    async fn evaluate(&self, ctx: &EvalContext) -> anyhow::Result<MetricResult> {
        // Check if the response contains citations or search-like markers
        // OpenAI's web search typically includes citations or source references
        let response = &ctx.agent_output.response;

        // The response should be non-empty and reasonably long (indicating real content)
        if response.is_empty() {
            return Ok(MetricResult::Fail {
                reason: "No response received".to_string(),
            });
        }

        // A good web search response should have substantial content
        if response.len() < 200 {
            return Ok(MetricResult::Fail {
                reason: format!(
                    "Response too short ({} chars) - may not have used web search",
                    response.len()
                ),
            });
        }

        // Check for indicators that web search was likely used:
        // - Multiple paragraphs or bullet points
        // - Specific dates or version numbers
        // - Multiple topics covered
        let has_structure =
            response.contains('\n') || response.contains("- ") || response.contains("• ");
        let has_specifics = response.chars().filter(|c| c.is_numeric()).count() > 3;

        if has_structure && has_specifics {
            Ok(MetricResult::Pass)
        } else {
            Ok(MetricResult::Fail {
                reason: "Response lacks structure or specific details expected from web search"
                    .to_string(),
            })
        }
    }
}

/// Execute a prompt with OpenAI Responses API and web search enabled.
///
/// Note: We use direct HTTP requests here because rig-core's Responses API
/// abstraction doesn't support OpenAI's server-side tools like `web_search_preview`.
/// The `additional_params` approach doesn't work as rig-core's `ResponsesToolDefinition`
/// only supports "function" type tools.
async fn execute_with_openai_web_search(
    _workspace: &std::path::Path,
    prompt: &str,
    system_prompt: Option<&str>,
    verbose_config: &VerboseConfig,
    config: &EvalConfig,
) -> anyhow::Result<AgentOutput> {
    use serde_json::{json, Value};

    let openai_config = config
        .openai
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("OpenAI configuration not available"))?;

    // Use gpt-4o which supports web search
    let model_id = "gpt-4o";

    let start = std::time::Instant::now();

    // Print the user prompt if verbose
    if verbose_config.enabled {
        println!();
        println!(
            "{}",
            color::cyan(&format!(
                "━━━ User ({} with web_search via Responses API) ━━━",
                model_id
            ))
        );
        println!("{}", prompt);
    }

    // Build request body for OpenAI Responses API with web_search_preview tool
    let mut request_body = json!({
        "model": model_id,
        "input": prompt,
        "tools": [
            {
                "type": "web_search_preview",
                "search_context_size": "medium"
            }
        ],
        "temperature": 0.3,
        "max_output_tokens": 1024
    });

    // Add system instructions if provided
    if let Some(sys_prompt) = system_prompt {
        request_body["instructions"] = json!(sys_prompt);
    }

    // Make direct HTTP request to OpenAI Responses API
    let client = reqwest::Client::new();
    let response = client
        .post("https://api.openai.com/v1/responses")
        .header("Authorization", format!("Bearer {}", openai_config.api_key))
        .header("Content-Type", "application/json")
        .json(&request_body)
        .send()
        .await?;

    let status = response.status();
    let response_text = response.text().await?;

    if !status.is_success() {
        return Err(anyhow::anyhow!(
            "OpenAI API error ({}): {}",
            status,
            response_text
        ));
    }

    // Parse the response
    let response_json: Value = serde_json::from_str(&response_text)?;

    // Extract text from the output array
    let mut result_text = String::new();
    if let Some(output) = response_json.get("output").and_then(|o| o.as_array()) {
        for item in output {
            if item.get("type").and_then(|t| t.as_str()) == Some("message") {
                if let Some(content) = item.get("content").and_then(|c| c.as_array()) {
                    for content_item in content {
                        if content_item.get("type").and_then(|t| t.as_str()) == Some("output_text")
                        {
                            if let Some(text) = content_item.get("text").and_then(|t| t.as_str()) {
                                result_text.push_str(text);
                            }
                        }
                    }
                }
            }
        }
    }

    // Extract token usage
    let tokens_used = response_json
        .get("usage")
        .and_then(|u| u.get("total_tokens"))
        .and_then(|t| t.as_u64())
        .map(|t| t as u32);

    if verbose_config.enabled {
        println!("\n{}", color::yellow("━━━ Agent ━━━"));
        println!("{}", result_text);
    }

    Ok(AgentOutput {
        response: result_text.trim().to_string(),
        tool_calls: vec![], // Web search is server-side, not tracked as tool calls
        files_modified: vec![],
        duration_ms: start.elapsed().as_millis() as u64,
        tokens_used,
    })
}

/// Testbed files for the openai-web-search scenario (minimal - no files needed).
pub fn testbed_files() -> Vec<(String, String)> {
    vec![(
        "README.md".to_string(),
        "# OpenAI Web Search Test\n\nMinimal workspace for testing OpenAI web search.\n"
            .to_string(),
    )]
}
