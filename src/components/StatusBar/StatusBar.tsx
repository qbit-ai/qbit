import { Bot, ChevronDown, Cpu, Terminal } from "lucide-react";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { initVertexAiAgent, VERTEX_AI_MODELS } from "@/lib/ai";
import { cn } from "@/lib/utils";
import { isMockBrowserMode } from "@/mocks";
import { useAiConfig, useInputMode, useStore } from "../../store";

// Available models for the dropdown
const AVAILABLE_MODELS = [
  { id: VERTEX_AI_MODELS.CLAUDE_OPUS_4_5, name: "Claude Opus 4.5" },
  { id: VERTEX_AI_MODELS.CLAUDE_SONNET_4_5, name: "Claude Sonnet 4.5" },
  { id: VERTEX_AI_MODELS.CLAUDE_HAIKU_4_5, name: "Claude Haiku 4.5" },
];

function formatModel(model: string): string {
  // Simplify Vertex AI model names
  if (!model) return "No Model";
  if (model.includes("claude-opus-4")) return "Claude Opus 4.5";
  if (model.includes("claude-sonnet-4-5")) return "Claude Sonnet 4.5";
  if (model.includes("claude-haiku-4-5")) return "Claude Haiku 4.5";
  return model;
}

interface StatusBarProps {
  sessionId: string | null;
}

export function StatusBar({ sessionId }: StatusBarProps) {
  const aiConfig = useAiConfig();
  const { model, status, errorMessage } = aiConfig;
  const inputMode = useInputMode(sessionId ?? "");
  const setInputMode = useStore((state) => state.setInputMode);
  const setAiConfig = useStore((state) => state.setAiConfig);

  const handleModelSelect = async (modelId: string) => {
    // Don't switch if already on this model or no vertex config
    if (model === modelId || !aiConfig.vertexConfig) {
      return;
    }

    const { vertexConfig } = aiConfig;
    const modelName = AVAILABLE_MODELS.find((m) => m.id === modelId)?.name ?? modelId;

    try {
      setAiConfig({ status: "initializing", model: modelId });

      await initVertexAiAgent({
        workspace: vertexConfig.workspace,
        credentialsPath: vertexConfig.credentialsPath,
        projectId: vertexConfig.projectId,
        location: vertexConfig.location,
        model: modelId,
      });

      setAiConfig({ status: "ready" });
      toast.success(`Switched to ${modelName}`, {
        style: {
          background: "var(--card)",
          color: "var(--ansi-magenta)",
          border: "1px solid var(--border)",
        },
      });
    } catch (error) {
      console.error("Failed to switch model:", error);
      setAiConfig({
        status: "error",
        errorMessage: error instanceof Error ? error.message : "Failed to switch model",
      });
      toast.error(`Failed to switch to ${modelName}`);
    }
  };

  return (
    <div className="h-9 bg-muted/20 backdrop-blur-sm border-t border-border/50 flex items-center justify-between px-3 text-xs text-muted-foreground relative z-10">
      {/* Left side */}
      <div className="flex items-center gap-3">
        {/* Mode segmented control - icons only */}
        <div className="flex items-center h-6 rounded-md bg-card p-1 border border-border">
          <button
            type="button"
            onClick={() => sessionId && setInputMode(sessionId, "terminal")}
            disabled={!sessionId}
            className={cn(
              "h-5 w-7 flex items-center justify-center rounded transition-colors",
              inputMode === "terminal"
                ? "bg-[var(--ansi-blue)] text-background"
                : "text-muted-foreground hover:text-[var(--ansi-blue)]"
            )}
          >
            <Terminal className="w-4 h-4" />
          </button>
          <button
            type="button"
            onClick={() => sessionId && setInputMode(sessionId, "agent")}
            disabled={!sessionId}
            className={cn(
              "h-5 w-7 flex items-center justify-center rounded transition-colors",
              inputMode === "agent"
                ? "bg-[var(--ansi-magenta)] text-background"
                : "text-muted-foreground hover:text-[var(--ansi-magenta)]"
            )}
          >
            <Bot className="w-4 h-4" />
          </button>
        </div>

        {/* Model selector badge or Terminal Mode indicator */}
        {inputMode === "terminal" ? (
          <div className="h-6 px-2.5 gap-1.5 text-xs font-normal rounded-md bg-[var(--ansi-blue)]/10 text-[var(--ansi-blue)] flex items-center">
            <Terminal className="w-4 h-4" />
            <span>Terminal Mode</span>
          </div>
        ) : status === "disconnected" ? (
          <div className="h-6 px-2.5 gap-1.5 text-xs font-normal rounded-md bg-muted-foreground/10 text-muted-foreground flex items-center">
            <Cpu className="w-4 h-4" />
            <span>AI Disconnected</span>
          </div>
        ) : status === "error" ? (
          <div className="h-6 px-2.5 gap-1.5 text-xs font-normal rounded-md bg-[var(--ansi-red)]/10 text-[var(--ansi-red)] flex items-center">
            <Cpu className="w-4 h-4" />
            <span>AI Error</span>
          </div>
        ) : status === "initializing" ? (
          <div className="h-6 px-2.5 gap-1.5 text-xs font-normal rounded-md bg-[var(--ansi-yellow)]/10 text-[var(--ansi-yellow)] flex items-center">
            <Cpu className="w-4 h-4 animate-pulse" />
            <span>Initializing...</span>
          </div>
        ) : (
          <DropdownMenu>
            <DropdownMenuTrigger asChild>
              <Button
                variant="ghost"
                size="sm"
                className="h-6 px-2.5 gap-1.5 text-xs font-normal rounded-md bg-[var(--ansi-magenta)]/10 text-[var(--ansi-magenta)] hover:bg-[var(--ansi-magenta)]/20 hover:text-[var(--ansi-magenta)]"
              >
                <Cpu className="w-4 h-4" />
                <span>{formatModel(model)}</span>
                <ChevronDown className="w-4 h-4" />
              </Button>
            </DropdownMenuTrigger>
            <DropdownMenuContent align="start" className="bg-card border-border min-w-[180px]">
              {AVAILABLE_MODELS.map((m) => (
                <DropdownMenuItem
                  key={m.id}
                  onClick={() => handleModelSelect(m.id)}
                  className={cn(
                    "text-xs cursor-pointer",
                    model === m.id
                      ? "text-[var(--ansi-magenta)] bg-[var(--ansi-magenta)]/10"
                      : "text-foreground hover:text-[var(--ansi-magenta)]"
                  )}
                >
                  {m.name}
                </DropdownMenuItem>
              ))}
            </DropdownMenuContent>
          </DropdownMenu>
        )}
      </div>

      {/* Right side - Status messages */}
      <div className="flex items-center gap-2">
        {isMockBrowserMode() ? (
          <span className="text-[var(--ansi-yellow)] truncate max-w-[200px]">
            Browser only mode enabled
          </span>
        ) : (
          status === "error" &&
          errorMessage && (
            <span className="text-[var(--ansi-red)] truncate max-w-[200px]">({errorMessage})</span>
          )
        )}
      </div>
    </div>
  );
}
