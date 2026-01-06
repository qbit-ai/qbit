//! OpenTelemetry/LangSmith tracing integration for Qbit.
//!
//! This module provides OpenTelemetry tracing setup that exports traces
//! to LangSmith for observability of LLM interactions and agent behavior.
//!
//! ## Configuration
//!
//! LangSmith tracing is configured via `~/.qbit/settings.toml`:
//!
//! ```toml
//! [telemetry.langsmith]
//! enabled = true
//! api_key = "$LANGSMITH_API_KEY"  # or set LANGSMITH_API_KEY env var
//! project = "my-qbit-agent"
//! # endpoint = "https://api.smith.langchain.com"  # default, or use EU endpoint
//! ```
//!
//! Or via environment variables:
//! - `LANGSMITH_API_KEY` - Your LangSmith API key
//! - `LANGSMITH_PROJECT` - Project name (optional, defaults to "default")
//! - `LANGSMITH_ENDPOINT` - API endpoint (optional, defaults to US endpoint)

use opentelemetry::trace::TracerProvider;
use opentelemetry::KeyValue;
use opentelemetry_otlp::{WithExportConfig, WithHttpConfig};
use opentelemetry_sdk::trace::{self as sdktrace, Sampler};
use opentelemetry_sdk::Resource;
use opentelemetry_semantic_conventions::resource::{SERVICE_NAME, SERVICE_VERSION};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_opentelemetry::OpenTelemetryLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Registry};

/// LangSmith configuration for OpenTelemetry tracing.
#[derive(Debug, Clone)]
pub struct LangSmithConfig {
    /// LangSmith API key (should start with `lsv2_sk_` for secret keys)
    pub api_key: String,
    /// Project name in LangSmith
    pub project: String,
    /// LangSmith API endpoint
    pub endpoint: String,
    /// Service name for this application
    pub service_name: String,
    /// Service version
    pub service_version: String,
    /// Sampling ratio (0.0 to 1.0, default 1.0 = sample everything)
    pub sampling_ratio: f64,
}

impl Default for LangSmithConfig {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            project: "default".to_string(),
            endpoint: "https://api.smith.langchain.com".to_string(),
            service_name: "qbit".to_string(),
            service_version: env!("CARGO_PKG_VERSION").to_string(),
            sampling_ratio: 1.0,
        }
    }
}

impl LangSmithConfig {
    /// Create config from environment variables.
    ///
    /// Reads from:
    /// - `LANGSMITH_API_KEY` (required)
    /// - `LANGSMITH_PROJECT` (optional, defaults to "default")
    /// - `LANGSMITH_ENDPOINT` (optional, defaults to US endpoint)
    pub fn from_env() -> Option<Self> {
        let api_key = std::env::var("LANGSMITH_API_KEY").ok()?;
        if api_key.is_empty() {
            return None;
        }

        Some(Self {
            api_key,
            project: std::env::var("LANGSMITH_PROJECT").unwrap_or_else(|_| "default".to_string()),
            endpoint: std::env::var("LANGSMITH_ENDPOINT")
                .unwrap_or_else(|_| "https://api.smith.langchain.com".to_string()),
            ..Default::default()
        })
    }

    /// Create config from settings.
    pub fn from_settings(settings: &crate::settings::LangSmithSettings) -> Option<Self> {
        if !settings.enabled {
            return None;
        }

        // Resolve API key from settings or environment
        let api_key = crate::settings::get_with_env_fallback(
            &settings.api_key,
            &["LANGSMITH_API_KEY"],
            None,
        )?;

        if api_key.is_empty() {
            return None;
        }

        Some(Self {
            api_key,
            project: settings
                .project
                .clone()
                .unwrap_or_else(|| "default".to_string()),
            endpoint: settings
                .endpoint
                .clone()
                .unwrap_or_else(|| "https://api.smith.langchain.com".to_string()),
            service_name: "qbit".to_string(),
            service_version: env!("CARGO_PKG_VERSION").to_string(),
            sampling_ratio: settings.sampling_ratio.unwrap_or(1.0),
        })
    }

    /// Get the OTLP traces endpoint URL.
    fn traces_endpoint(&self) -> String {
        format!("{}/otel/v1/traces", self.endpoint.trim_end_matches('/'))
    }
}

/// Result of telemetry initialization.
pub struct TelemetryGuard {
    /// Whether LangSmith tracing is active
    pub langsmith_active: bool,
    /// Guard for the file appender (keeps the background writer thread alive)
    pub file_guard: Option<WorkerGuard>,
}

impl Drop for TelemetryGuard {
    fn drop(&mut self) {
        // Drop the file guard first to flush any pending logs
        if self.file_guard.is_some() {
            tracing::debug!("Shutting down file logging...");
        }
        // The file_guard will be dropped automatically when TelemetryGuard is dropped,
        // which flushes and closes the file appender.
        // We explicitly take it here to ensure ordering.
        let _ = self.file_guard.take();

        // Then shutdown OpenTelemetry
        if self.langsmith_active {
            // Shutdown the tracer provider to flush pending spans
            opentelemetry::global::shutdown_tracer_provider();
        }
    }
}

/// Initialize tracing with optional LangSmith/OpenTelemetry export.
///
/// This function sets up:
/// 1. Standard `tracing_subscriber` with console output
/// 2. OpenTelemetry layer exporting to LangSmith (if configured)
///
/// # Arguments
///
/// * `langsmith_config` - Optional LangSmith configuration. If None, only console tracing is enabled.
/// * `log_level` - Log level for console output (e.g., "debug", "info", "warn")
/// * `extra_directives` - Additional tracing directives (e.g., "qbit=debug")
///
/// # Returns
///
/// A `TelemetryGuard` that should be held for the lifetime of the application.
/// When dropped, it will flush pending traces.
pub fn init_tracing(
    langsmith_config: Option<LangSmithConfig>,
    log_level: &str,
    extra_directives: &[&str],
) -> Result<TelemetryGuard, Box<dyn std::error::Error + Send + Sync>> {
    // Build the base env filter
    let mut filter = EnvFilter::from_default_env();

    // Add log level directive
    if let Ok(directive) = format!("qbit={}", log_level).parse() {
        filter = filter.add_directive(directive);
    }

    // Add extra directives
    for directive_str in extra_directives {
        if let Ok(directive) = directive_str.parse() {
            filter = filter.add_directive(directive);
        }
    }

    // Set up file logging to ~/.qbit/backend.log
    let (file_layer, file_guard) = if let Some(home) = dirs::home_dir() {
        let qbit_dir = home.join(".qbit");
        // Create ~/.qbit directory if it doesn't exist
        if let Err(e) = std::fs::create_dir_all(&qbit_dir) {
            eprintln!("Warning: Failed to create ~/.qbit directory: {}", e);
            (None, None)
        } else {
            let file_appender = tracing_appender::rolling::never(&qbit_dir, "backend.log");
            let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);
            let file_layer = tracing_subscriber::fmt::layer()
                .with_writer(non_blocking)
                .with_ansi(false);
            (Some(file_layer), Some(guard))
        }
    } else {
        (None, None)
    };

    // Create the base subscriber with fmt layer
    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_target(true)
        .with_thread_ids(false)
        .with_file(false)
        .with_line_number(false);

    if let Some(config) = langsmith_config {
        // Set up OpenTelemetry with OTLP exporter to LangSmith
        let tracer_provider = init_langsmith_tracer(&config)?;
        let tracer = tracer_provider.tracer("qbit");

        // Create the OpenTelemetry layer
        let otel_layer = OpenTelemetryLayer::new(tracer);

        // Build the subscriber with both layers
        Registry::default()
            .with(filter)
            .with(file_layer)
            .with(fmt_layer)
            .with(otel_layer)
            .try_init()
            .map_err(|e| format!("Failed to initialize tracing: {}", e))?;

        tracing::info!(
            langsmith_project = %config.project,
            langsmith_endpoint = %config.endpoint,
            "LangSmith tracing enabled"
        );

        Ok(TelemetryGuard {
            langsmith_active: true,
            file_guard,
        })
    } else {
        // No LangSmith, just use fmt layer
        Registry::default()
            .with(filter)
            .with(file_layer)
            .with(fmt_layer)
            .try_init()
            .map_err(|e| format!("Failed to initialize tracing: {}", e))?;

        Ok(TelemetryGuard {
            langsmith_active: false,
            file_guard,
        })
    }
}

/// Initialize the OpenTelemetry tracer provider for LangSmith.
fn init_langsmith_tracer(
    config: &LangSmithConfig,
) -> Result<sdktrace::TracerProvider, Box<dyn std::error::Error + Send + Sync>> {
    // Create the OTLP exporter with LangSmith endpoint
    let exporter = opentelemetry_otlp::SpanExporter::builder()
        .with_http()
        .with_endpoint(config.traces_endpoint())
        .with_headers(std::collections::HashMap::from([(
            "x-api-key".to_string(),
            config.api_key.clone(),
        )]))
        .build()?;

    // Build resource attributes
    let mut resource_attrs = vec![
        KeyValue::new(SERVICE_NAME, config.service_name.clone()),
        KeyValue::new(SERVICE_VERSION, config.service_version.clone()),
    ];

    // Add LangSmith-specific resource attributes
    resource_attrs.push(KeyValue::new("langsmith.project", config.project.clone()));

    let resource = Resource::new(resource_attrs);

    // Configure sampler based on sampling ratio
    let sampler = if (config.sampling_ratio - 1.0).abs() < f64::EPSILON {
        Sampler::AlwaysOn
    } else if config.sampling_ratio <= 0.0 {
        Sampler::AlwaysOff
    } else {
        Sampler::TraceIdRatioBased(config.sampling_ratio)
    };

    // Build the tracer provider with batch processing
    let provider = sdktrace::TracerProvider::builder()
        .with_batch_exporter(exporter, opentelemetry_sdk::runtime::Tokio)
        .with_sampler(sampler)
        .with_resource(resource)
        .build();

    // Set as global tracer provider
    opentelemetry::global::set_tracer_provider(provider.clone());

    Ok(provider)
}

/// Helper macro for creating spans with GenAI semantic conventions.
///
/// Usage:
/// ```ignore
/// gen_ai_span!("chat_completion", model = "claude-3", provider = "anthropic");
/// ```
#[macro_export]
macro_rules! gen_ai_span {
    ($operation:expr $(, $key:ident = $value:expr)*) => {
        tracing::info_span!(
            $operation,
            "gen_ai.operation.name" = $operation,
            $($key = $value,)*
        )
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_langsmith_config_default() {
        let config = LangSmithConfig::default();
        assert_eq!(config.project, "default");
        assert_eq!(config.endpoint, "https://api.smith.langchain.com");
        assert_eq!(config.service_name, "qbit");
        assert!((config.sampling_ratio - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_traces_endpoint() {
        let config = LangSmithConfig {
            endpoint: "https://api.smith.langchain.com".to_string(),
            ..Default::default()
        };
        assert_eq!(
            config.traces_endpoint(),
            "https://api.smith.langchain.com/otel/v1/traces"
        );

        // Test with trailing slash
        let config2 = LangSmithConfig {
            endpoint: "https://api.smith.langchain.com/".to_string(),
            ..Default::default()
        };
        assert_eq!(
            config2.traces_endpoint(),
            "https://api.smith.langchain.com/otel/v1/traces"
        );
    }

    #[test]
    fn test_from_env_missing_key() {
        // Ensure the env var is not set
        std::env::remove_var("LANGSMITH_API_KEY");
        assert!(LangSmithConfig::from_env().is_none());
    }
}
