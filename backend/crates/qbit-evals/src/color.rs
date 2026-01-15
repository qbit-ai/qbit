//! Color helpers that respect CI environment.
//!
//! In CI, ANSI escape codes are stripped for cleaner logs.

use std::sync::OnceLock;

static IS_CI: OnceLock<bool> = OnceLock::new();

/// Check if running in CI environment.
pub fn is_ci() -> bool {
    *IS_CI.get_or_init(|| std::env::var("CI").map(|v| v == "true").unwrap_or(false))
}

/// Red text (errors, failures).
pub fn red(s: &str) -> String {
    if is_ci() {
        s.to_string()
    } else {
        format!("\x1b[31m{}\x1b[0m", s)
    }
}

/// Green text (success, pass).
pub fn green(s: &str) -> String {
    if is_ci() {
        s.to_string()
    } else {
        format!("\x1b[32m{}\x1b[0m", s)
    }
}

/// Yellow text (warnings, agent output).
pub fn yellow(s: &str) -> String {
    if is_ci() {
        s.to_string()
    } else {
        format!("\x1b[33m{}\x1b[0m", s)
    }
}

/// Cyan text (user input, info).
pub fn cyan(s: &str) -> String {
    if is_ci() {
        s.to_string()
    } else {
        format!("\x1b[36m{}\x1b[0m", s)
    }
}

/// Gray/dim text (skipped items).
pub fn gray(s: &str) -> String {
    if is_ci() {
        s.to_string()
    } else {
        format!("\x1b[90m{}\x1b[0m", s)
    }
}

/// Reset sequence (no-op in CI).
pub fn reset() -> &'static str {
    if is_ci() {
        ""
    } else {
        "\x1b[0m"
    }
}

/// Pass check mark.
pub fn check_mark() -> &'static str {
    if is_ci() {
        "[PASS]"
    } else {
        "✓"
    }
}

/// Fail X mark.
pub fn x_mark() -> &'static str {
    if is_ci() {
        "[FAIL]"
    } else {
        "✗"
    }
}

/// Partial/warning bullet.
pub fn bullet() -> &'static str {
    if is_ci() {
        "[*]"
    } else {
        "●"
    }
}

/// Skip circle.
pub fn skip_mark() -> &'static str {
    if is_ci() {
        "[SKIP]"
    } else {
        "○"
    }
}
