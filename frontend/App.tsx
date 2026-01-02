import { useCallback, useEffect, useRef, useState } from "react";
import { CommandPalette, type PageRoute } from "./components/CommandPalette";
import { MockDevTools, MockDevToolsProvider } from "./components/MockDevTools";
import { PaneContainer } from "./components/PaneContainer";
import { SessionBrowser } from "./components/SessionBrowser";
import { SettingsDialog } from "./components/Settings";
import { Sidebar } from "./components/Sidebar";
import { ContextPanel, SidecarNotifications, SidecarPanel } from "./components/Sidecar";
import { StatusBar } from "./components/StatusBar";
import { TabBar } from "./components/TabBar";
import { TaskPlannerPanel } from "./components/TaskPlannerPanel";
import { TerminalLayer } from "./components/Terminal";
import { Skeleton } from "./components/ui/skeleton";
import { useAiEvents } from "./hooks/useAiEvents";
import { useTauriEvents } from "./hooks/useTauriEvents";
import { TerminalPortalProvider } from "./hooks/useTerminalPortal";
import { ThemeProvider } from "./hooks/useTheme";
import {
  getAnthropicApiKey,
  getOpenAiApiKey,
  getOpenRouterApiKey,
  initAiSession,
  isAiSessionInitialized,
  type ProviderConfig,
} from "./lib/ai";
import { notify } from "./lib/notify";
import { countLeafPanes, findPaneById } from "./lib/pane-utils";
import { getSettings, type QbitSettings } from "./lib/settings";
import {
  getGitBranch,
  ptyCreate,
  shellIntegrationInstall,
  shellIntegrationStatus,
} from "./lib/tauri";
import { isMockBrowserMode } from "./mocks";
import { ComponentTestbed } from "./pages/ComponentTestbed";
import {
  clearConversation,
  restoreSession,
  type SplitDirection,
  useFocusedSessionId,
  useStore,
  useTabLayout,
} from "./store";

/**
 * Build a ProviderConfig for the given provider/model settings.
 * This is used by both session-specific and global initialization.
 */
async function buildProviderConfig(
  settings: QbitSettings,
  workspace: string
): Promise<ProviderConfig> {
  const { default_provider, default_model } = settings.ai;

  switch (default_provider) {
    case "vertex_ai": {
      const { vertex_ai } = settings.ai;
      if (!vertex_ai.credentials_path || !vertex_ai.project_id) {
        throw new Error("Vertex AI credentials not configured");
      }
      return {
        provider: "vertex_ai",
        workspace,
        credentials_path: vertex_ai.credentials_path,
        project_id: vertex_ai.project_id,
        location: vertex_ai.location || "us-east5",
        model: default_model,
      };
    }

    case "anthropic": {
      const apiKey = settings.ai.anthropic.api_key || (await getAnthropicApiKey());
      if (!apiKey) throw new Error("Anthropic API key not configured");
      return { provider: "anthropic", workspace, model: default_model, api_key: apiKey };
    }

    case "openai": {
      const apiKey = settings.ai.openai.api_key || (await getOpenAiApiKey());
      if (!apiKey) throw new Error("OpenAI API key not configured");
      return { provider: "openai", workspace, model: default_model, api_key: apiKey };
    }

    case "openrouter": {
      const apiKey = settings.ai.openrouter.api_key || (await getOpenRouterApiKey());
      if (!apiKey) throw new Error("OpenRouter API key not configured");
      return { provider: "openrouter", workspace, model: default_model, api_key: apiKey };
    }

    case "ollama": {
      const baseUrl = settings.ai.ollama.base_url;
      return { provider: "ollama", workspace, model: default_model, base_url: baseUrl };
    }

    case "gemini": {
      const apiKey = settings.ai.gemini.api_key;
      if (!apiKey) throw new Error("Gemini API key not configured");
      return { provider: "gemini", workspace, model: default_model, api_key: apiKey };
    }

    case "groq": {
      const apiKey = settings.ai.groq.api_key;
      if (!apiKey) throw new Error("Groq API key not configured");
      return { provider: "groq", workspace, model: default_model, api_key: apiKey };
    }

    case "xai": {
      const apiKey = settings.ai.xai.api_key;
      if (!apiKey) throw new Error("xAI API key not configured");
      return { provider: "xai", workspace, model: default_model, api_key: apiKey };
    }

    default:
      throw new Error(`Unknown provider: ${default_provider}`);
  }
}

function App() {
  const {
    addSession,
    activeSessionId,
    sessions,
    tabLayouts,
    setInputMode,
    setAiConfig,
    setSessionAiConfig,
    setRenderMode,
    updateGitBranch,
    splitPane,
    closePane,
    navigatePane,
    removeSession,
  } = useStore();
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [commandPaletteOpen, setCommandPaletteOpen] = useState(false);
  const [sessionBrowserOpen, setSessionBrowserOpen] = useState(false);
  const [contextPanelOpen, setContextPanelOpen] = useState(false);
  const [taskPlannerOpen, setTaskPlannerOpen] = useState(false);
  const [sidecarPanelOpen, setSidecarPanelOpen] = useState(false);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [currentPage, setCurrentPage] = useState<PageRoute>("main");
  const [sidebarOpen, setSidebarOpen] = useState(false);
  const initializingRef = useRef(false);

  // Exclusive right panel toggles - only one right panel visible at a time
  const openContextPanel = useCallback(() => {
    setTaskPlannerOpen(false);
    setContextPanelOpen(true);
  }, []);

  const openTaskPlanner = useCallback(() => {
    setContextPanelOpen(false);
    setTaskPlannerOpen(true);
  }, []);

  // Get pane layout for the active tab
  const tabLayout = useTabLayout(activeSessionId);

  // Get focused session ID (the session in the currently focused pane)
  const focusedSessionId = useFocusedSessionId(activeSessionId);

  // Get focused session's working directory for sidebar/status bar
  const focusedSession = focusedSessionId ? sessions[focusedSessionId] : null;
  const workingDirectory = focusedSession?.workingDirectory;

  // Connect Tauri events to store
  useTauriEvents();

  // Subscribe to AI events for agent mode
  useAiEvents();

  // Create a new terminal tab
  const handleNewTab = useCallback(async () => {
    try {
      const session = await ptyCreate();
      const settings = await getSettings();
      const { default_provider, default_model } = settings.ai;

      // Add session with initial AI config
      addSession({
        id: session.id,
        name: "Terminal",
        workingDirectory: session.working_directory,
        createdAt: new Date().toISOString(),
        mode: "terminal",
        aiConfig: {
          provider: default_provider,
          model: default_model,
          status: "initializing",
        },
      });

      // Fetch git branch for the initial working directory
      try {
        const branch = await getGitBranch(session.working_directory);
        updateGitBranch(session.id, branch);
      } catch {
        // Silently ignore - not a git repo or git not installed
      }

      // Also update global config for backwards compatibility
      setAiConfig({
        provider: default_provider,
        model: default_model,
        status: "initializing",
      });

      // Initialize AI for this specific session
      try {
        const config = await buildProviderConfig(settings, session.working_directory);
        await initAiSession(session.id, config);

        // Update session-specific AI config
        setSessionAiConfig(session.id, { status: "ready" });

        // Also update global config for backwards compatibility
        setAiConfig({ status: "ready" });
      } catch (aiError) {
        console.error("Failed to initialize AI for new tab:", aiError);
        const errorMessage = aiError instanceof Error ? aiError.message : "Unknown error";

        setSessionAiConfig(session.id, {
          status: "error",
          errorMessage,
        });

        // Also update global config
        setAiConfig({
          provider: "",
          model: "",
          status: "error",
          errorMessage,
        });
      }
    } catch (e) {
      console.error("Failed to create new tab:", e);
      notify.error("Failed to create new tab");
    }
  }, [addSession, setAiConfig, setSessionAiConfig, updateGitBranch]);

  // Split the currently focused pane
  const handleSplitPane = useCallback(
    async (direction: SplitDirection) => {
      if (!activeSessionId || !tabLayout) return;

      // Check pane limit (max 4 panes per tab)
      const currentCount = countLeafPanes(tabLayout.root);
      if (currentCount >= 4) {
        notify.warning("Maximum pane limit (4) reached");
        return;
      }

      const focusedPane = findPaneById(tabLayout.root, tabLayout.focusedPaneId);
      if (!focusedPane || focusedPane.type !== "leaf") return;

      // Get source session for working directory
      const sourceSession = sessions[focusedPane.sessionId];
      if (!sourceSession) return;

      try {
        // Create new PTY session (inherits working directory)
        const newSession = await ptyCreate(sourceSession.workingDirectory);
        const settings = await getSettings();
        const { default_provider, default_model } = settings.ai;

        const newPaneId = crypto.randomUUID();

        // Add the new session to the store (as a pane, not a new tab)
        addSession(
          {
            id: newSession.id,
            name: "Terminal",
            workingDirectory: newSession.working_directory,
            createdAt: new Date().toISOString(),
            mode: "terminal",
            aiConfig: {
              provider: default_provider,
              model: default_model,
              status: "initializing",
            },
          },
          { isPaneSession: true }
        );

        // Fetch git branch for the new session
        try {
          const branch = await getGitBranch(newSession.working_directory);
          updateGitBranch(newSession.id, branch);
        } catch {
          // Silently ignore - not a git repo or git not installed
        }

        // Initialize AI for the new session
        try {
          const config = await buildProviderConfig(settings, newSession.working_directory);
          await initAiSession(newSession.id, config);
          setSessionAiConfig(newSession.id, { status: "ready" });
        } catch (aiError) {
          console.error("Failed to initialize AI for new pane:", aiError);
          const errorMessage = aiError instanceof Error ? aiError.message : "Unknown error";
          setSessionAiConfig(newSession.id, { status: "error", errorMessage });
        }

        // Split the pane
        splitPane(activeSessionId, tabLayout.focusedPaneId, direction, newPaneId, newSession.id);
      } catch (e) {
        console.error("Failed to split pane:", e);
        notify.error("Failed to split pane");
      }
    },
    [
      activeSessionId,
      tabLayout,
      sessions,
      addSession,
      splitPane,
      updateGitBranch,
      setSessionAiConfig,
    ]
  );

  // Close the currently focused pane
  const handleClosePane = useCallback(async () => {
    if (!activeSessionId || !tabLayout) return;

    const focusedPane = findPaneById(tabLayout.root, tabLayout.focusedPaneId);
    if (!focusedPane || focusedPane.type !== "leaf") return;

    const sessionIdToClose = focusedPane.sessionId;
    const isLastPane = countLeafPanes(tabLayout.root) === 1;

    try {
      // Shutdown AI session if initialized
      try {
        const { shutdownAiSession } = await import("./lib/ai");
        await shutdownAiSession(sessionIdToClose);
      } catch {
        // Ignore - session may not have been initialized
      }

      // Destroy PTY
      try {
        const { ptyDestroy } = await import("./lib/tauri");
        await ptyDestroy(sessionIdToClose);
      } catch {
        // Ignore - PTY may already be destroyed
      }

      if (isLastPane) {
        // Last pane - close the entire tab
        // First clean up the pane's session state if it differs from the tab ID
        if (sessionIdToClose !== activeSessionId) {
          // The pane's session was different from the tab's root session.
          // closePane handles this cleanup, but removeSession only cleans up
          // the tab's root session. We need to manually clean both.
          closePane(activeSessionId, tabLayout.focusedPaneId);
        }
        // Now remove the tab (this also cleans up tabLayouts)
        removeSession(activeSessionId);
      } else {
        // Close just this pane
        closePane(activeSessionId, tabLayout.focusedPaneId);
      }
    } catch (e) {
      console.error("Failed to close pane:", e);
      notify.error("Failed to close pane");
    }
  }, [activeSessionId, tabLayout, closePane, removeSession]);

  // Navigate between panes
  const handleNavigatePane = useCallback(
    (direction: "up" | "down" | "left" | "right") => {
      if (!activeSessionId) return;
      navigatePane(activeSessionId, direction);
    },
    [activeSessionId, navigatePane]
  );

  useEffect(() => {
    async function init() {
      try {
        // Prevent double-initialization from React StrictMode in development
        if (initializingRef.current) {
          return;
        }
        initializingRef.current = true;

        // Check and install shell integration if needed
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

        // Create initial terminal session
        const session = await ptyCreate();
        const settings = await getSettings();
        const { default_provider, default_model } = settings.ai;

        // Add session with initial AI config
        addSession({
          id: session.id,
          name: "Terminal",
          workingDirectory: session.working_directory,
          createdAt: new Date().toISOString(),
          mode: "terminal",
          aiConfig: {
            provider: default_provider,
            model: default_model,
            status: "initializing",
          },
        });

        // Fetch git branch for the initial working directory
        try {
          const branch = await getGitBranch(session.working_directory);
          updateGitBranch(session.id, branch);
        } catch {
          // Silently ignore - not a git repo or git not installed
        }

        // Also update global config for backwards compatibility
        setAiConfig({
          provider: default_provider,
          model: default_model,
          status: "initializing",
        });

        // Initialize AI agent for this session
        try {
          const sessionAlreadyInitialized = await isAiSessionInitialized(session.id);
          if (!sessionAlreadyInitialized) {
            const config = await buildProviderConfig(settings, session.working_directory);
            await initAiSession(session.id, config);

            // Update session-specific AI config
            setSessionAiConfig(session.id, { status: "ready" });
          } else {
            // Already initialized - just update the store
            setSessionAiConfig(session.id, { status: "ready" });
          }

          // Also update global config for backwards compatibility
          setAiConfig({ status: "ready" });
        } catch (aiError) {
          console.error("Failed to initialize AI agent:", aiError);
          const errorMessage = aiError instanceof Error ? aiError.message : "Unknown error";

          setSessionAiConfig(session.id, {
            status: "error",
            errorMessage,
          });

          setAiConfig({
            provider: "",
            model: "",
            status: "error",
            errorMessage,
          });
        }

        setIsLoading(false);
      } catch (e) {
        console.error("Failed to initialize:", e);
        setError(e instanceof Error ? e.message : String(e));
        setIsLoading(false);
      }
    }

    init();
  }, [addSession, setAiConfig, setSessionAiConfig, updateGitBranch]);

  // Handle toggle mode from command palette (switches between terminal and agent)
  // NOTE: This must be defined before the keyboard shortcut useEffect that uses it
  const handleToggleMode = useCallback(() => {
    if (activeSessionId) {
      const currentSession = sessions[activeSessionId];
      const newMode = currentSession?.mode === "agent" ? "terminal" : "agent";
      setInputMode(activeSessionId, newMode);
    }
  }, [activeSessionId, sessions, setInputMode]);

  // Keyboard shortcuts
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      // Cmd+, for settings
      if ((e.metaKey || e.ctrlKey) && e.key === ",") {
        e.preventDefault();
        setSettingsOpen(true);
        return;
      }
      // Cmd+K for command palette
      if ((e.metaKey || e.ctrlKey) && e.key === "k") {
        e.preventDefault();
        setCommandPaletteOpen(true);
        return;
      }

      // Cmd+T for new tab
      if ((e.metaKey || e.ctrlKey) && e.key === "t") {
        e.preventDefault();
        handleNewTab();
        return;
      }

      // Cmd+B for sidebar toggle
      if ((e.metaKey || e.ctrlKey) && e.key === "b") {
        e.preventDefault();
        setSidebarOpen((prev) => !prev);
        return;
      }

      // Cmd+H for session browser
      if ((e.metaKey || e.ctrlKey) && e.key === "h") {
        e.preventDefault();
        setSessionBrowserOpen(true);
        return;
      }

      // Cmd+I for toggle mode
      if ((e.metaKey || e.ctrlKey) && e.key === "i") {
        e.preventDefault();
        handleToggleMode();
        return;
      }

      // Cmd+Shift+C for context panel
      if ((e.metaKey || e.ctrlKey) && e.shiftKey && e.key === "c") {
        e.preventDefault();
        openContextPanel();
        return;
      }

      // Cmd+Shift+T for task planner panel
      if ((e.metaKey || e.ctrlKey) && e.shiftKey && e.key === "t") {
        e.preventDefault();
        if (taskPlannerOpen) {
          setTaskPlannerOpen(false);
        } else {
          openTaskPlanner();
        }
        return;
      }

      // Cmd+Shift+F for full terminal mode toggle
      if ((e.metaKey || e.ctrlKey) && e.shiftKey && e.key === "f") {
        e.preventDefault();
        if (activeSessionId) {
          const currentRenderMode = sessions[activeSessionId]?.renderMode ?? "timeline";
          setRenderMode(
            activeSessionId,
            currentRenderMode === "fullterm" ? "timeline" : "fullterm"
          );
        }
        return;
      }

      // Cmd+Shift+P for sidecar panel (patches/artifacts)
      if ((e.metaKey || e.ctrlKey) && e.shiftKey && e.key === "p") {
        e.preventDefault();
        setSidecarPanelOpen(true);
        return;
      }

      // Cmd+, for settings
      if ((e.metaKey || e.ctrlKey) && e.key === ",") {
        e.preventDefault();
        setSettingsOpen(true);
        return;
      }

      // Ctrl+] for next tab
      if (e.ctrlKey && e.key === "]") {
        e.preventDefault();
        const sIds = Object.keys(sessions);
        if (activeSessionId && sIds.length > 1) {
          const idx = sIds.indexOf(activeSessionId);
          useStore.getState().setActiveSession(sIds[(idx + 1) % sIds.length]);
        }
        return;
      }

      // Ctrl+[ for previous tab
      if (e.ctrlKey && e.key === "[") {
        e.preventDefault();
        const sIds = Object.keys(sessions);
        if (activeSessionId && sIds.length > 1) {
          const idx = sIds.indexOf(activeSessionId);
          useStore.getState().setActiveSession(sIds[(idx - 1 + sIds.length) % sIds.length]);
        }
        return;
      }

      // Cmd+D: Split pane vertically (new pane to the right)
      if ((e.metaKey || e.ctrlKey) && e.key === "d" && !e.shiftKey) {
        e.preventDefault();
        handleSplitPane("vertical");
        return;
      }

      // Cmd+Shift+D: Split pane horizontally (new pane below)
      if ((e.metaKey || e.ctrlKey) && e.shiftKey && e.key === "d") {
        e.preventDefault();
        handleSplitPane("horizontal");
        return;
      }

      // Cmd+W: Close current pane (or tab if last pane)
      if ((e.metaKey || e.ctrlKey) && e.key === "w") {
        e.preventDefault();
        handleClosePane();
        return;
      }

      // Cmd+Option+Arrow: Navigate between panes
      if ((e.metaKey || e.ctrlKey) && e.altKey) {
        const directionMap: Record<string, "up" | "down" | "left" | "right"> = {
          ArrowUp: "up",
          ArrowDown: "down",
          ArrowLeft: "left",
          ArrowRight: "right",
        };
        const direction = directionMap[e.key];
        if (direction) {
          e.preventDefault();
          handleNavigatePane(direction);
          return;
        }
      }
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [
    handleNewTab,
    handleToggleMode,
    sessions,
    activeSessionId,
    openContextPanel,
    openTaskPlanner,
    taskPlannerOpen,
    setRenderMode,
    handleSplitPane,
    handleClosePane,
    handleNavigatePane,
  ]);

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
      const currentRenderMode = sessions[activeSessionId]?.renderMode ?? "timeline";
      setRenderMode(activeSessionId, currentRenderMode === "fullterm" ? "timeline" : "fullterm");
    }
  }, [activeSessionId, sessions, setRenderMode]);

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
        {isMockBrowserMode() && <MockDevTools />}
      </div>
    );
  }

  if (error) {
    return (
      <div className="flex items-center justify-center h-screen bg-[#1a1b26]">
        <div className="text-[#f7768e] text-lg">Error: {error}</div>
        {/* Mock Dev Tools - available on error in browser mode */}
        {isMockBrowserMode() && <MockDevTools />}
      </div>
    );
  }

  // Render component testbed page
  if (currentPage === "testbed") {
    return (
      <>
        <ComponentTestbed />
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
          onOpenSessionBrowser={() => setSessionBrowserOpen(true)}
          onOpenSettings={() => setSettingsOpen(true)}
        />
        <SessionBrowser
          open={sessionBrowserOpen}
          onOpenChange={setSessionBrowserOpen}
          onSessionRestore={handleRestoreSession}
        />
        <SettingsDialog open={settingsOpen} onOpenChange={setSettingsOpen} />
        {/* Mock Dev Tools - available on testbed in browser mode */}
        {isMockBrowserMode() && <MockDevTools />}
      </>
    );
  }

  return (
    <TerminalPortalProvider>
      <div className="h-screen w-screen bg-background flex flex-col overflow-hidden app-bg-layered">
        {/* Tab bar */}
        <TabBar
          onNewTab={handleNewTab}
          onToggleContext={() => {
            if (contextPanelOpen) {
              setContextPanelOpen(false);
            } else {
              openContextPanel();
            }
          }}
          onOpenHistory={() => setSessionBrowserOpen(true)}
          onOpenSettings={() => setSettingsOpen(true)}
        />

        {/* Main content area with sidebar */}
        <div className="flex-1 min-h-0 min-w-0 flex overflow-hidden">
          {/* Sidebar */}
          <Sidebar
            workingDirectory={workingDirectory}
            isOpen={sidebarOpen}
            onToggle={() => setSidebarOpen(false)}
            onFileSelect={(_filePath, _line) => {
              // File selection is handled by Sidebar internally for now
            }}
          />

          {/* Main content - Pane layout */}
          {/* Render ALL tabs but only show the active one. This keeps Terminal instances
              mounted across tab switches so fullterm apps (claude, codex) don't lose state. */}
          <div className="flex-1 min-h-0 min-w-0 flex flex-col overflow-hidden relative">
            {Object.entries(tabLayouts).map(([tabId, layout]) => (
              <div
                key={tabId}
                className={`absolute inset-0 ${tabId === activeSessionId ? "visible" : "invisible pointer-events-none"}`}
              >
                <PaneContainer node={layout.root} tabId={tabId} />
              </div>
            ))}
            {!activeSessionId && (
              <div className="flex items-center justify-center h-full">
                <span className="text-[#565f89]">No active session</span>
              </div>
            )}
          </div>

          {/* Context Panel - integrated side panel, uses sidecar's current session */}
          <ContextPanel open={contextPanelOpen} onOpenChange={setContextPanelOpen} />

          {/* Task Planner Panel - right side panel showing task progress */}
          <TaskPlannerPanel
            open={taskPlannerOpen}
            onOpenChange={setTaskPlannerOpen}
            sessionId={focusedSessionId}
          />
        </div>

        {/* Terminal Layer - renders all Terminal instances via React portals.
            Terminals are rendered here (at a stable position in the tree) and portaled
            into their respective PaneLeaf targets. This prevents Terminal unmount/remount
            when pane structure changes during splits. */}
        <TerminalLayer />

        {/* Status bar at the very bottom - shows info for the focused pane's session */}
        <StatusBar sessionId={focusedSessionId} onOpenTaskPlanner={openTaskPlanner} />

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
          onToggleSidebar={() => setSidebarOpen((prev) => !prev)}
          onToggleFullTerminal={handleToggleFullTerminal}
          workingDirectory={workingDirectory}
          onOpenSessionBrowser={() => setSessionBrowserOpen(true)}
          onOpenContextPanel={openContextPanel}
          onOpenTaskPlanner={openTaskPlanner}
          onOpenSettings={() => setSettingsOpen(true)}
          onSplitPaneRight={() => handleSplitPane("vertical")}
          onSplitPaneDown={() => handleSplitPane("horizontal")}
          onClosePane={handleClosePane}
        />

        {/* Sidecar Panel (Patches & Artifacts) */}
        <SidecarPanel open={sidecarPanelOpen} onOpenChange={setSidecarPanelOpen} />

        {/* Session Browser */}
        <SessionBrowser
          open={sessionBrowserOpen}
          onOpenChange={setSessionBrowserOpen}
          onSessionRestore={handleRestoreSession}
        />

        {/* Settings Dialog */}
        <SettingsDialog open={settingsOpen} onOpenChange={setSettingsOpen} />

        {/* Sidecar event notifications */}
        <SidecarNotifications />

        {/* Mock Dev Tools - only in browser mode */}
        {isMockBrowserMode() && <MockDevTools />}
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
