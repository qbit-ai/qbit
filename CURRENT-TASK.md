# Q-183: File Browser Updates — Implementation Plan

## Feature 1: Show Hidden Files Toggle

**Problem**: The backend `list_directory` command hard-skips all dotfiles (`name.starts_with('.')`) with no way to override this.

**Changes**:

1. **Backend** (`backend/crates/qbit/src/commands/files.rs`):
   - Add a `show_hidden: bool` parameter to `list_directory`
   - Only skip `name.starts_with('.')` entries when `show_hidden` is `false`

2. **Frontend lib** (`frontend/lib/file-editor.ts`):
   - Update `listDirectory` to accept and pass `showHidden?: boolean`

3. **Frontend store** (`frontend/store/file-editor-sidebar.ts`):
   - Add `showHiddenFiles: boolean` to state (default `false`)
   - Add `setShowHiddenFiles` action
   - Persist it in the `partialize` section

4. **Frontend UI** (`frontend/components/FileEditorSidebar/FileBrowser.tsx`):
   - Add a toggle button (eye icon) in the toolbar next to Refresh
   - Pass `showHidden` through to `listDirectory`
   - Add `showHiddenFiles` and `onToggleHiddenFiles` props

5. **Wire up** in `FileEditorSidebarPanel.tsx`: read from store, pass to `FileBrowser`

---

## Feature 2: Editable Path Bar

**Problem**: The footer currently shows the path as read-only text. Users want to type/paste a path directly.

**Changes**:

1. **Frontend** (`frontend/components/FileEditorSidebar/FileEditorSidebarPanel.tsx`):
   - Replace the read-only `<span>` path display in the footer with an editable `<input>`
   - The input shows the current path and allows editing
   - On `Enter`, navigate to the typed path (call `setBrowserPath`)
   - On `Escape`, revert to current path
   - Style it as a minimal input that looks like text until focused

---

## Feature 3: Open File Hotkey with Fuzzy Search

**Problem**: No quick way to open a file by name. Users want a `Cmd+P` style fuzzy finder.

**Changes**:

1. **Frontend — New modal component** (`frontend/components/QuickOpenDialog/`):
   - Uses `searchFiles` from `@/lib/indexer` for fuzzy file search
   - Shows results in a scrollable list with keyboard navigation (up/down/enter)
   - On selection, opens the file in the file editor sidebar

2. **Hotkey** (`frontend/hooks/useKeyboardHandlerContext.ts`):
   - Add `Cmd+P` handler that opens the quick-open dialog
   - Add `openQuickOpen` callback to `KeyboardHandlerContext`

3. **App wiring** (`frontend/App.tsx`):
   - Add `quickOpenDialogOpen` state
   - Pass opener to keyboard context
   - Render `QuickOpenDialog` component

4. **Command Palette** (`frontend/components/CommandPalette/CommandPalette.tsx`):
   - Add "Open File" entry with `Cmd+P` shortcut
