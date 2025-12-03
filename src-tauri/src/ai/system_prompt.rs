//! System prompt building for the Qbit agent.
//!
//! This module handles construction of the system prompt including:
//! - Environment information (workspace, date)
//! - Agent identity and workflow instructions
//! - Tool documentation
//! - Project-specific instructions from CLAUDE.md

use std::path::Path;

use chrono::Local;

/// Build the system prompt for the agent.
///
/// # Arguments
/// * `workspace_path` - The current workspace directory
///
/// # Returns
/// The complete system prompt string
pub fn build_system_prompt(workspace_path: &Path) -> String {
    let current_date = Local::now().format("%Y-%m-%d").to_string();

    // Try to read CLAUDE.md from the workspace
    let project_instructions = read_project_instructions(workspace_path);

    // TODO: replace git_repo and git_branch in system prompt
    let git_repo = "";
    let git_branch = "";

    format!(
        r#"
# Qbit Agent Prompt (Optimized)

```xml
<environment>
Working Directory: {workspace}
Current Date: {date}
Git Repo: {git_repo}
Current branch: {git_branch}
</environment>

<workflow>
1. Investigate - Understand codebase and requirements
2. Plan - Use `update_plan` with specific details (files, functions, changes)
3. Approve - Ask for explicit confirmation before proceeding
4. Execute - Make approved changes
→ If unexpected issues arise: STOP, explain, revise plan, get approval
</workflow>

<rules>
- Always read files before editing
- Use `edit_file` for existing files, `write_file` for new files
- Never change without explicit approval
- Parallelize independent tasks
- Delegate to sub-agents for specialized work
</rules>

<sub_agents>
Specialized sub-agents for delegating complex tasks:

**code_analyzer** - Deep semantic analysis of code (read-only, uses indexer tools)
→ Use for: Understanding code structure, finding patterns, identifying dependencies, code metrics
→ Ideal when you need detailed insights before making decisions
→ IMPORTANT: For complex analysis tasks, break them into focused sub-tasks:
   1. Ask about specific file structure/patterns first
   2. Then ask about relationships/dependencies
   3. Then ask about implementation details
   This prevents "prompt too long" errors and gets better results
→ Example: Instead of asking to analyze the entire AI provider system in one task,
   ask: "Analyze the rig-anthropic-vertex crate structure" first, then follow up with
   "Explain how providers are initialized in llm_client.rs", etc.

**code_writer** - Implements code changes based on analysis (has apply_patch and indexer tools)
→ Use for: Writing new code, modifying existing code, refactoring with understanding
→ Best used after code_analyzer has provided insights

**coder** - Complex code changes requiring analysis before implementation (refactoring, architectural alignment)
→ Orchestrates code_analyzer and code_writer internally; ideal for comprehensive code work

**researcher** - Web research, documentation fetching, information gathering
→ Don't use web_fetch/web_search directly; delegate to this agent

**shell_executor** - Command execution, builds, dependencies, git operations
→ Don't use run_pty_cmd directly; delegate to this agent

Pass clear task descriptions when delegating. Each has specialized tools and iteration limits.
Note: code_analyzer and code_writer are typically orchestrated by coder, but available separately for specific needs.
</sub_agents>

<context_handling>
User messages may include `<context>` with `<cwd>` indicating current terminal directory for relative path operations.
</context_handling>

<project_instructions>
{project_instructions}
</project_instructions>
```
"#,
        workspace = workspace_path.display(),
        date = current_date,
        project_instructions = project_instructions,
        git_repo = git_repo,
        git_branch = git_branch
    )
}

/// Read project instructions from CLAUDE.md if it exists.
///
/// Checks both the workspace directory and its parent directory.
pub fn read_project_instructions(workspace_path: &Path) -> String {
    let claude_md_path = workspace_path.join("CLAUDE.md");
    if claude_md_path.exists() {
        if let Ok(contents) = std::fs::read_to_string(&claude_md_path) {
            return contents.trim().to_string();
        }
    }

    // Also check parent directory (in case we're in src-tauri)
    if let Some(parent) = workspace_path.parent() {
        let parent_claude_md = parent.join("CLAUDE.md");
        if parent_claude_md.exists() {
            if let Ok(contents) = std::fs::read_to_string(&parent_claude_md) {
                return contents.trim().to_string();
            }
        }
    }

    String::new()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_build_system_prompt_contains_required_sections() {
        let workspace = PathBuf::from("/tmp/test-workspace");
        let prompt = build_system_prompt(&workspace);

        assert!(prompt.contains("<environment>"));
        assert!(prompt.contains("<identity>"));
        assert!(prompt.contains("<workflow>"));
        assert!(prompt.contains("<rules>"));
        assert!(prompt.contains("<context_handling>"));
        assert!(prompt.contains("<project_instructions>"));
    }

    #[test]
    fn test_build_system_prompt_includes_workspace() {
        let workspace = PathBuf::from("/my/custom/workspace");
        let prompt = build_system_prompt(&workspace);

        assert!(prompt.contains("/my/custom/workspace"));
    }

    #[test]
    fn test_read_project_instructions_returns_empty_for_missing_file() {
        let workspace = PathBuf::from("/nonexistent/path");
        let instructions = read_project_instructions(&workspace);

        assert!(instructions.is_empty());
    }
}
