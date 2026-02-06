import { useCallback, useMemo } from "react";
import { readWorkspaceFile, writeWorkspaceFile } from "@/lib/file-editor";
import { notify } from "@/lib/notify";
import {
  type EditorFileState,
  fileTabIdFromPath,
  selectActiveFileTab,
  selectActiveTab,
  type Tab,
  useFileEditorSidebarStore,
} from "@/store/file-editor-sidebar";

function resolvePath(input: string, workingDirectory?: string) {
  if (!workingDirectory) return input;
  if (input.startsWith("/") || /^\w:[/\\]/.test(input)) return input;
  const trimmedBase = workingDirectory.endsWith("/")
    ? workingDirectory.slice(0, -1)
    : workingDirectory;
  const normalizedInput = input.replace(/^\.\//, "");
  return `${trimmedBase}/${normalizedInput}`;
}

function detectLanguageFromPath(path: string): string | undefined {
  const lower = path.toLowerCase();

  if (lower.endsWith(".ts") || lower.endsWith(".tsx")) return "typescript";
  if (lower.endsWith(".js") || lower.endsWith(".jsx")) return "javascript";
  if (lower.endsWith(".json")) return "json";
  if (lower.endsWith(".md") || lower.endsWith(".mdx")) return "markdown";

  if (lower.endsWith(".py")) return "python";
  if (lower.endsWith(".rs")) return "rust";
  if (lower.endsWith(".go")) return "go";

  if (lower.endsWith(".toml")) return "toml";
  if (lower.endsWith(".yml") || lower.endsWith(".yaml")) return "yaml";

  if (lower.endsWith(".html") || lower.endsWith(".htm")) return "html";
  if (lower.endsWith(".css") || lower.endsWith(".scss") || lower.endsWith(".less")) return "css";
  if (lower.endsWith(".sql")) return "sql";

  if (lower.endsWith(".xml")) return "xml";
  if (lower.endsWith(".java")) return "java";
  if (
    lower.endsWith(".c") ||
    lower.endsWith(".h") ||
    lower.endsWith(".cpp") ||
    lower.endsWith(".hpp")
  )
    return "cpp";

  return undefined;
}

export function useFileEditorSidebar(workingDirectory?: string) {
  // Subscribe to the global store state
  const state = useFileEditorSidebarStore();

  // Derive active tab and active file from state
  const activeTab = useMemo(() => selectActiveTab(state), [state]);
  const activeFileTab = useMemo(() => selectActiveFileTab(state), [state]);
  const activeFile = activeFileTab?.file ?? null;

  const actions = useMemo(() => {
    return {
      setOpen: (open: boolean) => {
        useFileEditorSidebarStore.getState().setOpen(open);
      },
      setWidth: (width: number) => {
        useFileEditorSidebarStore.getState().setWidth(width);
      },
      setStatus: (status?: string) => {
        useFileEditorSidebarStore.getState().setStatus(status);
      },
      setActiveTab: (tabId: string) => {
        useFileEditorSidebarStore.getState().setActiveTab(tabId);
      },
      closeTab: (tabId?: string) => {
        useFileEditorSidebarStore.getState().closeTab(tabId);
      },
      closeAllTabs: () => {
        useFileEditorSidebarStore.getState().closeAllTabs();
      },
      closeOtherTabs: (keepTabId: string) => {
        useFileEditorSidebarStore.getState().closeOtherTabs(keepTabId);
      },
      reorderTabs: (fromIndex: number, toIndex: number) => {
        useFileEditorSidebarStore.getState().reorderTabs(fromIndex, toIndex);
      },
      updateFileContent: (tabId: string, content: string) => {
        useFileEditorSidebarStore.getState().updateFileContent(tabId, content);
      },
      setBrowserPath: (tabId: string, path: string) => {
        useFileEditorSidebarStore.getState().setBrowserPath(tabId, path);
      },
      setVimMode: (enabled: boolean) => {
        useFileEditorSidebarStore.getState().setVimMode(enabled);
      },
      setVimModeState: (state: "normal" | "insert" | "visual") => {
        useFileEditorSidebarStore.getState().setVimModeState(state);
      },
      setWrap: (enabled: boolean) => {
        useFileEditorSidebarStore.getState().setWrap(enabled);
      },
      setLineNumbers: (enabled: boolean) => {
        useFileEditorSidebarStore.getState().setLineNumbers(enabled);
      },
      setRelativeLineNumbers: (enabled: boolean) => {
        useFileEditorSidebarStore.getState().setRelativeLineNumbers(enabled);
      },
      setShowHiddenFiles: (enabled: boolean) => {
        useFileEditorSidebarStore.getState().setShowHiddenFiles(enabled);
      },
      addRecentFile: (path: string) => {
        useFileEditorSidebarStore.getState().addRecentFile(path);
      },
      toggleMarkdownPreview: (tabId: string) => {
        useFileEditorSidebarStore.getState().toggleMarkdownPreview(tabId);
      },
    };
  }, []);

  const openFile = useCallback(
    async (inputPath: string) => {
      const fullPath = resolvePath(inputPath, workingDirectory);

      // If file is already open, just switch to it
      const tabId = fileTabIdFromPath(fullPath);
      const currentState = useFileEditorSidebarStore.getState();
      if (currentState.tabs[tabId]) {
        currentState.setActiveTab(tabId);
        return;
      }

      actions.setStatus("Loading file...");
      try {
        const result = await readWorkspaceFile(fullPath);
        const file: EditorFileState = {
          path: fullPath,
          content: result.content,
          originalContent: result.content,
          language: detectLanguageFromPath(fullPath),
          dirty: false,
          markdownPreview: false,
          lastReadAt: new Date().toISOString(),
          lastSavedAt: result.modifiedAt,
        };
        useFileEditorSidebarStore.getState().openFileTab(file);
        actions.addRecentFile(fullPath);
      } catch (error) {
        notify.error(`Failed to open file: ${error}`);
      } finally {
        actions.setStatus(undefined);
      }
    },
    [actions, workingDirectory]
  );

  const openBrowser = useCallback(
    (initialPath?: string) => {
      const path = initialPath ?? workingDirectory ?? "";
      useFileEditorSidebarStore.getState().openBrowserTab(path);
    },
    [workingDirectory]
  );

  const saveFile = useCallback(
    async (tabId?: string) => {
      const currentState = useFileEditorSidebarStore.getState();
      const targetTabId = tabId ?? currentState.activeTabId;
      if (!targetTabId) {
        notify.info("No file open to save");
        return;
      }

      const tab = currentState.tabs[targetTabId];
      if (!tab || tab.type !== "file") {
        notify.info("Current tab is not a file");
        return;
      }

      const file = tab.file;
      actions.setStatus("Saving...");
      try {
        const result = await writeWorkspaceFile(file.path, file.content, {
          expectedModifiedAt: file.lastSavedAt,
        });
        useFileEditorSidebarStore.getState().markFileSaved(targetTabId, result.modifiedAt);
        notify.success("Saved");
      } catch (error) {
        notify.error(`Failed to save file: ${error}`);
      } finally {
        actions.setStatus(undefined);
      }
    },
    [actions]
  );

  const reloadFile = useCallback(
    async (tabId?: string) => {
      const currentState = useFileEditorSidebarStore.getState();
      const targetTabId = tabId ?? currentState.activeTabId;
      if (!targetTabId) {
        notify.info("No file open to reload");
        return;
      }

      const tab = currentState.tabs[targetTabId];
      if (!tab || tab.type !== "file") {
        notify.info("Current tab is not a file");
        return;
      }

      const file = tab.file;
      actions.setStatus("Reloading...");
      try {
        const result = await readWorkspaceFile(file.path);
        const newFile: EditorFileState = {
          ...file,
          content: result.content,
          originalContent: result.content,
          dirty: false,
          lastReadAt: new Date().toISOString(),
          lastSavedAt: result.modifiedAt,
        };
        useFileEditorSidebarStore.getState().openFileTab(newFile);
      } catch (error) {
        notify.error(`Failed to reload file: ${error}`);
      } finally {
        actions.setStatus(undefined);
      }
    },
    [actions]
  );

  // Get tabs as array for rendering
  const tabs = useMemo((): Tab[] => {
    return state.tabOrder.map((id) => state.tabs[id]).filter((t): t is Tab => t !== undefined);
  }, [state.tabOrder, state.tabs]);

  return {
    // State (entire store state for convenience, but components should use specific fields)
    open: state.open,
    width: state.width,
    vimMode: state.vimMode,
    vimModeState: state.vimModeState,
    wrap: state.wrap,
    lineNumbers: state.lineNumbers,
    relativeLineNumbers: state.relativeLineNumbers,
    showHiddenFiles: state.showHiddenFiles,
    recentFiles: state.recentFiles,
    status: state.status,
    activeTabId: state.activeTabId,
    activeTab,
    activeFileTab,
    activeFile,
    tabs,
    // File operations
    openFile,
    openBrowser,
    saveFile,
    reloadFile,
    // Tab operations
    setActiveTab: actions.setActiveTab,
    closeTab: actions.closeTab,
    closeAllTabs: actions.closeAllTabs,
    closeOtherTabs: actions.closeOtherTabs,
    reorderTabs: actions.reorderTabs,
    // Editor state
    setOpen: actions.setOpen,
    setWidth: actions.setWidth,
    setStatus: actions.setStatus,
    updateFileContent: actions.updateFileContent,
    setBrowserPath: actions.setBrowserPath,
    setVimMode: actions.setVimMode,
    setVimModeState: actions.setVimModeState,
    setWrap: actions.setWrap,
    setLineNumbers: actions.setLineNumbers,
    setRelativeLineNumbers: actions.setRelativeLineNumbers,
    setShowHiddenFiles: actions.setShowHiddenFiles,
    toggleMarkdownPreview: actions.toggleMarkdownPreview,
  };
}
