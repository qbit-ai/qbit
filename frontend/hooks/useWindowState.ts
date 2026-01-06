import { logger } from "@/lib/logger";
/**
 * Hook to persist window state (size, position) across app restarts.
 *
 * This hook:
 * - Restores window state from settings on mount
 * - Saves window state to settings when the window is resized or moved
 * - Debounces save operations to avoid excessive writes
 */

import { getCurrentWindow, LogicalPosition, LogicalSize } from "@tauri-apps/api/window";
import { useCallback, useEffect, useRef } from "react";
import { getWindowState, saveWindowState } from "../lib/settings";
import { isMockBrowserMode } from "../mocks";

const SAVE_DEBOUNCE_MS = 500;

export function useWindowState() {
  const saveTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const isInitializedRef = useRef(false);
  const restoreAttemptedRef = useRef(false);

  // Debounced save function
  const debouncedSave = useCallback(async () => {
    if (isMockBrowserMode()) return;

    const window = getCurrentWindow();
    try {
      const [size, position, isMaximized] = await Promise.all([
        window.innerSize(),
        window.outerPosition(),
        window.isMaximized(),
      ]);

      await saveWindowState({
        width: size.width,
        height: size.height,
        x: position.x,
        y: position.y,
        maximized: isMaximized,
      });
    } catch (error) {
      logger.error("Failed to save window state:", error);
    }
  }, []);

  // Schedule a debounced save
  const scheduleSave = useCallback(() => {
    if (saveTimeoutRef.current) {
      clearTimeout(saveTimeoutRef.current);
    }
    saveTimeoutRef.current = setTimeout(debouncedSave, SAVE_DEBOUNCE_MS);
  }, [debouncedSave]);

  useEffect(() => {
    if (isMockBrowserMode()) return;

    const window = getCurrentWindow();
    let unlistenResize: (() => void) | null = null;
    let unlistenMove: (() => void) | null = null;

    const setup = async () => {
      // Guard against double-execution in React StrictMode
      if (restoreAttemptedRef.current) return;
      restoreAttemptedRef.current = true;

      // Restore window state on mount
      try {
        const state = await getWindowState();

        // Only restore if we have valid saved state
        if (state.width > 0 && state.height > 0) {
          if (state.maximized) {
            await window.maximize();
          } else {
            // Set size first
            await window.setSize(new LogicalSize(state.width, state.height));

            // Set position if we have one (otherwise let OS center it)
            if (state.x !== null && state.y !== null) {
              await window.setPosition(new LogicalPosition(state.x, state.y));
            }
          }
        }

        isInitializedRef.current = true;
      } catch (error) {
        logger.error("Failed to restore window state:", error);
        isInitializedRef.current = true;
      }

      // Listen for resize events
      unlistenResize = await window.onResized(() => {
        if (isInitializedRef.current) {
          scheduleSave();
        }
      });

      // Listen for move events
      unlistenMove = await window.onMoved(() => {
        if (isInitializedRef.current) {
          scheduleSave();
        }
      });
    };

    setup();

    // Cleanup
    return () => {
      if (saveTimeoutRef.current) {
        clearTimeout(saveTimeoutRef.current);
      }
      if (unlistenResize) {
        unlistenResize();
      }
      if (unlistenMove) {
        unlistenMove();
      }
    };
  }, [scheduleSave]);
}
