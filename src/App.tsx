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
  initAiAgent,
  initAiAgentUnified,
  initOpenAiAgent,
  initVertexAiAgent,
  isAiInitialized,
  updateAiWorkspace,
} from "./lib/ai";
import {
  getIndexedFileCount,
  indexDirectory,
  initIndexer,
  isIndexerInitialized,
} from "./lib/indexer";
import { notify } from "./lib/notify";
import { getSettings, type QbitSettings } from "./lib/settings";
import { ptyCreate, shellIntegrationInstall, shellIntegrationStatus } from "./lib/tauri";
import { isMockBrowserMode } from "./mocks";
import { ComponentTestbed } from "./pages/ComponentTestbed";
import type { AiConfig } from "./store";
import { clearConversation, restoreSession, useStore } from "./store";

/**
 * Initialize the AI agent based on settings.
 * Returns the provider config to set in the store.
 */
async function initializeProvider(
  settings: QbitSettings,
  workspace: string
): Promise<Partial<AiConfig>> {
  const { default_provider, default_model } = settings.ai;

  switch (default_provider) {
    case "vertex_ai": {
      const { vertex_ai } = settings.ai;
      if (!vertex_ai.credentials_path || !vertex_ai.project_id) {
        throw new Error("Vertex AI credentials not configured");
      }
      await initVertexAiAgent({
        workspace,
        credentialsPath: vertex_ai.credentials_path,
        projectId: vertex_ai.project_id,
        location: vertex_ai.location || "us-east5",
        model: default_model,
      });
      return {
        provider: "anthropic_vertex",
        model: default_model,
        vertexConfig: {
          workspace,
          credentialsPath: vertex_ai.credentials_path,
          projectId: vertex_ai.project_id,
          location: vertex_ai.location || "us-east5",
        },
      };
    }

    case "anthropic": {
      const apiKey = settings.ai.anthropic.api_key || (await getAnthropicApiKey());
      if (!apiKey) throw new Error("Anthropic API key not configured");
      await initAiAgent({ workspace, provider: "anthropic", model: default_model, apiKey });
      return { provider: "anthropic", model: default_model };
    }

    case "openai": {
      const apiKey = settings.ai.openai.api_key || (await getOpenAiApiKey());
      if (!apiKey) throw new Error("OpenAI API key not configured");
      await initOpenAiAgent({ workspace, model: default_model, apiKey });
      return { provider: "openai", model: default_model };
    }

    case "openrouter": {
      const apiKey = settings.ai.openrouter.api_key || (await getOpenRouterApiKey());
      if (!apiKey) throw new Error("OpenRouter API key not configured");
      await initAiAgent({ workspace, provider: "openrouter", model: default_model, apiKey });
      return { provider: "openrouter", model: default_model };
    }

    case "ollama": {
      const baseUrl = settings.ai.ollama.base_url;
      await initAiAgentUnified({
        provider: "ollama",
        workspace,
        model: default_model,
        base_url: baseUrl,
      });
      return { provider: "ollama", model: default_model };
    }

    case "gemini": {
      const apiKey = settings.ai.gemini.api_key;
      if (!apiKey) throw new Error("Gemini API key not configured");
      await initAiAgentUnified({
        provider: "gemini",
        workspace,
        model: default_model,
        api_key: apiKey,
      });
      return { provider: "gemini", model: default_model };
    }

    case "groq": {
      const apiKey = settings.ai.groq.api_key;
      if (!apiKey) throw new Error("Groq API key not configured");
      await initAiAgentUnified({
        provider: "groq",
        workspace,
        model: default_model,
        api_key: apiKey,
      });
      return { provider: "groq", model: default_model };
    }

    case "xai": {
      const apiKey = settings.ai.xai.api_key;
      if (!apiKey) throw new Error("xAI API key not configured");
      await initAiAgentUnified({
        provider: "xai",
        workspace,
        model: default_model,
        api_key: apiKey,
      });
      return { provider: "xai", model: default_model };
    }

    default:
      throw new Error(`Unknown provider: ${default_provider}`);
  }
}

function App() {
  const { addSession, activeSessionId, sessions, setInputMode, setAiConfig } = useStore();
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [commandPaletteOpen, setCommandPaletteOpen] = useState(false);
  const [sessionBrowserOpen, setSessionBrowserOpen] = useState(false);
  const [contextPanelOpen, setContextPanelOpen] = useState(false);
  const [sidecarPanelOpen, setSidecarPanelOpen] = useState(false);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [currentPage, setCurrentPage] = useState<PageRoute>("main");
  const [sidebarOpen, setSidebarOpen] = useState(false);
  const initializingRef = useRef(false);

  // Get current session's working directory
  const activeSession = activeSessionId ? sessions[activeSessionId] : null;
  const workingDirectory = activeSession?.workingDirectory;

  // Connect Tauri events to store
  useTauriEvents();

  // Subscribe to AI events for agent mode
  useAiEvents();

  // Create a new terminal tab
  const handleNewTab = useCallback(async () => {
    try {
      const session = await ptyCreate();
      addSession({
        id: session.id,
        name: "Terminal",
        workingDirectory: session.working_directory,
        createdAt: new Date().toISOString(),
        mode: "terminal",
      });

      // Reinitialize AI with default model from settings for the new tab
      try {
        const settings = await getSettings();
        const { default_provider, default_model } = settings.ai;

        setAiConfig({
          provider: default_provider,
          model: default_model,
          status: "initializing",
        });

        const providerConfig = await initializeProvider(settings, session.working_directory);
        setAiConfig({ ...providerConfig, status: "ready" });
      } catch (aiError) {
        console.error("Failed to initialize AI for new tab:", aiError);
        setAiConfig({
          provider: "",
          model: "",
          status: "error",
          errorMessage: aiError instanceof Error ? aiError.message : "Unknown error",
        });
      }
    } catch (e) {
      console.error("Failed to create new tab:", e);
      notify.error("Failed to create new tab");
    }
  }, [addSession, setAiConfig]);

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
        addSession({
          id: session.id,
          name: "Terminal",
          workingDirectory: session.working_directory,
          createdAt: new Date().toISOString(),
          mode: "terminal",
        });

        // Initialize code indexer for the workspace (without auto-indexing)
        // Users can manually trigger indexing via command palette or sidebar
        try {
          const indexerInitialized = await isIndexerInitialized();
          if (!indexerInitialized && session.working_directory) {
            await initIndexer(session.working_directory);
          }

          // Check if workspace has been indexed, auto-index if not
          const fileCount = await getIndexedFileCount();
          if (fileCount === 0 && session.working_directory) {
            notify.info("Indexing workspace...", {
              message: "Enable code search and symbol navigation",
            });
            try {
              const result = await indexDirectory(session.working_directory);
              notify.success(`Indexed ${result.files_indexed} files`);
            } catch (err) {
              notify.error(`Indexing failed: ${err}`);
            }
          }
        } catch (indexerError) {
          console.warn("Failed to initialize code indexer:", indexerError);
          // Non-fatal - indexer is optional
        }

        // Initialize AI agent using settings
        try {
          const settings = await getSettings();
          const { default_provider, default_model } = settings.ai;

          const alreadyInitialized = await isAiInitialized();
          if (!alreadyInitialized) {
            setAiConfig({
              provider: default_provider,
              model: default_model,
              status: "initializing",
            });

            const providerConfig = await initializeProvider(settings, session.working_directory);
            setAiConfig({ ...providerConfig, status: "ready" });

            // Sync AI workspace with the session's current working directory
            // The shell may have already reported a directory change before AI initialized
            const currentSession = useStore.getState().sessions[session.id];
            if (
              currentSession?.workingDirectory &&
              currentSession.workingDirectory !== session.working_directory
            ) {
              await updateAiWorkspace(currentSession.workingDirectory);
            }
          } else {
            // Already initialized from previous session - just update store with settings
            const providerConfig = await initializeProvider(settings, session.working_directory);
            setAiConfig({ ...providerConfig, status: "ready" });
          }
        } catch (aiError) {
          console.error("Failed to initialize AI agent:", aiError);
          setAiConfig({
            provider: "",
            model: "",
            status: "error",
            errorMessage: aiError instanceof Error ? aiError.message : "Unknown error",
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
  }, [addSession, setAiConfig]);

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
        setContextPanelOpen(true);
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
  }, [handleNewTab, handleToggleMode, sessions, activeSessionId]);

  // Handle clear conversation from command palette
  const handleClearConversation = useCallback(async () => {
    if (activeSessionId) {
      await clearConversation(activeSessionId);
      notify.success("Conversation cleared");
    }
  }, [activeSessionId]);

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
        onToggleContext={() => setContextPanelOpen((prev) => !prev)}
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
          {activeSessionId ? (
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
          ) : (
            <div className="flex items-center justify-center h-full">
              <span className="text-[#565f89]">No active session</span>
            </div>
          )}
        </div>

        {/* Context Panel - integrated side panel, uses sidecar's current session */}
        <ContextPanel open={contextPanelOpen} onOpenChange={setContextPanelOpen} />
      </div>

      {/* Status bar at the very bottom */}
      <StatusBar sessionId={activeSessionId} />

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
        workingDirectory={workingDirectory}
        onOpenSessionBrowser={() => setSessionBrowserOpen(true)}
        onOpenContextPanel={() => setContextPanelOpen(true)}
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
