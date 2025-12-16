import { Bot, ChevronDown, Cpu, Terminal } from "lucide-react";
import { useCallback, useEffect, useState } from "react";
import { NotificationWidget } from "@/components/NotificationWidget";
import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { getOpenRouterApiKey, initAiAgent, initVertexAiAgent, VERTEX_AI_MODELS } from "@/lib/ai";
import { notify } from "@/lib/notify";
import { getSettings } from "@/lib/settings";
import { cn } from "@/lib/utils";
import { isMockBrowserMode } from "@/mocks";
import { useAiConfig, useInputMode, useStore } from "../../store";

// Available Vertex AI models
const VERTEX_MODELS = [
  { id: VERTEX_AI_MODELS.CLAUDE_OPUS_4_5, name: "Claude Opus 4.5", provider: "vertex" as const },
  {
    id: VERTEX_AI_MODELS.CLAUDE_SONNET_4_5,
    name: "Claude Sonnet 4.5",
    provider: "vertex" as const,
  },
  { id: VERTEX_AI_MODELS.CLAUDE_HAIKU_4_5, name: "Claude Haiku 4.5", provider: "vertex" as const },
];

// Available OpenRouter models (fixed list per spec)
const OPENROUTER_MODELS = [
  { id: "mistralai/devstral-2512", name: "Devstral 2512", provider: "openrouter" as const },
  { id: "deepseek/deepseek-v3.2", name: "Deepseek v3.2", provider: "openrouter" as const },
  { id: "z-ai/glm-4.6", name: "GLM 4.6", provider: "openrouter" as const },
  { id: "x-ai/grok-code-fast-1", name: "Grok Code Fast 1", provider: "openrouter" as const },
  { id: "openai/gpt-oss-20b", name: "GPT OSS 20b", provider: "openrouter" as const },
  { id: "openai/gpt-oss-120b", name: "GPT OSS 120b", provider: "openrouter" as const },
  { id: "openai/gpt-5.2", name: "GPT 5.2", provider: "openrouter" as const },
];

function formatModel(model: string): string {
  if (!model) return "No Model";

  // Check Vertex AI models
  if (model.includes("claude-opus-4")) return "Claude Opus 4.5";
  if (model.includes("claude-sonnet-4-5")) return "Claude Sonnet 4.5";
  if (model.includes("claude-haiku-4-5")) return "Claude Haiku 4.5";

  // Check OpenRouter models
  const openRouterModel = OPENROUTER_MODELS.find((m) => m.id === model);
  if (openRouterModel) return openRouterModel.name;

  return model;
}

interface StatusBarProps {
  sessionId: string | null;
}

export function StatusBar({ sessionId }: StatusBarProps) {
  const aiConfig = useAiConfig();
  const { model, status, errorMessage, provider } = aiConfig;
  const inputMode = useInputMode(sessionId ?? "");
  const setInputMode = useStore((state) => state.setInputMode);
  const setAiConfig = useStore((state) => state.setAiConfig);

  // Track OpenRouter availability
  const [openRouterEnabled, setOpenRouterEnabled] = useState(false);
  const [openRouterApiKey, setOpenRouterApiKey] = useState<string | null>(null);

  // Track provider visibility settings
  const [providerVisibility, setProviderVisibility] = useState({
    vertex_ai: true,
    openrouter: true,
  });

  // Check for OpenRouter API key and provider visibility on mount and when dropdown opens
  const refreshProviderSettings = useCallback(async () => {
    try {
      const settings = await getSettings();
      setOpenRouterApiKey(settings.ai.openrouter.api_key);
      setOpenRouterEnabled(!!settings.ai.openrouter.api_key);
      setProviderVisibility({
        vertex_ai: settings.ai.vertex_ai.show_in_selector,
        openrouter: settings.ai.openrouter.show_in_selector,
      });
    } catch (e) {
      console.warn("Failed to get provider settings:", e);
      // Fallback to legacy method for API key
      try {
        const key = await getOpenRouterApiKey();
        setOpenRouterApiKey(key);
        setOpenRouterEnabled(!!key);
      } catch {
        setOpenRouterEnabled(false);
      }
    }
  }, []);

  useEffect(() => {
    refreshProviderSettings();
  }, [refreshProviderSettings]);

  // Listen for settings-updated events to refresh provider visibility and auto-switch if needed
  useEffect(() => {
    const handleSettingsUpdated = async () => {
      try {
        const settings = await getSettings();
        const newOpenRouterApiKey = settings.ai.openrouter.api_key;
        const newOpenRouterEnabled = !!newOpenRouterApiKey;
        const newVisibility = {
          vertex_ai: settings.ai.vertex_ai.show_in_selector,
          openrouter: settings.ai.openrouter.show_in_selector,
        };

        // Update state
        setOpenRouterApiKey(newOpenRouterApiKey);
        setOpenRouterEnabled(newOpenRouterEnabled);
        setProviderVisibility(newVisibility);

        // Check if current provider is now disabled and needs auto-switch
        const isCurrentVertexAi = provider === "anthropic_vertex";
        const isCurrentOpenRouter = provider === "openrouter";
        const vertexDisabled = !newVisibility.vertex_ai;
        const openRouterDisabled = !newVisibility.openrouter || !newOpenRouterEnabled;

        if (isCurrentVertexAi && vertexDisabled) {
          // Current provider (Vertex AI) is disabled, try to switch to OpenRouter
          if (!openRouterDisabled && newOpenRouterApiKey) {
            const firstModel = OPENROUTER_MODELS[0];
            setAiConfig({ status: "initializing", model: firstModel.id });
            const workspace = aiConfig.vertexConfig?.workspace ?? ".";
            await initAiAgent({
              workspace,
              provider: "openrouter",
              model: firstModel.id,
              apiKey: newOpenRouterApiKey,
            });
            setAiConfig({ status: "ready", provider: "openrouter" });
            notify.success(`Switched to ${firstModel.name}`);
          }
        } else if (isCurrentOpenRouter && openRouterDisabled) {
          // Current provider (OpenRouter) is disabled, try to switch to Vertex AI
          if (!vertexDisabled && aiConfig.vertexConfig) {
            const firstModel = VERTEX_MODELS[0];
            setAiConfig({ status: "initializing", model: firstModel.id });
            await initVertexAiAgent({
              workspace: aiConfig.vertexConfig.workspace,
              credentialsPath: aiConfig.vertexConfig.credentialsPath,
              projectId: aiConfig.vertexConfig.projectId,
              location: aiConfig.vertexConfig.location,
              model: firstModel.id,
            });
            setAiConfig({ status: "ready", provider: "anthropic_vertex" });
            notify.success(`Switched to ${firstModel.name}`);
          }
        }
      } catch (e) {
        console.warn("Failed to handle settings update:", e);
      }
    };

    window.addEventListener("settings-updated", handleSettingsUpdated);
    return () => {
      window.removeEventListener("settings-updated", handleSettingsUpdated);
    };
  }, [provider, aiConfig.vertexConfig, setAiConfig]);

  const handleModelSelect = async (modelId: string, modelProvider: "vertex" | "openrouter") => {
    // Don't switch if already on this model
    if (
      model === modelId &&
      provider === (modelProvider === "vertex" ? "anthropic_vertex" : "openrouter")
    ) {
      return;
    }

    const allModels = [...VERTEX_MODELS, ...OPENROUTER_MODELS];
    const modelName = allModels.find((m) => m.id === modelId)?.name ?? modelId;

    try {
      setAiConfig({ status: "initializing", model: modelId });

      if (modelProvider === "vertex") {
        // Vertex AI model switch
        if (!aiConfig.vertexConfig) {
          throw new Error("Vertex AI configuration not available");
        }
        const { vertexConfig } = aiConfig;
        await initVertexAiAgent({
          workspace: vertexConfig.workspace,
          credentialsPath: vertexConfig.credentialsPath,
          projectId: vertexConfig.projectId,
          location: vertexConfig.location,
          model: modelId,
        });
        setAiConfig({ status: "ready", provider: "anthropic_vertex" });
      } else {
        // OpenRouter model switch
        const apiKey = openRouterApiKey ?? (await getOpenRouterApiKey());
        if (!apiKey) {
          throw new Error("OpenRouter API key not configured");
        }
        // Get workspace from vertexConfig or use current directory
        const workspace = aiConfig.vertexConfig?.workspace ?? ".";
        await initAiAgent({
          workspace,
          provider: "openrouter",
          model: modelId,
          apiKey,
        });
        setAiConfig({ status: "ready", provider: "openrouter" });
      }

      notify.success(`Switched to ${modelName}`);
    } catch (error) {
      console.error("Failed to switch model:", error);
      setAiConfig({
        status: "error",
        errorMessage: error instanceof Error ? error.message : "Failed to switch model",
      });
      notify.error(`Failed to switch to ${modelName}`);
    }
  };

  return (
    <div className="h-9 bg-card border-t border-[var(--border-subtle)] flex items-center justify-between px-3 text-xs text-muted-foreground relative z-10">
      {/* Left side */}
      <div className="flex items-center gap-3">
        {/* Mode segmented control - icons only */}
        <div className="flex items-center rounded-md bg-muted p-0.5 border border-[var(--border-subtle)]">
          <button
            type="button"
            onClick={() => sessionId && setInputMode(sessionId, "terminal")}
            disabled={!sessionId}
            className={cn(
              "h-7 w-7 flex items-center justify-center rounded transition-all duration-150",
              inputMode === "terminal"
                ? "bg-[var(--bg-hover)] text-accent"
                : "text-muted-foreground hover:text-foreground"
            )}
          >
            <Terminal className="w-4 h-4" />
          </button>
          <button
            type="button"
            onClick={() => sessionId && setInputMode(sessionId, "agent")}
            disabled={!sessionId}
            className={cn(
              "h-7 w-7 flex items-center justify-center rounded transition-all duration-150",
              inputMode === "agent"
                ? "bg-[var(--bg-hover)] text-accent"
                : "text-muted-foreground hover:text-foreground"
            )}
          >
            <Bot className="w-4 h-4" />
          </button>
        </div>

        {/* Model selector badge or Terminal Mode indicator */}
        {inputMode === "terminal" ? (
          <div className="h-6 px-2.5 gap-1.5 text-xs font-normal rounded-md bg-muted text-muted-foreground flex items-center">
            <Terminal className="w-3.5 h-3.5 text-accent" />
            <span>Terminal</span>
          </div>
        ) : status === "disconnected" ? (
          <div className="h-6 px-2.5 gap-1.5 text-xs font-normal rounded-md bg-muted text-muted-foreground flex items-center">
            <Cpu className="w-3.5 h-3.5" />
            <span>AI Disconnected</span>
          </div>
        ) : status === "error" ? (
          <div className="h-6 px-2.5 gap-1.5 text-xs font-normal rounded-md bg-destructive/10 text-destructive flex items-center">
            <Cpu className="w-3.5 h-3.5" />
            <span>AI Error</span>
          </div>
        ) : status === "initializing" ? (
          <div className="h-6 px-2.5 gap-1.5 text-xs font-normal rounded-md bg-[var(--accent-dim)] text-accent flex items-center">
            <Cpu className="w-3.5 h-3.5 animate-pulse" />
            <span>Initializing...</span>
          </div>
        ) : (
          // Check if any providers have visible models
          (() => {
            const showVertexAi = providerVisibility.vertex_ai && !!aiConfig.vertexConfig;
            const showOpenRouter = providerVisibility.openrouter && openRouterEnabled;
            const hasVisibleProviders = showVertexAi || showOpenRouter;

            if (!hasVisibleProviders) {
              // All providers are hidden - show message
              return (
                <div className="h-6 px-2.5 gap-1.5 text-xs font-normal rounded-md bg-muted text-muted-foreground flex items-center">
                  <Cpu className="w-3.5 h-3.5" />
                  <span>Enable a provider in settings</span>
                </div>
              );
            }

            return (
              <DropdownMenu onOpenChange={(open) => open && refreshProviderSettings()}>
                <DropdownMenuTrigger asChild>
                  <Button
                    variant="ghost"
                    size="sm"
                    className="h-6 px-2.5 gap-1.5 text-xs font-normal rounded-md bg-[var(--accent-dim)] text-accent hover:bg-accent/20 hover:text-accent"
                  >
                    <Cpu className="w-3.5 h-3.5" />
                    <span>{formatModel(model)}</span>
                    <ChevronDown className="w-4 h-4" />
                  </Button>
                </DropdownMenuTrigger>
                <DropdownMenuContent
                  align="start"
                  className="bg-card border-[var(--border-medium)] min-w-[200px]"
                >
                  {/* Vertex AI Models - show only if visibility is enabled */}
                  {showVertexAi && (
                    <>
                      <div className="px-2 py-1 text-[10px] text-muted-foreground uppercase tracking-wide">
                        Vertex AI
                      </div>
                      {VERTEX_MODELS.map((m) => (
                        <DropdownMenuItem
                          key={m.id}
                          onClick={() => handleModelSelect(m.id, "vertex")}
                          disabled={!aiConfig.vertexConfig}
                          className={cn(
                            "text-xs cursor-pointer",
                            model === m.id && provider === "anthropic_vertex"
                              ? "text-accent bg-[var(--accent-dim)]"
                              : "text-foreground hover:text-accent"
                          )}
                        >
                          {m.name}
                        </DropdownMenuItem>
                      ))}
                    </>
                  )}

                  {/* OpenRouter Models - show only if visibility is enabled AND API key configured */}
                  {showOpenRouter && (
                    <>
                      {showVertexAi && <DropdownMenuSeparator />}
                      <div className="px-2 py-1 text-[10px] text-muted-foreground uppercase tracking-wide">
                        OpenRouter
                      </div>
                      {OPENROUTER_MODELS.map((m) => (
                        <DropdownMenuItem
                          key={m.id}
                          onClick={() => handleModelSelect(m.id, "openrouter")}
                          className={cn(
                            "text-xs cursor-pointer",
                            model === m.id && provider === "openrouter"
                              ? "text-accent bg-[var(--accent-dim)]"
                              : "text-foreground hover:text-accent"
                          )}
                        >
                          {m.name}
                        </DropdownMenuItem>
                      ))}
                    </>
                  )}
                </DropdownMenuContent>
              </DropdownMenu>
            );
          })()
        )}
      </div>

      {/* Right side - Status messages and notifications */}
      <div className="flex items-center gap-3">
        {isMockBrowserMode() ? (
          <span className="text-[var(--ansi-yellow)] truncate max-w-[200px]">
            Browser only mode enabled
          </span>
        ) : (
          status === "error" &&
          errorMessage && (
            <span className="text-destructive truncate max-w-[200px]">({errorMessage})</span>
          )
        )}
        <NotificationWidget />
      </div>
    </div>
  );
}
