//! Formatting utilities for system hook output.
//!
//! Hooks output messages wrapped in XML tags for clear identification.

/// Format hook messages as `<system>` XML blocks.
///
/// Each message is wrapped in its own block with blocks separated by blank lines.
///
/// # Example
///
/// ```
/// use qbit_ai::system_hooks::format_system_hooks;
///
/// let messages = vec!["First hook".to_string(), "Second hook".to_string()];
/// let formatted = format_system_hooks(&messages);
/// assert!(formatted.contains("<system>"));
/// assert!(formatted.contains("First hook"));
/// ```
pub fn format_system_hooks(hooks: &[String]) -> String {
    hooks
        .iter()
        .map(|h| format!("<system>\n{}\n</system>", h))
        .collect::<Vec<_>>()
        .join("\n\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_system_hooks_empty() {
        let hooks: Vec<String> = vec![];
        let formatted = format_system_hooks(&hooks);
        assert!(formatted.is_empty());
    }

    #[test]
    fn test_format_system_hooks_single() {
        let hooks = vec!["Test hook".to_string()];
        let formatted = format_system_hooks(&hooks);

        assert!(formatted.starts_with("<system>"));
        assert!(formatted.ends_with("</system>"));
        assert!(formatted.contains("Test hook"));
    }

    #[test]
    fn test_format_system_hooks_multiple() {
        let hooks = vec!["Hook 1".to_string(), "Hook 2".to_string()];
        let formatted = format_system_hooks(&hooks);

        assert!(formatted.contains("Hook 1"));
        assert!(formatted.contains("Hook 2"));
        assert_eq!(formatted.matches("<system>").count(), 2);
        assert_eq!(formatted.matches("</system>").count(), 2);

        // Should be separated by blank line
        assert!(formatted.contains("</system>\n\n<system>"));
    }

    #[test]
    fn test_format_system_hooks_multiline_content() {
        let hooks = vec!["Line 1\nLine 2\nLine 3".to_string()];
        let formatted = format_system_hooks(&hooks);

        assert!(formatted.contains("Line 1\nLine 2\nLine 3"));
        assert!(formatted.starts_with("<system>"));
        assert!(formatted.ends_with("</system>"));
    }
}
