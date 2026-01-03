# Eval Framework: Identified Gaps and Recommendations

This document identifies gaps between the eval framework and production agent, and provides recommendations for improving evaluation coverage.

## Gap Analysis

### 1. Missing Tool Coverage

**Gap**: Evals use `ToolConfig::default()` instead of `ToolConfig::main_agent()`.

**Missing Tools**:
- `execute_code` - Code execution for complex operations
- `apply_patch` - Patch-based editing

**Impact**: Cannot test scenarios requiring:
- Complex code execution with multiple steps
- Patch-based file modifications
- Advanced edit workflows

**Recommendation**:
```rust
// Option A: Add ToolPreset::Evaluation with execute_code
pub enum ToolPreset {
    Minimal,
    Standard,
    Evaluation,  // Standard + execute_code
    Full,
}

// Option B: Make tool config per-scenario
trait Scenario {
    fn tool_config(&self) -> ToolConfig {
        ToolConfig::default()  // Override for specific scenarios
    }
}
```

---

### 2. Sub-Agent Testing Disabled

**Gap**: Sub-agent registry is empty in evals.

**Code Location**: `eval_support.rs:158-159`
```rust
let sub_agent_registry = Arc::new(RwLock::new(SubAgentRegistry::new()));
```

**Impact**: Cannot test:
- Delegation decisions (when agent should delegate)
- Sub-agent execution quality
- Depth limiting behavior
- Cross-agent coordination

**Recommendation**:
```rust
// Option A: Optional sub-agent registration
pub struct EvalConfig {
    pub enable_sub_agents: bool,
    // ...
}

if config.enable_sub_agents {
    sub_agent_registry.register_multiple(create_default_sub_agents());
}

// Option B: Sub-agent-specific scenarios
pub struct SubAgentEvalScenario {
    expected_delegations: Vec<&str>,  // ["coder", "explorer"]
}
```

---

### 3. Context Management Disabled

**Gap**: Context pruning is disabled in evals.

**Code Location**: `eval_support.rs:175-182`
```rust
let context_manager = Arc::new(ContextManager::with_config(
    &config.model_name,
    ContextManagerConfig {
        enabled: false,  // DISABLED
        ..Default::default()
    },
));
```

**Impact**:
- No testing of context window behavior
- Long conversations may exceed limits
- Cannot validate pruning correctness
- No alert/warning testing

**Recommendation**:
```rust
// Option A: Configurable per scenario
trait Scenario {
    fn context_config(&self) -> ContextManagerConfig {
        ContextManagerConfig::disabled()  // Default
    }
}

// Option B: Context-specific eval scenarios
pub struct ContextOverflowScenario {
    // Specifically tests context management
    target_utilization: f32,  // e.g., 0.9 to trigger pruning
}
```

---

### 4. No HITL Flow Testing

**Gap**: AutoApprove mode bypasses all approval steps.

**Code Location**: `eval_support.rs:184-185`
```rust
let agent_mode = Arc::new(RwLock::new(AgentMode::AutoApprove));
```

**Impact**: Cannot test:
- Approval request generation
- Risk level classification
- Approval pattern learning
- Timeout handling
- User denial scenarios

**Recommendation**:
```rust
// Add mock HITL runtime for testing
pub struct MockHitlRuntime {
    approval_responses: Vec<ApprovalDecision>,
    delay_ms: u64,
}

impl QbitRuntime for MockHitlRuntime {
    async fn request_approval(&self, ...) -> ApprovalResult {
        // Return pre-configured responses
    }
}

// Scenario with expected approvals
pub struct HitlTestScenario {
    expected_approval_requests: Vec<ExpectedApproval>,
}
```

---

### 5. Indexer Integration Missing

**Gap**: `indexer_state = None` in evals.

**Code Location**: `eval_support.rs:241`
```rust
indexer_state: None,
```

**Impact**: Cannot test:
- Code search quality (`indexer_search_code`)
- File discovery (`indexer_search_files`)
- Symbol extraction (`indexer_extract_symbols`)
- Language detection accuracy

**Recommendation**:
```rust
// Option A: Optional indexer initialization
pub struct EvalConfig {
    pub enable_indexer: bool,
}

if config.enable_indexer {
    let indexer_state = initialize_indexer(&workspace).await?;
}

// Option B: Pre-indexed testbeds
// Embed index data with testbed files
fn testbed_with_index() -> (Vec<(String, String)>, IndexerState) {
    // ...
}
```

---

### 6. Tavily Web Search Fallback Missing

**Gap**: `tavily_state = None` in evals.

**Code Location**: `eval_support.rs:242`
```rust
tavily_state: None,
```

**Impact**:
- Native web search works (provider-specific)
- Tavily fallback unavailable
- Cannot test `web_search_answer`, `web_extract`

**Recommendation**:
```rust
// Check if Tavily API key available
if std::env::var("TAVILY_API_KEY").is_ok() {
    tavily_state = Some(initialize_tavily()?);
}

// Or mock Tavily for deterministic testing
pub struct MockTavilyState {
    responses: HashMap<String, SearchResult>,
}
```

---

### 7. Session Persistence for Recovery

**Gap**: Evals don't persist sessions.

**Impact**:
- Cannot resume failed evals
- No session trail for debugging
- Cannot analyze decision patterns post-hoc

**Recommendation**:
```rust
// Option A: Optional session recording
pub struct EvalConfig {
    pub persist_sessions: bool,
    pub session_dir: Option<PathBuf>,
}

// Option B: Always persist to temp dir, optionally save on failure
impl EvalRunner {
    pub fn save_failed_session(&self, report: &EvalReport) -> PathBuf {
        // Copy temp session to permanent location
    }
}
```

---

### 8. Sidecar Context Capture

**Gap**: `sidecar_state = None` in evals.

**Impact**:
- No decision pattern capture
- Cannot analyze reasoning traces
- No correlation with file operations

**Recommendation**:
```rust
// Lightweight capture for debugging
pub struct EvalCaptureContext {
    tool_calls: Vec<CapturedToolCall>,
    reasoning_traces: Vec<String>,
    file_operations: Vec<FileOp>,
}

// Attach to AgenticLoopContext
sidecar_state: Some(Arc::new(EvalCaptureContext::new())),
```

---

### 9. Multi-Turn History Fragility

**Gap**: Manual history management in multi-turn evals.

**Code Pattern**:
```rust
let mut current_history = Vec::new();
for prompt in prompts {
    current_history.push(user_msg);
    let (_, new_history, _) = run_agentic_loop_unified(...);
    current_history = new_history;  // Manual update
}
```

**Risks**:
- History corruption if loop fails mid-execution
- No checkpoint/recovery
- Relies on correct history passing

**Recommendation**:
```rust
// Add explicit history manager for multi-turn
pub struct MultiTurnSession {
    history: Vec<Message>,
    checkpoints: Vec<usize>,  // History length at each turn
}

impl MultiTurnSession {
    pub fn add_turn(&mut self, user_prompt: &str);
    pub fn apply_response(&mut self, new_history: Vec<Message>);
    pub fn rollback_to(&mut self, turn: usize);
}
```

---

### 10. Provider-Specific Testing Gaps

**Gap**: Some provider-specific behaviors not isolated for testing.

**Examples**:
- OpenAI Responses API reasoning ID preservation
- Anthropic extended thinking format
- Gemini grounding source handling

**Recommendation**:
```rust
// Provider-specific scenario traits
pub trait OpenAiScenario: Scenario {
    fn validate_reasoning_ids(&self, output: &AgentOutput) -> bool;
}

pub trait AnthropicScenario: Scenario {
    fn validate_thinking_format(&self, output: &AgentOutput) -> bool;
}

// Provider-aware metrics
pub struct ReasoningIdMetric;  // For OpenAI
pub struct ThinkingBlockMetric;  // For Anthropic
```

---

## Implementation Priority

### High Priority (Core Functionality)

| Gap | Effort | Value |
|-----|--------|-------|
| Sub-agent testing | Medium | High |
| Context management testing | Medium | High |
| Execute_code tool | Low | Medium |

### Medium Priority (Enhanced Coverage)

| Gap | Effort | Value |
|-----|--------|-------|
| Indexer integration | Medium | Medium |
| Session persistence | Low | Medium |
| HITL mock runtime | High | Medium |

### Lower Priority (Nice to Have)

| Gap | Effort | Value |
|-----|--------|-------|
| Tavily fallback | Low | Low |
| Sidecar capture | Medium | Low |
| Multi-turn checkpoints | Low | Low |

---

## Proposed Architecture Changes

### 1. EvalConfig Enhancement

```rust
pub struct EvalConfig {
    // Existing
    pub provider: EvalProvider,
    pub verbose: bool,

    // Proposed additions
    pub tool_preset: ToolPreset,
    pub enable_sub_agents: bool,
    pub enable_context_management: bool,
    pub enable_indexer: bool,
    pub enable_tavily: bool,
    pub persist_sessions: bool,
    pub hitl_mode: HitlMode,
}

pub enum HitlMode {
    AutoApprove,           // Current behavior
    MockResponses(Vec<ApprovalDecision>),
    Interactive,           // For manual testing
}
```

### 2. Scenario Configuration

```rust
pub trait Scenario: Send + Sync {
    // Existing
    fn name(&self) -> &str;
    fn prompt(&self) -> &str;
    fn metrics(&self) -> Vec<Box<dyn Metric>>;

    // Proposed additions
    fn tool_config(&self) -> ToolConfig {
        ToolConfig::default()
    }

    fn requires_sub_agents(&self) -> bool {
        false
    }

    fn requires_indexer(&self) -> bool {
        false
    }

    fn context_config(&self) -> ContextManagerConfig {
        ContextManagerConfig::disabled()
    }
}
```

### 3. Enhanced AgentOutput

```rust
pub struct AgentOutput {
    // Existing
    pub response: String,
    pub tool_calls: Vec<ToolCall>,
    pub files_modified: Vec<PathBuf>,
    pub duration_ms: u64,
    pub tokens_used: Option<u32>,

    // Proposed additions
    pub sub_agent_delegations: Vec<SubAgentCall>,
    pub context_events: Vec<ContextEvent>,
    pub approval_requests: Vec<ApprovalRequest>,
    pub reasoning_traces: Vec<ReasoningBlock>,
}
```

---

## Migration Path

### Phase 1: Non-Breaking Additions

1. Add optional config fields with defaults
2. Add new scenario trait methods with default implementations
3. Add new metrics without changing existing ones

### Phase 2: Optional Feature Activation

1. Add feature flags for sub-agents, indexer, context
2. Test with selected scenarios
3. Document new capabilities

### Phase 3: Full Integration

1. Create sub-agent eval scenarios
2. Create context management eval scenarios
3. Add provider-specific test suites
4. Document best practices

---

## Testing the Gaps

To validate these gaps exist, run:

```bash
# Verify sub-agents missing
cargo test --features evals -- test_sub_agent_unavailable

# Verify context pruning disabled
cargo test --features evals -- test_context_not_pruned

# Verify indexer unavailable
cargo test --features evals -- test_indexer_tools_missing
```

These tests should pass, confirming the documented behavior.
