//! Configuration for the evaluation framework.
//!
//! Loads settings from `~/.qbit/settings.toml` with environment variable fallback.

use anyhow::Result;

use qbit_settings::{get_with_env_fallback, QbitSettings, SettingsManager};

/// Configuration for running evaluations.
#[derive(Debug, Clone)]
pub struct EvalConfig {
    /// Vertex AI project ID.
    pub project_id: String,
    /// Vertex AI location (e.g., "us-east5").
    pub location: String,
    /// Path to service account credentials (optional, uses ADC if not set).
    pub credentials_path: Option<String>,
}

impl EvalConfig {
    /// Load eval configuration from settings.toml with env var fallback.
    ///
    /// Priority order:
    /// 1. Value in `~/.qbit/settings.toml` under `[ai.vertex_ai]`
    /// 2. Environment variable
    /// 3. Default value (for location only)
    pub async fn load() -> Result<Self> {
        let settings = SettingsManager::load_standalone().await?;
        Self::from_settings(&settings)
    }

    /// Create config from loaded settings.
    pub fn from_settings(settings: &QbitSettings) -> Result<Self> {
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

        Ok(Self {
            project_id,
            location,
            credentials_path,
        })
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

        let config = EvalConfig::from_settings(&settings).unwrap();
        assert_eq!(config.project_id, "test-project");
        assert_eq!(config.location, "us-central1");
    }

    #[test]
    fn test_eval_config_default_location() {
        let mut settings = QbitSettings::default();
        settings.ai.vertex_ai.project_id = Some("test-project".to_string());
        settings.ai.vertex_ai.location = None;

        let config = EvalConfig::from_settings(&settings).unwrap();
        assert_eq!(config.location, "us-east5");
    }

    #[test]
    fn test_eval_config_missing_project_id() {
        let settings = QbitSettings::default();
        // Don't set any env vars, project_id should be None

        let result = EvalConfig::from_settings(&settings);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("project_id not configured"));
    }
}
