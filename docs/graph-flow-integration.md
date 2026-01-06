# Graph-Flow Integration Plan for Qbit

## Status: Infrastructure COMPLETE, Wiring Needed

The `qbit-workflow` crate already provides full graph-flow integration. The remaining work is connecting it to the agent system.

---

## What Already Exists in `qbit-workflow`

### Core Infrastructure ✅
| Component | Location | Purpose |
|-----------|----------|---------|
| `WorkflowRunner` | `runner.rs` | Session-based execution with storage |
| `WorkflowDefinition` | `models.rs` | Trait for defining workflows |
| `WorkflowLlmExecutor` | `models.rs` | Trait for LLM calls (needs impl) |
| `WorkflowAgentConfig` | `models.rs` | Mini-agent configuration |
| `SubAgentTask` | `runner.rs` | Task wrapper for sub-agents |
| `RouterTask` | `runner.rs` | Conditional routing |
| `AgentWorkflowBuilder` | `runner.rs` | Fluent graph construction |
| `WorkflowRegistry` | `registry.rs` | Register/lookup by name |

### Working Example: `git_commit` Workflow ✅
```
initialize → gatherer → analyzer → organizer → planner → formatter
```

**State:** `GitCommitState` with typed fields (`FileChange`, `CommitPlan`, `WorkflowStage`)

---

## Remaining Work

### 1. Implement `WorkflowLlmExecutor` (connects to AgentBridge)

```rust
// NEW: qbit-ai/src/workflow_executor.rs

pub struct QbitWorkflowExecutor {
    bridge: Arc<AgentBridge>,
}

#[async_trait]
impl WorkflowLlmExecutor for QbitWorkflowExecutor {
    async fn complete(&self, system: &str, user: &str, _ctx: HashMap<String, Value>) -> Result<String> {
        self.bridge.complete(system, user).await
    }

    async fn run_agent(&self, config: WorkflowAgentConfig) -> Result<WorkflowAgentResult> {
        self.bridge.run_mini_agent(
            &config.system_prompt,
            &config.task,
            config.tools.as_deref(),
            config.max_iterations.unwrap_or(25),
        ).await
    }
}
```

### 2. Expose Workflows to Users

**Option A: Tool** (main agent invokes)
```rust
Tool::new("run_workflow")
    .param("name", "git_commit | implement | review")
    .param("input", "JSON input")
```

**Option B: Command** (user triggers)
```
/workflow git_commit
/workflow implement "Add auth feature"
```

### 3. Add New Workflow Definitions

```
definitions/
├── git_commit/      ✅ DONE
├── implement/       🔲 Feature implementation
├── code_review/     🔲 PR/diff review  
└── refactor/        🔲 Code restructuring
```

---

## Example: `implement` Workflow

### State Definition
```rust
// definitions/implement/state.rs

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ImplementState {
    pub request: String,
    pub intent: ImplementIntent,
    pub exploration: Option<ExplorationResult>,
    pub analysis: Option<AnalysisResult>,
    pub plan: Option<ImplementationPlan>,
    pub diffs: Vec<FileDiff>,
    pub stage: WorkflowStage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ImplementIntent {
    NewFeature,
    BugFix,
    Refactor,
    Enhancement,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExplorationResult {
    pub relevant_files: Vec<RelevantFile>,
    pub patterns: Vec<CodePattern>,
    pub entry_points: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImplementationPlan {
    pub summary: String,
    pub files_to_modify: Vec<PlannedChange>,
    pub files_to_create: Vec<PlannedFile>,
    pub constraints: Vec<String>,
}
```

### Graph Definition
```rust
// definitions/implement/mod.rs

impl WorkflowDefinition for ImplementWorkflow {
    fn name(&self) -> &str { "implement" }
    
    fn build_graph(&self, executor: Arc<dyn WorkflowLlmExecutor>) -> Arc<Graph> {
        let explorer = Arc::new(ExplorerTask::new(executor.clone()));
        let analyzer = Arc::new(AnalyzerTask::new(executor.clone()));
        let planner = Arc::new(PlannerTask::new(executor.clone()));
        let coder = Arc::new(CoderTask::new(executor));

        Arc::new(GraphBuilder::new("implement")
            .add_task(explorer.clone())
            .add_task(analyzer.clone())
            .add_task(planner.clone())
            .add_task(coder.clone())
            // Conditional: skip analyzer for simple requests
            .add_conditional_edge(
                explorer.id(),
                |ctx| ctx.get_sync::<bool>("needs_analysis").unwrap_or(false),
                analyzer.id(),
                planner.id(),
            )
            .add_edge(analyzer.id(), planner.id())
            // Conditional: human approval for large changes
            .add_conditional_edge(
                planner.id(),
                |ctx| ctx.get_sync::<bool>("auto_approve").unwrap_or(true),
                coder.id(),
                planner.id(), // WaitForInput
            )
            .add_edge(coder.id(), "end")
            .build())
    }
    
    fn start_task(&self) -> &str { "explorer" }
    fn state_key(&self) -> &str { "implement_state" }
}
```

### Task Example
```rust
// definitions/implement/explorer.rs

pub struct ExplorerTask {
    executor: Arc<dyn WorkflowLlmExecutor>,
}

#[async_trait]
impl Task for ExplorerTask {
    fn id(&self) -> &str { "explorer" }

    async fn run(&self, ctx: Context) -> graph_flow::Result<TaskResult> {
        let state: ImplementState = ctx.get(STATE_KEY).await.unwrap_or_default();
        
        // Use mini-agent with file tools
        let config = WorkflowAgentConfig::new(
            EXPLORER_SYSTEM_PROMPT,
            format!("Explore codebase for: {}", state.request),
        )
        .with_tools(vec!["list_files", "read_file", "grep", "ast_grep"])
        .with_max_iterations(15);
        
        let result = self.executor.run_agent(config).await?;
        
        // Parse exploration result and update state
        let exploration: ExplorationResult = parse_exploration(&result.response)?;
        let needs_analysis = exploration.relevant_files.len() > 5;
        
        let mut new_state = state;
        new_state.exploration = Some(exploration);
        new_state.stage = WorkflowStage::Explored;
        
        ctx.set(STATE_KEY, new_state).await;
        ctx.set("needs_analysis", needs_analysis).await;
        
        Ok(TaskResult::new(Some(result.response), NextAction::Continue))
    }
}
```

---

## Integration Points

### Replace XML Sub-Agent Handoffs

**Before (XML):**
```rust
// Main agent generates XML string
let xml = format!("<implementation_plan>...</implementation_plan>");
let result = sub_agent.execute(xml).await?;
```

**After (Workflow):**
```rust
// Main agent triggers workflow
let registry = create_default_registry();
let workflow = registry.get("implement")?;
let executor = Arc::new(QbitWorkflowExecutor::new(bridge));
let graph = workflow.build_graph(executor);
let runner = WorkflowRunner::new_in_memory(graph);

let session_id = runner.start_session(&user_request, workflow.start_task()).await?;
let result = runner.run_to_completion(&session_id).await?;
```

### Deprecation Path

1. **Phase 1:** Add workflow tool alongside existing XML system
2. **Phase 2:** Route "implement" requests to workflow
3. **Phase 3:** Migrate remaining sub-agents to workflows
4. **Phase 4:** Remove XML schemas and old sub-agent code

---

## Key Differences

| Aspect | Current (XML) | Workflow System |
|--------|---------------|-----------------|
| Data format | XML strings | Typed Rust structs |
| Type safety | None | Compile-time |
| Routing | Main agent decides upfront | Conditional edges at runtime |
| Human approval | Manual | Built-in `WaitForInput` |
| Persistence | None | Session storage |
| Progress tracking | None | `emit_step_started/completed` |

---

## Files to Create/Modify

### New Files
- `qbit-ai/src/workflow_executor.rs` - `WorkflowLlmExecutor` impl
- `qbit-workflow/src/definitions/implement/` - Implement workflow
- `qbit-workflow/src/definitions/code_review/` - Review workflow

### Modify
- `qbit-ai/src/agent_bridge.rs` - Add `run_mini_agent()` method
- `qbit-ai/src/tools/mod.rs` - Add `run_workflow` tool (optional)
- `qbit-workflow/src/definitions/mod.rs` - Register new workflows

---

## Open Questions

1. **Tool access in tasks:** Pass tool registry via `WorkflowAgentConfig.tools` or inject at task construction?

2. **Streaming:** How to stream partial output from workflow tasks to UI?

3. **Error recovery:** Should failed tasks retry or escalate to human?

4. **Parallel execution:** Use `FanOutTask` for exploring multiple files simultaneously?
