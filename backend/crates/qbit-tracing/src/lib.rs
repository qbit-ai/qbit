//! OpenTelemetry/LangSmith tracing infrastructure for Qbit.
//!
//! This crate provides utilities for instrumenting Qbit with OpenTelemetry
//! traces that are compatible with LangSmith's GenAI observability platform.
//!
//! # Features
//!
//! - **GenAI Semantic Conventions**: Constants for standard OTel GenAI attributes
//! - **LangSmith Attributes**: Constants for LangSmith-specific span attributes
//! - **Helper Functions**: Utilities for truncating large payloads, formatting tool args
//! - **Span Builders**: Macros and helpers for creating properly configured spans
//! - **Types**: Common types like `StreamCompletionResult` for instrumented functions
//!
//! # Usage
//!
//! ```rust,ignore
//! use qbit_tracing::prelude::*;
//!
//! #[tracing::instrument(
//!     target = gen_ai::TARGET_LLM,
//!     name = "chat",
//!     skip_all,
//!     fields(
//!         langsmith::SPAN_KIND = langsmith::KIND_LLM,
//!         gen_ai::OPERATION_NAME = "chat",
//!         gen_ai::PROVIDER_NAME = %provider,
//!         gen_ai::REQUEST_MODEL = %model,
//!         gen_ai::USAGE_INPUT_TOKENS = tracing::field::Empty,
//!         gen_ai::USAGE_OUTPUT_TOKENS = tracing::field::Empty,
//!     ),
//!     err,
//! )]
//! async fn stream_llm_completion(...) -> Result<StreamCompletionResult> {
//!     // ... streaming logic ...
//!
//!     // Record token usage after streaming
//!     let span = tracing::Span::current();
//!     span.record(gen_ai::USAGE_INPUT_TOKENS, usage.input_tokens as i64);
//!     span.record(gen_ai::USAGE_OUTPUT_TOKENS, usage.output_tokens as i64);
//!
//!     Ok(result)
//! }
//! ```
//!
//! # References
//!
//! - [OpenTelemetry GenAI Semantic Conventions](https://opentelemetry.io/docs/specs/semconv/gen-ai/gen-ai-spans/)
//! - [LangSmith OpenTelemetry Integration](https://docs.langchain.com/langsmith/trace-with-opentelemetry)
//! - [tracing-opentelemetry crate](https://docs.rs/tracing-opentelemetry)

pub mod attributes;
pub mod helpers;
pub mod types;

/// Prelude module for convenient imports.
///
/// ```rust,ignore
/// use qbit_tracing::prelude::*;
/// ```
pub mod prelude {
    pub use crate::attributes::{gen_ai, langsmith, otel};
    pub use crate::helpers::{truncate_json, truncate_string};
    pub use crate::types::StreamCompletionResult;

    // Re-export commonly used tracing items
    pub use tracing::{field::Empty, instrument, Instrument, Span};
    pub use tracing_opentelemetry::OpenTelemetrySpanExt;
}

// Re-export key items at crate root
pub use attributes::{gen_ai, langsmith, otel};
pub use helpers::{truncate_json, truncate_string};
pub use types::StreamCompletionResult;
