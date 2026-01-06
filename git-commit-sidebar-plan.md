# Git Changes & Commit Composer Sidebar — Plan

Context: Add a right-hand sidebar (mirroring the Task Plan panel) to manage pending git changes and compose commits. Draw UX cues from VS Code Source Control, Zed Git panel, and GitKraken.

## Assumptions
- Git CLI available in the working directory for each session (same source as current branch lookup).
- One repository per session’s working directory (no multi-repo aggregation in this iteration).
- Right panels remain mutually exclusive (Context/Task Planner/new Git panel) to preserve layout simplicity.

## Step-by-step plan
1) **Panel shell & entry points**
   - Add a Git sidebar component rendered on the right with the same resizable handle pattern as TaskPlannerPanel.
   - Provide a toggle hotkey (e.g., Cmd/Ctrl+Shift+G) and a status bar entry showing change counts; ensure panel exclusivity with other right panels.
   - Persist last open/closed state per session (or globally) for convenience.

2) **State model & store wiring**
   - Extend the store to track per-session git state: repo status (branch, ahead/behind), lists of unstaged/staged/untracked/conflicted files with change kinds, in-flight operations, and commit draft text.
   - Include timestamps/error states for the last refresh to surface failures (e.g., repo dirty, detached HEAD, not a repo).

3) **Backend git commands (Tauri)**
   - Implement commands for: `status` (porcelain with branch + ahead/behind), `diff` (per file, optionally by hunk), `stage/unstage` (file or hunk), `discard` (file/hunk), `commit` (message + amend + sign-off), `stash` (save/apply/drop), and `show`/`log` for recent commits when needed (for amend selection).
   - Normalize outputs into structured JSON (status entries with path, change type, staged flag; diff hunks with headers and line ranges) to minimize parsing on the frontend.
   - Add guards for large outputs and return hints when pagination is needed.

4) **Frontend data layer**
   - Create a git client module that wraps the new Tauri commands with typed helpers and error normalization.
   - Add a React hook (`useGitState(sessionId)`) that loads status on open, supports manual refresh, and exposes actions (stage/unstage, discard, commit, stash). Debounce/poll when terminal commands change the tree (optional future improvement: listen to FS events).

5) **Working tree list UI**
   - Layout similar to VS Code/Zed: sections for *Staged*, *Unstaged*, *Untracked*, *Conflicts* with counts and collapse controls.
   - Each file row shows icon + filename + path, change badges (M/A/D/R/U), and inline actions: stage/unstage toggle, discard, open file, view diff.
   - Support multi-select with bulk actions (stage/unstage/discard) and a refresh button at the top.

6) **Diff preview & hunk actions**
   - Inline diff preview panel within the sidebar or as an expandable area per file, showing colored additions/removals and line numbers.
   - Provide hunk-level controls (stage/unstage/discard hunk) and “Open in editor” to jump to the file at the hunk start.
   - Handle binary/large files with a fallback message and an “open externally” action.

7) **Commit composer**
   - Text area for commit subject/body with character guides (50/72), template placeholders, and a sign-off toggle.
   - Buttons for *Commit*, *Commit & Push* (if remote info available), *Amend last commit*, and *Stage all & commit*.
   - Optional AI assist hook (future): generate commit message from current diff, with editable preview.

8) **Conflict/stash flows**
   - Highlight conflicted files with a badge and quick links to open merge tools (external) or view conflict hunks.
   - Provide stash create/apply/drop with notes; guard against dirty index when applying.

9) **Empty/non-git states & errors**
   - Show friendly empty states for non-repo directories, clean working trees, and offline git binaries.
   - Surface errors inline with retry actions; log details to the console for debugging.

10) **Integration points & layout polish**
    - Ensure right-panel exclusivity logic in `App` matches existing Task Planner/Context behaviors.
    - Add a StatusBar capsule showing counts (staged/unstaged/conflicts) that opens the panel, similar to the Task Plan indicator.
    - Match theming/spacing of TaskPlannerPanel (header, footer kbd hint, scroll area, resize handle).

11) **Telemetry & performance**
    - Add lightweight timing/operation counters around git commands for troubleshooting.
    - Cap status/diff size and paginate when necessary to keep the UI responsive on large repos.

12) **Verification (to run after implementation)**
    - `pnpm lint`
    - `pnpm typecheck`
    - `pnpm test:run`
    - `pnpm build`
    - Manual smoke: open Git sidebar in repo and non-repo, stage/unstage/discard, view diffs, commit (with amend), stash apply/drop, toggle panel/hotkey, resize behavior.
