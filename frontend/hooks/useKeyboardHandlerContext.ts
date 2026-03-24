/**
 * Keyboard Handler Context Hook
 *
 * Provides a stable ref containing the current values needed by keyboard handlers.
 * This pattern allows the keyboard event listener to be set up once (empty deps)
 * while still having access to the latest state values.
 *
 * The ref is updated whenever relevant state changes, but the ref object itself
 * remains stable, so components using it don't need to re-render.
 */

import { useEffect, useRef } from "react";
import type { SplitDirection } from "@/lib/pane-utils";
import { useStore } from "@/store";

/**
 * Context values needed by keyboard handlers.
 * These are stored in a ref to avoid recreating the keyboard handler
 * every time any of these values change.
 */
export interface KeyboardHandlerContext {
  // Session state
  activeSessionId: string | null;

  // Panel state
  gitPanelOpen: boolean;

  // Callbacks
  handleNewTab: () => void;
  handleToggleMode: () => void;
  openContextPanel: () => void;
  openGitPanel: () => void;
  toggleFileEditorPanel: () => void;
  openSettingsTab: () => void;
  handleSplitPane: (direction: SplitDirection) => Promise<void>;
  handleClosePane: () => Promise<void>;
  handleNavigatePane: (direction: "up" | "down" | "left" | "right") => void;

  // Setters (for local state)
  setCommandPaletteOpen: (open: boolean) => void;
  setQuickOpenDialogOpen: (open: boolean) => void;
  setSidecarPanelOpen: (open: boolean) => void;
}

const defaultContext: KeyboardHandlerContext = {
  activeSessionId: null,
  gitPanelOpen: false,
  handleNewTab: () => {},
  handleToggleMode: () => {},
  openContextPanel: () => {},
  openGitPanel: () => {},
  toggleFileEditorPanel: () => {},
  openSettingsTab: () => {},
  handleSplitPane: async () => {},
  handleClosePane: async () => {},
  handleNavigatePane: () => {},
  setCommandPaletteOpen: () => {},
  setQuickOpenDialogOpen: () => {},
  setSidecarPanelOpen: () => {},
};

/**
 * Hook that provides a stable ref for keyboard handler context.
 *
 * The ref is updated whenever relevant state changes, but the ref object
 * itself remains stable, allowing keyboard handlers to be set up once.
 *
 * Usage:
 * ```tsx
 * const keyboardContextRef = useKeyboardHandlerContext();
 *
 * useEffect(() => {
 *   // Update ref with current callbacks
 *   keyboardContextRef.current = {
 *     ...keyboardContextRef.current,
 *     handleNewTab,
 *     handleToggleMode,
 *     // ... other callbacks
 *   };
 * }, [handleNewTab, handleToggleMode, ...]);
 *
 * useEffect(() => {
 *   const handleKeyDown = (e: KeyboardEvent) => {
 *     const ctx = keyboardContextRef.current;
 *     // Use ctx.handleNewTab(), etc.
 *   };
 *   window.addEventListener("keydown", handleKeyDown);
 *   return () => window.removeEventListener("keydown", handleKeyDown);
 * }, []); // Empty deps - handler never recreated
 * ```
 */
export function useKeyboardHandlerContext() {
  const contextRef = useRef<KeyboardHandlerContext>(defaultContext);

  // Subscribe to activeSessionId from store
  const activeSessionId = useStore((state) => state.activeSessionId);

  // Update ref when activeSessionId changes
  useEffect(() => {
    contextRef.current = {
      ...contextRef.current,
      activeSessionId,
    };
  }, [activeSessionId]);

  return contextRef;
}

/**
 * Creates a stable keyboard handler that reads values from a context ref.
 *
 * This is the actual handler that gets added to the window.
 * It reads current values from the ref instead of closing over them.
 */
export function createKeyboardHandler(
  contextRef: React.MutableRefObject<KeyboardHandlerContext>
): (e: KeyboardEvent) => void {
  return (e: KeyboardEvent) => {
    const ctx = contextRef.current;

    // Cmd+, for settings
    if ((e.metaKey || e.ctrlKey) && e.key === ",") {
      e.preventDefault();
      ctx.openSettingsTab();
      return;
    }

    // Cmd+K for command palette
    if ((e.metaKey || e.ctrlKey) && e.key === "k") {
      e.preventDefault();
      ctx.setCommandPaletteOpen(true);
      return;
    }

    // Cmd+T for new tab
    if ((e.metaKey || e.ctrlKey) && e.key === "t") {
      e.preventDefault();
      ctx.handleNewTab();
      return;
    }

    // Cmd+[1-9] for tab switching - read sessions at event time
    if (e.metaKey && !e.shiftKey && !e.altKey && e.key >= "1" && e.key <= "9") {
      const tabIndex = parseInt(e.key, 10) - 1;
      const tabIds = Object.keys(useStore.getState().sessions);
      if (tabIndex < tabIds.length) {
        e.preventDefault();
        useStore.getState().setActiveSession(tabIds[tabIndex]);
      }
      return;
    }

    // Cmd+I for toggle mode
    if ((e.metaKey || e.ctrlKey) && e.key === "i") {
      e.preventDefault();
      ctx.handleToggleMode();
      return;
    }

    // Cmd+Shift+C for context panel
    if ((e.metaKey || e.ctrlKey) && e.shiftKey && e.key === "c") {
      e.preventDefault();
      ctx.openContextPanel();
      return;
    }

    // Cmd+Shift+G for git panel
    if ((e.metaKey || e.ctrlKey) && e.shiftKey && e.key === "g") {
      e.preventDefault();
      if (ctx.gitPanelOpen) {
        // Close - need to use a setter, but we don't have direct access
        // The App component will handle this through the ref
      } else {
        ctx.openGitPanel();
      }
      return;
    }

    // Cmd+Shift+E for file editor panel
    if ((e.metaKey || e.ctrlKey) && e.shiftKey && e.key === "e") {
      e.preventDefault();
      ctx.toggleFileEditorPanel();
      return;
    }

    // Cmd+Shift+F for full terminal mode toggle
    if ((e.metaKey || e.ctrlKey) && e.shiftKey && e.key === "f") {
      e.preventDefault();
      const { activeSessionId } = ctx;
      if (activeSessionId) {
        const state = useStore.getState();
        const currentRenderMode = state.sessions[activeSessionId]?.renderMode ?? "timeline";
        state.setRenderMode(
          activeSessionId,
          currentRenderMode === "fullterm" ? "timeline" : "fullterm"
        );
      }
      return;
    }

    // Cmd+P for quick open file (without Shift)
    if ((e.metaKey || e.ctrlKey) && !e.shiftKey && e.key === "p") {
      e.preventDefault();
      ctx.setQuickOpenDialogOpen(true);
      return;
    }

    // Cmd+Shift+P for sidecar panel
    if ((e.metaKey || e.ctrlKey) && e.shiftKey && e.key === "p") {
      e.preventDefault();
      ctx.setSidecarPanelOpen(true);
      return;
    }

    // Ctrl+] for next tab
    if (e.ctrlKey && e.key === "]") {
      e.preventDefault();
      const sIds = Object.keys(useStore.getState().sessions);
      const { activeSessionId } = ctx;
      if (activeSessionId && sIds.length > 1) {
        const idx = sIds.indexOf(activeSessionId);
        useStore.getState().setActiveSession(sIds[(idx + 1) % sIds.length]);
      }
      return;
    }

    // Ctrl+[ for previous tab
    if (e.ctrlKey && e.key === "[") {
      e.preventDefault();
      const sIds = Object.keys(useStore.getState().sessions);
      const { activeSessionId } = ctx;
      if (activeSessionId && sIds.length > 1) {
        const idx = sIds.indexOf(activeSessionId);
        useStore.getState().setActiveSession(sIds[(idx - 1 + sIds.length) % sIds.length]);
      }
      return;
    }

    // Cmd+D: Split pane vertically
    if (e.metaKey && e.key === "d" && !e.shiftKey) {
      e.preventDefault();
      ctx.handleSplitPane("vertical");
      return;
    }

    // Cmd+Shift+D: Split pane horizontally
    if (e.metaKey && e.shiftKey && e.key === "d") {
      e.preventDefault();
      ctx.handleSplitPane("horizontal");
      return;
    }

    // Cmd+W: Close current pane
    if ((e.metaKey || e.ctrlKey) && e.key === "w") {
      e.preventDefault();
      ctx.handleClosePane();
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
        ctx.handleNavigatePane(direction);
        return;
      }
    }
  };
}
