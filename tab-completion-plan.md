# Tab Completion for UnifiedInput (Terminal Mode)

## Goal
Add custom tab-completion UI for files/folders in UnifiedInput's terminal mode, providing a dropdown with fuzzy search and file/folder icons.

## Overview
- **Trigger**: Tab key in terminal mode (when no popup is open)
- **Behavior**: Extract word at cursor, query backend for completions, show popup
- **Target**: `UnifiedInput` component only (not xterm.js terminal)

---

## Phase 1: Backend Command (Rust)

**Agent Instructions**: Create a new Tauri command for listing path completions.

### Tasks

1. **Create `src-tauri/src/commands/completions.rs`**

   Define types:
   ```rust
   #[derive(Debug, Clone, Serialize, PartialEq, Eq)]
   #[serde(rename_all = "snake_case")]
   pub enum PathEntryType {
       File,
       Directory,
       Symlink,
   }

   #[derive(Debug, Clone, Serialize)]
   pub struct PathCompletion {
       pub name: String,           // Display name (e.g., "Documents/")
       pub insert_text: String,    // Text to insert
       pub entry_type: PathEntryType,
   }
   ```

   Implement command:
   ```rust
   #[tauri::command]
   pub async fn list_path_completions(
       state: State<'_, AppState>,
       session_id: String,
       partial_path: String,
       limit: Option<usize>,
   ) -> Result<Vec<PathCompletion>>
   ```

   Implementation requirements:
   - Get working directory from PTY session via `state.pty_manager.get_session(&session_id)`
   - Handle empty input (list current directory)
   - Expand tilde (`~/` → home directory)
   - Support absolute paths (`/`)
   - Support relative paths (`./`, `../`)
   - Prefix matching (case-insensitive on macOS)
   - Skip hidden files unless prefix starts with `.`
   - Sort: directories first, then alphabetically
   - Append `/` to directory names
   - Default limit: 20

2. **Update `src-tauri/src/commands/mod.rs`**
   - Add `pub mod completions;`
   - Re-export: `pub use completions::*;`

3. **Update `src-tauri/src/lib.rs`**
   - Add `list_path_completions` to `tauri::generate_handler![]` macro

### Dependencies
- `dirs` crate (for home directory) - check if already in Cargo.toml, add if needed

### Verification
- `cargo check` passes
- `cargo test` passes

---

## Phase 2: Frontend Types & API

**Agent Instructions**: Add TypeScript types and invoke wrapper for the new command.

### Tasks

1. **Update `src/lib/tauri.ts`**

   Add types after existing `FileInfo` type:
   ```typescript
   export type PathEntryType = "file" | "directory" | "symlink";

   export interface PathCompletion {
     name: string;
     insert_text: string;
     entry_type: PathEntryType;
   }
   ```

   Add invoke wrapper:
   ```typescript
   export async function listPathCompletions(
     sessionId: string,
     partialPath: string,
     limit?: number
   ): Promise<PathCompletion[]> {
     return invoke("list_path_completions", {
       sessionId,
       partialPath,
       limit
     });
   }
   ```

### Verification
- `pnpm tsc --noEmit` passes
- `just check` passes

---

## Phase 3: Completion Hook

**Agent Instructions**: Create a React hook to fetch path completions.

### Tasks

1. **Create `src/hooks/usePathCompletion.ts`**

   ```typescript
   import { useCallback, useEffect, useState } from "react";
   import { listPathCompletions, type PathCompletion } from "@/lib/tauri";

   interface UsePathCompletionOptions {
     sessionId: string;
     partialPath: string;
     enabled: boolean;
   }

   export function usePathCompletion({
     sessionId,
     partialPath,
     enabled
   }: UsePathCompletionOptions) {
     const [completions, setCompletions] = useState<PathCompletion[]>([]);
     const [isLoading, setIsLoading] = useState(false);

     useEffect(() => {
       if (!enabled) {
         setCompletions([]);
         return;
       }

       let cancelled = false;
       setIsLoading(true);

       listPathCompletions(sessionId, partialPath, 20)
         .then((results) => {
           if (!cancelled) setCompletions(results);
         })
         .catch((error) => {
           console.error("Path completion error:", error);
           if (!cancelled) setCompletions([]);
         })
         .finally(() => {
           if (!cancelled) setIsLoading(false);
         });

       return () => { cancelled = true; };
     }, [sessionId, partialPath, enabled]);

     return { completions, isLoading };
   }
   ```

### Verification
- `pnpm tsc --noEmit` passes

---

## Phase 4: UI Component

**Agent Instructions**: Create the path completion popup component following the existing `FileCommandPopup` pattern.

### Tasks

1. **Create `src/components/PathCompletionPopup/PathCompletionPopup.tsx`**

   Pattern to follow: `src/components/FileCommandPopup/FileCommandPopup.tsx`

   Features:
   - Use Popover from `@/components/ui/popover`
   - Icon mapping using lucide-react:
     - `directory` → `Folder` (blue color)
     - `symlink` → `Link2` (cyan color)
     - `file` → `File` (muted color)
   - Keyboard navigation (Up/Down arrows)
   - Scroll selected into view
   - Max height 200px with overflow scroll
   - Width ~350px

   Props interface:
   ```typescript
   interface PathCompletionPopupProps {
     open: boolean;
     onOpenChange: (open: boolean) => void;
     completions: PathCompletion[];
     selectedIndex: number;
     onSelect: (completion: PathCompletion) => void;
     children: React.ReactNode;
   }
   ```

2. **Create `src/components/PathCompletionPopup/index.ts`**
   ```typescript
   export { PathCompletionPopup } from "./PathCompletionPopup";
   ```

### Verification
- `pnpm tsc --noEmit` passes
- `just check` passes

---

## Phase 5: Integration into UnifiedInput

**Agent Instructions**: Wire up the path completion popup into the UnifiedInput component.

### Tasks

1. **Modify `src/components/UnifiedInput/UnifiedInput.tsx`**

   **Add imports** (top of file):
   ```typescript
   import { PathCompletionPopup } from "@/components/PathCompletionPopup";
   import { usePathCompletion } from "@/hooks/usePathCompletion";
   import type { PathCompletion } from "@/lib/tauri";
   ```

   **Add state** (after existing useState declarations ~line 58):
   ```typescript
   const [showPathPopup, setShowPathPopup] = useState(false);
   const [pathSelectedIndex, setPathSelectedIndex] = useState(0);
   const [pathQuery, setPathQuery] = useState("");
   ```

   **Add hook** (after existing hooks):
   ```typescript
   const { completions: pathCompletions } = usePathCompletion({
     sessionId,
     partialPath: pathQuery,
     enabled: showPathPopup && inputMode === "terminal",
   });
   ```

   **Add utility function** (before component or as inner function):
   ```typescript
   function extractWordAtCursor(input: string, cursorPos: number): {
     word: string;
     startIndex: number
   } {
     const beforeCursor = input.slice(0, cursorPos);
     const match = beforeCursor.match(/[^\s|;&]+$/);
     if (!match) return { word: "", startIndex: cursorPos };
     return {
       word: match[0],
       startIndex: cursorPos - match[0].length,
     };
   }
   ```

   **Add selection handler** (with other handlers):
   ```typescript
   const handlePathSelect = useCallback(
     (completion: PathCompletion) => {
       const cursorPos = textareaRef.current?.selectionStart ?? input.length;
       const { startIndex } = extractWordAtCursor(input, cursorPos);

       const newInput =
         input.slice(0, startIndex) +
         completion.insert_text +
         input.slice(cursorPos);

       setInput(newInput);
       setShowPathPopup(false);
       setPathSelectedIndex(0);

       // Continue completion for directories
       if (completion.entry_type === "directory") {
         setPathQuery(completion.insert_text);
         setTimeout(() => setShowPathPopup(true), 50);
       }
     },
     [input]
   );
   ```

   **Modify handleKeyDown** - Add path popup keyboard handling BEFORE slash popup handling:
   ```typescript
   // Path completion keyboard navigation (add before slash popup handling)
   if (showPathPopup && pathCompletions.length > 0) {
     if (e.key === "Escape") {
       e.preventDefault();
       setShowPathPopup(false);
       return;
     }
     if (e.key === "ArrowDown") {
       e.preventDefault();
       setPathSelectedIndex((prev) =>
         prev < pathCompletions.length - 1 ? prev + 1 : prev
       );
       return;
     }
     if (e.key === "ArrowUp") {
       e.preventDefault();
       setPathSelectedIndex((prev) => prev > 0 ? prev - 1 : 0);
       return;
     }
     if (e.key === "Tab" || e.key === "Enter") {
       if (!e.shiftKey) {
         e.preventDefault();
         handlePathSelect(pathCompletions[pathSelectedIndex]);
         return;
       }
     }
   }
   ```

   **Modify terminal mode Tab handler** (~line 378, replace existing Tab handling):
   ```typescript
   if (inputMode === "terminal") {
     if (e.key === "Tab") {
       e.preventDefault();

       // If popup already open, select current item
       if (showPathPopup && pathCompletions.length > 0) {
         handlePathSelect(pathCompletions[pathSelectedIndex]);
         return;
       }

       // Extract word at cursor and show popup
       const cursorPos = textareaRef.current?.selectionStart ?? input.length;
       const { word } = extractWordAtCursor(input, cursorPos);
       setPathQuery(word);
       setShowPathPopup(true);
       setPathSelectedIndex(0);
       return;
     }
     // ... rest of terminal mode handling
   }
   ```

   **Wrap textarea with PathCompletionPopup** - Add as outermost popup wrapper in JSX:
   ```tsx
   <PathCompletionPopup
     open={showPathPopup}
     onOpenChange={setShowPathPopup}
     completions={pathCompletions}
     selectedIndex={pathSelectedIndex}
     onSelect={handlePathSelect}
   >
     {/* Existing SlashCommandPopup > FileCommandPopup > textarea structure */}
   </PathCompletionPopup>
   ```

   **Close popup on input change** - Add to the onChange handler or useEffect:
   ```typescript
   // Reset path popup when input changes significantly
   useEffect(() => {
     if (showPathPopup) {
       const cursorPos = textareaRef.current?.selectionStart ?? input.length;
       const { word } = extractWordAtCursor(input, cursorPos);
       if (word !== pathQuery) {
         setPathQuery(word);
         setPathSelectedIndex(0);
       }
     }
   }, [input, showPathPopup, pathQuery]);
   ```

### Verification
- `pnpm tsc --noEmit` passes
- `just check` passes
- Manual test: Tab key in terminal mode shows completion popup

---

## Files Summary

| Phase | File | Action |
|-------|------|--------|
| 1 | `src-tauri/src/commands/completions.rs` | Create |
| 1 | `src-tauri/src/commands/mod.rs` | Modify |
| 1 | `src-tauri/src/lib.rs` | Modify |
| 2 | `src/lib/tauri.ts` | Modify |
| 3 | `src/hooks/usePathCompletion.ts` | Create |
| 4 | `src/components/PathCompletionPopup/PathCompletionPopup.tsx` | Create |
| 4 | `src/components/PathCompletionPopup/index.ts` | Create |
| 5 | `src/components/UnifiedInput/UnifiedInput.tsx` | Modify |

---

## Edge Cases to Handle

1. **Empty input**: Complete from current working directory
2. **Paths with spaces**: Word extraction uses common shell delimiters
3. **Hidden files**: Only show when prefix starts with `.`
4. **Directory completion**: Append `/` and optionally continue completion
5. **Permission errors**: Skip unreadable directories silently
6. **Symlinks**: Show with distinct icon, resolve to determine type
7. **Tilde expansion**: `~/` expands to home directory
