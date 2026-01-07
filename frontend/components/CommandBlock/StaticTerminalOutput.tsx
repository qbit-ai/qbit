import { useEffect, useRef } from "react";
import { Terminal } from "@xterm/xterm";
import { ThemeManager } from "@/lib/theme";
import "@xterm/xterm/css/xterm.css";

interface StaticTerminalOutputProps {
  /** ANSI-formatted output to display */
  output: string;
}

/**
 * Renders terminal output using xterm.js in read-only mode.
 * This ensures visual consistency with LiveTerminalBlock.
 */
export function StaticTerminalOutput({ output }: StaticTerminalOutputProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const terminalRef = useRef<Terminal | null>(null);

  // Calculate rows needed for content (pre-render estimate)
  const lineCount = output.split("\n").length;
  const rows = Math.max(lineCount, 1);

  useEffect(() => {
    if (!containerRef.current || !output) return;

    // Create terminal if it doesn't exist
    if (!terminalRef.current) {
      const terminal = new Terminal({
        cursorBlink: false,
        cursorInactiveStyle: "none",
        disableStdin: true,
        fontSize: 12,
        fontFamily: "JetBrains Mono, Menlo, Monaco, Consolas, monospace",
        fontWeight: "normal",
        fontWeightBold: "bold",
        lineHeight: 1.4,
        scrollback: 0, // No scrollback - we set rows to fit all content
        convertEol: true,
        allowProposedApi: true,
        rows, // Set rows to match content
        cols: 200, // Wide enough to avoid wrapping most content
      });

      // Apply theme colors
      ThemeManager.applyToTerminal(terminal);

      // Override with our specific settings
      terminal.options.fontSize = 12;
      terminal.options.lineHeight = 1.4;
      terminal.options.fontWeight = "normal";
      terminal.options.letterSpacing = 0;
      terminal.options.theme = {
        ...terminal.options.theme,
        background: "rgba(0,0,0,0)",
      };

      terminal.open(containerRef.current);
      terminalRef.current = terminal;
    }

    // Update rows if content changed
    const terminal = terminalRef.current;
    if (terminal.rows !== rows) {
      terminal.resize(terminal.cols, rows);
    }

    // Write output
    terminal.clear();
    terminal.write(output);

    return () => {
      // Cleanup on unmount
      if (terminalRef.current) {
        terminalRef.current.dispose();
        terminalRef.current = null;
      }
    };
  }, [output, rows]);

  return (
    <div
      ref={containerRef}
      className="overflow-hidden [&_.xterm-viewport]:!overflow-hidden [&_.xterm-screen]:!h-auto"
    />
  );
}
