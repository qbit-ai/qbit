import { lazy, Suspense, useCallback, useEffect, useState } from "react";
import { logger } from "@/lib/logger";
import { CommandPalette, type PageRoute } from "./components/CommandPalette";
import { PaneContainer } from "./components/PaneContainer";
import { SidecarNotifications } from "./components/Sidecar";
import { TabBar } from "./components/TabBar";
import { TerminalLayer } from "./components/Terminal";
import { Skeleton } from "./components/ui/skeleton";
import {
  createKeyboardHandler,
  useKeyboardHandlerContext,
} from "./hooks/useKeyboardHandlerContext";
import { usePaneControls } from "./hooks/usePaneControls";

// Lazy loaded components - these are not needed on initial render
// and can be loaded on-demand to reduce initial bundle size
const FileEditorSidebarPanel = lazy(() =>
  import("./components/FileEditorSidebar").then((m) => ({
    default: m.FileEditorSidebarPanel,
  }))
);
const GitPanel = lazy(() => import("./components/GitPanel").then((m) => ({ default: m.GitPanel })));
const SessionBrowser = lazy(() =>
  import("./components/SessionBrowser/SessionBrowser").then((m) => ({
    default: m.SessionBrowser,
  }))
);
const SettingsDialog = lazy(() =>
  import("./components/Settings").then((m) => ({ default: m.SettingsDialog }))
);
const ContextPanel = lazy(() =>
  import("./components/Sidecar/ContextPanel").then((m) => ({
    default: m.ContextPanel,
  }))
);
const SidecarPanel = lazy(() =>
  import("./components/Sidecar/SidecarPanel").then((m) => ({
    default: m.SidecarPanel,
  }))
);
const ComponentTestbed = lazy(() =>
  import("./pages/ComponentTestbed").then((m) => ({
    default: m.ComponentTestbed,
  }))
);
const MockDevTools = lazy(() =>
  import("./components/MockDevTools").then((m) => ({
    default: m.MockDevTools,
  }))
);
const QuickOpenDialog = lazy(() =>
  import("./components/QuickOpenDialog").then((m) => ({
    default: m.QuickOpenDialog,
  }))
);

import { MockDevToolsProvider } from "./components/MockDevTools";
import { useAiEvents } from "./hooks/useAiEvents";
import { useCreateTerminalTab } from "./hooks/useCreateTerminalTab";
import { useTauriEvents } from "./hooks/useTauriEvents";
import { TerminalPortalProvider } from "./hooks/useTerminalPortal";
import { ThemeProvider } from "./hooks/useTheme";
import { isMockBrowserMode } from "./lib/isMockBrowser";
import { notify } from "./lib/notify";
import { initSystemNotifications, listenForSettingsUpdates } from "./lib/systemNotifications";
import { shellIntegrationInstall, shellIntegrationStatus } from "./lib/tauri";
import { clearConversation, restoreSession, useStore } from "./store";
import { useFileEditorSidebarStore } from "./store/file-editor-sidebar";
import { useAppState } from "./store/selectors";

function App() {
  // Get store state using optimized selectors that only subscribe to needed data
  const { activeSessionId, focusedWorkingDirectory: workingDirectory, tabLayouts } = useAppState();

  // Get stable action references (actions are stable by design in Zustand)
  const setInputMode = useStore((state) => state.setInputMode);
  const setRenderMode = useStore((state) => state.setRenderMode);
  const openSettingsTab = useStore((state) => state.openSettingsTab);
  const openHomeTab = useStore((state) => state.openHomeTab);

  // Panel state from store (replaces local useState)
  const gitPanelOpen = useStore((state) => state.gitPanelOpen);
  const contextPanelOpen = useStore((state) => state.contextPanelOpen);
  const fileEditorPanelOpen = useStore((state) => state.fileEditorPanelOpen);
  const sidecarPanelOpen = useStore((state) => state.sidecarPanelOpen);
  const sessionBrowserOpen = useStore((state) => state.sessionBrowserOpen);
  const openGitPanel = useStore((state) => state.openGitPanel);
  const openContextPanel = useStore((state) => state.openContextPanel);
  const toggleFileEditorPanel = useStore((state) => state.toggleFileEditorPanel);
  const closePanels = useStore((state) => state.closePanels);
  const setSessionBrowserOpen = useStore((state) => state.setSessionBrowserOpen);

  const { createTerminalTab } = useCreateTerminalTab();
  const { handleSplitPane, handleClosePane, handleNavigatePane } = usePaneControls(activeSessionId);

  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [commandPaletteOpen, setCommandPaletteOpen] = useState(false);
  const [quickOpenDialogOpen, setQuickOpenDialogOpen] = useState(false);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [currentPage, setCurrentPage] = useState<PageRoute>("main");

  // Subscribe to file editor sidebar store to sync open state
  // This allows openFile() calls from anywhere to open the sidebar
  const fileEditorStoreOpen = useFileEditorSidebarStore((state) => state.open);
  useEffect(() => {
    if (fileEditorStoreOpen && !fileEditorPanelOpen) {
      useStore.getState().openFileEditorPanel();
    }
  }, [fileEditorStoreOpen, fileEditorPanelOpen]);

  // Connect Tauri events to store
  useTauriEvents();

  // Subscribe to AI events for agent mode
  useAiEvents();

  // Handle toggle mode from command palette (cycles: terminal → agent → auto → terminal)
  const handleToggleMode = useCallback(() => {
    if (activeSessionId) {
      const currentSession = useStore.getState().sessions[activeSessionId];
      const current = currentSession?.inputMode ?? "terminal";
      const newMode = current === "terminal" ? "agent" : current === "agent" ? "auto" : "terminal";
      setInputMode(activeSessionId, newMode);
    }
  }, [activeSessionId, setInputMode]);

  // Create a new terminal tab
  const handleNewTab = useCallback(() => {
    createTerminalTab();
  }, [createTerminalTab]);

  useEffect(() => {
    async function init() {
      try {
        const currentSessions = useStore.getState().sessions;
        if (Object.keys(currentSessions).length > 0) {
          logger.info("[App] Sessions already exist, skipping initialization...");
          setIsLoading(false);
          return;
        }

        logger.info("[App] Starting initialization...");

        // Create home tab first (always visible, leftmost)
        openHomeTab();

        // Check and install shell integration in the background (non-blocking)
        void (async () => {
          try {
            const status = await shellIntegrationStatus();
            if (status.type === "NotInstalled") {
              notify.info("Installing shell integration...");
              await shellIntegrationInstall();
              notify.success("Shell integration installed! Restart your shell for full features.");
            } else if (status.type === "Outdated") {
              notify.info("Updating shell integration...");
              await shellIntegrationInstall();
              notify.success("Shell integration updated!");
            }
          } catch (e) {
            logger.warn("Shell integration check failed:", e);
          }
        })();

        // Create initial terminal session (only awaits PTY creation)
        await createTerminalTab();

        setIsLoading(false);
      } catch (e) {
        logger.error("Failed to initialize:", e);
        setError(e instanceof Error ? e.message : String(e));
        setIsLoading(false);
      }
    }

    init();
  }, [openHomeTab, createTerminalTab]);

  // Initialize system notifications and app focus/visibility tracking
  useEffect(() => {
    const { setAppIsFocused, setAppIsVisible } = useStore.getState();

    // Initialize notification system with store API
    initSystemNotifications(useStore).catch((error) => {
      logger.error("Failed to initialize system notifications:", error);
    });

    // Listen for settings updates to reactively update notification state
    const unlistenSettings = listenForSettingsUpdates();

    // Track window focus state
    const handleFocus = () => setAppIsFocused(true);
    const handleBlur = () => setAppIsFocused(false);

    // Track document visibility state
    const handleVisibilityChange = () => {
      setAppIsVisible(document.visibilityState === "visible");
    };

    window.addEventListener("focus", handleFocus);
    window.addEventListener("blur", handleBlur);
    document.addEventListener("visibilitychange", handleVisibilityChange);

    // Set initial state
    setAppIsFocused(document.hasFocus());
    setAppIsVisible(document.visibilityState === "visible");

    return () => {
      // Cleanup settings listener
      unlistenSettings();
      // Cleanup app focus/visibility listeners
      window.removeEventListener("focus", handleFocus);
      window.removeEventListener("blur", handleBlur);
      document.removeEventListener("visibilitychange", handleVisibilityChange);
    };
  }, []);

  // Keyboard shortcuts using refs pattern to avoid recreating the handler on every state change
  const keyboardContextRef = useKeyboardHandlerContext();

  keyboardContextRef.current = {
    ...keyboardContextRef.current,
    gitPanelOpen,
    handleNewTab,
    handleToggleMode,
    openContextPanel,
    openGitPanel,
    toggleFileEditorPanel,
    openSettingsTab,
    handleSplitPane,
    handleClosePane,
    handleNavigatePane,
    setCommandPaletteOpen,
    setQuickOpenDialogOpen,
    setSidecarPanelOpen: (open: boolean) => {
      if (open) {
        useStore.getState().openSidecarPanel();
      } else {
        closePanels();
      }
    },
  };

  // Set up the keyboard event listener once
  useEffect(() => {
    const handleKeyDown = createKeyboardHandler(keyboardContextRef);
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [keyboardContextRef]);

  // Handle clear conversation from command palette
  const handleClearConversation = useCallback(async () => {
    if (activeSessionId) {
      await clearConversation(activeSessionId);
      notify.success("Conversation cleared");
    }
  }, [activeSessionId]);

  // Handle toggle full terminal mode from command palette
  const handleToggleFullTerminal = useCallback(() => {
    if (activeSessionId) {
      const currentRenderMode =
        useStore.getState().sessions[activeSessionId]?.renderMode ?? "timeline";
      setRenderMode(activeSessionId, currentRenderMode === "fullterm" ? "timeline" : "fullterm");
    }
  }, [activeSessionId, setRenderMode]);

  // Handle session restore from session browser
  const handleRestoreSession = useCallback(
    async (identifier: string) => {
      if (!activeSessionId) {
        notify.error("No active session to restore into");
        return;
      }
      try {
        await restoreSession(activeSessionId, identifier);
        notify.success("Session restored");
      } catch (error) {
        notify.error(`Failed to restore session: ${error}`);
      }
    },
    [activeSessionId]
  );

  const handleOpenHistory = useCallback(() => setSessionBrowserOpen(true), [setSessionBrowserOpen]);

  // Panel onOpenChange callbacks
  const handleGitPanelOpenChange = useCallback(
    (open: boolean) => {
      if (open) {
        openGitPanel();
      } else {
        closePanels();
      }
    },
    [openGitPanel, closePanels]
  );

  const handleContextPanelOpenChange = useCallback(
    (open: boolean) => {
      if (open) {
        openContextPanel();
      } else {
        closePanels();
      }
    },
    [openContextPanel, closePanels]
  );

  const handleFileEditorPanelOpenChange = useCallback(
    (open: boolean) => {
      if (open) {
        useStore.getState().openFileEditorPanel();
      } else {
        closePanels();
        useFileEditorSidebarStore.getState().setOpen(false);
      }
    },
    [closePanels]
  );

  const handleSidecarPanelOpenChange = useCallback(
    (open: boolean) => {
      if (open) {
        useStore.getState().openSidecarPanel();
      } else {
        closePanels();
      }
    },
    [closePanels]
  );

  if (isLoading) {
    return (
      <div className="h-screen w-screen bg-[#1a1b26] flex flex-col overflow-hidden">
        {/* Skeleton tab bar */}
        <div className="flex items-center h-9 bg-[#1a1b26] pl-[78px] pr-2 gap-2 titlebar-drag">
          <Skeleton className="h-6 w-24 bg-[#1f2335]" />
          <Skeleton className="h-6 w-6 rounded bg-[#1f2335]" />
        </div>

        {/* Skeleton content area */}
        <div className="flex-1 p-4 space-y-3">
          <Skeleton className="h-16 w-full bg-[#1f2335]" />
          <Skeleton className="h-16 w-3/4 bg-[#1f2335]" />
          <Skeleton className="h-16 w-5/6 bg-[#1f2335]" />
        </div>

        {/* Skeleton input area */}
        <div className="bg-[#1a1b26] border-t border-[#1f2335] px-4 py-3 space-y-2">
          <div className="flex items-center justify-between">
            <Skeleton className="h-4 w-32 bg-[#1f2335]" />
            <Skeleton className="h-7 w-40 rounded-lg bg-[#1f2335]" />
          </div>
          <Skeleton className="h-8 w-full bg-[#1f2335]" />
        </div>

        {/* Mock Dev Tools - available during loading in browser mode */}
        {isMockBrowserMode() && (
          <Suspense fallback={null}>
            <MockDevTools />
          </Suspense>
        )}
      </div>
    );
  }

  if (error) {
    return (
      <div className="flex items-center justify-center h-screen bg-[#1a1b26]">
        <div className="text-[#f7768e] text-lg">Error: {error}</div>
        {/* Mock Dev Tools - available on error in browser mode */}
        {isMockBrowserMode() && (
          <Suspense fallback={null}>
            <MockDevTools />
          </Suspense>
        )}
      </div>
    );
  }

  // Render component testbed page
  if (currentPage === "testbed") {
    return (
      <>
        <Suspense fallback={<div className="h-screen w-screen bg-background" />}>
          <ComponentTestbed />
        </Suspense>
        <CommandPalette
          open={commandPaletteOpen}
          onOpenChange={setCommandPaletteOpen}
          currentPage={currentPage}
          onNavigate={setCurrentPage}
          activeSessionId={activeSessionId}
          onNewTab={handleNewTab}
          onToggleMode={handleToggleMode}
          onClearConversation={handleClearConversation}
          onToggleFullTerminal={handleToggleFullTerminal}
          onOpenSessionBrowser={handleOpenHistory}
          onOpenSettings={openSettingsTab}
        />
        <Suspense fallback={null}>
          <SessionBrowser
            open={sessionBrowserOpen}
            onOpenChange={setSessionBrowserOpen}
            onSessionRestore={handleRestoreSession}
          />
        </Suspense>
        <Suspense fallback={null}>
          <SettingsDialog open={settingsOpen} onOpenChange={setSettingsOpen} />
        </Suspense>
        {/* Mock Dev Tools - available on testbed in browser mode */}
        {isMockBrowserMode() && (
          <Suspense fallback={null}>
            <MockDevTools />
          </Suspense>
        )}
      </>
    );
  }

  return (
    <TerminalPortalProvider>
      <div className="h-screen w-screen bg-background flex flex-col overflow-hidden app-bg-layered">
        {/* Tab bar - self-sufficient, no props needed */}
        <TabBar />

        {/* Main content area */}
        <div className="flex-1 min-h-0 min-w-0 flex overflow-hidden">
          {/* Main content - Pane layout */}
          {/* Render ALL tabs but only show the active one. This keeps Terminal instances
              mounted across tab switches so fullterm apps (claude, codex) don't lose state. */}
          <div className="flex-1 min-h-0 min-w-0 flex flex-col overflow-hidden relative">
            {tabLayouts.map(({ tabId, root }) => (
              <div
                key={tabId}
                className={`absolute inset-0 ${tabId === activeSessionId ? "visible" : "invisible pointer-events-none"}`}
              >
                <PaneContainer node={root} tabId={tabId} />
              </div>
            ))}
            {!activeSessionId && (
              <div className="flex items-center justify-center h-full">
                <span className="text-[#565f89]">No active session</span>
              </div>
            )}
          </div>

          {/* Lazy-loaded side panels - wrapped in Suspense with null fallback
              since they render nothing when closed anyway */}
          <Suspense fallback={null}>
            <GitPanel open={gitPanelOpen} onOpenChange={handleGitPanelOpenChange} />
          </Suspense>

          {/* Context Panel - integrated side panel, uses sidecar's current session */}
          <Suspense fallback={null}>
            <ContextPanel open={contextPanelOpen} onOpenChange={handleContextPanelOpenChange} />
          </Suspense>

          {/* File Editor Panel - right side code editor (shared across all tabs) */}
          <Suspense fallback={null}>
            <FileEditorSidebarPanel
              open={fileEditorPanelOpen}
              onOpenChange={handleFileEditorPanelOpenChange}
            />
          </Suspense>
        </div>

        {/* Terminal Layer - renders all Terminal instances via React portals.
            Terminals are rendered here (at a stable position in the tree) and portaled
            into their respective PaneLeaf targets. This prevents Terminal unmount/remount
            when pane structure changes during splits. */}
        <TerminalLayer />

        {/* Command Palette */}
        <CommandPalette
          open={commandPaletteOpen}
          onOpenChange={setCommandPaletteOpen}
          currentPage={currentPage}
          onNavigate={setCurrentPage}
          activeSessionId={activeSessionId}
          onNewTab={handleNewTab}
          onToggleMode={handleToggleMode}
          onClearConversation={handleClearConversation}
          onToggleFullTerminal={handleToggleFullTerminal}
          workingDirectory={workingDirectory}
          onOpenSessionBrowser={handleOpenHistory}
          onToggleFileEditorPanel={toggleFileEditorPanel}
          onOpenContextPanel={openContextPanel}
          onOpenSettings={openSettingsTab}
          onSplitPaneRight={() => handleSplitPane("vertical")}
          onSplitPaneDown={() => handleSplitPane("horizontal")}
          onClosePane={handleClosePane}
          onOpenQuickOpen={() => setQuickOpenDialogOpen(true)}
        />

        {/* Quick Open Dialog (Cmd+P) */}
        <Suspense fallback={null}>
          <QuickOpenDialog
            open={quickOpenDialogOpen}
            onOpenChange={setQuickOpenDialogOpen}
            workingDirectory={workingDirectory}
          />
        </Suspense>

        {/* Lazy-loaded dialogs and panels - wrapped in Suspense with null fallback
            since they render nothing when closed anyway */}
        <Suspense fallback={null}>
          <SidecarPanel open={sidecarPanelOpen} onOpenChange={handleSidecarPanelOpenChange} />
        </Suspense>

        <Suspense fallback={null}>
          <SessionBrowser
            open={sessionBrowserOpen}
            onOpenChange={setSessionBrowserOpen}
            onSessionRestore={handleRestoreSession}
          />
        </Suspense>

        <Suspense fallback={null}>
          <SettingsDialog open={settingsOpen} onOpenChange={setSettingsOpen} />
        </Suspense>

        {/* Sidecar event notifications */}
        <SidecarNotifications />

        {/* Mock Dev Tools - only in browser mode */}
        {isMockBrowserMode() && (
          <Suspense fallback={null}>
            <MockDevTools />
          </Suspense>
        )}
      </div>
    </TerminalPortalProvider>
  );
}

function AppWithTheme() {
  const content = (
    <ThemeProvider defaultThemeId="qbit">
      <App />
    </ThemeProvider>
  );

  // Wrap with MockDevToolsProvider only in browser mode
  if (isMockBrowserMode()) {
    return <MockDevToolsProvider>{content}</MockDevToolsProvider>;
  }

  return content;
}

export default AppWithTheme;
