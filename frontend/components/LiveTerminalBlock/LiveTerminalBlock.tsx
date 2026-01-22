import { useEffect, useRef } from "react";
import { liveTerminalManager } from "@/lib/terminal";
import "@xterm/xterm/css/xterm.css";
import "@/styles/xterm-overrides.css";

interface LiveTerminalBlockProps {
  sessionId: string;
  /** The command being executed (captured from OSC 133;C) */
  command: string | null;
}

export function LiveTerminalBlock({ sessionId, command }: LiveTerminalBlockProps) {
  const containerRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!containerRef.current) {
      return;
    }

    // Get or create the terminal for this session
    liveTerminalManager.getOrCreate(sessionId);

    // Attach to this container
    liveTerminalManager.attachToContainer(sessionId, containerRef.current);

    // Cleanup: detach but don't dispose (might be reattaching)
    // Disposal happens in useTauriEvents when command completes
  }, [sessionId]);

  return (
    <div className="w-full">
      {/* Command header - matches CommandBlock style */}
      {command && (
        <div className="flex items-center gap-2 px-5 py-3 w-full">
          <code
            className="flex-1 truncate text-[var(--color-ansi-white)]"
            style={{
              fontSize: "12px",
              lineHeight: 1.4,
              fontFamily: "JetBrains Mono, Menlo, Monaco, Consolas, monospace",
            }}
          >
            <span className="text-[var(--color-ansi-green)]">$ </span>
            {command}
          </code>
          {/* Pulsing indicator to show command is running */}
          <span className="w-2 h-2 bg-[#7aa2f7] rounded-full animate-pulse flex-shrink-0" />
        </div>
      )}

      {/* Terminal container - shows only command output */}
      <div className="px-5 pb-4">
        <div
          ref={containerRef}
          className="h-96 overflow-hidden [&_.xterm-viewport]:!overflow-y-auto"
        />
      </div>
    </div>
  );
}
