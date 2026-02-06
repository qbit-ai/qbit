# MCP (Model Context Protocol) Support for Qbit

## Executive Summary

This document outlines a plan to add MCP support to Qbit, enabling it to connect to external tools, databases, and APIs through the standardized Model Context Protocol. This will allow users to extend Qbit's capabilities by connecting to any MCP-compatible server.

## What is MCP?

The Model Context Protocol (MCP) is an open standard created by Anthropic for connecting LLM applications to external data sources and tools. It provides a standardized way for AI assistants to:

- Access tools and capabilities from external servers
- Read resources (files, database schemas, API data)
- Use templated prompts from servers
- Communicate over multiple transport types (stdio, HTTP)

The protocol uses JSON-RPC 2.0 for message exchange between:
- **Hosts** (LLM applications like Qbit)
- **Clients** (connectors within the host)
- **Servers** (services providing tools/resources)

## Why Add MCP Support?

1. **Ecosystem compatibility**: MCP is becoming the de-facto standard for AI tool integration (Claude Desktop, VS Code Copilot, Claude Code all support it)
2. **Extensibility**: Users can connect to hundreds of existing MCP servers (databases, APIs, monitoring tools)
3. **Standardization**: No need to build custom integrations for each service
4. **Community**: Growing ecosystem of open-source MCP servers

---

## Research Findings

### Official Rust SDK: `rmcp`

The official Rust SDK for MCP is the `rmcp` crate (v0.14.0 as of writing):

```toml
rmcp = { version = "0.14.0", features = ["client", "transport-child-process", "transport-io"] }
```

**Key features:**
- Full MCP 2025-11-25 specification compliance
- Async/await with Tokio runtime
- Multiple transport types:
  - `transport-child-process`: Spawn servers as child processes (stdio)
  - `transport-io`: Direct I/O streams
  - `transport-streamable-http-client`: HTTP/SSE client transport
- Strongly typed tool definitions and results
- Built-in JSON Schema support via `schemars`

**Client Usage Pattern:**
```rust
use rmcp::{ServiceExt, model::CallToolRequestParams, transport::TokioChildProcess};
use tokio::process::Command;

// Connect to an MCP server
let service = ()
    .serve(TokioChildProcess::new(Command::new("mcp-server-command"))?)
    .await?;

// List available tools
let tools = service.list_tools(Default::default()).await?;

// Call a tool
let result = service.call_tool(CallToolRequestParams {
    name: "tool_name".into(),
    arguments: serde_json::json!({ "param": "value" }).as_object().cloned(),
    ..Default::default()
}).await?;

// Gracefully shutdown
service.cancel().await?;
```

### Existing Qbit Architecture

Qbit already has extension points designed for MCP integration:

1. **`McpServerConfig`** in settings schema (stub already exists):
   ```rust
   pub struct McpServerConfig {
       pub command: Option<String>,
       pub args: Vec<String>,
       pub env: HashMap<String, String>,
       pub url: Option<String>,
   }
   ```

2. **`AgenticLoopContext`** has fields for dynamic tools:
   - `additional_tool_definitions: Vec<ToolDefinition>` - inject MCP tools here
   - `custom_tool_executor` - handle MCP tool calls

3. **`ToolRoutingCategory`** enum for categorizing tool calls

4. **Environment variable interpolation** already handles MCP server env vars

### Configuration Patterns (Claude Code Reference)

Claude Code uses a JSON configuration format:
```json
{
  "mcpServers": {
    "server-name": {
      "type": "stdio",
      "command": "/path/to/server",
      "args": ["--flag", "value"],
      "env": { "API_KEY": "${API_KEY}" }
    },
    "remote-server": {
      "type": "http",
      "url": "https://mcp.example.com/mcp",
      "headers": { "Authorization": "Bearer ${TOKEN}" }
    }
  }
}
```

---

## Implementation Plan

### Phase 1: Core MCP Client Infrastructure

Create a new crate `qbit-mcp` (Layer 2 - Infrastructure) to handle MCP client functionality.

#### 1.1 New Crate Structure

```
backend/crates/qbit-mcp/
├── Cargo.toml
└── src/
    ├── lib.rs           # Public API
    ├── client.rs        # MCP client wrapper
    ├── config.rs        # Configuration types
    ├── manager.rs       # Multi-server connection manager
    ├── transport.rs     # Transport abstraction
    └── tools.rs         # Tool bridging to Qbit format
```

#### 1.2 Dependencies

```toml
[dependencies]
rmcp = { version = "0.14", features = [
    "client",
    "transport-child-process",
    "transport-io",
    "transport-streamable-http-client"
]}
tokio = { version = "1", features = ["process", "sync"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
anyhow = "1"
thiserror = "1"
tracing = "0.1"
```

#### 1.3 Core Types

```rust
// config.rs
use std::collections::HashMap;

/// Server transport type
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum McpTransportType {
    Stdio,
    Http,
    Sse,  // Deprecated but supported for compatibility
}

/// Enhanced MCP server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerConfig {
    /// Transport type (default: stdio)
    #[serde(default)]
    pub transport: McpTransportType,
    
    /// Command for stdio transport
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    
    /// Arguments for the command
    #[serde(default)]
    pub args: Vec<String>,
    
    /// Environment variables
    #[serde(default)]
    pub env: HashMap<String, String>,
    
    /// URL for HTTP/SSE transport
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    
    /// HTTP headers for remote servers
    #[serde(default)]
    pub headers: HashMap<String, String>,
    
    /// Whether this server is enabled
    #[serde(default = "default_true")]
    pub enabled: bool,
}

/// Tool definition from an MCP server
#[derive(Debug, Clone)]
pub struct McpTool {
    pub server_name: String,
    pub tool_name: String,
    pub description: Option<String>,
    pub input_schema: serde_json::Value,
}

/// Result of an MCP tool call
#[derive(Debug)]
pub struct McpToolResult {
    pub content: Vec<McpContent>,
    pub is_error: bool,
}

#[derive(Debug)]
pub enum McpContent {
    Text(String),
    Image { data: String, mime_type: String },
    Resource { uri: String, text: Option<String> },
}
```

#### 1.4 MCP Client Manager

```rust
// manager.rs
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;

/// Manages connections to multiple MCP servers
pub struct McpManager {
    /// Active server connections
    servers: Arc<RwLock<HashMap<String, McpServerConnection>>>,
    /// Tool cache (tool_name -> server_name)
    tool_index: Arc<RwLock<HashMap<String, String>>>,
    /// Configuration
    config: HashMap<String, McpServerConfig>,
}

impl McpManager {
    /// Create a new manager with the given configuration
    pub fn new(config: HashMap<String, McpServerConfig>) -> Self { ... }
    
    /// Connect to all enabled servers
    pub async fn connect_all(&self) -> Result<()> { ... }
    
    /// Connect to a specific server
    pub async fn connect(&self, name: &str) -> Result<()> { ... }
    
    /// Disconnect from a server
    pub async fn disconnect(&self, name: &str) -> Result<()> { ... }
    
    /// List all available tools across all connected servers
    pub async fn list_tools(&self) -> Result<Vec<McpTool>> { ... }
    
    /// Call a tool by fully-qualified name (mcp__{server}__{tool})
    pub async fn call_tool(
        &self,
        tool_name: &str,
        arguments: serde_json::Value,
    ) -> Result<McpToolResult> { ... }
    
    /// Get server status
    pub async fn server_status(&self, name: &str) -> Option<ServerStatus> { ... }
}

#[derive(Debug, Clone)]
pub enum ServerStatus {
    Connected { tool_count: usize },
    Disconnected,
    Error(String),
}
```

### Phase 2: Integration with Tool System

#### 2.1 Extend Tool Routing

Add a new routing category in `qbit-ai/src/tool_execution.rs`:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolRoutingCategory {
    Indexer,
    WebFetch,
    UpdatePlan,
    SubAgent,
    Mcp,      // New: MCP tool calls
    Registry,
}

impl ToolRoutingCategory {
    pub fn from_tool_name(name: &str) -> Self {
        // MCP tools use prefix: mcp__{server}__{tool}
        if name.starts_with("mcp__") {
            Self::Mcp
        } else if name.starts_with("indexer_") {
            Self::Indexer
        }
        // ... rest unchanged
    }
}
```

#### 2.2 Add MCP to Agentic Loop Context

Extend `AgenticLoopContext` in `qbit-ai/src/agentic_loop.rs`:

```rust
pub struct AgenticLoopContext<'a> {
    // ... existing fields ...
    
    /// MCP server manager for external tool calls
    pub mcp_manager: Option<&'a Arc<McpManager>>,
}
```

#### 2.3 Tool Definition Generation

Create a bridge to convert MCP tools to Qbit tool definitions:

```rust
// In qbit-mcp/src/tools.rs
use rig::completion::ToolDefinition;

impl McpTool {
    /// Convert to rig ToolDefinition for LLM consumption
    pub fn to_tool_definition(&self) -> ToolDefinition {
        // Prefix with mcp__{server}__ to namespace
        let full_name = format!("mcp__{}__{}",
            self.server_name.replace("-", "_"),
            self.tool_name.replace("-", "_")
        );
        
        ToolDefinition {
            name: full_name,
            description: self.description.clone()
                .unwrap_or_else(|| format!("MCP tool from {}", self.server_name)),
            parameters: self.input_schema.clone(),
        }
    }
}
```

### Phase 3: Configuration Loading

MCP configuration lives in dedicated `mcp.json` files (not in `settings.toml`).

#### 3.1 Config File Schema

Create `qbit-mcp/src/config.rs` with JSON schema types:

```rust
use std::collections::HashMap;
use serde::{Deserialize, Serialize};

/// Root structure of mcp.json files
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct McpConfigFile {
    #[serde(default)]
    pub mcp_servers: HashMap<String, McpServerConfig>,
}

/// Server transport type
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum McpTransportType {
    #[default]
    Stdio,
    Http,
    Sse,
}

/// MCP server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerConfig {
    /// Transport type (default: stdio)
    #[serde(default)]
    pub transport: McpTransportType,
    
    /// Command for stdio transport
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    
    /// Arguments for the command
    #[serde(default)]
    pub args: Vec<String>,
    
    /// Environment variables (supports $VAR and ${VAR} syntax)
    #[serde(default)]
    pub env: HashMap<String, String>,
    
    /// URL for HTTP/SSE transport
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    
    /// HTTP headers (supports $VAR and ${VAR} syntax)
    #[serde(default)]
    pub headers: HashMap<String, String>,
    
    /// Whether this server is enabled (default: true)
    #[serde(default = "default_true")]
    pub enabled: bool,
    
    /// Timeout in seconds for server startup (default: 30)
    #[serde(default = "default_timeout")]
    pub timeout: u64,
}

fn default_true() -> bool { true }
fn default_timeout() -> u64 { 30 }
```

#### 3.2 Config Loading Logic

```rust
// qbit-mcp/src/loader.rs

/// Load and merge MCP configs from user-global and project locations
pub fn load_mcp_config(project_dir: &Path) -> Result<McpConfigFile> {
    let mut merged = McpConfigFile::default();
    
    // 1. Load user-global config (~/.qbit/mcp.json)
    let user_config_path = dirs::home_dir()
        .map(|h| h.join(".qbit/mcp.json"));
    if let Some(path) = user_config_path {
        if path.exists() {
            let user_config: McpConfigFile = load_json_file(&path)?;
            merged.mcp_servers.extend(user_config.mcp_servers);
        }
    }
    
    // 2. Load project config (<project>/.qbit/mcp.json)
    // Project servers override user-global servers with same name
    let project_config_path = project_dir.join(".qbit/mcp.json");
    if project_config_path.exists() {
        let project_config: McpConfigFile = load_json_file(&project_config_path)?;
        merged.mcp_servers.extend(project_config.mcp_servers);
    }
    
    Ok(merged)
}

/// Interpolate environment variables in config values
/// Supports both $VAR and ${VAR} syntax
pub fn interpolate_env_vars(value: &str) -> String { ... }
```

#### 3.3 Project Config Trust

Track approved project configs in `~/.qbit/trusted-mcp-configs.json`:

```rust
/// Check if a project's MCP config has been approved
pub fn is_project_config_trusted(project_dir: &Path) -> bool { ... }

/// Mark a project's MCP config as trusted (after user approval)
pub fn trust_project_config(project_dir: &Path) -> Result<()> { ... }
```

### Phase 4: Tauri Commands and Frontend

#### 4.1 New Tauri Commands

Create `backend/crates/qbit/src/mcp/commands.rs`:

```rust
/// List configured MCP servers and their status
#[tauri::command]
pub async fn mcp_list_servers(
    state: State<'_, AppState>,
) -> Result<Vec<McpServerInfo>, String> { ... }

/// Connect to an MCP server
#[tauri::command]
pub async fn mcp_connect(
    name: String,
    state: State<'_, AppState>,
) -> Result<(), String> { ... }

/// Disconnect from an MCP server
#[tauri::command]
pub async fn mcp_disconnect(
    name: String,
    state: State<'_, AppState>,
) -> Result<(), String> { ... }

/// List tools from all connected MCP servers
#[tauri::command]
pub async fn mcp_list_tools(
    state: State<'_, AppState>,
) -> Result<Vec<McpToolInfo>, String> { ... }

/// Add a new MCP server configuration
#[tauri::command]
pub async fn mcp_add_server(
    name: String,
    config: McpServerConfig,
    scope: McpScope, // user, project
    state: State<'_, AppState>,
) -> Result<(), String> { ... }

/// Remove an MCP server configuration
#[tauri::command]
pub async fn mcp_remove_server(
    name: String,
    state: State<'_, AppState>,
) -> Result<(), String> { ... }
```

#### 4.2 Frontend Components

Add MCP management to Settings UI:

```typescript
// frontend/components/Settings/McpSettings.tsx
interface McpServerInfo {
  name: string;
  config: McpServerConfig;
  status: 'connected' | 'disconnected' | 'error';
  toolCount?: number;
  error?: string;
}

export function McpSettings() {
  // List configured servers
  // Show connection status
  // Add/remove/edit servers
  // View available tools per server
}
```

### Phase 5: Events and Notifications

#### 5.1 New AI Events

Add to `qbit-core/src/events.rs`:

```rust
pub enum AiEvent {
    // ... existing variants ...
    
    /// MCP server connected
    McpServerConnected {
        server_name: String,
        tool_count: usize,
    },
    
    /// MCP server disconnected
    McpServerDisconnected {
        server_name: String,
        reason: Option<String>,
    },
    
    /// MCP server error
    McpServerError {
        server_name: String,
        error: String,
    },
    
    /// MCP tools list updated (server sent list_changed notification)
    McpToolsUpdated {
        server_name: String,
        tools: Vec<String>,
    },
}
```

---

## Configuration Examples

### User-Global Configuration (`~/.qbit/mcp.json`)

```json
{
  "mcpServers": {
    "github": {
      "transport": "http",
      "url": "https://api.githubcopilot.com/mcp/",
      "headers": {
        "Authorization": "Bearer ${GITHUB_TOKEN}"
      }
    },
    "filesystem": {
      "transport": "stdio",
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-filesystem", "/home/user/projects"]
    }
  }
}
```

### Project Configuration (`<project>/.qbit/mcp.json`)

```json
{
  "mcpServers": {
    "project-db": {
      "transport": "stdio",
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-postgres"],
      "env": {
        "DATABASE_URL": "${DATABASE_URL}"
      }
    },
    "internal-api": {
      "transport": "http",
      "url": "https://mcp.internal.company.com/",
      "headers": {
        "Authorization": "Bearer ${INTERNAL_API_TOKEN}"
      }
    }
  }
}
```

---

## Security Considerations

1. **User Consent**: Always prompt before connecting to new MCP servers, especially from project config
2. **Tool Approval**: MCP tools go through the same HITL approval system as built-in tools
3. **Environment Variables**: Support `$VAR` syntax but never log resolved values
4. **Sandboxing**: stdio servers run as child processes with inherited environment
5. **URL Validation**: Validate remote server URLs to prevent SSRF
6. **Origin Header**: For HTTP transport, validate Origin headers per MCP spec

---

## Implementation Checklist

### Step 1: Foundation
- [x] Create `qbit-mcp` crate with basic structure
- [x] Define `McpConfigFile` and `McpServerConfig` types (JSON schema)
- [x] Implement config file loading (`~/.qbit/mcp.json`)
- [x] Implement environment variable interpolation (both `$VAR` and `${VAR}`)
- [x] Implement single server connection (stdio transport)
- [x] Basic tool listing and calling via `rmcp`

### Step 2: Multi-Server Management
- [x] Implement `McpManager` for multiple concurrent servers
- [x] Add tool namespacing (`mcp__{server}__{tool}`)
- [x] ~~Handle `list_changed` notifications from servers~~ (deferred - infrastructure exists, runtime refresh not needed for v1)
- [x] Implement connection persistence (per-session via configure_bridge)
- [x] Handle failed server connections (warn but continue)

**Note**: McpManager is created per-session in `configure_bridge()` based on workspace path. This allows project-specific MCP configs to be loaded correctly. The manager persists for the lifetime of the agent session.

### Step 3: Tool Integration
- [x] ~~Add `Mcp` variant to `ToolRoutingCategory`~~ (Not needed - using custom_tool_executor)
- [x] Integrate with `AgenticLoopContext`
- [x] Bridge MCP tools to rig `ToolDefinition`
- [x] Route MCP tool calls through manager
- [x] Ensure MCP tools use same HITL approval as registry tools

**Implementation approach**: 
- MCP tools are exposed via `AgenticLoopContext.additional_tool_definitions` (loaded from McpManager)
- Tool calls are routed via `custom_tool_executor` closure that calls `manager.call_tool()`
- No special routing category needed - the executor handles all `mcp__*` prefixed tools
- MCP tools go through the same HITL approval flow as registry tools (no special handling)
- Integration happens in `qbit/src/ai/commands/mod.rs::configure_bridge()` after sidecar/settings setup

### Step 4: HTTP/SSE Transport
- [x] Add HTTP transport support (via `StreamableHttpClientTransport`)
- [x] ~~Add SSE transport support~~ (SSE deprecated in rmcp 0.14; use streamable HTTP instead)
- [x] Handle authentication headers (Bearer token support)
- [x] Implement header environment variable interpolation

### Step 5: Configuration Loading
- [x] Load user-global config (`~/.qbit/mcp.json`)
- [x] Load project config (`<project>/.qbit/mcp.json`)
- [x] Implement config merging (project overrides user-global)
- [x] Implement project config trust system (`~/.qbit/trusted-mcp-configs.json`)
- [ ] First-time approval prompt for untrusted project configs (frontend UI needed)

### Step 6: Tauri Commands
- [x] `mcp_list_servers` - List configured servers with status
- [x] `mcp_connect` / `mcp_disconnect` - Manual connection control
- [x] `mcp_list_tools` - List tools from all connected servers
- [ ] `mcp_add_server` / `mcp_remove_server` - Config management (deferred to Step 8)
- [x] `mcp_trust_project_config` - Trust a project's MCP config
- [x] `mcp_is_project_trusted` - Check if project config is trusted
- [x] `mcp_get_config` - Get merged MCP config for a workspace
- [x] `mcp_has_project_config` - Check if project has MCP config
- [x] Add to command registration in `lib.rs`
- [x] Store McpManager per-session in AiState for connect/disconnect access
- [x] Create TypeScript wrappers in `frontend/lib/mcp.ts`

**Implementation notes**:
- Added `mcp_managers` field to `AiState` with `get_session_mcp_manager()`, `set_session_mcp_manager()`, `remove_session_mcp_manager()` methods
- `configure_bridge()` now passes session_id to `initialize_mcp_integration()`
- Manager is stored in AiState after initialization for later access by connect/disconnect commands
- Commands implemented in `backend/crates/qbit/src/mcp/commands.rs`

### Step 7: CLI Support
- [x] Load MCP config in headless CLI mode
- [x] Auto-connect to configured servers on CLI startup
- [x] Ensure MCP tools available in CLI agent loop

**Implementation notes**:
- MCP integration added to `backend/crates/qbit/src/cli/bootstrap.rs`
- `initialize_mcp_integration()` function mirrors the Tauri implementation
- Config loaded from user-global and project-specific paths
- Servers auto-connect during agent initialization
- MCP tools exposed via `bridge.set_mcp_tools()` and `bridge.set_mcp_executor()`
- Verbose mode (`-v`) shows MCP server count and tool count
- Non-fatal: if MCP fails, agent continues without MCP tools

### Step 8: Frontend
- [x] Add MCP section to Settings UI
- [x] Server list with connection status indicators
- [ ] Add/edit/remove server forms (deferred - config is JSON file based)
- [x] Tool browser per server
- [ ] Project config trust approval dialog (deferred)

**Implementation notes**:
- Created `frontend/components/Settings/McpSettings.tsx`
- Added "MCP Servers" section to Settings dialog navigation
- TypeScript wrappers in `frontend/lib/mcp.ts`
- Features:
  - Server list with transport type, source (user/project), and enabled status badges
  - Connection status indicators (connected, disconnected, connecting, error)
  - Connect/disconnect buttons per server
  - Tool browser (expandable per connected server) showing tool name and description
  - Session-aware: shows warning if no active session
  - Refresh button and "Browse servers" link to MCP registry
  - Config location info pointing to `~/.qbit/mcp.json` and project paths

### Step 9: Auto-Connect & Lifecycle
- [x] Auto-connect to enabled servers on session start (in `configure_bridge()`)
- [x] Graceful shutdown - kill stdio server processes on app close (via `McpManager::shutdown()`)
- [ ] Handle server reconnection on config changes

**Implementation notes**:
- `McpManager::shutdown()` / `disconnect_all()` disconnects from all servers
- CLI: `CliContext::shutdown()` calls `manager.shutdown().await` before other cleanup
- Tauri: Session cleanup calls `disconnect_all()` when bridge is removed

### Step 10: Polish & Testing
- [x] Add unit tests for config loading and merging (41 tests in qbit-mcp)
- [ ] Add integration tests for server connections (deferred - requires external MCP server)
- [ ] Error handling and user-friendly error messages
- [ ] Logging and diagnostics (without exposing secrets)
- [ ] Documentation updates (README, settings template)

**Unit test coverage (41 tests):**
- `config.rs`: 9 tests - JSON deserialization, defaults, transport types
- `loader.rs`: 18 tests - env var interpolation, config loading, merging, trust system
- `tools.rs`: 14 tests - tool name parsing, result conversion, tool definition generation
- Found and fixed bug in `interpolate_env_vars()` (empty braces handling)

---

## Design Decisions

The following decisions have been made:

1. **Transport scope**: Include all three transports from the start: stdio, HTTP, and SSE

2. **Config location**: MCP servers are configured exclusively in `.qbit/mcp.json` files (not in `settings.toml`). User-global config lives at `~/.qbit/mcp.json`, project-specific config at `<project>/.qbit/mcp.json`

3. **Tool approval UX**: MCP tools use the same `registry` category as built-in tools (no separate category)

4. **Auto-connect behavior**: Configured MCP servers auto-connect on session start

5. **CLI support**: Yes, headless CLI mode should also support MCP servers

6. **Server process lifecycle**: stdio MCP servers are killed immediately when Qbit closes

7. **Resources & Prompts**: Tools only for v1; resources and prompts deferred to future iteration

8. **Project config trust**: First-time approval prompt when a project's `.qbit/mcp.json` is detected (security for untrusted repos)

9. **Failed server behavior**: Show warning but continue (don't block on failed MCP server connections)

10. **Connection persistence**: MCP connections persist for the lifetime of the app (not per-session)

11. **Environment variable syntax**: Support both `$VAR` and `${VAR}` (matching shell conventions)

12. **Tool count limits**: Trust the user to configure wisely (no artificial limits)

---

## Appendix A: rmcp Crate API Reference

### Cargo Dependencies

```toml
[dependencies]
rmcp = { version = "0.14", features = [
    "client",
    "transport-child-process",
    "transport-io",
    "transport-streamable-http-client",
    "transport-streamable-http-client-reqwest"
]}
tokio = { version = "1", features = ["full"] }
serde_json = "1"
tracing = "0.1"
```

### Client Connection Patterns

**stdio Transport (child process):**
```rust
use rmcp::{ServiceExt, transport::TokioChildProcess};
use tokio::process::Command;

let transport = TokioChildProcess::new(
    Command::new("npx")
        .args(["-y", "@modelcontextprotocol/server-filesystem", "/path"])
)?;
let client = ().serve(transport).await?;
```

**SSE Transport:**
```rust
use rmcp::{ServiceExt, transport::sse::SseTransport};

let transport = SseTransport::start("http://localhost:8080/sse").await?;
let client = ().serve(transport).await?;
```

**Streamable HTTP Transport:**
```rust
use rmcp::{ServiceExt, transport::streamable_http_client::StreamableHttpClientTransport};

let transport = StreamableHttpClientTransport::new("http://localhost:8080/mcp")?;
let client = ().serve(transport).await?;
```

### Tool Operations

**Listing tools:**
```rust
use rmcp::model::ListToolsRequestParams;

let tools_result = client.list_tools(ListToolsRequestParams::default()).await?;
// tools_result.tools: Vec<Tool>
for tool in &tools_result.tools {
    println!("{}: {}", tool.name, tool.description.as_deref().unwrap_or(""));
    // tool.input_schema: JSON Schema for arguments
}
```

**Calling a tool:**
```rust
use rmcp::model::CallToolRequestParams;

let result = client.call_tool(CallToolRequestParams {
    meta: None,
    name: "tool_name".into(),
    arguments: serde_json::json!({"param": "value"}).as_object().cloned(),
    task: None,
}).await?;

// result.content: Vec<Content> - text, image, or resource
// result.is_error: Option<bool> - true if tool reports failure
for content in result.content {
    match content {
        Content::Text { text } => println!("Result: {}", text),
        Content::Image { data, mime_type } => { /* base64 image */ }
        Content::Resource { uri, text, .. } => { /* resource reference */ }
        _ => {}
    }
}
```

### Handling Server Notifications

Implement `ClientHandler` to receive notifications:

```rust
use rmcp::{
    ClientHandler,
    service::NotificationContext,
    model::{ProgressNotificationParam, RoleClient},
};

struct McpClientHandler {
    // Store reference to refresh tool cache
}

impl ClientHandler for McpClientHandler {
    async fn on_tools_list_changed(
        &self,
        _notification: (),
        context: NotificationContext<RoleClient>,
    ) {
        // Server's tool list changed - refresh cache
        if let Ok(tools) = context.peer.list_tools(Default::default()).await {
            // Update tool cache
        }
    }

    async fn on_progress(
        &self,
        notification: ProgressNotificationParam,
        _context: NotificationContext<RoleClient>,
    ) {
        // Handle progress updates for long-running tools
        println!("Progress: {} / {:?}", notification.progress, notification.total);
    }
    
    async fn on_resources_list_changed(&self, _: (), _: NotificationContext<RoleClient>) {}
    async fn on_prompts_list_changed(&self, _: (), _: NotificationContext<RoleClient>) {}
}

// Use with handler:
let client = McpClientHandler { /* ... */ }.serve(transport).await?;
```

### Error Handling

```rust
use rmcp::ErrorData as McpError;

match client.call_tool(params).await {
    Ok(result) => {
        if result.is_error.unwrap_or(false) {
            // Tool executed but reported an error
            eprintln!("Tool error: {:?}", result.content);
        } else {
            // Success
        }
    }
    Err(mcp_error) => {
        // Protocol-level error (connection, invalid request, etc.)
        eprintln!("MCP Error {}: {}", mcp_error.code, mcp_error.message);
    }
}
```

### Disconnection

```rust
// Graceful disconnect
client.cancel().await?;

// Wait for connection to end naturally (e.g., server closes)
let quit_reason = client.waiting().await?;
```

---

## Appendix B: Qbit Codebase Integration Points

### 1. ToolRoutingCategory

**Location:** `backend/crates/qbit-ai/src/tool_execution.rs` (lines ~190-250)

Add new variant for MCP:
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolRoutingCategory {
    Indexer,
    WebFetch,
    UpdatePlan,
    SubAgent,
    Mcp,      // NEW
    Registry,
}

impl ToolRoutingCategory {
    pub fn from_tool_name(name: &str) -> Self {
        if name.starts_with("mcp__") {
            Self::Mcp
        } else if name.starts_with("indexer_") {
            Self::Indexer
        }
        // ... existing logic
    }
}
```

### 2. AgenticLoopContext

**Location:** `backend/crates/qbit-ai/src/agentic_loop.rs` (lines ~233-296)

Key fields for MCP integration:
```rust
pub struct AgenticLoopContext<'a> {
    // ... existing fields ...
    
    /// Additional tool definitions injected at runtime (used for MCP tools)
    pub additional_tool_definitions: Vec<rig::completion::ToolDefinition>,
    
    /// Custom executor for tools not in registry (used for MCP tool calls)
    pub custom_tool_executor: Option<Arc<dyn CustomToolExecutor>>,
    
    // NEW: MCP manager reference
    pub mcp_manager: Option<&'a Arc<McpManager>>,
}
```

**Construction:** `build_loop_context()` in `backend/crates/qbit-ai/src/agent_bridge.rs` (line ~1417)

### 3. Using additional_tool_definitions

**How it works:** Tools are added to the LLM prompt at line ~1421 in agentic_loop.rs:
```rust
tools.extend(ctx.additional_tool_definitions.iter().cloned());
```

**For MCP:** Convert MCP tools to `rig::completion::ToolDefinition`:
```rust
impl McpTool {
    pub fn to_tool_definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: format!("mcp__{}__{}", 
                self.server_name.replace("-", "_"),
                self.tool_name.replace("-", "_")),
            description: self.description.clone()
                .unwrap_or_else(|| format!("MCP tool from {}", self.server_name)),
            parameters: self.input_schema.clone(),
        }
    }
}
```

### 4. HITL Approval Flow

**Location:** `backend/crates/qbit-ai/src/agentic_loop.rs` (lines ~1090-1175)

MCP tools go through the same flow:
1. `ToolApprovalRequest` event emitted to frontend
2. Frontend responds via coordinator or channel
3. `ApprovalRecorder` learns patterns for auto-approve

**Key event type:** `AiEvent::ToolApprovalRequest` in `backend/crates/qbit-core/src/events.rs`

MCP tools will use `ToolRoutingCategory::Registry` for approval (per design decision #3), 
so no changes needed to the approval logic itself.

### 5. Event Emission Pattern

**Event definitions:** `backend/crates/qbit-core/src/events.rs`

**Emission helpers in agentic_loop.rs:**
```rust
// Send to frontend + transcript
emit_to_frontend(&ctx.event_tx, event, &transcript).await;

// Send to frontend + transcript + sidecar
emit_event(&ctx.event_tx, event, &transcript, &sidecar).await;
```

**For MCP events (new variants to add):**
```rust
pub enum AiEvent {
    // ... existing variants ...
    
    McpServerConnected { server_name: String, tool_count: usize },
    McpServerDisconnected { server_name: String, reason: Option<String> },
    McpServerError { server_name: String, error: String },
    McpToolsUpdated { server_name: String, tools: Vec<String> },
}
```

### 6. Execution Flow for MCP Tools

In `route_tool_execution()` (tool_execution.rs), add MCP handling:

```rust
match ToolRoutingCategory::from_tool_name(&tool_name) {
    ToolRoutingCategory::Mcp => {
        if let Some(mcp_manager) = &ctx.mcp_manager {
            // Parse tool name: mcp__{server}__{tool}
            let (server_name, tool_name) = parse_mcp_tool_name(&tool_name)?;
            let result = mcp_manager.call_tool(server_name, tool_name, arguments).await?;
            // Convert McpToolResult to ToolResult
            Ok(convert_mcp_result(result))
        } else {
            Err(anyhow!("MCP not configured"))
        }
    }
    // ... existing categories
}
```

---

## Appendix C: MCP Protocol Reference

### Server Capabilities

Servers can declare these capabilities during initialization:
- `tools`: Server provides tools
- `resources`: Server provides resources (files, data)
- `prompts`: Server provides templated prompts
- `logging`: Server can send log messages

### JSON-RPC Methods (Client → Server)

| Method | Description |
|--------|-------------|
| `initialize` | Initialize the connection |
| `tools/list` | List available tools |
| `tools/call` | Execute a tool |
| `resources/list` | List available resources |
| `resources/read` | Read a resource |
| `prompts/list` | List available prompts |
| `prompts/get` | Get a prompt template |

### Notifications (Server → Client)

| Notification | Description |
|--------------|-------------|
| `notifications/tools/list_changed` | Tool list has changed |
| `notifications/resources/list_changed` | Resource list has changed |
| `notifications/progress` | Progress update |
| `notifications/message` | Log message |

---

## References

- [MCP Specification (2025-11-25)](https://modelcontextprotocol.io/specification/2025-11-25)
- [Official Rust SDK (rmcp)](https://github.com/modelcontextprotocol/rust-sdk)
- [rmcp crate docs](https://docs.rs/rmcp/latest/rmcp/)
- [Claude Code MCP Documentation](https://code.claude.com/docs/en/mcp)
- [MCP Server Registry](https://github.com/modelcontextprotocol/servers)
