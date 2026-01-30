//! JSON repair utilities for LLM tool call arguments
//!
//! LLMs (especially GLM models) sometimes produce malformed JSON that fails
//! to parse. This module provides repair functionality using the llm_json crate.

use serde_json::Value;
use tracing::debug;

/// Parse tool call arguments with automatic repair for malformed JSON.
///
/// Attempts standard parsing first, then falls back to repair if that fails.
/// Returns empty object `{}` if both parsing and repair fail.
pub fn parse_tool_args(args: &str) -> Value {
    // Fast path: try standard parsing first
    if let Ok(value) = serde_json::from_str(args) {
        return value;
    }

    // Slow path: attempt repair
    debug!("JSON parse failed, attempting repair");
    repair_and_parse(args).unwrap_or_else(|| {
        debug!("JSON repair failed, returning empty object");
        serde_json::json!({})
    })
}

/// Parse tool call arguments, returning None on failure instead of default.
///
/// Useful when you need to handle parse failures explicitly.
pub fn parse_tool_args_opt(args: &str) -> Option<Value> {
    // Fast path: try standard parsing first
    if let Ok(value) = serde_json::from_str(args) {
        return Some(value);
    }

    // Slow path: attempt repair
    repair_and_parse(args)
}

/// Repair malformed JSON string and return the fixed string.
///
/// Returns None if repair fails.
pub fn repair_json(args: &str) -> Option<String> {
    llm_json::repair_json(args, &Default::default()).ok()
}

/// Repair and parse JSON in one step.
fn repair_and_parse(args: &str) -> Option<Value> {
    match llm_json::loads(args, &Default::default()) {
        Ok(value) => {
            debug!("JSON repair succeeded");
            Some(value)
        }
        Err(e) => {
            debug!("JSON repair failed: {}", e);
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_json_passthrough() {
        let json = r#"{"name": "test", "value": 123}"#;
        let result = parse_tool_args(json);
        assert_eq!(result["name"], "test");
        assert_eq!(result["value"], 123);
    }

    #[test]
    fn test_unquoted_keys() {
        let json = r#"{name: "test", value: 123}"#;
        let result = parse_tool_args(json);
        assert_eq!(result["name"], "test");
    }

    #[test]
    fn test_single_quotes() {
        let json = r#"{'name': 'test'}"#;
        let result = parse_tool_args(json);
        assert_eq!(result["name"], "test");
    }

    #[test]
    fn test_trailing_comma() {
        let json = r#"{"name": "test",}"#;
        let result = parse_tool_args(json);
        assert_eq!(result["name"], "test");
    }

    #[test]
    fn test_python_booleans() {
        let json = r#"{"active": True, "disabled": False}"#;
        let result = parse_tool_args(json);
        assert_eq!(result["active"], true);
        assert_eq!(result["disabled"], false);
    }

    #[test]
    fn test_unclosed_object() {
        let json = r#"{"name": "test""#;
        let result = parse_tool_args(json);
        // Should repair by closing the object
        assert_eq!(result["name"], "test");
    }

    #[test]
    fn test_missing_value_quotes() {
        // This pattern was seen in GLM output
        let json = r#"{"explanation":Explore notification-related code}"#;
        let result = parse_tool_args(json);
        // Should repair the unquoted string value
        assert!(!result.is_null());
    }

    #[test]
    fn test_invalid_returns_something() {
        // Note: llm_json is very aggressive at repair and may produce
        // unexpected results from truly malformed input. The important
        // thing is that it doesn't panic.
        let json = "not json at all {{{";
        let result = parse_tool_args(json);
        // Result may be repaired unexpectedly; just verify we get a value
        assert!(result.is_object() || result.is_null());
    }
}
