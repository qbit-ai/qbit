import { useEffect, useRef } from "react";
import { Terminal } from "@xterm/xterm";
import { FitAddon } from "@xterm/addon-fit";
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
  const fitAddonRef = useRef<FitAddon | null>(null);

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
        scrollback: 0, // No scrollback for static output
        convertEol: true,
        allowProposedApi: true,
      });

      const fitAddon = new FitAddon();
      terminal.loadAddon(fitAddon);

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
      fitAddon.fit();

      terminalRef.current = terminal;
      fitAddonRef.current = fitAddon;
    }

    // Write output
    const terminal = terminalRef.current;
    terminal.clear();
    terminal.write(output);

    // Fit to content
    if (fitAddonRef.current) {
      fitAddonRef.current.fit();
    }

    return () => {
      // Cleanup on unmount
      if (terminalRef.current) {
        terminalRef.current.dispose();
        terminalRef.current = null;
        fitAddonRef.current = null;
      }
    };
  }, [output]);

  // Calculate approximate height based on line count
  const lineCount = output.split("\n").length;
  const lineHeight = 12 * 1.4; // fontSize * lineHeight
  const height = Math.min(Math.max(lineCount * lineHeight, 20), 400); // min 20px, max 400px

  return (
    <div
      ref={containerRef}
      style={{ height: `${height}px` }}
      className="overflow-hidden [&_.xterm-viewport]:!overflow-hidden"
    />
  );
}
