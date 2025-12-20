# Tab Completion

Tab completion provides filesystem path autocompletion in terminal mode, similar to shell tab completion.

## Usage

1. Switch to **Terminal mode** (click the Terminal button or press `Cmd+I`)
2. Start typing a path or command with a path argument
3. Press **Tab** to open the completion popup
4. Navigate with **Arrow Up/Down**, select with **Tab** or **Enter**
5. Press **Escape** to dismiss without selecting

### Keyboard Shortcuts

| Key | Action |
|-----|--------|
| `Tab` | Open popup / Select current item |
| `Enter` | Select current item |
| `Arrow Down` | Move selection down |
| `Arrow Up` | Move selection up |
| `Escape` | Close popup |
| Any typing | Close popup (reopen with Tab) |

### Directory Continuation

When you select a directory (ending with `/`), the popup automatically reopens to show the directory's contents, enabling quick navigation through the filesystem.

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                     UnifiedInput.tsx                         │
│  - Tab key handler triggers completion                       │
│  - Manages popup state (showPathPopup, pathSelectedIndex)    │
│  - Extracts word at cursor for partial path                  │
└─────────────────────────┬───────────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────────┐
│                  usePathCompletion.ts                        │
│  - React hook for fetching completions                       │
│  - Handles loading state and cancellation                    │
│  - Calls listPathCompletions() from lib/tauri.ts             │
└─────────────────────────┬───────────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────────┐
│                     lib/tauri.ts                             │
│  - listPathCompletions(sessionId, partialPath, limit)        │
│  - TypeScript interface for Tauri IPC                        │
└─────────────────────────┬───────────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────────┐
│              commands/completions.rs (Rust)                  │
│  - list_path_completions Tauri command                       │
│  - Path expansion (tilde, relative paths)                    │
│  - Directory reading and filtering                           │
│  - Case-insensitive matching on macOS                        │
└─────────────────────────────────────────────────────────────┘
```

## Components

### Frontend

#### `frontend/components/PathCompletionPopup/PathCompletionPopup.tsx`

Renders the completion popup using Radix UI Popover. Features:
- Icons for files, directories, and symlinks
- Keyboard-accessible list items
- Auto-scroll to selected item
- "No completions found" empty state

#### `frontend/hooks/usePathCompletion.ts`

React hook that manages completion fetching:
- Debounces requests on input changes
- Cancels in-flight requests when inputs change
- Clears completions when disabled

#### `frontend/components/UnifiedInput/UnifiedInput.tsx`

Integration point for tab completion:
- Intercepts Tab key in terminal mode
- Extracts the word at cursor position
- Manages popup visibility and selection state
- Handles directory continuation

### Backend

#### `backend/src/commands/completions.rs`

Rust implementation of path completion:

```rust
#[tauri::command]
pub async fn list_path_completions(
    session_id: String,
    partial_path: String,
    limit: Option<usize>,
) -> Result<Vec<PathCompletion>, String>
```

**Features:**
- Tilde (`~`) expansion to home directory
- Relative path resolution
- Hidden file filtering (only shown when prefix starts with `.`)
- Case-insensitive prefix matching on macOS
- Sorted results: directories first, then alphabetically

**Types:**

```rust
pub enum PathEntryType {
    File,
    Directory,
    Symlink,
}

pub struct PathCompletion {
    pub name: String,        // Display name (e.g., "frontend/")
    pub insert_text: String, // Text to insert (e.g., "frontend/")
    pub entry_type: PathEntryType,
}
```

## Configuration

Tab completion is enabled by default in terminal mode. There are no user-configurable options.

## Browser Development Mode

In browser-only mode (without Tauri backend), the mock system provides simulated completions. See `frontend/mocks.ts` for the mock implementation under `case "list_path_completions"`.

## Testing

### Unit Tests

```bash
# Frontend hook tests
pnpm test -- usePathCompletion

# Rust unit tests
cargo test -p qbit --lib completions
```

### Property-Based Tests

The Rust implementation includes proptest-based property tests:
- Tilde expansion produces valid paths
- Empty prefix returns sorted results
- Hidden files only appear with dot prefix
- Completion names match directory entries

### E2E Tests

```bash
pnpm exec playwright test e2e/tab-completion.spec.ts
```

Tests cover:
- Popup triggering (Tab key, terminal mode only)
- Keyboard navigation (Arrow keys, boundaries)
- Selection (Tab, Enter, Click)
- Dismissal (Escape, typing)
- Directory continuation
- Visual feedback (icons, highlighting)

### Evaluation Tests

```bash
cd evals
source .venv/bin/activate
pytest test_path_completion.py -v
```

Tests verify agent behavior with:
- Directory listing
- Hidden files
- Nested paths
- Tilde expansion
- Relative paths
- Edge cases (empty dirs, symlinks, non-existent paths)

## Troubleshooting

### Popup doesn't appear

1. Ensure you're in **Terminal mode** (not Agent mode)
2. Check that the textarea has focus
3. Verify the session is initialized

### No completions shown

1. The path may not exist
2. Hidden files require a `.` prefix
3. Check browser console for errors

### Completions are slow

1. Large directories may take longer to read
2. Network latency in development mode
3. Check for filesystem permission issues
