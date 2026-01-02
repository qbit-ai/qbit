import { listen } from "@tauri-apps/api/event";
import { FitAddon } from "@xterm/addon-fit";
import { WebLinksAddon } from "@xterm/addon-web-links";
import { WebglAddon } from "@xterm/addon-webgl";
import { Terminal as XTerm } from "@xterm/xterm";
import { useCallback, useEffect, useLayoutEffect, useRef } from "react";
import { SyncOutputBuffer } from "@/lib/terminal/SyncOutputBuffer";
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

  // Handle resize
  const handleResize = useCallback(() => {
    if (fitAddonRef.current && terminalRef.current) {
      fitAddonRef.current.fit();
      const { rows, cols } = terminalRef.current;
      ptyResize(sessionId, rows, cols).catch(console.error);
    }
  }, [sessionId]);

  // Handle terminal clear requests (for when xterm Terminal is used)
  useEffect(() => {
    if (clearRequest > 0 && terminalRef.current) {
      terminalRef.current.clear();
    }
  }, [clearRequest]);

  // Clear terminal when entering fullterm mode to prevent visual artifacts
  // Since Terminal is always mounted, it accumulates output from timeline mode.
  // Clearing on fullterm entry provides a clean slate for the fullterm app.
  // Using useLayoutEffect ensures the reset happens BEFORE the browser paints,
  // preventing the split-second flash of old content.
  useLayoutEffect(() => {
    const prevMode = prevRenderModeRef.current;
    prevRenderModeRef.current = renderMode;

    // Only clear when transitioning TO fullterm (not on initial mount or when exiting)
    if (renderMode === "fullterm" && prevMode !== "fullterm" && terminalRef.current) {
      // Use reset() for a full terminal reset - clears screen and resets all modes
      // This is cleaner than clear() which only clears scrollback
      terminalRef.current.reset();
      // Re-fit after reset to ensure correct dimensions
      if (fitAddonRef.current) {
        fitAddonRef.current.fit();
      }
    }
  }, [renderMode]);

  useEffect(() => {
    if (!containerRef.current) return;

    // Prevent duplicate setup in StrictMode - if terminal already exists, just focus
    if (terminalRef.current) {
      terminalRef.current.focus();
      return;
    }

    // Clear any previous cleanup functions before setting up new ones
    for (const fn of cleanupFnsRef.current) {
      fn();
    }
    cleanupFnsRef.current = [];

    // Create terminal first but don't enable input yet
    const terminal = new XTerm({
      cursorBlink: true,
      cursorStyle: "block",
      cursorInactiveStyle: "none", // Hide cursor when terminal loses focus
      fontSize: 14,
      fontFamily: "JetBrains Mono, Menlo, Monaco, Consolas, monospace",
      // Theme will be applied by ThemeManager
      allowProposedApi: true,
      scrollback: 10000, // Adequate scrollback buffer
      smoothScrollDuration: 0, // Disable smooth scroll for responsiveness
      ignoreBracketedPasteMode: false, // Respect bracketed paste from apps
      // Enable window size reporting for apps that query terminal dimensions
      // This is required for Ink-based CLI apps (Claude Code, etc.) to render correctly
      windowOptions: {
        getWinSizeChars: true, // CSI 18 t - Report size in characters
        getWinSizePixels: true, // CSI 14 t - Report size in pixels
        getCellSizePixels: true, // CSI 16 t - Report cell size in pixels
        getScreenSizeChars: true, // CSI 9 t - Report screen size in chars
      },
    });

    // Add addons
    const fitAddon = new FitAddon();
    terminal.loadAddon(fitAddon);
    terminal.loadAddon(new WebLinksAddon());

    // Open terminal
    terminal.open(containerRef.current);
    console.log("[Terminal] Opened terminal for session:", sessionId);

    // Apply current theme
    ThemeManager.applyToTerminal(terminal);

    // Listen for theme changes
    const unsubscribeTheme = ThemeManager.onChange(() => {
      if (terminalRef.current) {
        ThemeManager.applyToTerminal(terminalRef.current);
      }
    });
    cleanupFnsRef.current.push(unsubscribeTheme);

    // Try to load WebGL addon for better performance
    try {
      const webglAddon = new WebglAddon();
      terminal.loadAddon(webglAddon);
    } catch (e) {
      console.warn("WebGL not available, falling back to canvas", e);
    }

    // Initial fit
    fitAddon.fit();

    // Create and attach synchronized output buffer
    const syncBuffer = new SyncOutputBuffer();
    syncBuffer.attach(terminal);
    syncBufferRef.current = syncBuffer;

    terminalRef.current = terminal;
    fitAddonRef.current = fitAddon;

    // Abort flag to prevent race conditions with React StrictMode
    // When cleanup runs, we set this to true so any pending async work stops
    let aborted = false;

    // CRITICAL: Send initial resize IMMEDIATELY (before async listener setup)
    // This ensures the PTY has correct terminal dimensions when fullterm apps
    // (like codex/cdx) start and query the terminal size.
    // Without this, apps may get stale/incorrect size on second launch.
    const { rows: initialRows, cols: initialCols } = terminal;
    console.log("[Terminal] Sending immediate resize:", {
      sessionId,
      rows: initialRows,
      cols: initialCols,
    });
    ptyResize(sessionId, initialRows, initialCols).catch(console.error);

    // CRITICAL: Register input handler IMMEDIATELY (before async listener setup)
    // This ensures user input can be captured even during the async setup phase.
    // Without this, input typed during listener setup would be lost.
    const inputDisposable = terminal.onData((data) => {
      if (aborted) return;
      ptyWrite(sessionId, data).catch(console.error);
    });
    cleanupFnsRef.current.push(() => inputDisposable.dispose());

    // Handle xterm.js internal resize events (e.g., from fit addon or font changes)
    // This ensures the PTY is always synced with the terminal's actual size
    const resizeDisposable = terminal.onResize(({ rows, cols }) => {
      if (aborted) return;
      ptyResize(sessionId, rows, cols).catch(console.error);
    });
    cleanupFnsRef.current.push(() => resizeDisposable.dispose());

    // Set up event listeners asynchronously
    // The critical input/resize handlers are already registered above
    (async () => {
      // Set up terminal output listener
      console.log("[Terminal] Setting up output listener for session:", sessionId);
      const unlistenOutput = await listen<TerminalOutputEvent>("terminal_output", (event) => {
        // Check abort flag - if we're unmounted, don't write to the (potentially new) terminal
        if (aborted) return;
        if (event.payload.session_id === sessionId && syncBufferRef.current) {
          // Use sync buffer to handle DEC 2026 synchronized output
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

      // Check if we were unmounted during the await
      if (aborted) {
        unlistenSync();
        unlistenOutput();
        return;
      }
      cleanupFnsRef.current.push(unlistenSync);
      cleanupFnsRef.current.push(unlistenOutput);

      // Note: We intentionally do NOT listen to command_block events here.
      // In fullterm mode, we want the terminal to show everything without clearing.
      // The prompt_start clearing behavior is for timeline mode only.

      // Focus terminal after listeners are ready
      terminal.focus();

      // Set up focus event handlers (DEC 1004)
      // When apps enable focus mode, we send focus in/out sequences
      const handleFocus = () => {
        if (aborted) return;
        // Check if focus event mode is enabled (DEC 1004)
        // xterm.js exposes this via terminal.modes.sendFocusMode
        if ((terminal.modes as { sendFocusMode?: boolean })?.sendFocusMode) {
          // Send focus in sequence: CSI I
          ptyWrite(sessionId, "\x1b[I").catch(console.error);
        }
      };

      const handleBlur = () => {
        if (aborted) return;
        if ((terminal.modes as { sendFocusMode?: boolean })?.sendFocusMode) {
          // Send focus out sequence: CSI O
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

    // Handle window resize
    const resizeObserver = new ResizeObserver(() => {
      handleResize();
    });
    resizeObserver.observe(containerRef.current);

    return () => {
      // Signal abort to stop any pending async work
      setAborted();
      resizeObserver.disconnect();
      for (const fn of cleanupFnsRef.current) {
        fn();
      }
      cleanupFnsRef.current = [];
      syncBufferRef.current?.detach();
      syncBufferRef.current = null;
      terminal.dispose();
      terminalRef.current = null;
      fitAddonRef.current = null;
    };
  }, [sessionId, handleResize]);

  return <div ref={containerRef} className="w-full h-full min-h-0" style={{ lineHeight: 1 }} />;
}
