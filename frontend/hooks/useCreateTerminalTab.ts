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
import { getSettingsCached } from "@/lib/settings";
import { getGitBranch, gitStatus, ptyCreate } from "@/lib/tauri";
import { useStore } from "@/store";

/**
 * Hook that provides a function to create new terminal tabs.
 * Handles PTY creation, AI session initialization, git status, and project settings.
 */
export function useCreateTerminalTab() {
  const createTerminalTab = useCallback(
    async (workingDirectory?: string): Promise<string | null> => {
      const {
        addSession,
        setAiConfig,
        setSessionAiConfig,
        updateGitBranch,
        setGitStatus,
        setGitStatusLoading,
        setAgentMode,
      } = useStore.getState();

      try {
        // Only await PTY creation - this is the minimum needed to show the terminal
        const session = await ptyCreate(workingDirectory);

        // Add session immediately with default AI config (will be updated in background)
        addSession({
          id: session.id,
          name: "Terminal",
          workingDirectory: session.working_directory,
          createdAt: new Date().toISOString(),
          mode: "terminal",
          aiConfig: {
            provider: "",
            model: "",
            status: "initializing",
          },
        });

        // All remaining work happens in the background (non-blocking)
        void (async () => {
          // Fetch settings and project settings in parallel
          const [settings, projectSettings] = await Promise.all([
            getSettingsCached(),
            getProjectSettings(session.working_directory).catch((e) => {
              logger.warn("Failed to load project settings:", e);
              return { provider: null, model: null, agent_mode: null } as {
                provider: AiProvider | null;
                model: string | null;
                agent_mode: string | null;
              };
            }),
          ]);

          // Notify if project settings were loaded
          if (projectSettings.provider || projectSettings.model || projectSettings.agent_mode) {
            const parts: string[] = [];
            if (projectSettings.provider) parts.push(projectSettings.provider);
            if (projectSettings.model) parts.push(projectSettings.model);
            if (projectSettings.agent_mode) parts.push(projectSettings.agent_mode);
            notify.info(`Project settings loaded: ${parts.join(", ")}`);
          }

          const { default_provider, default_model } = settings.ai;
          const effectiveProvider = projectSettings.provider ?? default_provider;
          const effectiveModel = projectSettings.model ?? default_model;

          // Update session AI config with resolved provider/model
          setSessionAiConfig(session.id, {
            provider: effectiveProvider,
            model: effectiveModel,
          });

          // Fetch git branch and status in parallel
          setGitStatusLoading(session.id, true);
          try {
            const [branch, status] = await Promise.all([
              getGitBranch(session.working_directory),
              gitStatus(session.working_directory),
            ]);
            updateGitBranch(session.id, branch);
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
        })();

        return session.id;
      } catch (e) {
        logger.error("Failed to create new tab:", e);
        notify.error("Failed to create new tab");
        return null;
      }
    },
    []
  );

  return { createTerminalTab };
}
