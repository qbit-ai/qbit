import { listen } from "@tauri-apps/api/event";
import { FitAddon } from "@xterm/addon-fit";
import { WebLinksAddon } from "@xterm/addon-web-links";
import { WebglAddon } from "@xterm/addon-webgl";
import { Terminal as XTerm } from "@xterm/xterm";
import { useCallback, useEffect, useRef } from "react";
import { ThemeManager } from "@/lib/theme";
import { useTerminalClearRequest } from "@/store";
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
  const cleanupFnsRef = useRef<(() => void)[]>([]);
  const clearRequest = useTerminalClearRequest(sessionId);

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
      fontSize: 14,
      fontFamily: "JetBrains Mono, Menlo, Monaco, Consolas, monospace",
      // Theme will be applied by ThemeManager
      allowProposedApi: true,
    });

    // Add addons
    const fitAddon = new FitAddon();
    terminal.loadAddon(fitAddon);
    terminal.loadAddon(new WebLinksAddon());

    // Open terminal
    terminal.open(containerRef.current);

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

    terminalRef.current = terminal;
    fitAddonRef.current = fitAddon;

    // Abort flag to prevent race conditions with React StrictMode
    // When cleanup runs, we set this to true so any pending async work stops
    let aborted = false;

    // Set up all event listeners and user input handling
    // Use an async IIFE to await listener setup before enabling input
    (async () => {
      // Set up terminal output listener
      const unlistenOutput = await listen<TerminalOutputEvent>("terminal_output", (event) => {
        // Check abort flag - if we're unmounted, don't write to the (potentially new) terminal
        if (aborted) return;
        if (event.payload.session_id === sessionId && terminalRef.current) {
          terminalRef.current.write(event.payload.data);
        }
      });

      // Check if we were unmounted during the await
      if (aborted) {
        unlistenOutput();
        return;
      }
      cleanupFnsRef.current.push(unlistenOutput);

      // Note: We intentionally do NOT listen to command_block events here.
      // In fullterm mode, we want the terminal to show everything without clearing.
      // The prompt_start clearing behavior is for timeline mode only.

      // NOW enable user input - only after listeners are attached
      terminal.onData((data) => {
        if (aborted) return;
        ptyWrite(sessionId, data).catch(console.error);
      });

      // Send resize to PTY - this triggers SIGWINCH which causes any running
      // TUI apps to redraw their entire UI
      const { rows, cols } = terminal;
      await ptyResize(sessionId, rows, cols);

      // Check abort again after the await
      if (aborted) return;

      // Focus terminal
      terminal.focus();
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
      terminal.dispose();
      terminalRef.current = null;
      fitAddonRef.current = null;
    };
  }, [sessionId, handleResize]);

  return <div ref={containerRef} className="w-full h-full min-h-0" />;
}
