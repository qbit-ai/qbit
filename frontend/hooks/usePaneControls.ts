import { useCallback } from "react";
import { buildProviderConfig, initAiSession } from "@/lib/ai";
import { logger } from "@/lib/logger";
import { notify } from "@/lib/notify";
import { countLeafPanes, findPaneById } from "@/lib/pane-utils";
import { getSettings } from "@/lib/settings";
import { getGitBranch, ptyCreate } from "@/lib/tauri";
import { type SplitDirection, useStore } from "@/store";

export function usePaneControls(activeSessionId: string | null) {
  const addSession = useStore((state) => state.addSession);
  const splitPane = useStore((state) => state.splitPane);
  const closePane = useStore((state) => state.closePane);
  const removeSession = useStore((state) => state.removeSession);
  const navigatePane = useStore((state) => state.navigatePane);
  const updateGitBranch = useStore((state) => state.updateGitBranch);
  const setSessionAiConfig = useStore((state) => state.setSessionAiConfig);

  const handleSplitPane = useCallback(
    async (direction: SplitDirection) => {
      if (!activeSessionId) return;

      const tabLayout = useStore.getState().tabLayouts[activeSessionId];
      if (!tabLayout) return;

      const currentCount = countLeafPanes(tabLayout.root);
      if (currentCount >= 4) {
        notify.warning("Maximum pane limit (4) reached");
        return;
      }

      const focusedPane = findPaneById(tabLayout.root, tabLayout.focusedPaneId);
      if (!focusedPane || focusedPane.type !== "leaf") return;

      const sourceSession = useStore.getState().sessions[focusedPane.sessionId];
      if (!sourceSession) return;

      try {
        const newSession = await ptyCreate(sourceSession.workingDirectory);
        const settings = await getSettings();
        const { default_provider, default_model } = settings.ai;

        const newPaneId = crypto.randomUUID();

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

        try {
          const branch = await getGitBranch(newSession.working_directory);
          updateGitBranch(newSession.id, branch);
        } catch {
          // Not a git repo or git not installed
        }

        try {
          const config = await buildProviderConfig(settings, newSession.working_directory);
          await initAiSession(newSession.id, config);
          setSessionAiConfig(newSession.id, { status: "ready" });
        } catch (aiError) {
          logger.error("Failed to initialize AI for new pane:", aiError);
          const errorMessage = aiError instanceof Error ? aiError.message : "Unknown error";
          setSessionAiConfig(newSession.id, { status: "error", errorMessage });
        }

        splitPane(activeSessionId, tabLayout.focusedPaneId, direction, newPaneId, newSession.id);
      } catch (e) {
        logger.error("Failed to split pane:", e);
        notify.error("Failed to split pane");
      }
    },
    [activeSessionId, addSession, splitPane, updateGitBranch, setSessionAiConfig]
  );

  const handleClosePane = useCallback(async () => {
    if (!activeSessionId) return;

    const tabLayout = useStore.getState().tabLayouts[activeSessionId];
    if (!tabLayout) return;

    const focusedPane = findPaneById(tabLayout.root, tabLayout.focusedPaneId);
    if (!focusedPane || focusedPane.type !== "leaf") return;

    const sessionIdToClose = focusedPane.sessionId;
    const isLastPane = countLeafPanes(tabLayout.root) === 1;

    try {
      try {
        const { shutdownAiSession } = await import("@/lib/ai");
        await shutdownAiSession(sessionIdToClose);
      } catch {
        // Session may not have been initialized
      }

      try {
        const { ptyDestroy } = await import("@/lib/tauri");
        await ptyDestroy(sessionIdToClose);
      } catch {
        // PTY may already be destroyed
      }

      if (isLastPane) {
        if (sessionIdToClose !== activeSessionId) {
          closePane(activeSessionId, tabLayout.focusedPaneId);
        }
        removeSession(activeSessionId);
      } else {
        closePane(activeSessionId, tabLayout.focusedPaneId);
      }
    } catch (e) {
      logger.error("Failed to close pane:", e);
      notify.error("Failed to close pane");
    }
  }, [activeSessionId, closePane, removeSession]);

  const handleNavigatePane = useCallback(
    (direction: "up" | "down" | "left" | "right") => {
      if (!activeSessionId) return;
      navigatePane(activeSessionId, direction);
    },
    [activeSessionId, navigatePane]
  );

  return { handleSplitPane, handleClosePane, handleNavigatePane };
}
