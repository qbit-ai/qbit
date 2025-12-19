# Agent Modes

Agent modes allow users to control how the AI agent handles tool approvals and what operations it can perform. This feature provides presets that modify agent behavior for different use cases.

## Available Modes

### Default Mode
**Icon**: Shield
**Behavior**: Normal HITL (Human-in-the-Loop) operation

In default mode, tool approval follows the standard policy:
- Read-only tools (`read_file`, `grep_file`, `list_files`, etc.) are auto-approved
- Write operations (`edit_file`, `write_file`, `apply_patch`) require user approval
- Learned patterns can auto-approve frequently-used tools
- The `always_allow` list bypasses approval for specific tools

**Use cases**:
- Normal development work
- When you want oversight of file modifications
- First-time operations in unfamiliar codebases

### Auto-Approve Mode
**Icon**: Zap (lightning bolt)
**Behavior**: All tool calls automatically approved

In auto-approve mode:
- All tools execute without prompting for approval
- The system prompt notifies the AI that operations are auto-approved
- Policy deny rules still apply (tools explicitly denied are still blocked)

**Use cases**:
- Batch operations where you trust the AI's judgment
- Repetitive tasks you've done before
- When you want uninterrupted execution
- Experienced users working on familiar codebases

**Caution**: Use carefully. The AI can modify files without confirmation.

### Planning Mode
**Icon**: Eye
**Behavior**: Read-only operations only

In planning mode:
- Only read-only tools are allowed:
  - `read_file`, `grep_file`, `list_files`, `list_directory`, `find_files`
  - `indexer_*` tools (code analysis)
  - `web_search`, `web_fetch` (research)
  - `update_plan` (planning)
- Write operations are denied with a clear error message
- The system prompt instructs the AI to focus on analysis and planning
- If asked to make changes, the AI explains it's in planning mode and offers to create a plan instead

**Use cases**:
- Exploring unfamiliar codebases
- Understanding code before making changes
- Creating implementation plans for review
- Safe exploration without risk of modifications
- Code reviews and analysis

## How It Works

### Frontend
The agent mode selector appears in the status bar (footer) next to the model selector when:
- The AI is initialized (`status === "ready"`)
- The input mode is set to "agent" (not terminal)

Selecting a mode:
1. Updates the frontend Zustand store
2. Calls the `set_agent_mode` Tauri command
3. Shows a notification confirming the change

### Backend

#### Tool Approval Flow
The agent mode is checked during tool execution in `agentic_loop.rs`:

```
Step 0: Planning mode check
        → If planning mode AND tool not in ALLOW_TOOLS → DENY

Step 1: Policy deny check
        → If tool explicitly denied → DENY

Step 2: Apply constraints
        → Modify args if needed, or deny on violation

Step 3: Check if tool allowed by policy
        → If policy = Allow → AUTO-APPROVE

Step 4: Check learned patterns
        → If pattern qualifies → AUTO-APPROVE

Step 4.4: Check agent mode (auto-approve)
        → If auto-approve mode → AUTO-APPROVE

Step 4.5: Check runtime --auto-approve flag
        → If CLI flag set → AUTO-APPROVE

Step 5: Request HITL approval
        → Show approval dialog to user
```

#### System Prompt
The system prompt is dynamically modified based on agent mode:

- **Default**: No additional instructions
- **Auto-Approve**: Brief note that operations are auto-approved
- **Planning**: Detailed instructions about read-only restrictions, allowed/forbidden operations, and how to respond to modification requests

## API

### Frontend (TypeScript)

```typescript
// Store
import { useAgentMode, useStore, type AgentMode } from "@/store";

// Get current mode for a session
const agentMode = useAgentMode(sessionId); // "default" | "auto-approve" | "planning"

// Set mode
const setAgentMode = useStore((state) => state.setAgentMode);
setAgentMode(sessionId, "planning");

// Tauri commands
import { setAgentMode, getAgentMode } from "@/lib/ai";

await setAgentMode(sessionId, "auto-approve");
const mode = await getAgentMode(sessionId);
```

### Backend (Rust)

```rust
use crate::ai::agent_mode::AgentMode;

// On AgentBridge
bridge.set_agent_mode(AgentMode::Planning).await;
let mode = bridge.get_agent_mode().await;

// Check mode
if mode.is_planning() { /* read-only */ }
if mode.is_auto_approve() { /* all approved */ }
if mode.is_default() { /* normal HITL */ }
```

## Files

| File | Purpose |
|------|---------|
| `src/store/index.ts` | `AgentMode` type, `agentMode` state, `setAgentMode` action |
| `src/components/AgentModeSelector/` | Dropdown UI component |
| `src/components/StatusBar/StatusBar.tsx` | Integration point in footer |
| `src/lib/ai.ts` | Tauri command wrappers |
| `src-tauri/src/ai/agent_mode.rs` | `AgentMode` enum definition |
| `src-tauri/src/ai/commands/mode.rs` | Tauri commands |
| `src-tauri/src/ai/agentic_loop.rs` | Enforcement logic |
| `src-tauri/src/ai/system_prompt.rs` | System prompt modifications |
| `src-tauri/src/ai/tool_policy.rs` | `ALLOW_TOOLS` constant (read-only tools) |

## Read-Only Tools (Planning Mode)

The following tools are allowed in planning mode (defined in `ALLOW_TOOLS` in `tool_policy.rs`):

- `read_file` - Read file contents
- `grep_file` - Search within files
- `list_files` - List files matching pattern
- `indexer_search_code` - Semantic code search
- `indexer_search_files` - Find files by name
- `indexer_analyze_file` - Analyze file structure
- `indexer_extract_symbols` - Get symbols from file
- `indexer_get_metrics` - Get code metrics
- `indexer_detect_language` - Detect file language
- `debug_agent` - Agent debugging info
- `analyze_agent` - Analyze agent behavior
- `get_errors` - Get error information
- `update_plan` - Update the execution plan
- `list_skills` - List available skills
- `search_skills` - Search for skills
- `load_skill` - Load a skill
- `search_tools` - Search available tools

## update_plan Tool

The `update_plan` tool allows the agent to create and maintain a task plan with multiple steps.

### Usage

```json
{
  "explanation": "Optional high-level plan summary",
  "plan": [
    { "step": "Read the config file", "status": "completed" },
    { "step": "Analyze dependencies", "status": "in_progress" },
    { "step": "Propose changes", "status": "pending" }
  ]
}
```

### Constraints

- **Step count**: 1-12 steps allowed
- **In-progress limit**: Only one step can be `in_progress` at a time
- **Empty steps**: Step descriptions cannot be empty

### Step Status

- `pending` - Not started yet
- `in_progress` - Currently being worked on
- `completed` - Finished

### Implementation Files

| File | Purpose |
|------|---------|
| `src-tauri/src/tools/planner/mod.rs` | `PlanManager`, `TaskPlan`, `PlanStep` structs |
| `src-tauri/src/tools/definitions.rs` | Tool definition |
| `src-tauri/src/ai/tool_executors.rs` | `execute_plan_tool` function |
| `src-tauri/src/ai/events.rs` | `PlanUpdated` event for UI notifications |
