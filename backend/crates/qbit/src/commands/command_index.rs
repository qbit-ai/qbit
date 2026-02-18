use std::collections::HashSet;
use std::sync::RwLock;

use qbit_pty::ShellType;
use serde::Serialize;

/// Index of executable commands available in the user's PATH plus shell builtins.
/// Used by "auto" input mode to classify whether user input is a command or natural language.
pub struct CommandIndex {
    commands: RwLock<HashSet<String>>,
    initialized: RwLock<bool>,
}

/// Result of classifying user input as terminal command vs agent prompt.
#[derive(Debug, Clone, Serialize)]
pub struct ClassifyResult {
    pub route: String,
    pub detected_command: Option<String>,
}

impl Default for CommandIndex {
    fn default() -> Self {
        Self::new()
    }
}

impl CommandIndex {
    pub fn new() -> Self {
        Self {
            commands: RwLock::new(HashSet::new()),
            initialized: RwLock::new(false),
        }
    }

    /// Build the command index by scanning PATH directories for executables
    /// and adding shell builtins (detected from `$SHELL` env var).
    pub fn build(&self) {
        let shell_type = detect_shell_type();
        let mut commands = HashSet::new();

        // Resolve the user's full shell PATH. On macOS, GUI apps launched from
        // the dock/Finder don't inherit the user's shell PATH, so directories
        // like ~/.local/bin won't be included in std::env::var("PATH").
        let path_var = resolve_shell_path().or_else(|| std::env::var("PATH").ok());

        // Scan PATH directories for executables
        if let Some(ref path_var) = path_var {
            for dir in path_var.split(':') {
                let dir_path = std::path::Path::new(dir);
                if let Ok(entries) = std::fs::read_dir(dir_path) {
                    for entry in entries.flatten() {
                        // Use std::fs::metadata (not entry.metadata()) to follow
                        // symlinks. On Unix, entry.metadata() is equivalent to
                        // lstat and reports symlinks as non-files.
                        if let Ok(metadata) = std::fs::metadata(entry.path()) {
                            if metadata.is_file() && is_executable(&metadata) {
                                if let Some(name) = entry.file_name().to_str() {
                                    commands.insert(name.to_string());
                                }
                            }
                        }
                    }
                }
            }
        }

        // Add shell builtins
        for builtin in shell_builtins(shell_type) {
            commands.insert(builtin.to_string());
        }

        let count = commands.len();
        *self.commands.write().unwrap() = commands;
        *self.initialized.write().unwrap() = true;
        tracing::info!("[command-index] Built index with {} commands", count);
    }

    /// Classify user input as either a terminal command or an agent prompt.
    pub fn classify(&self, input: &str) -> ClassifyResult {
        let input = input.trim();
        if input.is_empty() {
            return ClassifyResult {
                route: "agent".to_string(),
                detected_command: None,
            };
        }

        // 1. Input starts with path prefix → Terminal
        if input.starts_with("./") || input.starts_with('/') || input.starts_with("~/") {
            return ClassifyResult {
                route: "terminal".to_string(),
                detected_command: None,
            };
        }

        // 2. Input contains shell operators → Terminal
        if contains_shell_operator(input) {
            // Extract first token as detected command
            let first_token = first_token(input);
            let commands = self.commands.read().unwrap();
            let detected = if commands.contains(first_token) {
                Some(first_token.to_string())
            } else {
                None
            };
            return ClassifyResult {
                route: "terminal".to_string(),
                detected_command: detected,
            };
        }

        // 3. Check first token against known commands
        let first = first_token(input);
        let commands = self.commands.read().unwrap();

        if commands.contains(first) {
            // First token is a known command — apply secondary heuristics
            let tokens: Vec<&str> = input.split_whitespace().collect();

            // Has flags (e.g. -x, --foo) → definitely a command
            if tokens.iter().any(|t| t.starts_with('-')) {
                return ClassifyResult {
                    route: "terminal".to_string(),
                    detected_command: Some(first.to_string()),
                };
            }

            // Only 1-2 tokens → likely a command (e.g. "ls", "git status")
            if tokens.len() <= 2 {
                return ClassifyResult {
                    route: "terminal".to_string(),
                    detected_command: Some(first.to_string()),
                };
            }

            // 3+ tokens with all plain English words (no special chars) → likely natural language
            // that happens to start with a command name (e.g. "make sure the tests pass")
            let rest_tokens = &tokens[1..];
            let all_plain_words = rest_tokens.iter().all(|t| {
                t.chars()
                    .all(|c| c.is_ascii_alphabetic() || c == '\'' || c == ',')
            });

            if all_plain_words && rest_tokens.len() >= 2 {
                return ClassifyResult {
                    route: "agent".to_string(),
                    detected_command: Some(first.to_string()),
                };
            }

            // Has paths, special chars, etc. → command
            return ClassifyResult {
                route: "terminal".to_string(),
                detected_command: Some(first.to_string()),
            };
        }

        // 4. First token not recognized → treat as natural language prompt
        ClassifyResult {
            route: "agent".to_string(),
            detected_command: None,
        }
    }
}

/// Check if a file is executable (Unix).
#[cfg(unix)]
fn is_executable(metadata: &std::fs::Metadata) -> bool {
    use std::os::unix::fs::PermissionsExt;
    metadata.permissions().mode() & 0o111 != 0
}

#[cfg(not(unix))]
fn is_executable(_metadata: &std::fs::Metadata) -> bool {
    true
}

/// Resolve the user's full shell PATH by spawning a login shell.
/// On macOS/Linux, GUI apps don't inherit PATH entries added by shell
/// rc files (e.g. ~/.local/bin from .zshrc/.bashrc).
#[cfg(unix)]
fn resolve_shell_path() -> Option<String> {
    let shell = std::env::var("SHELL").unwrap_or_else(|_| {
        if cfg!(target_os = "macos") {
            "/bin/zsh".to_string()
        } else {
            "/bin/sh".to_string()
        }
    });

    let output = std::process::Command::new(&shell)
        .args(["-lic", "echo __QBIT_CMD_IDX_PATH__=$PATH"])
        .output()
        .ok()?;

    if !output.status.success() {
        tracing::warn!(
            "[command-index] Login shell exited with status {} while resolving PATH",
            output.status
        );
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines() {
        if let Some(path) = line.strip_prefix("__QBIT_CMD_IDX_PATH__=") {
            let path = path.trim().to_string();
            tracing::debug!("[command-index] Resolved shell PATH: {}", path);
            return Some(path);
        }
    }

    tracing::warn!("[command-index] Failed to extract PATH from login shell output");
    None
}

#[cfg(not(unix))]
fn resolve_shell_path() -> Option<String> {
    None
}

/// Extract the first whitespace-delimited token from input.
fn first_token(input: &str) -> &str {
    input.split_whitespace().next().unwrap_or("")
}

/// Check if input contains common shell operators.
fn contains_shell_operator(input: &str) -> bool {
    // Check for pipe, redirect, logical operators, semicolon
    // Be careful not to match these inside quoted strings (simple heuristic)
    let mut in_single_quote = false;
    let mut in_double_quote = false;
    let chars: Vec<char> = input.chars().collect();
    let len = chars.len();

    for i in 0..len {
        let c = chars[i];
        match c {
            '\'' if !in_double_quote => in_single_quote = !in_single_quote,
            '"' if !in_single_quote => in_double_quote = !in_double_quote,
            _ if in_single_quote || in_double_quote => continue,
            '|' => return true,
            ';' => return true,
            '>' => return true,
            '<' => return true,
            '&' if i + 1 < len && chars[i + 1] == '&' => return true,
            _ => {}
        }
    }
    false
}

/// Detect the shell type from the `SHELL` environment variable.
fn detect_shell_type() -> ShellType {
    let shell = std::env::var("SHELL").unwrap_or_default();
    if shell.ends_with("/zsh") || shell.ends_with("/zsh5") {
        ShellType::Zsh
    } else if shell.ends_with("/bash") {
        ShellType::Bash
    } else if shell.ends_with("/fish") {
        ShellType::Fish
    } else {
        ShellType::Unknown
    }
}

/// Return shell builtins for the given shell type.
fn shell_builtins(shell_type: ShellType) -> &'static [&'static str] {
    match shell_type {
        ShellType::Zsh => &[
            "alias",
            "autoload",
            "bg",
            "bindkey",
            "builtin",
            "cd",
            "command",
            "compdef",
            "compadd",
            "declare",
            "dirs",
            "disown",
            "echo",
            "emulate",
            "eval",
            "exec",
            "exit",
            "export",
            "false",
            "fc",
            "fg",
            "float",
            "functions",
            "getln",
            "hash",
            "history",
            "integer",
            "jobs",
            "kill",
            "let",
            "limit",
            "local",
            "log",
            "logout",
            "noglob",
            "popd",
            "print",
            "printf",
            "pushd",
            "pushln",
            "pwd",
            "read",
            "readonly",
            "rehash",
            "return",
            "sched",
            "set",
            "setopt",
            "shift",
            "source",
            "stat",
            "suspend",
            "test",
            "times",
            "trap",
            "true",
            "ttyctl",
            "type",
            "typeset",
            "ulimit",
            "umask",
            "unalias",
            "unfunction",
            "unhash",
            "unlimit",
            "unset",
            "unsetopt",
            "vared",
            "wait",
            "whence",
            "where",
            "which",
            "zcompile",
            "zformat",
            "zle",
            "zmodload",
            "zparseopts",
            "zstyle",
        ],
        ShellType::Bash => &[
            "alias",
            "bg",
            "bind",
            "break",
            "builtin",
            "caller",
            "cd",
            "command",
            "compgen",
            "complete",
            "compopt",
            "continue",
            "declare",
            "dirs",
            "disown",
            "echo",
            "enable",
            "eval",
            "exec",
            "exit",
            "export",
            "false",
            "fc",
            "fg",
            "getopts",
            "hash",
            "help",
            "history",
            "jobs",
            "kill",
            "let",
            "local",
            "logout",
            "mapfile",
            "popd",
            "printf",
            "pushd",
            "pwd",
            "read",
            "readarray",
            "readonly",
            "return",
            "set",
            "shift",
            "shopt",
            "source",
            "suspend",
            "test",
            "times",
            "trap",
            "true",
            "type",
            "typeset",
            "ulimit",
            "umask",
            "unalias",
            "unset",
            "wait",
        ],
        ShellType::Fish => &[
            "abbr",
            "alias",
            "and",
            "argparse",
            "begin",
            "bg",
            "bind",
            "block",
            "break",
            "breakpoint",
            "builtin",
            "case",
            "cd",
            "command",
            "commandline",
            "complete",
            "contains",
            "continue",
            "count",
            "disown",
            "echo",
            "else",
            "emit",
            "end",
            "eval",
            "exec",
            "exit",
            "false",
            "fg",
            "for",
            "function",
            "functions",
            "history",
            "if",
            "jobs",
            "math",
            "not",
            "or",
            "popd",
            "printf",
            "pushd",
            "pwd",
            "random",
            "read",
            "realpath",
            "return",
            "set",
            "set_color",
            "source",
            "status",
            "string",
            "suspend",
            "switch",
            "test",
            "time",
            "trap",
            "true",
            "type",
            "ulimit",
            "wait",
            "while",
        ],
        ShellType::Unknown => &[
            // Common POSIX builtins as fallback
            "alias", "bg", "cd", "command", "echo", "eval", "exec", "exit", "export", "false", "fc",
            "fg", "getopts", "hash", "jobs", "kill", "local", "popd", "printf", "pushd", "pwd",
            "read", "readonly", "return", "set", "shift", "source", "test", "times", "trap",
            "true", "type", "ulimit", "umask", "unalias", "unset", "wait",
        ],
    }
}

// -- Tauri command --

#[tauri::command]
pub async fn classify_input(
    state: tauri::State<'_, crate::state::AppState>,
    input: String,
) -> Result<ClassifyResult, String> {
    Ok(state.command_index.classify(&input))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build_test_index(extra_commands: &[&str]) -> CommandIndex {
        let index = CommandIndex::new();
        {
            let mut cmds = index.commands.write().unwrap();
            // Add a minimal set of commands for testing
            for cmd in [
                "ls", "cd", "git", "cat", "grep", "echo", "python", "node", "cargo", "make",
                "docker", "ssh", "curl", "find", "rm", "mkdir", "cp", "mv",
            ] {
                cmds.insert(cmd.to_string());
            }
            for cmd in extra_commands {
                cmds.insert(cmd.to_string());
            }
            *index.initialized.write().unwrap() = true;
        }
        index
    }

    #[test]
    fn test_path_prefix_routes_to_terminal() {
        let index = build_test_index(&[]);
        assert_eq!(index.classify("./script.sh").route, "terminal");
        assert_eq!(index.classify("/usr/bin/python3").route, "terminal");
        assert_eq!(index.classify("~/bin/run.sh").route, "terminal");
    }

    #[test]
    fn test_shell_operators_route_to_terminal() {
        let index = build_test_index(&[]);
        assert_eq!(index.classify("cat foo | grep bar").route, "terminal");
        assert_eq!(index.classify("echo hello > file.txt").route, "terminal");
        assert_eq!(index.classify("ls && pwd").route, "terminal");
        assert_eq!(index.classify("cmd1 ; cmd2").route, "terminal");
    }

    #[test]
    fn test_known_command_with_flags() {
        let index = build_test_index(&[]);
        assert_eq!(index.classify("ls -la").route, "terminal");
        assert_eq!(index.classify("git --version").route, "terminal");
        assert_eq!(index.classify("docker run --rm").route, "terminal");
    }

    #[test]
    fn test_single_known_command() {
        let index = build_test_index(&[]);
        assert_eq!(index.classify("ls").route, "terminal");
        assert_eq!(index.classify("git status").route, "terminal");
    }

    #[test]
    fn test_natural_language_starting_with_command() {
        let index = build_test_index(&[]);
        // "make sure the tests pass" — 3+ plain English words after "make"
        assert_eq!(index.classify("make sure the tests pass").route, "agent");
        // "find all the bugs" — "find" is a command but rest is plain English
        assert_eq!(index.classify("find all the bugs").route, "agent");
    }

    #[test]
    fn test_unknown_first_token() {
        let index = build_test_index(&[]);
        assert_eq!(index.classify("what files are here").route, "agent");
        assert_eq!(index.classify("explain this code").route, "agent");
        assert_eq!(index.classify("python is great").route, "agent");
    }

    #[test]
    fn test_command_with_path_args() {
        let index = build_test_index(&[]);
        // "cat src/main.rs" — 2 tokens, known command → terminal
        assert_eq!(index.classify("cat src/main.rs").route, "terminal");
    }

    #[test]
    fn test_empty_input() {
        let index = build_test_index(&[]);
        assert_eq!(index.classify("").route, "agent");
        assert_eq!(index.classify("  ").route, "agent");
    }

    #[test]
    fn test_shell_operators_in_quotes_ignored() {
        let index = build_test_index(&[]);
        // Pipe inside quotes should not be treated as operator
        // But the first token "echo" is known, and there are 2 tokens → terminal
        assert_eq!(index.classify("echo \"hello | world\"").route, "terminal");
    }

    #[test]
    fn test_classify_result_detected_command() {
        let index = build_test_index(&[]);
        let result = index.classify("git status");
        assert_eq!(result.route, "terminal");
        assert_eq!(result.detected_command, Some("git".to_string()));

        let result = index.classify("what is this");
        assert_eq!(result.route, "agent");
        assert_eq!(result.detected_command, None);
    }
}
