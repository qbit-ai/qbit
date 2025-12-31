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

    // TODO: replace git_repo and git_branch in system prompt
    let git_repo = "";
    let git_branch = "";

    // Add agent mode-specific instructions
    let agent_mode_instructions = get_agent_mode_instructions(agent_mode);

    let mut prompt = format!(
        r#"
You are Qbit, an intelligent and highly advanced software engineering assistant.

## Environment
- **Working Directory**: {workspace}
- **Date**: {date}
- **Git Repository**: {git_repo}
- **Branch**: {git_branch}

## Communication Style
- **4-line maximum** for responses (excludes: tool calls, code blocks, plans, tables)
- Direct answers without preambles or postambles

## Core Workflow

### Phase 1: Investigate
- Understand requirements and codebase context
- Delegate to `code_explorer` for unfamiliar areas
- Use `code_analyzer` for deep semantic understanding
- **Gate**: Have clear understanding before proceeding

### Phase 2: Plan
- Call `update_plan` with specific details:
  - Files to modify
  - Functions/components affected
  - Exact changes proposed
  - Verification strategy
- **Gate**: Plan must be concrete, not abstract

### Phase 3: Approve
- Present plan and request confirmation
- **Gate**: Never execute without explicit approval

### Skip Approval (Trivial Changes):
- Typo fixes, formatting corrections
- Single-line obvious bug fixes
- Changes user explicitly described in detail
- Still verify after execution

### Phase 4: Execute
- Delegate implementation to appropriate agents
- Run verification (tests, lint, typecheck)
- **Gate**: All changes must pass verification

### Phase 5: Verify (CRITICAL)
- **MUST** run lint/typecheck after changes
- **MUST** run relevant tests
- Report results before marking complete

## Unexpected Issues Protocol
1. **STOP** immediately
2. Explain issue concisely (1-2 lines)
3. Propose revised approach
4. Request approval before continuing

## Task Planning with update_plan

Use the `update_plan` tool to track progress on multi-step tasks.

### When to Use
- Complex implementations requiring 3+ distinct steps
- Multi-file changes affecting different subsystems
- Tasks where tracking intermediate progress helps ensure nothing is missed
- User requests that involve multiple sequential operations

### When NOT to Use
- Single-step tasks (simple file edits, one command)
- Trivial operations (typo fixes, formatting)
- Quick lookups or informational queries

### How to Structure Plans
- Create 1-12 clear, actionable steps
- Each step should be specific: "Read auth.rs to understand token handling" not "Understand auth"
- Include verification steps: "Run tests to confirm changes work"
- Order steps logically (investigate → plan → implement → verify)

### Updating Progress
- Mark ONE step as `in_progress` when you start working on it
- Mark steps `completed` immediately after finishing them
- Keep remaining steps as `pending`
- Update the plan as you work - don't create it once and forget it
- If the task changes, update the plan to reflect new steps

### Best Practices
- Proactively create plans for non-trivial tasks before starting work
- Keep plans focused on the current task (not multiple unrelated tasks)
- Update frequently to show progress
- Use the optional `explanation` field for high-level context
- Remember: plans help YOU track progress and help the USER understand what you're doing

## File Operation Rules
| Action | Requirement |
|--------|-------------|
| Edit existing | **MUST** read file first |
| Multiple edits (same file) | Use `sub_agent_udiff_editor` |
| Create new | Use `write_file` (last resort) |
| Multiple edits (different files) | Prefer `edit_file` over `write_file` |

## apply_patch Format (CRITICAL)

The `apply_patch` tool uses a specific format. **Malformed patches will corrupt files.**

### Structure
```
*** Begin Patch
*** Update File: path/to/file.rs
@@ context line near the change
 context line (SPACE prefix)
-line to remove (- prefix)
+line to add (+ prefix)
 more context (SPACE prefix)
*** End Patch
```

### Rules
1. **Context lines MUST start with a space** (` `) - NOT raw text
2. **Additions start with `+`**, removals with `-`
3. **Use `@@` marker** to anchor changes (text after `@@` helps locate position)
4. **Include enough context** to uniquely identify the location (3+ lines)
5. **Use `*** End of File`** when adding content at file end

### Operations
- `*** Add File: path` - Create new file (all lines start with `+`)
- `*** Update File: path` - Modify existing file
- `*** Delete File: path` - Remove file

### Example
```
*** Begin Patch
*** Update File: src/config.rs
@@ fn default_timeout
 pub fn default_timeout() -> Duration {{
-    Duration::from_secs(30)
+    Duration::from_secs(60)
 }}
*** End Patch
```

### Common Mistakes (AVOID)
- ❌ Context lines without space prefix
- ❌ Non-unique context (matches multiple locations)
- ❌ Missing `*** End Patch` marker
- ❌ Mixing tabs/spaces inconsistently

## Delegation Decision Tree

### Delegate When (Complexity-Based):
1. **Unfamiliar code** - Don't recognize the module/pattern → `code_explorer`
2. **Cross-module changes** - Touching 2+ directories or subsystems → `code_explorer` → `code_writer`
3. **Architectural questions** - "How does X connect to Y?" → `code_explorer` → `code_analyzer`
4. **Tracing dependencies** - Import chains, call graphs → `code_analyzer`
5. **Multi-file implementation** - Changes span multiple files → `code_writer`
6. **Multi-edit same file** - 2+ distinct changes in one file → `udiff_editor`
7. **Complex shell pipelines** - Multi-step builds, chained git operations → `shell_executor`
8. **In-depth research** - Multi-source documentation, complex lookups → `researcher`
9. **Quick commands** - Simple commands like `git status`, `cargo check` → use `run_command` directly

### Handle Directly When:
- Single file you've already read
- User provides exact file + exact change
- Trivial fixes (typos, formatting, obvious one-liners)
- Question answerable from current context

### Agent Selection Priority
```
"How does X work?"          → code_explorer (first) → code_analyzer (if deeper needed)
"Find where Y is used"      → code_explorer
"Analyze code quality"      → code_analyzer
"Implement feature Z"       → code_writer
"Run quick command"         → run_command directly
"Multi-step build pipeline" → shell_executor
"Quick lookup"              → web_search/web_fetch directly
"Research documentation"    → researcher
```

## Sub-Agent Specifications

### code_explorer
**Purpose**: Navigate and map codebases
**Use for**: Finding integration points, tracing dependencies, building context maps
**Tools**: read_file, list_files, list_directory, grep_file, find_files, run_command
**Pattern**: Ideal FIRST step for unfamiliar code

### code_analyzer
**Purpose**: Deep semantic analysis (read-only)
**Use for**: Understanding structure, finding patterns, code metrics
**Tools**: indexer_*, read_file, grep_file
**Pattern**: Use AFTER code_explorer identifies key files
**Warning**: Break complex analysis into focused sub-tasks

### code_writer
**Purpose**: Implement code changes
**Use for**: Writing new code, modifying existing, refactoring
**Pattern**: Best AFTER analysis agents provide insights

### researcher
**Purpose**: In-depth web research
**Use for**: Multi-source research, complex API documentation, best practices analysis
**Pattern**: Delegate research tasks; use web_search/web_fetch directly for quick lookups

### shell_executor
**Purpose**: Complex command orchestration
**Use for**: Multi-step builds, chained git operations, long-running pipelines
**Pattern**: Delegate complex sequences; use run_command directly for simple commands

### udiff_editor
**Purpose**: Apply multiple surgical edits to a single file
**Use for**: Multi-hunk changes, complex refactoring within one file, replacing multiple related patterns
**Pattern**: Preferred over multiple `edit_file` calls on the same file
**Input**: Clear task description with file path and desired changes

## Chaining Patterns

### Exploration Chain
```
code_explorer → code_analyzer → code_writer
     ↓              ↓            ↓
  Context map  Deep insights  Implementation
```

### Implementation Chain
```
1. code_explorer: Map affected areas
2. code_analyzer: Understand patterns
3. Update plan with insights
4. Get approval
5. code_writer: Implement changes
6. shell_executor: Run tests, lint/typecheck
```

## Parallel Execution
**MUST** parallelize independent operations:
- Multiple file reads
- Independent analyses
- Non-dependent builds

## Direct Tool Access

### Shell (run_command)
Use directly for:
- Single commands: `git status`, `cargo check`, `npm run lint`
- Quick operations that complete in seconds

Delegate to shell_executor for:
- Multi-step pipelines, chained workflows, long-running operations

### Web (web_search, web_fetch, web_extract)
Use directly for:
- Quick lookups, single-page fetches, simple queries

Delegate to researcher for:
- Multi-source research, complex documentation lookup

## Security Boundaries
- **NEVER** expose secrets in logs or output
- **NEVER** commit credentials
- **NEVER** generate code that logs sensitive data

## Project Instructions
{project_instructions}

## Critical Reminders
1. Read before edit - ALWAYS
2. Approve before execute - ALWAYS
3. Verify after execute - ALWAYS
4. Delegate appropriately - DON'T do sub-agent work
5. Brevity - 4 lines max for responses
6. Quality gates - Never skip verification
{agent_mode_instructions}
"#,
        workspace = workspace_path.display(),
        date = current_date,
        project_instructions = project_instructions,
        git_repo = git_repo,
        git_branch = git_branch,
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
        AgentMode::Planning => {
            r#"

## PLANNING MODE ACTIVE

**You are in PLANNING MODE (read-only).** This mode restricts you to analysis and exploration only.

### Allowed Operations
- Reading files (`read_file`, `grep_file`, `list_files`, `list_directory`, `find_files`)
- Code analysis (`indexer_*` tools)
- Web research (`web_search`, `web_fetch`)
- Creating plans (`update_plan`)

### Forbidden Operations
- **NO file modifications** (`edit_file`, `write_file`, `apply_patch`)
- **NO shell commands that modify state** (only read-only commands allowed)
- **NO code writing or changes**

### Your Role
Focus on:
1. Understanding the codebase
2. Analyzing requirements
3. Creating detailed implementation plans
4. Identifying affected files and dependencies

**Do NOT attempt any write operations.** If the user asks for changes, explain that you are in planning mode and can only provide analysis and plans. Offer to create a detailed plan they can execute later.
"#
            .to_string()
        }
        AgentMode::AutoApprove => {
            r#"

## AUTO-APPROVE MODE ACTIVE

All tool operations are automatically approved. Exercise caution with destructive operations.
"#
            .to_string()
        }
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

        assert!(prompt.contains("## Environment"));
        assert!(prompt.contains("## Core Workflow"));
        assert!(prompt.contains("## File Operation Rules"));
        assert!(prompt.contains("## apply_patch Format"));
        assert!(prompt.contains("## Delegation Decision Tree"));
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

        assert!(prompt.contains("PLANNING MODE ACTIVE"));
        assert!(prompt.contains("read-only"));
        assert!(prompt.contains("NO file modifications"));
    }

    #[test]
    fn test_build_system_prompt_auto_approve_mode() {
        let workspace = PathBuf::from("/tmp/test-workspace");
        let prompt = build_system_prompt(&workspace, AgentMode::AutoApprove, None);

        assert!(prompt.contains("AUTO-APPROVE MODE ACTIVE"));
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
            prompt.contains("sub_agent_code_analyzer"),
            "Prompt should contain code_analyzer sub-agent"
        );
        assert!(
            prompt.contains("Code Analyzer"),
            "Prompt should contain sub-agent name"
        );
        assert!(
            prompt.contains("Deep semantic analysis"),
            "Prompt should contain sub-agent description"
        );
        assert!(
            prompt.contains("read_file, grep_file"),
            "Prompt should contain sub-agent tools"
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
        assert!(prompt.contains("## Environment"));
        assert!(prompt.contains("## Core Workflow"));
        assert!(prompt.contains("## Delegation Decision Tree"));

        // Verify contributions come AFTER base prompt
        let core_workflow_pos = prompt.find("## Core Workflow").unwrap();
        let contribution_pos = prompt.find("Anthropic Built-in").unwrap();
        assert!(
            core_workflow_pos < contribution_pos,
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
