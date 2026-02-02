# Debug badge implementation work tracker

Use this checklist to implement `DEBUG_BADGE_PLAN.md` with minimal back-and-forth. Keep boxes updated as you work.

---

## 0) Prep / sanity

- [x] Read `DEBUG_BADGE_PLAN.md` end-to-end
- [x] Create a feature branch (optional) (do not commit unless asked)

---

## 1) qbit-core: shared API request stats type

- [x] Create `backend/crates/qbit-core/src/api_request_stats.rs`
  - [x] Implement `ApiRequestStats` (tokio::RwLock<HashMap<...>>)
  - [x] Implement `ProviderRequestStatsSnapshot` (Serialize)
  - [x] Implement `ApiRequestStatsSnapshot` (Serialize)
  - [x] Implement `record_sent(provider)`
  - [x] Implement `record_received(provider)`
  - [x] Implement `snapshot()`
  - [x] Implement `now_ms()` helper (SystemTime → unix millis)
- [x] Update `backend/crates/qbit-core/src/lib.rs`
  - [x] Add `pub mod api_request_stats;`
  - [x] Add `pub use api_request_stats::{ApiRequestStats, ApiRequestStatsSnapshot, ProviderRequestStatsSnapshot};`
---

## 2) qbit-ai: store stats per session on AgentBridge

- [x] Update `backend/crates/qbit-ai/src/agent_bridge.rs`
  - [x] Import `qbit_core::{ApiRequestStats, ApiRequestStatsSnapshot}`
  - [x] Add field on `AgentBridge`: `api_request_stats: Arc<ApiRequestStats>`
  - [x] Initialize in `from_components_with_runtime` struct literal: `Arc::new(ApiRequestStats::new())`
  - [x] Add `pub async fn get_api_request_stats_snapshot(&self) -> ApiRequestStatsSnapshot`

---

## 3) qbit-ai: main agent loop wiring + instrumentation

- [x] Update `backend/crates/qbit-ai/src/agentic_loop.rs`
  - [x] Import `qbit_core::ApiRequestStats`
  - [x] Add field to `AgenticLoopContext<'a>`: `api_request_stats: &'a Arc<ApiRequestStats>`
  - [x] Instrument main `.stream` boundary
    - [x] Call `record_sent(ctx.provider_name)` immediately before the timeout-wrapped `model.stream(request)`
    - [x] Call `record_received(ctx.provider_name)` inside `Ok(Ok(s))` (stream successfully created)
- [x] Update `backend/crates/qbit-ai/src/agent_bridge.rs`
  - [x] In `build_loop_context`, set `api_request_stats: &self.api_request_stats`

---

## 4) qbit-sub-agents: executor context + instrumentation

- [x] Update `backend/crates/qbit-sub-agents/src/executor.rs`
  - [x] Import `qbit_core::ApiRequestStats`
  - [x] Extend `SubAgentExecutorContext<'a>` with:
    - [x] `api_request_stats: Option<&'a Arc<ApiRequestStats>>`
  - [x] Instrument sub-agent `.stream` boundary
    - [x] Before `model.stream(request)`: if stats present, `record_sent(ctx.provider_name)`
    - [x] In `Ok(s)` arm: if stats present, `record_received(ctx.provider_name)`

---

## 5) qbit-ai: pass stats into sub-agent execution context

- [x] Update `backend/crates/qbit-ai/src/agentic_loop.rs` (3 call sites)
  - [x] In each `SubAgentExecutorContext { ... }` literal, set:
    - [x] `api_request_stats: Some(ctx.api_request_stats)`

---

## 6) qbit-ai: compilation fixups for eval + tests

### 6.1 AgenticLoopContext literals

- [x] Update `backend/crates/qbit-ai/src/eval_support.rs` (3 `AgenticLoopContext { ... }` literals)
  - [x] Import `qbit_core::ApiRequestStats`
  - [x] For each ctx literal:
    - [x] Create local `let api_request_stats = Arc::new(ApiRequestStats::new());`
    - [x] Add field `api_request_stats: &api_request_stats,`

- [x] Update `backend/crates/qbit-ai/src/test_utils.rs` for `AgenticLoopContext` literal
  - [x] Import `qbit_core::ApiRequestStats`
  - [x] Add `api_request_stats: Arc<ApiRequestStats>` to `TestContext`
  - [x] In `TestContextBuilder::build()`, initialize `api_request_stats: Arc::new(ApiRequestStats::new())`
  - [x] In `TestContext::as_agentic_context_with_client` ctx literal, add `api_request_stats: &self.api_request_stats`

### 6.2 SubAgentExecutorContext literals (tests)

- [x] Update `backend/crates/qbit-ai/src/test_utils.rs` (6 `SubAgentExecutorContext { ... }` literals)
  - [x] Add field `api_request_stats: None, // tests`

---

## 7) qbit (Tauri backend): session-scoped stats command

- [x] Create `backend/crates/qbit/src/ai/commands/debug.rs`
  - [x] Implement `#[tauri::command] get_api_request_stats(state: State<'_, AppState>, session_id: String) -> Result<qbit_core::ApiRequestStatsSnapshot, String>`
  - [x] Use `state.ai_state.get_session_bridge(&session_id)`
  - [x] Error with `super::ai_session_not_initialized_error(&session_id)`
  - [x] Return `bridge.get_api_request_stats_snapshot().await`

- [x] Update `backend/crates/qbit/src/ai/commands/mod.rs`
  - [x] Add `pub mod debug;`
  - [x] Add `pub use debug::*;`

- [x] Update `backend/crates/qbit/src/ai/mod.rs`
  - [x] Add `get_api_request_stats` to `pub use commands::{ ... }` list

- [x] Update `backend/crates/qbit/src/lib.rs`
  - [x] Add `get_api_request_stats` to `use ai::{ ... }` list
  - [x] Add `get_api_request_stats` to `tauri::generate_handler![ ... ]`

---

## 8) Frontend: API wrapper

- [x] Update `frontend/lib/ai.ts`
  - [x] Add `ProviderRequestStats` type (snake_case fields)
  - [x] Add `ApiRequestStatsSnapshot` type
  - [x] Add `getApiRequestStats(sessionId: string)` wrapper calling `invoke("get_api_request_stats", { sessionId })`

---

## 9) Frontend: Debug badge UI

- [x] Update `frontend/components/UnifiedInput/InputStatusRow.tsx`
  - [x] Import `Bug` from `lucide-react`
  - [x] Import `getApiRequestStats` + `ApiRequestStatsSnapshot` from `@/lib/ai`
  - [x] Add state:
    - [x] `debugOpen`, `setDebugOpen`
    - [x] `debugPollRef`
    - [x] `apiRequestStats`, `apiRequestStatsError`
  - [x] Implement `refreshApiRequestStats()`
    - [x] Expected error handling: treat as normal if error contains:
      - [x] `"AI agent not initialized for session"` OR
      - [x] `"Call init_ai_session first"`
    - [x] Do not log expected case
  - [x] Controlled popover:
    - [x] `<Popover open={debugOpen} onOpenChange={setDebugOpen}>`
    - [x] Poll only while open (1500ms)
  - [x] Render badge only when:
    - [x] `import.meta.env.DEV`
    - [x] and (recommended) not `isMockBrowserMode()`
  - [x] Place badge immediately after Langfuse badge block
  - [x] Popover contents:
    - [x] Title: `Debug (This Tab)`
    - [x] Subtitle: `LLM API Requests (main + sub-agents)`
    - [x] Table: Provider | Req | Sent | Recv
    - [x] Sort providers by requests desc
    - [x] Relative time + absolute tooltip; null → `—`

---

## 10) Verification

- [ ] Run `just check` (fails: missing frontend deps/types in this environment)
- [ ] Run `just test-rust` (timed out after 120s)
- [ ] Manual UI verification
  - [ ] Two tabs: stats are independent per tab
  - [ ] Tab without AI initialized shows friendly empty state (no noisy error)
  - [ ] Main agent request increments counts + updates sent/recv
  - [ ] Sub-agent request increments counts + updates sent/recv
  - [ ] Sub-agent override provider produces separate provider row

---

## 11) Cleanup

- [ ] Ensure no secrets are logged or stored
- [ ] Ensure no background polling when popover is closed
- [ ] Ensure code formatted (run `just fmt` if needed)
