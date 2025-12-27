use anyhow::Result;
use async_trait::async_trait;

use super::{EvalContext, Metric, MetricResult};

pub struct TokenTrackingMetric {
    name: String,
    max_tokens: Option<u32>,
}

impl TokenTrackingMetric {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            max_tokens: None,
        }
    }

    pub fn with_max(name: &str, max_tokens: u32) -> Self {
        Self {
            name: name.to_string(),
            max_tokens: Some(max_tokens),
        }
    }
}

#[async_trait]
impl Metric for TokenTrackingMetric {
    fn name(&self) -> &str {
        &self.name
    }

    async fn evaluate(&self, ctx: &EvalContext) -> Result<MetricResult> {
        match ctx.agent_output.tokens_used {
            Some(tokens) => {
                if let Some(max) = self.max_tokens {
                    if tokens > max {
                        Ok(MetricResult::Fail {
                            reason: format!("Token usage {} exceeded maximum {}", tokens, max),
                        })
                    } else {
                        Ok(MetricResult::Pass)
                    }
                } else {
                    Ok(MetricResult::Score {
                        value: tokens as f64,
                        max: f64::MAX,
                    })
                }
            }
            None => Ok(MetricResult::Skip {
                reason: "Token usage not available".to_string(),
            }),
        }
    }
}
