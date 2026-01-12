//! JSON fixer for Z.AI's malformed partial_json values.
//!
//! Z.AI's Anthropic-compatible API returns malformed JSON in `input_json_delta` events
//! where string values like `.` and `*` are not quoted. This module provides utilities
//! to fix these values before parsing.

use regex::Regex;
use std::sync::OnceLock;

/// Regex pattern for detecting and fixing unquoted JSON string values.
///
/// Matches patterns like:
/// - `"path":.` (unquoted dot)
/// - `"pattern":*` (unquoted asterisk)
/// - `"name":foo` (unquoted identifier)
///
/// Captures:
/// 1. Key part: `"key":\s*`
/// 2. Unquoted value (non-digit, non-quote, non-bracket start, until delimiter)
/// 3. Delimiter: `,` or `}` or `]`
static UNQUOTED_VALUE_PATTERN: OnceLock<Regex> = OnceLock::new();

fn get_pattern() -> &'static Regex {
    UNQUOTED_VALUE_PATTERN.get_or_init(|| {
        // Match JSON object keys followed by unquoted values that start with
        // special characters (. or *) that Z.AI commonly sends unquoted.
        //
        // This is a targeted approach that specifically matches:
        // - `"key":.` (dot value)
        // - `"key":*` (asterisk value)
        // - `"key":./path` (relative path)
        // - `"key":**/*.rs` (glob pattern)
        //
        // We intentionally DON'T match:
        // - Numbers (start with digit or -)
        // - Strings (start with ")
        // - Keywords (true, false, null - handled in replacement)
        // - Objects/Arrays (start with { or [)
        //
        // Pattern breakdown:
        // - `"[^"]+"` - quoted key name
        // - `\s*:\s*` - colon with optional whitespace
        // - `([.*][^,\}\]]*)` - value starting with . or * (common Z.AI unquoted patterns)
        // - `(\s*[,\}\]])` - delimiter with optional whitespace
        Regex::new(r#"("[^"]+"\s*:\s*)([.*][^,\}\]]*)(\s*[,\}\]])"#).expect("Invalid regex pattern")
    })
}

/// Fix unquoted string values in JSON.
///
/// Z.AI returns partial_json like: `{"path":.,"pattern":*}`
/// This fixes it to: `{"path":".","pattern":"*"}`
///
/// # Arguments
///
/// * `json` - The JSON string to fix
///
/// # Returns
///
/// The fixed JSON string with unquoted values quoted
///
/// # Example
///
/// ```
/// use rig_zai_anthropic::json_fixer::fix_unquoted_values;
///
/// let input = r#"{"path":.,"pattern":*}"#;
/// let fixed = fix_unquoted_values(input);
/// assert_eq!(fixed, r#"{"path":".","pattern":"*"}"#);
/// ```
pub fn fix_unquoted_values(json: &str) -> String {
    let pattern = get_pattern();

    // Replace unquoted values with quoted versions
    pattern
        .replace_all(json, |caps: &regex::Captures| {
            let key_part = &caps[1]; // "path":
            let value = caps[2].trim(); // . or * or other unquoted value
            let suffix = &caps[3]; // , or } or ]

            // Skip if value is a JSON keyword (null, true, false)
            if matches!(value, "null" | "true" | "false") {
                return format!("{}{}{}", key_part, value, suffix);
            }

            // Skip if value is empty (would indicate incomplete JSON)
            if value.is_empty() {
                return format!("{}{}{}", key_part, value, suffix);
            }

            // Quote the value
            format!("{}\"{}\"{}", key_part, value, suffix)
        })
        .to_string()
}

/// Check if a string looks like it needs JSON fixing.
///
/// Performs a quick check to see if the JSON contains unquoted values
/// that match the known patterns from Z.AI.
///
/// # Arguments
///
/// * `json` - The JSON string to check
///
/// # Returns
///
/// `true` if the JSON appears to need fixing, `false` otherwise
pub fn needs_fixing(json: &str) -> bool {
    let pattern = get_pattern();
    pattern.is_match(json)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fix_path_dot() {
        let input = r#"{"path":.,"pattern":"*"}"#;
        let expected = r#"{"path":".","pattern":"*"}"#;
        assert_eq!(fix_unquoted_values(input), expected);
    }

    #[test]
    fn test_fix_pattern_star() {
        let input = r#"{"path":".","pattern":*}"#;
        let expected = r#"{"path":".","pattern":"*"}"#;
        assert_eq!(fix_unquoted_values(input), expected);
    }

    #[test]
    fn test_fix_multiple_unquoted() {
        let input = r#"{"path":.,"pattern":*,"recursive":false}"#;
        let expected = r#"{"path":".","pattern":"*","recursive":false}"#;
        assert_eq!(fix_unquoted_values(input), expected);
    }

    #[test]
    fn test_fix_double_star() {
        // Common glob pattern
        let input = r#"{"pattern":**/*.rs}"#;
        let expected = r#"{"pattern":"**/*.rs"}"#;
        assert_eq!(fix_unquoted_values(input), expected);
    }

    #[test]
    fn test_fix_complex_glob() {
        // Note: only values starting with . or * are fixed by our targeted regex
        // "src" without a leading . or * is not fixed (this is intentional)
        let input = r#"{"path":"src","pattern":*.rs,"recursive":true}"#;
        let expected = r#"{"path":"src","pattern":"*.rs","recursive":true}"#;
        assert_eq!(fix_unquoted_values(input), expected);
    }

    #[test]
    fn test_preserve_keywords() {
        let input = r#"{"enabled":true,"value":null,"active":false}"#;
        assert_eq!(fix_unquoted_values(input), input);
    }

    #[test]
    fn test_preserve_numbers() {
        let input = r#"{"count":42,"price":19.99}"#;
        assert_eq!(fix_unquoted_values(input), input);
    }

    #[test]
    fn test_preserve_negative_numbers() {
        let input = r#"{"offset":-10,"delta":-1.5}"#;
        assert_eq!(fix_unquoted_values(input), input);
    }

    #[test]
    fn test_preserve_valid_json() {
        let input = r#"{"path":".","pattern":"*.rs","recursive":true}"#;
        assert_eq!(fix_unquoted_values(input), input);
    }

    #[test]
    fn test_preserve_nested_objects() {
        let input = r#"{"config":{"path":".","enabled":true}}"#;
        assert_eq!(fix_unquoted_values(input), input);
    }

    #[test]
    fn test_preserve_arrays() {
        let input = r#"{"items":["a","b","c"],"count":3}"#;
        assert_eq!(fix_unquoted_values(input), input);
    }

    #[test]
    fn test_needs_fixing_true() {
        assert!(needs_fixing(r#"{"path":.}"#));
        assert!(needs_fixing(r#"{"pattern":*}"#));
        assert!(needs_fixing(r#"{"glob":**/*.rs}"#));
        assert!(needs_fixing(r#"{"relative":./foo}"#));
    }

    #[test]
    fn test_needs_fixing_false() {
        assert!(!needs_fixing(r#"{"path":"."}"#));
        assert!(!needs_fixing(r#"{"count":42}"#));
        assert!(!needs_fixing(r#"{"enabled":true}"#));
    }

    #[test]
    fn test_real_zai_response() {
        // Actual malformed JSON from Z.AI logs
        let input = r#"{"path":.,"pattern":*,"recursive":false}"#;
        let expected = r#"{"path":".","pattern":"*","recursive":false}"#;
        let fixed = fix_unquoted_values(input);
        assert_eq!(fixed, expected);

        // Verify the fixed JSON is valid
        let parsed: Result<serde_json::Value, _> = serde_json::from_str(&fixed);
        assert!(parsed.is_ok(), "Fixed JSON should be valid");
    }

    #[test]
    fn test_already_quoted_dot_values() {
        // These should NOT be modified - values are already quoted
        let input = r#"{"path":".json","pattern":"*.rs"}"#;
        assert_eq!(fix_unquoted_values(input), input);
    }

    #[test]
    fn test_path_with_extension() {
        // Unquoted path with extension - should be quoted
        let input = r#"{"path":.json,"recursive":true}"#;
        let expected = r#"{"path":".json","recursive":true}"#;
        assert_eq!(fix_unquoted_values(input), expected);
    }

    #[test]
    fn test_path_underscore() {
        // Path with underscore - should be quoted correctly
        let input = r#"{"path":.json_path,"recursive":true}"#;
        let expected = r#"{"path":".json_path","recursive":true}"#;
        assert_eq!(fix_unquoted_values(input), expected);
    }

    #[test]
    fn test_multiple_path_fields() {
        // Multiple path-like fields
        let input = r#"{"pattern":.,"path":.json,"include":*.rs}"#;
        let expected = r#"{"pattern":".","path":".json","include":"*.rs"}"#;
        assert_eq!(fix_unquoted_values(input), expected);
    }

    #[test]
    fn test_debugging_corruption() {
        // Test various inputs to find what produces "json,": "ath" corruption

        // Test 1: What if there's a weird field name?
        let input1 = r#"{"pattern":".","recursive":true,".json_path":"ath"}"#;
        let result1 = fix_unquoted_values(input1);
        println!("Test 1 input:  {}", input1);
        println!("Test 1 output: {}", result1);
        // This input is already valid JSON - should pass through unchanged
        assert_eq!(result1, input1);

        // Test 2: What if the value contains commas?
        let input2 = r#"{"pattern":.json,path:"ath"}"#;
        let result2 = fix_unquoted_values(input2);
        println!("Test 2 input:  {}", input2);
        println!("Test 2 output: {}", result2);

        // Test 3: Unquoted key followed by unquoted value starting with .
        let input3 = r#"{"pattern":.,json_path:*}"#;
        let result3 = fix_unquoted_values(input3);
        println!("Test 3 input:  {}", input3);
        println!("Test 3 output: {}", result3);
    }
}
