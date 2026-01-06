# Sidebar File Editor Plan

Goal: Add a right-hand sidebar (like the Task Plan panel) dedicated to viewing and editing files with syntax highlighting and Vim support. Use CodeMirror (@uiw/react-codemirror) and draw UX inspiration from VS Code, Zed, and GitKraken.

## 1) UX and interaction outline
1. Add a right-side panel that can be toggled (keyboard shortcut + command palette) and coexists with existing Context/Task panels (mutually exclusive to avoid clutter).
2. Panel chrome: header with file breadcrumb, file status (dirty/RO), actions (save, revert, open in pane, close), and a search/jump box.
3. Body layout: top mini-toolbar (go to file path, toggle Vim, toggle wrap, language select if unknown), main editor area, bottom metadata strip (encoding/line/col, git status, diagnostics summary).
4. Resizing: draggable handle on left edge (match TaskPlannerPanel behavior); remember width per session.
5. Empty state: prompt to pick a file from file tree/command palette; show recent files list.
6. Keyboard: Cmd/Ctrl+S save, Cmd/Ctrl+P opens file finder, Esc exits insert when Vim enabled; show Vim mode indicator.
7. Error states: read/write errors surface inline banners; external modifications prompt reload.

## 2) Data model and state
1. Store slice (Zustand): `fileEditorSidebar` with `{open, width, activeFilePath, openFiles: [{path, content, language, dirty, lastSavedAt}], vimModeEnabled, wrapEnabled}` plus per-session association to working directory.
2. Derive language from extension; fall back to plaintext with manual selector.
3. Track git status for active file (via existing git/indexer helpers) to show dirty/clean, and detect if file is new/deleted.
4. Persist width/open state per session; reset when session/workingDirectory changes.

## 3) File operations and services
1. Reading: use existing tauri FS command (or add) to load file contents with encoding handling (assume UTF-8 first pass).
2. Writing: add save command with safe write (temp file + atomic replace if available); update git status post-save.
3. External change detection: compare mtime/hash before save; on mismatch prompt user to reload or overwrite.
4. Quick open: reuse file search/indexer utilities to resolve paths (absolute, relative to workingDirectory) with completion.

## 4) Editor implementation (CodeMirror via @uiw/react-codemirror)
1. Add dependency `@uiw/react-codemirror` plus language packs used (common TS/JS/TSX/JSX/JSON/MD/Rust/Go/etc.) and theme that matches app (light/dark aware); add `@uiw/codemirror-extensions-vim` for Vim mode.
2. Configure extensions: basic setup, language per file, Vim toggle, highlight-active-line, line numbers, indent guides, soft wrap toggle, search panel bindings, bracket matching, whitespace render (configurable).
3. Keyboard map: include standard keymap + Vim (conditional), with Cmd/Ctrl+S to invoke save handler and Cmd/Ctrl+P to open file search.
4. Expose callbacks: onChange sets dirty state; onSelectionChange updates status bar line/col; onFocus ensures panel is considered "active editor" for commands.

## 5) Component structure
1. `FileEditorSidebarPanel` (new): resizable right panel wrapper (reuse TaskPlannerPanel patterns) that renders header + editor body; respects `open` prop.
2. `FileEditorHeader`: breadcrumb/path display, git status pill, actions (Save, Revert, Open in Pane, Close), Vim toggle, wrap toggle, language dropdown (when unknown), kebab menu for more.
3. `FileEditorTabs` (optional MVP: single active file; stretch goal: recent list). For MVP keep single file with recent list in empty state.
4. `FileEditorContent`: renders CodeMirror instance, status bar (mode, line/col, encoding, EOL), diagnostics summary placeholder.
5. `FileOpenPrompt`: quick file path input with completion + recent files list when no file loaded.
6. Hook `useFileEditorSidebar` to encapsulate loading/saving logic, state wiring, and command palette integration.

## 6) App integration
1. Add right-panel slot to `App.tsx` alongside ContextPanel/TaskPlannerPanel; add toggle in command palette + keyboard shortcut (e.g., Cmd+Shift+E) that opens this panel and closes others.
2. Wire file tree double-click/enter to open file in this sidebar (without changing pane layout); also allow command palette action "Open in Sidebar".
3. Update status bar to show active sidebar editor state (path, mode) when focused.
4. Ensure panel is hidden/disabled when no session/workingDirectory is available.

## 7) Vim support specifics
1. Default Vim mode enabled (configurable toggle in header); store preference per session.
2. Show mode indicator (Normal/Insert/Visual) in status strip; handle Esc to Normal even when focus inside panel container.
3. Ensure non-Vim shortcuts (save, palette) still work by placing custom keymap after Vim extension.

## 8) Validation, accessibility, and performance
1. Ensure keyboard-only operation (focusable header buttons, tab order, ARIA labels on controls).
2. Handle very large files gracefully: show warning and offer view-only mode if size > threshold; consider lazy-loading language bundles.
3. Support high DPI/theme: match Tailwind tokens; adopt existing code block theme or mirror Zed-like soft contrast.

## 9) Testing and verification plan
1. Unit: hook tests for `useFileEditorSidebar` (load/save state transitions, dirty tracking, vim/wrap toggles).
2. Integration: render `FileEditorSidebarPanel` with mocked fs services; assert save button/shortcut calls write; check resize behavior.
3. E2E: playwright test to open file from command palette into sidebar, edit, save, and observe git status change indicator.
4. Type/lint: ensure new dependencies configured (vite/tsconfig) and run lint/typecheck commands.

## 10) Deliverables (implementation order)
1. Add dependencies (CodeMirror + vim extension + language packs/theme) and wiring in build config if needed.
2. Implement store slice + service helpers (fs read/write, git status refresh, recent files persistence).
3. Build `FileEditorSidebarPanel` + header/content components.
4. Integrate panel into `App` shell and command palette; hook file tree + command palette actions.
2. Implement save/reload flows, dirty prompts, and status bar updates.
5. Add tests (unit/integration/e2e) and docs/README snippet for feature/shortcuts.
