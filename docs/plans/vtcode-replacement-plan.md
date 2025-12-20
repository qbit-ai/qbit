# vtcode-core Replacement Plan

## Overview

Replace the `vtcode-core` dependency with local implementations to gain full control over tool behavior, reduce external dependencies, and enable customization.

**Current State:** vtcode-core v0.47 provides ToolRegistry, file/shell tools, and utility functions.

**Target State:** Local `backend/src/tools/` module with equivalent functionality.

---

## Phase 1: Create Local ToolRegistry

### 1.1 Define Tool Trait and Registry

Create the core abstraction that mirrors vtcode-core's interface.

**Files to create:**
- `backend/src/tools/mod.rs` - Module root, re-exports
- `backend/src/tools/registry.rs` - ToolRegistry implementation

**Interface to implement:**
```rust
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn parameters(&self) -> serde_json::Value; // JSON Schema
    async fn execute(&self, args: serde_json::Value, workspace: &Path) -> Result<serde_json::Value>;
}

pub struct ToolRegistry {
    tools: HashMap<String, Arc<dyn Tool>>,
    workspace: PathBuf,
}

impl ToolRegistry {
    pub async fn new(workspace: PathBuf) -> Self;
    pub async fn execute_tool(&mut self, name: &str, args: serde_json::Value) -> Result<serde_json::Value>;
    pub async fn available_tools(&self) -> Vec<String>;
}
```

### 1.2 Create Tool Definition Generator

Replace `vtcode_core::tools::registry::build_function_declarations`.

**Files to create:**
- `backend/src/tools/definitions.rs` - JSON schema generation

**Approach:** Use `schemars` crate to derive JSON schemas from Rust structs, or hand-write schemas for precise control.

### 1.3 Integration Points to Update

**Files to modify:**
- `backend/src/ai/llm_client.rs` - Replace `ToolRegistry` import
- `backend/src/ai/agentic_loop.rs` - Replace `ToolRegistry` import
- `backend/src/ai/agent_bridge.rs` - Replace `ToolRegistry` import
- `backend/src/ai/tool_executors.rs` - Replace `ToolRegistry` import
- `backend/src/ai/sub_agent_executor.rs` - Replace `ToolRegistry` import
- `backend/src/ai/commands/workflow.rs` - Replace `ToolRegistry` import
- `backend/src/ai/tool_definitions.rs` - Replace `build_function_declarations`

---

## Phase 2: Implement Core Tools

### 2.1 File Operations

**Files to create:**
- `backend/src/tools/file_ops.rs`

**Tools to implement:**

#### `read_file`
```rust
struct ReadFileArgs {
    path: String,
    line_start: Option<usize>,  // 1-indexed
    line_end: Option<usize>,    // 1-indexed, inclusive
}
```
- Read file contents, optionally with line range
- Handle encoding (UTF-8 with fallback)
- Return line-numbered output for LLM context

#### `write_file`
```rust
struct WriteFileArgs {
    path: String,
    content: String,
}
```
- Overwrite file with new content
- Create parent directories if needed
- Preserve file permissions where possible

#### `create_file`
```rust
struct CreateFileArgs {
    path: String,
    content: String,
}
```
- Create new file (fail if exists)
- Create parent directories if needed

#### `edit_file`
```rust
struct EditFileArgs {
    path: String,
    old_text: String,
    new_text: String,
    display_description: Option<String>,
}
```
- Search and replace within file
- Must match exactly one occurrence (fail if 0 or >1 matches)
- Return diff or confirmation

#### `delete_file`
```rust
struct DeleteFileArgs {
    path: String,
}
```
- Delete file (not directories)
- Return confirmation

### 2.2 Directory Operations

**Files to create:**
- `backend/src/tools/directory_ops.rs`

**Tools to implement:**

#### `list_files`
```rust
struct ListFilesArgs {
    path: Option<String>,       // Default: workspace root
    pattern: Option<String>,    // Glob pattern
    recursive: Option<bool>,    // Default: true
}
```
- List files matching pattern
- Respect .gitignore
- Return relative paths

#### `list_directory`
```rust
struct ListDirectoryArgs {
    path: String,
}
```
- List immediate children of directory
- Show file/directory type, size, modified time

#### `grep_file`
```rust
struct GrepFileArgs {
    pattern: String,            // Regex pattern
    path: Option<String>,       // File or directory
    include: Option<String>,    // Glob for file filtering
}
```
- Search file contents with regex
- Return matches with context lines
- Respect .gitignore

### 2.3 Shell Execution

**Files to create:**
- `backend/src/tools/shell.rs`

**Tools to implement:**

#### `run_pty_cmd`
```rust
struct RunPtyCmdArgs {
    command: String,
    cwd: Option<String>,
    timeout: Option<u64>,       // Seconds, default: 120
}
```
- Execute shell command via existing PTY infrastructure
- Capture stdout/stderr
- Handle timeout
- Return exit code and output

**Integration:** Leverage existing `backend/src/pty/` module.

### 2.4 Patch Operations

**Files to create:**
- `backend/src/tools/patch.rs`

**Tools to implement:**

#### `apply_patch`
```rust
struct ApplyPatchArgs {
    patch: String,              // Unified diff format
}
```
- Parse unified diff
- Apply to target files
- Handle multiple files in single patch
- Return success/failure per file

**Dependencies:** Consider `diffy` or `similar` crate for diff parsing.

---

## Phase 3: Handle Remaining vtcode-core Dependencies

### 3.1 Tree-sitter Analysis

**Current usage:**
- `vtcode_core::tools::tree_sitter::analyzer::TreeSitterAnalyzer`
- `vtcode_core::tools::tree_sitter::analysis::CodeAnalyzer`
- `vtcode_core::tools::tree_sitter::languages::LanguageAnalyzer`

**Options:**
1. **Keep vtcode-core for tree-sitter only** - Lowest effort, split dependency
2. **Use tree-sitter crate directly** - More control, more work
3. **Remove tree-sitter features** - If not critical to core functionality

**Recommendation:** Keep vtcode-core for tree-sitter initially, migrate later if needed.

### 3.2 LLM Client Factory

**Current usage:**
- `vtcode_core::llm::{make_client, AnyClient}` - Only for non-OpenRouter/non-Vertex
- `vtcode_core::config::models::ModelId` - Model ID parsing

**Analysis:** The `LlmClient::Vtcode` variant is marked as "legacy, no tool support".

**Options:**
1. **Remove Vtcode client entirely** - Only support Vertex AI and OpenRouter
2. **Implement minimal client** - For any remaining use cases

**Recommendation:** Remove if not actively used; Vertex AI and OpenRouter cover main use cases.

### 3.3 Session Archive

**Current usage:**
- `vtcode_core::utils::session_archive::{SessionArchive, SessionArchiveMetadata, SessionMessage}`

**Analysis:** Simple serialization utilities for session persistence.

**Files to create:**
- `backend/src/ai/session_archive.rs` (or inline in `session.rs`)

**Implementation:** ~50-100 lines of serde structs and save/load functions.

### 3.4 vtcode-indexer

**Current usage:**
- `vtcode_indexer::SimpleIndexer` in `indexer/state.rs`

**Options:**
1. **Keep vtcode-indexer** - It's a separate, smaller crate
2. **Replace with local implementation** - More work, full control

**Recommendation:** Keep vtcode-indexer; it's orthogonal to vtcode-core replacement.

---

## Phase 4: Migration and Cleanup

### 4.1 Incremental Migration Strategy

1. Create new `tools/` module alongside existing code
2. Add feature flag `local-tools` to toggle between implementations
3. Migrate one tool at a time, testing each
4. Once all tools migrated, remove vtcode-core dependency

### 4.2 Testing Strategy

**Unit tests:**
- Each tool in isolation
- Edge cases: empty files, binary files, large files, unicode

**Integration tests:**
- Tool execution through registry
- Agentic loop with local tools

**Manual testing:**
- Run full agent sessions
- Compare behavior with vtcode-core

### 4.3 Final Cleanup

**Files to modify:**
- `backend/Cargo.toml` - Remove `vtcode-core` (keep `vtcode-indexer` if needed)
- Remove all `use vtcode_core::` imports
- Update CLAUDE.md documentation

---

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Tool behavior differences | Medium | High | Comprehensive testing, feature flag rollback |
| Edge cases in file ops | Medium | Medium | Property-based testing, fuzz testing |
| PTY integration issues | Low | High | Reuse existing PTY infrastructure |
| Performance regression | Low | Low | Benchmark critical paths |
| Missing tool features | Medium | Medium | Document vtcode-core tool behavior first |

---

## Open Questions

1. **Tool behavior documentation:** Do we have comprehensive docs on vtcode-core tool behavior, or need to reverse-engineer from source?

2. **Binary file handling:** How should tools handle binary files? Skip, error, or attempt?

3. **Symlink handling:** Follow symlinks or treat as special?

4. **Permission preservation:** Should write operations preserve original file permissions?

5. **Atomic writes:** Should file writes be atomic (write to temp, then rename)?

6. **Large file handling:** Memory limits for read_file? Streaming for large files?

7. **Shell environment:** What environment variables should run_pty_cmd inherit?

8. **Path security:** How to prevent path traversal attacks (../../../etc/passwd)?

---

---

## Deep Dive Analysis: Gaps Identified by Sub-Agents

The following gaps were identified through comprehensive codebase analysis:

### Critical Gaps (Must Address)

#### 1. Thread Safety Not Addressed
**Finding:** ToolRegistry is ALWAYS wrapped in `Arc<RwLock<ToolRegistry>>`. Local implementation MUST support concurrent tool execution from:
- Multiple concurrent tool executions in agentic loop
- Sub-agent execution (concurrent)
- Workflow execution (concurrent)

**Fix:** Use `Arc<DashMap>` or internal `RwLock` for tool storage.

#### 2. Path Traversal Prevention Missing
**Finding:** NO canonicalization or workspace boundary checking exists. Tools can access ANY file the user can access.

**Fix:** Implement in `execute_with_hitl()`:
```rust
fn validate_path(requested: &str, workspace: &Path) -> Result<PathBuf> {
    let requested_path = Path::new(requested);
    if requested_path.is_absolute() {
        return Err("Absolute paths not allowed");
    }
    let full_path = workspace.join(requested_path);
    let canonical = std::fs::canonicalize(&full_path)?;
    if !canonical.starts_with(std::fs::canonicalize(workspace)?) {
        return Err("Path traversal detected");
    }
    Ok(canonical)
}
```

#### 3. Success Determination Logic Complex
**Finding:** Success is determined by TWO factors, not just `Result<>`:
1. Rust Result doesn't error
2. JSON result has no "error" field AND exit_code == 0

**Fix:** Document and replicate this pattern in local tools.

#### 4. LlmClient::Vtcode is Dead Code
**Finding:** The Vtcode client variant is never used in practice:
- No tool support
- No streaming support
- Explicitly rejected by workflow system

**Fix:** Remove entirely before other refactors. Simplifies codebase.

### High Priority Gaps

#### 5. Workspace Mutability Not Addressed
**Finding:** If workspace changes, registry needs replacement. Plan doesn't address `set_workspace()` scenario.

**Fix:** Either add `set_workspace()` method or document that registry must be recreated.

#### 6. Sidecar Capture Integration Missing from Plan
**Finding:** Tools must integrate with CaptureContext for:
- File tracking (files_accessed, files_modified)
- Diff generation for edit_file
- Old content caching before edits

**Fix:** Add Phase 2.6 for sidecar integration checklist.

#### 7. Policy Constraint Enforcement Not Detailed
**Finding:** ToolPolicyManager checks happen BEFORE tool execution but plan doesn't show this integration.

**Fix:** Each tool must call `tool_policy_manager.apply_constraints()` before execution.

#### 8. Binary File Detection Missing
**Finding:** No binary detection in file tools. read_file could try to UTF-8 decode binary files.

**Fix:** Add magic byte detection or extension check before text operations.

### Medium Priority Gaps

#### 9. Schema Sanitization Not Documented
**Finding:** Tool schemas are sanitized for Anthropic API compatibility. Plan doesn't specify what sanitization is needed.

**Fix:** Document exact schema requirements from vtcode-core.

#### 10. Error Type Hierarchy Not Defined
**Finding:** Plan shows simple `Result<>` but actual errors need categorization:
- `file_not_found` vs `permission_denied` vs `policy_violation`

**Fix:** Define error response format with `error_type` field.

#### 11. Environment Variable Handling for Shell
**Finding:** `run_pty_cmd` inherits parent environment which may leak API keys.

**Fix:** Whitelist safe env vars, filter out `AWS_*`, `GOOGLE_*`, `OPENAI_*`.

#### 12. Atomic Write Safety Missing
**Finding:** No atomic writes implemented. File corruption possible on crash.

**Fix:** Write to temp file, fsync, rename to target.

### Low Priority Gaps

#### 13. MessageRole Conversion Coupling
**Finding:** session.rs uses vtcode_core::llm::provider::MessageRole. Need local enum or mapping.

**Fix:** Define local 4-variant enum: User, Assistant, System, Tool.

#### 14. Line Number Edge Cases
**Finding:** What if line_end > file line count? Plan doesn't specify.

**Fix:** Document: "If line_end > line_count, read to EOF"

#### 15. CRLF vs LF Normalization
**Finding:** No line ending normalization. Different behavior Windows vs Unix.

**Fix:** Normalize to LF on read, preserve original on write.

---

## Updated Implementation Priority (Post Deep-Dive)

### Phase 0: Quick Wins (1-2 hours)
**Files to modify:** 5 files, ~200 lines removed

1. **Remove LlmClient::Vtcode entirely**
   - Delete variant from enum (`llm_client.rs:28`)
   - Delete `VtcodeClientConfig` struct (`llm_client.rs:36-41`)
   - Delete `create_vtcode_components()` function (`llm_client.rs:97-115`)
   - Remove match arms in `agent_bridge.rs:520-580`
   - Remove error returns in `workflow.rs:192-195, 317-321`
   - Remove imports: `vtcode_core::llm::*`, `vtcode_core::config::models::ModelId`

### Phase 1: Core Registry (1 week)
**New files:** 3-4 files, ~800 lines

1. **Create `backend/src/tools/mod.rs`**
   - Tool trait with `execute()`, `name()`, `description()`, `parameters()`
   - Must be `Send + Sync` for concurrent execution

2. **Create `backend/src/tools/registry.rs`**
   - `ToolRegistry::new(workspace: PathBuf).await`
   - `registry.execute_tool(name, args).await -> Result<Value>`
   - `registry.available_tools().await -> Vec<String>`
   - Internal: `Arc<DashMap>` or `RwLock<HashMap>` for thread safety

3. **Create `backend/src/tools/definitions.rs`**
   - Replace `build_function_declarations()`
   - ~31 tool schemas (11 standard + 6 indexer + 3 web + 5 sub-agents + 1 workflow + extras)
   - Schema sanitization: remove `anyOf`/`allOf`/`oneOf`

4. **Update integration points (6 files)**
   - `llm_client.rs:84` - ToolRegistry creation
   - `agentic_loop.rs:424-425, 1028-1029` - execute_tool calls
   - `agent_bridge.rs:894-895` - execute_tool wrapper
   - `sub_agent_executor.rs:217-218` - sub-agent tool execution
   - `commands/workflow.rs:537` - workflow tool execution
   - `tool_definitions.rs:161` - replace build_function_declarations

### Phase 1.5: Security Foundation (3-4 days)
**New files:** 2 files, ~400 lines

1. **Create `backend/src/security/path_validator.rs`**
   - `validate_path_within_workspace(path, workspace) -> Result<PathBuf>`
   - `check_symlink_escape(path, workspace) -> Result<()>`
   - `normalize_relative_path(path) -> Result<PathBuf>`
   - Use `path-clean` crate for safe normalization

2. **Create `backend/src/security/mod.rs`**
   - Export all security functions
   - `is_path_blocked_safe()` using `globset` crate

3. **Integration points:**
   - `agentic_loop.rs:160` - Pre-execution validation
   - `tool_policy.rs:591-598` - Enhanced constraint checking
   - `sidecar/capture.rs:426` - Path extraction validation

4. **Add dependencies to Cargo.toml:**
   ```toml
   path-clean = "1.0"
   globset = "0.4"
   unicode-normalization = "0.1"
   ```

### Phase 2: Core File Tools (1 week)
**New file:** `backend/src/tools/file_ops.rs`, ~600 lines

| Tool | Input | Output | Special Handling |
|------|-------|--------|------------------|
| `read_file` | path, line_start?, line_end? | content, size, truncated | Binary detection, 100KB limit |
| `write_file` | path, content, overwrite? | success, bytes_written | Atomic write (temp + rename) |
| `create_file` | path, content | success, path | Fail if exists |
| `edit_file` | path, old_text, new_text | success, diff | Exactly 1 match required |
| `delete_file` | path, confirm | success, bytes_freed | Deny policy default |

**Sidecar integration for each:**
- `files_accessed` / `files_modified` tracking
- Diff generation for edit_file (max 4000 chars)
- Old content caching before edits

### Phase 2.5: Shell Tool (2-3 days)
**New file:** `backend/src/tools/shell.rs`, ~200 lines

1. **Implementation using `std::process::Command`** (not PTY)
   - Shell wrapper: `sh -c "command"`
   - Timeout via `tokio::time::timeout()`
   - SIGKILL on timeout

2. **Normalization already exists:** `normalize_run_pty_cmd_args()` in tool_executors.rs

3. **Environment handling:**
   - Inherit process env
   - Apply `env` parameter overrides
   - Filter sensitive vars: `AWS_*`, `OPENAI_*`, `GOOGLE_*`

4. **Output format:**
   ```json
   {"stdout": "...", "exit_code": 0}
   // OR
   {"error": "...", "exit_code": 1}
   ```

### Phase 3: Session Archive (~500 lines)
**New file:** `backend/src/ai/session_archive.rs`

1. **Structs to implement:**
   - `SessionArchive` - File persistence manager
   - `SessionArchiveMetadata` - Metadata container
   - `SessionMessage` - Message with role/content/tool_call_id
   - `MessageRole` - Enum: User, Assistant, System, Tool
   - `SessionSnapshot` - Full serializable snapshot
   - `SessionListing` - For list_recent_sessions

2. **Functions to implement:**
   - `SessionArchive::new(metadata).await`
   - `SessionArchive::finalize(transcript, count, tools, messages) -> PathBuf`
   - `list_recent_sessions(limit) -> Vec<SessionListing>`
   - `find_session_by_identifier(id) -> Option<SessionListing>`

3. **File format:** JSON (same as vtcode, backwards compatible)

4. **Directory:** `~/.qbit/sessions/` (respects `$VT_SESSION_DIR`)

### Phase 4: Testing Strategy (1 week)

**Test categories:**
- **Unit tests:** ~50 tests for registry, tools, policies
- **Integration tests:** ~20 tests for full pipeline
- **Security tests:** ~15 tests for path traversal, injection
- **Concurrent tests:** ~10 tests for deadlock prevention
- **Backwards compatibility:** ~10 tests for vtcode migration

**Key test files:**
- `tests/unit/tool_registry_tests.rs`
- `tests/integration/full_pipeline_tests.rs`
- `tests/security/path_traversal_tests.rs`

**CI integration:**
```yaml
- cargo test --lib          # Unit tests
- cargo test --test '*'     # Integration tests
- cargo tarpaulin --out Xml # Coverage
```

---

## Appendix: vtcode-core Tool Schemas

To be filled in with exact schemas from vtcode-core for reference during implementation.

### read_file
```json
{
  "name": "read_file",
  "description": "Read the contents of a file",
  "parameters": {
    "type": "object",
    "properties": {
      "path": { "type": "string", "description": "Path to the file" },
      "line_start": { "type": "integer", "description": "Starting line (1-indexed)" },
      "line_end": { "type": "integer", "description": "Ending line (1-indexed, inclusive)" }
    },
    "required": ["path"]
  }
}
```

### edit_file
```json
{
  "name": "edit_file",
  "description": "Edit a file by replacing text",
  "parameters": {
    "type": "object",
    "properties": {
      "path": { "type": "string", "description": "Path to the file" },
      "old_text": { "type": "string", "description": "Text to find and replace" },
      "new_text": { "type": "string", "description": "Replacement text" },
      "display_description": { "type": "string", "description": "Human-readable description of the edit" }
    },
    "required": ["path", "old_text", "new_text"]
  }
}
```

(Additional schemas to be documented)
