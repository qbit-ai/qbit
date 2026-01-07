# Terminal Streaming Implementation Plan

## Goal
Replace the current ANSI text-based output rendering for **streaming commands** with an embedded xterm.js terminal instance, making output appear like a real terminal emulator while keeping it in timeline blocks.

## Key Decisions
1. **Fixed height** - Terminal block will have a fixed height (matching current `max-h-96` / 384px)
2. **Auto-scroll on completion** - When command completes, scroll terminal to bottom
3. **Reuse ThemeManager** - Use existing `ThemeManager.applyToTerminal()` for theme consistency
4. **Single live terminal only** - Only the currently streaming command uses xterm.js; completed commands continue using `ansi-to-react`

## Architecture

### Current Flow (Streaming Output)
```
PTY Output → terminal_output event → appendOutput() → store.pendingCommand.output
                                   → virtualTerminalManager.write()
                                   ↓
UnifiedTimeline reads pendingCommand.output → useProcessedOutput() → <Ansi> component
```

### New Flow (Streaming Output)
```
PTY Output → terminal_output event → LiveTerminalManager.write()
                                   ↓
UnifiedTimeline renders <LiveTerminalBlock> with embedded xterm.js
                                   ↓
On command completion → serialize terminal content → CommandBlock with <Ansi>
```

## Files to Create

### 1. `frontend/components/LiveTerminalBlock/LiveTerminalBlock.tsx`
Embedded xterm.js terminal for active command streaming.

**Responsibilities:**
- Render xterm.js terminal in a fixed-height container (384px)
- Receive output via LiveTerminalManager
- Apply theme via ThemeManager
- Use canvas renderer (not WebGL - smaller area, less benefit)
- Minimal scrollback (500 lines - will serialize to CommandBlock anyway)
- No user input (read-only display)

**Props:**
```typescript
interface LiveTerminalBlockProps {
  sessionId: string;
  command: string;  // For header display
}
```

### 2. `frontend/lib/terminal/LiveTerminalManager.ts`
Singleton manager for the live streaming terminal instance.

**Responsibilities:**
- Manage single xterm.js instance lifecycle
- Write streaming output to terminal
- Serialize terminal content on command completion (via SerializeAddon)
- Reset terminal for next command
- Handle attach/detach from DOM containers

**API:**
```typescript
class LiveTerminalManager {
  // Get or create terminal for session
  getOrCreate(sessionId: string): Terminal;
  
  // Write output data
  write(sessionId: string, data: string): void;
  
  // Attach terminal to DOM container
  attachToContainer(sessionId: string, container: HTMLElement): void;
  
  // Serialize and dispose (called on command completion)
  serializeAndDispose(sessionId: string): string;
  
  // Scroll to bottom
  scrollToBottom(sessionId: string): void;
  
  // Dispose without serializing
  dispose(sessionId: string): void;
}
```

## Files to Modify

### 1. `frontend/components/UnifiedTimeline/UnifiedTimeline.tsx`
**Changes:**
- Replace streaming output section (lines ~205-225) with `<LiveTerminalBlock>`
- Remove `useProcessedOutput` hook usage for pending commands
- Import LiveTerminalBlock component

**Before:**
```tsx
{pendingOutput && (
  <div className="ansi-output text-xs ... max-h-96 overflow-auto">
    <Ansi useClasses>{pendingOutput}</Ansi>
  </div>
)}
```

**After:**
```tsx
{pendingCommand?.command && (
  <LiveTerminalBlock 
    sessionId={sessionId} 
    command={pendingCommand.command}
  />
)}
```

### 2. `frontend/hooks/useTauriEvents.ts`
**Changes:**
- On `terminal_output` event: write to `liveTerminalManager` instead of (or in addition to) `virtualTerminalManager`
- On `command_block` event (command completion): 
  - Call `liveTerminalManager.scrollToBottom()` 
  - Call `liveTerminalManager.serializeAndDispose()` to get final output
  - Store serialized output in CommandBlock

**Location:** Lines ~269-275 (terminal_output handler)

### 3. `frontend/lib/terminal/index.ts`
**Changes:**
- Export `liveTerminalManager` singleton

## Implementation Steps

### Phase 1: Create LiveTerminalManager
1. Create `frontend/lib/terminal/LiveTerminalManager.ts`
2. Implement singleton pattern similar to TerminalInstanceManager
3. Use FitAddon for sizing, SerializeAddon for content extraction
4. No WebGL addon (canvas renderer is fine for small terminal)
5. Export from `frontend/lib/terminal/index.ts`

### Phase 2: Create LiveTerminalBlock Component
1. Create `frontend/components/LiveTerminalBlock/LiveTerminalBlock.tsx`
2. Create `frontend/components/LiveTerminalBlock/index.ts` for export
3. Fixed height container (h-96 = 384px)
4. Attach to LiveTerminalManager on mount
5. Apply theme via ThemeManager
6. Include command header (similar to current streaming output header)

### Phase 3: Integrate into UnifiedTimeline
1. Import LiveTerminalBlock
2. Replace streaming output Ansi section with LiveTerminalBlock
3. Remove/simplify useProcessedOutput usage for pending commands

### Phase 4: Update Event Handlers
1. Modify `useTauriEvents.ts` terminal_output handler to write to liveTerminalManager
2. Modify command_block handler to serialize terminal on completion
3. Auto-scroll to bottom before serialization

### Phase 5: Cleanup
1. VirtualTerminalManager can remain for other uses (or remove if unused)
2. Test with various commands (spinners, progress bars, colored output)
3. Verify theme switching works

## Terminal Configuration

```typescript
const terminal = new Terminal({
  cursorBlink: false,        // Read-only display
  cursorStyle: "block",
  cursorInactiveStyle: "none",
  disableStdin: true,        // No user input
  fontSize: 12,              // Slightly smaller for timeline
  fontFamily: "JetBrains Mono, Menlo, Monaco, Consolas, monospace",
  scrollback: 500,           // Minimal - content serialized on completion
  convertEol: true,          // Handle \n properly
  allowProposedApi: true,
});
```

## Testing Checklist
- [ ] Basic command output displays correctly
- [ ] Spinner animations (e.g., npm install) render properly
- [ ] Progress bars update in place
- [ ] Colored output (ANSI colors) displays correctly
- [ ] Theme switching applies to live terminal
- [ ] Command completion scrolls to bottom
- [ ] Serialized output in CommandBlock matches terminal display
- [ ] Multiple rapid commands don't cause issues
- [ ] Memory usage stays bounded (only 1 xterm instance)

## Rollback Plan
If issues arise, revert to ansi-to-react by:
1. Removing LiveTerminalBlock usage from UnifiedTimeline
2. Restoring original streaming output section
3. LiveTerminalManager can remain dormant
