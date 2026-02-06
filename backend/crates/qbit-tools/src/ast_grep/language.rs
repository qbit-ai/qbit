//! Language detection and mapping for AST-grep.

use ast_grep_language::SupportLang;
use std::path::Path;

/// Detect the programming language from a file extension.
pub fn detect_language(path: &str) -> Option<SupportLang> {
    let path = Path::new(path);
    let ext = path.extension()?.to_str()?;

    match ext.to_lowercase().as_str() {
        // Rust
        "rs" => Some(SupportLang::Rust),

        // JavaScript/TypeScript
        "js" | "mjs" | "cjs" => Some(SupportLang::JavaScript),
        "jsx" => Some(SupportLang::JavaScript),
        "ts" | "mts" | "cts" => Some(SupportLang::TypeScript),
        "tsx" => Some(SupportLang::Tsx),

        // Python
        "py" | "pyi" | "pyw" => Some(SupportLang::Python),

        // Go
        "go" => Some(SupportLang::Go),

        // Java
        "java" => Some(SupportLang::Java),

        // C/C++
        "c" | "h" => Some(SupportLang::C),
        "cpp" | "cc" | "cxx" | "hpp" | "hxx" | "hh" => Some(SupportLang::Cpp),

        // Other supported languages
        "cs" => Some(SupportLang::CSharp),
        "css" => Some(SupportLang::Css),
        "html" | "htm" => Some(SupportLang::Html),
        "json" => Some(SupportLang::Json),
        "kt" | "kts" => Some(SupportLang::Kotlin),
        "lua" => Some(SupportLang::Lua),
        "swift" => Some(SupportLang::Swift),

        _ => None,
    }
}

/// Parse a language string to SupportLang.
pub fn parse_language(lang: &str) -> Option<SupportLang> {
    match lang.to_lowercase().as_str() {
        "rust" | "rs" => Some(SupportLang::Rust),
        "javascript" | "js" => Some(SupportLang::JavaScript),
        "typescript" | "ts" => Some(SupportLang::TypeScript),
        "tsx" => Some(SupportLang::Tsx),
        "jsx" => Some(SupportLang::JavaScript),
        "python" | "py" => Some(SupportLang::Python),
        "go" | "golang" => Some(SupportLang::Go),
        "java" => Some(SupportLang::Java),
        "c" => Some(SupportLang::C),
        "cpp" | "c++" | "cxx" => Some(SupportLang::Cpp),
        "csharp" | "c#" | "cs" => Some(SupportLang::CSharp),
        "css" => Some(SupportLang::Css),
        "html" => Some(SupportLang::Html),
        "json" => Some(SupportLang::Json),
        "kotlin" | "kt" => Some(SupportLang::Kotlin),
        "lua" => Some(SupportLang::Lua),
        "swift" => Some(SupportLang::Swift),
        _ => None,
    }
}

/// Get the file extensions for a given language.
pub fn language_extensions(lang: SupportLang) -> &'static [&'static str] {
    match lang {
        SupportLang::Rust => &["rs"],
        SupportLang::JavaScript => &["js", "mjs", "cjs", "jsx"],
        SupportLang::TypeScript => &["ts", "mts", "cts"],
        SupportLang::Tsx => &["tsx"],
        SupportLang::Python => &["py", "pyi", "pyw"],
        SupportLang::Go => &["go"],
        SupportLang::Java => &["java"],
        SupportLang::C => &["c", "h"],
        SupportLang::Cpp => &["cpp", "cc", "cxx", "hpp", "hxx", "hh"],
        SupportLang::CSharp => &["cs"],
        SupportLang::Css => &["css"],
        SupportLang::Html => &["html", "htm"],
        SupportLang::Json => &["json"],
        SupportLang::Kotlin => &["kt", "kts"],
        SupportLang::Lua => &["lua"],
        SupportLang::Swift => &["swift"],
        // Default to empty for unknown languages
        _ => &[],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_rust() {
        assert_eq!(detect_language("foo.rs"), Some(SupportLang::Rust));
        assert_eq!(detect_language("src/lib.rs"), Some(SupportLang::Rust));
    }

    #[test]
    fn test_detect_typescript() {
        assert_eq!(detect_language("foo.ts"), Some(SupportLang::TypeScript));
        assert_eq!(detect_language("foo.tsx"), Some(SupportLang::Tsx));
    }

    #[test]
    fn test_detect_javascript() {
        assert_eq!(detect_language("foo.js"), Some(SupportLang::JavaScript));
        assert_eq!(detect_language("foo.jsx"), Some(SupportLang::JavaScript));
        assert_eq!(detect_language("foo.mjs"), Some(SupportLang::JavaScript));
    }

    #[test]
    fn test_detect_python() {
        assert_eq!(detect_language("foo.py"), Some(SupportLang::Python));
        assert_eq!(detect_language("foo.pyi"), Some(SupportLang::Python));
    }

    #[test]
    fn test_detect_go() {
        assert_eq!(detect_language("main.go"), Some(SupportLang::Go));
    }

    #[test]
    fn test_detect_c_cpp() {
        assert_eq!(detect_language("foo.c"), Some(SupportLang::C));
        assert_eq!(detect_language("foo.h"), Some(SupportLang::C));
        assert_eq!(detect_language("foo.cpp"), Some(SupportLang::Cpp));
        assert_eq!(detect_language("foo.hpp"), Some(SupportLang::Cpp));
    }

    #[test]
    fn test_detect_unknown() {
        assert_eq!(detect_language("foo.txt"), None);
        assert_eq!(detect_language("README"), None);
    }

    #[test]
    fn test_parse_language() {
        assert_eq!(parse_language("rust"), Some(SupportLang::Rust));
        assert_eq!(parse_language("RUST"), Some(SupportLang::Rust));
        assert_eq!(parse_language("typescript"), Some(SupportLang::TypeScript));
        assert_eq!(parse_language("python"), Some(SupportLang::Python));
        assert_eq!(parse_language("go"), Some(SupportLang::Go));
        assert_eq!(parse_language("unknown"), None);
    }
}
