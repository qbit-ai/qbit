# vtcode-core Parallel Migration Plan

## Strategy: Drop-in Replacement Modules

Create entirely separate modules that mirror vtcode-core's interfaces exactly. This allows:
- **Zero disruption** to existing working code
- **Incremental migration** one component at a time
- **Easy rollback** via feature flags
- **Side-by-side testing** to verify behavior parity

---

## Architecture Overview

```
Current State:
┌─────────────────────────────────────────────────────┐
│                    AI Module                         │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  │
│  │agentic_loop │  │ agent_bridge│  │  session.rs │  │
│  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘  │
│         │                │                │         │
│         └────────────────┼────────────────┘         │
│                          ▼                          │
│              ┌───────────────────────┐              │
│              │  vtcode_core (v0.47)  │              │
│              │  - ToolRegistry       │              │
│              │  - SessionArchive     │              │
│              │  - MessageRole        │              │
│              └───────────────────────┘              │
└─────────────────────────────────────────────────────┘

Target State:
┌─────────────────────────────────────────────────────┐
│                    AI Module                         │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  │
│  │agentic_loop │  │ agent_bridge│  │  session.rs │  │
│  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘  │
│         │                │                │         │
│         └────────────────┼────────────────┘         │
│                          ▼                          │
│         ┌────────────────────────────────┐          │
│         │      Compatibility Layer       │          │
│         │   (feature flag: local-tools)  │          │
│         └───────────┬────────────────────┘          │
│                     │                               │
│        ┌────────────┴────────────┐                  │
│        ▼                         ▼                  │
│  ┌───────────────┐      ┌───────────────┐          │
│  │ vtcode_core   │  OR  │  Local Modules │          │
│  │  (existing)   │      │  (new)         │          │
│  └───────────────┘      └───────────────┘          │
└─────────────────────────────────────────────────────┘
```

---

## Module 1: Tool Registry (`backend/src/tools/`)

### 1.1 Interface Contract (Must Match Exactly)

The existing code uses `vtcode_core::tools::ToolRegistry` with this exact interface:

```rust
// Current usage patterns to preserve:

// 1. Creation (llm_client.rs:84)
ToolRegistry::new(workspace.to_path_buf()).await

// 2. Tool execution (agentic_loop.rs:425, workflow.rs:537)
registry.execute_tool(tool_name, tool_args.clone()).await
// Returns: Result<serde_json::Value, Error>

// 3. Tool listing (not currently used but part of interface)
registry.available_tools()
// Returns: Vec<String>
```

### 1.2 Files to Create

```
backend/src/tools/
├── mod.rs              # Module root + re-exports
├── registry.rs         # ToolRegistry struct
├── traits.rs           # Tool trait definition
├── error.rs            # ToolError enum
├── file_ops.rs         # read_file, write_file, create_file, edit_file, delete_file
├── directory_ops.rs    # list_files, list_directory, grep_file
├── shell.rs            # run_pty_cmd
└── definitions.rs      # build_function_declarations() replacement
```

### 1.3 Drop-in Interface

**File: `backend/src/tools/mod.rs`**
```rust
//! Local tool registry - drop-in replacement for vtcode_core::tools::ToolRegistry

mod registry;
mod traits;
mod error;
mod file_ops;
mod directory_ops;
mod shell;
mod definitions;

pub use registry::ToolRegistry;
pub use traits::Tool;
pub use error::ToolError;
pub use definitions::build_function_declarations;

// Re-export for compatibility
pub mod registry {
    pub use super::definitions::build_function_declarations;
}
```

**File: `backend/src/tools/registry.rs`**
```rust
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use anyhow::Result;
use serde_json::Value;

use super::traits::Tool;
use super::file_ops::*;
use super::directory_ops::*;
use super::shell::*;

/// Drop-in replacement for vtcode_core::tools::ToolRegistry
///
/// IMPORTANT: This struct's public interface MUST match vtcode_core exactly
/// to allow seamless migration.
pub struct ToolRegistry {
    tools: HashMap<String, Arc<dyn Tool>>,
    workspace: PathBuf,
}

impl ToolRegistry {
    /// Create a new ToolRegistry for the given workspace.
    ///
    /// Signature matches: vtcode_core::tools::ToolRegistry::new()
    pub async fn new(workspace: PathBuf) -> Self {
        let mut tools: HashMap<String, Arc<dyn Tool>> = HashMap::new();

        // Register all tools
        let tool_list: Vec<Arc<dyn Tool>> = vec![
            // File operations
            Arc::new(ReadFileTool),
            Arc::new(WriteFileTool),
            Arc::new(CreateFileTool),
            Arc::new(EditFileTool),
            Arc::new(DeleteFileTool),
            // Directory operations
            Arc::new(ListFilesTool),
            Arc::new(ListDirectoryTool),
            Arc::new(GrepFileTool),
            // Shell
            Arc::new(RunPtyCmdTool),
        ];

        for tool in tool_list {
            tools.insert(tool.name().to_string(), tool);
        }

        Self { tools, workspace }
    }

    /// Execute a tool by name with the given arguments.
    ///
    /// Signature matches: vtcode_core::tools::ToolRegistry::execute_tool()
    ///
    /// Returns JSON with optional `error` and `exit_code` fields for failure detection.
    pub async fn execute_tool(
        &mut self,
        name: &str,
        args: Value,
    ) -> Result<Value> {
        let tool = self.tools.get(name)
            .ok_or_else(|| anyhow::anyhow!("Unknown tool: {}", name))?;

        tool.execute(args, &self.workspace).await
    }

    /// List all available tool names.
    ///
    /// Signature matches: vtcode_core::tools::ToolRegistry::available_tools()
    pub fn available_tools(&self) -> Vec<String> {
        self.tools.keys().cloned().collect()
    }
}
```

**File: `backend/src/tools/traits.rs`**
```rust
use std::path::Path;
use anyhow::Result;
use serde_json::Value;

/// Trait for tool implementations.
///
/// All tools must be Send + Sync because ToolRegistry is wrapped in Arc<RwLock<>>.
#[async_trait::async_trait]
pub trait Tool: Send + Sync {
    /// Tool name (must match exactly what LLM requests)
    fn name(&self) -> &'static str;

    /// Tool description for LLM context
    fn description(&self) -> &'static str;

    /// JSON Schema for tool parameters
    fn parameters(&self) -> Value;

    /// Execute the tool with given arguments.
    ///
    /// ## Return Format Contract
    ///
    /// Success: Return any JSON value (object, string, etc.)
    /// - Do NOT include "error" field
    /// - For shell commands, include "exit_code": 0
    ///
    /// Failure: Return JSON object with:
    /// - "error": "error message"
    /// - For shell commands, also "exit_code": <non-zero>
    ///
    /// This contract is enforced by agentic_loop.rs:429-436
    async fn execute(&self, args: Value, workspace: &Path) -> Result<Value>;
}
```

### 1.4 Success/Failure Contract

The agentic loop determines success by checking two conditions:

```rust
// From agentic_loop.rs:429-436
let is_failure_by_exit_code = v.get("exit_code")
    .and_then(|ec| ec.as_i64())
    .map(|ec| ec != 0)
    .unwrap_or(false);
let has_error_field = v.get("error").is_some();
let is_success = !is_failure_by_exit_code && !has_error_field;
```

**All tools MUST follow this contract:**

| Scenario | Return Format | is_success |
|----------|---------------|------------|
| File read success | `{"content": "..."}` | true |
| File not found | `{"error": "File not found"}` | false |
| Shell success | `{"stdout": "...", "exit_code": 0}` | true |
| Shell failure | `{"stderr": "...", "exit_code": 1}` | false |
| Permission denied | `{"error": "Permission denied"}` | false |

---

## Module 2: Tool Definitions (`backend/src/tools/definitions.rs`)

### 2.1 Interface Contract

```rust
// Current usage (tool_definitions.rs:161)
use vtcode_core::tools::registry::build_function_declarations;

let declarations = build_function_declarations();
// Returns: Vec<FunctionDeclaration>
```

### 2.2 Drop-in Implementation

```rust
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

/// Function declaration format for LLM tool calling.
/// Must match vtcode_core::tools::registry::FunctionDeclaration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionDeclaration {
    pub name: String,
    pub description: String,
    pub parameters: Value,
}

/// Build all tool declarations for LLM consumption.
///
/// Drop-in replacement for vtcode_core::tools::registry::build_function_declarations()
pub fn build_function_declarations() -> Vec<FunctionDeclaration> {
    vec![
        // File operations
        FunctionDeclaration {
            name: "read_file".to_string(),
            description: "Read the contents of a file".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Path to the file (relative to workspace)"
                    },
                    "line_start": {
                        "type": "integer",
                        "description": "Starting line number (1-indexed)"
                    },
                    "line_end": {
                        "type": "integer",
                        "description": "Ending line number (1-indexed, inclusive)"
                    }
                },
                "required": ["path"]
            }),
        },
        FunctionDeclaration {
            name: "write_file".to_string(),
            description: "Write content to a file, replacing existing content".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Path to the file (relative to workspace)"
                    },
                    "content": {
                        "type": "string",
                        "description": "Content to write to the file"
                    }
                },
                "required": ["path", "content"]
            }),
        },
        FunctionDeclaration {
            name: "create_file".to_string(),
            description: "Create a new file (fails if file exists)".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Path for the new file (relative to workspace)"
                    },
                    "content": {
                        "type": "string",
                        "description": "Initial content for the file"
                    }
                },
                "required": ["path", "content"]
            }),
        },
        FunctionDeclaration {
            name: "edit_file".to_string(),
            description: "Edit a file by replacing text. The old_text must match exactly once.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Path to the file (relative to workspace)"
                    },
                    "old_text": {
                        "type": "string",
                        "description": "Text to find and replace (must match exactly once)"
                    },
                    "new_text": {
                        "type": "string",
                        "description": "Replacement text"
                    },
                    "display_description": {
                        "type": "string",
                        "description": "Human-readable description of the edit"
                    }
                },
                "required": ["path", "old_text", "new_text"]
            }),
        },
        FunctionDeclaration {
            name: "delete_file".to_string(),
            description: "Delete a file".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Path to the file to delete (relative to workspace)"
                    }
                },
                "required": ["path"]
            }),
        },
        // Directory operations
        FunctionDeclaration {
            name: "list_files".to_string(),
            description: "List files matching a pattern".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Directory to search (relative to workspace, default: root)"
                    },
                    "pattern": {
                        "type": "string",
                        "description": "Glob pattern to match files"
                    },
                    "recursive": {
                        "type": "boolean",
                        "description": "Search recursively (default: true)"
                    }
                },
                "required": []
            }),
        },
        FunctionDeclaration {
            name: "list_directory".to_string(),
            description: "List contents of a directory".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Directory path (relative to workspace)"
                    }
                },
                "required": ["path"]
            }),
        },
        FunctionDeclaration {
            name: "grep_file".to_string(),
            description: "Search file contents with regex".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "pattern": {
                        "type": "string",
                        "description": "Regex pattern to search for"
                    },
                    "path": {
                        "type": "string",
                        "description": "File or directory to search"
                    },
                    "include": {
                        "type": "string",
                        "description": "Glob pattern to filter files"
                    }
                },
                "required": ["pattern"]
            }),
        },
        // Shell execution
        FunctionDeclaration {
            name: "run_pty_cmd".to_string(),
            description: "Execute a shell command".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "command": {
                        "type": "string",
                        "description": "Shell command to execute"
                    },
                    "cwd": {
                        "type": "string",
                        "description": "Working directory (relative to workspace)"
                    },
                    "timeout": {
                        "type": "integer",
                        "description": "Timeout in seconds (default: 120)"
                    }
                },
                "required": ["command"]
            }),
        },
    ]
}
```

---

## Module 3: Session Archive (`backend/src/session/`)

### 3.1 Interface Contract

```rust
// Current usage patterns:

// 1. Create archive (session.rs:297)
SessionArchive::new(metadata).await

// 2. Finalize session (session.rs:305-312)
archive.finalize(transcript, message_count, distinct_tools, messages)
// Returns: PathBuf

// 3. Find session (session.rs:472)
session_archive::find_session_by_identifier(id)
// Returns: Option<SessionListing>

// 4. List sessions
session_archive::list_recent_sessions(limit)
// Returns: Vec<SessionListing>

// 5. Message creation (session.rs:299, 344)
SessionMessage::with_tool_call_id(role, content, tool_call_id)

// 6. Content access (session.rs:488)
message.content.as_text()
// Returns: String
```

### 3.2 Files to Create

```
backend/src/session/
├── mod.rs              # Module root + re-exports
├── archive.rs          # SessionArchive struct
├── message.rs          # SessionMessage, MessageRole
├── storage.rs          # File I/O operations
└── listing.rs          # SessionListing, find/list functions
```

### 3.3 Drop-in Implementation

**File: `backend/src/session/mod.rs`**
```rust
//! Local session archive - drop-in replacement for vtcode_core::utils::session_archive

mod archive;
mod message;
mod storage;
mod listing;

pub use archive::{SessionArchive, SessionArchiveMetadata};
pub use message::{SessionMessage, MessageRole, MessageContent};
pub use listing::{SessionListing, SessionSnapshot, find_session_by_identifier, list_recent_sessions};
```

**File: `backend/src/session/message.rs`**
```rust
use serde::{Deserialize, Serialize};

/// Message role enum - must match vtcode_core::llm::provider::MessageRole
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    User,
    Assistant,
    System,
    Tool,
}

/// Message content wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MessageContent {
    Text(String),
    Structured { text: String },
}

impl MessageContent {
    /// Extract text content - matches vtcode_core behavior
    pub fn as_text(&self) -> String {
        match self {
            MessageContent::Text(s) => s.clone(),
            MessageContent::Structured { text } => text.clone(),
        }
    }
}

/// Session message - drop-in for vtcode_core::utils::session_archive::SessionMessage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMessage {
    pub role: MessageRole,
    pub content: MessageContent,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

impl SessionMessage {
    /// Create message with tool call ID
    ///
    /// Signature matches: vtcode_core SessionMessage::with_tool_call_id()
    pub fn with_tool_call_id(
        role: MessageRole,
        content: &str,
        tool_call_id: Option<String>,
    ) -> Self {
        Self {
            role,
            content: MessageContent::Text(content.to_string()),
            tool_call_id,
        }
    }

    /// Create simple message without tool call ID
    pub fn new(role: MessageRole, content: &str) -> Self {
        Self::with_tool_call_id(role, content, None)
    }
}
```

**File: `backend/src/session/archive.rs`**
```rust
use std::path::PathBuf;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use anyhow::Result;

use super::message::SessionMessage;
use super::storage;

/// Session metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionArchiveMetadata {
    pub session_id: String,
    pub workspace: PathBuf,
    pub model: String,
    pub provider: String,
    #[serde(default)]
    pub tags: Vec<String>,
}

/// Session archive - drop-in for vtcode_core::utils::session_archive::SessionArchive
pub struct SessionArchive {
    metadata: SessionArchiveMetadata,
    started_at: DateTime<Utc>,
    session_dir: PathBuf,
}

impl SessionArchive {
    /// Create new session archive
    ///
    /// Signature matches: vtcode_core SessionArchive::new()
    pub async fn new(metadata: SessionArchiveMetadata) -> Result<Self> {
        let session_dir = storage::get_sessions_dir()?;

        Ok(Self {
            metadata,
            started_at: Utc::now(),
            session_dir,
        })
    }

    /// Finalize and persist the session
    ///
    /// Signature matches: vtcode_core SessionArchive::finalize()
    pub fn finalize(
        self,
        transcript: String,
        message_count: usize,
        distinct_tools: Vec<String>,
        messages: Vec<SessionMessage>,
    ) -> Result<PathBuf> {
        let ended_at = Utc::now();

        let snapshot = super::listing::SessionSnapshot {
            metadata: self.metadata,
            started_at: self.started_at,
            ended_at,
            message_count,
            distinct_tools,
            transcript,
            messages,
        };

        storage::save_session(&self.session_dir, &snapshot)
    }
}
```

**File: `backend/src/session/listing.rs`**
```rust
use std::path::PathBuf;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use anyhow::Result;

use super::archive::SessionArchiveMetadata;
use super::message::SessionMessage;
use super::storage;

/// Full session snapshot (serialized to disk)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSnapshot {
    pub metadata: SessionArchiveMetadata,
    pub started_at: DateTime<Utc>,
    pub ended_at: DateTime<Utc>,
    pub message_count: usize,
    pub distinct_tools: Vec<String>,
    pub transcript: String,
    pub messages: Vec<SessionMessage>,
}

/// Session listing entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionListing {
    pub session_id: String,
    pub path: PathBuf,
    pub started_at: DateTime<Utc>,
    pub ended_at: DateTime<Utc>,
    pub snapshot: SessionSnapshot,
}

/// Find session by ID or path
///
/// Signature matches: vtcode_core session_archive::find_session_by_identifier()
pub fn find_session_by_identifier(identifier: &str) -> Result<Option<SessionListing>> {
    storage::find_session(identifier)
}

/// List recent sessions
///
/// Signature matches: vtcode_core session_archive::list_recent_sessions()
pub fn list_recent_sessions(limit: usize) -> Result<Vec<SessionListing>> {
    storage::list_sessions(limit)
}
```

**File: `backend/src/session/storage.rs`**
```rust
use std::path::PathBuf;
use std::fs;
use anyhow::Result;

use super::listing::{SessionSnapshot, SessionListing};

/// Get sessions directory (respects VT_SESSION_DIR env var)
pub fn get_sessions_dir() -> Result<PathBuf> {
    let dir = if let Ok(custom) = std::env::var("VT_SESSION_DIR") {
        PathBuf::from(custom)
    } else {
        dirs::home_dir()
            .ok_or_else(|| anyhow::anyhow!("Cannot find home directory"))?
            .join(".qbit")
            .join("sessions")
    };

    fs::create_dir_all(&dir)?;
    Ok(dir)
}

/// Save session to disk
pub fn save_session(dir: &PathBuf, snapshot: &SessionSnapshot) -> Result<PathBuf> {
    let filename = format!(
        "{}_{}.json",
        snapshot.started_at.format("%Y%m%d_%H%M%S"),
        &snapshot.metadata.session_id[..8]
    );
    let path = dir.join(&filename);

    let json = serde_json::to_string_pretty(snapshot)?;
    fs::write(&path, json)?;

    Ok(path)
}

/// Find session by identifier
pub fn find_session(identifier: &str) -> Result<Option<SessionListing>> {
    let dir = get_sessions_dir()?;

    for entry in fs::read_dir(&dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.extension().map(|e| e == "json").unwrap_or(false) {
            if let Ok(content) = fs::read_to_string(&path) {
                if let Ok(snapshot) = serde_json::from_str::<SessionSnapshot>(&content) {
                    // Match by session_id or filename
                    if snapshot.metadata.session_id.starts_with(identifier)
                        || path.file_stem()
                            .and_then(|s| s.to_str())
                            .map(|s| s.contains(identifier))
                            .unwrap_or(false)
                    {
                        return Ok(Some(SessionListing {
                            session_id: snapshot.metadata.session_id.clone(),
                            path,
                            started_at: snapshot.started_at,
                            ended_at: snapshot.ended_at,
                            snapshot,
                        }));
                    }
                }
            }
        }
    }

    Ok(None)
}

/// List recent sessions
pub fn list_sessions(limit: usize) -> Result<Vec<SessionListing>> {
    let dir = get_sessions_dir()?;
    let mut sessions = Vec::new();

    for entry in fs::read_dir(&dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.extension().map(|e| e == "json").unwrap_or(false) {
            if let Ok(content) = fs::read_to_string(&path) {
                if let Ok(snapshot) = serde_json::from_str::<SessionSnapshot>(&content) {
                    sessions.push(SessionListing {
                        session_id: snapshot.metadata.session_id.clone(),
                        path,
                        started_at: snapshot.started_at,
                        ended_at: snapshot.ended_at,
                        snapshot,
                    });
                }
            }
        }
    }

    // Sort by started_at descending
    sessions.sort_by(|a, b| b.started_at.cmp(&a.started_at));
    sessions.truncate(limit);

    Ok(sessions)
}
```

---

## Module 4: Compatibility Layer

### 4.1 Feature Flag Setup

**File: `backend/Cargo.toml`**
```toml
[features]
default = ["tauri"]
tauri = ["tauri/..."]
cli = ["..."]
local-tools = []  # NEW: Use local tool implementations
```

### 4.2 Conditional Imports

**File: `backend/src/ai/mod.rs`**
```rust
// Conditional tool registry import
#[cfg(feature = "local-tools")]
pub use crate::tools::ToolRegistry;

#[cfg(not(feature = "local-tools"))]
pub use vtcode_core::tools::ToolRegistry;

// Conditional session archive import
#[cfg(feature = "local-tools")]
pub use crate::session::{
    SessionArchive, SessionArchiveMetadata, SessionMessage, MessageRole,
    find_session_by_identifier, list_recent_sessions,
};

#[cfg(not(feature = "local-tools"))]
pub use vtcode_core::utils::session_archive::{
    SessionArchive, SessionArchiveMetadata, SessionMessage,
    find_session_by_identifier, list_recent_sessions,
};

#[cfg(not(feature = "local-tools"))]
pub use vtcode_core::llm::provider::MessageRole;
```

### 4.3 Unified Re-exports

Create a single import point that works with either implementation:

**File: `backend/src/compat.rs`**
```rust
//! Compatibility layer for vtcode-core migration
//!
//! This module provides unified imports that work with either:
//! - vtcode-core (default)
//! - local implementations (with `local-tools` feature)

// Tool Registry
#[cfg(feature = "local-tools")]
pub mod tools {
    pub use crate::tools::{ToolRegistry, Tool, build_function_declarations};
}

#[cfg(not(feature = "local-tools"))]
pub mod tools {
    pub use vtcode_core::tools::{ToolRegistry, registry::build_function_declarations};

    // Placeholder trait for compatibility
    pub trait Tool: Send + Sync {
        fn name(&self) -> &str;
    }
}

// Session Archive
#[cfg(feature = "local-tools")]
pub mod session {
    pub use crate::session::*;
}

#[cfg(not(feature = "local-tools"))]
pub mod session {
    pub use vtcode_core::utils::session_archive::*;
    pub use vtcode_core::llm::provider::MessageRole;
}
```

---

## Migration Workflow

### Step 1: Create New Modules (No Changes to Existing Code)

```bash
# Create module directories
mkdir -p backend/src/tools
mkdir -p backend/src/session

# Create all new files (empty stubs initially)
touch backend/src/tools/{mod,registry,traits,error,file_ops,directory_ops,shell,definitions}.rs
touch backend/src/session/{mod,archive,message,storage,listing}.rs
touch backend/src/compat.rs
```

### Step 2: Implement and Test in Isolation

```bash
# Run with local-tools feature to test new implementation
cargo test --features local-tools

# Run without feature to verify existing code still works
cargo test
```

### Step 3: Add Integration Tests

**File: `backend/tests/tool_registry_compat.rs`**
```rust
//! Tests that verify local tools behave identically to vtcode-core

#[cfg(feature = "local-tools")]
mod local_tests {
    use crate::tools::ToolRegistry;
    use serde_json::json;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_read_file_success_format() {
        let dir = tempdir().unwrap();
        let workspace = dir.path().to_path_buf();

        // Create test file
        std::fs::write(workspace.join("test.txt"), "hello world").unwrap();

        let mut registry = ToolRegistry::new(workspace).await;
        let result = registry.execute_tool("read_file", json!({"path": "test.txt"})).await.unwrap();

        // Verify success format
        assert!(result.get("error").is_none(), "Success should not have error field");
        assert!(result.get("content").is_some(), "Success should have content");
    }

    #[tokio::test]
    async fn test_read_file_failure_format() {
        let dir = tempdir().unwrap();
        let workspace = dir.path().to_path_buf();

        let mut registry = ToolRegistry::new(workspace).await;
        let result = registry.execute_tool("read_file", json!({"path": "nonexistent.txt"})).await.unwrap();

        // Verify failure format
        assert!(result.get("error").is_some(), "Failure must have error field");
    }

    #[tokio::test]
    async fn test_run_pty_cmd_exit_code() {
        let dir = tempdir().unwrap();
        let workspace = dir.path().to_path_buf();

        let mut registry = ToolRegistry::new(workspace).await;

        // Success case
        let result = registry.execute_tool("run_pty_cmd", json!({"command": "echo hello"})).await.unwrap();
        assert_eq!(result.get("exit_code").and_then(|v| v.as_i64()), Some(0));

        // Failure case
        let result = registry.execute_tool("run_pty_cmd", json!({"command": "exit 1"})).await.unwrap();
        assert_ne!(result.get("exit_code").and_then(|v| v.as_i64()), Some(0));
    }
}
```

### Step 4: Gradual Migration

1. **Week 1:** Implement tools module, test with `--features local-tools`
2. **Week 2:** Implement session module, test compatibility
3. **Week 3:** Run full app with `local-tools` feature in development
4. **Week 4:** Enable by default, keep vtcode-core as fallback
5. **Week 5:** Remove vtcode-core dependency

### Step 5: Final Cleanup

```bash
# Remove vtcode-core from Cargo.toml
# Remove all #[cfg(not(feature = "local-tools"))] blocks
# Remove compat.rs
# Update CLAUDE.md
```

---

## Hardcoded Tool Names to Preserve

These tool names are hardcoded in multiple files and MUST remain unchanged:

### File Tracking (sidecar/capture.rs)

```rust
// Read tools (line 350-354)
"read_file", "list_files", "list_directory", "grep", "find_path", "diagnostics"

// Write tools (line 358-371)
"write_file", "create_file", "edit_file", "delete_file", "delete_path",
"rename_file", "move_file", "move_path", "copy_path", "create_directory"

// Edit tools (line 375-376)
"edit_file", "write_file", "create_file"
```

### HITL Risk Levels (hitl/approval_recorder.rs)

```rust
// Low risk (auto-approve candidates)
"read_file", "grep_file", "list_files", "indexer_*", "search_*", "get_errors", "diagnostics"

// Medium risk
"write_file", "create_file", "edit_file", "apply_patch", "sub_agent_*"

// High risk
"run_pty_cmd", "create_pty_session", "send_pty_input"

// Critical risk
"delete_file", "execute_code"
```

### Argument Names to Preserve

```rust
// Path extraction (sidecar/capture.rs:426-432)
"path", "file_path", "filepath", "target_path"  // Primary path
"source_path", "from"                            // Source for move/rename
"destination_path", "to"                         // Destination for move/rename
```

---

## Testing Checklist

### Unit Tests (per tool)
- [ ] read_file: success returns content, no error field
- [ ] read_file: missing file returns error field
- [ ] read_file: line range works correctly
- [ ] write_file: creates parent directories
- [ ] write_file: overwrites existing file
- [ ] edit_file: exactly one match required
- [ ] edit_file: zero matches returns error
- [ ] edit_file: multiple matches returns error
- [ ] run_pty_cmd: exit_code 0 for success
- [ ] run_pty_cmd: non-zero exit_code for failure
- [ ] run_pty_cmd: timeout works

### Integration Tests
- [ ] ToolRegistry wraps correctly in Arc<RwLock<>>
- [ ] Concurrent tool execution doesn't deadlock
- [ ] build_function_declarations() returns valid schemas
- [ ] Tool definitions match actual tool implementations

### Session Archive Tests
- [ ] SessionMessage serialization matches vtcode format
- [ ] SessionArchive.finalize() creates valid JSON
- [ ] find_session_by_identifier() works
- [ ] list_recent_sessions() returns correct order
- [ ] Backwards compatible with existing session files

### End-to-End Tests
- [ ] Agentic loop works with local tools
- [ ] Workflow execution works with local tools
- [ ] Sub-agent execution works with local tools
- [ ] Sidecar file tracking works
- [ ] HITL approval works
- [ ] Session save/load round-trip works

---

## Summary

This parallel migration approach ensures:

1. **Zero risk to existing functionality** - vtcode-core continues working
2. **Testable in isolation** - new modules can be validated independently
3. **Gradual rollout** - feature flag controls which implementation is used
4. **Easy rollback** - just disable the feature flag
5. **Clear contracts** - interfaces match exactly for drop-in replacement
