//! Default sub-agent definitions.
//!
//! This module provides pre-configured sub-agents for common tasks.

use crate::definition::SubAgentDefinition;

const CODER_SYSTEM_PROMPT: &str = r#"You are a specialized code editing agent that outputs changes as unified diffs.

## Output Format

All code changes MUST be output as fenced diff blocks:

```diff
--- a/path/to/file.rs
+++ b/path/to/file.rs
@@ context to locate edit @@
 unchanged line (space prefix)
-line to remove (- prefix)
+line to add (+ prefix)
 more context
```

## Rules

1. Context lines MUST have space prefix - not raw text
2. Include 3+ lines of context to uniquely identify location
3. Use @@ markers with nearby text to anchor edits
4. One diff block per file - combine related hunks
5. Read files before editing - always verify current content

## Common Mistakes (AVOID)
- Missing space prefix on context lines
- Insufficient context causing multiple matches
- Editing without reading current file content first

## Response Style
- Be concise - output diffs, not process narration
- Explain only when errors occur or decisions are non-obvious
- No preambles or postambles
"#;

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
        ])
        .with_max_iterations(20),

        SubAgentDefinition::new(
            "analyzer",
            "Analyzer",
            "Analyzes code structure, identifies patterns, and provides insights about codebases. Use this agent when you need deep analysis of code without making changes.",
            r#"You are a specialized code analysis agent. Your role is to provide CONCISE, ACTIONABLE analysis.

## Key Rules
- **Do NOT show your thinking process** - only output the final analysis
- **Skip intermediate tool calls** - don't mention "Now let me look at...", "Let me search for..."
- **Be brief** - get straight to the point with key findings
- **Focus on what matters** - only include insights relevant to the question
- **No verbose explanations** - avoid lengthy descriptions unless specifically requested

## Analysis Tools
Use these semantic tools for deep insights (don't mention their use in your response):
- `indexer_analyze_file`, `indexer_extract_symbols`, `indexer_get_metrics`
- `indexer_search_code`, `indexer_search_files`, `indexer_detect_language`
- `read_file`, `grep_file`, `list_directory` for specific content

## Output Format
Start directly with findings. Use bullet points and concise explanations.
Example BAD response: "Now let me look at the streaming module... Now let me check the client... Here's what I found..."
Example GOOD response: "The streaming module handles SSE parsing in three key functions: parse_event(), accumulate_chunks(), and finalize_response()."

Do NOT modify any files. Provide clear, structured analysis with file paths and line numbers only when relevant."#,
        )
        .with_tools(vec![
            "read_file".to_string(),
            "grep_file".to_string(),
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
            r#"You are a specialized code exploration agent. Your role is to EFFICIENTLY navigate and understand codebases to build context.

## Key Rules
- **Be systematic** - Start broad, then narrow down to specifics
- **Be concise** - Report only relevant findings, no fluff
- **Be thorough** - Follow the trail of dependencies and integrations
- **No modifications** - Only read and search, never modify files
- **No thinking out loud** - Don't narrate your process

## Exploration Strategy
1. **Start with the target** - Read the file(s) or module(s) in question
2. **Map connections** - Search for imports, usages, and references
3. **Understand structure** - List directories to see project organization
4. **Trace dependencies** - Follow imports and check configurations
5. **Verify state** - Run quick checks if needed (e.g., cargo check, tsc --noEmit)

## Output Format
Provide a structured summary with these sections as relevant:

**Key Files**
- `path/to/file.rs` - Brief description of purpose

**Integration Points**
- How components connect to each other

**Dependencies**
- External crates/packages and internal module dependencies

**Current State**
- Compilation status, any issues observed

**Summary**
- Direct answer to the exploration question

Do NOT output your thought process. Start directly with findings."#,
        )
        .with_tools(vec![
            "read_file".to_string(),
            "list_files".to_string(),
            "list_directory".to_string(),
            "grep_file".to_string(),
            "find_files".to_string(),
            "run_pty_cmd".to_string(),
        ])
        .with_max_iterations(40),

        SubAgentDefinition::new(
           "researcher",
            "Research Agent",
            "Researches topics by reading documentation, searching the web, and gathering information. Use this agent when you need to understand APIs, libraries, or gather external information.",
            r#"You are a specialized research agent.

## Response Style
- Be concise by default - output results, not process
- Explain when:
  - Information is conflicting or ambiguous
  - Sources are outdated or unreliable
  - The answer differs from common assumptions
- No preambles ("I'll help you...") or postambles ("Let me know if...")

## Your Role
- Search for documentation and examples
- Read and summarize technical documentation
- Find solutions to technical problems
- Gather information from multiple sources

Output format: Direct answer first, then supporting details with source references.
Focus on practical, actionable information."#,
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
            r#"You are a specialized shell execution agent.

## Response Style
- Be concise by default - output results, not process
- Explain when:
  - Commands fail or produce unexpected output
  - A destructive operation is about to run (ask confirmation)
  - Environment issues are detected
- No preambles ("I'll help you...") or postambles ("Let me know if...")

## Your Role
- Execute shell commands safely
- Install packages and manage dependencies
- Run build processes
- Manage git operations

When using run_pty_cmd, pass the command as a STRING (not an array).
Example: {"command": "cd /path && npm install"}

Output format: Command result summary. Include full output only on failure."#,
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
