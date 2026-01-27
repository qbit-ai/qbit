import type { ILink, Terminal as TerminalType } from "@xterm/xterm";
import { Terminal } from "@xterm/xterm";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { FilePathPopup } from "@/components/FilePathPopup";
import { useFileEditorSidebar } from "@/hooks/useFileEditorSidebar";
import { type DetectedPath, detectFilePaths } from "@/lib/pathDetection";
import { type ResolvedPath, resolvePath } from "@/lib/pathResolution";
import { ThemeManager } from "@/lib/theme";
import "@xterm/xterm/css/xterm.css";

// ANSI escape codes for styling detected file paths
// Using cyan (36) for accent color and underline (4)
const LINK_START = "\x1b[4;36m"; // underline + cyan
const LINK_END = "\x1b[24;39m"; // no underline + default color

/**
 * Highlights detected file paths in terminal output with ANSI styling.
 * Processes each line to find file paths and wraps them with color/underline codes.
 */
function highlightFilePaths(text: string): string {
  const lines = text.split("\n");
  const highlightedLines = lines.map((line) => {
    const detected = detectFilePaths(line);
    if (detected.length === 0) return line;

    // Build the line with highlighted paths
    // Process in reverse order to preserve indices
    let result = line;
    for (let i = detected.length - 1; i >= 0; i--) {
      const path = detected[i];
      result =
        result.slice(0, path.start) +
        LINK_START +
        result.slice(path.start, path.end) +
        LINK_END +
        result.slice(path.end);
    }
    return result;
  });
  return highlightedLines.join("\n");
}

interface StaticTerminalOutputProps {
  /** ANSI-formatted output to display */
  output: string;
  /** Session ID for file editor */
  sessionId?: string;
  /** Working directory for path resolution */
  workingDirectory?: string;
}

/**
 * Renders terminal output using xterm.js in read-only mode.
 * This ensures visual consistency with LiveTerminalBlock.
 */
export function StaticTerminalOutput({
  output,
  sessionId,
  workingDirectory,
}: StaticTerminalOutputProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const terminalRef = useRef<TerminalType | null>(null);

  const [popupOpen, setPopupOpen] = useState(false);
  const [popupPosition, setPopupPosition] = useState<{ x: number; y: number } | null>(null);
  const [popupPaths, setPopupPaths] = useState<ResolvedPath[]>([]);
  const [popupLoading, setPopupLoading] = useState(false);
  // Store the detected path for resolution
  const pendingDetectedRef = useRef<DetectedPath | null>(null);

  const { openFile } = useFileEditorSidebar(workingDirectory);

  // Calculate rows needed for content (pre-render estimate)
  const lineCount = output.split("\n").length;
  const rows = Math.max(lineCount, 1);

  // Effect to create terminal (runs once on mount)
  // biome-ignore lint/correctness/useExhaustiveDependencies: rows is only needed for initial creation; resizing is handled in separate effect
  useEffect(() => {
    if (!containerRef.current) return;

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

    return () => {
      // Cleanup on unmount
      if (terminalRef.current) {
        terminalRef.current.dispose();
        terminalRef.current = null;
      }
    };
  }, []);

  // Effect to register link provider when sessionId/workingDirectory available
  useEffect(() => {
    if (!sessionId || !workingDirectory || !terminalRef.current) return;

    const terminal = terminalRef.current;
    const wdRef = workingDirectory; // Capture for closure

    const disposable = terminal.registerLinkProvider({
      provideLinks: (bufferLineNumber, callback) => {
        const buffer = terminal.buffer.active;
        const line = buffer.getLine(bufferLineNumber - 1);
        if (!line) {
          callback(undefined);
          return;
        }

        const lineText = line.translateToString(true);
        const detected = detectFilePaths(lineText);

        if (detected.length === 0) {
          callback(undefined);
          return;
        }

        const links: ILink[] = detected.map((pathInfo) => ({
          range: {
            start: { x: pathInfo.start + 1, y: bufferLineNumber },
            end: { x: pathInfo.end, y: bufferLineNumber },
          },
          text: pathInfo.raw,
          activate: async (event: MouseEvent) => {
            // Store the detected path for resolution
            pendingDetectedRef.current = pathInfo;

            setPopupLoading(true);
            setPopupPosition({ x: event.clientX, y: event.clientY });
            setPopupOpen(true);

            try {
              const resolved = await resolvePath(pathInfo, wdRef);
              setPopupPaths(resolved);
            } catch (error) {
              console.error("Failed to resolve path:", error);
              setPopupPaths([]);
            } finally {
              setPopupLoading(false);
            }
          },
        }));

        callback(links);
      },
    });

    return () => {
      disposable.dispose();
    };
  }, [sessionId, workingDirectory]);

  // Pre-process output to highlight file paths when links are enabled
  const processedOutput = useMemo(() => {
    if (!sessionId || !workingDirectory || !output) return output;
    return highlightFilePaths(output);
  }, [output, sessionId, workingDirectory]);

  // Effect to write content
  useEffect(() => {
    const terminal = terminalRef.current;
    if (!terminal || !processedOutput) return;

    // Update rows if content changed
    if (terminal.rows !== rows) {
      terminal.resize(terminal.cols, rows);
    }

    // Write output
    terminal.clear();
    terminal.write(processedOutput);
  }, [processedOutput, rows]);

  const handleOpenFile = useCallback(
    (absolutePath: string, _line?: number, _column?: number) => {
      // TODO: Support line navigation when CodeMirror supports it
      openFile(absolutePath);
      setPopupOpen(false);
    },
    [openFile]
  );

  return (
    <>
      <div
        ref={containerRef}
        className="overflow-hidden [&_.xterm-viewport]:!overflow-hidden [&_.xterm-screen]:!h-auto"
      />
      {popupPosition && (
        <FilePathPopup
          open={popupOpen}
          onOpenChange={setPopupOpen}
          paths={popupPaths}
          loading={popupLoading}
          onOpenFile={handleOpenFile}
          position={popupPosition}
        />
      )}
    </>
  );
}
