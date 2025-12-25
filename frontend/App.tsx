import { useCallback, useEffect, useRef, useState } from "react";
import { ToolApprovalDialog } from "./components/AgentChat";
import { CommandPalette, type PageRoute } from "./components/CommandPalette";
import { MockDevTools, MockDevToolsProvider } from "./components/MockDevTools";
import { SessionBrowser } from "./components/SessionBrowser";
import { SettingsDialog } from "./components/Settings";
import { Sidebar } from "./components/Sidebar";
import { ContextPanel, SidecarNotifications, SidecarPanel } from "./components/Sidecar";
import { StatusBar } from "./components/StatusBar";
import { TabBar } from "./components/TabBar";
import { TaskPlannerPanel } from "./components/TaskPlannerPanel";
import { Terminal } from "./components/Terminal";
import { UnifiedInput } from "./components/UnifiedInput";
import { UnifiedTimeline } from "./components/UnifiedTimeline";
import { Skeleton } from "./components/ui/skeleton";
import { useAiEvents } from "./hooks/useAiEvents";
import { useTauriEvents } from "./hooks/useTauriEvents";
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
import { getSettings, type QbitSettings } from "./lib/settings";
import { ptyCreate, shellIntegrationInstall, shellIntegrationStatus } from "./lib/tauri";
import { isMockBrowserMode } from "./mocks";
import { ComponentTestbed } from "./pages/ComponentTestbed";
import { clearConversation, restoreSession, useRenderMode, useStore } from "./store";

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
    setInputMode,
    setAiConfig,
    setSessionAiConfig,
    setRenderMode,
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

  // Get current session's working directory
  const activeSession = activeSessionId ? sessions[activeSessionId] : null;
  const workingDirectory = activeSession?.workingDirectory;

  // Get render mode for current session (timeline vs fullterm)
  const renderMode = useRenderMode(activeSessionId ?? "");

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
  }, [addSession, setAiConfig, setSessionAiConfig]);

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
  }, [addSession, setAiConfig, setSessionAiConfig]);

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

        {/* Main content */}
        <div className="flex-1 min-h-0 min-w-0 flex flex-col overflow-hidden">
          {/* Render all fullterm terminals but only show active one - preserves state on tab switch */}
          {Object.entries(sessions).map(([sessionId, session]) => {
            const isActive = sessionId === activeSessionId;
            const isFullterm = session.renderMode === "fullterm";
            // Only render terminal if session is/was in fullterm mode
            if (!isFullterm) return null;
            return (
              <div
                key={sessionId}
                className="flex-1 min-h-0"
                style={{ display: isActive ? "flex" : "none" }}
              >
                <Terminal sessionId={sessionId} />
              </div>
            );
          })}

          {activeSessionId ? (
            renderMode !== "fullterm" ? (
              // Timeline mode - structured display with UnifiedInput
              <>
                {/* Scrollable content area - auto-scroll handled in UnifiedTimeline */}
                <div className="flex-1 min-w-0 overflow-auto">
                  <UnifiedTimeline sessionId={activeSessionId} />
                </div>

                {/* Unified input at bottom */}
                <UnifiedInput sessionId={activeSessionId} workingDirectory={workingDirectory} />

                {/* Tool approval dialog */}
                <ToolApprovalDialog sessionId={activeSessionId} />
              </>
            ) : null
          ) : (
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
          sessionId={activeSessionId}
        />
      </div>

      {/* Status bar at the very bottom */}
      <StatusBar sessionId={activeSessionId} onOpenTaskPlanner={openTaskPlanner} />

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
