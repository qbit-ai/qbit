//! LLM-based evaluation metrics.
//!
//! Uses Vertex Claude Sonnet to evaluate agent outputs against criteria.

use std::path::Path;

use anyhow::Result;
use async_trait::async_trait;
use rig::completion::{CompletionModel as RigCompletionModel, Message, ToolDefinition};
use rig::message::{Text, UserContent};
use rig::one_or_many::OneOrMany;
use rig_anthropic_vertex::{models, Client};
use serde::Deserialize;
use serde_json::json;

use super::{EvalContext, Metric, MetricResult};
use crate::config::EvalConfig;

// =============================================================================
// Judge Tools
// =============================================================================

/// Build tool definitions for the judge.
fn build_tool_definitions() -> Vec<ToolDefinition> {
    vec![
        ToolDefinition {
            name: "read_file".to_string(),
            description: "Read the contents of a file from the workspace. Use this to verify \
                 actual code changes, check file contents, or examine implementation details."
                .to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Path to the file to read, relative to the workspace root."
                    }
                },
                "required": ["path"]
            }),
        },
        ToolDefinition {
            name: "list_files".to_string(),
            description: "List files and directories in a path. Directories end with '/'. \
                 Use this to discover what files exist in the workspace."
                .to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Directory path relative to workspace root. Use '.' for root."
                    }
                },
                "required": ["path"]
            }),
        },
    ]
}

/// Execute the read_file tool.
fn execute_read_file(workspace: &Path, path: &str) -> String {
    let full_path = workspace.join(path);

    // Security: ensure path is within workspace
    let canonical = match full_path.canonicalize() {
        Ok(p) => p,
        Err(e) => return format!("Error: Cannot resolve path '{}': {}", path, e),
    };
    let workspace_canonical = match workspace.canonicalize() {
        Ok(p) => p,
        Err(e) => return format!("Error: Cannot resolve workspace: {}", e),
    };

    if !canonical.starts_with(&workspace_canonical) {
        return format!("Error: Path '{}' is outside the workspace", path);
    }

    match std::fs::read_to_string(&canonical) {
        Ok(content) => content,
        Err(e) => format!("Error: Cannot read '{}': {}", path, e),
    }
}

/// Execute the list_files tool.
fn execute_list_files(workspace: &Path, path: &str) -> String {
    let full_path = workspace.join(path);

    // Security: ensure path is within workspace
    let canonical = match full_path.canonicalize() {
        Ok(p) => p,
        Err(e) => return format!("Error: Cannot resolve path '{}': {}", path, e),
    };
    let workspace_canonical = match workspace.canonicalize() {
        Ok(p) => p,
        Err(e) => return format!("Error: Cannot resolve workspace: {}", e),
    };

    if !canonical.starts_with(&workspace_canonical) {
        return format!("Error: Path '{}' is outside the workspace", path);
    }

    match std::fs::read_dir(&canonical) {
        Ok(entries) => {
            let files: Vec<String> = entries
                .filter_map(|entry| entry.ok())
                .map(|entry| {
                    let name = entry.file_name().to_string_lossy().to_string();
                    if entry.path().is_dir() {
                        format!("{}/", name)
                    } else {
                        name
                    }
                })
                .collect();
            files.join("\n")
        }
        Err(e) => format!("Error: Cannot read directory '{}': {}", path, e),
    }
}

/// Tool arguments for deserialization.
#[derive(Debug, Deserialize)]
struct PathArg {
    path: String,
}

// =============================================================================
// LLM Judge
// =============================================================================

/// System prompt for LLM judge evaluations.
const JUDGE_SYSTEM_PROMPT: &str = r#"You are an expert code reviewer evaluating AI assistant outputs.
You will be given:
1. The original task/prompt given to the assistant
2. The assistant's response
3. Evaluation criteria to judge against

Evaluate strictly and objectively. Focus on whether the criteria are met, not on style preferences.
"#;

/// Create a Vertex AI client for LLM judge evaluations.
async fn create_judge_client() -> Result<rig_anthropic_vertex::CompletionModel> {
    // Load configuration from settings.toml with env var fallback
    let config = EvalConfig::load().await?;

    let vertex_config = config
        .vertex
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Vertex AI configuration not available for LLM judge"))?;

    // Create client using service account credentials if available, otherwise fall back to ADC
    let client = if let Some(ref creds_path) = vertex_config.credentials_path {
        Client::from_service_account(
            creds_path,
            &vertex_config.project_id,
            &vertex_config.location,
        )
        .await?
    } else {
        Client::from_env(&vertex_config.project_id, &vertex_config.location).await?
    };
    Ok(client.completion_model(models::CLAUDE_SONNET_4_5))
}

/// Metric that uses an LLM to judge whether output meets criteria.
///
/// Returns Pass if the LLM determines the criteria are met, Fail otherwise.
pub struct LlmJudgeMetric {
    /// Name of this metric instance.
    name: String,
    /// Criteria for the LLM to evaluate against.
    criteria: String,
    /// Threshold for passing (0.0 to 1.0). Default is 0.7.
    #[allow(dead_code)]
    threshold: f64,
    /// Whether to give the judge read-only tools to explore the workspace.
    use_tools: bool,
}

impl LlmJudgeMetric {
    /// Create a new LLM judge metric.
    pub fn new(name: impl Into<String>, criteria: impl Into<String>, threshold: f64) -> Self {
        Self {
            name: name.into(),
            criteria: criteria.into(),
            threshold,
            use_tools: false,
        }
    }

    /// Create with default threshold of 0.7.
    pub fn with_criteria(name: impl Into<String>, criteria: impl Into<String>) -> Self {
        Self::new(name, criteria, 0.7)
    }

    /// Enable read-only tools (read_file, list_files) for the judge to explore the workspace.
    pub fn with_tools(mut self) -> Self {
        self.use_tools = true;
        self
    }
}

#[async_trait]
impl Metric for LlmJudgeMetric {
    fn name(&self) -> &str {
        &self.name
    }

    async fn evaluate(&self, ctx: &EvalContext) -> Result<MetricResult> {
        let model = match create_judge_client().await {
            Ok(m) => m,
            Err(e) => {
                tracing::warn!(
                    metric = %self.name,
                    error = %e,
                    "Failed to create LLM client for judge metric"
                );
                return Ok(MetricResult::Skip {
                    reason: format!("LLM client unavailable: {}", e),
                });
            }
        };

        // Build tool calls section if any
        let tool_calls_section = if ctx.agent_output.tool_calls.is_empty() {
            String::new()
        } else {
            let calls: Vec<String> = ctx
                .agent_output
                .tool_calls
                .iter()
                .map(|tc| {
                    format!(
                        "- {}({}): {}",
                        tc.name,
                        serde_json::to_string(&tc.input).unwrap_or_default(),
                        if tc.success { "success" } else { "failed" }
                    )
                })
                .collect();
            format!("\n\n## Tool Calls Made\n{}", calls.join("\n"))
        };

        // Build the initial prompt
        let tools_note = if self.use_tools {
            "\nYou have access to read_file and list_files tools to explore the workspace and verify the actual code/changes. Use them as needed before making your verdict.\n"
        } else {
            ""
        };

        let initial_prompt = format!(
            r#"## Original Task
{prompt}

## Assistant Response
{response}{tool_calls_section}

## Evaluation Criteria
{criteria}

## Instructions
Evaluate whether the assistant's response meets the criteria above.
{tools_note}
When you are ready to give your verdict, your response MUST start with exactly one of these two words:
- PASS - if the criteria are fully met
- FAIL - if the criteria are not met

If FAIL, add a brief reason after a colon, like: FAIL: reason here"#,
            prompt = ctx.prompt,
            response = ctx.agent_output.response,
            tool_calls_section = tool_calls_section,
            criteria = self.criteria,
            tools_note = tools_note,
        );

        // Build tools if enabled
        let tools: Vec<ToolDefinition> = if self.use_tools {
            build_tool_definitions()
        } else {
            vec![]
        };

        // Agentic loop
        let mut chat_history: Vec<Message> = vec![Message::User {
            content: OneOrMany::one(UserContent::Text(Text {
                text: initial_prompt,
            })),
        }];

        const MAX_ITERATIONS: usize = 10;
        for iteration in 0..MAX_ITERATIONS {
            let request = rig::completion::CompletionRequest {
                preamble: Some(JUDGE_SYSTEM_PROMPT.to_string()),
                chat_history: OneOrMany::many(chat_history.clone())
                    .unwrap_or_else(|_| OneOrMany::one(chat_history[0].clone())),
                documents: vec![],
                tools: tools.clone(),
                temperature: Some(0.0),
                max_tokens: Some(1024),
                tool_choice: None,
                additional_params: None,
            };

            let response = model.completion(request).await?;

            // Check for tool calls
            let tool_calls: Vec<_> = response
                .choice
                .iter()
                .filter_map(|c| match c {
                    rig::completion::AssistantContent::ToolCall(tc) => Some(tc.clone()),
                    _ => None,
                })
                .collect();

            // Extract text response
            let response_text: String = response
                .choice
                .iter()
                .filter_map(|c| match c {
                    rig::completion::AssistantContent::Text(t) => Some(t.text.as_str()),
                    _ => None,
                })
                .collect::<Vec<_>>()
                .join("");

            // If no tool calls, check for verdict
            if tool_calls.is_empty() {
                return Self::parse_verdict(&response_text, &self.name);
            }

            // Process tool calls
            tracing::debug!(
                metric = %self.name,
                iteration = iteration,
                tool_count = tool_calls.len(),
                "Judge using tools"
            );

            // Add assistant message with tool calls
            chat_history.push(Message::Assistant {
                id: None,
                content: response.choice.clone(),
            });

            // Execute tools and add results
            for tool_call in tool_calls {
                let args_str = tool_call.function.arguments.to_string();
                let result = match tool_call.function.name.as_str() {
                    "read_file" => match serde_json::from_str::<PathArg>(&args_str) {
                        Ok(arg) => execute_read_file(&ctx.workspace, &arg.path),
                        Err(e) => format!("Error parsing arguments: {}", e),
                    },
                    "list_files" => match serde_json::from_str::<PathArg>(&args_str) {
                        Ok(arg) => execute_list_files(&ctx.workspace, &arg.path),
                        Err(e) => format!("Error parsing arguments: {}", e),
                    },
                    _ => format!("Unknown tool: {}", tool_call.function.name),
                };

                chat_history.push(Message::User {
                    content: OneOrMany::one(UserContent::ToolResult(rig::message::ToolResult {
                        id: tool_call.id.clone(),
                        call_id: Some(tool_call.id),
                        content: OneOrMany::one(rig::message::ToolResultContent::Text(Text {
                            text: result,
                        })),
                    })),
                });
            }
        }

        // Max iterations reached
        Ok(MetricResult::Fail {
            reason: "Judge exceeded maximum tool iterations without verdict".to_string(),
        })
    }
}

impl LlmJudgeMetric {
    /// Parse the verdict from the response text.
    fn parse_verdict(response_text: &str, metric_name: &str) -> Result<MetricResult> {
        let response_trimmed = response_text.trim();
        let response_upper = response_trimmed.to_uppercase();

        // Always log the full response at debug level for troubleshooting
        tracing::debug!(
            metric = %metric_name,
            response = %response_text,
            "LLM judge full response"
        );

        // First try: check if response starts with PASS/FAIL
        if response_upper.starts_with("PASS") {
            tracing::info!(metric = %metric_name, "Judge verdict: PASS");
            return Ok(MetricResult::Pass);
        }
        if response_upper.starts_with("FAIL") {
            let reason = response_trimmed
                .strip_prefix("FAIL:")
                .or_else(|| response_trimmed.strip_prefix("FAIL"))
                .or_else(|| response_trimmed.strip_prefix("Fail:"))
                .or_else(|| response_trimmed.strip_prefix("Fail"))
                .map(|s| s.trim().to_string())
                .unwrap_or_else(|| "Criteria not met".to_string());
            tracing::info!(metric = %metric_name, reason = %reason, "Judge verdict: FAIL");
            return Ok(MetricResult::Fail { reason });
        }

        // Fallback: look for PASS/FAIL anywhere in the response
        if response_upper.contains("PASS") && !response_upper.contains("FAIL") {
            tracing::info!(
                metric = %metric_name,
                "Judge verdict: PASS (found in response body)"
            );
            return Ok(MetricResult::Pass);
        }
        if response_upper.contains("FAIL") {
            let reason = if let Some(pos) = response_trimmed.to_uppercase().find("FAIL") {
                let after_fail = &response_trimmed[pos + 4..];
                after_fail
                    .strip_prefix(':')
                    .or(Some(after_fail))
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .unwrap_or_else(|| "Criteria not met".to_string())
            } else {
                "Criteria not met".to_string()
            };
            tracing::info!(
                metric = %metric_name,
                reason = %reason,
                "Judge verdict: FAIL (found in response body)"
            );
            return Ok(MetricResult::Fail { reason });
        }

        // Unexpected response format - include full response for debugging
        tracing::warn!(
            metric = %metric_name,
            response = %response_text,
            "Unexpected LLM judge response format - no PASS/FAIL found"
        );
        // Include more of the response in the error for debugging
        let preview_len = response_text.len().min(500);
        Ok(MetricResult::Fail {
            reason: format!(
                "Unexpected judge response (no PASS/FAIL): {}{}",
                response_text.chars().take(preview_len).collect::<String>(),
                if response_text.len() > preview_len {
                    "..."
                } else {
                    ""
                }
            ),
        })
    }
}

/// Metric that uses an LLM to score output on a numeric scale.
///
/// Returns a Score result with the LLM's numeric evaluation.
pub struct LlmScoreMetric {
    /// Name of this metric instance.
    name: String,
    /// Criteria for scoring.
    criteria: String,
    /// Minimum passing score.
    min_score: f64,
    /// Maximum possible score.
    max_score: f64,
}

impl LlmScoreMetric {
    /// Create a new LLM score metric.
    pub fn new(
        name: impl Into<String>,
        criteria: impl Into<String>,
        min_score: f64,
        max_score: f64,
    ) -> Self {
        Self {
            name: name.into(),
            criteria: criteria.into(),
            min_score,
            max_score,
        }
    }

    /// Create a metric that scores on a 0-10 scale.
    pub fn scale_10(
        name: impl Into<String>,
        criteria: impl Into<String>,
        min_passing: f64,
    ) -> Self {
        Self::new(name, criteria, min_passing, 10.0)
    }
}

#[async_trait]
impl Metric for LlmScoreMetric {
    fn name(&self) -> &str {
        &self.name
    }

    async fn evaluate(&self, ctx: &EvalContext) -> Result<MetricResult> {
        let model = match create_judge_client().await {
            Ok(m) => m,
            Err(e) => {
                tracing::warn!(
                    metric = %self.name,
                    error = %e,
                    "Failed to create LLM client for score metric"
                );
                return Ok(MetricResult::Skip {
                    reason: format!("LLM client unavailable: {}", e),
                });
            }
        };

        let prompt = format!(
            r#"## Original Task
{prompt}

## Assistant Response
{response}

## Scoring Criteria
{criteria}

## Instructions
Score the assistant's response on a scale of 0 to {max_score:.0}.

Consider:
- How well the criteria are met
- Code quality and correctness
- Completeness of the solution

Respond with EXACTLY one number between 0 and {max_score:.0} (can include decimals like 7.5).
Do not include any other text.

Your score:"#,
            prompt = ctx.prompt,
            response = ctx.agent_output.response,
            criteria = self.criteria,
            max_score = self.max_score,
        );

        let chat_history: Vec<Message> = vec![Message::User {
            content: OneOrMany::one(UserContent::Text(Text { text: prompt })),
        }];

        let request = rig::completion::CompletionRequest {
            preamble: Some(JUDGE_SYSTEM_PROMPT.to_string()),
            chat_history: OneOrMany::many(chat_history.clone())
                .unwrap_or_else(|_| OneOrMany::one(chat_history[0].clone())),
            documents: vec![],
            tools: vec![],
            temperature: Some(0.0), // Deterministic evaluation
            max_tokens: Some(32),
            tool_choice: None,
            additional_params: None,
        };

        let response = model.completion(request).await?;

        // Extract text response
        let response_text = response
            .choice
            .iter()
            .filter_map(|c| match c {
                rig::completion::AssistantContent::Text(t) => Some(t.text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("");

        // Parse the score
        let score_str = response_text.trim();
        match score_str.parse::<f64>() {
            Ok(score) => {
                let clamped = score.clamp(0.0, self.max_score);
                if (score - clamped).abs() > 0.01 {
                    tracing::warn!(
                        metric = %self.name,
                        raw_score = score,
                        clamped = clamped,
                        "Score was out of range, clamped"
                    );
                }

                if clamped >= self.min_score {
                    Ok(MetricResult::Score {
                        value: clamped,
                        max: self.max_score,
                    })
                } else {
                    Ok(MetricResult::Fail {
                        reason: format!("Score {:.1} below minimum {:.1}", clamped, self.min_score),
                    })
                }
            }
            Err(_) => {
                tracing::warn!(
                    metric = %self.name,
                    response = %response_text,
                    "Failed to parse LLM score response"
                );
                Ok(MetricResult::Fail {
                    reason: format!(
                        "Invalid score response: {}",
                        score_str.chars().take(50).collect::<String>()
                    ),
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_llm_judge_metric_creation() {
        let metric = LlmJudgeMetric::new("test", "criteria here", 0.8);
        assert_eq!(metric.name(), "test");
        assert_eq!(metric.criteria, "criteria here");
    }

    #[test]
    fn test_llm_judge_with_criteria() {
        let metric = LlmJudgeMetric::with_criteria("test", "criteria");
        assert_eq!(metric.threshold, 0.7);
    }

    #[test]
    fn test_llm_score_metric_scale_10() {
        let metric = LlmScoreMetric::scale_10("quality", "code quality", 7.0);
        assert_eq!(metric.min_score, 7.0);
        assert_eq!(metric.max_score, 10.0);
    }
}
