import { ArrowDown, ArrowUp, Folder, GitBranch, Package } from "lucide-react";
import { cn } from "@/lib/utils";
import { useStore } from "@/store";
import { useUnifiedInputState } from "@/store/selectors/unified-input";
import { selectDisplaySettings } from "@/store/slices";

interface ContextBarProps {
  sessionId: string;
  isAgentBusy: boolean;
}

export function ContextBar({ sessionId, isAgentBusy }: ContextBarProps) {
  const workingDirectory = useStore((state) => state.sessions[sessionId]?.workingDirectory);
  const openGitPanel = useStore((state) => state.openGitPanel);
  const { virtualEnv, gitBranch, gitStatus } = useUnifiedInputState(sessionId);
  const display = useStore(selectDisplaySettings);

  // Abbreviate path like fish shell: ~/C/p/my-project
  const displayPath = (() => {
    if (!workingDirectory) return "~";
    const withTilde = workingDirectory.replace(/^\/Users\/[^/]+/, "~");
    const parts = withTilde.split("/");
    if (parts.length <= 2) return withTilde;
    const first = parts[0];
    const last = parts[parts.length - 1];
    const middle = parts.slice(1, -1).map((p) => p[0] || p);
    return [first, ...middle, last].join("/");
  })();

  const parentOn = display.showTerminalContext;
  const pathVisible = parentOn && display.showWorkingDirectory;
  const gitVisible = parentOn && display.showGitBranch;
  const rowVisible = pathVisible || gitVisible;

  return (
    <div
      style={{
        maxHeight: rowVisible ? "40px" : "0px",
        opacity: rowVisible ? 1 : 0,
        overflow: "hidden",
        pointerEvents: rowVisible ? undefined : "none",
        transition: "max-height 300ms ease-in-out, opacity 250ms ease-in-out",
        willChange: "max-height, opacity",
      }}
    >
      <div
        className={cn(
          "flex items-center gap-2 px-4 py-1.5",
          isAgentBusy && "agent-loading-shimmer"
        )}
      >
        {/* Path badge */}
        <div
          style={{
            maxWidth: pathVisible ? "400px" : "0px",
            opacity: pathVisible ? 1 : 0,
            overflow: "hidden",
            pointerEvents: pathVisible ? undefined : "none",
            transition: "max-width 300ms ease-in-out, opacity 250ms ease-in-out",
            willChange: "max-width, opacity",
            flexShrink: 0,
          }}
        >
          <div
            className="h-5 px-1.5 gap-1 text-xs rounded bg-muted/50 border border-border/50 inline-flex items-center"
            title={workingDirectory || "~"}
          >
            <Folder className="size-icon-context-bar text-[#e0af68] shrink-0" />
            <span className="text-muted-foreground">{displayPath}</span>
          </div>
        </div>

        {/* Git badge */}
        <div
          style={{
            maxWidth: gitVisible && gitBranch ? "300px" : "0px",
            opacity: gitVisible && gitBranch ? 1 : 0,
            overflow: "hidden",
            pointerEvents: gitVisible && gitBranch ? undefined : "none",
            transition: "max-width 300ms ease-in-out, opacity 250ms ease-in-out",
            willChange: "max-width, opacity",
            flexShrink: 0,
          }}
        >
          <button
            type="button"
            onClick={openGitPanel}
            className="h-5 px-1.5 gap-1 text-xs rounded flex items-center border transition-colors shrink-0 bg-muted/50 hover:bg-muted border-border/50 cursor-pointer"
            title="Toggle Git Panel"
          >
            <GitBranch className="size-icon-context-bar text-[#7dcfff]" />
            <span className="text-muted-foreground">{gitBranch}</span>
            {gitStatus && (
              <>
                <span className="text-muted-foreground ml-0.5">|</span>
                <span className="text-[#9ece6a]">+{gitStatus.insertions ?? 0}</span>
                <span className="text-muted-foreground">/</span>
                <span className="text-[#f7768e]">-{gitStatus.deletions ?? 0}</span>
                {((gitStatus.ahead ?? 0) > 0 || (gitStatus.behind ?? 0) > 0) && (
                  <>
                    <span className="text-muted-foreground ml-0.5">|</span>
                    {(gitStatus.ahead ?? 0) > 0 && (
                      <span
                        className="flex items-center text-[#9ece6a]"
                        title={`${gitStatus.ahead} to push`}
                      >
                        <ArrowUp className="w-2.5 h-2.5" />
                        {gitStatus.ahead}
                      </span>
                    )}
                    {(gitStatus.behind ?? 0) > 0 && (
                      <span
                        className="flex items-center text-[#e0af68]"
                        title={`${gitStatus.behind} to pull`}
                      >
                        <ArrowDown className="w-2.5 h-2.5" />
                        {gitStatus.behind}
                      </span>
                    )}
                  </>
                )}
              </>
            )}
          </button>
        </div>

        {/* Virtual env badge — always visible, not gated by display settings */}
        {virtualEnv && (
          <div className="h-5 px-1.5 gap-1 text-xs rounded bg-[#9ece6a]/10 text-[#9ece6a] flex items-center border border-[#9ece6a]/20 shrink-0">
            <Package className="size-icon-context-bar" />
            <span>{virtualEnv}</span>
          </div>
        )}
      </div>
    </div>
  );
}
