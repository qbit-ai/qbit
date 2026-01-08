//! Shared XML schemas for agent handoffs.
//!
//! These schemas define the structured formats used for communication between
//! the main agent and the coder agent.
//!
//! **Important**: The Explorer and Analyzer agents do NOT use these XML schemas.
//! They return natural language reports that the main agent processes and formats
//! into XML when preparing handoffs to the Coder agent.
//!
//! Only the following agents use XML:
//! - **Main Agent** → **Coder**: Uses `IMPLEMENTATION_PLAN_SCHEMA`
//! - **Coder**: Receives `<implementation_plan>` XML input, outputs unified diffs
//!
//! The separation ensures that research agents (Explorer, Analyzer) can focus on
//! clear, flexible natural language reporting while the implementation handoff
//! remains structured and parseable.

/// Schema description for `<implementation_plan>` - what fields exist and their purpose.
/// Used by both main agent (output) and coder (input).
pub const IMPLEMENTATION_PLAN_SCHEMA: &str = r#"<implementation_plan>
  <request>
    <!-- The original user request, for context -->
  </request>
  
  <summary>
    <!-- 1-2 sentence description of what needs to happen -->
  </summary>
  
  <files>
    <file operation="modify|create|delete" path="path/to/file">
      <current_content>
        <!-- For modify: relevant portions of the file (include ~50 lines context for targeted edits) -->
      </current_content>
      <changes>
        <!-- Specific changes: what function, what line range, what transformation -->
      </changes>
      <template>
        <!-- For create: skeleton or pattern for new file -->
      </template>
    </file>
  </files>
  
  <patterns>
    <!-- Codebase patterns the coder should follow -->
    <pattern name="pattern name">
      Description or example location
    </pattern>
  </patterns>
  
  <constraints>
    <!-- Rules the coder must respect -->
  </constraints>
</implementation_plan>"#;

/// Full example of an `<implementation_plan>` showing all fields in use.
pub const IMPLEMENTATION_PLAN_FULL_EXAMPLE: &str = r#"<implementation_plan>
  <request>
    Add error handling to the process function and add tests
  </request>
  
  <summary>
    Wrap the database call in a try-catch, return a Result type, and add a test case
    for the error path.
  </summary>
  
  <files>
    <file operation="modify" path="src/processor.rs">
      <current_content>
use anyhow::Result;

pub fn process(id: u32) -> Data {
    let conn = get_connection();
    conn.query(id)
}
      </current_content>
      <changes>
        - Change return type to `Result<Data, Error>`
        - Wrap `get_connection()` call with `?` operator
        - Add `.context()` to the query call for better error messages
      </changes>
    </file>
    
    <file operation="create" path="src/utils/helper.rs">
      <template>
//! Helper utilities for processing.

use anyhow::Result;

/// Helper function skeleton
pub fn helper() -> Result<()> {
    todo!()
}
      </template>
    </file>
    
    <file operation="modify" path="tests/test_processor.rs">
      <current_content>
#[test]
fn test_process_success() {
    let result = process(1);
    assert!(result.is_valid());
}
      </current_content>
      <changes>
        - Add new test `test_process_connection_error` that mocks a failed connection
        - Use `#[should_panic]` or assert on the Result error
      </changes>
    </file>
  </files>
  
  <patterns>
    <pattern name="error handling">
      This codebase uses `anyhow::Result` with `.context()` for errors.
      See src/lib.rs:42 for example.
    </pattern>
    <pattern name="testing">
      Tests use `tokio::test` with mock traits for external services.
    </pattern>
  </patterns>
  
  <constraints>
    - Do not change the public API signature of `process()`
    - Maintain backward compatibility with existing callers
    - Follow existing error message style: "Failed to {action}: {details}"
  </constraints>
</implementation_plan>"#;

/// Minimal example for simple, single-file changes.
pub const IMPLEMENTATION_PLAN_MINIMAL_EXAMPLE: &str = r#"<implementation_plan>
  <request>Fix the typo in the error message</request>
  <summary>Change "recieved" to "received" in the error string</summary>
  <files>
    <file operation="modify" path="src/handler.rs">
      <current_content>
fn handle_request() -> Result<()> {
    Err(anyhow!("Invalid request recieved"))
}
      </current_content>
      <changes>
        - Line 2: Change "recieved" to "received"
      </changes>
    </file>
  </files>
</implementation_plan>"#;

/// Schema for `<exploration_result>` - explorer sub-agent output format.
pub const EXPLORATION_RESULT_SCHEMA: &str = r#"<exploration_result>
  <overview>
    <!-- Brief description of project architecture (1-2 paragraphs) -->
  </overview>
  
  <relevant_files>
    <!-- Files related to the task, ordered by relevance -->
    <file path="path/to/file" relevance="primary|secondary">
      <purpose>Why this file matters</purpose>
      <key_elements>
        - Notable functions, structs, patterns with line numbers
      </key_elements>
    </file>
  </relevant_files>
  
  <patterns>
    <!-- Codebase conventions to pass to the coder -->
    <pattern name="pattern name" example_file="path" example_line="N">
      Description of the pattern
    </pattern>
  </patterns>
  
  <entry_points>
    <!-- How code flows, to understand impact -->
    <entry path="path" type="binary|library|handler">
      Flow description
    </entry>
  </entry_points>
  
  <dependencies>
    <!-- External crates/packages relevant to the task -->
    <dependency name="name" purpose="what it's used for" />
  </dependencies>
  
  <recommendations>
    <!-- Your assessment of what files likely need changes -->
  </recommendations>
</exploration_result>"#;

/// Minimal exploration result for simple tasks.
pub const EXPLORATION_RESULT_MINIMAL: &str = r#"<exploration_result>
  <relevant_files>
    <file path="path/to/file" relevance="primary">
      <purpose>Why this file matters</purpose>
    </file>
  </relevant_files>
  <recommendations>
    Brief recommendation
  </recommendations>
</exploration_result>"#;

/// Schema for `<analysis_result>` - analyzer sub-agent output format.
pub const ANALYSIS_RESULT_SCHEMA: &str = r#"<analysis_result>
  <question>
    <!-- Restate what was asked to analyze -->
  </question>
  
  <summary>
    <!-- 2-3 sentence executive summary -->
  </summary>
  
  <findings>
    <!-- Detailed findings with file:line citations -->
    <finding severity="high|medium|low" file="path" lines="N-M">
      <description>Issue or insight</description>
      <evidence>
        ```language
        // Relevant code snippet
        ```
      </evidence>
      <recommendation>Suggested action</recommendation>
    </finding>
  </findings>
  
  <call_graph>
    <!-- If relevant: who calls what -->
    <function name="function_name" file="path">
      <called_by>
        <caller file="path" line="N">caller_function</caller>
      </called_by>
      <calls>
        <callee file="path" line="N">callee_function</callee>
      </calls>
    </function>
  </call_graph>
  
  <impact_assessment>
    <!-- What would change if we modify the analyzed code -->
  </impact_assessment>
  
  <implementation_guidance>
    <!-- Guidance for main agent to pass to coder -->
    <files_to_modify>
      <file path="path" reason="why" />
    </files_to_modify>
    <patterns_to_follow>
      <pattern name="name" example="path:line">
        Description
      </pattern>
    </patterns_to_follow>
  </implementation_guidance>
</analysis_result>"#;

/// Minimal analysis result for simpler analyses.
pub const ANALYSIS_RESULT_MINIMAL: &str = r#"<analysis_result>
  <summary>Key finding</summary>
  <findings>
    <finding severity="..." file="..." lines="...">
      <description>...</description>
      <recommendation>...</recommendation>
    </finding>
  </findings>
  <implementation_guidance>
    <files_to_modify>
      <file path="..." reason="..." />
    </files_to_modify>
  </implementation_guidance>
</analysis_result>"#;
