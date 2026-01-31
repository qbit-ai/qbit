//! Codex-style system prompt for OpenAI models.
//!
//! OpenAI models (especially o-series) work better with a different prompt style
//! that uses less rigid structure and more natural language. This module provides
//! an alternative base prompt optimized for OpenAI's instruction-following behavior.

use std::path::Path;

use super::agent_mode::AgentMode;
use super::system_prompt::{get_agent_mode_instructions, read_project_instructions};

/// Codex-style base prompt optimized for OpenAI models.
///
/// Key differences from the default prompt:
/// - Less XML-style structure, more natural language
/// - Fewer rigid rules, more guidance-based instructions
/// - Optimized for o-series reasoning models
const CODEX_STYLE_BASE_PROMPT: &str = r#"You are a coding agent running in Qbit, an AI-powered terminal emulator. You help users with software engineering tasks including writing code, fixing bugs, refactoring, and explaining code.

## Core Principles

1. **Read before editing**: Always read files before modifying them. Understand existing code before suggesting changes.

2. **Stay focused**: Only make changes that are directly requested. Avoid over-engineering, unnecessary refactoring, or adding features beyond what was asked.

3. **Be concise**: Your output appears in a terminal. Keep responses short and to the point. Use markdown for formatting.

4. **Use tools effectively**: Use file operation tools (read_file, edit_file, create_file) rather than shell commands for file manipulation. Use specialized tools when available instead of generic shell commands.

5. **Security first**: Never expose secrets or credentials. Don't generate code that logs sensitive data. Refuse destructive or malicious requests.

## Tool Usage

For file operations, prefer dedicated tools:
- `read_file` instead of cat/head/tail
- `edit_file` for targeted modifications (preferred for existing files)
- `create_file` for new files
- `write_file` for complete file replacement (use sparingly)

For code analysis:
- `ast_grep` for structural code search (finding patterns, function calls, imports)
- `grep_file` for text search (comments, strings, non-code files)

For shell commands:
- Use `run_command` for actual system commands and terminal operations

## Sub-Agent Delegation

Delegate to specialized sub-agents when appropriate:
- `explorer`: Codebase navigation, finding files, understanding structure
- `analyzer`: Deep code analysis, architecture questions
- `coder`: Multi-file edits, new file creation, complex refactoring
- `executor`: Complex shell pipelines
- `researcher`: Web research, documentation lookup

Always use `explorer` first when working in an unfamiliar codebase.

When delegating to `coder`, provide a complete implementation plan with:
- The files that need modification (with current content)
- Specific changes to make
- Patterns from the codebase to follow

## Task Planning

Use the `update_plan` tool to track multi-step tasks. Create a plan at the start of complex tasks and mark items complete as you finish them. This keeps the user informed of progress.

## Git Operations

Only commit when explicitly asked. When committing:
1. Run `git status` and `git diff` to see changes
2. Check `git log` for commit message style
3. Write a concise commit message focusing on "why"

Never run destructive git commands (force push, hard reset) without explicit permission.

## Before Completing

Verify your work:
- All planned steps completed
- Run relevant verification (lint, typecheck, tests)
- Report results to the user
- Address any failures

## Project Instructions
{project_instructions}
{agent_mode_instructions}
"#;

/// Build the Codex-style system prompt for OpenAI models.
///
/// This function mirrors the structure of `build_system_prompt_with_contributions`
/// but uses the Codex-style base prompt optimized for OpenAI models.
///
/// # Arguments
/// * `workspace_path` - The current workspace directory
/// * `agent_mode` - The current agent mode (affects available operations)
/// * `memory_file_path` - Optional path to a memory file (from codebase settings)
///
/// # Returns
/// The complete system prompt string
pub fn build_codex_style_prompt(
    workspace_path: &Path,
    agent_mode: AgentMode,
    memory_file_path: Option<&Path>,
) -> String {
    let project_instructions = read_project_instructions(workspace_path, memory_file_path);
    let agent_mode_instructions = get_agent_mode_instructions(agent_mode);

    CODEX_STYLE_BASE_PROMPT
        .replace("{project_instructions}", &project_instructions)
        .replace("{agent_mode_instructions}", &agent_mode_instructions)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_codex_prompt_contains_core_sections() {
        let workspace = PathBuf::from("/tmp/test-workspace");
        let prompt = build_codex_style_prompt(&workspace, AgentMode::Default, None);

        assert!(prompt.contains("Core Principles"));
        assert!(prompt.contains("Tool Usage"));
        assert!(prompt.contains("Sub-Agent Delegation"));
        assert!(prompt.contains("Task Planning"));
        assert!(prompt.contains("Git Operations"));
        assert!(prompt.contains("Project Instructions"));
    }

    #[test]
    fn test_codex_prompt_planning_mode() {
        let workspace = PathBuf::from("/tmp/test-workspace");
        let prompt = build_codex_style_prompt(&workspace, AgentMode::Planning, None);

        assert!(prompt.contains("<planning_mode>"));
        assert!(prompt.contains("Planning Mode Active"));
    }

    #[test]
    fn test_codex_prompt_auto_approve_mode() {
        let workspace = PathBuf::from("/tmp/test-workspace");
        let prompt = build_codex_style_prompt(&workspace, AgentMode::AutoApprove, None);

        assert!(prompt.contains("<autoapprove_mode>"));
        assert!(prompt.contains("AutoApprove Mode Active"));
    }

    #[test]
    fn test_codex_prompt_is_shorter_than_default() {
        use super::super::system_prompt::build_system_prompt;

        let workspace = PathBuf::from("/tmp/test-workspace");
        let codex_prompt = build_codex_style_prompt(&workspace, AgentMode::Default, None);
        let default_prompt = build_system_prompt(&workspace, AgentMode::Default, None);

        // Codex prompt should be more concise
        assert!(
            codex_prompt.len() < default_prompt.len(),
            "Codex prompt ({} chars) should be shorter than default prompt ({} chars)",
            codex_prompt.len(),
            default_prompt.len()
        );
    }
}
