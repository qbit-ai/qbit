import { useCallback, useEffect, useMemo } from "react";
import { readWorkspaceFile, writeWorkspaceFile } from "@/lib/file-editor";
import { notify } from "@/lib/notify";
import {
  type EditorFileState,
  fileTabIdFromPath,
  selectActiveFileTab,
  selectActiveTab,
  selectSessionState,
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

export function useFileEditorSidebar(sessionId: string | null, workingDirectory?: string) {
  useEffect(() => {
    if (sessionId) {
      useFileEditorSidebarStore.getState().ensureSession(sessionId);
    }
  }, [sessionId]);

  const session = useFileEditorSidebarStore(
    useCallback((state) => (sessionId ? selectSessionState(state, sessionId) : null), [sessionId])
  );

  // Derive active tab and active file from session
  const activeTab = useMemo(() => (session ? selectActiveTab(session) : null), [session]);
  const activeFileTab = useMemo(() => (session ? selectActiveFileTab(session) : null), [session]);
  const activeFile = activeFileTab?.file ?? null;

  const actions = useMemo(() => {
    return {
      setOpen: (open: boolean) => {
        if (!sessionId) return;
        useFileEditorSidebarStore.getState().setOpen(sessionId, open);
      },
      setWidth: (width: number) => {
        if (!sessionId) return;
        useFileEditorSidebarStore.getState().setWidth(sessionId, width);
      },
      setStatus: (status?: string) => {
        if (!sessionId) return;
        useFileEditorSidebarStore.getState().setStatus(sessionId, status);
      },
      setActiveTab: (tabId: string) => {
        if (!sessionId) return;
        useFileEditorSidebarStore.getState().setActiveTab(sessionId, tabId);
      },
      closeTab: (tabId?: string) => {
        if (!sessionId) return;
        useFileEditorSidebarStore.getState().closeTab(sessionId, tabId);
      },
      closeAllTabs: () => {
        if (!sessionId) return;
        useFileEditorSidebarStore.getState().closeAllTabs(sessionId);
      },
      closeOtherTabs: (keepTabId: string) => {
        if (!sessionId) return;
        useFileEditorSidebarStore.getState().closeOtherTabs(sessionId, keepTabId);
      },
      reorderTabs: (fromIndex: number, toIndex: number) => {
        if (!sessionId) return;
        useFileEditorSidebarStore.getState().reorderTabs(sessionId, fromIndex, toIndex);
      },
      updateFileContent: (tabId: string, content: string) => {
        if (!sessionId) return;
        useFileEditorSidebarStore.getState().updateFileContent(sessionId, tabId, content);
      },
      setBrowserPath: (tabId: string, path: string) => {
        if (!sessionId) return;
        useFileEditorSidebarStore.getState().setBrowserPath(sessionId, tabId, path);
      },
      setVimMode: (enabled: boolean) => {
        if (!sessionId) return;
        useFileEditorSidebarStore.getState().setVimMode(sessionId, enabled);
      },
      setVimModeState: (state: "normal" | "insert" | "visual") => {
        if (!sessionId) return;
        useFileEditorSidebarStore.getState().setVimModeState(sessionId, state);
      },
      setWrap: (enabled: boolean) => {
        if (!sessionId) return;
        useFileEditorSidebarStore.getState().setWrap(sessionId, enabled);
      },
      setLineNumbers: (enabled: boolean) => {
        if (!sessionId) return;
        useFileEditorSidebarStore.getState().setLineNumbers(sessionId, enabled);
      },
      setRelativeLineNumbers: (enabled: boolean) => {
        if (!sessionId) return;
        useFileEditorSidebarStore.getState().setRelativeLineNumbers(sessionId, enabled);
      },
      addRecentFile: (path: string) => {
        if (!sessionId) return;
        useFileEditorSidebarStore.getState().addRecentFile(sessionId, path);
      },
      toggleMarkdownPreview: (tabId: string) => {
        if (!sessionId) return;
        useFileEditorSidebarStore.getState().toggleMarkdownPreview(sessionId, tabId);
      },
    };
  }, [sessionId]);

  const openFile = useCallback(
    async (inputPath: string) => {
      if (!sessionId) {
        notify.error("No active session for file open");
        return;
      }
      const fullPath = resolvePath(inputPath, workingDirectory);

      // If file is already open, just switch to it
      const tabId = fileTabIdFromPath(fullPath);
      const state = useFileEditorSidebarStore.getState();
      const currentSession = state.sessions[sessionId];
      if (currentSession?.tabs[tabId]) {
        state.setActiveTab(sessionId, tabId);
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
        useFileEditorSidebarStore.getState().openFileTab(sessionId, file);
        actions.addRecentFile(fullPath);
      } catch (error) {
        notify.error(`Failed to open file: ${error}`);
      } finally {
        actions.setStatus(undefined);
      }
    },
    [actions, sessionId, workingDirectory]
  );

  const openBrowser = useCallback(
    (initialPath?: string) => {
      if (!sessionId) {
        notify.error("No active session");
        return;
      }
      const path = initialPath ?? workingDirectory ?? "";
      useFileEditorSidebarStore.getState().openBrowserTab(sessionId, path);
    },
    [sessionId, workingDirectory]
  );

  const saveFile = useCallback(
    async (tabId?: string) => {
      if (!sessionId || !session) return;

      const targetTabId = tabId ?? session.activeTabId;
      if (!targetTabId) {
        notify.info("No file open to save");
        return;
      }

      const tab = session.tabs[targetTabId];
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
        useFileEditorSidebarStore
          .getState()
          .markFileSaved(sessionId, targetTabId, result.modifiedAt);
        notify.success("Saved");
      } catch (error) {
        notify.error(`Failed to save file: ${error}`);
      } finally {
        actions.setStatus(undefined);
      }
    },
    [actions, session, sessionId]
  );

  const reloadFile = useCallback(
    async (tabId?: string) => {
      if (!sessionId || !session) return;

      const targetTabId = tabId ?? session.activeTabId;
      if (!targetTabId) {
        notify.info("No file open to reload");
        return;
      }

      const tab = session.tabs[targetTabId];
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
        useFileEditorSidebarStore.getState().openFileTab(sessionId, newFile);
      } catch (error) {
        notify.error(`Failed to reload file: ${error}`);
      } finally {
        actions.setStatus(undefined);
      }
    },
    [actions, session, sessionId]
  );

  // Get tabs as array for rendering
  const tabs = useMemo((): Tab[] => {
    if (!session) return [];
    return session.tabOrder.map((id) => session.tabs[id]).filter((t): t is Tab => t !== undefined);
  }, [session]);

  return {
    session,
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
    toggleMarkdownPreview: actions.toggleMarkdownPreview,
  };
}
