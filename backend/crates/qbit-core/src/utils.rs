//! Utility functions for common operations.

/// Truncates a string to at most `max_bytes` bytes, respecting UTF-8 character boundaries.
///
/// This function ensures the truncation point falls on a valid UTF-8 character boundary,
/// preventing panics that would occur from slicing in the middle of a multi-byte character.
///
/// # Arguments
/// * `s` - The string to truncate
/// * `max_bytes` - Maximum number of bytes in the result
///
/// # Returns
/// A string slice that is at most `max_bytes` bytes long, ending at a valid character boundary.
///
/// # Example
/// ```
/// # use qbit_core::utils::truncate_str;
/// let s = "Hello, 世界!"; // "世" and "界" are 3 bytes each
/// assert_eq!(truncate_str(s, 10), "Hello, 世");
/// assert_eq!(truncate_str(s, 7), "Hello, ");
/// assert_eq!(truncate_str(s, 100), s); // No truncation needed
/// ```
pub fn truncate_str(s: &str, max_bytes: usize) -> &str {
    if s.len() <= max_bytes {
        return s;
    }

    // Find the last character boundary at or before max_bytes
    // char_indices() yields (byte_position, char) for each character
    let mut end = 0;
    for (idx, _) in s.char_indices() {
        if idx > max_bytes {
            break;
        }
        end = idx;
    }

    // Handle edge case: if first char is already beyond max_bytes, return empty
    // Also handle the case where we stopped at a boundary that fits
    if end == 0 && s.len() > max_bytes {
        // Check if first character fits
        if let Some((first_char_end, _)) = s.char_indices().nth(1) {
            if first_char_end <= max_bytes {
                end = first_char_end;
            }
        } else if s.len() <= max_bytes {
            // Single character string that fits
            return s;
        }
    }

    // Get the byte position of the next character (or end of string)
    // to include the character at position `end`
    let actual_end = s[end..]
        .char_indices()
        .nth(1)
        .map(|(idx, _)| end + idx)
        .unwrap_or(s.len());

    if actual_end <= max_bytes {
        &s[..actual_end]
    } else {
        &s[..end]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate_str_ascii() {
        let s = "Hello, World!";
        assert_eq!(truncate_str(s, 5), "Hello");
        assert_eq!(truncate_str(s, 7), "Hello, ");
        assert_eq!(truncate_str(s, 100), s);
        assert_eq!(truncate_str(s, 0), "");
    }

    #[test]
    fn test_truncate_str_unicode() {
        // Each CJK character is 3 bytes
        let s = "Hello, 世界!";
        assert_eq!(truncate_str(s, 10), "Hello, 世"); // 7 + 3 = 10
        assert_eq!(truncate_str(s, 9), "Hello, "); // Can't fit 世 (3 bytes)
        assert_eq!(truncate_str(s, 8), "Hello, "); // Can't fit 世 (3 bytes)
        assert_eq!(truncate_str(s, 7), "Hello, ");
    }

    #[test]
    fn test_truncate_str_box_drawing() {
        // Box drawing character ─ is 3 bytes (the one that caused the original panic)
        let s = "Result: ─────";
        assert_eq!(truncate_str(s, 8), "Result: ");
        assert_eq!(truncate_str(s, 11), "Result: ─"); // 8 + 3 = 11
        assert_eq!(truncate_str(s, 10), "Result: "); // 8 + 2 not enough for ─
    }

    #[test]
    fn test_truncate_str_emoji() {
        // Emoji can be 4 bytes
        let s = "Hi 👋 there";
        assert_eq!(truncate_str(s, 3), "Hi ");
        assert_eq!(truncate_str(s, 7), "Hi 👋"); // 3 + 4 = 7
        assert_eq!(truncate_str(s, 6), "Hi "); // Can't fit emoji
    }

    #[test]
    fn test_truncate_str_empty() {
        assert_eq!(truncate_str("", 10), "");
        assert_eq!(truncate_str("", 0), "");
    }

    #[test]
    fn test_truncate_str_exact_boundary() {
        let s = "abc";
        assert_eq!(truncate_str(s, 3), "abc");
        assert_eq!(truncate_str(s, 2), "ab");
        assert_eq!(truncate_str(s, 1), "a");
    }
}
