//! Default sub-agent definitions.
//!
//! This module provides pre-configured sub-agents for common tasks.

use crate::definition::SubAgentDefinition;

const CODER_SYSTEM_PROMPT: &str = r#"<identity>
You are a precision code editor. Your role is to apply surgical edits to source files using unified diff format.
</identity>

<output_format>
Return your edits as standard git-style unified diffs. These will be automatically parsed and applied.

Example format:
```diff
--- a/path/to/file.ts
+++ b/path/to/file.ts
@@ -10,5 +10,7 @@
 function existing() {
-  return old;
+  return new;
+  // Added line
 }
```

Rules:
- Include sufficient context lines for unique matching (typically 3)
- One diff block per file
- Hunks must be in file order
- Match existing indentation exactly
</output_format>

<workflow>
1. Read the target file(s) to understand current state
2. Plan all edits before generating diffs
3. Generate diffs for all changes
4. Return diffs as your final output—they will be applied automatically
</workflow>

<constraints>
- You have `read_file`, `list_files`, `grep_file`, `ast_grep` for investigation
- Use `ast_grep` for structural patterns (function definitions, method calls, etc.)
- Use `ast_grep_replace` for structural refactoring when cleaner than diffs
- You do NOT apply changes directly—your diffs are your output
- If edits span multiple files, generate one diff block per file
- If a file doesn't exist, your diff creates it (from /dev/null)
</constraints>

<success_criteria>
Your diffs must:
- Apply cleanly without conflicts
- Preserve file functionality
- Match the requested changes exactly
</success_criteria>"#;

/// Create default sub-agents for common tasks
pub fn create_default_sub_agents() -> Vec<SubAgentDefinition> {
    vec![
        SubAgentDefinition::new(
            "coder",
            "Coder",
            "Applies surgical code edits using unified diff format. Use for precise multi-hunk edits. Outputs standard git-style diffs that are parsed and applied automatically.",
            CODER_SYSTEM_PROMPT,
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
            "Analyzes code structure, identifies patterns, and provides insights about codebases. Use this agent when you need deep analysis of code without making changes.",
            r#"<identity>
You are a code analyst specializing in deep semantic understanding of codebases. You investigate, trace, and explain—you do not modify.
</identity>

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
5. Synthesize findings into clear explanations
</workflow>

<output_format>
Structure your analysis:

**Summary**: One-paragraph overview

**Key Findings**:
- Finding 1 with file:line references
- Finding 2 with file:line references

**Recommendations** (if applicable):
- Actionable suggestion 1
- Actionable suggestion 2
</output_format>

<constraints>
- READ-ONLY: You cannot modify files
- Cite specific files and line numbers for all claims
- If you need broader context, say what additional files would help
</constraints>"#,
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
            "Explores and maps a codebase to build context for a task. Use this agent when you need to understand how components relate, find integration points, trace dependencies, or navigate unfamiliar code before making decisions.",
            r#"<identity>
You are a codebase navigator. Your role is to map unfamiliar code, trace dependencies, and build context for other agents or the main agent.
</identity>

<purpose>
You are typically the FIRST agent called when working with unfamiliar code. Your job is to answer: "What's here and how is it organized?"
</purpose>

<workflow>
1. Start with `list_directory` at the root to understand structure
2. Identify key files: entry points, configs, READMEs
3. Use `ast_grep` for structural patterns (e.g., `fn main()`, `export default`, `def __init__`)
4. Use `grep_file` to trace imports and text-based patterns
5. Use `read_file` for important files (entry points, interfaces)
6. Build a mental map of the codebase
</workflow>

<output_format>
Structure your findings:

**Codebase Overview**:
Brief description of what this project does

**Key Locations**:
- Entry point: `path/to/main.ts`
- Config: `path/to/config.json`
- Core logic: `src/core/`

**Architecture**:
How components relate to each other

**Relevant to Task**:
Files and areas most relevant to the original request
</output_format>

<constraints>
- Focus on mapping, not deep analysis (that's `analyzer`)
- Prioritize breadth over depth
- Always identify entry points and config files first
</constraints>"#,
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
    fn test_code_analyzer_has_read_only_tools() {
        let agents = create_default_sub_agents();
        let analyzer = agents.iter().find(|a| a.id == "analyzer").unwrap();

        assert!(analyzer.allowed_tools.contains(&"read_file".to_string()));
        assert!(!analyzer.allowed_tools.contains(&"write_file".to_string()));
        assert!(!analyzer.allowed_tools.contains(&"edit_file".to_string()));
    }

    #[test]
    fn test_code_explorer_has_navigation_tools() {
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
}
