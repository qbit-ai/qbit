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

  // Handle resize
  // Note: We only call fit() here - ptyResize is handled by terminal.onResize
  // which is triggered by fit() internally. This prevents duplicate resize calls.
  const handleResize = useCallback(() => {
    if (fitAddonRef.current && terminalRef.current) {
      try {
        fitAddonRef.current.fit();
      } catch (error) {
        // Renderer may not be ready yet (race condition during reattachment)
        // This is non-fatal - terminal will resize properly on next resize event
        logger.debug("[Terminal] fit() failed, renderer may not be ready:", error);
      }
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
  useLayoutEffect(() => {
    const prevMode = prevRenderModeRef.current;
    prevRenderModeRef.current = renderMode;

    // Only act when transitioning TO fullterm (not on initial mount or when exiting)
    if (renderMode === "fullterm" && prevMode !== "fullterm" && terminalRef.current) {
      // Use reset() for a full terminal reset - clears screen and resets all modes.
      // Always reset regardless of isReattachmentRef: if the terminal was reattached
      // while in fullterm mode, prevMode would already be "fullterm" and this branch
      // wouldn't execute. If it was reattached in timeline mode and is now transitioning
      // to fullterm, we must reset to get a clean rendering surface.
      terminalRef.current.reset();

      // Schedule fit() for after the container has computed dimensions.
      // When transitioning from hidden to visible, the container needs a
      // layout pass before dimensions are available. Double RAF ensures
      // we run after the browser has painted and computed layout.
      //
      // The double RAF (~33 ms) also outlasts the output coalescing window in the
      // backend emitter thread (16 ms), so any pre-TUI output bytes that arrived
      // after reset() have already been processed by xterm.js by this point.
      // For non-alternate-screen TUI apps (e.g. claude, codex) those bytes can
      // include \r\n that pushes the cursor off row 0; the explicit cursor-home
      // below restores it before fit() triggers a SIGWINCH redraw.
      let innerRafId: number | undefined;
      const outerRafId = requestAnimationFrame(() => {
        innerRafId = requestAnimationFrame(() => {
          // Home the cursor for normal-screen sessions. Alternate-screen TUI apps
          // (vim, htop, etc.) manage their own cursor via ESC[?1049h which clears
          // the alternate screen and homes the cursor — writing to the normal screen
          // here would be harmless but unnecessary. Non-alternate-screen apps
          // (claude, codex, etc.) need this to counteract pre-TUI \r\n bytes that
          // moved the cursor after the earlier reset().
          if (terminalRef.current) {
            const isAltScreen = terminalRef.current.buffer.active.type === "alternate";
            if (!isAltScreen) {
              terminalRef.current.write("\x1b[H");
            }
          }
          if (fitAddonRef.current) {
            try {
              fitAddonRef.current.fit();
            } catch (error) {
              logger.debug("[Terminal] fit() after fullterm transition failed:", error);
            }
          }
        });
      });

      // Auto-focus terminal so user can interact with TUI apps immediately
      terminalRef.current.focus();

      return () => {
        cancelAnimationFrame(outerRafId);
        if (innerRafId !== undefined) {
          cancelAnimationFrame(innerRafId);
        }
      };
    }
  }, [renderMode]);

  useEffect(() => {
    if (!containerRef.current) return;

    // Check if we already have a terminal instance for this session (reattachment case)
    const existingInstance = TerminalInstanceManager.get(sessionId);
    let terminal: XTerm;
    let fitAddon: FitAddon;

    if (existingInstance) {
      // Reattachment: Terminal exists in manager, just reattach to new container
      terminal = existingInstance.terminal;
      fitAddon = existingInstance.fitAddon;

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

      // Register with manager AFTER opening (so terminal.element exists)
      TerminalInstanceManager.register(sessionId, terminal, fitAddon);
      TerminalInstanceManager.attachToContainer(sessionId, containerRef.current);

      // Apply current theme
      ThemeManager.applyToTerminal(terminal);

      // Try to load WebGL addon for better performance
      // Includes context loss handling for recovery from GPU driver issues
      // First check if WebGL is available in the browser
      const testCanvas = document.createElement("canvas");
      const gl = testCanvas.getContext("webgl2") || testCanvas.getContext("webgl");

      if (gl) {
        try {
          const webglAddon = new WebglAddon();

          // Handle WebGL context loss (e.g., GPU driver crash, resource exhaustion)
          // When context is lost, dispose the addon and fall back to canvas renderer
          webglAddon.onContextLoss(() => {
            logger.warn("[Terminal] WebGL context lost for session:", sessionId);
            try {
              webglAddon.dispose();
            } catch (disposeError) {
              // Ignore disposal errors - addon may already be in bad state
              logger.debug("[Terminal] WebGL addon disposal error (expected):", disposeError);
            }
            // Terminal will automatically fall back to canvas renderer
          });

          terminal.loadAddon(webglAddon);
          logger.debug("[Terminal] WebGL renderer active for session:", sessionId);

          // Store cleanup function for proper disposal on unmount
          cleanupFnsRef.current.push(() => {
            try {
              webglAddon.dispose();
            } catch (disposeError) {
              // Ignore disposal errors during cleanup - addon may have already
              // been disposed due to context loss
              logger.debug("[Terminal] WebGL cleanup disposal error (expected):", disposeError);
            }
          });
        } catch (e) {
          logger.warn("[Terminal] WebGL addon failed, using canvas renderer:", e);
        }
      } else {
        logger.debug("[Terminal] WebGL not available, using canvas renderer");
      }

      // Initial fit (may fail if renderer not ready, will be retried on resize)
      try {
        fitAddon.fit();
      } catch (error) {
        logger.debug("[Terminal] Initial fit() failed:", error);
      }

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

    // Send resize to PTY only if container has valid dimensions.
    // In timeline mode, the container is hidden and has no dimensions,
    // so we skip the resize here - it will happen when switching to fullterm
    // or when the ResizeObserver fires after the container becomes visible.
    const containerEl = containerRef.current;
    if (containerEl && containerEl.clientWidth > 0 && containerEl.clientHeight > 0) {
      const { rows, cols } = terminal;
      ptyResize(sessionId, rows, cols).catch((err) => logger.error("PTY resize failed:", err));
    }

    // Register input handler (captures sessionId via closure)
    const inputDisposable = terminal.onData((data) => {
      if (aborted) return;
      ptyWrite(sessionId, data).catch((err) => logger.error("PTY write failed:", err));
    });
    cleanupFnsRef.current.push(() => inputDisposable.dispose());

    // Handle xterm.js internal resize events
    const resizeDisposable = terminal.onResize(({ rows, cols }) => {
      if (aborted) return;
      ptyResize(sessionId, rows, cols).catch((err) => logger.error("PTY resize failed:", err));
    });
    cleanupFnsRef.current.push(() => resizeDisposable.dispose());

    // Set up event listeners asynchronously
    (async () => {
      // Set up terminal output listener
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
          ptyWrite(sessionId, "\x1b[I").catch((err) =>
            logger.error("PTY write focus failed:", err)
          );
        }
      };

      const handleBlur = () => {
        if (aborted) return;
        if ((terminal.modes as { sendFocusMode?: boolean })?.sendFocusMode) {
          ptyWrite(sessionId, "\x1b[O").catch((err) => logger.error("PTY write blur failed:", err));
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

    // Handle window resize with RAF throttling
    // Simplified from double RAF + 50ms timeout to single RAF for better responsiveness
    const resizeObserver = new ResizeObserver(() => {
      // Skip if already pending
      if (resizeRafRef.current !== null) {
        return;
      }
      resizeRafRef.current = requestAnimationFrame(() => {
        resizeRafRef.current = null;
        if (!aborted) {
          handleResize();
        }
      });
    });
    resizeObserver.observe(containerRef.current);

    return () => {
      // Signal abort to stop any pending async work
      setAborted();

      // Cancel any pending resize RAF
      if (resizeRafRef.current !== null) {
        cancelAnimationFrame(resizeRafRef.current);
        resizeRafRef.current = null;
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
