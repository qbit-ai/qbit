import { listen } from "@tauri-apps/api/event";
import { FitAddon } from "@xterm/addon-fit";
import { WebLinksAddon } from "@xterm/addon-web-links";
import { WebglAddon } from "@xterm/addon-webgl";
import { Terminal as XTerm } from "@xterm/xterm";
import { useCallback, useEffect, useRef } from "react";
import { ThemeManager } from "@/lib/theme";
import { ptyResize, ptyWrite } from "../../lib/tauri";
import "@xterm/xterm/css/xterm.css";

interface TerminalProps {
  sessionId: string;
}

interface TerminalOutputEvent {
  session_id: string;
  data: string;
}

interface CommandBlockEvent {
  session_id: string;
  event_type: "prompt_start" | "prompt_end" | "command_start" | "command_end";
}

export function Terminal({ sessionId }: TerminalProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const terminalRef = useRef<XTerm | null>(null);
  const fitAddonRef = useRef<FitAddon | null>(null);
  const cleanupFnsRef = useRef<(() => void)[]>([]);

  // Handle resize
  const handleResize = useCallback(() => {
    if (fitAddonRef.current && terminalRef.current) {
      fitAddonRef.current.fit();
      const { rows, cols } = terminalRef.current;
      ptyResize(sessionId, rows, cols).catch(console.error);
    }
  }, [sessionId]);

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

    // Set up all event listeners and user input handling
    // Use an async IIFE to await listener setup before enabling input
    (async () => {
      // Buffer to collect any output that arrives before terminal is fully ready
      const pendingData: string[] = [];
      let listenerReady = false;

      // Set up terminal output listener
      const unlistenOutput = await listen<TerminalOutputEvent>("terminal_output", (event) => {
        if (event.payload.session_id === sessionId) {
          if (listenerReady && terminalRef.current) {
            terminalRef.current.write(event.payload.data);
          } else {
            pendingData.push(event.payload.data);
          }
        }
      });
      cleanupFnsRef.current.push(unlistenOutput);

      // Set up command block listener
      const unlistenCommandBlock = await listen<CommandBlockEvent>("command_block", (event) => {
        if (event.payload.session_id === sessionId && terminalRef.current) {
          if (event.payload.event_type === "prompt_start") {
            terminalRef.current.clear();
          }
        }
      });
      cleanupFnsRef.current.push(unlistenCommandBlock);

      // Now that listeners are ready, flush any buffered data
      for (const data of pendingData) {
        terminal.write(data);
      }
      listenerReady = true;

      // NOW enable user input - only after listeners are attached
      terminal.onData((data) => {
        ptyWrite(sessionId, data).catch(console.error);
      });

      // Initial resize notification
      const { rows, cols } = terminal;
      ptyResize(sessionId, rows, cols).catch(console.error);

      // Focus terminal
      terminal.focus();
    })();

    // Handle window resize
    const resizeObserver = new ResizeObserver(() => {
      handleResize();
    });
    resizeObserver.observe(containerRef.current);

    return () => {
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
