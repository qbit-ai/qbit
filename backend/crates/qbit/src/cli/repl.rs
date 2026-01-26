//! Lightweight REPL (Read-Eval-Print-Loop) for qbit-cli.
//!
//! Provides an interactive mode when no prompt is provided via `-e` or `-f`.
//! Supports commands:
//! - `/quit`, `/exit`, `/q` - Exit the REPL
//! - `/<prompt-name>` or `/<skill-name>` [args] - Execute a prompt or skill with optional arguments
//!
//! Any other input is sent as a prompt to the agent.

use std::collections::HashMap;
use std::fs;
use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};

use anyhow::Result;

use super::bootstrap::CliContext;
use super::runner::execute_once;

/// REPL command variants.
#[derive(Debug, Clone, PartialEq)]
pub enum ReplCommand {
    /// Exit the REPL
    Quit,
    /// Unknown command (will show help)
    Unknown(String),
    /// Regular prompt to send to the agent
    Prompt(String),
    /// Slash command (prompt or skill) with optional arguments
    SlashCommand { name: String, args: Option<String> },
    /// Empty input (skip)
    Empty,
}

impl ReplCommand {
    /// Parse user input into a REPL command.
    pub fn parse(input: &str) -> Self {
        let trimmed = input.trim();

        if trimmed.is_empty() {
            return ReplCommand::Empty;
        }

        if let Some(after_slash) = trimmed.strip_prefix('/') {
            // Check for built-in commands first (case-insensitive)
            let lower = after_slash.to_lowercase();
            if lower == "quit" || lower == "exit" || lower == "q" {
                return ReplCommand::Quit;
            }

            // Parse as slash command: /name [args]
            let parts: Vec<&str> = after_slash.splitn(2, ' ').collect();
            let name = parts[0].to_string();
            let args = parts
                .get(1)
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty());

            if name.is_empty() {
                return ReplCommand::Unknown(trimmed.to_string());
            }

            ReplCommand::SlashCommand { name, args }
        } else {
            ReplCommand::Prompt(trimmed.to_string())
        }
    }
}

/// Find a prompt file by name, checking local then global directories.
fn find_prompt(workspace: &Path, name: &str) -> Option<PathBuf> {
    // Check local prompts first
    let local_path = workspace
        .join(".qbit")
        .join("prompts")
        .join(format!("{}.md", name));
    if local_path.exists() {
        return Some(local_path);
    }

    // Check global prompts
    if let Some(home) = dirs::home_dir() {
        let global_path = home
            .join(".qbit")
            .join("prompts")
            .join(format!("{}.md", name));
        if global_path.exists() {
            return Some(global_path);
        }
    }

    None
}

/// Find a skill directory by name, checking local then global directories.
fn find_skill(workspace: &Path, name: &str) -> Option<PathBuf> {
    // Check local skills first
    let local_path = workspace.join(".qbit").join("skills").join(name);
    if local_path.join("SKILL.md").exists() {
        return Some(local_path);
    }

    // Check global skills
    if let Some(home) = dirs::home_dir() {
        let global_path = home.join(".qbit").join("skills").join(name);
        if global_path.join("SKILL.md").exists() {
            return Some(global_path);
        }
    }

    None
}

/// Parse SKILL.md content and extract just the body (instructions).
fn parse_skill_body(content: &str) -> String {
    // Check for YAML frontmatter delimiters
    if !content.starts_with("---") {
        return content.to_string();
    }

    // Find the closing delimiter
    let after_first = &content[3..];
    if let Some(end_pos) = after_first.find("\n---") {
        // Extract body (everything after closing delimiter and newline)
        let body_start = 3 + end_pos + 4; // "---" + yaml + "\n---"
        if body_start < content.len() {
            return content[body_start..].trim_start_matches('\n').to_string();
        }
    }

    content.to_string()
}

/// List available prompts and skills for help message.
fn list_available_commands(workspace: &Path) -> (Vec<String>, Vec<String>) {
    let mut prompts = Vec::new();
    let mut skills = Vec::new();
    let mut seen_prompts: HashMap<String, bool> = HashMap::new();
    let mut seen_skills: HashMap<String, bool> = HashMap::new();

    // Collect local prompts
    let local_prompts_dir = workspace.join(".qbit").join("prompts");
    if local_prompts_dir.exists() {
        if let Ok(entries) = fs::read_dir(&local_prompts_dir) {
            for entry in entries.filter_map(|e| e.ok()) {
                let path = entry.path();
                if path.extension().is_some_and(|ext| ext == "md") {
                    if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                        seen_prompts.insert(stem.to_string(), true);
                        prompts.push(stem.to_string());
                    }
                }
            }
        }
    }

    // Collect global prompts (only if not already seen)
    if let Some(home) = dirs::home_dir() {
        let global_prompts_dir = home.join(".qbit").join("prompts");
        if global_prompts_dir.exists() {
            if let Ok(entries) = fs::read_dir(&global_prompts_dir) {
                for entry in entries.filter_map(|e| e.ok()) {
                    let path = entry.path();
                    if path.extension().is_some_and(|ext| ext == "md") {
                        if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                            if !seen_prompts.contains_key(stem) {
                                prompts.push(stem.to_string());
                            }
                        }
                    }
                }
            }
        }
    }

    // Collect local skills
    let local_skills_dir = workspace.join(".qbit").join("skills");
    if local_skills_dir.exists() {
        if let Ok(entries) = fs::read_dir(&local_skills_dir) {
            for entry in entries.filter_map(|e| e.ok()) {
                let path = entry.path();
                if path.is_dir() && path.join("SKILL.md").exists() {
                    if let Some(name) = path.file_name().and_then(|s| s.to_str()) {
                        seen_skills.insert(name.to_string(), true);
                        skills.push(name.to_string());
                    }
                }
            }
        }
    }

    // Collect global skills (only if not already seen)
    if let Some(home) = dirs::home_dir() {
        let global_skills_dir = home.join(".qbit").join("skills");
        if global_skills_dir.exists() {
            if let Ok(entries) = fs::read_dir(&global_skills_dir) {
                for entry in entries.filter_map(|e| e.ok()) {
                    let path = entry.path();
                    if path.is_dir() && path.join("SKILL.md").exists() {
                        if let Some(name) = path.file_name().and_then(|s| s.to_str()) {
                            if !seen_skills.contains_key(name) {
                                skills.push(name.to_string());
                            }
                        }
                    }
                }
            }
        }
    }

    prompts.sort();
    skills.sort();
    (prompts, skills)
}

/// Run an interactive REPL session.
///
/// Supports:
/// - `/quit`, `/exit`, `/q` - Exit the REPL
/// - `/<prompt-name>` or `/<skill-name>` [args] - Execute a prompt or skill
/// - Any other input - Send as prompt to agent
///
/// Returns when the user exits or on EOF (Ctrl+D).
pub async fn run_repl(ctx: &mut CliContext) -> Result<()> {
    let stdin = io::stdin();
    let mut stdout = io::stdout();

    // Print banner
    eprintln!("qbit-cli interactive mode");
    eprintln!("Type /quit to exit\n");

    loop {
        // Print prompt
        print!("> ");
        stdout.flush()?;

        // Read line
        let mut input = String::new();
        if stdin.lock().read_line(&mut input)? == 0 {
            // EOF (Ctrl+D)
            eprintln!("\nGoodbye!");
            break;
        }

        // Parse and handle command
        match ReplCommand::parse(&input) {
            ReplCommand::Empty => {
                continue;
            }
            ReplCommand::Quit => {
                eprintln!("Goodbye!");
                break;
            }
            ReplCommand::Unknown(cmd) => {
                eprintln!("Unknown command: {}", cmd);
                eprintln!("Available: /quit, /exit, /q");
                continue;
            }
            ReplCommand::SlashCommand { name, args } => {
                // Try to find prompt first (prompts take precedence over skills)
                if let Some(prompt_path) = find_prompt(&ctx.workspace, &name) {
                    match fs::read_to_string(&prompt_path) {
                        Ok(content) => {
                            let full_content = if let Some(ref args_str) = args {
                                format!("{}\n\n{}", content, args_str)
                            } else {
                                content
                            };
                            if let Err(e) = execute_once(ctx, &full_content).await {
                                eprintln!("Error: {}", e);
                            }
                        }
                        Err(e) => {
                            eprintln!("Failed to read prompt '{}': {}", name, e);
                        }
                    }
                } else if let Some(skill_path) = find_skill(&ctx.workspace, &name) {
                    // Try skill
                    let skill_md_path = skill_path.join("SKILL.md");
                    match fs::read_to_string(&skill_md_path) {
                        Ok(content) => {
                            let body = parse_skill_body(&content);
                            let full_content = if let Some(ref args_str) = args {
                                format!("{}\n\n{}", body, args_str)
                            } else {
                                body
                            };
                            if let Err(e) = execute_once(ctx, &full_content).await {
                                eprintln!("Error: {}", e);
                            }
                        }
                        Err(e) => {
                            eprintln!("Failed to read skill '{}': {}", name, e);
                        }
                    }
                } else {
                    // Command not found - show available commands
                    eprintln!("Unknown command: /{}", name);
                    let (prompts, skills) = list_available_commands(&ctx.workspace);
                    eprintln!("Available: /quit, /exit, /q");
                    if !prompts.is_empty() {
                        eprintln!(
                            "Prompts: {}",
                            prompts
                                .iter()
                                .map(|p| format!("/{}", p))
                                .collect::<Vec<_>>()
                                .join(", ")
                        );
                    }
                    if !skills.is_empty() {
                        eprintln!(
                            "Skills: {}",
                            skills
                                .iter()
                                .map(|s| format!("/{}", s))
                                .collect::<Vec<_>>()
                                .join(", ")
                        );
                    }
                }
                println!(); // Blank line between interactions
            }
            ReplCommand::Prompt(prompt) => {
                // Execute prompt via agent
                if let Err(e) = execute_once(ctx, &prompt).await {
                    eprintln!("Error: {}", e);
                }

                println!(); // Blank line between interactions
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // ────────────────────────────────────────────────────────────────────────────────
    // Tests for ReplCommand::parse
    // ────────────────────────────────────────────────────────────────────────────────

    mod parse_tests {
        use super::*;

        #[test]
        fn parses_quit_command() {
            assert_eq!(ReplCommand::parse("/quit"), ReplCommand::Quit);
        }

        #[test]
        fn parses_exit_command() {
            assert_eq!(ReplCommand::parse("/exit"), ReplCommand::Quit);
        }

        #[test]
        fn parses_q_command() {
            assert_eq!(ReplCommand::parse("/q"), ReplCommand::Quit);
        }

        #[test]
        fn parses_quit_case_insensitive() {
            assert_eq!(ReplCommand::parse("/QUIT"), ReplCommand::Quit);
            assert_eq!(ReplCommand::parse("/Quit"), ReplCommand::Quit);
            assert_eq!(ReplCommand::parse("/EXIT"), ReplCommand::Quit);
            assert_eq!(ReplCommand::parse("/Q"), ReplCommand::Quit);
        }

        #[test]
        fn parses_slash_command_without_args() {
            assert_eq!(
                ReplCommand::parse("/my-prompt"),
                ReplCommand::SlashCommand {
                    name: "my-prompt".to_string(),
                    args: None
                }
            );
        }

        #[test]
        fn parses_slash_command_with_args() {
            assert_eq!(
                ReplCommand::parse("/my-prompt some arguments here"),
                ReplCommand::SlashCommand {
                    name: "my-prompt".to_string(),
                    args: Some("some arguments here".to_string())
                }
            );
        }

        #[test]
        fn parses_slash_command_with_multiword_args() {
            assert_eq!(
                ReplCommand::parse("/test-skill fix the bug in auth.rs"),
                ReplCommand::SlashCommand {
                    name: "test-skill".to_string(),
                    args: Some("fix the bug in auth.rs".to_string())
                }
            );
        }

        #[test]
        fn parses_slash_command_trims_args() {
            assert_eq!(
                ReplCommand::parse("/my-prompt   spaced args  "),
                ReplCommand::SlashCommand {
                    name: "my-prompt".to_string(),
                    args: Some("spaced args".to_string())
                }
            );
        }

        #[test]
        fn parses_slash_command_empty_args_becomes_none() {
            assert_eq!(
                ReplCommand::parse("/my-prompt   "),
                ReplCommand::SlashCommand {
                    name: "my-prompt".to_string(),
                    args: None
                }
            );
        }

        #[test]
        fn parses_unknown_for_bare_slash() {
            assert_eq!(
                ReplCommand::parse("/"),
                ReplCommand::Unknown("/".to_string())
            );
        }

        #[test]
        fn parses_regular_prompt() {
            assert_eq!(
                ReplCommand::parse("Hello world"),
                ReplCommand::Prompt("Hello world".to_string())
            );
        }

        #[test]
        fn parses_prompt_with_slash_in_middle() {
            // Slash in middle should not be treated as command
            assert_eq!(
                ReplCommand::parse("Read /tmp/file.txt"),
                ReplCommand::Prompt("Read /tmp/file.txt".to_string())
            );
        }

        #[test]
        fn parses_empty_input() {
            assert_eq!(ReplCommand::parse(""), ReplCommand::Empty);
            assert_eq!(ReplCommand::parse("   "), ReplCommand::Empty);
            assert_eq!(ReplCommand::parse("\t\n"), ReplCommand::Empty);
        }

        #[test]
        fn trims_whitespace_from_prompt() {
            assert_eq!(
                ReplCommand::parse("  hello  "),
                ReplCommand::Prompt("hello".to_string())
            );
        }

        #[test]
        fn trims_whitespace_from_command() {
            assert_eq!(ReplCommand::parse("  /quit  "), ReplCommand::Quit);
        }

        #[test]
        fn handles_newline_in_input() {
            // This simulates input from stdin with trailing newline
            assert_eq!(
                ReplCommand::parse("hello\n"),
                ReplCommand::Prompt("hello".to_string())
            );
            assert_eq!(ReplCommand::parse("/quit\n"), ReplCommand::Quit);
        }
    }

    mod skill_body_tests {
        use super::*;

        #[test]
        fn parses_skill_with_frontmatter() {
            let content = r#"---
name: test-skill
description: A test skill
---

You are a testing assistant."#;
            let body = parse_skill_body(content);
            assert_eq!(body.trim(), "You are a testing assistant.");
        }

        #[test]
        fn returns_content_without_frontmatter() {
            let content = "Just plain markdown content";
            let body = parse_skill_body(content);
            assert_eq!(body, "Just plain markdown content");
        }

        #[test]
        fn handles_empty_body() {
            let content = r#"---
name: empty-skill
description: Empty body
---
"#;
            let body = parse_skill_body(content);
            assert!(body.is_empty() || body.chars().all(|c| c.is_whitespace()));
        }
    }
}
