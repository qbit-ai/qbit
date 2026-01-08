import { listen } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-shell";
import { FitAddon } from "@xterm/addon-fit";
import { WebLinksAddon } from "@xterm/addon-web-links";
import { WebglAddon } from "@xterm/addon-webgl";
import { Terminal as XTerm } from "@xterm/xterm";
import { useCallback, useEffect, useLayoutEffect, useRef } from "react";
import { logger } from "@/lib/logger";
import { SyncOutputBuffer } from "@/lib/terminal/SyncOutputBuffer";
import { TerminalInstanceManager } from "@/lib/terminal/TerminalInstanceManager";
import { ThemeManager } from "@/lib/theme";
import { useRenderMode, useTerminalClearRequest } from "@/store";
import { ptyResize, ptyWrite } from "../../lib/tauri";
import "@xterm/xterm/css/xterm.css";

interface TerminalProps {
  sessionId: string;
}

interface TerminalOutputEvent {
  session_id: string;
  data: string;
}

export function Terminal({ sessionId }: TerminalProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const terminalRef = useRef<XTerm | null>(null);
  const fitAddonRef = useRef<FitAddon | null>(null);
  const syncBufferRef = useRef<SyncOutputBuffer | null>(null);
  const cleanupFnsRef = useRef<(() => void)[]>([]);
  const clearRequest = useTerminalClearRequest(sessionId);
  const renderMode = useRenderMode(sessionId);
  const prevRenderModeRef = useRef(renderMode);
  // Track pending resize RAF to debounce rapid resize events during DOM restructuring
  const resizeRafRef = useRef<number | null>(null);
  // Track if this is a reattachment (terminal already existed in manager)
  const isReattachmentRef = useRef(false);

  // Handle resize
  // Note: We only call fit() here - ptyResize is handled by terminal.onResize
  // which is triggered by fit() internally. This prevents duplicate resize calls.
  const handleResize = useCallback(() => {
    if (fitAddonRef.current && terminalRef.current) {
      fitAddonRef.current.fit();
    }
  }, []);

  // Handle terminal clear requests (for when xterm Terminal is used)
  useEffect(() => {
    if (clearRequest > 0 && terminalRef.current) {
      terminalRef.current.clear();
    }
  }, [clearRequest]);

  // Handle fullterm mode transitions:
  // 1. Clear terminal when entering fullterm to prevent visual artifacts
  // 2. Auto-focus terminal so user can interact immediately with TUI apps
  // Using useLayoutEffect ensures the reset happens BEFORE the browser paints,
  // preventing the split-second flash of old content.
  // Skip clearing on reattachment since the terminal already has the correct content.
  useLayoutEffect(() => {
    const prevMode = prevRenderModeRef.current;
    prevRenderModeRef.current = renderMode;

    // Only act when transitioning TO fullterm (not on initial mount or when exiting)
    if (renderMode === "fullterm" && prevMode !== "fullterm" && terminalRef.current) {
      // Clear terminal if not a reattachment (avoids losing content)
      if (!isReattachmentRef.current) {
        // Use reset() for a full terminal reset - clears screen and resets all modes
        // This is cleaner than clear() which only clears scrollback
        terminalRef.current.reset();
        // Re-fit after reset to ensure correct dimensions
        if (fitAddonRef.current) {
          fitAddonRef.current.fit();
        }
      }
      // Auto-focus terminal so user can interact with TUI apps immediately
      terminalRef.current.focus();
    }
  }, [renderMode]);

  useEffect(() => {
    if (!containerRef.current) return;

    // Check if we already have a terminal instance for this session (reattachment case)
    const existingInstance = TerminalInstanceManager.get(sessionId);
    let terminal: XTerm;
    let fitAddon: FitAddon;
    let isNewInstance = false;

    if (existingInstance) {
      // Reattachment: Terminal exists in manager, just reattach to new container
      logger.debug("[Terminal] Reattaching existing terminal for session:", sessionId);
      terminal = existingInstance.terminal;
      fitAddon = existingInstance.fitAddon;
      isReattachmentRef.current = true;

      // Move terminal DOM to new container
      TerminalInstanceManager.attachToContainer(sessionId, containerRef.current);

      // Get or create sync buffer for this instance
      if (!syncBufferRef.current) {
        const syncBuffer = new SyncOutputBuffer();
        syncBuffer.attach(terminal);
        syncBufferRef.current = syncBuffer;
      }
    } else {
      // Fresh instance: Create new terminal
      logger.debug("[Terminal] Creating new terminal for session:", sessionId);
      isNewInstance = true;
      isReattachmentRef.current = false;

      terminal = new XTerm({
        cursorBlink: true,
        cursorStyle: "block",
        cursorInactiveStyle: "none",
        fontSize: 14,
        fontFamily: "JetBrains Mono, Menlo, Monaco, Consolas, monospace",
        allowProposedApi: true,
        scrollback: 10000,
        smoothScrollDuration: 0,
        ignoreBracketedPasteMode: false,
        windowOptions: {
          getWinSizeChars: true,
          getWinSizePixels: true,
          getCellSizePixels: true,
          getScreenSizeChars: true,
        },
      });

      // Add addons
      fitAddon = new FitAddon();
      terminal.loadAddon(fitAddon);
      terminal.loadAddon(
        new WebLinksAddon((_event, uri) => {
          open(uri).catch((err: unknown) => logger.error("[Terminal] Failed to open URL:", err));
        })
      );

      // Open terminal in container
      terminal.open(containerRef.current);
      logger.debug("[Terminal] Opened terminal for session:", sessionId);

      // Register with manager AFTER opening (so terminal.element exists)
      TerminalInstanceManager.register(sessionId, terminal, fitAddon);
      TerminalInstanceManager.attachToContainer(sessionId, containerRef.current);

      // Apply current theme
      ThemeManager.applyToTerminal(terminal);

      // Try to load WebGL addon for better performance
      try {
        const webglAddon = new WebglAddon();
        terminal.loadAddon(webglAddon);
      } catch (e) {
        logger.warn("WebGL not available, falling back to canvas", e);
      }

      // Initial fit
      fitAddon.fit();

      // Create and attach synchronized output buffer
      const syncBuffer = new SyncOutputBuffer();
      syncBuffer.attach(terminal);
      syncBufferRef.current = syncBuffer;
    }

    // Store refs for use in callbacks
    terminalRef.current = terminal;
    fitAddonRef.current = fitAddon;

    // Listen for theme changes (register on each mount)
    const unsubscribeTheme = ThemeManager.onChange(() => {
      if (terminalRef.current) {
        ThemeManager.applyToTerminal(terminalRef.current);
      }
    });
    cleanupFnsRef.current.push(unsubscribeTheme);

    // Abort flag to prevent race conditions
    let aborted = false;

    // Send resize to PTY (needed for both new and reattached terminals)
    // For reattached terminals, the container size may have changed
    const { rows, cols } = terminal;
    logger.debug("[Terminal] Sending resize:", {
      sessionId,
      rows,
      cols,
      isReattachment: !isNewInstance,
    });
    ptyResize(sessionId, rows, cols).catch(console.error);

    // Register input handler (captures sessionId via closure)
    const inputDisposable = terminal.onData((data) => {
      if (aborted) return;
      ptyWrite(sessionId, data).catch(console.error);
    });
    cleanupFnsRef.current.push(() => inputDisposable.dispose());

    // Handle xterm.js internal resize events
    const resizeDisposable = terminal.onResize(({ rows, cols }) => {
      if (aborted) return;
      ptyResize(sessionId, rows, cols).catch(console.error);
    });
    cleanupFnsRef.current.push(() => resizeDisposable.dispose());

    // Set up event listeners asynchronously
    (async () => {
      // Set up terminal output listener
      logger.debug("[Terminal] Setting up output listener for session:", sessionId);
      const unlistenOutput = await listen<TerminalOutputEvent>("terminal_output", (event) => {
        if (aborted) return;
        if (event.payload.session_id === sessionId && syncBufferRef.current) {
          syncBufferRef.current.write(event.payload.data);
        }
      });

      // Set up synchronized output listener (DEC 2026)
      const unlistenSync = await listen<{ session_id: string; enabled: boolean }>(
        "synchronized_output",
        (event) => {
          if (aborted) return;
          if (event.payload.session_id === sessionId && syncBufferRef.current) {
            syncBufferRef.current.setSyncEnabled(event.payload.enabled);
          }
        }
      );

      if (aborted) {
        unlistenSync();
        unlistenOutput();
        return;
      }
      cleanupFnsRef.current.push(unlistenSync);
      cleanupFnsRef.current.push(unlistenOutput);

      // Focus terminal after listeners are ready
      terminal.focus();

      // Set up focus event handlers (DEC 1004)
      const handleFocus = () => {
        if (aborted) return;
        if ((terminal.modes as { sendFocusMode?: boolean })?.sendFocusMode) {
          ptyWrite(sessionId, "\x1b[I").catch(console.error);
        }
      };

      const handleBlur = () => {
        if (aborted) return;
        if ((terminal.modes as { sendFocusMode?: boolean })?.sendFocusMode) {
          ptyWrite(sessionId, "\x1b[O").catch(console.error);
        }
      };

      terminal.textarea?.addEventListener("focus", handleFocus);
      terminal.textarea?.addEventListener("blur", handleBlur);

      cleanupFnsRef.current.push(() => {
        terminal.textarea?.removeEventListener("focus", handleFocus);
        terminal.textarea?.removeEventListener("blur", handleBlur);
      });
    })();

    // Store abort setter for cleanup
    const setAborted = () => {
      aborted = true;
    };

    // Handle window resize with debouncing
    let resizeTimeoutRef: ReturnType<typeof setTimeout> | null = null;
    const resizeObserver = new ResizeObserver(() => {
      if (resizeRafRef.current !== null) {
        cancelAnimationFrame(resizeRafRef.current);
      }
      if (resizeTimeoutRef !== null) {
        clearTimeout(resizeTimeoutRef);
      }
      resizeRafRef.current = requestAnimationFrame(() => {
        resizeRafRef.current = requestAnimationFrame(() => {
          resizeRafRef.current = null;
          resizeTimeoutRef = setTimeout(() => {
            resizeTimeoutRef = null;
            if (!aborted) {
              handleResize();
            }
          }, 50);
        });
      });
    });
    resizeObserver.observe(containerRef.current);

    return () => {
      // Signal abort to stop any pending async work
      setAborted();

      // Cancel any pending resize RAF and timeout
      if (resizeRafRef.current !== null) {
        cancelAnimationFrame(resizeRafRef.current);
        resizeRafRef.current = null;
      }
      if (resizeTimeoutRef !== null) {
        clearTimeout(resizeTimeoutRef);
        resizeTimeoutRef = null;
      }

      // Disconnect resize observer
      resizeObserver.disconnect();

      // Run cleanup functions (unsubscribe listeners, dispose handlers)
      for (const fn of cleanupFnsRef.current) {
        fn();
      }
      cleanupFnsRef.current = [];

      // Detach sync buffer but don't dispose terminal
      syncBufferRef.current?.detach();
      syncBufferRef.current = null;

      // CRITICAL: Do NOT dispose terminal - let manager handle lifecycle
      // Just detach from container so it can be reattached later
      TerminalInstanceManager.detach(sessionId);

      // Clear local refs (but terminal lives on in manager)
      terminalRef.current = null;
      fitAddonRef.current = null;
    };
  }, [sessionId, handleResize]);

  return <div ref={containerRef} className="w-full h-full min-h-0" style={{ lineHeight: 1 }} />;
}
