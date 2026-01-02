# vtcode-core Migration Plan

**Status**: Partially Complete
**Priority**: Medium
**Estimated Effort**: 12-20 hours remaining

## Overview

Migrate away from `vtcode-core` external dependency to fully local implementations. This reduces external dependencies, gives us full control over the codebase, and simplifies maintenance.

## Current State

### Already Migrated

| Component | vtcode-core | Local Implementation | Status |
|-----------|-------------|---------------------|--------|
| ToolRegistry | `vtcode_core::tools::ToolRegistry` | `qbit_tools::ToolRegistry` | **Done** |
| Function declarations | `vtcode_core::tools::registry::build_function_declarations` | `qbit_tools::build_function_declarations` | **Done** |
| Session types | `vtcode_core::utils::session_archive::*` | `qbit_core::session::*` | **Done** |

### Remaining Dependencies

| Component | Current Usage | Replacement Needed |
|-----------|--------------|-------------------|
| TreeSitterAnalyzer | `vtcode_core::tools::tree_sitter::analyzer` | New `qbit-tree-sitter` crate |
| CodeAnalyzer | `vtcode_core::tools::tree_sitter::analysis` | New `qbit-tree-sitter` crate |
| LanguageAnalyzer | `vtcode_core::tools::tree_sitter::languages` | New `qbit-tree-sitter` crate |
| Session archive | `vtcode_core::utils::session_archive` | Already exists, need to enable by default |

### Files Still Using vtcode-core

```
crates/qbit-ai/src/tool_executors.rs:15,116    # tree-sitter
crates/qbit/src/indexer/commands.rs:9,316,361,394,424  # tree-sitter
crates/qbit-indexer/src/state.rs:6             # tree-sitter
crates/qbit-session/src/lib.rs:15-16           # session archive
crates/qbit/src/compat.rs:91-92                # session archive (non-local-tools)
```

## Phase 1: Tree-sitter Migration

### New Crate: `qbit-tree-sitter`

Create a Layer 2 infrastructure crate that provides semantic code analysis.

#### Directory Structure

```
backend/crates/qbit-tree-sitter/
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── analyzer.rs      # TreeSitterAnalyzer - parser wrapper
│   ├── analysis.rs      # CodeAnalyzer, CodeMetrics, CodeAnalysis
│   ├── languages/
│   │   ├── mod.rs       # LanguageAnalyzer trait + registry
│   │   ├── rust.rs      # Rust-specific queries
│   │   ├── typescript.rs
│   │   ├── python.rs
│   │   ├── go.rs
│   │   └── generic.rs   # Fallback for unsupported languages
│   └── types.rs         # SymbolInfo, DependencyInfo, etc.
```

#### API Surface to Implement

```rust
// analyzer.rs
pub struct TreeSitterAnalyzer { ... }
impl TreeSitterAnalyzer {
    pub fn new() -> Result<Self>;
    pub async fn parse_file(&mut self, path: &Path) -> Result<ParsedTree>;
    pub fn detect_language_from_path(&self, path: &Path) -> Result<Language>;
}

// analysis.rs
pub struct CodeAnalyzer { ... }
impl CodeAnalyzer {
    pub fn new(language: &Language) -> Self;
    pub fn analyze(&self, tree: &ParsedTree, file_path: &str) -> CodeAnalysis;
}

pub struct CodeAnalysis {
    pub symbols: Vec<SymbolInfo>,
    pub metrics: CodeMetrics,
    pub dependencies: Vec<DependencyInfo>,
}

pub struct CodeMetrics {
    pub lines_of_code: usize,
    pub lines_of_comments: usize,
    pub blank_lines: usize,
    pub functions_count: usize,
    pub classes_count: usize,
    pub variables_count: usize,
    pub imports_count: usize,
    pub comment_ratio: f64,
}

// languages/mod.rs
pub struct LanguageAnalyzer { ... }
impl LanguageAnalyzer {
    pub fn new(language: &Language) -> Self;
    pub fn extract_symbols(&self, tree: &ParsedTree) -> Vec<SymbolInfo>;
}

// types.rs
pub struct SymbolInfo {
    pub name: String,
    pub kind: SymbolKind,
    pub position: Position,
    pub scope: Option<String>,
    pub signature: Option<String>,
    pub documentation: Option<String>,
}

pub enum SymbolKind {
    Function, Class, Struct, Enum, Interface, Variable, Constant, Module, ...
}

pub struct DependencyInfo {
    pub name: String,
    pub kind: DependencyKind,
    pub source: Option<String>,
}
```

#### Dependencies

```toml
[dependencies]
tree-sitter = "0.24"
tree-sitter-rust = "0.23"
tree-sitter-typescript = "0.23"
tree-sitter-python = "0.23"
tree-sitter-javascript = "0.23"
tree-sitter-go = "0.23"
tree-sitter-json = "0.24"
tree-sitter-toml = "0.23"
tree-sitter-c = "0.23"
tree-sitter-cpp = "0.23"
tokio = { workspace = true }
anyhow = { workspace = true }
```

#### Implementation Tasks

- [ ] Create crate structure and Cargo.toml
- [ ] Implement `TreeSitterAnalyzer` with language detection
- [ ] Implement `ParsedTree` wrapper type
- [ ] Implement `CodeMetrics` calculation (LOC, comments, blanks)
- [ ] Implement `LanguageAnalyzer` trait
- [ ] Add Rust language support (symbols, dependencies)
- [ ] Add TypeScript/JavaScript support
- [ ] Add Python support
- [ ] Add Go support
- [ ] Add generic fallback for unsupported languages
- [ ] Write tests for each language
- [ ] Update `qbit-indexer` to use new crate
- [ ] Update `qbit-ai/tool_executors.rs`
- [ ] Update `qbit/indexer/commands.rs`

### Estimated Effort

| Task | Hours |
|------|-------|
| Crate setup + analyzer | 2 |
| Metrics calculation | 2 |
| Rust language support | 2 |
| TypeScript support | 2 |
| Python support | 1 |
| Go support | 1 |
| Generic fallback | 1 |
| Integration + testing | 4 |
| **Total** | **~15 hours** |

## Phase 2: Session Archive Migration

The local session implementation already exists in `qbit_core::session`. Need to:

1. Enable `local-tools` feature by default
2. Update `qbit-session` crate to use `qbit_core::session` instead of vtcode-core
3. Remove vtcode-core session imports from compat layer
4. Test session persistence thoroughly

### Tasks

- [ ] Update `qbit-session/Cargo.toml` to depend on `qbit-core` instead of `vtcode-core`
- [ ] Update `qbit-session/src/lib.rs` imports
- [ ] Update `qbit/src/compat.rs` to always use local session
- [ ] Remove `local-tools` feature flag (make local the only option)
- [ ] Test session save/load/list operations

### Estimated Effort: 2-4 hours

## Phase 3: Remove vtcode-core Dependency

After Phases 1 and 2 are complete:

1. Remove `vtcode-core` from workspace `Cargo.toml`
2. Remove from all crate `Cargo.toml` files
3. Remove `local-tools` feature flag
4. Clean up compat layer
5. Update documentation

### Files to Update

- `backend/Cargo.toml` - Remove from workspace dependencies
- `backend/crates/qbit-ai/Cargo.toml`
- `backend/crates/qbit-indexer/Cargo.toml`
- `backend/crates/qbit-session/Cargo.toml`
- `backend/crates/qbit/Cargo.toml`
- `CLAUDE.md` - Update dependency documentation

## Benefits of Migration

1. **No external dependency**: Full control over implementation
2. **Faster compilation**: vtcode-core pulls in many transitive dependencies
3. **Customization**: Can optimize for qbit's specific needs
4. **Maintainability**: No need to track upstream changes
5. **Smaller binary**: Only include what we need

## Risks and Mitigations

| Risk | Mitigation |
|------|------------|
| Missing edge cases in language support | Start with most-used languages, add others incrementally |
| Performance regression | Benchmark against vtcode-core implementation |
| Breaking existing functionality | Comprehensive test coverage before migration |

## Related Files

- `backend/crates/qbit-tools/` - Already migrated ToolRegistry
- `backend/crates/qbit-core/src/session/` - Local session implementation
- `backend/crates/qbit/src/compat.rs` - Compatibility layer

## Notes

- The `tree-sitter` crate is well-maintained and widely used
- Language grammars are published as separate crates
- vtcode-core itself uses tree-sitter under the hood
- Consider using `tree-sitter-highlight` for syntax highlighting if needed later
