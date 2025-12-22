//! LLM-based evaluation metrics.
//!
//! Uses Vertex Claude Haiku to evaluate agent outputs against criteria.

use anyhow::Result;
use async_trait::async_trait;
use rig::completion::{CompletionModel as RigCompletionModel, Message};
use rig::message::{Text, UserContent};
use rig::one_or_many::OneOrMany;
use rig_anthropic_vertex::{models, Client};

use super::{EvalContext, Metric, MetricResult};

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
    let project_id = std::env::var("VERTEX_AI_PROJECT_ID")
        .or_else(|_| std::env::var("GOOGLE_CLOUD_PROJECT"))
        .map_err(|_| anyhow::anyhow!("VERTEX_AI_PROJECT_ID not set"))?;

    let location = std::env::var("VERTEX_AI_LOCATION").unwrap_or_else(|_| "us-east5".to_string());

    let client = Client::from_env(&project_id, &location).await?;
    Ok(client.completion_model(models::CLAUDE_HAIKU_4_5))
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
}

impl LlmJudgeMetric {
    /// Create a new LLM judge metric.
    pub fn new(name: impl Into<String>, criteria: impl Into<String>, threshold: f64) -> Self {
        Self {
            name: name.into(),
            criteria: criteria.into(),
            threshold,
        }
    }

    /// Create with default threshold of 0.7.
    pub fn with_criteria(name: impl Into<String>, criteria: impl Into<String>) -> Self {
        Self::new(name, criteria, 0.7)
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

        let prompt = format!(
            r#"## Original Task
{prompt}

## Assistant Response
{response}

## Evaluation Criteria
{criteria}

## Instructions
Evaluate whether the assistant's response meets the criteria above.

Respond with EXACTLY one of:
- "PASS" if the criteria are fully met
- "FAIL: <reason>" if the criteria are not met (explain why briefly)

Your response:"#,
            prompt = ctx.prompt,
            response = ctx.agent_output.response,
            criteria = self.criteria,
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
            max_tokens: Some(256),
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

        let response_upper = response_text.trim().to_uppercase();

        if response_upper.starts_with("PASS") {
            Ok(MetricResult::Pass)
        } else if response_upper.starts_with("FAIL") {
            let reason = response_text
                .trim()
                .strip_prefix("FAIL:")
                .or_else(|| response_text.trim().strip_prefix("FAIL"))
                .map(|s| s.trim().to_string())
                .unwrap_or_else(|| "Criteria not met".to_string());
            Ok(MetricResult::Fail { reason })
        } else {
            // Unexpected response format - treat as inconclusive
            tracing::warn!(
                metric = %self.name,
                response = %response_text,
                "Unexpected LLM judge response format"
            );
            Ok(MetricResult::Fail {
                reason: format!(
                    "Unexpected judge response: {}",
                    response_text.chars().take(100).collect::<String>()
                ),
            })
        }
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
