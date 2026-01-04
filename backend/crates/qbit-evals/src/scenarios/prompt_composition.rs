//! Prompt composition evaluation scenarios.
//!
//! Tests that dynamic system prompt composition affects agent behavior correctly.
//! These scenarios verify that:
//! - Custom instructions in the system prompt are followed
//! - Different prompt configurations produce different behaviors
//! - The prompt composition system works end-to-end

use async_trait::async_trait;

use crate::metrics::{FileStateMetric, LlmJudgeMetric, Metric};
use crate::scenarios::Scenario;

// =============================================================================
// Scenario 1: Custom Output Format Instructions
// =============================================================================

/// Tests that custom formatting instructions in the system prompt are followed.
///
/// This scenario gives the agent a system prompt that requires JSON output format,
/// then verifies the agent produces JSON.
pub struct OutputFormatScenario;

const OUTPUT_FORMAT_SYSTEM_PROMPT: &str = r#"You are a coding assistant being evaluated.

CRITICAL REQUIREMENT: All your responses MUST be valid JSON objects.
Structure: {"thought": "your reasoning", "action": "what you did", "result": "outcome"}

You have access to file tools (read_file, write_file, etc.) and shell commands (run_pty_cmd).
Complete the task, but format ALL responses as JSON objects.
"#;

#[async_trait]
impl Scenario for OutputFormatScenario {
    fn name(&self) -> &str {
        "prompt-output-format"
    }

    fn description(&self) -> &str {
        "Tests that custom output format instructions are followed"
    }

    fn testbed(&self) -> &str {
        "rust-prompt-test"
    }

    fn prompt(&self) -> &str {
        "Read the file src/lib.rs and describe what it contains."
    }

    fn system_prompt(&self) -> Option<&str> {
        Some(OUTPUT_FORMAT_SYSTEM_PROMPT)
    }

    fn metrics(&self) -> Vec<Box<dyn Metric>> {
        vec![
            Box::new(LlmJudgeMetric::new(
                "follows_json_format",
                "The agent's final response should be structured as JSON or clearly attempt \
                 JSON formatting. Look for curly braces, key-value pairs, or JSON-like structure.",
                0.7,
            )),
            Box::new(LlmJudgeMetric::new(
                "completes_task",
                "The agent should successfully read and describe the contents of src/lib.rs, \
                 regardless of output format.",
                0.7,
            )),
        ]
    }
}

// =============================================================================
// Scenario 2: Coding Convention Instructions
// =============================================================================

/// Tests that coding convention instructions affect code generation.
///
/// This scenario gives the agent specific coding conventions and verifies
/// generated code follows them.
pub struct CodingConventionsScenario;

const CODING_CONVENTIONS_SYSTEM_PROMPT: &str = r#"You are a coding assistant being evaluated.

CODING CONVENTIONS (MUST FOLLOW):
1. All function names MUST use snake_case
2. All struct names MUST use PascalCase
3. Every public function MUST have a doc comment starting with "///"
4. All functions MUST have explicit return types

You have access to file tools (read_file, write_file, edit_file, create_file) and shell.
Complete the task following these conventions strictly.
"#;

#[async_trait]
impl Scenario for CodingConventionsScenario {
    fn name(&self) -> &str {
        "prompt-coding-conventions"
    }

    fn description(&self) -> &str {
        "Tests that coding convention instructions are followed in generated code"
    }

    fn testbed(&self) -> &str {
        "rust-prompt-test"
    }

    fn prompt(&self) -> &str {
        "Add a new public function called 'calculate_total' to src/lib.rs that takes \
         a Vec<i32> and returns the sum. Follow all coding conventions."
    }

    fn system_prompt(&self) -> Option<&str> {
        Some(CODING_CONVENTIONS_SYSTEM_PROMPT)
    }

    fn metrics(&self) -> Vec<Box<dyn Metric>> {
        vec![
            Box::new(FileStateMetric::modified(
                "file_was_modified",
                "src/lib.rs",
            )),
            Box::new(FileStateMetric::contains(
                "has_snake_case_function",
                "src/lib.rs",
                "fn calculate_total",
            )),
            Box::new(FileStateMetric::contains(
                "has_doc_comment",
                "src/lib.rs",
                "///",
            )),
            Box::new(
                LlmJudgeMetric::new(
                    "follows_conventions",
                    "The added function should follow all specified conventions: \
                     snake_case name (calculate_total), doc comment (///), explicit return type (-> i32 or similar). \
                     Use the read_file tool to check src/lib.rs.",
                    0.7,
                )
                .with_tools(),
            ),
        ]
    }
}

// =============================================================================
// Scenario 3: Tool Usage Instructions
// =============================================================================

/// Tests that tool usage instructions affect which tools the agent uses.
///
/// This scenario instructs the agent to prefer certain tools and verifies
/// the agent follows those preferences.
pub struct ToolPreferenceScenario;

const TOOL_PREFERENCE_SYSTEM_PROMPT: &str = r#"You are a coding assistant being evaluated.

TOOL PREFERENCES:
- ALWAYS use grep_file to search before reading entire files
- PREFER edit_file over write_file for modifications
- ALWAYS run tests after making changes using: run_pty_cmd with "cargo test"

You have access to: read_file, write_file, edit_file, grep_file, run_pty_cmd, list_files.
Follow the tool preferences strictly.
"#;

#[async_trait]
impl Scenario for ToolPreferenceScenario {
    fn name(&self) -> &str {
        "prompt-tool-preference"
    }

    fn description(&self) -> &str {
        "Tests that tool preference instructions affect tool selection"
    }

    fn testbed(&self) -> &str {
        "rust-prompt-test"
    }

    fn prompt(&self) -> &str {
        "Find where 'greet' is defined in the codebase and add a new parameter 'formal: bool' \
         that changes the greeting to 'Good day' when true. Make sure tests pass."
    }

    fn system_prompt(&self) -> Option<&str> {
        Some(TOOL_PREFERENCE_SYSTEM_PROMPT)
    }

    fn metrics(&self) -> Vec<Box<dyn Metric>> {
        vec![
            Box::new(LlmJudgeMetric::new(
                "used_grep_for_search",
                "The agent should have used grep_file to find where 'greet' is defined \
                 before reading files. Check the tool calls in the conversation.",
                0.7,
            )),
            Box::new(LlmJudgeMetric::new(
                "used_edit_not_write",
                "The agent should have used edit_file (not write_file) to modify the code. \
                 Check which tools were called for the modification.",
                0.7,
            )),
            Box::new(LlmJudgeMetric::new(
                "ran_tests",
                "The agent should have run 'cargo test' after making changes. \
                 Check for run_pty_cmd with cargo test.",
                0.7,
            )),
        ]
    }
}

// =============================================================================
// Scenario 4: Compare With/Without Instructions
// =============================================================================

/// Tests behavior difference when specific instructions are present vs absent.
///
/// This scenario uses a system prompt with brevity instructions and verifies
/// the response is concise.
pub struct BrevityInstructionScenario;

const BREVITY_SYSTEM_PROMPT: &str = r#"You are a coding assistant being evaluated.

RESPONSE STYLE:
- Maximum 3 sentences per response
- No preambles or postambles
- Direct, actionable answers only
- Skip explaining what you're about to do - just do it

You have access to file and shell tools. Be extremely concise.
"#;

#[async_trait]
impl Scenario for BrevityInstructionScenario {
    fn name(&self) -> &str {
        "prompt-brevity"
    }

    fn description(&self) -> &str {
        "Tests that brevity instructions result in concise responses"
    }

    fn testbed(&self) -> &str {
        "rust-prompt-test"
    }

    fn prompt(&self) -> &str {
        "What does the greet function in src/lib.rs do?"
    }

    fn system_prompt(&self) -> Option<&str> {
        Some(BREVITY_SYSTEM_PROMPT)
    }

    fn metrics(&self) -> Vec<Box<dyn Metric>> {
        vec![
            Box::new(LlmJudgeMetric::new(
                "response_is_brief",
                "The agent's final response (excluding tool calls) should be very concise - \
                 ideally 3 sentences or fewer. Long explanations or preambles indicate failure \
                 to follow brevity instructions.",
                0.7,
            )),
            Box::new(LlmJudgeMetric::new(
                "no_preamble",
                "The response should NOT contain preambles like 'Sure, I'll help you with that' \
                 or 'Let me explain' or 'I'll now look at the file'. It should be direct.",
                0.7,
            )),
        ]
    }
}

// =============================================================================
// Scenario 5: A/B Comparison - With vs Without Instructions
// =============================================================================

/// Tests that behavior differs when instructions are present vs absent.
///
/// This scenario uses the DEFAULT eval prompt (no custom instructions) and
/// should produce different behavior than BrevityInstructionScenario.
pub struct NoInstructionsBaselineScenario;

#[async_trait]
impl Scenario for NoInstructionsBaselineScenario {
    fn name(&self) -> &str {
        "prompt-no-instructions-baseline"
    }

    fn description(&self) -> &str {
        "Baseline: same task as brevity scenario but with default prompt"
    }

    fn testbed(&self) -> &str {
        "rust-prompt-test"
    }

    fn prompt(&self) -> &str {
        // Same prompt as BrevityInstructionScenario
        "What does the greet function in src/lib.rs do?"
    }

    // No custom system_prompt - uses default

    fn metrics(&self) -> Vec<Box<dyn Metric>> {
        vec![
            Box::new(LlmJudgeMetric::new(
                "response_is_typical_length",
                "The response should be a typical AI response - likely longer and more \
                 explanatory than a heavily constrained response. It may include context, \
                 explanation of the code structure, etc.",
                0.7,
            )),
            Box::new(LlmJudgeMetric::new(
                "task_completed",
                "The agent should successfully explain what the greet function does.",
                0.7,
            )),
        ]
    }
}

// =============================================================================
// Scenario 6: Sub-Agent Awareness
// =============================================================================

/// Tests that the agent acknowledges sub-agent capabilities when documented.
///
/// This scenario includes sub-agent documentation and asks a question that
/// could benefit from delegation, verifying the agent is aware of the option.
pub struct SubAgentAwarenessScenario;

const SUB_AGENT_AWARE_SYSTEM_PROMPT: &str = r#"You are a coding assistant being evaluated.

## Available Sub-Agents

You can delegate tasks to specialized sub-agents:

### sub_agent_code_analyzer
**Code Analyzer**: Deep semantic analysis of code structure, patterns, and dependencies.
Available tools: read_file, grep_file, indexer tools

### sub_agent_code_writer
**Code Writer**: Implements code changes based on specifications.
Available tools: read_file, write_file, edit_file

When a task would benefit from specialized analysis or implementation,
mention which sub-agent would be appropriate (even if you handle it directly).

You have access to: read_file, write_file, edit_file, grep_file, run_pty_cmd.
"#;

#[async_trait]
impl Scenario for SubAgentAwarenessScenario {
    fn name(&self) -> &str {
        "prompt-sub-agent-awareness"
    }

    fn description(&self) -> &str {
        "Tests that agent acknowledges sub-agents when documented in prompt"
    }

    fn testbed(&self) -> &str {
        "rust-prompt-test"
    }

    fn prompt(&self) -> &str {
        "I need to understand how the greet function works and then add a new function \
         that greets multiple people. Based on the capabilities described in your system prompt, \
         how would you approach this task? What sub-agents or specialized tools could help?"
    }

    fn system_prompt(&self) -> Option<&str> {
        Some(SUB_AGENT_AWARE_SYSTEM_PROMPT)
    }

    fn metrics(&self) -> Vec<Box<dyn Metric>> {
        vec![
            Box::new(LlmJudgeMetric::new(
                "mentions_sub_agents",
                "The agent should mention or reference sub-agents (code_analyzer, code_writer) \
                 as options for the task, even if it handles it directly. Look for mentions of \
                 'sub_agent', 'code_analyzer', 'code_writer', or 'delegate'.",
                0.7,
            )),
            // Note: We only test that sub-agents are mentioned, not the exact mapping.
            // The core test is whether the prompt composition system successfully
            // delivers sub-agent information to the agent's context.
        ]
    }
}

// =============================================================================
// Scenario 7: Provider Context Awareness
// =============================================================================

/// Tests that the agent uses provider-specific context from the prompt.
///
/// This scenario includes provider information and asks about capabilities.
pub struct ProviderContextScenario;

const PROVIDER_CONTEXT_SYSTEM_PROMPT: &str = r#"You are a coding assistant being evaluated.

## Environment
- Provider: Anthropic Claude
- Model: claude-sonnet-4
- Workspace: /test/project

## Provider-Specific Features
- Web search is available via the web_search tool
- Extended thinking is enabled for complex reasoning
- This model excels at code analysis and generation

When asked about your capabilities, reference these provider-specific features.

You have access to: read_file, write_file, edit_file, grep_file, run_pty_cmd, web_search.
"#;

#[async_trait]
impl Scenario for ProviderContextScenario {
    fn name(&self) -> &str {
        "prompt-provider-context"
    }

    fn description(&self) -> &str {
        "Tests that agent uses provider context from the prompt"
    }

    fn testbed(&self) -> &str {
        "rust-prompt-test"
    }

    fn prompt(&self) -> &str {
        "According to your system prompt, what tools and provider-specific capabilities \
         do you have available? Please list them, including any special features like \
         web search or extended thinking. I want to understand a codebase and then make changes."
    }

    fn system_prompt(&self) -> Option<&str> {
        Some(PROVIDER_CONTEXT_SYSTEM_PROMPT)
    }

    fn metrics(&self) -> Vec<Box<dyn Metric>> {
        vec![
            Box::new(LlmJudgeMetric::new(
                "mentions_web_search",
                "The agent should mention web_search as an available capability.",
                0.7,
            )),
            Box::new(LlmJudgeMetric::new(
                "mentions_provider_features",
                "The agent should reference provider-specific features like extended thinking \
                 or code analysis capabilities mentioned in the prompt.",
                0.6,
            )),
        ]
    }
}

// =============================================================================
// Scenario 8: Instruction Specificity
// =============================================================================

/// Tests that specific instructions override general behavior.
///
/// This scenario provides very specific file naming conventions and verifies
/// the agent follows them exactly.
pub struct SpecificInstructionsScenario;

const SPECIFIC_INSTRUCTIONS_SYSTEM_PROMPT: &str = r#"You are a coding assistant being evaluated.

## MANDATORY FILE NAMING CONVENTION
When creating new files, you MUST follow this EXACT pattern:
- All new Rust files MUST be named with the prefix "qbit_"
- Example: qbit_helpers.rs, qbit_utils.rs, qbit_config.rs
- This is a hard requirement - files without this prefix will be rejected

You have access to: read_file, write_file, create_file, edit_file, list_files, run_pty_cmd.
"#;

#[async_trait]
impl Scenario for SpecificInstructionsScenario {
    fn name(&self) -> &str {
        "prompt-specific-instructions"
    }

    fn description(&self) -> &str {
        "Tests that specific naming instructions are followed exactly"
    }

    fn testbed(&self) -> &str {
        "rust-prompt-test"
    }

    fn prompt(&self) -> &str {
        "Create a new Rust file with helper functions for string manipulation. \
         Add a function to reverse a string and another to count vowels."
    }

    fn system_prompt(&self) -> Option<&str> {
        Some(SPECIFIC_INSTRUCTIONS_SYSTEM_PROMPT)
    }

    fn metrics(&self) -> Vec<Box<dyn Metric>> {
        vec![
            Box::new(
                LlmJudgeMetric::new(
                    "follows_naming_convention",
                    "Any new file created should follow the qbit_ prefix convention. \
                     Use list_files to check the src/ directory and verify a file like \
                     'qbit_helpers.rs' or 'qbit_string.rs' was created (not 'helpers.rs').",
                    0.8,
                )
                .with_tools(),
            ),
            Box::new(
                LlmJudgeMetric::new(
                    "creates_requested_functions",
                    "The agent should create the requested functions (reverse string, count vowels). \
                     Use read_file to check the actual file content.",
                    0.7,
                )
                .with_tools(),
            ),
        ]
    }
}

// =============================================================================
// Testbed Files
// =============================================================================

/// Testbed files for prompt composition scenarios.
pub fn testbed_files() -> Vec<(String, String)> {
    vec![
        (
            "Cargo.toml".to_string(),
            r#"[package]
name = "prompt-test"
version = "0.1.0"
edition = "2021"

[dependencies]
"#
            .to_string(),
        ),
        (
            "src/lib.rs".to_string(),
            r#"/// A simple greeting module.

/// Returns a greeting for the given name.
pub fn greet(name: &str) -> String {
    format!("Hello, {}!", name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_greet() {
        assert_eq!(greet("World"), "Hello, World!");
    }

    #[test]
    fn test_greet_name() {
        assert_eq!(greet("Alice"), "Hello, Alice!");
    }
}
"#
            .to_string(),
        ),
    ]
}
