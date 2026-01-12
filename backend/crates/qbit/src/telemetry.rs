//! OpenTelemetry/Langfuse tracing integration for Qbit.
//!
//! This module provides OpenTelemetry tracing setup that exports traces
//! to Langfuse for observability of LLM interactions and agent behavior.
//!
//! ## Configuration
//!
//! Langfuse tracing is configured via `~/.qbit/settings.toml`:
//!
//! ```toml
//! [telemetry.langfuse]
//! enabled = true
//! public_key = "$LANGFUSE_PUBLIC_KEY"
//! secret_key = "$LANGFUSE_SECRET_KEY"
//! # host = "https://cloud.langfuse.com"  # default
//! ```
//!
//! Or via environment variables:
//! - `LANGFUSE_PUBLIC_KEY` - Your Langfuse public key
//! - `LANGFUSE_SECRET_KEY` - Your Langfuse secret key
//! - `LANGFUSE_HOST` - API host (optional, defaults to https://cloud.langfuse.com)

use opentelemetry::trace::TracerProvider as _;
use opentelemetry::KeyValue;
use opentelemetry_langfuse::ExporterBuilder;
use opentelemetry_sdk::runtime::Tokio as TokioRuntime;
use opentelemetry_sdk::trace::span_processor_with_async_runtime::BatchSpanProcessor;
use opentelemetry_sdk::trace::{Sampler, SdkTracerProvider};
use opentelemetry_sdk::Resource;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_opentelemetry::OpenTelemetryLayer;
use tracing_subscriber::fmt::format::FmtSpan;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Registry};

/// Langfuse configuration for OpenTelemetry tracing.
#[derive(Debug, Clone)]
pub struct LangfuseConfig {
    /// Langfuse public key
    pub public_key: String,
    /// Langfuse secret key
    pub secret_key: String,
    /// Langfuse host URL
    pub host: String,
    /// Service name for this application
    pub service_name: String,
    /// Service version
    pub service_version: String,
    /// Sampling ratio (0.0 to 1.0, default 1.0 = sample everything)
    pub sampling_ratio: f64,
}

impl Default for LangfuseConfig {
    fn default() -> Self {
        Self {
            public_key: String::new(),
            secret_key: String::new(),
            host: "https://cloud.langfuse.com".to_string(),
            service_name: "qbit".to_string(),
            service_version: env!("CARGO_PKG_VERSION").to_string(),
            sampling_ratio: 1.0,
        }
    }
}

impl LangfuseConfig {
    /// Create config from environment variables.
    ///
    /// Reads from:
    /// - `LANGFUSE_PUBLIC_KEY` (required)
    /// - `LANGFUSE_SECRET_KEY` (required)
    /// - `LANGFUSE_HOST` (optional, defaults to https://cloud.langfuse.com)
    pub fn from_env() -> Option<Self> {
        let public_key = std::env::var("LANGFUSE_PUBLIC_KEY").ok()?;
        let secret_key = std::env::var("LANGFUSE_SECRET_KEY").ok()?;

        if public_key.is_empty() || secret_key.is_empty() {
            return None;
        }

        Some(Self {
            public_key,
            secret_key,
            host: std::env::var("LANGFUSE_HOST")
                .unwrap_or_else(|_| "https://cloud.langfuse.com".to_string()),
            ..Default::default()
        })
    }

    /// Create config from settings.
    pub fn from_settings(settings: &crate::settings::LangfuseSettings) -> Option<Self> {
        if !settings.enabled {
            return None;
        }

        // Resolve public key from settings or environment
        let public_key = crate::settings::get_with_env_fallback(
            &settings.public_key,
            &["LANGFUSE_PUBLIC_KEY"],
            None,
        )?;

        // Resolve secret key from settings or environment
        let secret_key = crate::settings::get_with_env_fallback(
            &settings.secret_key,
            &["LANGFUSE_SECRET_KEY"],
            None,
        )?;

        if public_key.is_empty() || secret_key.is_empty() {
            return None;
        }

        Some(Self {
            public_key,
            secret_key,
            host: settings
                .host
                .clone()
                .unwrap_or_else(|| "https://cloud.langfuse.com".to_string()),
            service_name: "qbit".to_string(),
            service_version: env!("CARGO_PKG_VERSION").to_string(),
            sampling_ratio: settings.sampling_ratio.unwrap_or(1.0),
        })
    }
}

/// Result of telemetry initialization.
pub struct TelemetryGuard {
    /// Whether Langfuse tracing is active
    pub langfuse_active: bool,
    /// Guard for the file appender (keeps the background writer thread alive)
    pub file_guard: Option<WorkerGuard>,
    /// Tracer provider (kept to ensure proper shutdown/flush)
    tracer_provider: Option<SdkTracerProvider>,
}

impl Drop for TelemetryGuard {
    fn drop(&mut self) {
        // Shutdown OpenTelemetry first to flush pending spans
        if let Some(provider) = self.tracer_provider.take() {
            tracing::debug!("Flushing OpenTelemetry spans...");
            if let Err(e) = provider.shutdown() {
                eprintln!(
                    "Warning: Failed to shutdown OpenTelemetry provider: {:?}",
                    e
                );
            }
        }

        // Drop the file guard to flush any pending logs
        if self.file_guard.is_some() {
            tracing::debug!("Shutting down file logging...");
        }
        let _ = self.file_guard.take();
    }
}

/// Initialize tracing with optional Langfuse/OpenTelemetry export.
///
/// This function sets up:
/// 1. Standard `tracing_subscriber` with console output
/// 2. OpenTelemetry layer exporting to Langfuse (if configured)
///
/// # Arguments
///
/// * `langfuse_config` - Optional Langfuse configuration. If None, only console tracing is enabled.
/// * `log_level` - Log level for console output (e.g., "debug", "info", "warn")
/// * `extra_directives` - Additional tracing directives (e.g., "qbit=debug")
///
/// # Returns
///
/// A `TelemetryGuard` that should be held for the lifetime of the application.
/// When dropped, it will flush pending traces.
pub fn init_tracing(
    langfuse_config: Option<LangfuseConfig>,
    log_level: &str,
    extra_directives: &[&str],
) -> Result<TelemetryGuard, Box<dyn std::error::Error + Send + Sync>> {
    // Build the base env filter for console/file output
    // This filter is intentionally more restrictive to reduce log verbosity
    let mut filter = EnvFilter::from_default_env();

    // Add log level directive
    if let Ok(directive) = format!("qbit={}", log_level).parse() {
        filter = filter.add_directive(directive);
    }

    // Reduce verbosity of deeply nested agent spans for console/file output
    // These modules produce very verbose DEBUG logs that clutter the output
    // OpenTelemetry/Langfuse still captures everything via its own layer
    if log_level == "debug" || log_level == "trace" {
        // Limit sub-agent executor to info (it creates nested llm_completion spans)
        if let Ok(directive) = "qbit_sub_agents::executor=info".parse() {
            filter = filter.add_directive(directive);
        }
        // Limit agentic loop streaming details to info
        if let Ok(directive) = "qbit_ai::agentic_loop=info".parse() {
            filter = filter.add_directive(directive);
        }
    }

    // Add extra directives
    for directive_str in extra_directives {
        if let Ok(directive) = directive_str.parse() {
            filter = filter.add_directive(directive);
        }
    }

    // Set up file logging to ~/.qbit/backend.log
    // Using compact format with span events disabled to reduce verbosity
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
                .with_ansi(false)
                .with_span_events(FmtSpan::NONE) // Don't log span enter/exit events
                .compact();
            (Some(file_layer), Some(guard))
        }
    } else {
        (None, None)
    };

    // Create the base subscriber with fmt layer
    // Using compact format with minimal span context for cleaner console output
    // Span events are disabled to reduce noise - OpenTelemetry layer captures full spans
    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_target(true)
        .with_thread_ids(false)
        .with_file(false)
        .with_line_number(false)
        .with_span_events(FmtSpan::NONE) // Don't log span enter/exit events
        .compact();

    if let Some(config) = langfuse_config {
        // Set up OpenTelemetry with Langfuse exporter
        let tracer_provider = init_langfuse_tracer(&config)?;
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
            langfuse_host = %config.host,
            "Langfuse tracing enabled"
        );

        Ok(TelemetryGuard {
            langfuse_active: true,
            file_guard,
            tracer_provider: Some(tracer_provider),
        })
    } else {
        // No Langfuse, just use fmt layer
        Registry::default()
            .with(filter)
            .with(file_layer)
            .with(fmt_layer)
            .try_init()
            .map_err(|e| format!("Failed to initialize tracing: {}", e))?;

        Ok(TelemetryGuard {
            langfuse_active: false,
            file_guard,
            tracer_provider: None,
        })
    }
}

/// Initialize the OpenTelemetry tracer provider for Langfuse.
fn init_langfuse_tracer(
    config: &LangfuseConfig,
) -> Result<SdkTracerProvider, Box<dyn std::error::Error + Send + Sync>> {
    // Create the Langfuse exporter with direct configuration
    let exporter = ExporterBuilder::new()
        .with_host(&config.host)
        .with_basic_auth(&config.public_key, &config.secret_key)
        .build()?;

    // Build resource with service info
    let resource = Resource::builder()
        .with_service_name(config.service_name.clone())
        .with_attributes([KeyValue::new(
            "service.version",
            config.service_version.clone(),
        )])
        .build();

    // Configure sampler based on sampling ratio
    let sampler = if (config.sampling_ratio - 1.0).abs() < f64::EPSILON {
        Sampler::AlwaysOn
    } else if config.sampling_ratio <= 0.0 {
        Sampler::AlwaysOff
    } else {
        Sampler::TraceIdRatioBased(config.sampling_ratio)
    };

    // Build batch span processor with Tokio async runtime
    // This uses the experimental async runtime feature that properly handles async exporters
    let batch_processor = BatchSpanProcessor::builder(exporter, TokioRuntime).build();

    // Build the tracer provider with the batch processor
    let provider = SdkTracerProvider::builder()
        .with_span_processor(batch_processor)
        .with_sampler(sampler)
        .with_resource(resource)
        .build();

    tracing::info!(
        host = %config.host,
        public_key_prefix = %&config.public_key[..20],
        "Langfuse exporter initialized"
    );

    // Set as global tracer provider
    opentelemetry::global::set_tracer_provider(provider.clone());

    Ok(provider)
}

/// Helper macro for creating spans with GenAI semantic conventions for Langfuse.
///
/// This creates spans that Langfuse will recognize as "generation" observations
/// when they include model information.
///
/// ## Langfuse Property Mapping
///
/// | Attribute | Langfuse Mapping |
/// |-----------|------------------|
/// | `gen_ai.request.model` | Model name |
/// | `gen_ai.system` | Provider/system |
/// | `gen_ai.prompt` | Input (prompt) |
/// | `gen_ai.completion` | Output (completion) |
/// | `gen_ai.usage.prompt_tokens` | Input token count |
/// | `gen_ai.usage.completion_tokens` | Output token count |
/// | `langfuse.session.id` | Session grouping |
/// | `langfuse.observation.type` | "generation" for LLM calls |
///
/// Usage:
/// ```ignore
/// let _span = gen_ai_span!(
///     "chat_completion",
///     model = "claude-3-opus",
///     provider = "anthropic",
///     session_id = "sess_123"
/// );
/// ```
#[macro_export]
macro_rules! gen_ai_span {
    ($operation:expr, model = $model:expr, provider = $provider:expr $(, session_id = $session:expr)? $(,)?) => {
        tracing::info_span!(
            $operation,
            "gen_ai.operation.name" = $operation,
            "gen_ai.request.model" = $model,
            "gen_ai.system" = $provider,
            "langfuse.observation.type" = "generation",
            $("langfuse.session.id" = $session,)?
        )
    };
    ($operation:expr $(, $key:ident = $value:expr)*) => {
        tracing::info_span!(
            $operation,
            "gen_ai.operation.name" = $operation,
            $($key = $value,)*
        )
    };
}

/// Record LLM usage metrics on the current span.
///
/// Call this after an LLM completion to record token usage.
/// Uses GenAI semantic conventions: prompt_tokens and completion_tokens.
///
/// Usage:
/// ```ignore
/// record_llm_usage!(prompt_tokens = 100, completion_tokens = 50);
/// ```
#[macro_export]
macro_rules! record_llm_usage {
    (prompt_tokens = $input:expr, completion_tokens = $output:expr $(, total_tokens = $total:expr)?) => {
        tracing::Span::current().record("gen_ai.usage.prompt_tokens", $input);
        tracing::Span::current().record("gen_ai.usage.completion_tokens", $output);
        $(tracing::Span::current().record("gen_ai.usage.total_tokens", $total);)?
    };
}

/// Record the prompt/input for an LLM call on the current span.
///
/// Usage:
/// ```ignore
/// record_llm_input!("What is the capital of France?");
/// ```
#[macro_export]
macro_rules! record_llm_input {
    ($input:expr) => {
        tracing::Span::current().record("gen_ai.prompt", $input);
    };
}

/// Record the completion/output for an LLM call on the current span.
///
/// Usage:
/// ```ignore
/// record_llm_output!("The capital of France is Paris.");
/// ```
#[macro_export]
macro_rules! record_llm_output {
    ($output:expr) => {
        tracing::Span::current().record("gen_ai.completion", $output);
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_langfuse_config_default() {
        let config = LangfuseConfig::default();
        assert_eq!(config.host, "https://cloud.langfuse.com");
        assert_eq!(config.service_name, "qbit");
        assert!((config.sampling_ratio - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_from_env_missing_keys() {
        // Ensure the env vars are not set
        std::env::remove_var("LANGFUSE_PUBLIC_KEY");
        std::env::remove_var("LANGFUSE_SECRET_KEY");
        assert!(LangfuseConfig::from_env().is_none());
    }
}
