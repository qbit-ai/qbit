import { useCallback } from "react";
import {
  type AiProvider,
  buildProviderConfig,
  getProjectSettings,
  initAiSession,
  setAgentMode as setAgentModeBackend,
} from "@/lib/ai";
import { logger } from "@/lib/logger";
import { notify } from "@/lib/notify";
import { getSettings } from "@/lib/settings";
import { getGitBranch, gitStatus, ptyCreate } from "@/lib/tauri";
import { useStore } from "@/store";

/**
 * Hook that provides a function to create new terminal tabs.
 * Handles PTY creation, AI session initialization, git status, and project settings.
 */
export function useCreateTerminalTab() {
  const {
    addSession,
    setAiConfig,
    setSessionAiConfig,
    updateGitBranch,
    setGitStatus,
    setGitStatusLoading,
    setAgentMode,
  } = useStore();

  /**
   * Create a new terminal tab in the specified directory.
   * @param workingDirectory - Directory to open the terminal in (uses default if not specified)
   * @returns The session ID of the created tab, or null if creation failed
   */
  const createTerminalTab = useCallback(
    async (workingDirectory?: string): Promise<string | null> => {
      try {
        const session = await ptyCreate(workingDirectory);
        const settings = await getSettings();

        // Load project settings for overrides
        let projectSettings: {
          provider: AiProvider | null;
          model: string | null;
          agent_mode: string | null;
        } = {
          provider: null,
          model: null,
          agent_mode: null,
        };
        try {
          projectSettings = await getProjectSettings(session.working_directory);
          // Notify if project settings were loaded
          if (projectSettings.provider || projectSettings.model || projectSettings.agent_mode) {
            const parts: string[] = [];
            if (projectSettings.provider) parts.push(projectSettings.provider);
            if (projectSettings.model) parts.push(projectSettings.model);
            if (projectSettings.agent_mode) parts.push(projectSettings.agent_mode);
            notify.info(`Project settings loaded: ${parts.join(", ")}`);
          }
        } catch (projectError) {
          logger.warn("Failed to load project settings:", projectError);
        }

        const { default_provider, default_model } = settings.ai;

        // Apply project setting overrides if available
        const effectiveProvider = projectSettings.provider ?? default_provider;
        const effectiveModel = projectSettings.model ?? default_model;

        // Add session with initial AI config
        addSession({
          id: session.id,
          name: "Terminal",
          workingDirectory: session.working_directory,
          createdAt: new Date().toISOString(),
          mode: "terminal",
          aiConfig: {
            provider: effectiveProvider,
            model: effectiveModel,
            status: "initializing",
          },
        });

        // Fetch git branch and status for the working directory
        setGitStatusLoading(session.id, true);
        try {
          const branch = await getGitBranch(session.working_directory);
          updateGitBranch(session.id, branch);
          const status = await gitStatus(session.working_directory);
          setGitStatus(session.id, status);
        } catch {
          // Silently ignore - not a git repo or git not installed
        } finally {
          setGitStatusLoading(session.id, false);
        }

        // Also update global config for backwards compatibility
        setAiConfig({
          provider: effectiveProvider,
          model: effectiveModel,
          status: "initializing",
        });

        // Initialize AI for this specific session
        try {
          const config = await buildProviderConfig(settings, session.working_directory, {
            provider: projectSettings.provider,
            model: projectSettings.model,
          });
          await initAiSession(session.id, config);

          // Update session-specific AI config
          setSessionAiConfig(session.id, { status: "ready" });

          // Apply agent mode from project settings if set
          if (projectSettings.agent_mode) {
            const mode = projectSettings.agent_mode as "default" | "auto-approve" | "planning";
            // Update UI state
            setAgentMode(session.id, mode);
            // Update backend state
            try {
              await setAgentModeBackend(session.id, mode);
            } catch (err) {
              logger.warn("Failed to set agent mode on backend:", err);
            }
          }

          // Also update global config for backwards compatibility
          setAiConfig({ status: "ready" });
        } catch (aiError) {
          logger.error("Failed to initialize AI for new tab:", aiError);
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

        return session.id;
      } catch (e) {
        logger.error("Failed to create new tab:", e);
        notify.error("Failed to create new tab");
        return null;
      }
    },
    [
      addSession,
      setAiConfig,
      setSessionAiConfig,
      updateGitBranch,
      setGitStatus,
      setGitStatusLoading,
      setAgentMode,
    ]
  );

  return { createTerminalTab };
}
