//! Parser for pseudo-XML tool calls embedded in text content.
//!
//! GLM models sometimes output tool calls in a pseudo-XML format within their text/reasoning
//! content instead of using the structured tool calling API. This module parses those patterns.
//!
//! Example format:
//! ```text
//! <tool_call>read_file<arg_key>path</arg_key><arg_value>file.txt</arg_value></tool_call>
//! ```

use regex::Regex;
use serde_json::{json, Value};
use std::sync::LazyLock;
use tracing::debug;

/// Regex pattern to match the pseudo-XML tool call format
static TOOL_CALL_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"<tool_call>([^<]+)((?:<arg_key>[^<]*</arg_key><arg_value>[^<]*</arg_value>)*)</tool_call>")
        .expect("Invalid tool call regex")
});

/// Regex pattern to extract arg_key/arg_value pairs
static ARG_PAIR_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"<arg_key>([^<]*)</arg_key><arg_value>([^<]*)</arg_value>")
        .expect("Invalid arg pair regex")
});

/// A parsed tool call extracted from text content.
#[derive(Debug, Clone)]
pub struct ParsedToolCall {
    /// The function name
    pub name: String,
    /// The arguments as a JSON object
    pub arguments: Value,
}

/// Parse pseudo-XML tool calls from text content.
///
/// Returns a list of parsed tool calls and the remaining text with tool calls removed.
pub fn parse_tool_calls_from_text(text: &str) -> (Vec<ParsedToolCall>, String) {
    let mut tool_calls = Vec::new();
    let mut remaining_text = text.to_string();

    for cap in TOOL_CALL_REGEX.captures_iter(text) {
        let full_match = cap.get(0).unwrap().as_str();
        let name = cap.get(1).unwrap().as_str().trim().to_string();
        let args_str = cap.get(2).map(|m| m.as_str()).unwrap_or("");

        // Parse argument pairs
        let mut args = serde_json::Map::new();
        for arg_cap in ARG_PAIR_REGEX.captures_iter(args_str) {
            let key = arg_cap.get(1).unwrap().as_str().to_string();
            let value_str = arg_cap.get(2).unwrap().as_str();

            // Try to parse as number, otherwise keep as string
            let value: Value = if let Ok(n) = value_str.parse::<i64>() {
                json!(n)
            } else if let Ok(n) = value_str.parse::<f64>() {
                json!(n)
            } else if value_str == "true" {
                json!(true)
            } else if value_str == "false" {
                json!(false)
            } else if value_str == "null" {
                Value::Null
            } else {
                json!(value_str)
            };

            args.insert(key, value);
        }

        debug!(
            "Parsed pseudo-XML tool call: {} with {} arguments",
            name,
            args.len()
        );

        tool_calls.push(ParsedToolCall {
            name,
            arguments: Value::Object(args),
        });

        // Remove the tool call from the text
        remaining_text = remaining_text.replace(full_match, "");
    }

    // Clean up the remaining text (trim and collapse multiple spaces/newlines)
    let remaining_text = remaining_text
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>()
        .join("\n");

    (tool_calls, remaining_text)
}

/// Check if text contains any pseudo-XML tool calls.
pub fn contains_pseudo_xml_tool_calls(text: &str) -> bool {
    TOOL_CALL_REGEX.is_match(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_single_tool_call() {
        let text = "Let me read the file:<tool_call>read_file<arg_key>path</arg_key><arg_value>src/main.rs</arg_value></tool_call>";
        let (tool_calls, remaining) = parse_tool_calls_from_text(text);

        assert_eq!(tool_calls.len(), 1);
        assert_eq!(tool_calls[0].name, "read_file");
        assert_eq!(tool_calls[0].arguments["path"], "src/main.rs");
        assert_eq!(remaining, "Let me read the file:");
    }

    #[test]
    fn test_parse_tool_call_with_multiple_args() {
        let text = "<tool_call>read_file<arg_key>path</arg_key><arg_value>file.txt</arg_value><arg_key>line_start</arg_key><arg_value>1</arg_value><arg_key>line_end</arg_key><arg_value>100</arg_value></tool_call>";
        let (tool_calls, _) = parse_tool_calls_from_text(text);

        assert_eq!(tool_calls.len(), 1);
        assert_eq!(tool_calls[0].name, "read_file");
        assert_eq!(tool_calls[0].arguments["path"], "file.txt");
        assert_eq!(tool_calls[0].arguments["line_start"], 1);
        assert_eq!(tool_calls[0].arguments["line_end"], 100);
    }

    #[test]
    fn test_parse_multiple_tool_calls() {
        let text = "First<tool_call>func1<arg_key>a</arg_key><arg_value>1</arg_value></tool_call>Middle<tool_call>func2<arg_key>b</arg_key><arg_value>2</arg_value></tool_call>Last";
        let (tool_calls, remaining) = parse_tool_calls_from_text(text);

        assert_eq!(tool_calls.len(), 2);
        assert_eq!(tool_calls[0].name, "func1");
        assert_eq!(tool_calls[1].name, "func2");
        // The remaining text is trimmed and filtered for empty lines, so it becomes a single line
        assert_eq!(remaining, "FirstMiddleLast");
    }

    #[test]
    fn test_no_tool_calls() {
        let text = "Just regular text without any tool calls";
        let (tool_calls, remaining) = parse_tool_calls_from_text(text);

        assert!(tool_calls.is_empty());
        assert_eq!(remaining, text);
    }

    #[test]
    fn test_contains_check() {
        assert!(contains_pseudo_xml_tool_calls(
            "<tool_call>test<arg_key>a</arg_key><arg_value>b</arg_value></tool_call>"
        ));
        assert!(!contains_pseudo_xml_tool_calls("no tool calls here"));
    }

    #[test]
    fn test_boolean_and_null_values() {
        let text = "<tool_call>test<arg_key>enabled</arg_key><arg_value>true</arg_value><arg_key>disabled</arg_key><arg_value>false</arg_value><arg_key>empty</arg_key><arg_value>null</arg_value></tool_call>";
        let (tool_calls, _) = parse_tool_calls_from_text(text);

        assert_eq!(tool_calls[0].arguments["enabled"], true);
        assert_eq!(tool_calls[0].arguments["disabled"], false);
        assert!(tool_calls[0].arguments["empty"].is_null());
    }
}
