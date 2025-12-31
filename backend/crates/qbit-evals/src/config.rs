//! Configuration for the evaluation framework.
//!
//! Loads settings from `~/.qbit/settings.toml` with environment variable fallback.
//! Supports multiple LLM providers for running evals.

use std::fmt;
use std::str::FromStr;

use anyhow::Result;

use qbit_settings::{get_with_env_fallback, QbitSettings, SettingsManager};

/// LLM provider for running evaluations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EvalProvider {
    /// Anthropic Claude via Vertex AI (default)
    #[default]
    VertexClaude,
    /// Z.AI GLM-4.7
    Zai,
}

impl fmt::Display for EvalProvider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EvalProvider::VertexClaude => write!(f, "vertex-claude"),
            EvalProvider::Zai => write!(f, "zai"),
        }
    }
}

impl FromStr for EvalProvider {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "vertex" | "vertex-claude" | "claude" | "anthropic" => Ok(EvalProvider::VertexClaude),
            "zai" | "z.ai" | "glm" | "glm-4.7" => Ok(EvalProvider::Zai),
            _ => anyhow::bail!(
                "Unknown provider: '{}'. Valid options: vertex-claude, zai",
                s
            ),
        }
    }
}

/// Configuration for Vertex AI (Claude).
#[derive(Debug, Clone)]
pub struct VertexConfig {
    /// Vertex AI project ID.
    pub project_id: String,
    /// Vertex AI location (e.g., "us-east5").
    pub location: String,
    /// Path to service account credentials (optional, uses ADC if not set).
    pub credentials_path: Option<String>,
}

/// Configuration for Z.AI (GLM).
#[derive(Debug, Clone)]
pub struct ZaiConfig {
    /// Z.AI API key.
    pub api_key: String,
}

/// Configuration for running evaluations.
#[derive(Debug, Clone)]
pub struct EvalConfig {
    /// Which provider to use for evals.
    pub provider: EvalProvider,
    /// Vertex AI configuration (if using Vertex Claude).
    pub vertex: Option<VertexConfig>,
    /// Z.AI configuration (if using Z.AI).
    pub zai: Option<ZaiConfig>,
}

impl EvalConfig {
    /// Load eval configuration from settings.toml with env var fallback.
    ///
    /// Priority order:
    /// 1. Value in `~/.qbit/settings.toml`
    /// 2. Environment variable
    /// 3. Default value (for location only)
    pub async fn load() -> Result<Self> {
        Self::load_for_provider(EvalProvider::default()).await
    }

    /// Load eval configuration for a specific provider.
    pub async fn load_for_provider(provider: EvalProvider) -> Result<Self> {
        let settings = SettingsManager::load_standalone().await?;
        Self::from_settings_for_provider(&settings, provider)
    }

    /// Create config from loaded settings for a specific provider.
    pub fn from_settings_for_provider(
        settings: &QbitSettings,
        provider: EvalProvider,
    ) -> Result<Self> {
        match provider {
            EvalProvider::VertexClaude => {
                let vertex = Self::load_vertex_config(settings)?;
                Ok(Self {
                    provider,
                    vertex: Some(vertex),
                    zai: None,
                })
            }
            EvalProvider::Zai => {
                let zai = Self::load_zai_config(settings)?;
                Ok(Self {
                    provider,
                    vertex: None,
                    zai: Some(zai),
                })
            }
        }
    }

    /// Load Vertex AI configuration.
    fn load_vertex_config(settings: &QbitSettings) -> Result<VertexConfig> {
        let project_id = get_with_env_fallback(
            &settings.ai.vertex_ai.project_id,
            &["VERTEX_AI_PROJECT_ID", "GOOGLE_CLOUD_PROJECT"],
            None,
        )
        .ok_or_else(|| {
            anyhow::anyhow!(
                "Vertex AI project_id not configured.\n\n\
                Set in ~/.qbit/settings.toml:\n\n\
                [ai.vertex_ai]\n\
                project_id = \"your-project-id\"\n\n\
                Or set VERTEX_AI_PROJECT_ID environment variable."
            )
        })?;

        let location = get_with_env_fallback(
            &settings.ai.vertex_ai.location,
            &["VERTEX_AI_LOCATION"],
            Some("us-east5".to_string()),
        )
        .unwrap(); // Safe: has default

        let credentials_path = get_with_env_fallback(
            &settings.ai.vertex_ai.credentials_path,
            &[
                "VERTEX_AI_CREDENTIALS_PATH",
                "GOOGLE_APPLICATION_CREDENTIALS",
            ],
            None,
        );

        Ok(VertexConfig {
            project_id,
            location,
            credentials_path,
        })
    }

    /// Load Z.AI configuration.
    fn load_zai_config(settings: &QbitSettings) -> Result<ZaiConfig> {
        let api_key = get_with_env_fallback(&settings.ai.zai.api_key, &["ZAI_API_KEY"], None)
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "Z.AI API key not configured.\n\n\
                Set in ~/.qbit/settings.toml:\n\n\
                [ai.zai]\n\
                api_key = \"your-api-key\"\n\n\
                Or set ZAI_API_KEY environment variable."
                )
            })?;

        Ok(ZaiConfig { api_key })
    }

    /// Create config from loaded settings (legacy compatibility).
    pub fn from_settings(settings: &QbitSettings) -> Result<Self> {
        Self::from_settings_for_provider(settings, EvalProvider::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_eval_config_from_settings_with_values() {
        let mut settings = QbitSettings::default();
        settings.ai.vertex_ai.project_id = Some("test-project".to_string());
        settings.ai.vertex_ai.location = Some("us-central1".to_string());

        let config =
            EvalConfig::from_settings_for_provider(&settings, EvalProvider::VertexClaude).unwrap();
        assert_eq!(config.provider, EvalProvider::VertexClaude);
        let vertex = config.vertex.unwrap();
        assert_eq!(vertex.project_id, "test-project");
        assert_eq!(vertex.location, "us-central1");
    }

    #[test]
    fn test_eval_config_default_location() {
        let mut settings = QbitSettings::default();
        settings.ai.vertex_ai.project_id = Some("test-project".to_string());
        settings.ai.vertex_ai.location = None;

        let config =
            EvalConfig::from_settings_for_provider(&settings, EvalProvider::VertexClaude).unwrap();
        let vertex = config.vertex.unwrap();
        assert_eq!(vertex.location, "us-east5");
    }

    #[test]
    fn test_eval_config_missing_project_id() {
        let settings = QbitSettings::default();
        // Don't set any env vars, project_id should be None

        let result =
            EvalConfig::from_settings_for_provider(&settings, EvalProvider::VertexClaude);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("project_id not configured"));
    }

    #[test]
    fn test_provider_from_str() {
        assert_eq!(
            EvalProvider::from_str("vertex").unwrap(),
            EvalProvider::VertexClaude
        );
        assert_eq!(
            EvalProvider::from_str("vertex-claude").unwrap(),
            EvalProvider::VertexClaude
        );
        assert_eq!(
            EvalProvider::from_str("claude").unwrap(),
            EvalProvider::VertexClaude
        );
        assert_eq!(EvalProvider::from_str("zai").unwrap(), EvalProvider::Zai);
        assert_eq!(EvalProvider::from_str("z.ai").unwrap(), EvalProvider::Zai);
        assert_eq!(EvalProvider::from_str("glm").unwrap(), EvalProvider::Zai);
        assert!(EvalProvider::from_str("unknown").is_err());
    }

    #[test]
    fn test_provider_display() {
        assert_eq!(EvalProvider::VertexClaude.to_string(), "vertex-claude");
        assert_eq!(EvalProvider::Zai.to_string(), "zai");
    }
}
