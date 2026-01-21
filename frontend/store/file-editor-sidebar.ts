import { create } from "zustand";
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

export interface FileEditorSessionState {
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
}

interface FileEditorSidebarState {
  sessions: Record<string, FileEditorSessionState>;
  ensureSession: (sessionId: string) => FileEditorSessionState;
  setOpen: (sessionId: string, open: boolean) => void;
  setWidth: (sessionId: string, width: number) => void;
  setStatus: (sessionId: string, status?: string) => void;
  // Tab operations
  openFileTab: (sessionId: string, file: EditorFileState) => void;
  openBrowserTab: (sessionId: string, initialPath?: string) => void;
  setActiveTab: (sessionId: string, tabId: string) => void;
  closeTab: (sessionId: string, tabId?: string) => void;
  closeAllTabs: (sessionId: string) => void;
  closeOtherTabs: (sessionId: string, keepTabId: string) => void;
  reorderTabs: (sessionId: string, fromIndex: number, toIndex: number) => void;
  // File tab specific
  updateFileContent: (sessionId: string, tabId: string, content: string) => void;
  markFileSaved: (sessionId: string, tabId: string, timestamp?: string) => void;
  toggleMarkdownPreview: (sessionId: string, tabId: string) => void;
  // Browser tab specific
  setBrowserPath: (sessionId: string, tabId: string, path: string) => void;
  // Editor settings
  setVimMode: (sessionId: string, enabled: boolean) => void;
  setVimModeState: (sessionId: string, state: "normal" | "insert" | "visual") => void;
  setWrap: (sessionId: string, enabled: boolean) => void;
  setLineNumbers: (sessionId: string, enabled: boolean) => void;
  setRelativeLineNumbers: (sessionId: string, enabled: boolean) => void;
  addRecentFile: (sessionId: string, path: string) => void;
  resetSession: (sessionId: string) => void;
}

const DEFAULT_WIDTH = 420;

function createDefaultSessionState(): FileEditorSessionState {
  return {
    open: false,
    width: DEFAULT_WIDTH,
    tabs: {},
    activeTabId: null,
    tabOrder: [],
    recentFiles: [],
    vimMode: true,
    vimModeState: "normal",
    wrap: false,
    lineNumbers: true,
    relativeLineNumbers: false,
    status: undefined,
  };
}

// Generate unique tab IDs
let tabIdCounter = 0;
function generateTabId(type: TabType): string {
  return `${type}-${Date.now()}-${++tabIdCounter}`;
}

// For file tabs, use path as a stable identifier to detect duplicates
function getFileTabId(path: string): string {
  return `file:${path}`;
}

export const useFileEditorSidebarStore = create<FileEditorSidebarState>()(
  immer((set, get) => ({
    sessions: {},

    ensureSession: (sessionId) => {
      const state = get();
      if (!state.sessions[sessionId]) {
        set((draft) => {
          draft.sessions[sessionId] = createDefaultSessionState();
        });
      }
      return get().sessions[sessionId] ?? createDefaultSessionState();
    },

    setOpen: (sessionId, open) => {
      set((draft) => {
        const session = draft.sessions[sessionId] ?? createDefaultSessionState();
        draft.sessions[sessionId] = { ...session, open };
      });
    },

    setWidth: (sessionId, width) => {
      set((draft) => {
        const session = draft.sessions[sessionId] ?? createDefaultSessionState();
        draft.sessions[sessionId] = { ...session, width };
      });
    },

    setStatus: (sessionId, status) => {
      set((draft) => {
        const session = draft.sessions[sessionId] ?? createDefaultSessionState();
        draft.sessions[sessionId] = { ...session, status };
      });
    },

    openFileTab: (sessionId, file) => {
      set((draft) => {
        const session = draft.sessions[sessionId] ?? createDefaultSessionState();
        const tabId = getFileTabId(file.path);

        // Check if file is already open
        if (session.tabs[tabId]) {
          // Just switch to it
          session.activeTabId = tabId;
        } else {
          // Create new file tab
          const tab: FileTab = {
            id: tabId,
            type: "file",
            file,
          };
          session.tabs[tabId] = tab;
          session.tabOrder.push(tabId);
          session.activeTabId = tabId;
        }

        // Auto-open sidebar and add to recent
        session.open = true;
        session.recentFiles = [
          file.path,
          ...session.recentFiles.filter((p) => p !== file.path),
        ].slice(0, 10);

        draft.sessions[sessionId] = session;
      });
    },

    openBrowserTab: (sessionId, initialPath) => {
      set((draft) => {
        const session = draft.sessions[sessionId] ?? createDefaultSessionState();
        const tabId = generateTabId("browser");

        const tab: BrowserTab = {
          id: tabId,
          type: "browser",
          browser: {
            currentPath: initialPath ?? "",
          },
        };

        session.tabs[tabId] = tab;
        session.tabOrder.push(tabId);
        session.activeTabId = tabId;
        session.open = true;

        draft.sessions[sessionId] = session;
      });
    },

    setActiveTab: (sessionId, tabId) => {
      set((draft) => {
        const session = draft.sessions[sessionId];
        if (!session) return;
        if (session.tabs[tabId]) {
          session.activeTabId = tabId;
        }
      });
    },

    closeTab: (sessionId, tabId) => {
      set((draft) => {
        const session = draft.sessions[sessionId];
        if (!session) return;

        const targetId = tabId ?? session.activeTabId;
        if (!targetId) return;

        // Remove tab
        delete session.tabs[targetId];

        // Remove from order
        const tabIndex = session.tabOrder.indexOf(targetId);
        if (tabIndex !== -1) {
          session.tabOrder.splice(tabIndex, 1);
        }

        // Update active tab
        if (session.activeTabId === targetId) {
          if (session.tabOrder.length === 0) {
            session.activeTabId = null;
          } else {
            const newIndex = Math.min(tabIndex, session.tabOrder.length - 1);
            session.activeTabId = session.tabOrder[newIndex] ?? null;
          }
        }
      });
    },

    closeAllTabs: (sessionId) => {
      set((draft) => {
        const session = draft.sessions[sessionId];
        if (!session) return;
        session.tabs = {};
        session.tabOrder = [];
        session.activeTabId = null;
      });
    },

    closeOtherTabs: (sessionId, keepTabId) => {
      set((draft) => {
        const session = draft.sessions[sessionId];
        if (!session) return;
        const tabToKeep = session.tabs[keepTabId];
        if (!tabToKeep) return;
        session.tabs = { [keepTabId]: tabToKeep };
        session.tabOrder = [keepTabId];
        session.activeTabId = keepTabId;
      });
    },

    reorderTabs: (sessionId, fromIndex, toIndex) => {
      set((draft) => {
        const session = draft.sessions[sessionId];
        if (!session) return;
        if (fromIndex < 0 || fromIndex >= session.tabOrder.length) return;
        if (toIndex < 0 || toIndex >= session.tabOrder.length) return;
        const [removed] = session.tabOrder.splice(fromIndex, 1);
        if (removed) {
          session.tabOrder.splice(toIndex, 0, removed);
        }
      });
    },

    updateFileContent: (sessionId, tabId, content) => {
      set((draft) => {
        const session = draft.sessions[sessionId];
        if (!session) return;
        const tab = session.tabs[tabId];
        if (!tab || tab.type !== "file") return;
        tab.file.content = content;
        tab.file.dirty = content !== tab.file.originalContent;
      });
    },

    markFileSaved: (sessionId, tabId, timestamp) => {
      set((draft) => {
        const session = draft.sessions[sessionId];
        if (!session) return;
        const tab = session.tabs[tabId];
        if (!tab || tab.type !== "file") return;
        tab.file.dirty = false;
        tab.file.originalContent = tab.file.content;
        tab.file.lastSavedAt = timestamp ?? new Date().toISOString();
      });
    },

    toggleMarkdownPreview: (sessionId, tabId) => {
      set((draft) => {
        const session = draft.sessions[sessionId];
        if (!session) return;
        const tab = session.tabs[tabId];
        if (!tab || tab.type !== "file") return;
        if (tab.file.language !== "markdown") return;
        tab.file.markdownPreview = !tab.file.markdownPreview;
      });
    },

    setBrowserPath: (sessionId, tabId, path) => {
      set((draft) => {
        const session = draft.sessions[sessionId];
        if (!session) return;
        const tab = session.tabs[tabId];
        if (!tab || tab.type !== "browser") return;
        tab.browser.currentPath = path;
      });
    },

    setVimMode: (sessionId, enabled) => {
      set((draft) => {
        const session = draft.sessions[sessionId] ?? createDefaultSessionState();
        draft.sessions[sessionId] = { ...session, vimMode: enabled };
      });
    },

    setVimModeState: (sessionId, state) => {
      set((draft) => {
        const session = draft.sessions[sessionId] ?? createDefaultSessionState();
        draft.sessions[sessionId] = { ...session, vimModeState: state };
      });
    },

    setWrap: (sessionId, enabled) => {
      set((draft) => {
        const session = draft.sessions[sessionId] ?? createDefaultSessionState();
        draft.sessions[sessionId] = { ...session, wrap: enabled };
      });
    },

    setLineNumbers: (sessionId, enabled) => {
      set((draft) => {
        const session = draft.sessions[sessionId] ?? createDefaultSessionState();
        draft.sessions[sessionId] = { ...session, lineNumbers: enabled };
      });
    },

    setRelativeLineNumbers: (sessionId, enabled) => {
      set((draft) => {
        const session = draft.sessions[sessionId] ?? createDefaultSessionState();
        draft.sessions[sessionId] = { ...session, relativeLineNumbers: enabled };
      });
    },

    addRecentFile: (sessionId, path) => {
      set((draft) => {
        const session = draft.sessions[sessionId] ?? createDefaultSessionState();
        const existing = session.recentFiles.filter((p) => p !== path);
        draft.sessions[sessionId] = {
          ...session,
          recentFiles: [path, ...existing].slice(0, 10),
        };
      });
    },

    resetSession: (sessionId) => {
      set((draft) => {
        draft.sessions[sessionId] = createDefaultSessionState();
      });
    },
  }))
);

// Cached default state to avoid creating new objects on every selector call
const DEFAULT_SESSION_STATE: FileEditorSessionState = Object.freeze({
  ...createDefaultSessionState(),
  tabs: Object.freeze({}) as unknown as Record<string, Tab>,
  tabOrder: Object.freeze([]) as unknown as string[],
  recentFiles: Object.freeze([]) as unknown as string[],
});

export function selectSessionState(state: FileEditorSidebarState, sessionId: string) {
  return state.sessions[sessionId] ?? DEFAULT_SESSION_STATE;
}

// Helper to get active tab from session
export function selectActiveTab(session: FileEditorSessionState): Tab | null {
  if (!session.activeTabId) return null;
  return session.tabs[session.activeTabId] ?? null;
}

// Helper to get active file tab (if active tab is a file)
export function selectActiveFileTab(session: FileEditorSessionState): FileTab | null {
  const tab = selectActiveTab(session);
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

// Helper to generate stable file tab ID from path
export function fileTabIdFromPath(path: string): string {
  return `file:${path}`;
}
