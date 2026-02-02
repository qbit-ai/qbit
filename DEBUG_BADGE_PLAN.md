# Debug badge + per-session API request stats plan

## Goal
Add a **Debug** badge to the Unified Input footer badge row, **immediately to the right of the existing Langfuse tracing badge**. The badge is used for Qbit developer-focused debug info.

First debug feature:
- Per **tab/pane session** (`sessionId`) stats:
  - Number of LLM API requests made **per provider**
  - Time the **last request was sent**
  - Time the **last request was received**

Must count both:
- Main agent loop requests (`qbit-ai`)
- Sub-agent requests (`qbit-sub-agents`)

UI behavior:
- Badge shown only in dev builds (`import.meta.env.DEV`)
- Popover polling happens **only while the popover is open**

---

## High-level architecture (locked)

- Store stats **per session** by attaching an `Arc<ApiRequestStats>` to the session’s `AgentBridge`.
- Share the stats type via **Layer 1** crate `qbit-core` so both `qbit-ai` and `qbit-sub-agents` can update it.
- Expose stats to the frontend via a **session-scoped** Tauri command:
  - `get_api_request_stats(session_id: String) -> ApiRequestStatsSnapshot`

Provider key:
- Use the existing provider name strings already carried through contexts (`ctx.provider_name`).

Definition of timestamps:
- **sent**: immediately before calling `model.stream(request).await`
- **received**: when that call returns `Ok(stream)` (stream handle created successfully)

---

## Backend: qbit-core (shared stats type)

### Files
- **New:** `backend/crates/qbit-core/src/api_request_stats.rs`
- **Edit:** `backend/crates/qbit-core/src/lib.rs`

### Implementation details
Create `ApiRequestStats` with:
- `tokio::sync::RwLock<HashMap<String, ProviderRequestStats>>`
- Public snapshot types (serde-serializable) returned to frontend

Public types (exported from `qbit-core`):
- `ApiRequestStats`
- `ApiRequestStatsSnapshot`
- `ProviderRequestStatsSnapshot`

Snapshot fields (snake_case):
- `requests: u64`
- `last_sent_at: Option<u64>` (unix millis)
- `last_received_at: Option<u64>` (unix millis)

Methods:
- `pub fn new() -> Self`
- `pub async fn record_sent(&self, provider: &str)`
- `pub async fn record_received(&self, provider: &str)`
- `pub async fn snapshot(&self) -> ApiRequestStatsSnapshot`

Time source:
- `SystemTime::now().duration_since(UNIX_EPOCH)` in millis (`u64`)

### Export wiring
In `qbit-core/src/lib.rs`:
1. Add module declaration:
   - `pub mod api_request_stats;`
2. Add re-export:
   - `pub use api_request_stats::{ApiRequestStats, ApiRequestStatsSnapshot, ProviderRequestStatsSnapshot};`

Downstream imports become:
- `use qbit_core::{ApiRequestStats, ApiRequestStatsSnapshot};`

---

## Backend: qbit-ai (per-session storage + main loop instrumentation)

### Files
- **Edit:** `backend/crates/qbit-ai/src/agent_bridge.rs`
- **Edit:** `backend/crates/qbit-ai/src/agentic_loop.rs`
- **Edit:** `backend/crates/qbit-ai/src/eval_support.rs` (3 `AgenticLoopContext { .. }` literals)
- **Edit:** `backend/crates/qbit-ai/src/test_utils.rs` (1 `AgenticLoopContext { .. }` literal + 6 `SubAgentExecutorContext { .. }` literals)

### 1) AgentBridge owns per-session stats

#### Add field
In `qbit-ai/src/agent_bridge.rs` `pub struct AgentBridge`:
- `pub(crate) api_request_stats: Arc<qbit_core::ApiRequestStats>,`

Add import:
- `use qbit_core::{ApiRequestStats, ApiRequestStatsSnapshot};`

#### Single init point (confirmed)
`AgentBridge` is constructed in exactly one place:
- `fn from_components_with_runtime(...) -> Self` (contains a `Self { ... }` struct literal)

Add in that `Self { ... }` literal:
- `api_request_stats: Arc::new(ApiRequestStats::new()),`

#### Add async accessor
In `impl AgentBridge`:
- `pub async fn get_api_request_stats_snapshot(&self) -> ApiRequestStatsSnapshot { self.api_request_stats.snapshot().await }`

### 2) Extend AgenticLoopContext and wire it

#### Add field
In `qbit-ai/src/agentic_loop.rs` `pub struct AgenticLoopContext<'a>`:
- `pub api_request_stats: &'a Arc<ApiRequestStats>,`

Add import:
- `use qbit_core::ApiRequestStats;`

#### Wire via build_loop_context
In `qbit-ai/src/agent_bridge.rs`, `fn build_loop_context` literal (anchor provided below), add:
- `api_request_stats: &self.api_request_stats,`

**Anchor:**
```rust
fn build_loop_context<'a>(
    &'a self,
    loop_event_tx: &'a mpsc::UnboundedSender<AiEvent>,
) -> AgenticLoopContext<'a> {
    AgenticLoopContext {
        event_tx: loop_event_tx,
        tool_registry: &self.tool_registry,
        ...
        coordinator: self.coordinator.as_ref(),
    }
}
```

### 3) Instrument main agent `.stream` boundary

In `qbit-ai/src/agentic_loop.rs`, at the stream request code (anchor below):

**Anchor:**
```rust
let stream_result = tokio::time::timeout(
    stream_timeout,
    async { model.stream(request).await }.instrument(llm_span.clone()),
)
.await;

let mut stream = match stream_result {
    Ok(Ok(s)) => {
        tracing::info!("[OpenAI Debug] Stream created successfully, consuming chunks...");
        s
    }
    ...
};
```

Add:
1) Immediately before the `timeout(...)` call:
- `ctx.api_request_stats.record_sent(ctx.provider_name).await;`

2) Inside `Ok(Ok(s)) => { ... }` at the top:
- `ctx.api_request_stats.record_received(ctx.provider_name).await;`

Do not record `received` on error branches.

### 4) Compile-fix updates: AgenticLoopContext literals
Adding `api_request_stats` to `AgenticLoopContext` requires updating all struct literals:

- `qbit-ai/src/agent_bridge.rs` (already covered)
- `qbit-ai/src/eval_support.rs` — **3** literals
- `qbit-ai/src/test_utils.rs` — **1** literal (`TestContext::as_agentic_context_with_client`)

#### eval_support.rs (3 literals)
For each `let ctx = AgenticLoopContext { ... }`:
- Create local: `let api_request_stats = Arc::new(ApiRequestStats::new());`
- Add in ctx literal: `api_request_stats: &api_request_stats,`
- Import `ApiRequestStats` from `qbit_core`

#### qbit-ai/src/test_utils.rs (AgenticLoopContext literal)
Update `TestContext` to own stats so the single helper can supply it:
- Add `pub api_request_stats: Arc<ApiRequestStats>,` to `TestContext`
- In `TestContextBuilder::build()` (the `TestContext { ... }` literal), add:
  - `api_request_stats: Arc::new(ApiRequestStats::new()),`
- In `as_agentic_context_with_client` `AgenticLoopContext { ... }` literal add:
  - `api_request_stats: &self.api_request_stats,`

---

## Backend: qbit-sub-agents (sub-agent instrumentation)

### Files
- **Edit:** `backend/crates/qbit-sub-agents/src/executor.rs`

### 1) Extend SubAgentExecutorContext
In `qbit-sub-agents/src/executor.rs`:
- Import:
  - `use qbit_core::ApiRequestStats;`

In `pub struct SubAgentExecutorContext<'a>` add:
- `pub api_request_stats: Option<&'a Arc<ApiRequestStats>>,`

### 2) Instrument sub-agent `.stream` boundary
At the exact stream code (anchor below):

**Anchor:**
```rust
let mut stream = match model.stream(request).await {
    Ok(s) => s,
    Err(e) => { ... }
};
```

Add:
1) Immediately before the match:
```rust
if let Some(stats) = ctx.api_request_stats {
    stats.record_sent(ctx.provider_name).await;
}
```

2) Change `Ok(s) => s,` to:
```rust
Ok(s) => {
    if let Some(stats) = ctx.api_request_stats {
        stats.record_received(ctx.provider_name).await;
    }
    s
}
```

Provider key is `ctx.provider_name`, which qbit-ai already sets correctly for override models.

---

## Backend: qbit-ai → qbit-sub-agents plumbing (3 production call sites)

### Files
- **Edit:** `backend/crates/qbit-ai/src/agentic_loop.rs`
- **Edit:** `backend/crates/qbit-ai/src/test_utils.rs` (6 call sites)

### 1) Update SubAgentExecutorContext literals in qbit-ai/src/agentic_loop.rs
There are **3** struct literals (override, override-fallback, and no-override).

**Anchor snippet:**
```rust
let sub_ctx = SubAgentExecutorContext {
    event_tx: ctx.event_tx,
    tool_registry: ctx.tool_registry,
    workspace: ctx.workspace,
    provider_name: override_provider,
    model_name: override_model,
    session_id: ctx.session_id,
    transcript_base_dir: ctx.transcript_base_dir,
};
```

Add to each literal:
- `api_request_stats: Some(ctx.api_request_stats),`

### 2) Update SubAgentExecutorContext literals in qbit-ai/src/test_utils.rs
There are **6** occurrences (grep confirmed).

For each of these literals, add:
- `api_request_stats: None, // tests`

This keeps test plumbing minimal.

---

## Backend: qbit (Tauri) command to fetch per-session stats

### Files
- **New:** `backend/crates/qbit/src/ai/commands/debug.rs`
- **Edit:** `backend/crates/qbit/src/ai/commands/mod.rs`
- **Edit:** `backend/crates/qbit/src/ai/mod.rs` (explicit re-export list)
- **Edit:** `backend/crates/qbit/src/lib.rs` (explicit imports + generate_handler)

### Command
Create `get_api_request_stats` command:

Signature:
- `pub async fn get_api_request_stats(state: State<'_, AppState>, session_id: String) -> Result<qbit_core::ApiRequestStatsSnapshot, String>`

Implementation:
- Use per-session bridge:
  - `state.ai_state.get_session_bridge(&session_id).await`
- If missing, return:
  - `super::ai_session_not_initialized_error(&session_id)`
- Else return:
  - `bridge.get_api_request_stats_snapshot().await`

### Wiring
1) `qbit/src/ai/commands/mod.rs`:
- `pub mod debug;`
- `pub use debug::*;`

2) `qbit/src/ai/mod.rs`:
- Add `get_api_request_stats,` to the explicit `pub use commands::{ ... }` list.

3) `qbit/src/lib.rs`:
- Add `get_api_request_stats,` to the explicit `use ai::{ ... }` list.
- Add `get_api_request_stats,` to `tauri::generate_handler![ ... ]`.

---

## Frontend: API wrapper + Debug badge UI

### Files
- **Edit:** `frontend/lib/ai.ts`
- **Edit:** `frontend/components/UnifiedInput/InputStatusRow.tsx`

### 1) Add TS types + invoke wrapper
In `frontend/lib/ai.ts`:

```ts
export interface ProviderRequestStats {
  requests: number;
  last_sent_at: number | null;
  last_received_at: number | null;
}

export interface ApiRequestStatsSnapshot {
  providers: Record<string, ProviderRequestStats>;
}

export async function getApiRequestStats(sessionId: string): Promise<ApiRequestStatsSnapshot> {
  return invoke("get_api_request_stats", { sessionId });
}
```

### 2) Add Debug badge next to Langfuse badge
In `frontend/components/UnifiedInput/InputStatusRow.tsx`:

#### Imports
- Add `Bug` to lucide import list:
  - `import { Bot, Cpu, Gauge, Terminal, Bug } from "lucide-react";`
- Add to the existing `@/lib/ai` import block:
  - `getApiRequestStats`
  - `type ApiRequestStatsSnapshot`

#### State
Add:
- `const [debugOpen, setDebugOpen] = useState(false);`
- `const debugPollRef = useRef<ReturnType<typeof setInterval> | null>(null);`
- `const [apiRequestStats, setApiRequestStats] = useState<ApiRequestStatsSnapshot | null>(null);`
- `const [apiRequestStatsError, setApiRequestStatsError] = useState<string | null>(null);`

#### Refresh function
Create `refreshApiRequestStats` callback bound to `sessionId`.

**Expected error handling (locked):**
Backend error string is produced by:
```rust
"AI agent not initialized for session '{}'. Call init_ai_session first."
```

Frontend should treat as expected if:
- `msg.includes("AI agent not initialized for session")` OR
- `msg.includes("Call init_ai_session first")`

On expected:
- `setApiRequestStats(null);`
- `setApiRequestStatsError(null);`
- do not log

On unexpected:
- `setApiRequestStatsError(msg);`

#### Poll only while popover open
Use controlled popover state:
- `<Popover open={debugOpen} onOpenChange={setDebugOpen}>`

Effect:
- If not `import.meta.env.DEV`, do nothing.
- When `debugOpen` becomes true:
  - refresh once immediately
  - set interval 1500ms
- When it becomes false or on unmount:
  - clear interval

#### Placement
Insert the Debug badge block **immediately after** the Langfuse badge block in the left badge row.

Render gate:
- `import.meta.env.DEV && !isMockBrowserMode()` (recommended)

#### Badge display
- Show label: `Debug`
- Optionally show total requests (sum across providers) if > 0.

#### Popover content
Title:
- `Debug (This Tab)`

Subtitle:
- `LLM API Requests (main + sub-agents)`

Table columns:
- Provider | Req | Sent | Recv

Sorting:
- requests desc, then provider asc.

Time formatting:
- display relative (e.g. `12s`, `3m`, `1h`, `2d`), with tooltip `new Date(ms).toLocaleString()`.
- if delta negative, show `0s`.
- if timestamp null, show `—`.

---

## Verification checklist

### Build/test
- `just check`
- `just test-rust`

### Manual
1. Open two tabs (two sessionIds).
2. Init AI in tab A only; run a prompt.
3. Open Debug popover in tab A:
   - provider row exists
   - requests increments
   - last_sent_at and last_received_at appear
4. Tab B (no AI): open Debug popover:
   - shows “No AI agent initialized…” (no red error, no log spam)
5. Trigger a prompt that invokes a sub-agent tool (e.g. `sub_agent_coder`):
   - counts increase (same session)
6. If sub-agent has model override provider, confirm provider key differs and counts split.

---

## Exhaustive compilation breakpoints (must update)

### Adding `api_request_stats` to `AgenticLoopContext` requires updating these literals:
- `qbit-ai/src/agent_bridge.rs` (build_loop_context)
- `qbit-ai/src/eval_support.rs` (3 literals)
- `qbit-ai/src/test_utils.rs` (1 literal)

### Adding `api_request_stats` to `SubAgentExecutorContext` requires updating these literals:
- `qbit-ai/src/agentic_loop.rs` (3 literals)
- `qbit-ai/src/test_utils.rs` (6 literals)

No other occurrences were found by grep.
