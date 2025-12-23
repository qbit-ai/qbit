# Terminal Parsing and Event System

This document explains how Qbit parses terminal output to enable features like the timeline view, directory sync, and automatic fullterm mode switching.

## Overview

Qbit uses a VTE (Virtual Terminal Emulator) parser to extract semantic information from terminal escape sequences. This enables:

- **Timeline View**: Parsed command blocks with input, output, and exit codes
- **Directory Sync**: Automatic workspace updates when you `cd`
- **Fullterm Mode**: Auto-switching for TUI applications (vim, htop, etc.)

## Architecture

```
Terminal Output (raw bytes)
        │
        ▼
┌─────────────────────────────────┐
│  VTE Parser (backend/src/pty/parser.rs)  │
│  - Extracts OSC sequences       │
│  - Extracts CSI sequences       │
└─────────────────────────────────┘
        │
        ▼
┌─────────────────────────────────┐
│  OscEvent enum                  │
│  - PromptStart/End              │
│  - CommandStart/End             │
│  - DirectoryChanged             │
│  - AlternateScreenEnabled/Disabled │
└─────────────────────────────────┘
        │
        ▼
┌─────────────────────────────────┐
│  Tauri Events                   │
│  - command_block                │
│  - directory_changed            │
│  - alternate_screen             │
└─────────────────────────────────┘
        │
        ▼
┌─────────────────────────────────┐
│  Frontend (useTauriEvents.ts)   │
│  - Updates Zustand store        │
│  - Manages render mode          │
└─────────────────────────────────┘
```

## Escape Sequences

### OSC 133 - Shell Integration (Command Lifecycle)

OSC 133 is part of the [Shell Integration](https://gitlab.freedesktop.org/Per_Bothner/specifications/blob/master/proposals/semantic-prompts.md) specification. Modern shells (zsh, bash, fish) emit these sequences to mark prompt and command boundaries.

| Sequence | Event | Description |
|----------|-------|-------------|
| `ESC]133;A BEL` | `PromptStart` | Shell is about to display the prompt |
| `ESC]133;B BEL` | `PromptEnd` | Prompt displayed, cursor at user input position |
| `ESC]133;C;cmd BEL` | `CommandStart` | User pressed Enter, command `cmd` is executing |
| `ESC]133;D;N BEL` | `CommandEnd` | Command finished with exit code `N` |

**Example lifecycle:**
```
[PromptStart] → [PromptEnd] → user types "ls -la" → [CommandStart;ls -la] → output... → [CommandEnd;0]
```

**Used for:**
- Parsing command blocks in timeline view
- Tracking exit codes (success/failure highlighting)
- Knowing when to capture command output

**Shell configuration:**
- **zsh**: Usually enabled by default with modern versions, or via `autoload -Uz add-zsh-hook`
- **bash**: Requires manual setup or tools like starship
- **fish**: Built-in support

### OSC 7 - Current Working Directory

OSC 7 reports the current working directory when it changes.

| Sequence | Event | Description |
|----------|-------|-------------|
| `ESC]7;file://hostname/path BEL` | `DirectoryChanged` | CWD changed to `/path` |

**Used for:**
- Syncing the AI agent's workspace when you `cd`
- Updating the UI to show current directory

**Note:** The path is URL-encoded (spaces become `%20`). The parser handles decoding.

### CSI ? 1049/47/1047 - Alternate Screen Buffer

These DEC private mode sequences control the alternate screen buffer, used by full-screen TUI applications.

| Sequence | Event | Description |
|----------|-------|-------------|
| `ESC[?1049h` | `AlternateScreenEnabled` | App entered alternate screen (vim, htop, etc.) |
| `ESC[?1049l` | `AlternateScreenDisabled` | App exited alternate screen |
| `ESC[?47h/l` | Same | Legacy alternate screen |
| `ESC[?1047h/l` | Same | Alternate screen without cursor save |

**Used for:**
- Auto-switching to fullterm mode when TUI apps start
- Auto-switching back to timeline mode when they exit

**Why multiple codes?**
- `1049` - Modern xterm-style, saves/restores cursor position
- `47` - Original DT alternate screen
- `1047` - Alternate screen without cursor save/restore

Most modern apps use `1049`, but the parser handles all three for compatibility.

## Fullterm Mode Detection

Qbit uses a layered approach to detect when fullterm mode is needed:

### Primary: ANSI Sequence Detection

The VTE parser detects alternate screen buffer sequences. This handles ~95% of TUI apps automatically:
- vim, neovim, nano
- htop, top, btop
- less, more
- tmux, screen
- fzf, lazygit

### Fallback: Command Name Matching

Some apps use raw terminal mode but don't use the alternate screen buffer (they want output to persist in terminal history). These are detected by command name:

**Built-in list:**
```typescript
const BUILTIN_FULLTERM_COMMANDS = [
  "claude", "cc", "codex", "cdx", "aider", "cursor", "gemini"
];
```

**User-configurable:** Add more in `~/.qbit/settings.toml`:
```toml
[terminal]
fullterm_commands = ["my-tui-app", "custom-tool"]
```

### Mode Switching Flow

```
Command Start
     │
     ├─► Check command name against fullterm_commands list
     │   └─► Match? → Switch to fullterm mode
     │
     ▼
Terminal Output
     │
     ├─► VTE parser detects ESC[?1049h
     │   └─► Switch to fullterm mode
     │
     ├─► VTE parser detects ESC[?1049l
     │   └─► Switch to timeline mode
     │
     ▼
Command End (OSC 133;D)
     │
     └─► Fallback: Switch to timeline mode
         (catches apps that crash without sending disable sequence)
```

## Frontend Event Handling

The `useTauriEvents.ts` hook subscribes to Tauri events and updates the Zustand store:

| Tauri Event | Handler | Store Update |
|-------------|---------|--------------|
| `command_block` | `handlePromptStart/End`, `handleCommandStart/End` | Command blocks, exit codes |
| `directory_changed` | `updateWorkingDirectory` | Session CWD, AI workspace |
| `alternate_screen` | `setRenderMode` | Timeline ↔ Fullterm |
| `terminal_output` | `appendOutput` | Raw output capture |

## Troubleshooting

### Timeline view not showing command blocks

1. **Check shell integration is enabled:**
   ```bash
   # In your shell, run:
   echo -e '\e]133;A\a'  # Should produce no visible output
   ```

2. **Verify OSC 133 sequences are being emitted:**
   - zsh: Check `~/.zshrc` for shell integration setup
   - bash: May need explicit configuration
   - fish: Should work out of the box

### Fullterm mode not activating for a TUI app

1. **Check if app uses alternate screen:**
   - Run the app in a normal terminal
   - Does scrollback history disappear while app is running? → Uses alternate screen
   - Does output remain in scrollback? → Doesn't use alternate screen

2. **Add to fallback list:**
   ```toml
   # ~/.qbit/settings.toml
   [terminal]
   fullterm_commands = ["your-app-name"]
   ```

### Directory sync not working

1. **Check OSC 7 is being emitted:**
   ```bash
   # After cd'ing, this should show the OSC 7 sequence:
   cd /tmp && printf '%s' "$TERM_PROGRAM"
   ```

2. **Verify shell emits OSC 7:**
   - zsh: Usually automatic with `PROMPT_SUBST`
   - bash: May need `PROMPT_COMMAND` setup
   - fish: Built-in via `fish_title`

## Implementation Details

### Parser Location
- **Rust**: `backend/src/pty/parser.rs` - VTE parser with `OscEvent` enum
- **Frontend**: `frontend/hooks/useTauriEvents.ts` - Event subscription and store updates

### Event Emission
- **PTY Manager**: `backend/src/pty/manager.rs` - Emits parsed events via Tauri
- **Events**: `command_block`, `directory_changed`, `alternate_screen`, `terminal_output`

### State Management
- **Store**: `frontend/store/index.ts` - Zustand store with `RenderMode` type
- **Modes**: `"timeline"` (default) | `"fullterm"` (TUI apps)
