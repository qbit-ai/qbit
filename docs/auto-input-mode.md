# Auto Input Mode

Auto input mode intelligently classifies user input as either a terminal command or an AI agent prompt, removing the need to manually switch between modes. It uses a locally-built command index and heuristic rules to route each submission to the correct handler.

## Usage

1. Press **Cmd+I** to cycle input modes: Terminal → Agent → **Auto** → Terminal
2. Alternatively, click a mode button in the input status row — clicking the active mode button advances to Auto
3. Type your input and press **Enter**
4. Qbit classifies your input and routes it:
   - Commands like `ls -la` or `git status` go to the terminal
   - Natural language like `explain this code` goes to the AI agent

### Mode Cycling

| Current Mode | Cmd+I / Toggle | Result |
|--------------|----------------|--------|
| Terminal | → | Agent |
| Agent | → | Auto |
| Auto | → | Terminal |

In Auto mode, all agent features (vision, image paste, `@`-file references, drag-and-drop) are available since the mode may route to the agent.

## How It Works

### Classification Flow

```
User submits input in Auto mode
            │
            ▼
┌───────────────────────────────────┐
│   classifyInput(input) → Tauri    │
│   invokes classify_input command  │
└───────────────┬───────────────────┘
                │
                ▼
┌───────────────────────────────────┐
│         CommandIndex.classify()   │
│                                   │
│  1. Path prefix? (./  /  ~/)      │
│     → Terminal                    │
│                                   │
│  2. Shell operators? (| > && ;)   │
│     → Terminal                    │
│                                   │
│  3. First token is known command? │
│     ├─ Has flags (-x, --foo)      │
│     │  → Terminal                 │
│     ├─ 1-2 tokens                 │
│     │  → Terminal                 │
│     ├─ 3+ plain English words     │
│     │  → Agent                    │
│     └─ Otherwise                  │
│        → Terminal                 │
│                                   │
│  4. Unknown first token           │
│     → Agent                       │
└───────────────┬───────────────────┘
                │
                ▼
    Route to Terminal or Agent
```

### Classification Examples

| Input | Route | Reason |
|-------|-------|--------|
| `ls` | Terminal | Known command, single token |
| `git status` | Terminal | Known command, 2 tokens |
| `ls -la` | Terminal | Known command with flags |
| `cat foo \| grep bar` | Terminal | Shell operator (pipe) |
| `./script.sh` | Terminal | Path prefix |
| `~/bin/run.sh` | Terminal | Path prefix |
| `echo hello > file.txt` | Terminal | Shell operator (redirect) |
| `make sure the tests pass` | Agent | Known command but 3+ plain English words |
| `find all the bugs` | Agent | Known command but 3+ plain English words |
| `explain this code` | Agent | Unknown first token |
| `what files are here` | Agent | Unknown first token |

### Command Index

At startup, the backend builds an index of all executable commands available to the user:

1. **PATH scan**: Reads every directory in `$PATH` and collects names of executable files
2. **Shell builtins**: Adds builtins for the detected shell type (`$SHELL` → zsh, bash, or fish)

The index is built once in a background thread via `tauri::async_runtime::spawn_blocking` and stored in `AppState`. Building typically indexes several thousand commands.

### Shell Operator Detection

The classifier recognizes shell operators outside of quoted strings:

- `|` — pipe
- `>` / `<` — redirect
- `&&` — logical AND
- `;` — command separator

Operators inside single or double quotes are ignored, so `echo "hello | world"` is not misclassified as a pipe.

## Architecture

```
┌──────────────────────────────────────────────────────────────────┐
│                    UnifiedInput.tsx                              │
│  - On submit in auto mode, calls classifyInput(input)            │
│  - Routes to terminal (ptyWrite) or agent (sendAiMessage)        │
│  - Falls back to terminal on classification error                │
└──────────────────────────┬───────────────────────────────────────┘
                           │
                           ▼
┌──────────────────────────────────────────────────────────────────┐
│                      lib/tauri.ts                                │
│  - classifyInput(input): Promise<ClassifyResult>                 │
│  - ClassifyResult = { route, detected_command }                  │
└──────────────────────────┬───────────────────────────────────────┘
                           │ invoke("classify_input", { input })
                           ▼
┌──────────────────────────────────────────────────────────────────┐
│               commands/command_index.rs (Rust)                   │
│  - CommandIndex struct (HashSet<String> + RwLock)                │
│  - build(): scans PATH + shell builtins                          │
│  - classify(input): applies heuristic rules                      │
│  - classify_input Tauri command                                  │
└──────────────────────────────────────────────────────────────────┘
```

## API

### Frontend (TypeScript)

```typescript
import { classifyInput, type ClassifyResult } from "@/lib/tauri";

// Classify user input
const result: ClassifyResult = await classifyInput("git status");
// { route: "terminal", detected_command: "git" }

const result2: ClassifyResult = await classifyInput("explain this code");
// { route: "agent", detected_command: null }
```

```typescript
// Store types
import type { InputMode } from "@/store";

// InputMode = "terminal" | "agent" | "auto"
```

### Backend (Rust)

```rust
use crate::commands::CommandIndex;

// Create and build the index
let index = CommandIndex::new();
index.build(); // scans PATH + shell builtins

// Classify input
let result = index.classify("ls -la");
assert_eq!(result.route, "terminal");
assert_eq!(result.detected_command, Some("ls".to_string()));
```

```rust
// ClassifyResult
pub struct ClassifyResult {
    pub route: String,              // "terminal" or "agent"
    pub detected_command: Option<String>, // first token if it's a known command
}
```

## Files

| File | Purpose |
|------|---------|
| `backend/crates/qbit/src/commands/command_index.rs` | `CommandIndex`, `ClassifyResult`, classification heuristics, Tauri command, unit tests |
| `backend/crates/qbit/src/commands/mod.rs` | Module declaration and re-export |
| `backend/crates/qbit/src/state.rs` | `command_index` field on `AppState` |
| `backend/crates/qbit/src/lib.rs` | Background index build at startup, `classify_input` command registration |
| `frontend/lib/tauri.ts` | `classifyInput()` typed wrapper, `ClassifyResult` interface |
| `frontend/store/index.ts` | `InputMode` type extended with `"auto"` |
| `frontend/store/selectors/unified-input.ts` | `inputMode` selector type updated |
| `frontend/components/UnifiedInput/UnifiedInput.tsx` | Auto mode submit logic, three-state toggle, agent feature enablement |
| `frontend/components/UnifiedInput/InputStatusRow.tsx` | Mode button cycling for three states |
| `frontend/App.tsx` | `handleToggleMode` updated for three-state cycle |
| `frontend/mocks.ts` | Mock response for `classify_input` in browser-only mode |

## Testing

### Rust Unit Tests

```bash
cargo test -p qbit --lib command_index
```

Tests cover:
- Path prefixes route to terminal (`./`, `/`, `~/`)
- Shell operators route to terminal (`|`, `>`, `&&`, `;`)
- Known commands with flags route to terminal
- Single/two-token known commands route to terminal
- Natural language starting with a command name routes to agent
- Unknown first tokens route to agent
- Empty input routes to agent
- Shell operators inside quotes are ignored
- `detected_command` is populated correctly

### Frontend Tests

```bash
just test-fe
```

The `UnifiedInput.callbacks.test.tsx` suite verifies the Cmd+I toggle cycles through all three modes correctly.

## Browser Development Mode

In browser-only mode (without Tauri backend), the mock system returns a default classification. See `frontend/mocks.ts` under `case "classify_input"` — it returns `{ route: "terminal", detected_command: null }`.

## Related Documentation

- [Agent modes](agent-modes.md) — controls tool approval behavior (default, auto-approve, planning)
- [Tab completion](tab-completion.md) — path completion in terminal mode
- [Tool use](tool-use.md) — agent tool execution
