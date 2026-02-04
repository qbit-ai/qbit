# Performance Implementation Plan

## Overview

This plan addresses 49 remaining performance issues identified in `perf-summary.md`. Issues are grouped by common solutions to maximize efficiency - a single fix often addresses multiple issues.

**Current Status:** 33/82 issues fixed (40%)
- SEVERE: 5/5 (100%)
- HIGH: 25/26 (96%)
- MEDIUM: 2/28 (7%)
- LOW: 1/23 (4%)

---

## Phase 1: Quick Wins (Low effort, immediate impact)

### Batch 1.1: Console & Constants Cleanup
**Issues:** #51, #64

| Issue | File | Fix |
|-------|------|-----|
| #51 | Multiple files | Remove console.log statements from production code |
| #64 | `store/index.ts:1993-2024` | Consolidate duplicate empty array constants |

### Batch 1.2: Debouncing
**Issues:** #44

| Issue | File | Fix |
|-------|------|-----|
| #44 | `usePathCompletion.ts:16-47` | Add debouncing to API calls using `useDeferredValue` |

### Batch 1.3: Lazy Load Settings Tabs
**Issues:** #52

| Issue | File | Fix |
|-------|------|-----|
| #52 | `Settings/index.tsx:26-33` | Apply React.lazy pattern (same as PaneLeaf) |

---

## Phase 2: Memoization Pass (Medium effort, high impact)

### Batch 2.1: Inline Function Extraction
**Issues:** #39, #58, #59, #69, #81

| Issue | File | Fix |
|-------|------|-----|
| #39 | `HistorySearchPopup.tsx:19-51` | Memoize `highlightMatch` with useCallback |
| #58 | `SessionBrowser.tsx:262-309` | Extract inline callbacks with useCallback |
| #59 | `SettingsTabContent.tsx:250-272` | Extract nav onClick handlers with useCallback |
| #69 | `SlashCommandPopup.tsx:129-136` | Memoize filter function |
| #81 | `HomeView.tsx:94-146, 296-348` | Extract context menu callbacks |

### Batch 2.2: Object/Component Memoization
**Issues:** #56, #82

| Issue | File | Fix |
|-------|------|-----|
| #56 | `ThinkingBlock.tsx:62-128` | Move `components` object to module scope |
| #82 | `HomeView.tsx:46-80` | Add React.memo to StatsBadge/WorktreeBadge |

---

## Phase 3: Selector Architecture (Medium effort, high impact)

### Batch 3.1: Create Granular Selectors
**Issues:** #37, #57

| Issue | File | Pattern |
|-------|------|---------|
| #37 | `GitPanel.tsx:453-456` | Create `store/selectors/git-panel.ts` with combined selector |
| #57 | `InlineTaskPlan.tsx:21` | Create `store/selectors/task-plan.ts` for session subset |

**Pattern:** Follow existing `useSessionState()` in `store/selectors/session.ts`

---

## Phase 4: Virtualization (Higher effort, high impact)

### Batch 4.1: List Virtualization
**Issues:** #38, #40, #50

| Issue | Component | Approach |
|-------|-----------|----------|
| #38 | `HistorySearchPopup.tsx:120-146` | Add @tanstack/react-virtual |
| #40 | `FileBrowser.tsx:175-198` | Add virtualization for file tree |
| #50 | `SessionBrowser.tsx:99` | Add pagination or cursor-based loading |

**Pattern:** Follow `VirtualizedMessagesList` in `SessionBrowser.tsx`

---

## Phase 5: Cleanup & Cancellation (Medium effort, prevents memory leaks)

### Batch 5.1: Effect Cleanup Returns
**Issues:** #32, #35, #61, #62

| Issue | File | Fix |
|-------|------|-----|
| #32 | `systemNotifications.ts:255-268` | Add cleanup for `listenForSettingsUpdates` |
| #35 | `useTauriEvents.ts:488-491` | Await cleanup promises in effect return |
| #61 | `LiveTerminalBlock.tsx:15-28` | Add cleanup return to effect |
| #62 | `TerminalInstanceManager.ts:92-103` | Cancel RAF in cleanup |

### Batch 5.2: Async Cancellation
**Issues:** #34, #43, #46

| Issue | File | Fix |
|-------|------|-----|
| #34 | `Terminal.tsx:234-287` | Fix async listener race with AbortController |
| #43 | `ContextPanel.tsx:203-213` | Add AbortController for async operations |
| #46 | `HomeView.tsx:391-398` | Make focus handler cancellable |

---

## Phase 6: Terminal System Improvements (Higher effort, complex)

### Batch 6.1: Terminal Memory Management
**Issues:** #33, #53, #55, #60

| Issue | Description | Fix |
|-------|-------------|-----|
| #33 | Terminal parking lot DOM accumulation | Add LRU cache or cleanup strategy |
| #53 | StaticTerminalOutput creates xterm per block | Pool xterm instances |
| #55 | VirtualTerminal unbounded Promise array | Add cleanup for resolved promises |
| #60 | VirtualTerminal retention on interrupt | Clear on interrupt |

### Batch 6.2: Terminal Architecture
**Issues:** #54, #77, #78, #79, #80

| Issue | Description | Fix |
|-------|-------------|-----|
| #54 | Portal target Map recreation | Memoize Map creation |
| #77 | SyncOutputBuffer timeout hardcoded | Make configurable |
| #78 | TerminalInstanceManager fixed positioning | Review fixed positioning |
| #79 | Terminal disposal edge cases | Add proper disposal guards |
| #80 | LiveTerminalManager creates before attach | Delay creation until attach |

---

## Phase 7: Effect Refactoring (Higher effort, improves maintainability)

### Batch 7.1: Large Effect Splitting
**Issues:** #70, #71, #72, #75

| Issue | File | Approach |
|-------|------|----------|
| #70 | `useTauriEvents.ts` (350 lines) | Split into focused custom hooks |
| #71 | `useTheme.tsx` (3 effects) | Consolidate into single effect |
| #72 | `Terminal.tsx` (240 lines) | Extract listener setup to hook |
| #75 | `Terminal.tsx:234-287` | Extract listener pattern to reusable hook |

---

## Phase 8: Caching & Optimization (Medium effort)

### Batch 8.1: Data Fetching Optimization
**Issues:** #41, #42, #48, #49

| Issue | File | Fix |
|-------|------|-----|
| #41 | `useCommandHistory.ts:42-64` | Add fetch deduplication |
| #42 | `SessionBrowser.tsx:110-137` | Batch state updates with React 18 automatic batching |
| #48 | `HomeView.tsx:387-389` | Cache data across mounts |
| #49 | `useSlashCommands.ts:77-79` | Cache commands per directory |

---

## Phase 9: Error Handling & Polish (Lower priority)

### Batch 9.1: Error Boundaries
**Issues:** #47

| Issue | File | Fix |
|-------|------|-----|
| #47 | `UnifiedTimeline.tsx:317-398` | Add React error boundaries around streaming blocks |

### Batch 9.2: Miscellaneous Optimizations
**Issues:** #63, #65, #66, #68, #73, #76

| Issue | File | Fix |
|-------|------|-----|
| #63 | `ThemeManager.ts:35-39` | Use Set instead of array filter |
| #65 | `store/index.ts:51-65` | Add index lookup for markTabNewActivityInDraft |
| #66 | `store/index.ts:44-45` | Evaluate if enableMapSet() is needed |
| #68 | `WorkflowTree.tsx:71-225` | Flatten nested loop data structure |
| #73 | Multiple popup files | Extract shared useClickOutside hook |
| #76 | `GitPanel.tsx:671-675` | Debounce refresh on dialog open |

---

## Phase 10: UnifiedInput Refactor (Separate track)

**Issue:** #16 - UnifiedInput component too large (1479 lines)

See `frontend/components/UnifiedInput/REFACTOR_PLAN.md` for detailed breakdown.

This should be implemented as a separate focused effort after the other phases are complete.

---

## Recommended Execution Order

| Priority | Phase | Rationale |
|----------|-------|-----------|
| 1 | Phase 1 (Quick Wins) | Immediate, low risk |
| 2 | Phase 2 (Memoization) | High ROI, familiar patterns |
| 3 | Phase 3 (Selectors) | Uses established patterns |
| 4 | Phase 5 (Cleanup) | Prevents memory leaks |
| 5 | Phase 4 (Virtualization) | Highest user-facing impact |
| 6 | Phase 8 (Caching) | Reduces redundant work |
| 7 | Phase 6 (Terminal) | Complex, needs careful testing |
| 8 | Phase 7 (Effect Refactoring) | Improves maintainability |
| 9 | Phase 9 (Polish) | Lower priority |
| 10 | Phase 10 (UnifiedInput) | Separate track |

---

## Progress Tracking

Update `perf-summary.md` as issues are completed. Each batch should be committed separately with a descriptive message following the pattern:

```
perf(frontend): <batch description>

Fixes #<issue numbers>
```
