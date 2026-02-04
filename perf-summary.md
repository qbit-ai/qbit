# Performance Issues - Complete Consolidated List

## SEVERE (Critical Performance Impact)

| # | Issue | Location | Status | Notes |
|---|-------|----------|--------|-------|
| 1 | **Sidebar resize handler with no throttling** | `Sidebar.tsx:292-317` | ✅ FIXED | Added `useThrottledResize` hook |
| 2 | **FileEditorSidebarPanel resize handler no throttle** | `FileEditorSidebarPanel.tsx:354-385` | ✅ FIXED | Added `useThrottledResize` hook |
| 3 | **ContextPanel resize handler no throttle** | `ContextPanel.tsx:86-110` | ✅ FIXED | Added `useThrottledResize` hook |
| 4 | **Terminal resize over-debouncing** | `Terminal.tsx:329-342` | ✅ FIXED | Changed from double RAF + 50ms to single RAF (~16ms) |
| 5 | **App.tsx keyboard shortcuts effect** | `App.tsx` | ✅ FIXED | Extracted to `useKeyboardHandlerContext` with stable refs |

**Progress: 5/5 COMPLETE**

---

## HIGH Priority

| # | Issue | Location | Status | Notes |
|---|-------|----------|--------|-------|
| 6 | `notificationToTabMap` grows indefinitely | `systemNotifications.ts` | ✅ FIXED | Added cleanup on tab close |
| 7 | `lastSeenSeq` Map with async cleanup | `useAiEvents.ts` | ✅ FIXED | Added proper cleanup handling |
| 8 | App.tsx subscribes to entire Records | `App.tsx:84-97` | ✅ FIXED | Created `store/selectors/app.ts` with granular selectors |
| 9 | PaneLeaf subscribes to full objects | `PaneLeaf.tsx:34-38` | ✅ FIXED | Created `store/selectors/pane-leaf.ts` with granular selectors |
| 10 | Timeline selectors missing shallow comparison | `lib/timeline/selectors.ts` | ✅ FIXED | Added shallow comparison with `arraysShallowEqual` |
| 11 | SessionBrowser filter no debouncing | `SessionBrowser.tsx` | ✅ FIXED | Added `useDeferredValue` for search |
| 12 | SessionBrowser messages lacks virtualization | `SessionBrowser.tsx` | ✅ FIXED | Added `VirtualizedMessagesList` component |
| 13 | AgentMessage cascading memoization chain | `AgentMessage.tsx:164-205` | ✅ FIXED | Consolidated into single `derivedState` useMemo |
| 14 | InputStatusRow duplicated settings logic | `InputStatusRow.tsx` | ✅ FIXED | Extracted `useProviderSettings` and `useModelSwitch` hooks |
| 15 | UnifiedInput stateRef pattern | `UnifiedInput.tsx` | ✅ FIXED | Created `store/selectors/unified-input.ts` |
| 16 | **UnifiedInput component too large** | `UnifiedInput.tsx` | ⚠️ PLANNED | REFACTOR_PLAN.md created, not yet implemented |
| 17 | Missing lazy loading in PaneLeaf | `PaneLeaf.tsx` | ✅ FIXED | Added lazy imports for HomeView, SettingsTabContent |
| 18 | Markdown loads SyntaxHighlighter synchronously | `Markdown.tsx` | ✅ FIXED | Added lazy loading with Suspense fallback |
| 19 | Settings loaded multiple times | Multiple files | ✅ FIXED | Added caching in `lib/settings.ts` with invalidation |
| 20 | Git status polling without deduplication | `useTauriEvents.ts` | ✅ FIXED | Added sequence number tracking |
| 21 | Waterfall requests in useCreateTerminalTab | `useCreateTerminalTab.ts` | ✅ FIXED | Parallelized PTY creation + settings fetch |
| 22 | HomeView data fetched on every focus | `HomeView.tsx` | ✅ FIXED | Added debouncing with 500ms delay |
| 23 | CodeMirror languages loaded eagerly | `FileEditorSidebarPanel.tsx` | ✅ FIXED | Created `lib/codemirror-languages.ts` with dynamic imports |
| 24 | Missing Vite manual chunks config | `vite.config.ts` | ✅ FIXED | Added manual chunks for vendor splitting |
| 25 | mocks.ts potentially in production | `App.tsx:71`, `mocks.ts` | ✅ FIXED | Extracted `isMockBrowserMode` to separate module for tree-shaking |
| 26 | WebGL addon no error recovery | `Terminal.tsx` | ✅ FIXED | Added context loss/restore handlers |
| 27 | Theme changes trigger multiple re-renders | `ThemeManager.ts` | ✅ FIXED | Batched terminal option updates |
| 28 | LiveTerminalBlock inline style | `LiveTerminalBlock.tsx` | ✅ FIXED | Moved style to constant |
| 29 | HomeView ProjectRow/RecentDirectoryRow no memo | `HomeView.tsx` | ✅ FIXED | Added React.memo to row components |
| 30 | Inline arrow functions in HomeView | `HomeView.tsx` | ✅ FIXED | Extracted callbacks with useCallback |
| 31 | SlashCommandPopup list items no memoization | `SlashCommandPopup.tsx` | ✅ FIXED | Added memoization to list items |

**Progress: 25/26 COMPLETE** (96%)

---

## MEDIUM Priority

| # | Issue | Location | Status | Notes |
|---|-------|----------|--------|-------|
| 32 | `listenForSettingsUpdates` no cleanup | `systemNotifications.ts:255-268` | ❌ TODO | |
| 33 | Terminal parking lot DOM accumulation | `TerminalInstanceManager.ts:29-51` | ❌ TODO | |
| 34 | Async listener race condition in Terminal | `Terminal.tsx:234-287` | ❌ TODO | |
| 35 | Tauri event cleanup promises not awaited | `useTauriEvents.ts:488-491` | ❌ TODO | |
| 36 | `updateAgentStreaming` calls join() on every delta | `store/index.ts` | ✅ FIXED | Added `agentStreamingBuffer` array - no more join() on hot path |
| 37 | GitPanel uses three separate selectors | `GitPanel.tsx:453-456` | ❌ TODO | |
| 38 | HistorySearchPopup lacks virtualization | `HistorySearchPopup.tsx:120-146` | ❌ TODO | |
| 39 | `highlightMatch` creates React elements | `HistorySearchPopup.tsx:19-51` | ❌ TODO | |
| 40 | FileBrowser lacks virtualization | `FileBrowser.tsx:175-198` | ❌ TODO | |
| 41 | useCommandHistory potential over-fetching | `useCommandHistory.ts:42-64` | ❌ TODO | |
| 42 | SessionBrowser cascading state updates | `SessionBrowser.tsx:110-137` | ❌ TODO | |
| 43 | ContextPanel missing async cancellation | `ContextPanel.tsx:203-213` | ❌ TODO | |
| 44 | usePathCompletion API calls not debounced | `usePathCompletion.ts:16-47` | ❌ TODO | |
| 45 | UnifiedTimeline scroll handler | `UnifiedTimeline.tsx` | ✅ FIXED | Created `store/selectors/session.ts` - single combined selector |
| 46 | HomeView focus handler not cancellable | `HomeView.tsx:391-398` | ❌ TODO | |
| 47 | Missing error boundaries on streaming blocks | `UnifiedTimeline.tsx:317-398` | ❌ TODO | |
| 48 | HomeView fetches on every mount | `HomeView.tsx:387-389` | ❌ TODO | |
| 49 | Slash commands reloaded on directory change | `useSlashCommands.ts:77-79` | ❌ TODO | |
| 50 | SessionBrowser loads all 50 sessions | `SessionBrowser.tsx:99` | ❌ TODO | |
| 51 | console.log statements in production | Multiple files | ❌ TODO | |
| 52 | Settings dialog loads all tabs eagerly | `Settings/index.tsx:26-33` | ❌ TODO | |
| 53 | StaticTerminalOutput creates xterm per block | `StaticTerminalOutput.tsx:79-124` | ❌ TODO | |
| 54 | Terminal portal target causes Map recreation | `useTerminalPortal.tsx:53-60` | ❌ TODO | |
| 55 | VirtualTerminal unbounded Promise array | `VirtualTerminal.ts:42-48` | ❌ TODO | |
| 56 | ThinkingBlock components object recreated | `ThinkingBlock.tsx:62-128` | ❌ TODO | |
| 57 | InlineTaskPlan subscribes to entire session | `InlineTaskPlan.tsx:21` | ❌ TODO | |
| 58 | SessionBrowser inline functions in list | `SessionBrowser.tsx:262-309` | ❌ TODO | |
| 59 | SettingsTabContent nav onclick recreated | `SettingsTabContent.tsx:250-272` | ❌ TODO | |

**Progress: 2/28 COMPLETE** (7%)

---

## LOW Priority

| # | Issue | Location | Status | Notes |
|---|-------|----------|--------|-------|
| 60 | VirtualTerminal retention on interrupt | `VirtualTerminalManager.ts:9-11` | ❌ TODO | |
| 61 | LiveTerminalBlock missing cleanup return | `LiveTerminalBlock.tsx:15-28` | ❌ TODO | |
| 62 | RAF orphan potential in safeFit | `TerminalInstanceManager.ts:92-103` | ❌ TODO | |
| 63 | Theme listener array filter | `ThemeManager.ts:35-39` | ❌ TODO | |
| 64 | Duplicate empty array constants | `store/index.ts:1993-2024` | ❌ TODO | |
| 65 | `markTabNewActivityInDraft` iterates all | `store/index.ts:51-65` | ❌ TODO | |
| 66 | `enableMapSet()` adds overhead | `store/index.ts:44-45` | ❌ TODO | |
| 67 | HomeView row components could use memo | `HomeView.tsx:614-647` | ❌ TODO | |
| 68 | WorkflowTree nested loops | `WorkflowTree.tsx:71-225` | ❌ TODO | |
| 69 | SlashCommandPopup filter needs memoization | `SlashCommandPopup.tsx:129-136` | ❌ TODO | |
| 70 | useTauriEvents large 350-line effect | `useTauriEvents.ts:148-493` | ❌ TODO | |
| 71 | useTheme three subscription effects | `useTheme.tsx:34-80` | ❌ TODO | |
| 72 | Terminal.tsx complex 240-line effect | `Terminal.tsx:109-352` | ❌ TODO | |
| 73 | Popup duplicate click-outside logic | Multiple files | ❌ TODO | |
| 74 | TabBar computes props inline | `TabBar.tsx` | ✅ FIXED | Created `store/selectors/tab-bar.ts` with memoized state |
| 75 | Terminal listener pattern needs hook | `Terminal.tsx:234-287` | ❌ TODO | |
| 76 | GitPanel refreshes on dialog open | `GitPanel.tsx:671-675` | ❌ TODO | |
| 77 | SyncOutputBuffer timeout hardcoded | `SyncOutputBuffer.ts:20-21` | ❌ TODO | |
| 78 | TerminalInstanceManager fixed positioning | `TerminalInstanceManager.ts:31-51` | ❌ TODO | |
| 79 | Terminal disposal edge cases | `store/index.ts:1851` | ❌ TODO | |
| 80 | LiveTerminalManager creates before attach | `LiveTerminalManager.ts:39-110` | ❌ TODO | |
| 81 | Context menu inline functions | `HomeView.tsx:94-146, 296-348` | ❌ TODO | |
| 82 | StatsBadge/WorktreeBadge could be memoized | `HomeView.tsx:46-80` | ❌ TODO | |

**Progress: 1/23 COMPLETE** (4%)

---

## Summary by Category

| Priority | Total | Fixed | Remaining | % Complete |
|----------|-------|-------|-----------|------------|
| **Severe** | 5 | 5 | 0 | **100%** ✅ |
| **High** | 26 | 25 | 1 | **96%** ✅ |
| **Medium** | 28 | 2 | 26 | **7%** |
| **Low** | 23 | 1 | 22 | **4%** |
| **TOTAL** | **82** | **33** | **49** | **40%** |

---

## Completed Fixes Summary

### New Files Created
- `frontend/hooks/useThrottledResize.ts` - Throttled resize hook
- `frontend/hooks/useKeyboardHandlerContext.ts` - Keyboard handler with stable refs
- `frontend/hooks/useProviderSettings.ts` - Consolidated provider settings state
- `frontend/hooks/useModelSwitch.ts` - Extracted model switch logic
- `frontend/lib/isMockBrowser.ts` - Tiny module for tree-shaking
- `frontend/lib/codemirror-languages.ts` - Dynamic CodeMirror language imports
- `frontend/store/selectors/app.ts` - App-level granular selectors
- `frontend/store/selectors/pane-leaf.ts` - PaneLeaf granular selectors
- `frontend/store/selectors/session.ts` - Session-level combined selectors
- `frontend/store/selectors/tab-bar.ts` - TabBar granular selectors
- `frontend/store/selectors/unified-input.ts` - UnifiedInput granular selectors

### Major Architectural Improvements
1. **Store Selector Architecture** - Created granular selector system to replace full Record subscriptions
2. **Settings Caching** - Implemented caching with event-based invalidation
3. **Buffer-based Streaming** - Replaced string concatenation with array buffer for agent streaming
4. **Hook Extraction** - Split large duplicated logic into reusable hooks
5. **Lazy Loading** - Added code splitting for heavy dependencies
6. **Memoization Strategy** - Consolidated cascading useMemo calls into single-pass computations

### Performance Impact
- **Eliminated** hundreds of resize state updates per second
- **Reduced** re-renders from full Record subscriptions across multiple components
- **Improved** streaming performance with buffer-based accumulation
- **Decreased** initial bundle size with lazy loading and code splitting
- **Optimized** timeline rendering with shallow comparison and memoization

---

## Top Remaining Issues

### Critical Path
1. **#16 - UnifiedInput refactor** (1479 lines) - Plan exists, needs implementation
2. **#37 - GitPanel selectors** - Should use combined selector pattern
3. **#38-40 - Virtualization gaps** - HistorySearchPopup, FileBrowser need virtualization

### Quick Wins
- **#32** - Add settings listener cleanup
- **#44** - Debounce path completion
- **#49** - Cache slash commands
- **#51** - Remove console.log statements
- **#52** - Lazy load Settings tabs

### Performance Multipliers
- **#47** - Error boundaries on streaming blocks
- **#50** - Paginate/cache SessionBrowser sessions
- **#56-59** - Memoization passes on existing components
