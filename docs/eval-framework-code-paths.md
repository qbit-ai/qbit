# Eval Framework: Code Path Analysis

This document provides a technical deep-dive into the exact code paths used by the eval framework vs. production agent.

## Entry Points

### Production Agent Entry

**Tauri GUI** (`backend/crates/qbit/src/ai/commands/run_agent.rs`):
```rust
#[tauri::command]
pub async fn ai_execute_prompt(
    state: State<'_, AgentBridgeState>,
    prompt: String,
) -> Result<String, String> {
    let bridge = state.bridge.lock().await;
    bridge.execute(&prompt).await
}
```

**CLI** (`backend/crates/qbit/src/cli/runner.rs`):
```rust
pub async fn execute_once(ctx: &mut CliContext, prompt: &str) -> Result<()> {
    let bridge = ctx.bridge().await;
    bridge.execute(prompt).await
}
```

### Eval Framework Entry

**CLI Eval Mode** (`backend/crates/qbit/src/cli/eval.rs`):
```rust
pub async fn run_evals(
    scenario_filter: Option<&str>,
    // ...
) -> Result<()> {
    // Does NOT use AgentBridge
    // Directly invokes qbit_evals::runner::EvalRunner
    let runner = EvalRunner::new_verbose_with_provider(verbose, provider)?;

    for scenario in scenarios {
        let report = scenario.run(&runner).await?;
    }
}
```

**Key Difference**: Production uses `AgentBridge`, evals use `EvalRunner` directly.

---

## AgentBridge Initialization

### Production (`llm_client.rs:69-102`)

```rust
pub async fn create_shared_components(
    workspace: &std::path::Path,
    model_name: &str,
    context_config: Option<ContextManagerConfig>,
) -> Result<SharedAgentComponents> {
    // 1. Full tool registry
    let tool_registry = Arc::new(RwLock::new(ToolRegistry::new(workspace)));

    // 2. Sub-agents REGISTERED
    let mut sub_agent_registry = SubAgentRegistry::new();
    sub_agent_registry.register_multiple(create_default_sub_agents());

    // 3. Context manager ENABLED
    let context_manager = Arc::new(ContextManager::with_config(
        model_name,
        context_config.unwrap_or_default(),  // Enabled by default
    ));

    // 4. Approval recorder with persistent directory
    let approval_recorder = Arc::new(
        ApprovalRecorder::new(dirs::home_dir().unwrap().join(".qbit"))
    );

    // 5. Loop detector
    let loop_detector = Arc::new(RwLock::new(LoopDetector::with_defaults()));

    Ok(SharedAgentComponents { /* ... */ })
}
```

### Eval Framework (`eval_support.rs:150-222`)

```rust
pub async fn run_eval_agentic_loop<M>(
    model: &M,
    system_prompt: &str,
    user_prompt: &str,
    config: AiEvalConfig,
) -> Result<EvalAgentOutput>
where M: rig::completion::CompletionModel + Sync
{
    // 1. Standard tool registry
    let tool_registry = Arc::new(RwLock::new(ToolRegistry::new(&config.workspace)));

    // 2. Sub-agents EMPTY
    let sub_agent_registry = Arc::new(RwLock::new(SubAgentRegistry::new()));
    // ^^^ No agents registered

    // 3. Context manager DISABLED
    let context_manager = Arc::new(ContextManager::with_config(
        &config.model_name,
        ContextManagerConfig {
            enabled: false,  // <-- DISABLED
            ..Default::default()
        },
    ));

    // 4. Approval recorder (temp directory)
    let approval_recorder = Arc::new(
        ApprovalRecorder::new(std::env::temp_dir().join("qbit-eval"))
    );

    // 5. Loop detector (same defaults)
    let loop_detector = Arc::new(RwLock::new(LoopDetector::with_defaults()));

    // ...
}
```

---

## Tool Configuration

### Production (`tool_definitions.rs:97-127`)

```rust
impl ToolConfig {
    /// Main agent gets Standard + execute_code + apply_patch
    pub fn main_agent() -> Self {
        Self {
            preset: ToolPreset::Standard,
            additional: vec![
                "execute_code".to_string(),
                "apply_patch".to_string(),
            ],
            disabled: vec![],
        }
    }
}
```

### Eval Framework (`eval_support.rs:199`)

```rust
// Uses default which is Standard preset with no additions
let tool_config = ToolConfig::default();
// Equivalent to:
// ToolConfig {
//     preset: ToolPreset::Standard,
//     additional: vec![],
//     disabled: vec![],
// }
```

---

## Agent Mode

### Production (`agent_bridge.rs:163-166`)

```rust
// Default mode - requires HITL
pub(crate) agent_mode: Arc<RwLock<AgentMode>>,

// Initialized as Default
agent_mode: Arc::new(RwLock::new(AgentMode::Default)),
```

### Eval Framework (`eval_support.rs:184-185`)

```rust
// Auto-approve mode - bypasses HITL
let agent_mode = Arc::new(RwLock::new(AgentMode::AutoApprove));
```

---

## AgenticLoopContext Construction

### Production (passed from `AgentBridge::execute()`)

```rust
let ctx = AgenticLoopContext {
    event_tx: &loop_event_tx,
    tool_registry: &self.tool_registry,
    sub_agent_registry: &self.sub_agent_registry,  // With 5 agents
    indexer_state: self.indexer_state.as_ref(),    // Optional, often Some
    tavily_state: self.tavily_state.as_ref(),      // Optional, often Some
    workspace: &self.workspace,
    client: &self.client,
    approval_recorder: &self.approval_recorder,
    pending_approvals: &self.pending_approvals,
    tool_policy_manager: &self.tool_policy_manager,
    context_manager: &self.context_manager,        // Enabled
    loop_detector: &self.loop_detector,
    tool_config: &self.tool_config,                // main_agent()
    sidecar_state: self.sidecar_state.as_ref(),    // Optional, often Some
    runtime: self.runtime.as_ref(),                // Tauri or CLI runtime
    agent_mode: &self.agent_mode,                  // Default
    plan_manager: &self.plan_manager,
    provider_name: &self.provider_name,
    model_name: &self.model_name,
    openai_web_search_config: self.openai_web_search_config.as_ref(),
};
```

### Eval Framework (`eval_support.rs:227-250`)

```rust
let ctx = AgenticLoopContext {
    event_tx: &event_tx,
    tool_registry: &tool_registry,
    sub_agent_registry: &sub_agent_registry,       // Empty
    indexer_state: None,                           // Disabled
    tavily_state: None,                            // Disabled
    workspace: &workspace_arc,
    client: &client,
    approval_recorder: &approval_recorder,
    pending_approvals: &pending_approvals,
    tool_policy_manager: &tool_policy_manager,
    context_manager: &context_manager,             // Disabled
    loop_detector: &loop_detector,
    tool_config: &tool_config,                     // default()
    sidecar_state: None,                           // Disabled
    runtime: None,                                 // None
    agent_mode: &agent_mode,                       // AutoApprove
    plan_manager: &plan_manager,
    provider_name: &config.provider_name,
    model_name: &config.model_name,
    openai_web_search_config: None,
};
```

---

## Agentic Loop Entry

Both paths converge at the same function:

### `run_agentic_loop_unified()` (`agentic_loop.rs:764-1428`)

```rust
pub async fn run_agentic_loop_unified<M>(
    model: &M,
    system_prompt: &str,
    initial_history: Vec<Message>,
    sub_agent_context: SubAgentContext,
    ctx: &AgenticLoopContext<'_>,
    config: AgenticLoopConfig,
) -> Result<(String, Vec<Message>, Option<TokenUsage>)>
```

**Production Path**:
```
AgentBridge::execute()
  → prepare_execution_context()
  → run_agentic_loop() or run_agentic_loop_generic()
    → run_agentic_loop_unified()
```

**Eval Path**:
```
run_eval_agentic_loop()
  → run_agentic_loop_unified()  // Direct call
```

---

## Tool Execution Path

### Shared: `execute_with_hitl_generic()` (`agentic_loop.rs:384-621`)

```rust
pub async fn execute_with_hitl_generic<M>(
    tool_name: &str,
    tool_args: &serde_json::Value,
    tool_id: &str,
    ctx: &AgenticLoopContext<'_>,
    // ...
) -> Result<ToolExecutionResult> {
    // STEP 0: Check agent mode for planning mode
    let agent_mode = *ctx.agent_mode.read().await;
    if agent_mode.is_planning() {
        // Deny non-read tools
    }

    // STEP 1: Check policy denial
    if ctx.tool_policy_manager.is_denied(tool_name).await {
        return deny;
    }

    // STEP 2: Apply constraints
    let (effective_args, _) = ctx.tool_policy_manager.apply_constraints(...);

    // STEP 3: Check ALLOW_TOOLS
    if policy == ToolPolicy::Allow {
        return execute_tool_direct_generic(...);
    }

    // STEP 4: Check learned patterns
    if ctx.approval_recorder.should_auto_approve(tool_name).await {
        return execute_tool_direct_generic(...);
    }

    // STEP 4.4: Check agent mode (AutoApprove)
    if agent_mode.is_auto_approve() {
        // ^^^ THIS is where evals diverge
        return execute_tool_direct_generic(...);
    }

    // STEP 4.5: Check runtime flag
    if runtime.auto_approve() {
        return execute_tool_direct_generic(...);
    }

    // STEP 5: Request approval (production only reaches here)
    // ... emit ToolApprovalRequest, wait for response
}
```

**Production**: May reach STEP 5 (HITL approval).
**Evals**: Always exits at STEP 4.4 (AutoApprove mode).

---

## Sub-Agent Tool Availability

### Production (`agentic_loop.rs:825-830`)

```rust
// Add sub-agent tools if not at max depth
if sub_agent_context.depth < MAX_AGENT_DEPTH - 1 {
    let registry = ctx.sub_agent_registry.read().await;
    tools.extend(get_sub_agent_tool_definitions(&registry).await);
    // ^^^ Adds sub_agent_coder, sub_agent_analyzer, etc.
}
```

### Eval Framework

Same code executes, but:
```rust
let registry = ctx.sub_agent_registry.read().await;
// registry is EMPTY, so:
get_sub_agent_tool_definitions(&registry)  // Returns empty vec
```

No sub-agent tools are added because registry is empty.

---

## Context Enforcement

### Production (`agentic_loop.rs:834-880`)

```rust
// Update context manager with current conversation
{
    let manager = ctx.context_manager;
    manager.update(/* message info */);

    if manager.is_enabled() {  // TRUE for production
        let enforcement = manager.enforce_context_window(&chat_history).await;

        if enforcement.pruned {
            // Emit ContextPruned event
            chat_history = enforcement.pruned_messages;
        }
        if enforcement.warning {
            // Emit ContextWarning event
        }
    }
}
```

### Eval Framework

Same code executes, but:
```rust
if manager.is_enabled() {  // FALSE for evals
    // This block is SKIPPED
}
// No pruning, no warnings
```

---

## Event Capture

### Production

```rust
// Events go to frontend and sidecar
ctx.event_tx.send(event)?;

// Sidecar captures if available
if let Some(sidecar) = ctx.sidecar_state {
    sidecar.capture(event);
}
```

### Eval Framework

```rust
// Events go to temp channel
ctx.event_tx.send(event)?;

// sidecar_state is None, so no capture
```

Events are collected in `EvalAgentOutput::events` vector instead.

---

## Multi-Turn Differences

### Production (implicit, via UI)

```rust
// Each turn:
// 1. User message added to conversation_history
// 2. execute() called with current history
// 3. Response added to conversation_history
// 4. Session saved

// History persists across bridge.execute() calls
```

### Eval Framework (`eval_support.rs:263-320`)

```rust
pub async fn run_multi_turn_eval<M>(
    model: &M,
    system_prompt: &str,
    user_prompts: &[&str],
    config: AiEvalConfig,
) -> Result<MultiTurnEvalOutput> {
    // SHARED resources across turns
    let tool_registry = Arc::new(RwLock::new(ToolRegistry::new(&config.workspace)));
    let sub_agent_registry = Arc::new(RwLock::new(SubAgentRegistry::new()));
    let approval_recorder = Arc::new(ApprovalRecorder::new(...));
    let loop_detector = Arc::new(RwLock::new(LoopDetector::with_defaults()));

    // MANUAL history accumulation
    let mut current_history: Vec<Message> = Vec::new();
    let mut turns = Vec::new();

    for user_prompt in user_prompts {
        // Add user message
        current_history.push(Message::User {
            content: OneOrMany::one(UserContent::Text(Text {
                text: user_prompt.to_string(),
            })),
        });

        // Run loop with accumulated history
        let (response, new_history, tokens) = run_agentic_loop_unified(
            model,
            system_prompt,
            current_history.clone(),  // Pass current history
            /* ctx */,
        ).await?;

        // Update history for next turn
        current_history = new_history;

        turns.push(/* turn output */);
    }

    Ok(MultiTurnEvalOutput { turns, /* ... */ })
}
```

**Key Difference**: Evals explicitly manage history; production relies on `AgentBridge::conversation_history`.

---

## System Prompt Construction

### Production (`system_prompt.rs:54-256`)

```rust
pub fn build_system_prompt_with_contributions(
    workspace_path: &Path,
    agent_mode: AgentMode,
    memory_file_path: Option<&Path>,
    registry: Option<&PromptContributorRegistry>,
    context: Option<&PromptContext>,
) -> String {
    let mut prompt = String::new();

    // Identity block
    prompt.push_str("<identity>...");

    // Environment block (workspace, date)
    prompt.push_str("<environment>...");

    // Style block
    prompt.push_str("<style>...");

    // Workflow block (5 phases)
    prompt.push_str("<workflow>...");

    // Tool documentation
    prompt.push_str("## Tools\n...");

    // Security constraints
    prompt.push_str("<security>...");

    // Project instructions (CLAUDE.md)
    if let Some(memory_path) = memory_file_path {
        prompt.push_str(&load_memory_file(memory_path));
    }

    // Dynamic contributions (sub-agents, provider tools)
    if let Some(reg) = registry {
        for section in reg.collect_contributions(context) {
            prompt.push_str(&section.content);
        }
    }

    // Agent mode specific instructions
    prompt.push_str(&get_mode_instructions(agent_mode));

    prompt
}
```

### Eval Framework (`executor.rs:23-38`)

```rust
const EVAL_SYSTEM_PROMPT: &str = r#"
You are an AI coding assistant.

You have access to these tools:
- read_file: Read a file's contents
- write_file: Write or overwrite a file
- create_file: Create a new file
- edit_file: Edit an existing file
- delete_file: Delete a file
- list_files: List files matching a pattern
- list_directory: List directory contents
- grep_file: Search for patterns in files
- run_pty_cmd: Run a shell command

Complete the task efficiently. When done, provide a summary.
Do not ask for clarification - make reasonable assumptions.
"#;
```

**Key Differences**:
- No identity/workflow/security blocks in evals
- No project instructions (CLAUDE.md)
- No dynamic contributions
- No agent mode instructions
- Focused, minimal prompt

---

## Summary: Divergence Points

| Code Location | Production | Evals |
|---------------|-----------|-------|
| `sub_agent_registry.register_multiple()` | Called | Skipped |
| `ContextManagerConfig.enabled` | `true` | `false` |
| `AgentMode` | `Default` | `AutoApprove` |
| `ToolConfig` | `main_agent()` | `default()` |
| `indexer_state` | `Some(...)` | `None` |
| `tavily_state` | `Some(...)` | `None` |
| `sidecar_state` | `Some(...)` | `None` |
| `runtime` | `Some(TauriRuntime/CliRuntime)` | `None` |
| System prompt | Full composition | Minimal constant |
| Session persistence | `QbitSessionManager` | In-memory only |
