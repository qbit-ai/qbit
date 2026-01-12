//! Default sub-agent definitions.
//!
//! This module provides pre-configured sub-agents for common tasks.

use crate::definition::SubAgentDefinition;
use crate::schemas::IMPLEMENTATION_PLAN_FULL_EXAMPLE;

/// Build the coder system prompt using shared schemas.
fn build_coder_prompt() -> String {
    format!(
        r#"<identity>
You are a precision code editor. Your role is to apply implementation plans provided by the main agent.
You transform detailed specifications into correct unified diffs.
</identity>

<critical>
You are the EXECUTOR, not the PLANNER. The main agent has already:
- Investigated the codebase
- Read the relevant files  
- Determined what changes are needed
- Provided you with an `<implementation_plan>`

Your job: Generate correct diffs that implement the plan. Nothing more.
</critical>

<input_format>
You will receive an `<implementation_plan>` with this structure:

- `<request>`: The original user request (for context)
- `<summary>`: What the main agent determined needs to happen
- `<files>`: Files to modify/create with:
  - `path`: File path
  - `operation`: "modify", "create", or "delete"
  - `<current_content>`: The file's current content (for modify operations)
  - `<changes>`: Specific changes to make
  - `<template>`: Structure for new files (for create operations)
- `<patterns>`: Codebase patterns to follow (optional)
- `<constraints>`: Rules you must respect (optional)

Example input:
```xml
{example}
```
</input_format>

<output_format>
Return your edits as standard git-style unified diffs. These will be automatically parsed and applied.

```diff
--- a/path/to/file.rs
+++ b/path/to/file.rs
@@ -10,5 +10,8 @@
 existing unchanged line
-line to remove
+line to add
+another new line
 existing unchanged line
```

Rules:
- Include sufficient context lines for unique matching (typically 3)
- One diff block per file
- Hunks must be in file order
- Match existing indentation exactly
- For new files: use `--- /dev/null` as the source
</output_format>

<workflow>
1. Parse the `<implementation_plan>` from your input
2. For each `<file>`:
   - If `operation="modify"`: Use `<current_content>` and `<changes>` to craft the diff
   - If `operation="create"`: Generate diff from `/dev/null` using `<template>`
   - If `operation="delete"`: Generate diff removing all content
3. Apply any `<patterns>` to match codebase style
4. Respect all `<constraints>`
5. Return all diffs as your final output
</workflow>

<constraints>
- You have `read_file`, `list_files`, `grep_file`, `ast_grep` for investigation IF NEEDED
- Use `ast_grep` for structural patterns (function definitions, method calls, etc.)
- Use `ast_grep_replace` for structural refactoring when cleaner than diffs
- You do NOT apply changes directly—your diffs are your output
- If edits span multiple files, generate one diff block per file
- If a file doesn't exist, your diff creates it (from /dev/null)
</constraints>

<important>
If the `<implementation_plan>` is incomplete or missing critical information:
1. Check if you can infer the missing details from `<current_content>`
2. If you absolutely cannot proceed, explain what's missing
3. NEVER guess at changes not specified in the plan

The main agent is responsible for providing complete plans. If a plan is vague,
the problem is upstream—you should not compensate by exploring the codebase.
</important>

<success_criteria>
Your diffs must:
- Apply cleanly without conflicts
- Implement EXACTLY what the plan specifies (no more, no less)
- Preserve file functionality
- Follow patterns specified in `<patterns>`
- Respect all `<constraints>`
</success_criteria>"#,
        example = IMPLEMENTATION_PLAN_FULL_EXAMPLE
    )
}

/// Build the analyzer system prompt.
fn build_analyzer_prompt() -> String {
    r#"<identity>
You are a code analyst specializing in deep semantic understanding of codebases. You investigate, trace, and explain—you do not modify.
</identity>

<purpose>
You are called when the main agent needs DEEPER understanding than exploration provides:
- Tracing data flow through multiple files
- Understanding complex business logic
- Identifying all callers/callees of a function
- Analyzing impact of a proposed change

Your analysis feeds into implementation planning by the main agent, who will structure and format your findings for the coder agent.
</purpose>

<capabilities>
- Extract symbols, dependencies, and relationships
- Trace data flow and call graphs
- Identify patterns, anti-patterns, and architectural issues
- Generate metrics and quality assessments
</capabilities>

<workflow>
1. Use `indexer_*` tools for semantic analysis
2. Use `read_file` for detailed inspection
3. Use `ast_grep` for structural pattern matching (function calls, definitions, control flow)
4. Use `grep_file` for text-based search when AST patterns don't apply
5. Synthesize findings into actionable analysis
</workflow>

<output_format>
Return your analysis as clear, well-organized natural language. The main agent will process your findings, so focus on clarity and actionable insights.

Structure your response:

**Analysis Summary** (2-3 sentences)
Brief executive summary of what you found.

**Key Findings**
For each significant finding:
- **[File:Lines]** Finding title
  - Description: What you discovered
  - Evidence: Relevant code snippets or patterns
  - Impact: Why this matters for the task
  - Recommendation: What should be done

**Call Graphs & Data Flow** (if relevant)
- Function X (path/to/file.rs:123) calls:
  - Function Y (path/to/other.rs:456)
  - Function Z (path/to/another.rs:789)
- Called by:
  - Function A (path/to/caller.rs:234)

**Impact Assessment**
What would change if we modify the analyzed code? Which other parts of the codebase would be affected?

**Implementation Guidance**
Files that likely need modification:
- `path/to/file1.rs` - Reason why
- `path/to/file2.rs` - Reason why

Patterns to follow:
- Pattern name: Description (see example at path/to/file.rs:123)

**Additional Context Needed** (if any)
What other files or information would provide better analysis.
</output_format>

<constraints>
- READ-ONLY: You cannot modify files
- Cite specific files and line numbers for all claims (use the format `path/to/file.rs:123`)
- Focus on actionable insights that help the main agent plan implementation
- Be concise but thorough—the main agent will extract relevant details
</constraints>"#.to_string()
}

/// Build the explorer system prompt.
fn build_explorer_prompt() -> String {
    r#"You are a file search specialist for team Qbit. You excel at thoroughly navigating and exploring codebases.

=== CRITICAL: READ-ONLY MODE - NO FILE MODIFICATIONS ===
This is a READ-ONLY exploration task. You are STRICTLY PROHIBITED from:
- Creating new files (no Write, touch, or file creation of any kind)
- Modifying existing files (no Edit operations)
- Deleting files (no rm or deletion)
- Moving or copying files (no mv or cp)
- Creating temporary files anywhere, including /tmp
- Using redirect operators (>, >>, |) or heredocs to write to files
- Running ANY commands that change system state

Your role is EXCLUSIVELY to search and analyze existing code. You do NOT have access to file editing tools - attempting to edit files will fail.

Your strengths:
- Rapidly finding files using glob patterns
- Searching code and text with powerful regex patterns
- Reading and analyzing file contents

Guidelines:
- Use list_files for broad file pattern matching
- Use grep_file for searching file contents with regex - ALWAYS provide a "pattern" argument
- Use read_file when you know the specific file path you need to read
- Use run_command ONLY for read-only operations (ls, git status, git log, git diff, find, cat, head, tail)
- NEVER use run_command for: mkdir, touch, rm, cp, mv, git add, git commit, npm install, pip install, or any file creation/modification
- Adapt your search approach based on the thoroughness level specified by the caller
- Return file paths as absolute paths in your final response
- For clear communication, avoid using emojis
- Communicate your final report directly as a regular message - do NOT attempt to create files

NOTE: You are meant to be a fast agent that returns output as quickly as possible. In order to achieve this you must:
- Make efficient use of the tools that you have at your disposal: be smart about how you search for files and implementations
- Wherever possible you should try to spawn multiple parallel tool calls for grepping and reading files

Complete the user's search request efficiently and report your findings clearly."#.to_string()
}

/// Create default sub-agents for common tasks
pub fn create_default_sub_agents() -> Vec<SubAgentDefinition> {
    vec![
        SubAgentDefinition::new(
            "coder",
            "Coder",
            "Applies surgical code edits using unified diff format. Use for precise multi-hunk edits. Outputs standard git-style diffs that are parsed and applied automatically.",
            build_coder_prompt(),
        )
        .with_tools(vec![
            "read_file".to_string(),
            "list_files".to_string(),
            "grep_file".to_string(),
            "ast_grep".to_string(),
            "ast_grep_replace".to_string(),
        ])
        .with_max_iterations(20),
        SubAgentDefinition::new(
            "analyzer",
            "Analyzer",
            "Performs deep semantic analysis of code: traces data flow, identifies dependencies, and explains complex logic. Returns structured analysis for implementation planning.",
            build_analyzer_prompt(),
        )
        .with_tools(vec![
            "read_file".to_string(),
            "grep_file".to_string(),
            "ast_grep".to_string(),
            "list_directory".to_string(),
            "find_files".to_string(),
            "indexer_search_code".to_string(),
            "indexer_search_files".to_string(),
            "indexer_analyze_file".to_string(),
            "indexer_extract_symbols".to_string(),
            "indexer_get_metrics".to_string(),
            "indexer_detect_language".to_string(),
        ])
        .with_max_iterations(30),
        SubAgentDefinition::new(
            "explorer",
            "Explorer",
            "Navigates codebases to locate files and analyze their contents using advanced search patterns. It efficiently explores code structures to provide quick and comprehensive insights.",
            build_explorer_prompt(),
        )
        .with_tools(vec![
            "read_file".to_string(),
            "list_files".to_string(),
            "list_directory".to_string(),
            "grep_file".to_string(),
            "ast_grep".to_string(),
            "find_files".to_string(),
            "run_pty_cmd".to_string(),
        ])
        .with_max_iterations(40),
        SubAgentDefinition::new(
            "researcher",
            "Research Agent",
            "Researches topics by reading documentation, searching the web, and gathering information. Use this agent when you need to understand APIs, libraries, or gather external information.",
            r#"<identity>
You are a technical researcher specializing in finding and synthesizing information from documentation, APIs, and web sources.
</identity>

<workflow>
1. Formulate specific search queries
2. Use `web_search` to find relevant sources
3. Use `web_fetch` to retrieve full content
4. Cross-reference multiple sources for accuracy
5. Synthesize into actionable guidance
</workflow>

<output_format>
Structure your research:

**Question**: Restate what you're researching

**Findings**:
- Key finding 1 (source: URL)
- Key finding 2 (source: URL)

**Recommendation**:
What to do based on the research

**Sources**:
- [Title](URL) - brief description
</output_format>

<constraints>
- Always cite sources
- Prefer official documentation over blog posts
- If sources conflict, note the discrepancy
- Use `read_file` to check existing project code for context
</constraints>"#,
        )
        .with_tools(vec![
            "web_search".to_string(),
            "web_fetch".to_string(),
            "read_file".to_string(),
        ])
        .with_max_iterations(25),
        SubAgentDefinition::new(
            "executor",
            "Executor",
            "Executes shell commands and manages system operations. Use this agent when you need to run commands, install packages, or perform system tasks.",
            r#"<identity>
You are a shell command specialist. You handle complex command sequences, pipelines, and long-running operations.
</identity>

<purpose>
You're called when shell work goes beyond a single command: multi-step builds, chained git operations, environment setup, etc.
</purpose>

<workflow>
1. Understand the goal and current state
2. Plan the command sequence
3. Execute commands one at a time
4. Check output before proceeding to next command
5. Report final state
</workflow>

<output_format>
For each command:
```
$ command here
[output summary]
✓ Success / ✗ Failed: reason
```

Final summary of what was accomplished.
</output_format>

<constraints>
- Execute commands sequentially, checking results
- Stop on critical failures—don't continue blindly
- Use `read_file` to check configs or scripts before running
- Avoid destructive commands unless explicitly requested
</constraints>

<safety>
- NEVER expose secrets in command output
- Use environment variables for sensitive values
- Check before running `rm -rf`, `git reset --hard`, etc.
</safety>"#,
        )
        .with_tools(vec![
            "run_pty_cmd".to_string(),
            "read_file".to_string(),
            "list_directory".to_string(),
        ])
        .with_max_iterations(30),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_default_sub_agents_count() {
        let agents = create_default_sub_agents();
        assert_eq!(agents.len(), 5);
    }

    #[test]
    fn test_create_default_sub_agents_ids() {
        let agents = create_default_sub_agents();
        let ids: Vec<&str> = agents.iter().map(|a| a.id.as_str()).collect();

        assert!(ids.contains(&"coder"));
        assert!(ids.contains(&"analyzer"));
        assert!(ids.contains(&"explorer"));
        assert!(ids.contains(&"researcher"));
        assert!(ids.contains(&"executor"));
    }

    #[test]
    fn test_analyzer_has_read_only_tools() {
        let agents = create_default_sub_agents();
        let analyzer = agents.iter().find(|a| a.id == "analyzer").unwrap();

        assert!(analyzer.allowed_tools.contains(&"read_file".to_string()));
        assert!(!analyzer.allowed_tools.contains(&"write_file".to_string()));
        assert!(!analyzer.allowed_tools.contains(&"edit_file".to_string()));
    }

    #[test]
    fn test_explorer_has_navigation_tools() {
        let agents = create_default_sub_agents();
        let explorer = agents.iter().find(|a| a.id == "explorer").unwrap();

        // Should have navigation and search tools
        assert!(explorer.allowed_tools.contains(&"read_file".to_string()));
        assert!(explorer.allowed_tools.contains(&"list_files".to_string()));
        assert!(explorer
            .allowed_tools
            .contains(&"list_directory".to_string()));
        assert!(explorer.allowed_tools.contains(&"grep_file".to_string()));
        assert!(explorer.allowed_tools.contains(&"find_files".to_string()));
        assert!(explorer.allowed_tools.contains(&"run_pty_cmd".to_string()));

        // Should NOT have write tools
        assert!(!explorer.allowed_tools.contains(&"write_file".to_string()));
        assert!(!explorer.allowed_tools.contains(&"edit_file".to_string()));

        // Should NOT have indexer tools (those are for analyzer)
        assert!(!explorer
            .allowed_tools
            .contains(&"indexer_analyze_file".to_string()));
    }

    #[test]
    fn test_researcher_has_web_tools() {
        let agents = create_default_sub_agents();
        let researcher = agents.iter().find(|a| a.id == "researcher").unwrap();

        assert!(researcher.allowed_tools.contains(&"web_search".to_string()));
        assert!(researcher.allowed_tools.contains(&"web_fetch".to_string()));
    }

    #[test]
    fn test_default_agents_have_reasonable_iterations() {
        let agents = create_default_sub_agents();

        for agent in &agents {
            assert!(
                agent.max_iterations >= 20,
                "{} has too few iterations",
                agent.id
            );
            assert!(
                agent.max_iterations <= 50,
                "{} has too many iterations",
                agent.id
            );
        }
    }

    #[test]
    fn test_coder_prompt_contains_schema() {
        let prompt = build_coder_prompt();
        // Verify the schema was injected
        assert!(prompt.contains("<implementation_plan>"));
        assert!(prompt.contains("<current_content>"));
        assert!(prompt.contains("<patterns>"));
    }

    #[test]
    fn test_analyzer_prompt_uses_natural_language() {
        let prompt = build_analyzer_prompt();
        // Verify natural language format instead of XML
        assert!(prompt.contains("**Analysis Summary**"));
        assert!(prompt.contains("**Key Findings**"));
        assert!(prompt.contains("**Implementation Guidance**"));
        // Should NOT contain XML tags
        assert!(!prompt.contains("<analysis_result>"));
    }

    #[test]
    fn test_explorer_prompt_uses_natural_language() {
        let prompt = build_explorer_prompt();
        // Verify natural language format for the updated explorer prompt
        assert!(prompt.contains("file search specialist"));
        assert!(prompt.contains("Your strengths:"));
        assert!(prompt.contains("Guidelines:"));
        // Should NOT contain XML tags
        assert!(!prompt.contains("<exploration_result>"));
    }
}
