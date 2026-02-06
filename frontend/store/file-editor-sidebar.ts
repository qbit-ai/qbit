import { create } from "zustand";
import { createJSONStorage, persist } from "zustand/middleware";
import { immer } from "zustand/middleware/immer";

// Tab types
export type TabType = "file" | "browser";

export interface EditorFileState {
  path: string;
  content: string;
  originalContent: string;
  language?: string;
  dirty: boolean;
  markdownPreview: boolean;
  lastReadAt?: string;
  lastSavedAt?: string;
  externallyModified?: boolean;
}

export interface FileBrowserState {
  currentPath: string;
  // Could add more state like selected items, view mode, etc.
}

// Base tab interface
interface BaseTab {
  id: string;
  type: TabType;
}

export interface FileTab extends BaseTab {
  type: "file";
  file: EditorFileState;
}

export interface BrowserTab extends BaseTab {
  type: "browser";
  browser: FileBrowserState;
}

export type Tab = FileTab | BrowserTab;

// Single global state (no longer per-session)
interface FileEditorSidebarState {
  open: boolean;
  width: number;
  // Tab-based model
  tabs: Record<string, Tab>; // keyed by tab id
  activeTabId: string | null;
  tabOrder: string[]; // ordered list of tab ids
  // Recent files for quick access
  recentFiles: string[];
  // Editor settings (shared across file tabs)
  vimMode: boolean;
  vimModeState: "normal" | "insert" | "visual";
  wrap: boolean;
  lineNumbers: boolean;
  relativeLineNumbers: boolean;
  status?: string;

  // Actions
  setOpen: (open: boolean) => void;
  setWidth: (width: number) => void;
  setStatus: (status?: string) => void;
  // Tab operations
  openFileTab: (file: EditorFileState) => void;
  openBrowserTab: (initialPath?: string) => void;
  setActiveTab: (tabId: string) => void;
  closeTab: (tabId?: string) => void;
  closeAllTabs: () => void;
  closeOtherTabs: (keepTabId: string) => void;
  reorderTabs: (fromIndex: number, toIndex: number) => void;
  // File tab specific
  updateFileContent: (tabId: string, content: string) => void;
  markFileSaved: (tabId: string, timestamp?: string) => void;
  toggleMarkdownPreview: (tabId: string) => void;
  // Browser tab specific
  setBrowserPath: (tabId: string, path: string) => void;
  // Editor settings
  setVimMode: (enabled: boolean) => void;
  setVimModeState: (state: "normal" | "insert" | "visual") => void;
  setWrap: (enabled: boolean) => void;
  setLineNumbers: (enabled: boolean) => void;
  setRelativeLineNumbers: (enabled: boolean) => void;
  addRecentFile: (path: string) => void;
  // External change handling
  markExternallyModified: (tabId: string) => void;
  acceptExternalChange: (tabId: string, content: string, modifiedAt?: string) => void;
  keepLocalVersion: (tabId: string) => void;
  reset: () => void;
}

const DEFAULT_WIDTH = 420;

// Generate unique tab IDs
let tabIdCounter = 0;
function generateTabId(type: TabType): string {
  return `${type}-${Date.now()}-${++tabIdCounter}`;
}

// For file tabs, use path as a stable identifier to detect duplicates
function getFileTabId(path: string): string {
  return `file:${path}`;
}

// Export for use in hook
export function fileTabIdFromPath(path: string): string {
  return getFileTabId(path);
}

const initialState = {
  open: false,
  width: DEFAULT_WIDTH,
  tabs: {} as Record<string, Tab>,
  activeTabId: null as string | null,
  tabOrder: [] as string[],
  recentFiles: [] as string[],
  vimMode: true,
  vimModeState: "normal" as const,
  wrap: false,
  lineNumbers: true,
  relativeLineNumbers: false,
  status: undefined as string | undefined,
};

export const useFileEditorSidebarStore = create<FileEditorSidebarState>()(
  persist(
    immer((set) => ({
      ...initialState,

      setOpen: (open) => {
        set((draft) => {
          draft.open = open;
        });
      },

      setWidth: (width) => {
        set((draft) => {
          draft.width = width;
        });
      },

      setStatus: (status) => {
        set((draft) => {
          draft.status = status;
        });
      },

      openFileTab: (file) => {
        set((draft) => {
          const tabId = getFileTabId(file.path);

          // Check if file is already open
          if (draft.tabs[tabId]) {
            // Just switch to it
            draft.activeTabId = tabId;
          } else {
            // Create new file tab
            const tab: FileTab = {
              id: tabId,
              type: "file",
              file,
            };
            draft.tabs[tabId] = tab;
            draft.tabOrder.push(tabId);
            draft.activeTabId = tabId;
          }

          // Auto-open sidebar and add to recent
          draft.open = true;
          draft.recentFiles = [
            file.path,
            ...draft.recentFiles.filter((p) => p !== file.path),
          ].slice(0, 10);
        });
      },

      openBrowserTab: (initialPath) => {
        set((draft) => {
          const tabId = generateTabId("browser");

          const tab: BrowserTab = {
            id: tabId,
            type: "browser",
            browser: {
              currentPath: initialPath ?? "",
            },
          };

          draft.tabs[tabId] = tab;
          draft.tabOrder.push(tabId);
          draft.activeTabId = tabId;
          draft.open = true;
        });
      },

      setActiveTab: (tabId) => {
        set((draft) => {
          if (draft.tabs[tabId]) {
            draft.activeTabId = tabId;
          }
        });
      },

      closeTab: (tabId) => {
        set((draft) => {
          const targetId = tabId ?? draft.activeTabId;
          if (!targetId) return;

          // Remove tab
          delete draft.tabs[targetId];

          // Remove from order
          const tabIndex = draft.tabOrder.indexOf(targetId);
          if (tabIndex !== -1) {
            draft.tabOrder.splice(tabIndex, 1);
          }

          // Update active tab
          if (draft.activeTabId === targetId) {
            if (draft.tabOrder.length === 0) {
              draft.activeTabId = null;
            } else {
              const newIndex = Math.min(tabIndex, draft.tabOrder.length - 1);
              draft.activeTabId = draft.tabOrder[newIndex] ?? null;
            }
          }
        });
      },

      closeAllTabs: () => {
        set((draft) => {
          draft.tabs = {};
          draft.tabOrder = [];
          draft.activeTabId = null;
        });
      },

      closeOtherTabs: (keepTabId) => {
        set((draft) => {
          const tabToKeep = draft.tabs[keepTabId];
          if (!tabToKeep) return;
          draft.tabs = { [keepTabId]: tabToKeep };
          draft.tabOrder = [keepTabId];
          draft.activeTabId = keepTabId;
        });
      },

      reorderTabs: (fromIndex, toIndex) => {
        set((draft) => {
          if (fromIndex < 0 || fromIndex >= draft.tabOrder.length) return;
          if (toIndex < 0 || toIndex >= draft.tabOrder.length) return;
          const [removed] = draft.tabOrder.splice(fromIndex, 1);
          if (removed) {
            draft.tabOrder.splice(toIndex, 0, removed);
          }
        });
      },

      updateFileContent: (tabId, content) => {
        set((draft) => {
          const tab = draft.tabs[tabId];
          if (!tab || tab.type !== "file") return;
          tab.file.content = content;
          tab.file.dirty = content !== tab.file.originalContent;
        });
      },

      markFileSaved: (tabId, timestamp) => {
        set((draft) => {
          const tab = draft.tabs[tabId];
          if (!tab || tab.type !== "file") return;
          tab.file.dirty = false;
          tab.file.originalContent = tab.file.content;
          tab.file.lastSavedAt = timestamp ?? new Date().toISOString();
        });
      },

      toggleMarkdownPreview: (tabId) => {
        set((draft) => {
          const tab = draft.tabs[tabId];
          if (!tab || tab.type !== "file") return;
          if (tab.file.language !== "markdown") return;
          tab.file.markdownPreview = !tab.file.markdownPreview;
        });
      },

      setBrowserPath: (tabId, path) => {
        set((draft) => {
          const tab = draft.tabs[tabId];
          if (!tab || tab.type !== "browser") return;
          tab.browser.currentPath = path;
        });
      },

      setVimMode: (enabled) => {
        set((draft) => {
          draft.vimMode = enabled;
        });
      },

      setVimModeState: (state) => {
        set((draft) => {
          draft.vimModeState = state;
        });
      },

      setWrap: (enabled) => {
        set((draft) => {
          draft.wrap = enabled;
        });
      },

      setLineNumbers: (enabled) => {
        set((draft) => {
          draft.lineNumbers = enabled;
        });
      },

      setRelativeLineNumbers: (enabled) => {
        set((draft) => {
          draft.relativeLineNumbers = enabled;
        });
      },

      addRecentFile: (path) => {
        set((draft) => {
          const existing = draft.recentFiles.filter((p) => p !== path);
          draft.recentFiles = [path, ...existing].slice(0, 10);
        });
      },

      markExternallyModified: (tabId) => {
        set((draft) => {
          const tab = draft.tabs[tabId];
          if (!tab || tab.type !== "file") return;
          tab.file.externallyModified = true;
        });
      },

      acceptExternalChange: (tabId, content, modifiedAt) => {
        set((draft) => {
          const tab = draft.tabs[tabId];
          if (!tab || tab.type !== "file") return;
          tab.file.content = content;
          tab.file.originalContent = content;
          tab.file.dirty = false;
          tab.file.externallyModified = false;
          tab.file.lastReadAt = new Date().toISOString();
          if (modifiedAt) {
            tab.file.lastSavedAt = modifiedAt;
          }
        });
      },

      keepLocalVersion: (tabId) => {
        set((draft) => {
          const tab = draft.tabs[tabId];
          if (!tab || tab.type !== "file") return;
          tab.file.externallyModified = false;
        });
      },

      reset: () => {
        set(() => ({ ...initialState }));
      },
    })),
    {
      name: "file-editor-sidebar-storage",
      storage: createJSONStorage(() => localStorage),
      partialize: (state) => ({
        width: state.width,
        recentFiles: state.recentFiles,
        vimMode: state.vimMode,
        wrap: state.wrap,
        lineNumbers: state.lineNumbers,
        relativeLineNumbers: state.relativeLineNumbers,
      }),
    }
  )
);

// Helper to get active tab
export function selectActiveTab(state: FileEditorSidebarState): Tab | null {
  if (!state.activeTabId) return null;
  return state.tabs[state.activeTabId] ?? null;
}

// Helper to get active file tab (if active tab is a file)
export function selectActiveFileTab(state: FileEditorSidebarState): FileTab | null {
  const tab = selectActiveTab(state);
  if (!tab || tab.type !== "file") return null;
  return tab;
}

// Helper to check if a tab is dirty
export function isTabDirty(tab: Tab): boolean {
  return tab.type === "file" && tab.file.dirty;
}

// Helper to get tab display name
export function getTabDisplayName(tab: Tab): string {
  if (tab.type === "file") {
    const parts = tab.file.path.split("/");
    return parts[parts.length - 1] ?? tab.file.path;
  }
  if (tab.type === "browser") {
    if (!tab.browser.currentPath) return "Browser";
    const parts = tab.browser.currentPath.split("/");
    return parts[parts.length - 1] ?? "Browser";
  }
  return "Unknown";
}
