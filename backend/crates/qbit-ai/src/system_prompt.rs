//! System prompt building for the Qbit agent.
//!
//! This module handles construction of the system prompt including:
//! - Environment information (workspace, date)
//! - Agent identity and workflow instructions
//! - Tool documentation
//! - Project-specific instructions from CLAUDE.md
//! - Agent mode-specific instructions
//! - Dynamic contributions from registered prompt contributors

use std::path::Path;

use chrono::Local;
use qbit_core::PromptContext;

use super::agent_mode::AgentMode;
use super::prompt_registry::PromptContributorRegistry;

/// Build the system prompt for the agent.
///
/// This is a convenience wrapper that calls `build_system_prompt_with_contributions`
/// without any contributors. Use this for backward compatibility or when dynamic
/// contributions are not needed.
///
/// # Arguments
/// * `workspace_path` - The current workspace directory
/// * `agent_mode` - The current agent mode (affects available operations)
/// * `memory_file_path` - Optional path to a memory file (from codebase settings)
///
/// # Returns
/// The complete system prompt string
pub fn build_system_prompt(
    workspace_path: &Path,
    agent_mode: AgentMode,
    memory_file_path: Option<&Path>,
) -> String {
    build_system_prompt_with_contributions(workspace_path, agent_mode, memory_file_path, None, None)
}

/// Build the system prompt with dynamic contributions from registered contributors.
///
/// This is the full version that supports dynamic prompt composition.
/// Use this when you have a PromptContributorRegistry available.
///
/// # Arguments
/// * `workspace_path` - The current workspace directory
/// * `agent_mode` - The current agent mode (affects available operations)
/// * `memory_file_path` - Optional path to a memory file (from codebase settings)
/// * `registry` - Optional registry of prompt contributors
/// * `context` - Optional context for prompt contribution (provider, model, tools, etc.)
///
/// # Returns
/// The complete system prompt string with all contributions appended
pub fn build_system_prompt_with_contributions(
    workspace_path: &Path,
    agent_mode: AgentMode,
    memory_file_path: Option<&Path>,
    registry: Option<&PromptContributorRegistry>,
    context: Option<&PromptContext>,
) -> String {
    let current_date = Local::now().format("%Y-%m-%d").to_string();

    // Read project instructions from memory file (if configured) or return empty
    let project_instructions = read_project_instructions(workspace_path, memory_file_path);

    // Add agent mode-specific instructions
    let agent_mode_instructions = get_agent_mode_instructions(agent_mode);

    let mut prompt = format!(
        r#"<identity>
You are Qbit, an intelligent software engineering assistant operating in a terminal environment.
You orchestrate development tasks by combining direct tool use with specialized sub-agent delegation.
</identity>

<environment>
Working Directory: {workspace}
Date: {date}
</environment>

<style>
- Direct answers. No preambles ("I'll help you...") or postambles ("Let me know if...")
- Concise explanations. Show reasoning only when it aids understanding
- Code over prose. When explaining changes, show the code
</style>

# Workflow

Execute tasks through five phases. Each phase has a gate—do not proceed until the gate condition is met.

## Phase 1: Investigate
Gather context before acting.

**Actions**:
- Read files mentioned in the request
- For unfamiliar code: delegate to `explorer` first
- Ask clarifying questions if requirements are ambiguous

⛔ **GATE**: Can you state specifically what needs to change and where? If no → continue investigating.

## Phase 2: Plan
Create a concrete action plan using `update_plan`.

**Actions**:
- Break work into discrete steps
- Identify files to modify
- Note verification commands (tests, lint, typecheck)

⛔ **GATE**: Does your plan include verification steps? If no → add them.

## Phase 3: Approve
For non-trivial changes, confirm the plan with the user.

**Skip approval when**:
- Single-line typo fixes
- User explicitly said "just do it" or similar
- AutoApprove mode is enabled

## Phase 4: Execute
Implement the plan using appropriate tools and sub-agents.

**Rules**:
- Update plan progress as you complete steps (`update_plan`)
- If a step fails, stop and report—do not continue blindly
- For multiple related edits to one file → use `coder` sub-agent

## Phase 5: Verify
<critical>
NEVER claim completion without verification. This phase is MANDATORY.
</critical>

**Actions**:
1. Run the project's lint/typecheck commands
2. Run relevant tests
3. If no tests exist for new code, note this to the user

⛔ **GATE**: Have you run verification AND reported results? If no → run verification.

---

# Tool Selection

## File Operations

| Need | Tool | Notes |
|------|------|-------|
| Read file content | `read_file` | Always read before editing |
| Targeted edit | `edit_file` | Preferred for existing files |
| Create new file | `create_file` | Fails if file exists (safety) |
| Overwrite entire file | `write_file` | Use sparingly, prefer `edit_file` |
| Search content | `grep_file` | Regex search across files |
| List files | `list_files` | Pattern matching |

<rule name="read-before-edit">
Before using `edit_file` or `write_file` on an existing file, you MUST read it first.
Edits without reading will fail or corrupt content.
</rule>

## Shell Commands

| Need | Tool |
|------|------|
| Single command | `run_command` |
| Multi-step pipeline | Delegate to `executor` |
| Long-running process | `run_command` (it handles PTY) |

## Web & Research

| Need | Tool |
|------|------|
| Quick lookup | `web_fetch` (if URL known) |
| Search query | `web_search` (if available) |
| Deep research | Delegate to `researcher` |

## Code Analysis

| Need | Tool |
|------|------|
| Symbol extraction | `indexer_extract_symbols` |
| Semantic analysis | `indexer_analyze_file` |
| Deep understanding | Delegate to `analyzer` |

---

# Delegation

## When to Delegate

| Situation | Delegate To |
|-----------|-------------|
| Unfamiliar codebase | `explorer` → then `analyzer` if needed |
| Multiple edits to same file | `coder` |
| Cross-module tracing | `explorer` |
| Architecture questions | `analyzer` |
| Multi-source research | `researcher` |
| Complex shell pipelines | `executor` |

## When to Handle Directly

- Single file you've already read in this conversation
- User provided exact file path AND exact change
- Trivial fixes (typos, formatting, one-line changes)
- Question answerable from current context

<rule name="explorer-first">
For unfamiliar code, ALWAYS start with `explorer` to map the codebase before diving into analysis or changes.
</rule>

---

<security>
# Security Boundaries

- NEVER expose secrets, API keys, or credentials in output
- NEVER commit credentials to version control
- NEVER generate code that logs sensitive data
- If you encounter secrets, note their presence but do not display them
</security>

---

<completion_checklist>
# Before Claiming Completion

✓ All planned steps completed (check `update_plan`)
✓ Verification commands executed (lint, typecheck, tests)
✓ Results of verification reported to user
✓ Any failures addressed or explicitly noted

If ANY item is unchecked, you are NOT done.
</completion_checklist>

## Project Instructions
{project_instructions}
{agent_mode_instructions}
"#,
        workspace = workspace_path.display(),
        date = current_date,
        project_instructions = project_instructions,
        agent_mode_instructions = agent_mode_instructions
    );

    // Append dynamic contributions from registered contributors
    if let (Some(registry), Some(ctx)) = (registry, context) {
        let contributions = registry.build_prompt(ctx);
        if !contributions.is_empty() {
            tracing::debug!(
                "Appending {} chars of dynamic prompt contributions",
                contributions.len()
            );
            prompt.push_str("\n\n");
            prompt.push_str(&contributions);
        }
    }

    prompt
}

/// Get agent mode-specific instructions to append to the system prompt.
fn get_agent_mode_instructions(mode: AgentMode) -> String {
    match mode {
        AgentMode::Planning => r#"

<planning_mode>
# Planning Mode Active

You are in READ-ONLY mode. You may investigate and plan, but NOT execute changes.

**Allowed**:
- `read_file`, `list_files`, `list_directory`, `grep_file`, `find_files`
- `indexer_*` tools (all analysis tools)
- `web_search`, `web_fetch` (research)
- `update_plan` (creating plans)
- Delegating to `explorer`, `analyzer`, `researcher`

**Forbidden**:
- `edit_file`, `write_file`, `create_file`, `delete_file`
- `run_command` (except read-only commands like `git status`, `ls`)
- `apply_patch`, `execute_code`
- Delegating to `coder`, `executor`

When you have a complete plan, present it and wait for the user to switch to execution mode.
</planning_mode>
"#
        .to_string(),
        AgentMode::AutoApprove => r#"

<autoapprove_mode>
# AutoApprove Mode Active

All tool operations will be automatically approved. Exercise additional caution:
- Double-check destructive operations (delete, overwrite)
- Verify you have the correct file paths
- Run verification after changes
</autoapprove_mode>
"#
        .to_string(),
        AgentMode::Default => String::new(),
    }
}

/// Read project instructions from a memory file.
///
/// # Arguments
/// * `workspace_path` - The current workspace directory
/// * `memory_file_path` - Optional explicit path to a memory file (from codebase settings)
///
/// # Behavior
/// - If `memory_file_path` is provided (from codebase settings), reads from that file.
///   If the file doesn't exist, returns an error message.
/// - If `memory_file_path` is None (no codebase configured or no memory file set),
///   returns empty string (no project instructions).
pub fn read_project_instructions(workspace_path: &Path, memory_file_path: Option<&Path>) -> String {
    // If a memory file path is configured, use it
    if let Some(path) = memory_file_path {
        // Handle relative paths (just filename like "CLAUDE.md")
        let full_path = if path.is_absolute() {
            path.to_path_buf()
        } else {
            workspace_path.join(path)
        };

        if full_path.exists() {
            match std::fs::read_to_string(&full_path) {
                Ok(contents) => return contents.trim().to_string(),
                Err(e) => {
                    tracing::warn!("Failed to read memory file {:?}: {}", full_path, e);
                    return format!(
                        "The {} memory file could not be read. Update in settings.",
                        path.display()
                    );
                }
            }
        } else {
            // Memory file configured but not found
            return format!(
                "The {} memory file not found. Update in settings.",
                path.display()
            );
        }
    }

    // No memory file configured - return empty (no project instructions)
    String::new()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_build_system_prompt_contains_required_sections() {
        let workspace = PathBuf::from("/tmp/test-workspace");
        let prompt = build_system_prompt(&workspace, AgentMode::Default, None);

        assert!(prompt.contains("<identity>"));
        assert!(prompt.contains("<environment>"));
        assert!(prompt.contains("<style>"));
        assert!(prompt.contains("# Workflow"));
        assert!(prompt.contains("# Tool Selection"));
        assert!(prompt.contains("# Delegation"));
        assert!(prompt.contains("<security>"));
        assert!(prompt.contains("<completion_checklist>"));
        assert!(prompt.contains("## Project Instructions"));
    }

    #[test]
    fn test_build_system_prompt_includes_workspace() {
        let workspace = PathBuf::from("/my/custom/workspace");
        let prompt = build_system_prompt(&workspace, AgentMode::Default, None);

        assert!(prompt.contains("/my/custom/workspace"));
    }

    #[test]
    fn test_build_system_prompt_planning_mode() {
        let workspace = PathBuf::from("/tmp/test-workspace");
        let prompt = build_system_prompt(&workspace, AgentMode::Planning, None);

        assert!(prompt.contains("<planning_mode>"));
        assert!(prompt.contains("Planning Mode Active"));
        assert!(prompt.contains("READ-ONLY mode"));
        assert!(prompt.contains("**Forbidden**"));
    }

    #[test]
    fn test_build_system_prompt_auto_approve_mode() {
        let workspace = PathBuf::from("/tmp/test-workspace");
        let prompt = build_system_prompt(&workspace, AgentMode::AutoApprove, None);

        assert!(prompt.contains("<autoapprove_mode>"));
        assert!(prompt.contains("AutoApprove Mode Active"));
    }

    #[test]
    fn test_read_project_instructions_returns_empty_when_no_memory_file() {
        let workspace = PathBuf::from("/nonexistent/path");
        let instructions = read_project_instructions(&workspace, None);

        assert!(instructions.is_empty());
    }

    #[test]
    fn test_read_project_instructions_returns_error_for_missing_configured_file() {
        let workspace = PathBuf::from("/tmp/test-workspace");
        let memory_file = PathBuf::from("NONEXISTENT.md");
        let instructions = read_project_instructions(&workspace, Some(&memory_file));

        assert!(instructions.contains("not found"));
        assert!(instructions.contains("NONEXISTENT.md"));
    }

    #[test]
    fn test_read_project_instructions_reads_configured_file() {
        // Create a temp directory with a memory file
        let temp_dir = std::env::temp_dir().join("qbit_test_memory_file");
        std::fs::create_dir_all(&temp_dir).unwrap();
        let memory_file_path = temp_dir.join("TEST_MEMORY.md");
        std::fs::write(&memory_file_path, "Test project instructions content").unwrap();

        let instructions = read_project_instructions(&temp_dir, Some(Path::new("TEST_MEMORY.md")));

        assert_eq!(instructions, "Test project instructions content");

        // Cleanup
        std::fs::remove_dir_all(&temp_dir).ok();
    }

    // =========================================================================
    // Integration tests for dynamic prompt composition
    // =========================================================================

    use crate::contributors::{ProviderBuiltinToolsContributor, SubAgentPromptContributor};
    use crate::prompt_registry::PromptContributorRegistry;
    use qbit_core::PromptContext;
    use qbit_sub_agents::{SubAgentDefinition, SubAgentRegistry};
    use std::sync::Arc;
    use tokio::sync::RwLock;

    fn create_test_sub_agent_registry() -> Arc<RwLock<SubAgentRegistry>> {
        let mut registry = SubAgentRegistry::new();
        registry.register(
            SubAgentDefinition::new(
                "code_analyzer",
                "Code Analyzer",
                "Deep semantic analysis of code structure and patterns",
                "You analyze code.",
            )
            .with_tools(vec!["read_file".to_string(), "grep_file".to_string()]),
        );
        registry.register(SubAgentDefinition::new(
            "code_writer",
            "Code Writer",
            "Implements code changes based on specifications",
            "You write code.",
        ));
        Arc::new(RwLock::new(registry))
    }

    #[test]
    fn test_prompt_composition_includes_sub_agent_docs() {
        let sub_agent_registry = create_test_sub_agent_registry();
        let mut registry = PromptContributorRegistry::new();
        registry.register(Arc::new(SubAgentPromptContributor::new(sub_agent_registry)));

        let ctx = PromptContext::new("anthropic", "claude-sonnet-4").with_sub_agents(true);

        let workspace = PathBuf::from("/tmp/test");
        let prompt = build_system_prompt_with_contributions(
            &workspace,
            AgentMode::Default,
            None,
            Some(&registry),
            Some(&ctx),
        );

        // Verify sub-agent documentation is included
        assert!(
            prompt.contains("## Available Sub-Agents"),
            "Prompt should contain sub-agent section header"
        );
        assert!(
            prompt.contains("### `code_analyzer`"),
            "Prompt should contain code_analyzer sub-agent"
        );
        assert!(
            prompt.contains("Deep semantic analysis"),
            "Prompt should contain sub-agent description"
        );
    }

    #[test]
    fn test_prompt_composition_excludes_sub_agents_when_disabled() {
        let sub_agent_registry = create_test_sub_agent_registry();
        let mut registry = PromptContributorRegistry::new();
        registry.register(Arc::new(SubAgentPromptContributor::new(sub_agent_registry)));

        // has_sub_agents = false
        let ctx = PromptContext::new("anthropic", "claude-sonnet-4").with_sub_agents(false);

        let workspace = PathBuf::from("/tmp/test");
        let prompt = build_system_prompt_with_contributions(
            &workspace,
            AgentMode::Default,
            None,
            Some(&registry),
            Some(&ctx),
        );

        // Verify sub-agent documentation is NOT included
        assert!(
            !prompt.contains("## Available Sub-Agents"),
            "Prompt should NOT contain sub-agent section when disabled"
        );
    }

    #[test]
    fn test_prompt_composition_includes_provider_instructions() {
        let mut registry = PromptContributorRegistry::new();
        registry.register(Arc::new(ProviderBuiltinToolsContributor));

        let ctx = PromptContext::new("anthropic", "claude-sonnet-4").with_web_search(true);

        let workspace = PathBuf::from("/tmp/test");
        let prompt = build_system_prompt_with_contributions(
            &workspace,
            AgentMode::Default,
            None,
            Some(&registry),
            Some(&ctx),
        );

        // Verify Anthropic-specific instructions are included
        assert!(
            prompt.contains("Anthropic Built-in"),
            "Prompt should contain Anthropic-specific web search instructions"
        );
    }

    #[test]
    fn test_prompt_composition_provider_specific_for_openai() {
        let mut registry = PromptContributorRegistry::new();
        registry.register(Arc::new(ProviderBuiltinToolsContributor));

        let ctx = PromptContext::new("openai", "gpt-4").with_web_search(true);

        let workspace = PathBuf::from("/tmp/test");
        let prompt = build_system_prompt_with_contributions(
            &workspace,
            AgentMode::Default,
            None,
            Some(&registry),
            Some(&ctx),
        );

        // Verify OpenAI-specific instructions are included
        assert!(
            prompt.contains("OpenAI Built-in"),
            "Prompt should contain OpenAI-specific instructions"
        );
        assert!(
            !prompt.contains("Anthropic Built-in"),
            "Prompt should NOT contain Anthropic instructions for OpenAI"
        );
    }

    #[test]
    fn test_prompt_composition_multiple_contributors() {
        let sub_agent_registry = create_test_sub_agent_registry();
        let mut registry = PromptContributorRegistry::new();
        registry.register(Arc::new(SubAgentPromptContributor::new(sub_agent_registry)));
        registry.register(Arc::new(ProviderBuiltinToolsContributor));

        let ctx = PromptContext::new("anthropic", "claude-sonnet-4")
            .with_sub_agents(true)
            .with_web_search(true);

        let workspace = PathBuf::from("/tmp/test");
        let prompt = build_system_prompt_with_contributions(
            &workspace,
            AgentMode::Default,
            None,
            Some(&registry),
            Some(&ctx),
        );

        // Verify BOTH contributors added their sections
        assert!(
            prompt.contains("## Available Sub-Agents"),
            "Prompt should contain sub-agent docs"
        );
        assert!(
            prompt.contains("Anthropic Built-in"),
            "Prompt should contain provider instructions"
        );

        // Verify ordering: sub-agents (Tools priority) before provider (Provider priority)
        let sub_agent_pos = prompt.find("## Available Sub-Agents").unwrap();
        let provider_pos = prompt.find("Anthropic Built-in").unwrap();
        assert!(
            sub_agent_pos < provider_pos,
            "Sub-agent docs (Tools priority) should come before provider instructions"
        );
    }

    #[test]
    fn test_prompt_composition_preserves_base_prompt() {
        let mut registry = PromptContributorRegistry::new();
        registry.register(Arc::new(ProviderBuiltinToolsContributor));

        let ctx = PromptContext::new("anthropic", "claude-sonnet-4").with_web_search(true);

        let workspace = PathBuf::from("/tmp/test");
        let prompt = build_system_prompt_with_contributions(
            &workspace,
            AgentMode::Default,
            None,
            Some(&registry),
            Some(&ctx),
        );

        // Verify base prompt sections are still present
        assert!(prompt.contains("You are Qbit"));
        assert!(prompt.contains("<environment>"));
        assert!(prompt.contains("# Workflow"));
        assert!(prompt.contains("# Delegation"));

        // Verify contributions come AFTER base prompt
        let workflow_pos = prompt.find("# Workflow").unwrap();
        let contribution_pos = prompt.find("Anthropic Built-in").unwrap();
        assert!(
            workflow_pos < contribution_pos,
            "Base prompt should come before contributions"
        );
    }

    #[test]
    fn test_prompt_composition_no_registry_same_as_base() {
        let workspace = PathBuf::from("/tmp/test");

        let base_prompt = build_system_prompt(&workspace, AgentMode::Default, None);
        let composed_prompt = build_system_prompt_with_contributions(
            &workspace,
            AgentMode::Default,
            None,
            None, // No registry
            None, // No context
        );

        assert_eq!(
            base_prompt, composed_prompt,
            "Without registry, composed prompt should equal base prompt"
        );
    }
}
