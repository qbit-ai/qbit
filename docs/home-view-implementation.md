# Home View Implementation Plan

## Status: Phases 1-2 Complete

Last updated: 2026-01-28

## Overview

The Home view is a new tab type that displays:
1. **PROJECTS** - Configured codebases with expandable git branches, file stats, and warnings
2. **RECENT DIRECTORIES** - Recently opened workspaces derived from session history

The Home tab is always visible as the leftmost tab and cannot be closed.

---

# PHASE 1: UI Implementation ✅ COMPLETE

## 1.1 Type System & Store Changes ✅

**File: `frontend/store/index.ts`**

1. ✅ Extended `TabType`:
   ```ts
   export type TabType = "terminal" | "settings" | "home";
   ```

2. ✅ Added home tab state:
   ```ts
   homeTabId: string | null;
   ```

3. ✅ Added `openHomeTab()` action (singleton pattern like settings)

---

## 1.2 Backend Stub Endpoints ✅

**File: `backend/crates/qbit/src/indexer/commands.rs`**

Added placeholder commands that return empty data:
- ✅ `list_projects_for_home` - Returns `Vec<ProjectInfo>`
- ✅ `list_recent_directories` - Returns `Vec<RecentDirectory>`

Rust types defined:
```rust
pub struct ProjectInfo {
    pub path: String,
    pub name: String,
    pub branches: Vec<BranchInfo>,
    pub warnings: u32,
    pub last_activity: String,
}

pub struct BranchInfo {
    pub name: String,
    pub path: String,
    pub file_count: u32,
    pub insertions: i32,
    pub deletions: i32,
    pub last_activity: String,
}

pub struct RecentDirectory {
    pub path: String,
    pub name: String,
    pub branch: Option<String>,
    pub file_count: u32,
    pub insertions: i32,
    pub deletions: i32,
    pub last_accessed: String,
}
```

---

## 1.3 Frontend Components ✅

**File: `frontend/components/HomeView/HomeView.tsx`**

Styled to match GitHub-dark mockup with:
- ✅ `StatsBadge` - File count + additions/deletions pill
- ✅ `WorktreeBadge` - TreePine icon with worktree count
- ✅ `ProjectRow` - Expandable project rows with branches
- ✅ `RecentDirectoryRow` - Recent directory rows
- ✅ Empty states for no projects/directories
- ✅ GitHub-dark color scheme (`#0d1117`, `#161b22`, `#30363d`, etc.)

**File: `frontend/components/HomeView/SetupProjectModal.tsx`**

Modal for creating new projects with:
- ✅ Project name input
- ✅ Root path input with folder picker button
- ✅ Worktrees directory input with folder picker button
- ✅ Command inputs: test, lint, build, start
- ✅ Worktree initialization script textarea
- ✅ Cancel/Create Project buttons

**File: `frontend/lib/indexer.ts`**

Added TypeScript interfaces and invoke wrappers:
- ✅ `BranchInfo` interface
- ✅ `ProjectInfo` interface
- ✅ `RecentDirectory` interface
- ✅ `listProjectsForHome()` function
- ✅ `listRecentDirectories(limit?)` function

---

## 1.4 TabBar Modifications ✅

**File: `frontend/components/TabBar/TabBar.tsx`**

1. ✅ Home icon from lucide-react
2. ✅ Home tab sorted first in session list
3. ✅ Home tab not closable (`canClose={session.tabType !== "home"}`)
4. ✅ Home tab icon-only (no text label)
5. ✅ Tooltip shows "Home"

---

## 1.5 PaneLeaf Updates ✅

**File: `frontend/components/PaneContainer/PaneLeaf.tsx`**

Added home case to content routing:
```tsx
case "home":
  return <HomeView />;
```

---

## 1.6 App Initialization ✅

**File: `frontend/App.tsx`**

- ✅ Call `openHomeTab()` at start of initialization
- ✅ Home tab created before PTY initialization

---

# PHASE 2: Project Configuration Storage ✅ COMPLETE

## 2.1 Design

**Location**: `~/.qbit/projects/`

**File format**: One TOML file per project, named by slugified project name

**Example**: `~/.qbit/projects/my-project.toml`
```toml
name = "my-project"
root_path = "/Users/xlyk/Code/my-project"
worktrees_dir = "/Users/xlyk/Code/my-project-worktrees"

[commands]
test = "npm test"
lint = "npm run lint"
build = "npm run build"
start = "npm start"

[worktree]
init_script = """
npm install
npm run setup
"""
```

---

## 2.2 Backend: Project Storage ✅

**Module**: `backend/crates/qbit/src/projects/`

### Files:
- `schema.rs` - Rust types (ProjectConfig, ProjectCommands, WorktreeConfig)
- `storage.rs` - TOML file operations (list_projects, load_project, save_project, delete_project, slugify)
- `commands.rs` - Tauri commands with form data conversion
- `mod.rs` - Module exports

### Functions Implemented:
- ✅ `list_projects() -> Vec<ProjectConfig>` - Load all from `~/.qbit/projects/`
- ✅ `save_project(config: &ProjectConfig)` - Save to `~/.qbit/projects/{slug}.toml`
- ✅ `delete_project(name: &str)` - Remove project file
- ✅ `slugify(name: &str) -> String` - Convert name to valid filename

---

## 2.3 Backend: Tauri Commands ✅

**File**: `backend/crates/qbit/src/projects/commands.rs`

Commands implemented:
- ✅ `save_project(form: ProjectFormData)` - Save project config to disk
- ✅ `delete_project_config(name: String)` - Delete project config file
- ✅ `list_project_configs()` - List all saved project configs
- ✅ `get_project_config(name: String)` - Get single project config

Registered in `lib.rs`:
- ✅ Added to `tauri::generate_handler![]`

---

## 2.4 Backend: Update list_projects_for_home ✅

Modified `list_projects_for_home` in `indexer/commands.rs` to:
- ✅ Load project configs from `~/.qbit/projects/` (via `crate::projects::list_projects()`)
- ✅ For each project, get git stats (branch name, file count, insertions, deletions)
- ✅ Return enriched `ProjectInfo` structs

---

## 2.5 Frontend: Project API ✅

**File**: `frontend/lib/projects.ts`

Implemented:
- ✅ `ProjectFormData` interface
- ✅ `ProjectData` interface
- ✅ `saveProject()` function
- ✅ `deleteProject()` function
- ✅ `listProjectConfigs()` function
- ✅ `getProjectConfig()` function

---

## 2.6 Frontend: Wire Up SetupProjectModal ✅

**File**: `frontend/components/HomeView/HomeView.tsx`

- ✅ Import `saveProject` from `@/lib/projects`
- ✅ In `handleProjectSubmit`, call `saveProject()` with form data
- ✅ After save, refresh project list via `fetchData(false)`

## 2.7 Frontend: Refresh Functionality ✅

- ✅ Manual refresh button (with spinning animation)
- ✅ Focus-based refresh (when window regains focus)
- ✅ Refresh after project save

---

## 2.7 Frontend: Project Loading & Refresh ⏳ TODO

**File**: `frontend/components/HomeView/HomeView.tsx`

### On Mount
- [ ] Fetch projects via `listProjectsForHome()`
- [ ] Fetch recent directories via `listRecentDirectories()`

### Periodic Refresh
- [ ] Add `useEffect` with interval (e.g., every 30 seconds)
- [ ] Refresh project data to pick up git changes
- [ ] Clear interval on unmount

### Manual Refresh
- [ ] Add refresh button in header
- [ ] Call fetch functions on click

### Focus Refresh
- [ ] Listen for window focus event
- [ ] Refresh data when app regains focus

---

# PHASE 3: Interactions ⏳ TODO

## 3.1 Open Directory in New Tab

| Action | Status |
|--------|--------|
| Click project branch row | ⏳ TODO - Open new terminal in that directory |
| Click recent directory row | ⏳ TODO - Open new terminal in that directory |

Implementation:
- [ ] Add `openNewTabInDirectory(path: string)` to store
- [ ] Wire up row click handlers

---

## 3.2 Context Menu ⏳ IN PROGRESS

Right-click on project row:
- [ ] Remove from projects
- [ ] Reindex/Refresh
- [ ] Open in Finder/Explorer
- [ ] Edit project settings

Right-click on worktree row:
- [x] Delete worktree

---

## 3.3 Folder Picker ⏳ TODO

Wire up folder picker buttons in SetupProjectModal:
- [ ] Use Tauri's `dialog.open()` API
- [ ] Set selected path in form field

---

# Files Summary

## Created
- ✅ `frontend/components/HomeView/index.ts`
- ✅ `frontend/components/HomeView/HomeView.tsx`
- ✅ `frontend/components/HomeView/SetupProjectModal.tsx`
- ✅ `docs/home-view-implementation.md`

## Modified
- ✅ `frontend/store/index.ts` - TabType, homeTabId, openHomeTab()
- ✅ `frontend/components/TabBar/TabBar.tsx` - Home tab display
- ✅ `frontend/components/PaneContainer/PaneLeaf.tsx` - Route home tab
- ✅ `frontend/App.tsx` - Auto-create home tab on init
- ✅ `frontend/lib/indexer.ts` - Added invoke wrappers
- ✅ `backend/crates/qbit/src/indexer/commands.rs` - Stub commands
- ✅ `backend/crates/qbit/src/lib.rs` - Register commands

## To Create
- ⏳ `frontend/lib/projects.ts` - Project CRUD API
- ⏳ `backend/crates/qbit/src/commands/projects.rs` - Project commands

## To Modify
- ⏳ `backend/crates/qbit/src/lib.rs` - Register project commands
- ⏳ `backend/crates/qbit/src/indexer/commands.rs` - Real project loading
