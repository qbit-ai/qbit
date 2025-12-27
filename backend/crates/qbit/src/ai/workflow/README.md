# Workflow System

A generic, graph-based multi-agent workflow system built on [graph-flow](https://crates.io/crates/graph-flow).

## Overview

The workflow system allows you to define complex, multi-step AI workflows that:

- Execute as a sequence of tasks (agents)
- Share state between tasks via a typed context
- Support step-by-step execution with human-in-the-loop
- Can be started, paused, resumed, and cancelled
- Are fully decoupled from specific workflow implementations

## Architecture

```
workflow/
├── mod.rs                      # Module exports
├── models.rs                   # Core traits (WorkflowDefinition, WorkflowLlmExecutor)
├── registry.rs                 # WorkflowRegistry - stores workflow definitions
├── runner.rs                   # WorkflowRunner - executes workflow sessions
└── definitions/                # Workflow implementations
    ├── mod.rs                  # Registration helpers
    └── git_commit/             # Example workflow
        ├── mod.rs              # WorkflowDefinition implementation
        ├── state.rs            # Workflow-specific state types
        ├── analyzer.rs         # Task implementation
        ├── organizer.rs        # Task implementation
        └── planner.rs          # Task implementation
```

## Core Concepts

### WorkflowDefinition

Every workflow implements the `WorkflowDefinition` trait:

```rust
pub trait WorkflowDefinition: Send + Sync {
    /// Unique identifier (e.g., "git_commit", "code_review")
    fn name(&self) -> &str;

    /// Human-readable description
    fn description(&self) -> &str;

    /// Build the task graph with edges
    fn build_graph(&self, executor: Arc<dyn WorkflowLlmExecutor>) -> Arc<Graph>;

    /// Initialize state from user input (JSON)
    fn init_state(&self, input: serde_json::Value) -> anyhow::Result<serde_json::Value>;

    /// ID of the first task to execute
    fn start_task(&self) -> &str;

    /// Context key for storing workflow state
    fn state_key(&self) -> &str;
}
```

### Tasks

Tasks are the building blocks of workflows. Each task:

- Implements the `graph_flow::Task` trait
- Reads/writes shared state via the `Context`
- Returns a `TaskResult` with output and next action
- Can call the LLM via `WorkflowLlmExecutor`

```rust
#[async_trait]
impl Task for MyTask {
    fn id(&self) -> &str {
        "my_task"
    }

    async fn run(&self, context: Context) -> graph_flow::Result<TaskResult> {
        // Read state
        let state: MyState = context.get(STATE_KEY).await.unwrap_or_default();

        // Do work (call LLM, process data, etc.)
        let result = self.executor.complete(SYSTEM_PROMPT, &user_prompt, HashMap::new()).await?;

        // Update state
        let mut new_state = state;
        new_state.result = Some(result);
        context.set(STATE_KEY, new_state).await;

        // Return result with next action
        Ok(TaskResult::new(
            Some("Task completed".to_string()),
            NextAction::ContinueAndExecute,  // Continue to next task
        ))
    }
}
```

### NextAction

Tasks return a `NextAction` to control flow:

| Action | Description |
|--------|-------------|
| `Continue` | Move to next task in graph |
| `ContinueAndExecute` | Move to next task and execute immediately |
| `GoTo(task_id)` | Jump to specific task |
| `End` | End the workflow |
| `WaitForInput` | Pause for human input |

### State Management

Workflows use a shared `Context` for state:

```rust
// Store state
context.set("my_key", my_value).await;

// Retrieve state (typed)
let value: Option<MyType> = context.get("my_key").await;

// Retrieve with default
let value: MyType = context.get("my_key").await.unwrap_or_default();
```

State should be serializable (`Serialize + Deserialize`) for persistence.

### WorkflowLlmExecutor

Tasks that need LLM capabilities receive an executor:

```rust
pub trait WorkflowLlmExecutor: Send + Sync {
    /// Simple completion without tools
    async fn complete(
        &self,
        system_prompt: &str,
        user_prompt: &str,
        context: HashMap<String, serde_json::Value>,
    ) -> anyhow::Result<String>;

    /// Completion with configuration (tools, model, temperature, etc.)
    async fn complete_with_config(
        &self,
        system_prompt: &str,
        user_prompt: &str,
        context: HashMap<String, serde_json::Value>,
        config: WorkflowLlmConfig,
    ) -> anyhow::Result<WorkflowLlmResult>;
}
```

The executor is injected when building the graph, allowing tasks to call the LLM without knowing implementation details.

### WorkflowLlmConfig (Optional)

Tasks can customize LLM behavior per-call using `WorkflowLlmConfig`:

```rust
#[derive(Debug, Clone, Default)]
pub struct WorkflowLlmConfig {
    /// Model override (e.g., "claude-3-haiku" for faster/cheaper tasks)
    pub model: Option<String>,
    /// Temperature (0.0-1.0)
    pub temperature: Option<f32>,
    /// Max response tokens
    pub max_tokens: Option<u32>,
    /// Tools available to this task
    /// - None: No tools
    /// - Some(vec![]): All tools
    /// - Some(vec!["tool1", "tool2"]): Specific tools only
    pub tools: Option<Vec<String>>,
    /// Enable extended thinking/reasoning
    pub extended_thinking: Option<bool>,
}
```

#### Example: Task with Tools

```rust
use crate::ai::workflow::{WorkflowLlmConfig, WorkflowLlmExecutor};

const SYSTEM_PROMPT: &str = r#"You are a code analyzer. Use the provided tools to read and analyze files."#;

pub struct CodeAnalyzerTask {
    executor: Arc<dyn WorkflowLlmExecutor>,
}

#[async_trait]
impl Task for CodeAnalyzerTask {
    fn id(&self) -> &str { "code_analyzer" }

    async fn run(&self, context: Context) -> graph_flow::Result<TaskResult> {
        let state: MyState = context.get(STATE_KEY).await.unwrap_or_default();

        let user_prompt = format!("Analyze the file: {}", state.file_path);

        // Configure this task to use tools and a faster model
        let config = WorkflowLlmConfig::default()
            .with_model("claude-3-5-haiku")  // Use faster model
            .with_temperature(0.3)            // Lower temp for consistency
            .with_tools(vec!["read_file", "grep_file", "list_files"]);

        let result = self.executor
            .complete_with_config(SYSTEM_PROMPT, &user_prompt, HashMap::new(), config)
            .await
            .map_err(|e| graph_flow::Error::TaskFailed(e.to_string()))?;

        // result.text contains the response
        // result.tool_calls contains any tool calls made
        // result.tool_results contains tool execution results

        let mut new_state = state;
        new_state.analysis = Some(result.text);
        context.set(STATE_KEY, new_state).await;

        Ok(TaskResult::new(Some("Analysis complete".to_string()), NextAction::ContinueAndExecute))
    }
}
```

#### Example: Task without Tools (Default)

For simple tasks that don't need tools, use the basic `complete()` method:

```rust
// Simple completion - no tools, default model
let response = self.executor
    .complete(SYSTEM_PROMPT, &user_prompt, HashMap::new())
    .await?;
```

#### Configuration Options

| Option | Description | Example |
|--------|-------------|---------|
| `model` | Override the LLM model | `"claude-3-5-haiku"` for speed |
| `temperature` | Control randomness (0.0-1.0) | `0.3` for consistency |
| `max_tokens` | Limit response length | `4096` |
| `tools` | Enable tool access | `vec!["read_file"]` |
| `extended_thinking` | Enable chain-of-thought | `true` |

## Creating a New Workflow

### Step 1: Create the Directory Structure

```bash
mkdir -p src/ai/workflow/definitions/my_workflow
```

Create these files:
- `mod.rs` - WorkflowDefinition implementation
- `state.rs` - State types
- One file per task (e.g., `analyzer.rs`, `processor.rs`)

### Step 2: Define State Types

In `state.rs`:

```rust
use serde::{Deserialize, Serialize};

/// Input for starting the workflow
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MyWorkflowInput {
    pub query: String,
    pub options: Option<MyOptions>,
}

/// Workflow state - shared between all tasks
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MyWorkflowState {
    pub input_query: Option<String>,
    pub intermediate_result: Option<String>,
    pub final_output: Option<String>,
    pub errors: Vec<String>,
    pub stage: MyWorkflowStage,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub enum MyWorkflowStage {
    #[default]
    Initialized,
    Processing,
    Finalizing,
    Completed,
    Failed,
}
```

### Step 3: Implement Tasks

In `processor.rs`:

```rust
use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use graph_flow::{Context, NextAction, Task, TaskResult};

use super::state::{MyWorkflowState, MyWorkflowStage};
use super::STATE_KEY;
use crate::ai::workflow::models::WorkflowLlmExecutor;

const SYSTEM_PROMPT: &str = r#"You are a helpful assistant..."#;

pub struct ProcessorTask {
    executor: Arc<dyn WorkflowLlmExecutor>,
}

impl ProcessorTask {
    pub fn new(executor: Arc<dyn WorkflowLlmExecutor>) -> Self {
        Self { executor }
    }
}

#[async_trait]
impl Task for ProcessorTask {
    fn id(&self) -> &str {
        "processor"
    }

    async fn run(&self, context: Context) -> graph_flow::Result<TaskResult> {
        // Get current state
        let mut state: MyWorkflowState = context
            .get(STATE_KEY)
            .await
            .unwrap_or_default();

        // Update stage
        state.stage = MyWorkflowStage::Processing;
        context.set(STATE_KEY, state.clone()).await;

        // Build prompt
        let user_prompt = format!(
            "Process this: {}",
            state.input_query.as_deref().unwrap_or("N/A")
        );

        // Call LLM
        let response = match self
            .executor
            .complete(SYSTEM_PROMPT, &user_prompt, HashMap::new())
            .await
        {
            Ok(r) => r,
            Err(e) => {
                state.errors.push(format!("Processor error: {}", e));
                state.stage = MyWorkflowStage::Failed;
                context.set(STATE_KEY, state).await;
                return Ok(TaskResult::new(
                    Some(format!("Processing failed: {}", e)),
                    NextAction::GoTo("formatter".to_string()),
                ));
            }
        };

        // Update state with result
        state.intermediate_result = Some(response);
        context.set(STATE_KEY, state).await;

        Ok(TaskResult::new(
            Some("Processing complete".to_string()),
            NextAction::ContinueAndExecute,
        ))
    }
}
```

### Step 4: Implement WorkflowDefinition

In `mod.rs`:

```rust
mod processor;
pub mod state;

pub use processor::ProcessorTask;
pub use state::{MyWorkflowInput, MyWorkflowState, MyWorkflowStage};

use std::sync::Arc;

use async_trait::async_trait;
use graph_flow::{Context, GraphBuilder, NextAction, Task, TaskResult};

use crate::ai::workflow::models::{WorkflowDefinition, WorkflowLlmExecutor};

pub const STATE_KEY: &str = "my_workflow_state";

pub struct MyWorkflow;

impl WorkflowDefinition for MyWorkflow {
    fn name(&self) -> &str {
        "my_workflow"
    }

    fn description(&self) -> &str {
        "A workflow that does something useful"
    }

    fn build_graph(&self, executor: Arc<dyn WorkflowLlmExecutor>) -> Arc<graph_flow::Graph> {
        let initialize = Arc::new(InitializeTask);
        let processor = Arc::new(ProcessorTask::new(executor.clone()));
        let formatter = Arc::new(FormatterTask);

        let graph = GraphBuilder::new("my_workflow")
            .add_task(initialize.clone())
            .add_task(processor.clone())
            .add_task(formatter.clone())
            .add_edge(initialize.id(), processor.id())
            .add_edge(processor.id(), formatter.id())
            .build();

        Arc::new(graph)
    }

    fn init_state(&self, input: serde_json::Value) -> anyhow::Result<serde_json::Value> {
        let input: MyWorkflowInput = serde_json::from_value(input)?;

        let state = MyWorkflowState {
            input_query: Some(input.query),
            intermediate_result: None,
            final_output: None,
            errors: vec![],
            stage: MyWorkflowStage::Initialized,
        };

        Ok(serde_json::to_value(state)?)
    }

    fn start_task(&self) -> &str {
        "initialize"
    }

    fn state_key(&self) -> &str {
        STATE_KEY
    }
}

// Simple tasks that don't need LLM
struct InitializeTask;

#[async_trait]
impl Task for InitializeTask {
    fn id(&self) -> &str { "initialize" }

    async fn run(&self, context: Context) -> graph_flow::Result<TaskResult> {
        // Verify state exists
        let state: Option<MyWorkflowState> = context.get(STATE_KEY).await;
        if state.is_none() {
            return Ok(TaskResult::new(
                Some("Error: State not initialized".to_string()),
                NextAction::End,
            ));
        }
        Ok(TaskResult::new(Some("Initialized".to_string()), NextAction::ContinueAndExecute))
    }
}

struct FormatterTask;

#[async_trait]
impl Task for FormatterTask {
    fn id(&self) -> &str { "formatter" }

    async fn run(&self, context: Context) -> graph_flow::Result<TaskResult> {
        let mut state: MyWorkflowState = context.get(STATE_KEY).await.unwrap_or_default();

        // Format final output
        state.final_output = state.intermediate_result.clone();
        state.stage = MyWorkflowStage::Completed;
        context.set(STATE_KEY, state.clone()).await;

        Ok(TaskResult::new(
            state.final_output,
            NextAction::End,
        ))
    }
}
```

### Step 5: Register the Workflow

In `definitions/mod.rs`:

```rust
pub mod git_commit;
pub mod my_workflow;  // Add this

use std::sync::Arc;

use super::registry::WorkflowRegistry;

pub use git_commit::GitCommitWorkflow;
pub use my_workflow::MyWorkflow;  // Add this

pub fn register_builtin_workflows(registry: &mut WorkflowRegistry) {
    registry.register(Arc::new(GitCommitWorkflow));
    registry.register(Arc::new(MyWorkflow));  // Add this
}
```

### Step 6: (Optional) Export Types

If you want to expose your workflow's types publicly, add to `workflow/mod.rs`:

```rust
pub use definitions::my_workflow::{MyWorkflowState, MyWorkflow};
```

## Frontend API

### TypeScript Types

```typescript
interface WorkflowInfo {
  name: string;
  description: string;
}

interface StartWorkflowResponse {
  session_id: string;
  workflow_name: string;
}

interface WorkflowStepResponse {
  output: string | null;
  status: "paused" | "waiting_for_input" | "completed" | "error";
  next_task_id: string | null;
  error: string | null;
}

interface WorkflowStateResponse {
  state: any;  // Workflow-specific state
  status: string;
  current_task: string;
}
```

### Tauri Commands

```typescript
// List available workflows
const workflows = await invoke<WorkflowInfo[]>("list_workflows");

// Start a workflow
const { session_id } = await invoke<StartWorkflowResponse>("start_workflow", {
  workflowName: "git_commit",
  input: {
    git_status: "M  file.txt",
    git_diff: "diff content..."
  }
});

// Execute next step
const step = await invoke<WorkflowStepResponse>("step_workflow", {
  sessionId: session_id
});

// Or run to completion
const result = await invoke<string>("run_workflow_to_completion", {
  sessionId: session_id
});

// Get current state
const state = await invoke<WorkflowStateResponse>("get_workflow_state", {
  sessionId: session_id
});

// List active sessions
const sessions = await invoke<string[]>("list_workflow_sessions");

// Cancel a workflow
await invoke("cancel_workflow", { sessionId: session_id });
```

## Events

Workflows emit events for frontend visibility:

```typescript
// Listen for workflow events
listen("ai-event", (event) => {
  const data = event.payload;

  switch (data.type) {
    case "workflow_started":
      console.log(`Started ${data.workflow_name}: ${data.session_id}`);
      break;
    case "workflow_step_started":
      console.log(`Step ${data.step_index}/${data.total_steps}: ${data.step_name}`);
      break;
    case "workflow_step_completed":
      console.log(`Completed: ${data.step_name} (${data.duration_ms}ms)`);
      break;
    case "workflow_completed":
      console.log(`Workflow done: ${data.final_output}`);
      break;
    case "workflow_error":
      console.error(`Error in ${data.step_name}: ${data.error}`);
      break;
  }
});
```

## Testing

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    struct MockExecutor;

    #[async_trait]
    impl WorkflowLlmExecutor for MockExecutor {
        async fn complete(
            &self,
            _system_prompt: &str,
            _user_prompt: &str,
            _context: HashMap<String, serde_json::Value>,
        ) -> anyhow::Result<String> {
            Ok("Mock response".to_string())
        }
    }

    #[test]
    fn test_workflow_definition() {
        let workflow = MyWorkflow;
        assert_eq!(workflow.name(), "my_workflow");
        assert_eq!(workflow.start_task(), "initialize");
    }

    #[test]
    fn test_init_state() {
        let workflow = MyWorkflow;
        let input = serde_json::json!({ "query": "test" });
        let state = workflow.init_state(input).unwrap();

        let parsed: MyWorkflowState = serde_json::from_value(state).unwrap();
        assert_eq!(parsed.input_query, Some("test".to_string()));
    }

    #[tokio::test]
    async fn test_task_execution() {
        let executor = Arc::new(MockExecutor);
        let task = ProcessorTask::new(executor);
        let context = Context::new();

        // Set up initial state
        let state = MyWorkflowState {
            input_query: Some("test query".to_string()),
            ..Default::default()
        };
        context.set(STATE_KEY, state).await;

        let result = task.run(context.clone()).await.unwrap();
        assert!(result.response.is_some());
    }
}
```

## Running Mini Agents Within Tasks

For complex tasks that need to use tools iteratively, you can spawn a mini agent within a workflow task using `run_agent()`. This gives tasks full agent capabilities including tool access.

### WorkflowAgentConfig

Configure a mini agent with `WorkflowAgentConfig`:

```rust
#[derive(Debug, Clone, Default)]
pub struct WorkflowAgentConfig {
    /// System prompt for this agent
    pub system_prompt: String,
    /// Initial task/message for the agent
    pub task: String,
    /// Tools available:
    /// - None: No tools (same as complete())
    /// - Some(vec![]): All available tools
    /// - Some(vec!["tool1", "tool2"]): Specific tools only
    pub tools: Option<Vec<String>>,
    /// Max iterations before stopping (default: 25)
    pub max_iterations: Option<usize>,
    /// Model override (e.g., "claude-3-5-haiku")
    pub model: Option<String>,
    /// Temperature (0.0-1.0)
    pub temperature: Option<f32>,
    /// Emit events for tool calls (default: true)
    pub emit_events: Option<bool>,
}
```

### WorkflowAgentResult

The agent returns a result with execution history:

```rust
#[derive(Debug, Clone, Default)]
pub struct WorkflowAgentResult {
    /// Final text response from the agent
    pub response: String,
    /// All tool calls made during execution
    pub tool_history: Vec<WorkflowToolCall>,
    /// Number of LLM iterations taken
    pub iterations: usize,
    /// Total tokens used (if available)
    pub tokens_used: Option<u64>,
    /// Whether the agent completed successfully
    pub completed: bool,
    /// Error message if the agent failed
    pub error: Option<String>,
}
```

### Example: Task Using run_agent()

```rust
use crate::ai::workflow::{WorkflowAgentConfig, WorkflowLlmExecutor};

const ANALYZER_SYSTEM_PROMPT: &str = r#"
You are a code analyzer. Use the provided tools to:
1. Read source files
2. Search for patterns
3. Analyze code structure
Report your findings in a structured format.
"#;

pub struct CodeAnalyzerTask {
    executor: Arc<dyn WorkflowLlmExecutor>,
}

#[async_trait]
impl Task for CodeAnalyzerTask {
    fn id(&self) -> &str { "code_analyzer" }

    async fn run(&self, context: Context) -> graph_flow::Result<TaskResult> {
        let state: MyState = context.get(STATE_KEY).await.unwrap_or_default();

        // Configure the mini agent
        let config = WorkflowAgentConfig::new(
            ANALYZER_SYSTEM_PROMPT,
            format!("Analyze the code changes in these files: {:?}", state.changed_files),
        )
        .with_tools(vec!["read_file", "grep_file", "list_files"])
        .with_max_iterations(20)
        .with_temperature(0.3);

        // Run the agent loop
        let result = self.executor
            .run_agent(config)
            .await
            .map_err(|e| graph_flow::Error::TaskFailed(e.to_string()))?;

        if !result.completed {
            return Err(graph_flow::Error::TaskFailed(
                result.error.unwrap_or_else(|| "Agent did not complete".to_string())
            ));
        }

        // Use the result
        let mut new_state = state;
        new_state.analysis = Some(result.response);
        new_state.tools_used = result.tool_history.len();
        context.set(STATE_KEY, new_state).await;

        Ok(TaskResult::new(
            Some(format!("Analysis complete ({} tool calls)", result.tool_history.len())),
            NextAction::ContinueAndExecute,
        ))
    }
}
```

### When to Use run_agent() vs complete()

| Use Case | Method | Reason |
|----------|--------|--------|
| Simple text generation | `complete()` | No tools needed |
| Single LLM call with output | `complete()` | Straightforward |
| Need to read/write files | `run_agent()` | Requires tools |
| Multi-step reasoning with tools | `run_agent()` | Iterative tool use |
| Code search and analysis | `run_agent()` | Needs grep, read |
| Complex task with planning | `run_agent()` | Self-directed execution |

### Configuration Tips

1. **Limit tools** - Only enable tools the task actually needs
2. **Set max_iterations** - Prevent runaway agents (default is 25)
3. **Use faster models** - For simple tool tasks, use haiku
4. **Emit events** - Keep `emit_events: true` for UI visibility
5. **Handle incomplete** - Check `result.completed` before using response

## Best Practices

1. **Keep tasks focused** - Each task should do one thing well
2. **Handle errors gracefully** - Store errors in state and route to formatter
3. **Use meaningful task IDs** - They appear in logs and events
4. **Make state serializable** - Use `#[derive(Serialize, Deserialize)]`
5. **Validate input early** - Check in `init_state()` before starting
6. **Test with mock executors** - Don't require real LLM for unit tests
7. **Emit progress events** - Keep the frontend informed
8. **Document your prompts** - System prompts are important; make them clear
9. **Use run_agent() for tool tasks** - When tasks need to use tools iteratively
10. **Limit agent scope** - Constrain tools and iterations to prevent runaway behavior
