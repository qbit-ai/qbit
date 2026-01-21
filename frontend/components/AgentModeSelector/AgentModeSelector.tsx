import { Eye, Shield, Zap } from "lucide-react";
import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { setAgentMode as setAgentModeBackend } from "@/lib/ai";
import { logger } from "@/lib/logger";
import { notify } from "@/lib/notify";
import { cn } from "@/lib/utils";
import { type AgentMode, useAgentMode, useStore } from "@/store";

interface AgentModeSelectorProps {
  sessionId: string;
  showLabel?: boolean;
}

const AGENT_MODES: {
  id: AgentMode;
  name: string;
  description: string;
  icon: React.ComponentType<{ className?: string }>;
}[] = [
  {
    id: "default",
    name: "Default",
    description: "Tool approval based on policy",
    icon: Shield,
  },
  {
    id: "auto-approve",
    name: "Auto-approve",
    description: "All tools automatically approved",
    icon: Zap,
  },
  {
    id: "planning",
    name: "Planning",
    description: "Read-only tools only",
    icon: Eye,
  },
];

export function AgentModeSelector({ sessionId, showLabel = true }: AgentModeSelectorProps) {
  const agentMode = useAgentMode(sessionId);
  const setAgentMode = useStore((state) => state.setAgentMode);
  const workspace = useStore((state) => state.sessions[sessionId]?.workingDirectory);

  const currentMode = AGENT_MODES.find((m) => m.id === agentMode) ?? AGENT_MODES[0];
  const CurrentIcon = currentMode.icon;

  const handleModeSelect = async (mode: AgentMode) => {
    if (mode === agentMode) return;

    try {
      // Update frontend state
      setAgentMode(sessionId, mode);

      // Notify backend (pass workspace to persist to project settings)
      await setAgentModeBackend(sessionId, mode, workspace);

      const modeName = AGENT_MODES.find((m) => m.id === mode)?.name ?? mode;
      notify.success(`Agent mode: ${modeName}`);
    } catch (error) {
      logger.error("Failed to set agent mode:", error);
      notify.error(`Failed to set agent mode: ${error}`);
      // Revert on error
      setAgentMode(sessionId, agentMode);
    }
  };

  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button
          variant="ghost"
          size="sm"
          className={cn(
            "h-6 px-2 text-xs font-medium rounded-lg transition-all duration-200 flex items-center",
            showLabel ? "gap-1.5" : "gap-0",
            "bg-muted/60 text-muted-foreground hover:bg-muted hover:text-foreground border border-transparent",
            agentMode === "auto-approve" &&
              "bg-[var(--ansi-yellow)]/10 text-[var(--ansi-yellow)] hover:bg-[var(--ansi-yellow)]/20 border-[var(--ansi-yellow)]/20 hover:border-[var(--ansi-yellow)]/30",
            agentMode === "planning" &&
              "bg-[var(--ansi-blue)]/10 text-[var(--ansi-blue)] hover:bg-[var(--ansi-blue)]/20 border-[var(--ansi-blue)]/20 hover:border-[var(--ansi-blue)]/30"
          )}
        >
          <CurrentIcon className="w-3.5 h-3.5" />
          <span
            className={cn(
              "transition-all duration-200 overflow-hidden whitespace-nowrap",
              showLabel ? "max-w-[100px] opacity-100" : "max-w-0 opacity-0"
            )}
          >
            {currentMode.name}
          </span>
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent
        align="start"
        className="bg-card border-[var(--border-medium)] min-w-[200px]"
      >
        {AGENT_MODES.map((mode) => {
          const Icon = mode.icon;
          return (
            <DropdownMenuItem
              key={mode.id}
              onClick={() => handleModeSelect(mode.id)}
              className={cn(
                "text-xs cursor-pointer flex items-start gap-2 py-2",
                agentMode === mode.id
                  ? "text-accent bg-[var(--accent-dim)]"
                  : "text-foreground hover:text-accent"
              )}
            >
              <Icon className="w-4 h-4 mt-0.5 shrink-0" />
              <div className="flex flex-col">
                <span className="font-medium">{mode.name}</span>
                <span className="text-[10px] text-muted-foreground">{mode.description}</span>
              </div>
            </DropdownMenuItem>
          );
        })}
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
