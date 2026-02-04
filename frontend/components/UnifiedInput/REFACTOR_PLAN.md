# UnifiedInput Refactoring Plan

## Current Structure Analysis

### Metrics
- **Total lines:** 1,487
- **Number of useState calls:** 21
- **Number of useEffect calls:** 7
- **Number of useCallback calls:** 15
- **Number of useMemo calls:** 2
- **Number of useRef calls:** 8

### Main Responsibilities

The UnifiedInput component currently handles 10+ distinct responsibilities:

1. **Input Management**
   - Text input state and textarea handling
   - Auto-resize textarea height
   - Mode switching (terminal/agent)
   - Input submission handling

2. **Command History Navigation**
   - Arrow up/down history navigation
   - History search (Ctrl+R) with search query state
   - History match selection

3. **Slash Command Popup**
   - Show/hide popup state
   - Command filtering
   - Selection navigation and execution

4. **File Command Popup (@-mentions)**
   - Show/hide popup state
   - File filtering and selection

5. **Path Completion (Terminal Mode)**
   - Path query state and completions
   - Ghost text hint display
   - Tab completion behavior

6. **Image Attachments (Agent Mode)**
   - Attachment state management
   - Vision capabilities checking
   - Drag-and-drop handling via Tauri events
   - Clipboard paste handling

7. **Keyboard Event Handling**
   - 200+ lines of handleKeyDown logic
   - Mode-specific shortcuts (Ctrl+C, Ctrl+D, Ctrl+L for terminal)
   - Popup navigation (arrows, Tab, Enter, Escape)

8. **Status Display**
   - Working directory badge
   - Git branch/status badge
   - Virtual environment badge

9. **Session Lifecycle**
   - Session switching cleanup
   - Agent busy/dead state handling
   - Submission state management

10. **Drop Zone Management**
    - Pane-level drag-over visual state
    - Drop zone rect caching and hit testing
    - Portal rendering for overlay

### State Dependencies

```
Local State (21 useState):
├── input                    -> used by: handleSubmit, handleKeyDown, onChange
├── isSubmitting             -> used by: isAgentBusy, handleSubmit, session effects
├── showSlashPopup           -> used by: popup rendering, handleKeyDown
├── slashSelectedIndex       -> used by: popup navigation, handleKeyDown
├── showFilePopup            -> used by: popup rendering, handleKeyDown, onChange
├── fileSelectedIndex        -> used by: popup navigation, handleKeyDown
├── showPathPopup            -> used by: popup rendering, handleKeyDown, onChange
├── pathSelectedIndex        -> used by: popup navigation, handleKeyDown
├── pathQuery                -> used by: usePathCompletion, ghostText
├── showHistorySearch        -> used by: popup rendering, handleKeyDown
├── historySearchQuery       -> used by: useHistorySearch, handleKeyDown
├── historySelectedIndex     -> used by: handleKeyDown
├── originalInput            -> used by: history search cancel
├── imageAttachments         -> used by: handleSubmit, ImageAttachment, paste/drop handlers
├── visionCapabilities       -> used by: ImageAttachment, handleSubmit validation
├── isDragOver               -> used by: drop zone overlay
└── dragError                -> used by: drop zone overlay

Store Subscriptions (via useUnifiedInputState selector):
├── inputMode
├── virtualEnv
├── isAgentResponding
├── isCompacting
├── isSessionDead
├── streamingBlocksLength
├── gitBranch
└── gitStatus
```

### Callback Dependencies Graph

```
handleSubmit
├── uses: input, inputMode, isAgentBusy, imageAttachments, visionCapabilities
├── calls: setInput, resetHistory, addToHistory, addAgentMessage, ptyWrite
└── calls: setLastSentCommand, sendPromptSession, sendPromptWithAttachments

handleKeyDown
├── uses: ALL popup states, ALL selection indices, input, inputMode
├── calls: ALL popup state setters, handleSubmit, navigation hooks
└── handles: 15+ different key combinations

handleSlashSelect
├── uses: inputMode
├── calls: setShowSlashPopup, setInput, setInputMode, addAgentMessage
└── calls: setIsSubmitting, sendPromptSession

handleFileSelect
├── uses: input
└── calls: setShowFilePopup, setInput, setFileSelectedIndex

handlePathSelect / handlePathSelectFinal
├── uses: input, pathCompletions, pathSelectedIndex
└── calls: setInput, setPathQuery, setShowPathPopup, setPathSelectedIndex

handleHistorySelect
└── calls: setInput, setShowHistorySearch, setHistorySearchQuery, setHistorySelectedIndex

processImageFiles / processFilePaths
├── uses: visionCapabilities
└── called by: handlePaste, Tauri drag-drop listener
```

---

## Proposed Component Structure

### 1. UnifiedInput.tsx (Orchestrator) - Target: ~250 lines

**Purpose:** Top-level composition and mode orchestration

**Responsibilities:**
- Compose sub-components
- Manage mode switching (terminal/agent)
- Session-level state coordination
- Render layout structure

**State to retain:**
- `input` and `setInput` (shared across modes)
- `isSubmitting` (affects both modes)

**Receives from children:**
- Submission callbacks
- Popup trigger signals

```tsx
// Simplified structure
export function UnifiedInput({ sessionId, workingDirectory, onOpenGitPanel }) {
  const [input, setInput] = useState("");
  const [isSubmitting, setIsSubmitting] = useState(false);
  const textareaRef = useRef<HTMLTextAreaElement>(null);

  const { inputMode, ... } = useUnifiedInputState(sessionId);

  return (
    <div className="border-t">
      <InlineTaskPlan sessionId={sessionId} />
      <InputStatusBadges
        sessionId={sessionId}
        workingDirectory={workingDirectory}
        onOpenGitPanel={onOpenGitPanel}
        isAgentBusy={isAgentBusy}
      />
      <InputContainer
        sessionId={sessionId}
        input={input}
        setInput={setInput}
        inputMode={inputMode}
        textareaRef={textareaRef}
        isSubmitting={isSubmitting}
        setIsSubmitting={setIsSubmitting}
        workingDirectory={workingDirectory}
      />
      <InputStatusRow sessionId={sessionId} />
    </div>
  );
}
```

---

### 2. InputStatusBadges.tsx - Target: ~120 lines

**Purpose:** Display working directory, git, and virtualenv badges

**Responsibilities:**
- Path abbreviation logic
- Git branch/status display
- Virtual environment badge
- Shimmer animation on busy state

**Props:**
```tsx
interface InputStatusBadgesProps {
  sessionId: string;
  workingDirectory?: string;
  onOpenGitPanel?: () => void;
  isAgentBusy: boolean;
}
```

**State:** None (purely presentational, derived from props/store)

---

### 3. InputContainer.tsx - Target: ~300 lines

**Purpose:** Main input area with popups and drag-drop handling

**Responsibilities:**
- Textarea rendering and auto-resize
- Popup wrapper composition (nested Radix popovers)
- Send button
- Image attachment integration
- Drop zone overlay (portal rendering)

**Extracted components composed within:**
- HistorySearchPopup
- PathCompletionPopup
- SlashCommandPopup
- FileCommandPopup
- GhostTextHint
- ImageAttachment (already extracted)

**Props:**
```tsx
interface InputContainerProps {
  sessionId: string;
  input: string;
  setInput: (value: string) => void;
  inputMode: "terminal" | "agent";
  textareaRef: React.RefObject<HTMLTextAreaElement>;
  isSubmitting: boolean;
  setIsSubmitting: (value: boolean) => void;
  workingDirectory?: string;
}
```

**State to manage:**
- All popup show/hide states
- All selected index states
- `pathQuery` for path completions
- `historySearchQuery` and `originalInput`
- `isDragOver` and `dragError`

---

### 4. useInputSubmission.ts (Custom Hook) - Target: ~150 lines

**Purpose:** Encapsulate submission logic for both terminal and agent modes

**Responsibilities:**
- Terminal command submission (ptyWrite)
- Agent prompt submission (with/without attachments)
- Clear command handling
- Submission state management
- History integration

**Interface:**
```tsx
interface UseInputSubmissionOptions {
  sessionId: string;
  inputMode: "terminal" | "agent";
  imageAttachments: ImagePart[];
  visionCapabilities: VisionCapabilities | null;
  onClearAttachments: () => void;
}

interface UseInputSubmissionReturn {
  handleSubmit: (input: string) => Promise<void>;
  isSubmitting: boolean;
  setIsSubmitting: (value: boolean) => void;
}
```

---

### 5. useKeyboardNavigation.ts (Custom Hook) - Target: ~250 lines

**Purpose:** Centralize keyboard event handling

**Responsibilities:**
- History navigation (arrows, Ctrl+R)
- Popup navigation (arrows, Tab, Enter, Escape)
- Mode-specific shortcuts (terminal: Ctrl+C, Ctrl+D, Ctrl+L)
- Mode toggle (Cmd+I, Cmd+Shift+T)
- Slash command execution

**Interface:**
```tsx
interface UseKeyboardNavigationOptions {
  sessionId: string;
  inputMode: "terminal" | "agent";
  // Popup states
  popupStates: {
    slash: PopupState<SlashCommand>;
    file: PopupState<FileInfo>;
    path: PopupState<PathCompletion>;
    history: PopupState<HistoryMatch>;
  };
  // Callbacks
  onSubmit: () => Promise<void>;
  onInputChange: (value: string) => void;
  onToggleMode: () => void;
  // History hooks
  historyNav: { navigateUp: () => string | null; navigateDown: () => string };
  // Refs
  textareaRef: React.RefObject<HTMLTextAreaElement>;
}

interface PopupState<T> {
  isOpen: boolean;
  setIsOpen: (open: boolean) => void;
  items: T[];
  selectedIndex: number;
  setSelectedIndex: (index: number) => void;
  onSelect: (item: T, args?: string) => void;
}
```

---

### 6. useImageAttachments.ts (Custom Hook) - Target: ~120 lines

**Purpose:** Manage image attachment lifecycle and validation

**Responsibilities:**
- Vision capabilities fetching
- File processing (from paste and drop)
- Attachment array state
- Validation against provider limits

**Interface:**
```tsx
interface UseImageAttachmentsOptions {
  sessionId: string;
  inputMode: "terminal" | "agent";
}

interface UseImageAttachmentsReturn {
  attachments: ImagePart[];
  setAttachments: (attachments: ImagePart[]) => void;
  capabilities: VisionCapabilities | null;
  processImageFiles: (files: FileList | File[]) => Promise<ImagePart[]>;
  processFilePaths: (paths: string[]) => Promise<ImagePart[]>;
  clearAttachments: () => void;
}
```

---

### 7. useDragDrop.ts (Custom Hook) - Target: ~100 lines

**Purpose:** Handle Tauri drag-drop events for image attachment

**Responsibilities:**
- Tauri event listeners setup/cleanup
- Drop zone rect caching
- Position hit testing
- Drag state management

**Interface:**
```tsx
interface UseDragDropOptions {
  sessionId: string;
  inputMode: "terminal" | "agent";
  onDrop: (filePaths: string[]) => Promise<void>;
}

interface UseDragDropReturn {
  isDragOver: boolean;
  dragError: string | null;
  dropZoneRef: React.RefObject<HTMLDivElement>;
  paneContainerRef: React.RefObject<HTMLElement>;
}
```

---

### 8. usePopupTriggers.ts (Custom Hook) - Target: ~80 lines

**Purpose:** Detect when to show popups based on input changes

**Responsibilities:**
- Slash command detection (`/` at start)
- File mention detection (`@` in agent mode)
- Path completion trigger (Tab in terminal mode)

**Interface:**
```tsx
interface UsePopupTriggersOptions {
  input: string;
  inputMode: "terminal" | "agent";
  commands: SlashCommand[];
}

interface UsePopupTriggersReturn {
  shouldShowSlashPopup: boolean;
  shouldShowFilePopup: boolean;
  slashCommandName: string;
  fileQuery: string;
}
```

---

## File Structure After Refactoring

```
frontend/components/UnifiedInput/
├── UnifiedInput.tsx           # Main orchestrator (~250 lines)
├── InputStatusBadges.tsx      # Status badges component (~120 lines)
├── InputContainer.tsx         # Input area with popups (~300 lines)
├── InputStatusRow.tsx         # Already extracted (1312 lines - see note below)
├── ImageAttachment.tsx        # Already extracted (276 lines)
├── GhostTextHint.tsx          # Small presentational component (~30 lines)
├── hooks/
│   ├── useInputSubmission.ts  # Submission logic (~150 lines)
│   ├── useKeyboardNavigation.ts # Keyboard handling (~250 lines)
│   ├── useImageAttachments.ts # Image attachment logic (~120 lines)
│   ├── useDragDrop.ts         # Drag-drop handling (~100 lines)
│   └── usePopupTriggers.ts    # Popup show/hide triggers (~80 lines)
├── utils/
│   └── inputHelpers.ts        # Pure utility functions (~50 lines)
└── index.ts                   # Barrel exports
```

**Note:** InputStatusRow.tsx is already quite large (1312 lines) and may warrant its own refactoring in a separate effort, but it's already extracted from UnifiedInput.

---

## Migration Strategy

### Phase 1: Extract Utility Functions and Types (Low Risk)

1. Create `utils/inputHelpers.ts` with pure functions:
   - `extractWordAtCursor()`
   - `isCursorOnFirstLine()`
   - `isCursorOnLastLine()`
   - `clearTerminal()`

2. Create shared types file if needed

3. **Verification:** Run tests, ensure no behavior change

### Phase 2: Extract Custom Hooks (Medium Risk)

**Order matters - extract in dependency order:**

1. `usePopupTriggers.ts` - No dependencies on other new hooks
2. `useDragDrop.ts` - Uses only Tauri events
3. `useImageAttachments.ts` - May use `useDragDrop`
4. `useInputSubmission.ts` - Independent logic
5. `useKeyboardNavigation.ts` - Uses other hooks/state

**For each hook:**
- Extract with minimal changes to interface
- Keep stateRef pattern for stable callbacks if needed
- Add unit tests before extraction
- Verify existing tests pass after extraction

### Phase 3: Extract UI Components (Medium Risk)

1. Extract `GhostTextHint` (already memoized inline, just move to file)
2. Extract `InputStatusBadges`
3. Extract `InputContainer`

**For each component:**
- Move JSX and related logic
- Wire up to parent via props
- Ensure CSS classes and animations transfer correctly

### Phase 4: Simplify UnifiedInput.tsx (Final Cleanup)

1. Replace inline code with hook calls
2. Replace inline JSX with component composition
3. Remove unused imports
4. Add/update barrel exports

### Phase 5: Test Updates and Documentation

1. Update existing tests to target new file locations
2. Add unit tests for new hooks
3. Add integration tests for component composition
4. Update Storybook stories if applicable

---

## Risk Assessment

### Breaking Changes

1. **Import paths** - Components importing from `./UnifiedInput` need updates
   - Mitigation: Use index.ts barrel exports to maintain backwards compatibility

2. **Context/ref threading** - textareaRef is used across multiple concerns
   - Mitigation: Keep ref in top-level UnifiedInput, pass down via props

3. **stateRef pattern** - Complex optimization pattern for stable callbacks
   - Mitigation: May need to keep this in InputContainer or use a simpler approach with useCallback dependencies now that hooks are separated

### Test Coverage Gaps

Current test files identified:
- `UnifiedInput.stateRef.test.tsx` - Tests stateRef optimization
- `InputStatusRow.test.tsx` - Tests status row

**Additional tests needed:**
- Unit tests for each extracted hook
- Integration tests for popup interactions
- Edge case tests for keyboard navigation
- Drag-drop event simulation tests

### State Synchronization Concerns

1. **Popup state race conditions** - Multiple popups shouldn't be open simultaneously
   - Mitigation: Create `useExclusivePopup` hook that manages mutual exclusion

2. **Submission state across mode switch** - isSubmitting should reset on mode change
   - Mitigation: Handle in useInputSubmission hook, triggered by inputMode dep

3. **Session switching cleanup** - All local state should reset on session change
   - Mitigation: Consolidate cleanup logic in UnifiedInput, pass reset callbacks to hooks

4. **stateRef sync** - Current code updates ref properties directly in render
   - Mitigation: Consider moving to a custom `useLatest` hook for cleaner semantics, or keep the pattern in the component that needs it

---

## Performance Considerations

### Current Optimizations to Preserve

1. **stateRef pattern** - Prevents callback recreation on every keystroke
2. **useUnifiedInputState selector** - Single subscription instead of ~15
3. **GhostTextHint memo** - Prevents re-render on parent changes
4. **requestAnimationFrame for textarea resize** - Batched DOM operations
5. **Cached drop zone rect** - Avoids getBoundingClientRect on every drag-over

### New Optimization Opportunities

1. **Separate popup state subscriptions** - Each popup only subscribes to its own state
2. **Lazy loading of popup components** - Load PathCompletionPopup only in terminal mode
3. **Debounced path query** - Reduce completion API calls during fast typing
4. **Virtualized history matches** - For users with very long history

---

## Success Metrics

After refactoring, we should see:

1. **UnifiedInput.tsx < 300 lines** (down from 1,487)
2. **No single file > 400 lines** in the UnifiedInput directory
3. **Each file has a single responsibility** clearly stated in header comment
4. **All existing tests pass** without modification
5. **No performance regression** in typing/submission latency
6. **Easier to unit test** individual concerns
