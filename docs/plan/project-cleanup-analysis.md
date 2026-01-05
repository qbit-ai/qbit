# Project Cleanup Analysis

**Date**: 2026-01-05
**Status**: Active

## Summary

Analysis of the Qbit codebase identified several opportunities for cleanup, simplification, and improvement. This document captures findings and recommendations without proposing new features.

---

## High Priority Issues

### 1. LLM API Code Duplication (~1000 lines)

**Location**: `qbit-synthesis/src/lib.rs` and `qbit-artifacts/src/lib.rs`

Both crates contain nearly identical code for:
- OpenAI API calls (lines 385-446 in both)
- Grok/xAI API calls (lines 449-504 in both)
- Vertex AI Anthropic calls (lines 508-573 in both)
- Backend enums (`SynthesisBackend` vs `ArtifactSynthesisBackend`)
- Config structs (`SynthesisConfig` vs `ArtifactSynthesisConfig`)

**Recommendation**: Create `qbit-llm-client` crate with:
```rust
// Unified LLM client
pub enum LlmBackend { Template, Vertex, OpenAi, Grok }
pub trait LlmClient { async fn call(&self, prompt: &str) -> Result<String>; }
pub struct OpenAiClient { ... }
pub struct GrokClient { ... }
pub struct VertexAnthropicClient { ... }
```

**Impact**: Remove ~800-1000 lines of duplicated code.

---

### 2. qbit-artifacts Disabled (2224 lines)

**Location**: `qbit-artifacts/src/lib.rs:1`
```rust
#![allow(dead_code)] // Artifact system implemented but not yet integrated
```

The entire crate is disabled but maintained in the workspace.

**Recommendation**:
- **Option A**: Complete the integration
- **Option B**: Move to a feature branch until ready
- **Option C**: Remove if not needed in near term

---

### 3. Tool Helper Duplication (4 copies)

**Location**:
- `qbit-file-ops/src/lib.rs`
- `qbit-directory-ops/src/lib.rs`
- `qbit-shell-exec/src/lib.rs`
- `qbit-ast-grep/src/tool.rs`

All four crates duplicate these identical functions:
```rust
fn get_required_str<'a>(args: &'a Value, key: &str) -> Result<&'a str, Value>
fn get_optional_str<'a>(args: &'a Value, key: &str) -> Option<&'a str>
fn get_optional_bool(args: &Value, key: &str) -> Option<bool>
fn get_optional_i64(args: &Value, key: &str) -> Option<i64>
```

**Recommendation**: Move to `qbit-core::json_args` or `qbit-tools::args`.

**Impact**: Remove ~200-400 lines of duplication.

---

### 4. Documentation Mismatch (29 vs 30 crates)

**Location**: `CLAUDE.md` line 48

Documentation states "29 crates in 4 layers" but there are actually 30. `qbit-ast-grep` is not listed in the Internal Workspace Crates table.

**Recommendation**:
1. Update count to 30
2. Add row: `qbit-ast-grep | 2 (Infra) | AST-based code search`

---

### 5. Stale Planning Documents

**Location**: `docs/plan/`

| File | Status | Recommendation |
|------|--------|----------------|
| `vtcode-core-migration.md` | "Partially Complete" | Update with remaining items or mark complete |
| `system-prompts.md` | 22KB design notes | Clarify if current or archive |
| `terminal-quality-improvement-plan.md` | Undated roadmap | Add status/completion tracking |
| `image-handling-implementation.md` | Implementation plan | Add status header |

**Recommendation**: Add clear status headers (Active/Complete/Archived) to all planning documents.

---

## Medium Priority Issues

### 6. Dead Code Markers (47 instances)

47 instances of `#[allow(dead_code)]` across 20+ files.

**Key locations**:
- `qbit-artifacts/src/lib.rs` - entire crate
- `qbit-loop-detection/src/lib.rs`
- `qbit-session/src/lib.rs`
- `qbit-llm-providers/src/lib.rs`

**Recommendation**:
1. Run `cargo +nightly udeps` to find unused dependencies
2. Audit each `#[allow(dead_code)]` - either remove code or document why it exists

---

### 7. Crate Granularity Issues

Several crates are very small and could be merged:

| Crate | Lines | Merge Into |
|-------|-------|------------|
| `qbit-cli-output` | 1213 | `qbit/src/cli/output.rs` |
| `qbit-llm-providers` | 381 | `qbit-ai/src/llm_providers.rs` |
| `qbit-file-ops` | 997 | Merge with directory-ops and shell-exec |
| `qbit-directory-ops` | 696 | → `qbit-fs-tools` |
| `qbit-shell-exec` | 388 | |

**Impact**: Reduce from 30 crates to ~26, simpler dependency graph.

---

### 8. Inconsistent Error Handling

Tool implementations use mixed error handling:
- Some use `thiserror` (good)
- Some return `anyhow::Result` (less type-safe)
- Some return `json!({"error": "..."})` (stringly-typed)

**Recommendation**: Define proper error types for tools:
```rust
#[derive(thiserror::Error)]
pub enum ToolError {
    #[error("File not found: {0}")]
    NotFound(String),
    #[error("Permission denied: {0}")]
    PermissionDenied(String),
}
```

---

### 9. Documentation Duplication

Architecture descriptions appear in 3 files:
- `README.md` lines 158-183
- `CLAUDE.md` lines 38-85
- `AGENTS.md` lines 5-11

**Recommendation**:
- Keep `CLAUDE.md` as single source of truth
- `README.md` should have minimal overview with "See CLAUDE.md for details"
- `AGENTS.md` should focus on conventions, not repeat architecture

---

## Low Priority Issues

### 10. Unused /evals/ Directory

**Location**: `/evals/` contains only `.deepeval/.deepeval_telemetry.txt`

Rust evals are in `backend/crates/qbit-evals`. This directory appears orphaned.

**Recommendation**: Remove or add README explaining purpose.

---

### 11. Feature Flag Clarity (local-llm)

The `local-llm` feature is documented but marked "currently disabled".

**Recommendation**: Either:
- Add clear deprecation notice if abandoned
- Mark as "Experimental - WIP" if planned
- Remove all references if permanently abandoned

---

### 12. Minor Documentation Issues

| Issue | Location | Fix |
|-------|----------|-----|
| Generic page title | `index.html` line 7 | Change to "Qbit - AI Terminal" |
| Empty workspace file | `pnpm-workspace.yaml` | Add comment or remove |
| Stale gitignore entry | `.gitignore` line 77 | Remove `mistral-testbed/target/` |

---

## What's Working Well

- **4-layer architecture** is correctly followed
- **CI/CD workflows** are well-designed and efficient
- **Test infrastructure** is clean with good coverage
- **Frontend component organization** is solid
- **Justfile commands** are clear and useful
- **Package.json** is appropriately minimal

---

## Recommended Action Order

### Phase 1: Quick Wins
1. Fix crate count in CLAUDE.md (29→30)
2. Add qbit-ast-grep to documentation
3. Add status headers to planning documents
4. Remove /evals/ directory or document purpose

### Phase 2: Code Cleanup
5. Extract common tool helpers to shared module
6. Audit and clean up dead code markers
7. Decide on qbit-artifacts (integrate or remove)

### Phase 3: Architecture Refinement
8. Create qbit-llm-client to consolidate LLM code
9. Merge small crates (cli-output, llm-providers, fs-tools)
10. Standardize error handling in tool implementations

---

## Metrics

| Category | Current | After Cleanup |
|----------|---------|---------------|
| Crates | 30 | ~26 |
| Duplicated lines | ~1200 | ~0 |
| Dead code markers | 47 | <10 |
| Planning docs without status | 4 | 0 |
