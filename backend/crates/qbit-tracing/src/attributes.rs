//! OpenTelemetry and LangSmith attribute constants.
//!
//! This module provides constants for standardized attribute names used in
//! OpenTelemetry GenAI semantic conventions and LangSmith-specific attributes.
//!
//! # References
//!
//! - [OpenTelemetry GenAI Semantic Conventions](https://opentelemetry.io/docs/specs/semconv/gen-ai/gen-ai-spans/)
//! - [LangSmith OpenTelemetry Integration](https://docs.langchain.com/langsmith/trace-with-opentelemetry)

/// OpenTelemetry GenAI semantic convention attributes.
///
/// These attributes follow the OpenTelemetry specification for generative AI
/// operations. They are recognized by observability platforms like LangSmith,
/// Datadog, and others.
///
/// # Example
///
/// ```rust,ignore
/// use qbit_tracing::gen_ai;
///
/// #[tracing::instrument(fields(
///     gen_ai::OPERATION_NAME = "chat",
///     gen_ai::REQUEST_MODEL = %model,
/// ))]
/// async fn my_llm_call() { }
/// ```
pub mod gen_ai {
    // =========================================================================
    // Tracing Targets
    // =========================================================================

    /// Target for LLM completion spans.
    pub const TARGET_LLM: &str = "qbit::llm";

    /// Target for tool execution spans.
    pub const TARGET_TOOLS: &str = "qbit::tools";

    /// Target for agent-level spans (sessions, turns).
    pub const TARGET_AGENT: &str = "qbit::agent";

    // =========================================================================
    // Required Attributes
    // =========================================================================

    /// The name of the operation being performed.
    ///
    /// Well-known values: `chat`, `text_completion`, `embeddings`, `execute_tool`,
    /// `create_agent`, `invoke_agent`.
    pub const OPERATION_NAME: &str = "gen_ai.operation.name";

    /// The name of the GenAI provider.
    ///
    /// Examples: `anthropic`, `openai`, `aws.bedrock`, `gcp.vertex_ai`.
    pub const PROVIDER_NAME: &str = "gen_ai.provider.name";

    // =========================================================================
    // Request Attributes
    // =========================================================================

    /// The name of the model requested.
    pub const REQUEST_MODEL: &str = "gen_ai.request.model";

    /// The temperature setting for the request.
    pub const REQUEST_TEMPERATURE: &str = "gen_ai.request.temperature";

    /// The top_p (nucleus sampling) setting.
    pub const REQUEST_TOP_P: &str = "gen_ai.request.top_p";

    /// The top_k sampling setting.
    pub const REQUEST_TOP_K: &str = "gen_ai.request.top_k";

    /// Maximum number of tokens to generate.
    pub const REQUEST_MAX_TOKENS: &str = "gen_ai.request.max_tokens";

    /// Frequency penalty setting.
    pub const REQUEST_FREQUENCY_PENALTY: &str = "gen_ai.request.frequency_penalty";

    /// Presence penalty setting.
    pub const REQUEST_PRESENCE_PENALTY: &str = "gen_ai.request.presence_penalty";

    /// Stop sequences for generation.
    pub const REQUEST_STOP_SEQUENCES: &str = "gen_ai.request.stop_sequences";

    /// Seed for reproducible results.
    pub const REQUEST_SEED: &str = "gen_ai.request.seed";

    // =========================================================================
    // Response Attributes
    // =========================================================================

    /// The actual model name used in the response.
    pub const RESPONSE_MODEL: &str = "gen_ai.response.model";

    /// Unique identifier for the completion.
    pub const RESPONSE_ID: &str = "gen_ai.response.id";

    /// Reasons why the model stopped generating.
    ///
    /// Examples: `stop`, `end_turn`, `max_tokens`, `tool_use`.
    pub const RESPONSE_FINISH_REASONS: &str = "gen_ai.response.finish_reasons";

    // =========================================================================
    // Usage Attributes
    // =========================================================================

    /// Number of tokens in the prompt/input.
    pub const USAGE_INPUT_TOKENS: &str = "gen_ai.usage.input_tokens";

    /// Number of tokens in the response/output.
    pub const USAGE_OUTPUT_TOKENS: &str = "gen_ai.usage.output_tokens";

    // =========================================================================
    // Tool Execution Attributes
    // =========================================================================

    /// The name of the tool being executed.
    pub const TOOL_NAME: &str = "gen_ai.tool.name";

    /// Unique identifier for this tool call.
    pub const TOOL_CALL_ID: &str = "gen_ai.tool.call.id";

    /// The type of tool (`function`, `extension`, `datastore`).
    pub const TOOL_TYPE: &str = "gen_ai.tool.type";

    /// Description of the tool's purpose.
    pub const TOOL_DESCRIPTION: &str = "gen_ai.tool.description";

    // =========================================================================
    // Opt-In Content Attributes (Sensitive)
    // =========================================================================

    /// The input messages/prompt (may contain sensitive data).
    pub const INPUT_MESSAGES: &str = "gen_ai.input.messages";

    /// The output messages/completion (may contain sensitive data).
    pub const OUTPUT_MESSAGES: &str = "gen_ai.output.messages";

    /// System instructions/prompt (may contain sensitive data).
    pub const SYSTEM_INSTRUCTIONS: &str = "gen_ai.system_instructions";

    /// Tool call arguments (may contain sensitive data).
    pub const TOOL_CALL_ARGUMENTS: &str = "gen_ai.tool.call.arguments";

    /// Tool call result (may contain sensitive data).
    pub const TOOL_CALL_RESULT: &str = "gen_ai.tool.call.result";

    // =========================================================================
    // Operation Name Values
    // =========================================================================

    /// Operation name for chat completions.
    pub const OP_CHAT: &str = "chat";

    /// Operation name for text completions.
    pub const OP_TEXT_COMPLETION: &str = "text_completion";

    /// Operation name for embeddings.
    pub const OP_EMBEDDINGS: &str = "embeddings";

    /// Operation name for tool execution.
    pub const OP_EXECUTE_TOOL: &str = "execute_tool";

    /// Operation name for agent creation.
    pub const OP_CREATE_AGENT: &str = "create_agent";

    /// Operation name for agent invocation.
    pub const OP_INVOKE_AGENT: &str = "invoke_agent";

    // =========================================================================
    // Provider Name Values
    // =========================================================================

    /// Provider name for Anthropic.
    pub const PROVIDER_ANTHROPIC: &str = "anthropic";

    /// Provider name for Anthropic on Vertex AI.
    pub const PROVIDER_ANTHROPIC_VERTEX: &str = "anthropic_vertex";

    /// Provider name for OpenAI.
    pub const PROVIDER_OPENAI: &str = "openai";

    /// Provider name for Google Vertex AI.
    pub const PROVIDER_VERTEX_AI: &str = "gcp.vertex_ai";

    /// Provider name for AWS Bedrock.
    pub const PROVIDER_BEDROCK: &str = "aws.bedrock";

    // =========================================================================
    // Finish Reason Values
    // =========================================================================

    /// Model stopped at a natural end.
    pub const FINISH_STOP: &str = "stop";

    /// Model finished its turn.
    pub const FINISH_END_TURN: &str = "end_turn";

    /// Model hit max tokens limit.
    pub const FINISH_MAX_TOKENS: &str = "max_tokens";

    /// Model requested tool use.
    pub const FINISH_TOOL_USE: &str = "tool_use";
}

/// LangSmith-specific attributes.
///
/// These attributes are recognized by LangSmith for enhanced visualization
/// and organization of traces.
///
/// # Example
///
/// ```rust,ignore
/// use qbit_tracing::langsmith;
///
/// #[tracing::instrument(fields(
///     langsmith::SPAN_KIND = langsmith::KIND_LLM,
///     langsmith::SESSION_ID = %session_id,
/// ))]
/// async fn my_traced_function() { }
/// ```
pub mod langsmith {
    // =========================================================================
    // Span Classification
    // =========================================================================

    /// The type of run/span in LangSmith UI.
    ///
    /// Values: `llm`, `tool`, `chain`, `retriever`, `embedding`, `prompt`, `parser`.
    pub const SPAN_KIND: &str = "langsmith.span.kind";

    /// Override the trace/run name.
    pub const TRACE_NAME: &str = "langsmith.trace.name";

    /// Custom tags (comma-separated).
    pub const SPAN_TAGS: &str = "langsmith.span.tags";

    // =========================================================================
    // Session/Thread Grouping
    // =========================================================================

    /// Session identifier for grouping related traces.
    ///
    /// Use this to group multiple agent runs into a single conversation thread.
    pub const SESSION_ID: &str = "langsmith.trace.session_id";

    /// Human-readable session name.
    pub const SESSION_NAME: &str = "langsmith.trace.session_name";

    // =========================================================================
    // Custom Metadata
    // =========================================================================

    /// Prefix for custom metadata fields.
    ///
    /// Fields like `langsmith.metadata.user_id` map to `metadata.user_id` in LangSmith.
    pub const METADATA_PREFIX: &str = "langsmith.metadata.";

    // =========================================================================
    // Span Kind Values
    // =========================================================================

    /// Span kind for LLM calls.
    pub const KIND_LLM: &str = "llm";

    /// Span kind for tool executions.
    pub const KIND_TOOL: &str = "tool";

    /// Span kind for chains/workflows.
    pub const KIND_CHAIN: &str = "chain";

    /// Span kind for retrievers.
    pub const KIND_RETRIEVER: &str = "retriever";

    /// Span kind for embeddings.
    pub const KIND_EMBEDDING: &str = "embedding";

    /// Span kind for prompts.
    pub const KIND_PROMPT: &str = "prompt";

    /// Span kind for parsers.
    pub const KIND_PARSER: &str = "parser";
}

/// OpenTelemetry special attributes for tracing-opentelemetry.
///
/// These attributes are recognized by the `tracing-opentelemetry` crate
/// and have special behavior.
///
/// # Example
///
/// ```rust,ignore
/// use qbit_tracing::otel;
///
/// #[tracing::instrument(fields(
///     otel::NAME = %dynamic_name,
///     otel::KIND = otel::KIND_CLIENT,
/// ))]
/// async fn my_function() { }
/// ```
pub mod otel {
    // =========================================================================
    // Special Fields
    // =========================================================================

    /// Override the span name sent to OpenTelemetry exporters.
    ///
    /// Useful for displaying dynamic information in span names.
    pub const NAME: &str = "otel.name";

    /// Set the span kind.
    ///
    /// Values: `"client"`, `"server"`, `"producer"`, `"consumer"`, `"internal"`.
    pub const KIND: &str = "otel.kind";

    /// Set the span status code.
    ///
    /// Values: `"OK"`, `"ERROR"`.
    pub const STATUS_CODE: &str = "otel.status_code";

    /// Status description (meaningful with status_code).
    pub const STATUS_DESCRIPTION: &str = "otel.status_description";

    // =========================================================================
    // Span Kind Values
    // =========================================================================

    /// Client span kind (outgoing request).
    pub const KIND_CLIENT: &str = "client";

    /// Server span kind (incoming request).
    pub const KIND_SERVER: &str = "server";

    /// Producer span kind (message producer).
    pub const KIND_PRODUCER: &str = "producer";

    /// Consumer span kind (message consumer).
    pub const KIND_CONSUMER: &str = "consumer";

    /// Internal span kind (default).
    pub const KIND_INTERNAL: &str = "internal";

    // =========================================================================
    // Status Code Values
    // =========================================================================

    /// OK status.
    pub const STATUS_OK: &str = "OK";

    /// Error status.
    pub const STATUS_ERROR: &str = "ERROR";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gen_ai_constants() {
        assert_eq!(gen_ai::OPERATION_NAME, "gen_ai.operation.name");
        assert_eq!(gen_ai::USAGE_INPUT_TOKENS, "gen_ai.usage.input_tokens");
        assert_eq!(gen_ai::TOOL_NAME, "gen_ai.tool.name");
    }

    #[test]
    fn test_langsmith_constants() {
        assert_eq!(langsmith::SPAN_KIND, "langsmith.span.kind");
        assert_eq!(langsmith::SESSION_ID, "langsmith.trace.session_id");
        assert_eq!(langsmith::KIND_LLM, "llm");
    }

    #[test]
    fn test_otel_constants() {
        assert_eq!(otel::NAME, "otel.name");
        assert_eq!(otel::KIND, "otel.kind");
        assert_eq!(otel::KIND_CLIENT, "client");
    }
}
