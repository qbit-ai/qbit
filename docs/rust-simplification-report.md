# Rust Codebase Simplification Report

Generated: 2025-12-16

This report analyzes the Rust backend (`src-tauri/src/`) for simplification opportunities. The codebase was analyzed by module, identifying code duplication, unnecessary complexity, and areas that could be made more idiomatic.

---

## Table of Contents

1. [AI Module](#1-ai-module)
2. [PTY Module](#2-pty-module)
3. [Sidecar Module](#3-sidecar-module)
4. [Settings Module](#4-settings-module)
5. [Commands Module](#5-commands-module)
6. [CLI Module & lib.rs](#6-cli-module--librs)
7. [Prioritized Action Items](#prioritized-action-items)

---

## 1. AI Module

**Path**: `/src-tauri/src/ai/`

The AI module is a well-structured codebase with clear separation of concerns, but it exhibits several opportunities for simplification, including significant code duplication between the Vertex and generic agentic loops, overly fragmented extension modules, and transitional dual-path event emission that could be consolidated.

### 1.1 Massive Code Duplication: `run_agentic_loop` vs `run_agentic_loop_generic`

**Files**: `ai/agentic_loop.rs`

**Current**: Lines 529-973 (`run_agentic_loop`) and lines 1251-1623 (`run_agentic_loop_generic`) are nearly identical implementations - approximately 400+ lines of duplicated logic. The same duplication exists for `execute_with_hitl` (lines 143-334) vs `execute_with_hitl_generic` (lines 1054-1243) and `execute_tool_direct` (lines 337-447) vs `execute_tool_direct_generic` (lines 977-1051).

**Problem**: This is the single largest source of technical debt in the module. When a bug is fixed or feature added to one loop, it must be manually synchronized to the other. The only real differences are:
- Generic loop does not support extended thinking (reasoning blocks in history)
- Generic loop does not support sub-agent calls
- Generic loop uses `execute_tool_direct_generic` instead of `execute_tool_direct`

**Suggested**: Refactor into a single generic function with trait-based customization:

```rust
// Define what varies between implementations
pub trait AgenticModelCapabilities {
    fn supports_thinking(&self) -> bool { false }
    fn supports_sub_agents(&self) -> bool { false }
}

impl AgenticModelCapabilities for rig_anthropic_vertex::CompletionModel {
    fn supports_thinking(&self) -> bool { true }
    fn supports_sub_agents(&self) -> bool { true }
}

impl<M: RigCompletionModel + Sync> AgenticModelCapabilities for M {
    // Default implementations (no thinking, no sub-agents)
}

pub async fn run_agentic_loop_unified<M>(
    model: &M,
    system_prompt: &str,
    initial_history: Vec<Message>,
    context: SubAgentContext,
    ctx: &AgenticLoopContext<'_>,
) -> Result<(String, Vec<Message>)>
where
    M: RigCompletionModel + AgenticModelCapabilities + Sync,
{
    // Single implementation with capability checks
    let supports_thinking = model.supports_thinking();
    let supports_sub_agents = model.supports_sub_agents();

    // ... rest of implementation with conditional branches
}
```

**Effort**: High
**Impact**: Eliminates ~600+ lines of duplicated code, reduces maintenance burden significantly

---

### 1.2 Fragmented AgentBridge Extension Modules

**Files**:
- `ai/bridge_session.rs` (141 lines)
- `ai/bridge_hitl.rs` (82 lines)
- `ai/bridge_context.rs` (89 lines)
- `ai/bridge_policy.rs` (97 lines)

**Current**: These four files implement methods on `AgentBridge` through `impl AgentBridge` blocks in separate files. Each file is small (80-140 lines) and the separation adds cognitive overhead without clear architectural benefit.

**Problem**:
- Requires navigating 5+ files to understand the full AgentBridge API
- The modules are pure delegations - they add no logic, just forward to inner components
- This pattern fragments the type's interface unnecessarily

**Suggested**: Consolidate into `agent_bridge.rs` itself, organized by sections:

```rust
// In agent_bridge.rs
impl AgentBridge {
    // ========================================================================
    // Session Methods (from bridge_session.rs)
    // ========================================================================
    pub async fn set_session_persistence_enabled(&self, enabled: bool) { ... }
    pub async fn clear_conversation_history(&self) { ... }
    // ... other session methods

    // ========================================================================
    // HITL Methods (from bridge_hitl.rs)
    // ========================================================================
    pub async fn get_approval_patterns(&self) -> Vec<ApprovalPattern> { ... }
    // ... other HITL methods

    // etc.
}
```

**Effort**: Low-Medium
**Impact**: Improves code navigation, reduces file count, makes the AgentBridge API discoverable

---

### 1.3 Transitional Dual-Path Event Emission

**File**: `ai/agent_bridge.rs` (lines 68-69, 307-357)

**Current**:
```rust
pub(crate) event_tx: Option<mpsc::UnboundedSender<AiEvent>>,
pub(crate) runtime: Option<Arc<dyn QbitRuntime>>,

pub(crate) fn emit_event(&self, event: AiEvent) {
    // Emit through legacy event_tx channel if available
    if let Some(ref tx) = self.event_tx {
        let _ = tx.send(event.clone());
    }
    // Emit through runtime abstraction if available
    if let Some(ref rt) = self.runtime {
        if let Err(e) = rt.emit(RuntimeEvent::Ai(Box::new(event))) {
            tracing::warn!(...);
        }
    }
}
```

**Problem**:
- Code maintains two parallel event paths during a migration
- `get_or_create_event_tx()` spawns a new task every time it's called when only runtime exists
- The comment says "After migration is complete, only runtime will be used" - the migration should be completed

**Suggested**: Complete the migration and remove the dual-path:
- Choose one abstraction (runtime appears to be the target)
- Remove `event_tx` field entirely
- Update `AgenticLoopContext` to accept `&dyn QbitRuntime` instead of `&mpsc::UnboundedSender<AiEvent>`
- Remove `get_or_create_event_tx()` method

**Effort**: Medium
**Impact**: Removes transitional complexity, eliminates per-call task spawning

---

### 1.4 Nearly Identical Constructor Pairs

**File**: `ai/agent_bridge.rs` (lines 120-209)

**Current**: Four constructors that are nearly identical:
- `new()` and `new_with_runtime()`
- `new_vertex_anthropic()` and `new_vertex_anthropic_with_runtime()`

And two `from_components_*` methods (lines 212-301) that are identical except for setting `event_tx` vs `runtime`.

**Problem**: 80+ lines of duplicated boilerplate code

**Suggested**: Use a builder pattern or consolidate:

```rust
impl AgentBridge {
    /// Create builder for vtcode client
    pub async fn vtcode(workspace: PathBuf, provider: &str, model: &str, api_key: &str) -> Result<AgentBridgeBuilder> {
        let components = create_vtcode_components(VtcodeClientConfig { ... }).await?;
        Ok(AgentBridgeBuilder { components })
    }

    /// Create builder for Vertex AI
    pub async fn vertex_anthropic(...) -> Result<AgentBridgeBuilder> { ... }
}

pub struct AgentBridgeBuilder {
    components: AgentBridgeComponents,
}

impl AgentBridgeBuilder {
    pub fn with_event_tx(self, tx: mpsc::UnboundedSender<AiEvent>) -> AgentBridge { ... }
    pub fn with_runtime(self, runtime: Arc<dyn QbitRuntime>) -> AgentBridge { ... }
}
```

**Effort**: Medium
**Impact**: Reduces constructor duplication, makes API more flexible

---

### 1.5 Redundant `execute_with_vertex_model` and `execute_with_openrouter_model`

**File**: `ai/agent_bridge.rs` (lines 583-886)

**Current**: `execute_with_vertex_model()` (lines 583-740) and `execute_with_openrouter_model()` (lines 743-886) share roughly 80% of their code. The main differences:
- Vertex stores sidecar session ID in AI session manager
- OpenRouter ends the sidecar session at the end
- They call `run_agentic_loop` vs `run_agentic_loop_generic`

**Problem**: ~300 lines with substantial duplication

**Suggested**: Extract common logic into a helper:

```rust
async fn prepare_execution(&self, initial_prompt: &str) -> ExecutionSetup {
    // System prompt building
    // Session context injection
    // Session starting
    // User prompt sidecar capture
    // Initial history preparation
    // AgenticLoopContext building
}

async fn finalize_execution(&self, response: &str, duration_ms: u64, sidecar_behavior: SidecarBehavior) {
    // Response persistence
    // Session recording
    // Sidecar capture
    // Completion event emission
}
```

**Effort**: Medium
**Impact**: Reduces duplication by ~150 lines

---

### 1.6 Quick Wins

#### Simplify `execute_with_context` Match
**File**: `ai/agent_bridge.rs` (lines 495-539)

Manual cloning and dropping is verbose. Use a local scope for cleaner lock management.

#### Remove Unused `#[allow(dead_code)]` Suppression
**File**: `ai/agent_bridge.rs` (line 16)

`#![allow(dead_code)]` at module level suppresses warnings for the entire module.

#### Consolidate Tool Execution Pattern
**File**: `ai/agentic_loop.rs` (lines 337-447)

Multiple if-else checks for different tool types. Use a match with tool categories or a dispatch table.

#### Use `?` Operator in Error Paths
**File**: `ai/tool_executors.rs` (lines 84-88, 93-94, etc.)

Replace verbose match blocks with `?` operator.

#### Simplify `mod.rs` Re-exports
**File**: `ai/mod.rs` (lines 46-88)

Many `#[allow(unused_imports)]` annotations for public API types.

---

### 1.7 Items That Look Complex But Serve a Purpose

1. **ToolSource Enum**: The `ToolSource` enum in events.rs with Main/SubAgent/Workflow variants enables proper attribution of tool calls in the UI. This complexity is warranted.

2. **ApprovalRecorder Persistence**: The JSON persistence in `approval_recorder.rs` with version field supports future migrations. This forward-thinking design is appropriate.

3. **Workflow Module Structure**: The graph-flow based workflow system in `workflow/` is well-architected for extensibility.

4. **Context Manager Orchestration**: The separation of `TokenBudgetManager`, `ContextPruner`, and `ContextManager` follows good SRP principles.

---

## 2. PTY Module

**Path**: `/src-tauri/src/pty/`

The PTY module is reasonably well-structured but contains several opportunities for simplification: a deprecated API path that adds maintenance burden, overly verbose emitter abstraction, redundant field storage in `ActiveSession`, and a custom URL decoder that could use a well-tested crate.

### 2.1 Remove Deprecated `create_session` Method and `AppHandleEmitter`

**File**: `pty/manager.rs` (lines 64-113, 400-414)

**Current**: The code maintains two parallel event emission paths:
- `AppHandleEmitter` wrapping `AppHandle` directly
- `RuntimeEmitter` wrapping `Arc<dyn QbitRuntime>`

The `create_session` method is marked `#[deprecated]` and the comment says to use `create_session_with_runtime()` with `TauriRuntime` instead.

**Problem**: Maintaining deprecated code adds cognitive load and increases the surface area for bugs. The `AppHandleEmitter` struct (lines 71-112) is ~40 lines of code that duplicates `TauriRuntime`'s event emission logic.

**Suggested**:
1. Search for any remaining callers of `create_session`
2. Migrate them to `create_session_with_runtime`
3. Delete `AppHandleEmitter`, the `PtyEventEmitter` trait, and the deprecated method

This would remove approximately 90 lines of code while simplifying the API surface.

**Effort**: Medium

---

### 2.2 Inline the `PtyEventEmitter` Trait

**File**: `pty/manager.rs` (lines 49-62)

**Current**: The `PtyEventEmitter` trait exists as an internal abstraction with only two implementations, and after removing the deprecated path, would have only one.

**Problem**: This trait adds a layer of indirection. The `QbitRuntime` trait already provides the `emit()` method, and `RuntimeEmitter` is just a thin wrapper.

**Suggested**: After removing `AppHandleEmitter`, either:
- Keep `RuntimeEmitter` as a simple helper struct without the trait
- Or inline the emission logic directly in the read thread closure using `Arc<dyn QbitRuntime>` directly

**Effort**: Low (after removing deprecated path)

---

### 2.3 Use `percent_encoding` Crate Instead of Custom Decoder

**File**: `pty/parser.rs` (lines 225-252)

**Current**: A custom `urlencoding_decode` function handles URL decoding (~27 lines).

**Problem**:
1. The implementation has subtle bugs (incomplete `%2` produces a control character)
2. It doesn't handle UTF-8 multi-byte sequences correctly
3. Could be replaced by a well-tested crate

**Suggested**: Use the `percent-encoding` crate:

```rust
fn urlencoding_decode(input: &str) -> String {
    percent_encoding::percent_decode_str(input)
        .decode_utf8_lossy()
        .into_owned()
}
```

**Effort**: Low

---

### 2.4 Consolidate `ActiveSession` Fields with Simpler Locking

**File**: `pty/manager.rs` (lines 199-208)

**Current**: `ActiveSession` has multiple individually-locked fields:

```rust
struct ActiveSession {
    #[allow(dead_code)]
    child: Mutex<Box<dyn Child + Send + Sync>>,
    master: Arc<Mutex<Box<dyn MasterPty + Send>>>,
    writer: Mutex<Box<dyn Write + Send>>,
    working_directory: Mutex<PathBuf>,
    rows: Mutex<u16>,
    cols: Mutex<u16>,
}
```

**Problem**:
1. The `child` field is never used (has `#[allow(dead_code)]`)
2. Four separate mutexes for fields that are often accessed together
3. `master` is wrapped in both `Arc` and `Mutex` redundantly

**Suggested**:
1. Remove the unused `child` field (or document why it must be kept for ownership)
2. Group related fields into a single struct with one mutex

**Effort**: Medium

---

### 2.5 Simplify Working Directory Resolution Logic

**File**: `pty/manager.rs` (lines 264-294)

**Current**: Complex nested fallback logic with multiple environment variable checks.

**Problem**: Deeply nested conditionals are hard to follow. The logic has several concerns mixed together (tilde expansion, env var priority, src-tauri adjustment).

**Suggested**: Extract to a helper function with clearer structure:

```rust
fn resolve_working_directory(explicit: Option<PathBuf>) -> PathBuf {
    explicit
        .or_else(|| env_var_path("QBIT_WORKSPACE").map(expand_tilde))
        .or_else(|| env_var_path("INIT_CWD"))
        .or_else(|| std::env::current_dir().ok().map(adjust_for_src_tauri))
        .or_else(dirs::home_dir)
        .unwrap_or_else(|| PathBuf::from("/"))
}
```

**Effort**: Low

---

### 2.6 Quick Wins

- Remove `#![allow(unused)]` at module level in `parser.rs`
- Simplify `OscEvent::to_command_block_event` using builder pattern
- Remove redundant `#[allow(unused_imports)]` in `mod.rs`
- Use `matches!` macro consistently in tests

---

## 3. Sidecar Module

**Path**: `/src-tauri/src/sidecar/`

The sidecar module contains approximately 9,000 lines of Rust code across 12 files. The code is generally well-structured but has significant duplication in LLM backend implementations, overly complex async patterns using blocking threads, and several abstractions that add complexity without proportional benefit.

### 3.1 Duplicate LLM Backend Implementations

**Files**: `sidecar/synthesis.rs`, `sidecar/artifacts.rs`

**Current**: Both files implement nearly identical HTTP client code for OpenAI, Grok, and Vertex AI backends. Each file has separate synthesizer structs and implementations.

**Problem**: Approximately 400+ lines of duplicated HTTP request code across two files. Changes to API handling must be made in multiple places.

**Suggested**: Extract a generic LLM client module:

```rust
// Proposed: shared llm_client.rs
pub struct LlmClient {
    backend: LlmBackend,
    config: LlmConfig,
}

impl LlmClient {
    pub async fn complete(&self, system_prompt: &str, user_prompt: &str) -> Result<String> {
        match self.backend {
            LlmBackend::OpenAi => self.openai_complete(system_prompt, user_prompt).await,
            LlmBackend::Grok => self.grok_complete(system_prompt, user_prompt).await,
            LlmBackend::VertexAnthropic => self.vertex_complete(system_prompt, user_prompt).await,
        }
    }
}
```

**Effort**: Medium
**Impact**: Eliminates ~400 lines of duplicated code

---

### 3.2 Blocking Thread Spawns for Async Work

**File**: `sidecar/state.rs` (lines 221-235, 277-299, 335-351, 499-510)

**Current**: `start_session`, `resume_session`, `end_session`, and `shutdown` all spawn new threads with their own Tokio runtimes:

```rust
std::thread::spawn(move || {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    rt.block_on(async { /* async work */ });
})
```

**Problem**: Creates unnecessary threads and runtimes. Each operation spawns a new OS thread and builds a new Tokio runtime.

**Suggested**: Either:
1. Make these methods `async` (preferred)
2. Use `tokio::task::spawn_blocking` with the existing runtime
3. Use `Handle::current().block_on()` if already on a Tokio runtime

**Effort**: Medium

---

### 3.3 Triple-Redundant Synthesizer Trait Hierarchies

**File**: `sidecar/synthesis.rs`

**Current**: Three separate trait hierarchies with nearly identical structure:
- `CommitMessageSynthesizer` (lines 373-380)
- `StateSynthesizer` (lines 726-734)
- `SessionTitleSynthesizer` (lines 1201-1204)

Each has 4 implementations (Template, OpenAI, Grok, VertexAnthropic), totaling 12 struct definitions.

**Problem**: ~800 lines of boilerplate.

**Suggested**: Use a single generic approach or function-based design with the shared `LlmClient`.

**Effort**: High

---

### 3.4 Hand-Rolled Diff Algorithm

**File**: `sidecar/commits.rs` (lines 845-1064)

**Current**: A hand-rolled line diff algorithm spanning 220 lines including `compute_line_diff`, `find_match_ahead`, `DiffOp`, and `generate_diff_hunks`.

**Problem**: The codebase already uses the `similar` crate in `capture.rs`. This is a partial reimplementation.

**Suggested**: Use `similar` crate consistently:

```rust
fn generate_diff_from_strings(file_path: &str, old: &str, new: &str) -> String {
    use similar::TextDiff;

    let diff = TextDiff::from_lines(old, new);
    let mut output = String::new();

    writeln!(output, "diff --git a/{} b/{}", file_path, file_path).ok();
    writeln!(output, "--- a/{}", file_path).ok();
    writeln!(output, "+++ b/{}", file_path).ok();

    for hunk in diff.unified_diff().context_radius(3).iter_hunks() {
        write!(output, "{}", hunk).ok();
    }
    output
}
```

**Effort**: Low
**Impact**: Eliminates ~200 lines

---

### 3.5 Redundant Path Extraction Functions

**Files**: `sidecar/capture.rs`, `sidecar/processor.rs`

**Current**: Multiple functions for extracting paths from JSON args with duplicated logic.

**Suggested**: Consolidate into a single module with clear functions.

**Effort**: Low

---

### 3.6 Empty Module

**File**: `sidecar/formats.rs`

**Current**: 9 lines including only comments, marked as a placeholder.

**Suggested**: Delete the file and remove from `mod.rs`.

**Effort**: Trivial

---

### 3.7 Quick Wins

- Consolidate `SynthesisBackend` and `ArtifactSynthesisBackend` enums
- Remove unused `#[allow(dead_code)]` annotations
- Simplify `SidecarConfig::from_qbit_settings` construction
- Use `?` more consistently for error handling

---

## 4. Settings Module

**Path**: `/src-tauri/src/settings/`

The settings module is reasonably well-structured for a TOML-based configuration system. However, there are several areas where redundancy and unnecessary complexity have crept in.

### 4.1 Duplicate API Key Structs

**File**: `settings/schema.rs` (lines 96-124)

**Current**: `OpenRouterSettings`, `AnthropicSettings`, and `SynthesisGrokSettings` are identical single-field structs containing only `api_key: Option<String>`.

**Problem**: Unnecessary type proliferation with no semantic benefit.

**Suggested**: Either collapse into a single generic `ApiKeyConfig` struct, or inline the `Option<String>` directly.

**Effort**: Medium (requires TOML migration)

---

### 4.2 Manual Environment Variable Resolution

**File**: `settings/loader.rs` (lines 71-102)

**Current**: Manual plumbing for each field:
```rust
fn resolve_env_vars(settings: &mut QbitSettings) {
    resolve_opt(&mut settings.ai.vertex_ai.credentials_path);
    resolve_opt(&mut settings.ai.vertex_ai.project_id);
    // ... 6 more calls
}
```

**Problem**: Error-prone. Every new field requires manual addition. The synthesis settings are NOT being resolved (likely a bug).

**Suggested**: Use a recursive visitor on the TOML value before deserializing.

**Effort**: Medium

---

### 4.3 Dual-Layer Environment Fallback Confusion

**Current**: TWO env var resolution mechanisms:
1. `resolve_env_ref()` - resolves `$VAR` syntax in TOML values during load
2. `get_with_env_fallback()` - called at usage sites if setting is None

**Problem**: Confusion about where env vars are resolved. Edge cases like `$VAR` when env var is missing leaves literal string.

**Suggested**: Pick one approach and be consistent.

**Effort**: Low

---

### 4.4 String-Typed Enums

**File**: `settings/schema.rs`

**Current**: `default_provider`, `theme`, `log_level` are all `String` with comment documentation of valid values.

**Problem**: No compile-time safety. Invalid values silently pass through.

**Suggested**: Use proper enums with `#[serde(rename_all = "snake_case")]`.

**Effort**: Low

---

### 4.5 Synthesis Settings Duplication

**File**: `settings/schema.rs` (lines 291-337)

**Current**: `SynthesisVertexSettings`, `SynthesisOpenAiSettings`, `SynthesisGrokSettings` duplicate fields from main provider settings.

**Suggested**: Remove duplicate structs. Have sidecar reference main provider config based on `synthesis_backend`.

**Effort**: Medium

---

### 4.6 Quick Wins

- Fix `resolve_env_ref` clarity for `${VAR}` format
- Reduce redundant locking in `set_value`
- Remove unnecessary `#[serde(default)]` on Default-derived structs

---

## 5. Commands Module

**Path**: `/src-tauri/src/commands/`

The commands module is generally well-structured with clean Tauri command implementations. However, there are several opportunities for simplification.

### 5.1 Inconsistent Error Handling Strategy

**Files**: `commands/themes.rs` vs other command files

**Current**: `themes.rs` returns `Result<..., String>` while `pty.rs`, `files.rs`, and `prompts.rs` use `crate::error::Result<T>` (which wraps `QbitError`).

**Problem**: Inconsistency makes the codebase harder to reason about.

**Suggested**: Migrate `themes.rs` to use `QbitError`.

**Effort**: Low

---

### 5.2 Duplicated Directory Resolution Pattern

**Files**: `commands/shell.rs` (lines 130-144), `commands/themes.rs` (lines 12-24)

**Current**: Multiple files have nearly identical helper functions to resolve qbit directories.

**Problem**: Different approaches (config_dir vs home_dir) with potential confusion.

**Suggested**: Create a shared `paths` module with consistent directory resolution.

**Effort**: Medium

---

### 5.3 Duplicated Prompt File Reading Logic

**File**: `commands/prompts.rs` (lines 23-70)

**Current**: The directory reading logic for global and local prompts is duplicated.

**Suggested**: Extract a helper function:

```rust
fn read_prompts_from_dir(dir: &Path, source: &str, prompts: &mut HashMap<String, PromptInfo>) {
    // ...
}
```

**Effort**: Low

---

### 5.4 Complex zshrc Update Logic

**File**: `commands/shell.rs` (lines 267-345)

**Current**: 78 lines of complex state machine for parsing .zshrc. The `found_and_replaced` variable appears to serve no purpose.

**Suggested**: Simplify to filter-and-append approach:

```rust
let filtered: Vec<&str> = content
    .lines()
    .filter(|line| {
        !line.contains("# Qbit shell integration")
            && !line.contains("qbit/integration.zsh")
    })
    .collect();
// ... append new integration
```

**Effort**: Medium

---

### 5.5 Quick Wins

- Use `let else` guards for cleaner early returns
- Use `sort_by_key` instead of `sort_by` with closure
- Consider functional style with `filter_map` in loops

---

## 6. CLI Module & lib.rs

**Path**: `/src-tauri/src/cli/`, `/src-tauri/src/lib.rs`

The CLI module is generally well-structured with clean separation of concerns. However, there are several opportunities for simplification.

### 6.1 Duplicate `truncate` Functions

**File**: `cli/output.rs`

**Current**: Two nearly identical `truncate` functions:
- `truncate_output` (lines 451-457): Unicode-aware
- `truncate` (lines 770-776): Byte-based with ellipsis

And a third in `cli/runner.rs` (lines 121-127).

**Problem**: Three truncation functions with slightly different semantics.

**Suggested**: Consolidate into a single function.

**Effort**: Low

---

### 6.2 Repetitive `convert_to_cli_json` Match Arms

**File**: `cli/output.rs` (lines 88-436)

**Current**: 35+ match arms that all follow the same pattern (~350 lines).

**Problem**: Adding a new `AiEvent` variant requires adding another match arm with the exact same structure.

**Suggested**: Use serde's built-in serialization with field renaming.

**Effort**: Medium

---

### 6.3 Duplicate Path Expansion Logic

**File**: `lib.rs` (lines 110-116, 186-193)

**Current**: Home directory expansion (`~/` to absolute path) is duplicated.

**Suggested**: Extract to a utility function:

```rust
pub fn expand_tilde(path: &str) -> PathBuf {
    if path.starts_with("~/") {
        dirs::home_dir()
            .map(|home| home.join(&path[2..]))
            .unwrap_or_else(|| PathBuf::from(path))
    } else {
        PathBuf::from(path)
    }
}
```

**Effort**: Low

---

### 6.4 Overly Verbose Command Registration

**File**: `lib.rs` (lines 214-364)

**Current**: The `tauri::generate_handler![]` macro call spans ~150 lines with 150+ command names.

**Problem**: Hard to maintain. Command groupings are indicated by comments but there's no enforcement.

**Suggested**: Consider auditing command usage - some may be unused or redundant.

**Effort**: Medium

---

### 6.5 Unused Fields

**File**: `runtime/cli.rs` (line 12-13)

**Current**: `quiet_mode` field marked with `#[allow(dead_code)]`.

**Suggested**: Either use it or remove it.

**Effort**: Low

---

### 6.6 Redundant `ReplCommand::Empty` Variant

**File**: `cli/repl.rs`

**Current**: `Empty` variant only used to `continue` the loop.

**Suggested**: Handle empty input inline in `run_repl` instead.

**Effort**: Low

---

### 6.7 Quick Wins

- Remove `format_args_summary` function (only used in tests)
- Consolidate Box-Drawing constants (`BOX_TOP` and `BOX_BOT` are identical)
- Remove redundant import guard tests in `server/mod.rs`
- Add helper method for bridge access pattern in `CliContext`

---

## Prioritized Action Items

The following table lists all identified simplifications sorted by priority (combination of impact and effort). Priority 1 is highest.

| Priority | Module | Issue | Impact | Effort | Lines Saved |
|----------|--------|-------|--------|--------|-------------|
| 1 | AI | Agentic loop duplication (`run_agentic_loop` vs `run_agentic_loop_generic`) | Critical | High | ~600 |
| 2 | Sidecar | Duplicate LLM backend implementations (synthesis.rs + artifacts.rs) | High | Medium | ~400 |
| 3 | Sidecar | Hand-rolled diff algorithm (use `similar` crate) | High | Low | ~200 |
| 4 | AI | Dual-path event emission (complete migration to runtime) | High | Medium | ~50 |
| 5 | AI | Fragmented bridge extension files (consolidate 4 files) | Medium | Low | N/A |
| 6 | AI | Redundant execute methods (`execute_with_vertex_model`/`execute_with_openrouter_model`) | Medium | Medium | ~150 |
| 7 | Settings | String-typed enums (use proper enums) | Medium | Low | N/A |
| 8 | Settings | Dual-layer env var resolution confusion | Medium | Low | N/A |
| 9 | PTY | Deprecated `create_session` and `AppHandleEmitter` | Medium | Medium | ~90 |
| 10 | Sidecar | Blocking thread spawns in state.rs | Medium | Medium | ~40 |
| 11 | CLI | Three duplicate truncate functions | Medium | Low | ~30 |
| 12 | CLI | Repetitive `convert_to_cli_json` match arms | Medium | Medium | ~200 |
| 13 | Commands | Inconsistent error handling (String vs QbitError) | Medium | Low | N/A |
| 14 | PTY | Custom URL decoder (use `percent-encoding` crate) | Low | Low | ~25 |
| 15 | Commands | Duplicated directory resolution pattern | Low | Medium | ~30 |
| 16 | AI | Nearly identical constructor pairs (use builder) | Low | Medium | ~80 |
| 17 | Settings | Manual `resolve_env_vars` plumbing | Low | Medium | N/A |
| 18 | Settings | Synthesis settings duplication | Low | Medium | ~50 |
| 19 | PTY | Consolidate `ActiveSession` mutex fields | Low | Medium | N/A |
| 20 | Commands | Complex zshrc update logic | Low | Medium | ~30 |
| 21 | Commands | Duplicated prompt file reading logic | Low | Low | ~20 |
| 22 | CLI | Duplicate path expansion in lib.rs | Low | Low | ~10 |
| 23 | Sidecar | Empty `formats.rs` module | Trivial | Trivial | 9 |
| 24 | Multiple | Remove `#[allow(dead_code)]` and `#[allow(unused)]` suppressions | Trivial | Trivial | N/A |
| 25 | CLI | Unused `quiet_mode` field | Trivial | Trivial | ~5 |
| 26 | Sidecar | Redundant path extraction functions | Low | Low | ~30 |
| 27 | PTY | Simplify working directory resolution | Low | Low | ~15 |
| 28 | CLI | Verbose command registration in lib.rs | Low | Medium | N/A |
| 29 | Sidecar | Triple-redundant synthesizer traits | Medium | High | ~300 |
| 30 | Settings | Duplicate API key structs | Low | Medium | ~30 |

### Recommended Implementation Order

**Phase 1: Quick Wins (1-2 days)**
- Delete `sidecar/formats.rs`
- Replace sidecar diff algorithm with `similar` crate
- Consolidate truncate functions in CLI
- Remove module-level `#[allow(...)]` suppressions
- Fix string-typed enums in settings

**Phase 2: Medium Effort (3-5 days)**
- Extract shared LLM client for sidecar
- Complete AI module event emission migration
- Consolidate AI bridge extension files
- Migrate commands/themes.rs to QbitError
- Create shared paths module for commands

**Phase 3: High Effort (1-2 weeks)**
- Unify agentic loop implementations
- Refactor sidecar state.rs async patterns
- Consider synthesizer trait consolidation

---

## Appendix: Files by Lines of Duplicated/Redundant Code

| File | Estimated Redundant Lines |
|------|---------------------------|
| `ai/agentic_loop.rs` | ~600 |
| `sidecar/synthesis.rs` | ~400 |
| `cli/output.rs` | ~230 |
| `sidecar/commits.rs` | ~200 |
| `ai/agent_bridge.rs` | ~150 |
| `pty/manager.rs` | ~90 |
| `settings/schema.rs` | ~80 |
| Other files | ~150 |
| **Total** | **~1,900** |
