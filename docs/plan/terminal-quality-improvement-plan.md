# Terminal Quality Improvement Plan

Date: January 1, 2026

## Goals

Elevate qbit's terminal UX and feature set to modern standards (Warp, Ghostty, iTerm2) while staying within xterm.js, Tauri, and cross-platform constraints.

## Competitive Analysis (Summary)

### Warp (modern UX + AI-first terminal)

Key differentiators:

- Blocks: Commands and outputs are grouped into discrete blocks with visual state and navigation. Blocks grow bottom-up and encode exit status visually. Source: Warp Block Basics (https://docs.warp.dev/terminal/blocks/block-basics)
- Block actions: per-block copy, share, bookmark. Source: Warp Block Actions (https://docs.warp.dev/terminal/blocks/block-actions)
- Command palette: global search across workflows, prompts, sessions, actions. Source: Warp Command Palette (https://docs.warp.dev/features/command-palette)
- Universal Input: unified command + AI prompt input with auto-detection, contextual chips, IDE-like editing, completions, syntax highlighting. Source: Warp Universal Input (https://docs.warp.dev/terminal/universal-input)
- Agent Mode: natural language tasks with multi-step command execution and control. Source: Warp Agent Mode (https://docs.warp.dev/features/warp-ai/agent-mode)

### Ghostty (native performance + config-first)

Key differentiators:

- GPU-accelerated rendering and native UI with tabs/splits. Source: Ghostty Features (https://ghostty.org/docs/features)
- Unicode/typography fidelity: ligatures, grapheme clustering. Source: Ghostty Features (https://ghostty.org/docs/features)
- Kitty graphics protocol support for inline images. Source: Ghostty Features (https://ghostty.org/docs/features)
- Text-based configuration with many options, hot reload. Source: Ghostty Configuration (https://ghostty.org/docs/config)

### Zed (IDE-integrated terminal with Alacritty backend)

Key differentiators:

- Alacritty backend: Uses Alacritty terminal emulator for high-performance text processing and rendering. Source: Zed Terminal (https://zed.dev/docs/terminal)
- Path hyperlink navigation: Auto-detects file paths, enables Cmd-click navigation with configurable regex patterns. Source: Zed Terminal (https://zed.dev/docs/terminal)
- Task runner integration: Terminal spawns tasks with configurable commands, args, working directories, and env vars. Source: Zed Tasks (https://zed.dev/docs/tasks)
- Split panes: Left, right, up, down splits with docking to editor sides. Source: Zed Terminal (https://zed.dev/docs/terminal)
- Buffer search: Search within terminal scrollback. Source: Zed Terminal (https://zed.dev/docs/terminal)
- Python virtualenv auto-detection: Automatically activates virtual environments. Source: Zed Terminal (https://zed.dev/docs/terminal)
- Copy on select: Automatic clipboard copy with configurable selection retention. Source: Zed Terminal (https://zed.dev/docs/terminal)

### iTerm2 (mature power features)

Key differentiators:

- Shell integration with command markers, cwd, host, history. Source: iTerm2 Shell Integration (https://iterm2.com/documentation-shell-integration.html)
- Triggers: regex-based actions on output. Source: iTerm2 Triggers (https://iterm2.com/triggers.html)
- Dynamic profiles: change profiles by editing files, live reload. Source: iTerm2 Dynamic Profiles (https://iterm2.com/documentation-dynamic-profiles.html)
- Inline images (imgcat / OSC 1337 image protocol). Source: iTerm2 Images (https://iterm2.com/documentation-images.html)
- tmux integration with native UI. Source: iTerm2 tmux Integration (https://iterm2.com/documentation-tmux-integration.html)
- Split panes (menu-level split actions). Source: iTerm2 Menu Items (https://iterm2.com/documentation/2.1/documentation-menu-items.html)

## Modern Terminal Quality: Top 12 Features/UX Expectations

1. Block-based command grouping with rich metadata (exit status, timing, cwd) and block actions.
2. Unified input editor with IDE-like editing, completions, syntax highlighting, and command history search.
3. Command palette or launcher for fast navigation and actions.
4. Tabs and split panes with focus management, drag/drop, and workspace awareness.
5. Fast, GPU-accelerated rendering with low input latency.
6. Search across terminal output/history with scoped filtering (by command, cwd, time).
7. Shell integration for prompt markers, cwd, host, and command boundaries (local + remote).
8. Inline images / graphics protocols for modern CLI tooling.
9. Session management: restore, export, annotate, and share session history.
10. Profiles, themes, typography controls, and per-session overrides.
11. Automation hooks (triggers, notifications, alerts, workflows).
12. Visual polish: crisp type, cursor styles, animations, focus indicators, and status badges.

## Current State Assessment (qbit)

### Frontend (Terminal UX)

- Two render modes per session:
  - `timeline`: UnifiedTimeline + UnifiedInput (block-based view).
  - `fullterm`: xterm.js terminal for raw TUI apps.
  - Source: `frontend/components/PaneContainer/PaneLeaf.tsx`
- xterm.js integration:
  - Addons: FitAddon, WebLinksAddon, WebglAddon.
  - SyncOutputBuffer for DEC 2026 synchronized output.
  - ThemeManager drives terminal theme options.
  - Source: `frontend/components/Terminal/Terminal.tsx`, `frontend/lib/terminal/SyncOutputBuffer.ts`
- Command blocks & parsed output:
  - Uses @xterm/headless + SerializeAddon to render timeline-friendly output.
  - Source: `frontend/lib/terminal/VirtualTerminal.ts`
- Unified input features:
  - Command history with search (Ctrl+R), path completion, slash commands, file commands.
  - Source: `frontend/components/UnifiedInput/UnifiedInput.tsx`, `frontend/hooks/useHistorySearch.ts`
- Tabs and splits:
  - Tab bar; resizable split panes via react-resizable-panels.
  - Source: `frontend/components/TabBar/TabBar.tsx`, `frontend/components/PaneContainer/PaneContainer.tsx`
- Command palette exists (app-level) with workspace search / actions.
  - Source: `frontend/components/CommandPalette/CommandPalette.tsx`

### Backend (PTY + Shell Integration)

- PTY manager emits output events, command block events, cwd, and virtualenv.
  - Source: `backend/crates/qbit-pty/src/manager.rs`
- ANSI parser extracts:
  - OSC 133 prompt markers (prompt start/end, command start/end)
  - OSC 7 cwd changes
  - OSC 1337 virtual env
  - CSI ?1049 alternate screen (fullterm detection)
  - CSI ?2026 synchronized output
  - Source: `backend/crates/qbit-pty/src/parser.rs`
- Shell integration script to emit OSC markers (install/uninstall).
  - Source: `backend/crates/qbit/src/commands/shell.rs`

### Settings / Themability

- Terminal settings include font family, size, scrollback, shell override, and extra fullterm commands.
  - Source: `frontend/lib/settings.ts`, `frontend/components/Settings/TerminalSettings.tsx`

## Feature Gap Analysis (Competitors vs qbit)

Below, each item includes: competitor behavior, qbit status, and feasibility.

### 1) Block-level actions (copy/share/bookmark) and block navigation

- Competitors: Warp block actions (copy/share/bookmark).
- qbit: Timeline blocks exist but no action toolbar or bookmarking.
- Feasibility: High (UI + store metadata).

### 2) Unified input polish (completions, autosuggest, contextual chips)

- Competitors: Warp Universal Input includes auto-detection, completions, contextual chips.
- qbit: Path completion + slash/file commands; no autosuggestions or rich chips.
- Feasibility: Medium (frontend UI; optional backend hints for smart completions).

### 3) Inline images / graphics protocols

- Competitors: iTerm2 inline images (imgcat / OSC 1337) and Ghostty kitty graphics.
- qbit: No inline image support in fullterm or timeline.
- Feasibility: Medium/High with xterm.js image addon + OSC parser. Timeline needs safe embedding.

### 4) Shell integration depth (command markers, cwd, host, SSH)

- Competitors: iTerm2 shell integration provides detailed metadata (prompt markers, cwd, host, history).
- qbit: OSC 133/7/1337 supported; host/SSH metadata is limited.
- Feasibility: Medium (expand OSC handling + store metadata).

### 5) Profiles and configuration system

- Competitors: Ghostty and iTerm2 support robust config/profiles (including dynamic profiles).
- qbit: Global terminal settings only; no per-profile/per-tab overrides.
- Feasibility: Medium (settings schema + session overrides).

### 6) Triggers / automation hooks

- Competitors: iTerm2 triggers based on regex output.
- qbit: No trigger system.
- Feasibility: Medium (frontend + backend event pipeline).

### 7) Session management + export

- Competitors: Warp block sharing; iTerm2 tmux integration for persistence; Ghostty config and native windowing.
- qbit: Session browser for AI sessions; no terminal session export, snapshot, or sharing.
- Feasibility: Medium (store output/logs; add export UI).

### 8) Visual polish (cursor, selection, animations)

- Competitors: Warp and Ghostty have modern typography and UI polish.
- qbit: Basic cursor style + ThemeManager; minimal animations.
- Feasibility: High (frontend theming + CSS).

### 9) Performance tuning + rendering fidelity

- Competitors: Ghostty GPU rendering; Warp performance + smooth block UI.
- qbit: WebGL addon enabled; no perf instrumentation.
- Feasibility: Medium (telemetry, render tuning, optional canvas fallback).

## Implementation Plan

### Prioritized Feature List (Impact x Feasibility)

Legend: Effort = S (small), M (medium), L (large)

| Priority | Feature | Impact | Effort | Notes |
| --- | --- | --- | --- | --- |
| P0 | Block actions + bookmarks | High | S/M | Copy output, share, rerun, bookmark, navigate |
| P1 | Output metadata + status badges | High | M | Exit status, duration, cwd, process, host |
| P2 | Unified input improvements | High | M | Autosuggest, hinting, command preview, chips |
| P3 | Session export + share | Medium | M | Export timeline + fullterm scrollback |
| P4 | Inline images support | Medium | L | Implement iTerm2 + Kitty protocols via xterm addon |
| P5 | Profiles & per-tab settings | Medium | M | Dynamic settings per session |
| P6 | Triggers + notifications | Medium | M | Regex actions + OS notifications |
| P7 | Visual polish | Medium | S/M | Cursor, selection, focus, animations |
| P8 | Performance instrumentation | Medium | M | FPS/latency counters + logging |

### Technical Approach (per feature)

#### P0: Block actions + bookmarks

- Add block toolbar in `UnifiedBlock` with actions: copy command, copy output, copy both, bookmark, share.
- Store bookmarks + pin list in session state; add jump-to-bookmark UI (left rail).
- Architecture: extend `UnifiedBlock` data with `bookmarked` and `share_id` fields.
- Dependencies: none.
- Effort: S/M.

#### P1: Output metadata + status badges

- Extend command block data model: timestamps, duration, cwd, host, exit status.
- Capture timestamps at `command_start` and `command_end` in `useTauriEvents`.
- Backend: emit `host` and `user` via shell integration or OSC; store in session state.
- UI: show badges in block header + status bar; allow filtering by exit status.
- Dependencies: none.
- Effort: M.

#### P2: Unified input improvements

- Add autosuggestions from history + filesystem (ghosted text).
- Add inline chips for cwd, git branch, virtualenv (already available in state).
- Add lightweight syntax highlighting of command tokens (client-side lexer).
- Add per-mode indicators (terminal vs agent) aligned with Warp's universal input patterns.
- Dependencies: optional `@codemirror` or lightweight tokenizer.
- Effort: M.

#### P3: Session export + share

- Persist terminal output per command into a log file (per session) with timestamps and metadata.
- Add export to markdown / JSON (block metadata + output).
- Optional share link generation (local file) to open in a viewer.
- Dependencies: none.
- Effort: M.

#### P4: Inline images support

- Add `@xterm/addon-image` (or equivalent) in fullterm mode.
- Support iTerm2 OSC 1337 inline images and Kitty graphics protocol parsing.
- For timeline, parse image escape sequences into placeholder blocks and allow click-to-open.
- Security: enforce size limits and disable remote file fetch.
- Dependencies: `@xterm/addon-image` (or custom parser), optional Rust decoder crate.
- Effort: L.

#### P5: Profiles & per-tab settings

- Add profiles to settings schema; allow per-tab overrides (font, theme, scrollback, env).
- Persist profiles in settings file; add UI to manage.
- Dependencies: none.
- Effort: M.

#### P6: Triggers + notifications

- Add trigger definitions (regex + action) to settings.
- Evaluate triggers on output (backend or frontend) and dispatch actions: notify, highlight, open URL, run command.
- Dependencies: optional `notify-rust` or Tauri notification API.
- Effort: M.

#### P7: Visual polish

- Cursor styles (block, bar, underline), animation for focus/blur, selection glow.
- Better block grouping with subtle separators and exit-state colors.
- Add soft transitions for new blocks and scrolling.
- Dependencies: none.
- Effort: S/M.

#### P8: Performance instrumentation

- Add frame timing and write/paint metrics (xterm write latency, render time) to a dev overlay.
- Track dropped frames and large writes; throttle output for UI stability.
- Dependencies: none.
- Effort: M.

### Architecture Changes

1. **Terminal Data Model**: Extend session state to store block metadata (timestamps, cwd, host, exit status, duration) and bookmarks.
2. **Session Logs**: Add backend log sink for terminal output with timestamps for export and history.
3. **Profile System**: Add profile definitions to settings; allow per-tab/per-session overrides.
4. **Trigger Engine**: Add output processing pipeline (frontend or backend) with regex matchers.

### Dependency Additions

Frontend:

- `@xterm/addon-image` (or alternative image addon)
- Optional: `@xterm/addon-ligatures`, `@xterm/addon-unicode11` for typography fidelity

Backend:

- Optional image decoder or protocol parser crates if xterm addon is insufficient

### Effort Breakdown (Relative)

- Small: P0, P7
- Medium: P1, P2, P3, P5, P6, P8
- Large: P4

## Risks & Constraints

- xterm.js rendering performance can bottleneck on massive output; avoid heavy DOM overlays.
- Inline image protocols differ (iTerm2 OSC 1337 vs Kitty); need careful parsing and size limits.
- Shell integration metadata is only as good as the shell scripts; ensure robust install/diagnostics.
- Cross-platform (macOS/Windows/Linux) input handling must be validated for keybindings and clipboard behaviors.

## Suggested Phasing

Phase 1: P0, P1, P7
Phase 2: P2, P3, P5
Phase 3: P6, P8
Phase 4: P4

## Open Questions

- Where should trigger evaluation live (frontend vs backend) for cross-platform reliability?
- Do we want to support Kitty graphics or only iTerm2 OSC 1337 initially?
- How should session logs be stored (per-session file size limits, retention policy)?

## Sources

- Zed Terminal: https://zed.dev/docs/terminal
- Zed Tasks: https://zed.dev/docs/tasks
- Warp Block Basics: https://docs.warp.dev/terminal/blocks/block-basics
- Warp Block Actions: https://docs.warp.dev/terminal/blocks/block-actions
- Warp Command Palette: https://docs.warp.dev/features/command-palette
- Warp Universal Input: https://docs.warp.dev/terminal/universal-input
- Warp Agent Mode: https://docs.warp.dev/features/warp-ai/agent-mode
- Ghostty Features: https://ghostty.org/docs/features
- Ghostty Configuration: https://ghostty.org/docs/config
- iTerm2 Shell Integration: https://iterm2.com/documentation-shell-integration.html
- iTerm2 Triggers: https://iterm2.com/triggers.html
- iTerm2 Dynamic Profiles: https://iterm2.com/documentation-dynamic-profiles.html
- iTerm2 Images: https://iterm2.com/documentation-images.html
- iTerm2 tmux Integration: https://iterm2.com/documentation-tmux-integration.html
- iTerm2 Split Panes (menu): https://iterm2.com/documentation/2.1/documentation-menu-items.html
