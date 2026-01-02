# Plan: Consolidate Agentic Loop Implementations

## Problem Statement

The codebase has ~575+ lines of duplicated code across three agentic loop implementations:

| Implementation | Location | Lines |
|----------------|----------|-------|
| `run_agentic_loop` | `agentic_loop.rs:633-1243` | ~610 |
| `run_agentic_loop_generic<M>` | `agentic_loop.rs:1621-2177` | ~556 |
| `execute_sub_agent<M>` | `executor.rs:88-483` | ~395 |

## User Decisions

- **Sub-agent HITL**: Keep trusted (no approval required)
- **Thinking parity**: YES - unify thinking history tracking for all providers that support it

## Current Test Coverage (CRITICAL GAP)

| Component | Tests | Status |
|-----------|-------|--------|
| `agentic_loop.rs` | 0 | ❌ **NONE** |
| `agent_bridge.rs` | 0 | ❌ **NONE** |
| Sub-agent executor | 0 | ❌ **NONE** |
| Tool definitions | 25+ | ✓ |
| Loop detection | 9 | ✓ |
| HITL approval | ~4 | ⚠️ Partial |

**Risk**: Refactoring without tests could silently break behavior.

---

## Phase 0: Baseline Tests (BEFORE ANY REFACTORING)

### Step 0.1: Create Mock LLM Model for Testing

**File:** `backend/crates/qbit-ai/src/test_utils.rs` (new)

Create a mock `CompletionModel` that returns predefined responses:

```rust
/// Mock model for testing agentic loop behavior
pub struct MockCompletionModel {
    responses: Vec<MockResponse>,
    current: AtomicUsize,
}

pub struct MockResponse {
    pub text: Option<String>,
    pub tool_calls: Vec<ToolCall>,
    pub thinking: Option<String>,
}

impl CompletionModel for MockCompletionModel {
    // Returns responses in sequence, allowing multi-turn testing
}
```

### Step 0.2: HITL Approval Flow Tests

**File:** `backend/crates/qbit-ai/src/agentic_loop.rs` (add `#[cfg(test)]` module)

Test cases for `execute_with_hitl` / `execute_with_hitl_generic`:

```rust
#[tokio::test]
async fn test_hitl_planning_mode_blocks_write_tools()
#[tokio::test]
async fn test_hitl_planning_mode_allows_read_tools()
#[tokio::test]
async fn test_hitl_denied_by_policy()
#[tokio::test]
async fn test_hitl_allowed_by_policy_bypasses_approval()
#[tokio::test]
async fn test_hitl_auto_approve_from_learned_patterns()
#[tokio::test]
async fn test_hitl_auto_approve_from_agent_mode()
#[tokio::test]
async fn test_hitl_auto_approve_from_runtime_flag()
#[tokio::test]
async fn test_hitl_constraint_violation_denied()
#[tokio::test]
async fn test_hitl_approval_request_emitted()
#[tokio::test]
async fn test_hitl_approval_timeout()
```

### Step 0.3: Tool Routing Tests

**File:** `backend/crates/qbit-ai/src/agentic_loop.rs`

Test cases for `execute_tool_direct` / `execute_tool_direct_generic`:

```rust
#[tokio::test]
async fn test_tool_routing_indexer_tools()
#[tokio::test]
async fn test_tool_routing_web_fetch()
#[tokio::test]
async fn test_tool_routing_tavily_web_search()
#[tokio::test]
async fn test_tool_routing_update_plan()
#[tokio::test]
async fn test_tool_routing_sub_agent_invocation()
#[tokio::test]
async fn test_tool_routing_run_command_alias()
#[tokio::test]
async fn test_tool_routing_registry_fallback()
```

### Step 0.4: Agentic Loop Integration Tests

**File:** `backend/crates/qbit-ai/src/agentic_loop.rs`

Test the full loop behavior with mock model:

```rust
#[tokio::test]
async fn test_loop_single_turn_no_tools()
#[tokio::test]
async fn test_loop_single_tool_call_and_result()
#[tokio::test]
async fn test_loop_multiple_tool_calls_single_turn()
#[tokio::test]
async fn test_loop_multi_turn_conversation()
#[tokio::test]
async fn test_loop_max_iterations_reached()
#[tokio::test]
async fn test_loop_thinking_tracked_in_history_anthropic()
#[tokio::test]
async fn test_loop_thinking_not_tracked_generic()
#[tokio::test]
async fn test_loop_context_pruning_triggered()
#[tokio::test]
async fn test_loop_detection_warning_emitted()
#[tokio::test]
async fn test_loop_detection_blocked()
```

### Step 0.5: Sub-Agent Executor Tests

**File:** `backend/crates/qbit-sub-agents/src/executor.rs` (add tests)

```rust
#[tokio::test]
async fn test_sub_agent_basic_execution()
#[tokio::test]
async fn test_sub_agent_tool_filtering()
#[tokio::test]
async fn test_sub_agent_depth_increment()
#[tokio::test]
async fn test_sub_agent_max_iterations()
#[tokio::test]
async fn test_sub_agent_events_emitted()
#[tokio::test]
async fn test_sub_agent_coder_udiff_processing()
#[tokio::test]
async fn test_sub_agent_files_modified_tracking()
```

### Step 0.6: Behavioral Equivalence Tests

**Critical:** These tests verify `run_agentic_loop` and `run_agentic_loop_generic` produce identical results:

```rust
#[tokio::test]
async fn test_equivalence_single_turn_response()
#[tokio::test]
async fn test_equivalence_tool_execution_order()
#[tokio::test]
async fn test_equivalence_hitl_approval_flow()
#[tokio::test]
async fn test_equivalence_loop_detection_behavior()
#[tokio::test]
async fn test_equivalence_context_pruning()
```

### Baseline Test Summary

| Category | Test Count | Purpose |
|----------|------------|---------|
| HITL Approval | 10 | Verify approval logic unchanged |
| Tool Routing | 7 | Verify routing unchanged |
| Loop Integration | 10 | Verify loop behavior unchanged |
| Sub-Agent | 7 | Verify sub-agent behavior unchanged |
| Equivalence | 5 | Verify both loops match |
| **Total** | **39** | Baseline before refactoring |

---

## Implementation Steps (AFTER Baseline Tests Pass)

### Step 1: Add Model Capability Detection

**File:** `backend/crates/qbit-llm-providers/src/lib.rs`

Add `model_supports_thinking_history(provider, model)` function:
- Anthropic: all models
- OpenAI: o1, o1-preview, o3, o3-mini
- Gemini: gemini-2.0-flash-thinking-exp

Add `ModelCapabilities` struct:
```rust
pub struct ModelCapabilities {
    pub supports_temperature: bool,
    pub supports_thinking_history: bool,
}
```

### Step 2: Create Shared Tool Execution Module

**File:** `backend/crates/qbit-ai/src/tool_execution.rs` (new)

Extract tool routing logic into shared functions:

```rust
pub struct ToolExecutionConfig {
    pub require_hitl: bool,  // false for sub-agents
    pub tool_source: ToolSource,
}

pub async fn route_tool_execution<M>(
    tool_name: &str,
    tool_args: &serde_json::Value,
    ctx: &AgenticLoopContext<'_>,
    model: &M,
    config: &ToolExecutionConfig,
) -> Result<ToolExecutionResult>
where
    M: RigCompletionModel + Sync,
```

Handles: `indexer_*`, `web_fetch`, `web_search*`, `update_plan`, `sub_agent_*`, then falls through to HITL or trusted registry execution.

### Step 3: Create Unified Agentic Loop

**File:** `backend/crates/qbit-ai/src/agentic_loop.rs`

Add `run_agentic_loop_unified<M>` with config:

```rust
pub struct AgenticLoopConfig {
    pub capabilities: ModelCapabilities,
    pub require_hitl: bool,
    pub is_sub_agent: bool,
}

pub async fn run_agentic_loop_unified<M>(
    model: &M,
    system_prompt: &str,
    initial_history: Vec<Message>,
    context: SubAgentContext,
    ctx: &AgenticLoopContext<'_>,
    config: AgenticLoopConfig,
) -> Result<(String, Vec<Message>, Option<TokenUsage>)>
```

Key unification:
- Thinking/reasoning tracked in history when `capabilities.supports_thinking_history`
- HITL controlled by `require_hitl` flag
- Uses shared tool routing from Step 2

### Step 4: Migrate Existing Loops

1. Change `run_agentic_loop` to call `run_agentic_loop_unified` with Anthropic config
2. Change `run_agentic_loop_generic<M>` to call `run_agentic_loop_unified` with appropriate config
3. Run tests to verify identical behavior

### Step 5: Refactor Sub-Agent Executor

**File:** `backend/crates/qbit-sub-agents/src/executor.rs`

Option A (simple): Keep existing loop but use shared tool routing
Option B (full unification): Call `run_agentic_loop_unified` with trusted config

Recommend Option A initially - less risk, still eliminates tool routing duplication.

### Step 6: Simplify Agent Bridge Dispatch

**File:** `backend/crates/qbit-ai/src/agent_bridge.rs`

Extract common setup to helper, create generic execution method:

```rust
async fn execute_with_model_generic<M>(
    &self,
    model: &M,
    prompt: &str,
    start_time: Instant,
    context: SubAgentContext,
) -> Result<String>
where
    M: RigCompletionModel + Sync,
```

Simplifies ~10 match arms to use same execution path with different model types.

### Step 7: Cleanup

Delete redundant functions:
- `execute_with_hitl` (Anthropic-specific)
- `execute_tool_direct` (Anthropic-specific)
- `execute_with_hitl_generic`
- `execute_tool_direct_generic`

---

## Files to Modify

| File | Changes |
|------|---------|
| `backend/crates/qbit-llm-providers/src/lib.rs` | Add capability detection |
| `backend/crates/qbit-ai/src/tool_execution.rs` | NEW - shared tool routing |
| `backend/crates/qbit-ai/src/agentic_loop.rs` | Unified loop, delete duplicates |
| `backend/crates/qbit-ai/src/agent_bridge.rs` | Simplify dispatch |
| `backend/crates/qbit-sub-agents/src/executor.rs` | Use shared tool routing |
| `backend/crates/qbit-ai/src/lib.rs` | Update exports |

---

## Estimated Impact

- **Lines removed**: ~400-500
- **Lines added**: ~150-200
- **Net reduction**: ~250-300 lines
- **Behavioral consistency**: All agents use identical HITL/routing/policy logic
- **Thinking parity**: All capable models track reasoning in history

---

## Sequence Summary

### Phase 0: Baseline Tests (MUST COMPLETE FIRST)
0.1. Create mock LLM model for testing
0.2. Add HITL approval flow tests (10 tests)
0.3. Add tool routing tests (7 tests)
0.4. Add agentic loop integration tests (10 tests)
0.5. Add sub-agent executor tests (7 tests)
0.6. Add behavioral equivalence tests (5 tests)

**Gate:** All 39 baseline tests must pass before proceeding.

### Phase 1: Refactoring (Tests Protect Us)
1. Add capability detection (additive, safe) → run tests
2. Create tool_execution.rs with shared routing (additive, safe) → run tests
3. Create unified loop alongside existing (additive, safe) → run tests
4. Migrate main loops to call unified → **equivalence tests verify no regression**
5. Refactor sub-agent to use shared routing → **sub-agent tests verify no regression**
6. Simplify agent_bridge dispatch → run tests
7. Delete old duplicate code (cleanup) → run full test suite

### Rollback Strategy
- Each step is testable independently
- If any test fails after a step, revert that step immediately
- Equivalence tests are the safety net for behavioral changes
- Keep old functions as `_deprecated` until all tests pass on new code
