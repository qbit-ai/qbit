//! Helper functions for tracing instrumentation.
//!
//! This module provides utilities for preparing data for trace attributes,
//! including truncation of large payloads and formatting.

use tracing::Span;

/// Default maximum size for string attributes (10KB).
pub const DEFAULT_MAX_STRING_SIZE: usize = 10_000;

/// Default maximum size for JSON attributes (10KB).
pub const DEFAULT_MAX_JSON_SIZE: usize = 10_000;

/// Truncate a string to a maximum length, appending "... (truncated)" if needed.
///
/// # Arguments
///
/// * `s` - The string to truncate
/// * `max_len` - Maximum length in bytes
///
/// # Returns
///
/// The original string if it fits, or a truncated version with suffix.
///
/// # Example
///
/// ```
/// use qbit_tracing::helpers::truncate_string;
///
/// let short = "hello";
/// assert_eq!(truncate_string(short, 100), "hello");
///
/// let long = "a".repeat(100);
/// let truncated = truncate_string(&long, 50);
/// assert!(truncated.len() <= 70); // 50 + suffix
/// assert!(truncated.ends_with("... (truncated)"));
/// ```
pub fn truncate_string(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        // Find a valid UTF-8 boundary
        let truncate_at = s
            .char_indices()
            .take_while(|(i, _)| *i < max_len)
            .last()
            .map(|(i, c)| i + c.len_utf8())
            .unwrap_or(0);

        format!("{}... (truncated)", &s[..truncate_at])
    }
}

/// Truncate a JSON value to a maximum string length.
///
/// Serializes the JSON value and truncates if necessary.
///
/// # Arguments
///
/// * `value` - The JSON value to truncate
/// * `max_len` - Maximum length in bytes for the serialized string
///
/// # Returns
///
/// A string representation of the JSON, possibly truncated.
///
/// # Example
///
/// ```
/// use qbit_tracing::helpers::truncate_json;
/// use serde_json::json;
///
/// let small = json!({"key": "value"});
/// let result = truncate_json(&small, 1000);
/// assert!(!result.contains("truncated"));
///
/// let large = json!({"data": "x".repeat(1000)});
/// let result = truncate_json(&large, 100);
/// assert!(result.contains("truncated"));
/// ```
pub fn truncate_json(value: &serde_json::Value, max_len: usize) -> String {
    match serde_json::to_string(value) {
        Ok(s) => truncate_string(&s, max_len),
        Err(_) => "<serialization error>".to_string(),
    }
}

/// Truncate JSON with pretty printing for readability.
///
/// Uses indented JSON format, then truncates. Useful for debugging.
pub fn truncate_json_pretty(value: &serde_json::Value, max_len: usize) -> String {
    match serde_json::to_string_pretty(value) {
        Ok(s) => truncate_string(&s, max_len),
        Err(_) => "<serialization error>".to_string(),
    }
}

/// Record a field on the current span, truncating if necessary.
///
/// This is a convenience function that combines getting the current span,
/// truncating the value, and recording it.
///
/// # Arguments
///
/// * `field` - The field name to record
/// * `value` - The string value to record
/// * `max_len` - Maximum length before truncation
///
/// # Example
///
/// ```rust,ignore
/// use qbit_tracing::helpers::record_truncated;
///
/// // Inside an instrumented function
/// record_truncated("gen_ai.completion", &response_text, 10_000);
/// ```
pub fn record_truncated(field: &'static str, value: &str, max_len: usize) {
    let truncated = truncate_string(value, max_len);
    Span::current().record(field, truncated.as_str());
}

/// Record a JSON field on the current span, truncating if necessary.
///
/// # Arguments
///
/// * `field` - The field name to record
/// * `value` - The JSON value to record
/// * `max_len` - Maximum length before truncation
pub fn record_json_truncated(field: &'static str, value: &serde_json::Value, max_len: usize) {
    let truncated = truncate_json(value, max_len);
    Span::current().record(field, truncated.as_str());
}

/// Record token usage on the current span.
///
/// Records both input and output tokens using GenAI semantic conventions.
///
/// # Arguments
///
/// * `input_tokens` - Number of input/prompt tokens
/// * `output_tokens` - Number of output/completion tokens
///
/// # Example
///
/// ```rust,ignore
/// use qbit_tracing::helpers::record_token_usage;
///
/// // After streaming completes
/// record_token_usage(usage.input_tokens, usage.output_tokens);
/// ```
pub fn record_token_usage(input_tokens: u64, output_tokens: u64) {
    let span = Span::current();
    span.record(crate::gen_ai::USAGE_INPUT_TOKENS, input_tokens as i64);
    span.record(crate::gen_ai::USAGE_OUTPUT_TOKENS, output_tokens as i64);
}

/// Record an error on the current span using OTel conventions.
///
/// Sets `otel.status_code` to "ERROR" and `otel.status_description` to the error message.
///
/// # Arguments
///
/// * `error` - The error to record
pub fn record_error<E: std::fmt::Display>(error: &E) {
    let span = Span::current();
    span.record(crate::otel::STATUS_CODE, crate::otel::STATUS_ERROR);
    span.record(crate::otel::STATUS_DESCRIPTION, error.to_string().as_str());
}

/// Record success status on the current span.
///
/// Sets `otel.status_code` to "OK".
pub fn record_success() {
    Span::current().record(crate::otel::STATUS_CODE, crate::otel::STATUS_OK);
}

/// Extract a prompt preview from the last user message.
///
/// Searches for the last user message containing text and returns it,
/// truncated to the specified maximum length.
///
/// # Arguments
///
/// * `messages` - Iterator of (role, content) tuples
/// * `max_len` - Maximum length for the preview
///
/// # Returns
///
/// The last user message text, truncated if necessary.
pub fn extract_prompt_preview<'a, I>(messages: I, max_len: usize) -> String
where
    I: Iterator<Item = (&'a str, &'a str)>,
{
    let last_user_message = messages
        .filter(|(role, _)| *role == "user")
        .last()
        .map(|(_, content)| content)
        .unwrap_or("");

    truncate_string(last_user_message, max_len)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_truncate_string_short() {
        let s = "hello world";
        assert_eq!(truncate_string(s, 100), "hello world");
    }

    #[test]
    fn test_truncate_string_exact() {
        let s = "hello";
        assert_eq!(truncate_string(s, 5), "hello");
    }

    #[test]
    fn test_truncate_string_long() {
        let s = "a".repeat(100);
        let result = truncate_string(&s, 50);
        assert!(result.starts_with("aaaa"));
        assert!(result.ends_with("... (truncated)"));
        // Should be ~50 + "... (truncated)" length
        assert!(result.len() < 70);
    }

    #[test]
    fn test_truncate_string_utf8() {
        // Test with multi-byte UTF-8 characters
        let s = "日本語テスト"; // Japanese characters
        let result = truncate_string(s, 10);
        // Should not panic and should be valid UTF-8
        assert!(result.is_ascii() || result.chars().count() > 0);
    }

    #[test]
    fn test_truncate_json_small() {
        let value = json!({"key": "value"});
        let result = truncate_json(&value, 1000);
        assert!(!result.contains("truncated"));
    }

    #[test]
    fn test_truncate_json_large() {
        let value = json!({"data": "x".repeat(1000)});
        let result = truncate_json(&value, 100);
        assert!(result.contains("truncated"));
    }

    #[test]
    fn test_extract_prompt_preview() {
        let messages = vec![
            ("system", "You are a helpful assistant."),
            ("user", "Hello!"),
            ("assistant", "Hi there!"),
            ("user", "How are you?"),
        ];

        let preview = extract_prompt_preview(messages.iter().map(|(r, c)| (*r, *c)), 100);
        assert_eq!(preview, "How are you?");
    }

    #[test]
    fn test_extract_prompt_preview_empty() {
        let messages: Vec<(&str, &str)> = vec![];
        let preview = extract_prompt_preview(messages.iter().map(|(r, c)| (*r, *c)), 100);
        assert_eq!(preview, "");
    }
}
