# Tavily Tools Full Migration Plan

## Overview

Migrate from legacy special-case Tavily tool handling to the standard tool registry pattern, with explicit user opt-in via settings.

**Key Simplifications**:
1. User explicitly enables web search in settings → no API key detection at registration
2. No hot-reloading → read settings once at session init
3. TavilyState stays simple → just holds API key + HTTP client
4. Clear error at execution time if API key missing
5. **Generic ToolRegistryConfig** → pass settings, tools extract their own config

---

## Generic ToolRegistryConfig Design

Instead of adding a field for each tool's config:

```rust
// ❌ BAD: Grows with each tool
pub struct ToolRegistryConfig {
    pub shell: Option<String>,
    pub tavily_api_key: Option<String>,
    pub future_tool_key: Option<String>,  // Tedious!
}
```

Pass the settings struct and let tools extract what they need:

```rust
// ✅ GOOD: Generic and extensible
pub struct ToolRegistryConfig {
    pub settings: QbitSettings,  // Tools read their own config
}
```

### Tool Registration Pattern

```rust
impl ToolRegistry {
    pub async fn with_config(workspace: PathBuf, config: ToolRegistryConfig) -> Self {
        let mut tools: HashMap<String, Arc<dyn Tool>> = HashMap::new();
        let settings = &config.settings;
        
        // Core tools (always registered)
        tools.extend(register_core_tools(settings));
        
        // Optional tools (check settings.tools.*)
        if settings.tools.web_search {
            tools.extend(register_tavily_tools(settings));
        }
        
        // Future tools follow same pattern:
        // if settings.tools.some_feature {
        //     tools.extend(register_some_feature_tools(settings));
        // }
        
        Self { tools, workspace }
    }
}

fn register_tavily_tools(settings: &QbitSettings) -> HashMap<String, Arc<dyn Tool>> {
    let api_key = get_with_env_fallback(
        &settings.api_keys.tavily,
        &["TAVILY_API_KEY"],
        None,
    );
    let tavily = Arc::new(TavilyState::from_api_key(api_key));
    
    create_tavily_tools(tavily)
        .into_iter()
        .map(|t| (t.name().to_string(), t))
        .collect()
}

fn register_core_tools(settings: &QbitSettings) -> HashMap<String, Arc<dyn Tool>> {
    vec![
        Arc::new(ReadFileTool) as Arc<dyn Tool>,
        Arc::new(WriteFileTool),
        // ...
        Arc::new(RunPtyCmdTool::with_shell(settings.terminal.shell.clone())),
    ]
    .into_iter()
    .map(|t| (t.name().to_string(), t))
    .collect()
}
```

### Benefits

1. **Adding new tools**: Just add `settings.tools.new_feature` check + registration function
2. **No ToolRegistryConfig changes**: Config struct stays stable
3. **Tools own their config**: Each tool module defines what settings it needs
4. **Type-safe**: Settings schema is the source of truth

---

## Settings Schema

### New Settings Structure

**File**: `~/.qbit/settings.toml`

```toml
[tools]
# Enable Tavily web search tools (web_search, web_extract, etc.)
# When enabled, requires api_keys.tavily to be set
web_search = true  # default: false

[api_keys]
# Tavily API key for web search (get from https://tavily.com)
tavily = "tvly-xxxxxxxxxx"
# Or use environment variable reference:
# tavily = "$TAVILY_API_KEY"
```

### Behavior Matrix

| `tools.web_search` | `api_keys.tavily` | Result |
|--------------------|-------------------|--------|
| `false` (default)  | not set           | Tools not registered, LLM doesn't see them |
| `false`            | set               | Tools not registered (user hasn't opted in) |
| `true`             | not set           | Tools registered, error on execution |
| `true`             | set               | Tools registered and work |

---

## Architecture

### Flow

```
Session Init
    ↓
Read settings.tools.web_search
    ↓
If enabled:
    ├── Read settings.api_keys.tavily (with $ENV fallback)
    ├── Create TavilyState { api_key: Option<String>, client }
    └── Register Tavily tools in ToolRegistry
    ↓
Tool Execution
    ↓
If api_key is None → return clear error message
Else → make API call
```

### TavilyState (Simplified)

```rust
pub struct TavilyState {
    api_key: Option<String>,  // Read once from settings
    client: reqwest::Client,
}

impl TavilyState {
    /// Create from settings. API key can be None (will error on use).
    pub fn from_settings(api_key: Option<String>) -> Self {
        Self {
            api_key,
            client: reqwest::Client::new(),
        }
    }
    
    fn get_api_key(&self) -> Result<&str> {
        self.api_key.as_deref().ok_or_else(|| anyhow::anyhow!(
            "Tavily API key not configured. Set api_keys.tavily in ~/.qbit/settings.toml"
        ))
    }
}
```

---

## Migration Steps

### Phase 1: Update Settings Schema

**File**: `crates/qbit-settings/src/schema.rs`

```rust
/// Tool enablement settings
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ToolsSettings {
    /// Enable Tavily web search tools
    #[serde(default)]
    pub web_search: bool,
}

impl Default for ToolsSettings {
    fn default() -> Self {
        Self {
            web_search: false,  // Opt-in by default
        }
    }
}

// Add to QbitSettings:
pub struct QbitSettings {
    // ... existing fields ...
    #[serde(default)]
    pub tools: ToolsSettings,
}
```

**File**: `crates/qbit-settings/src/loader.rs`

No changes needed - env var resolution already handles `api_keys.tavily`.

---

### Phase 2: Update TavilyState

**File**: `crates/qbit-web/src/tavily.rs`

```rust
pub struct TavilyState {
    api_key: Option<String>,
    client: reqwest::Client,
}

impl TavilyState {
    /// Create from an optional API key (read from settings).
    pub fn from_api_key(api_key: Option<String>) -> Self {
        if api_key.is_some() {
            tracing::info!("Tavily web search tools enabled");
        }
        Self {
            api_key,
            client: reqwest::Client::new(),
        }
    }
    
    /// Legacy constructor for backward compatibility during migration.
    /// Reads from TAVILY_API_KEY env var only.
    #[deprecated(note = "Use from_api_key() with settings integration")]
    pub fn new() -> Self {
        let api_key = std::env::var("TAVILY_API_KEY")
            .ok()
            .filter(|k| !k.is_empty());
        Self::from_api_key(api_key)
    }
    
    fn get_api_key(&self) -> Result<&str> {
        self.api_key.as_deref().ok_or_else(|| anyhow::anyhow!(
            "Tavily API key not configured. Set api_keys.tavily in ~/.qbit/settings.toml"
        ))
    }
}
```

---

### Phase 3: Update ToolRegistryConfig (Generic)

**File**: `crates/qbit-tools/src/registry.rs`

```rust
use qbit_settings::QbitSettings;

/// Configuration for tool registry - just pass settings.
/// Individual tools extract their own config from settings.
#[derive(Clone)]
pub struct ToolRegistryConfig {
    pub settings: QbitSettings,
}

impl ToolRegistry {
    pub async fn with_config(workspace: PathBuf, config: ToolRegistryConfig) -> Self {
        let mut tools: HashMap<String, Arc<dyn Tool>> = HashMap::new();
        let settings = &config.settings;
        
        // Core tools (always registered)
        let core_tools: Vec<Arc<dyn Tool>> = vec![
            Arc::new(ReadFileTool),
            Arc::new(WriteFileTool),
            Arc::new(CreateFileTool),
            Arc::new(EditFileTool),
            Arc::new(DeleteFileTool),
            Arc::new(ListFilesTool),
            Arc::new(ListDirectoryTool),
            Arc::new(GrepFileTool),
            Arc::new(RunPtyCmdTool::with_shell(settings.terminal.shell.clone())),
            Arc::new(AstGrepTool),
            Arc::new(AstGrepReplaceTool),
        ];
        
        for tool in core_tools {
            tools.insert(tool.name().to_string(), tool);
        }
        
        // Optional: Tavily web search tools
        if settings.tools.web_search {
            let api_key = get_with_env_fallback(
                &settings.api_keys.tavily,
                &["TAVILY_API_KEY"],
                None,
            );
            let tavily = Arc::new(TavilyState::from_api_key(api_key));
            for tool in create_tavily_tools(tavily) {
                tools.insert(tool.name().to_string(), tool);
            }
            tracing::debug!("Registered Tavily web search tools");
        }
        
        // Future tools follow same pattern:
        // if settings.tools.some_other_feature {
        //     register_other_tools(&mut tools, settings);
        // }
        
        Self { tools, workspace }
    }
}
```

**Note**: `qbit-tools` now depends on `qbit-settings`. This is a reasonable dependency since tools need config.

---

### Phase 4: Update SharedComponentsConfig

**File**: `crates/qbit-ai/src/llm_client.rs`

```rust
pub struct SharedComponentsConfig {
    pub context_config: Option<ContextConfig>,
    pub settings: QbitSettings,  // Just pass settings, not individual fields
}
```

This simplifies the config - no need to extract individual fields at the call site.

---

### Phase 5: Update Bootstrap/Session Init

**File**: `crates/qbit/src/cli/bootstrap.rs`

```rust
pub async fn initialize_agent(...) -> Result<AgentBridge> {
    let settings = settings_manager.get().await;
    
    let shared_config = SharedComponentsConfig {
        context_config,
        settings: settings.clone(),  // Just pass settings
    };
    
    // ToolRegistry reads what it needs from settings internally
    // ... rest of initialization ...
}
```

**File**: `crates/qbit/src/ai/commands/core.rs`

Same simplification - just pass `settings`.

---

### Phase 6: Update create_tavily_tools

**File**: `crates/qbit-web/src/tool.rs`

```rust
/// Create Tavily tools. Always returns the full list.
/// API key availability is checked at execution time.
pub fn create_tavily_tools(tavily: Arc<TavilyState>) -> Vec<Arc<dyn Tool>> {
    vec![
        Arc::new(WebSearchTool::new(tavily.clone())),
        Arc::new(WebSearchAnswerTool::new(tavily.clone())),
        Arc::new(WebExtractTool::new(tavily.clone())),
        Arc::new(WebCrawlTool::new(tavily.clone())),
        Arc::new(WebMapTool::new(tavily)),
    ]
}
```

---

### Phase 7: Remove Legacy Code

#### Files to Clean Up

| File | Remove |
|------|--------|
| `crates/qbit-ai/src/tool_definitions.rs` | `get_tavily_tool_definitions()` |
| `crates/qbit-ai/src/tool_executors.rs` | `execute_tavily_tool()` |
| `crates/qbit-ai/src/tool_execution.rs` | `ToolCategory::TavilySearch`, `execute_tavily_tool_routed()` |
| `crates/qbit-ai/src/agentic_loop.rs` | `tavily_state` from context, special-case routing |
| `crates/qbit-ai/src/agent_bridge.rs` | `tavily_state` field, `set_tavily_state()` |
| `crates/qbit/src/state.rs` | `tavily_state` field |
| `crates/qbit-sub-agents/src/executor.rs` | Tavily-specific handling |

---

### Phase 8: Add Registry Method for Tool Definitions

**File**: `crates/qbit-tools/src/registry.rs`

```rust
impl ToolRegistry {
    /// Get tool definitions for LLM tool use.
    pub fn get_tool_definitions(&self) -> Vec<ToolDefinition> {
        self.tools.values()
            .map(|tool| ToolDefinition {
                name: tool.name().to_string(),
                description: tool.description().to_string(),
                parameters: tool.parameters(),
            })
            .collect()
    }
}
```

**File**: `crates/qbit-ai/src/agentic_loop.rs`

Replace `get_tavily_tool_definitions()` with registry query:
```rust
let registry = ctx.tool_registry.read().await;
let web_tools = registry.get_tool_definitions()
    .into_iter()
    .filter(|t| t.name.starts_with("web_"))
    .collect::<Vec<_>>();
all_tools.extend(web_tools);
```

---

## File Summary

### New/Modified Files

| File | Changes |
|------|---------|
| `crates/qbit-settings/src/schema.rs` | Add `ToolsSettings` with `web_search: bool` |
| `crates/qbit-web/src/tavily.rs` | Add `from_api_key()` constructor |
| `crates/qbit-web/src/tool.rs` | `create_tavily_tools` returns `Vec` not `Option<Vec>` |
| `crates/qbit-tools/src/registry.rs` | Take `QbitSettings`, check `settings.tools.*` for optional tools |
| `crates/qbit-tools/Cargo.toml` | Add dependency on `qbit-settings` |
| `crates/qbit-ai/src/llm_client.rs` | `SharedComponentsConfig` takes `settings: QbitSettings` |
| `crates/qbit/src/cli/bootstrap.rs` | Pass `settings` to `SharedComponentsConfig` |
| `crates/qbit/src/ai/commands/core.rs` | Same as bootstrap |

### Files to Remove Code From

| File | Code to Remove |
|------|----------------|
| `crates/qbit-ai/src/tool_definitions.rs` | `get_tavily_tool_definitions()` |
| `crates/qbit-ai/src/tool_executors.rs` | `execute_tavily_tool()` |
| `crates/qbit-ai/src/tool_execution.rs` | `ToolCategory::TavilySearch` |
| `crates/qbit-ai/src/agentic_loop.rs` | `tavily_state` context field |
| `crates/qbit-ai/src/agent_bridge.rs` | `tavily_state` field + setter |
| `crates/qbit/src/state.rs` | `tavily_state` field |
| `crates/qbit-sub-agents/src/executor.rs` | Tavily handling |

---

## Verification

### Test Cases

1. **Default (web_search disabled)**:
   ```bash
   # No tools.web_search in settings
   cargo run
   # Verify: web_* tools NOT in available tools list
   ```

2. **Enabled without API key**:
   ```toml
   [tools]
   web_search = true
   ```
   ```bash
   cargo run
   # Verify: web_* tools ARE in list
   # Verify: calling web_search returns "API key not configured" error
   ```

3. **Enabled with API key**:
   ```toml
   [tools]
   web_search = true
   
   [api_keys]
   tavily = "tvly-xxx"
   ```
   ```bash
   cargo run
   # Verify: web_search tool works
   ```

4. **Enabled with env var**:
   ```toml
   [tools]
   web_search = true
   
   [api_keys]
   tavily = "$TAVILY_API_KEY"
   ```
   ```bash
   TAVILY_API_KEY=tvly-xxx cargo run
   # Verify: web_search tool works
   ```

---

## Estimated Effort

| Phase | Effort |
|-------|--------|
| Phase 1: Settings schema | 30 min |
| Phase 2: TavilyState update | 30 min |
| Phase 3-5: Config propagation | 1 hour |
| Phase 6: create_tavily_tools | 15 min |
| Phase 7: Remove legacy code | 1-2 hours |
| Phase 8: Registry definitions | 30 min |
| Verification | 1 hour |
| **Total** | **~5 hours** |

---

## Success Criteria

- [ ] `tools.web_search` setting controls tool registration
- [ ] `api_keys.tavily` with `$ENV` fallback provides the key
- [ ] No `TavilyState` propagation through AgentBridge/Context
- [ ] No `get_tavily_tool_definitions()` or `execute_tavily_tool()` legacy code
- [ ] Clear error message when API key missing but tools enabled
- [ ] All tests pass
- [ ] `cargo clippy` clean

---

## Appendix: Current Code Reference

### A1: Current AgenticLoopContext (to modify)

**File**: `crates/qbit-ai/src/agentic_loop.rs` (line ~206)

```rust
pub struct AgenticLoopContext<'a> {
    pub event_tx: &'a mpsc::UnboundedSender<AiEvent>,
    pub tool_registry: &'a Arc<RwLock<ToolRegistry>>,
    pub sub_agent_registry: &'a Arc<RwLock<SubAgentRegistry>>,
    pub indexer_state: Option<&'a Arc<IndexerState>>,
    pub tavily_state: Option<&'a Arc<TavilyState>>,  // ← REMOVE THIS
    // ... other fields ...
}
```

### A2: Current ToolRegistryConfig (to replace)

**File**: `crates/qbit-tools/src/registry.rs` (line ~30)

```rust
#[derive(Default, Clone)]
pub struct ToolRegistryConfig {
    pub shell: Option<String>,
    pub tavily_state: Option<Arc<TavilyState>>,  // ← REPLACE with settings
}
```

### A3: Current TavilyState (to simplify)

**File**: `crates/qbit-web/src/tavily.rs` (line ~13)

```rust
pub struct TavilyState {
    api_key: RwLock<Option<String>>,  // ← SIMPLIFY to Option<String>
    client: reqwest::Client,
}

impl TavilyState {
    pub fn new() -> Self {
        // Only checks env var, ignores settings
        let api_key = std::env::var("TAVILY_API_KEY").ok().filter(|k| !k.is_empty());
        Self {
            api_key: RwLock::new(api_key),
            client: reqwest::Client::new(),
        }
    }
}
```

### A4: Import Locations

| Symbol | Location |
|--------|----------|
| `get_with_env_fallback` | `qbit_settings::get_with_env_fallback` |
| `QbitSettings` | `qbit_settings::QbitSettings` |
| `TavilyState` | `qbit_web::TavilyState` |
| `create_tavily_tools` | `qbit_web::create_tavily_tools` |
| `ToolRegistry` | `qbit_tools::ToolRegistry` |

### A5: Cargo.toml Change for qbit-tools

**File**: `crates/qbit-tools/Cargo.toml`

Add:
```toml
[dependencies]
# ... existing deps ...

# Settings for tool configuration
qbit-settings = { workspace = true }
```

### A6: Default Implementation for ToolRegistryConfig

Since we're changing `ToolRegistryConfig` to require settings, we need to handle the `Default` case:

```rust
// Option 1: Remove Default derive, require explicit construction
#[derive(Clone)]
pub struct ToolRegistryConfig {
    pub settings: QbitSettings,
}

// Option 2: Use QbitSettings::default() (empty/default settings)
impl Default for ToolRegistryConfig {
    fn default() -> Self {
        Self {
            settings: QbitSettings::default(),
        }
    }
}
```

**Recommendation**: Option 1 (remove Default) is safer - forces callers to provide real settings.

### A7: ToolRegistry::new() Backward Compatibility

The current `ToolRegistry::new(workspace)` signature is used in tests. Options:

```rust
// Keep for tests only, uses default settings
impl ToolRegistry {
    pub async fn new(workspace: PathBuf) -> Self {
        Self::with_config(workspace, ToolRegistryConfig {
            settings: QbitSettings::default(),
        }).await
    }
}
```

Or update all test call sites to use `with_config`.

---

## Appendix: Dependency Graph

```
qbit (main app)
  └── qbit-ai
        └── qbit-tools
              └── qbit-settings  ← NEW DEPENDENCY
              └── qbit-web (already depends on this)
```

**No circular dependency**: `qbit-settings` is a leaf crate with no internal dependencies.
