/**
 * InputStatusRow - Status elements for per-pane display.
 * Contains mode toggle, model selector, token usage, and context metrics.
 * This component was extracted from StatusBar to support multi-pane layouts.
 */

import { Bot, Bug, Cpu, Gauge, Terminal } from "lucide-react";
import { type JSX, useCallback, useEffect, useRef, useState } from "react";
import { SiOpentelemetry } from "react-icons/si";
import { AgentModeSelector } from "@/components/AgentModeSelector";
import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuSub,
  DropdownMenuSubContent,
  DropdownMenuSubTrigger,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { Popover, PopoverContent, PopoverTrigger } from "@/components/ui/popover";
import { useProviderSettings } from "@/hooks/useProviderSettings";
import {
  type ApiRequestStatsSnapshot,
  getApiRequestStats,
  getOpenAiApiKey,
  getOpenRouterApiKey,
  initAiSession,
  type ProviderConfig,
  type ReasoningEffort,
  saveProjectModel,
} from "@/lib/ai";
import { logger } from "@/lib/logger";
import {
  formatModelName,
  getProviderGroup,
  getProviderGroupNested,
  type ModelEntry,
} from "@/lib/models";
import { notify } from "@/lib/notify";
import { cn } from "@/lib/utils";
import { isMockBrowserMode } from "@/mocks";
import { useContextMetrics, useInputMode, useSessionAiConfig, useStore } from "@/store";

interface InputStatusRowProps {
  sessionId: string;
}

/**
 * Format token count with commas for detailed display.
 */
function formatTokenCountDetailed(tokens: number): string {
  return tokens.toLocaleString();
}

/**
 * Format uptime from a Unix timestamp to a human-readable string.
 */
function formatUptime(startedAtMs: number): string {
  const now = Date.now();
  const elapsedMs = now - startedAtMs;

  if (elapsedMs < 0) return "0s";

  const seconds = Math.floor(elapsedMs / 1000);
  const minutes = Math.floor(seconds / 60);
  const hours = Math.floor(minutes / 60);

  if (hours > 0) {
    const remainingMinutes = minutes % 60;
    return `${hours}h ${remainingMinutes}m`;
  }
  if (minutes > 0) {
    const remainingSeconds = seconds % 60;
    return `${minutes}m ${remainingSeconds}s`;
  }
  return `${seconds}s`;
}

function formatRelativeTime(timestampMs: number): string {
  const now = Date.now();
  const diffMs = now - timestampMs;
  const diffSec = Math.floor(diffMs / 1000);
  const diffMin = Math.floor(diffSec / 60);
  const diffHour = Math.floor(diffMin / 60);

  if (diffSec < 60) return "just now";
  if (diffMin < 60) return `${diffMin}m ago`;
  if (diffHour < 24) return `${diffHour}h ago`;
  return new Date(timestampMs).toLocaleDateString();
}

// How long to show labels before hiding them (in ms)
const LABEL_HIDE_DELAY = 3000;

export function InputStatusRow({ sessionId }: InputStatusRowProps) {
  const aiConfig = useSessionAiConfig(sessionId);
  const model = aiConfig?.model ?? "";
  const status = aiConfig?.status ?? "disconnected";
  const errorMessage = aiConfig?.errorMessage;
  const provider = aiConfig?.provider ?? "";
  const currentReasoningEffort = aiConfig?.reasoningEffort;
  const inputMode = useInputMode(sessionId);
  const setInputMode = useStore((state) => state.setInputMode);
  const setSessionAiConfig = useStore((state) => state.setSessionAiConfig);

  // Auto-hide labels after a delay in agent mode
  const [showLabels, setShowLabels] = useState(true);
  const hideTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const containerRef = useRef<HTMLDivElement>(null);

  // Reset timer when model changes
  // biome-ignore lint/correctness/useExhaustiveDependencies: We want to show labels when model changes
  useEffect(() => {
    setShowLabels(true);
    if (hideTimerRef.current) {
      clearTimeout(hideTimerRef.current);
    }
    hideTimerRef.current = setTimeout(() => {
      setShowLabels(false);
    }, LABEL_HIDE_DELAY);

    return () => {
      if (hideTimerRef.current) {
        clearTimeout(hideTimerRef.current);
      }
    };
  }, [model]);

  // Show labels on hover, hide after delay when mouse leaves
  const handleMouseEnter = useCallback(() => {
    setShowLabels(true);
    if (hideTimerRef.current) {
      clearTimeout(hideTimerRef.current);
      hideTimerRef.current = null;
    }
  }, []);

  const handleMouseLeave = useCallback(() => {
    hideTimerRef.current = setTimeout(() => {
      setShowLabels(false);
    }, LABEL_HIDE_DELAY);
  }, []);
  const sessionWorkingDirectory = useStore((state) => state.sessions[sessionId]?.workingDirectory);
  const contextMetrics = useContextMetrics(sessionId);

  // Use consolidated provider settings hook
  const [providerSettings, refreshProviderSettings] = useProviderSettings();

  // Extract values from consolidated state for easier access
  const {
    enabled: providerEnabled,
    apiKeys,
    vertexAiCredentials,
    vertexGeminiCredentials,
    visibility: providerVisibility,
    langfuseActive,
    telemetryStats,
  } = providerSettings;

  const [debugOpen, setDebugOpen] = useState(false);
  const debugPollRef = useRef<ReturnType<typeof setInterval> | null>(null);
  const [apiRequestStats, setApiRequestStats] = useState<ApiRequestStatsSnapshot | null>(null);
  const [apiRequestStatsError, setApiRequestStatsError] = useState<string | null>(null);

  const refreshApiRequestStats = useCallback(async () => {
    try {
      const stats = await getApiRequestStats(sessionId);
      setApiRequestStats(stats);
      setApiRequestStatsError(null);
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      if (
        message.includes("AI agent not initialized for session") ||
        message.includes("Call init_ai_session first")
      ) {
        setApiRequestStats(null);
        setApiRequestStatsError(null);
        return;
      }
      setApiRequestStatsError(message);
    }
  }, [sessionId]);

  useEffect(() => {
    if (!debugOpen) {
      if (debugPollRef.current) {
        clearInterval(debugPollRef.current);
        debugPollRef.current = null;
      }
      return;
    }

    refreshApiRequestStats();
    debugPollRef.current = setInterval(() => {
      refreshApiRequestStats();
    }, 1500);

    return () => {
      if (debugPollRef.current) {
        clearInterval(debugPollRef.current);
        debugPollRef.current = null;
      }
    };
  }, [debugOpen, refreshApiRequestStats]);

  const handleModelSelect = async (
    modelId: string,
    modelProvider:
      | "vertex"
      | "vertex_gemini"
      | "openrouter"
      | "openai"
      | "anthropic"
      | "ollama"
      | "gemini"
      | "groq"
      | "xai"
      | "zai_sdk",
    reasoningEffort?: ReasoningEffort
  ) => {
    // Don't switch if already on this model (and same reasoning effort for OpenAI)
    const providerMap = {
      vertex: "anthropic_vertex",
      vertex_gemini: "vertex_gemini",
      openrouter: "openrouter",
      openai: "openai",
      anthropic: "anthropic",
      ollama: "ollama",
      gemini: "gemini",
      groq: "groq",
      xai: "xai",
      zai_sdk: "zai_sdk",
    };
    if (model === modelId && provider === providerMap[modelProvider]) {
      // For OpenAI, also check reasoning effort
      if (modelProvider !== "openai" || reasoningEffort === currentReasoningEffort) {
        return;
      }
    }

    const modelName = formatModelName(modelId, reasoningEffort);
    const workspace = aiConfig?.vertexConfig?.workspace ?? sessionWorkingDirectory ?? ".";

    try {
      setSessionAiConfig(sessionId, { status: "initializing", model: modelId });

      let config: ProviderConfig;

      if (modelProvider === "vertex") {
        // Vertex AI model switch - use session config if available, otherwise use settings credentials
        const vertexConfig = aiConfig?.vertexConfig;
        const credentials = vertexConfig
          ? {
              credentials_path: vertexConfig.credentialsPath,
              project_id: vertexConfig.projectId,
              location: vertexConfig.location,
            }
          : vertexAiCredentials;

        if (!credentials?.credentials_path && !credentials?.project_id) {
          throw new Error("Vertex AI credentials not configured");
        }

        const credentialsPath = credentials.credentials_path ?? "";
        const projectId = credentials.project_id ?? "";
        const location = credentials.location ?? "us-east5";

        config = {
          provider: "vertex_ai",
          workspace,
          model: modelId,
          credentials_path: credentialsPath,
          project_id: projectId,
          location: location,
        };
        await initAiSession(sessionId, config);
        setSessionAiConfig(sessionId, {
          status: "ready",
          provider: "anthropic_vertex",
          vertexConfig: {
            workspace,
            credentialsPath,
            projectId,
            location,
          },
        });
      } else if (modelProvider === "vertex_gemini") {
        // Vertex Gemini model switch - use settings credentials
        const credentials = vertexGeminiCredentials;

        if (!credentials?.credentials_path && !credentials?.project_id) {
          throw new Error("Vertex Gemini credentials not configured");
        }

        const credentialsPath = credentials.credentials_path ?? "";
        const projectId = credentials.project_id ?? "";
        const location = credentials.location ?? "us-central1";

        config = {
          provider: "vertex_gemini",
          workspace,
          model: modelId,
          credentials_path: credentialsPath,
          project_id: projectId,
          location: location,
        };
        await initAiSession(sessionId, config);
        setSessionAiConfig(sessionId, {
          status: "ready",
          provider: "vertex_gemini",
        });
      } else if (modelProvider === "openrouter") {
        // OpenRouter model switch
        const apiKey = apiKeys.openrouter ?? (await getOpenRouterApiKey());
        if (!apiKey) {
          throw new Error("OpenRouter API key not configured");
        }
        config = {
          provider: "openrouter",
          workspace,
          model: modelId,
          api_key: apiKey,
        };
        await initAiSession(sessionId, config);
        setSessionAiConfig(sessionId, { status: "ready", provider: "openrouter" });
      } else if (modelProvider === "openai") {
        // OpenAI model switch
        const apiKey = apiKeys.openai ?? (await getOpenAiApiKey());
        if (!apiKey) {
          throw new Error("OpenAI API key not configured");
        }
        config = {
          provider: "openai",
          workspace,
          model: modelId,
          api_key: apiKey,
          reasoning_effort: reasoningEffort,
        };
        await initAiSession(sessionId, config);
        setSessionAiConfig(sessionId, { status: "ready", provider: "openai", reasoningEffort });
      } else if (modelProvider === "anthropic") {
        // Anthropic direct API model switch
        const apiKey = apiKeys.anthropic;
        if (!apiKey) {
          throw new Error("Anthropic API key not configured");
        }
        config = {
          provider: "anthropic",
          workspace,
          model: modelId,
          api_key: apiKey,
        };
        await initAiSession(sessionId, config);
        setSessionAiConfig(sessionId, { status: "ready", provider: "anthropic" });
      } else if (modelProvider === "ollama") {
        // Ollama local model switch
        config = {
          provider: "ollama",
          workspace,
          model: modelId,
        };
        await initAiSession(sessionId, config);
        setSessionAiConfig(sessionId, { status: "ready", provider: "ollama" });
      } else if (modelProvider === "gemini") {
        // Gemini model switch
        const apiKey = apiKeys.gemini;
        if (!apiKey) {
          throw new Error("Gemini API key not configured");
        }
        config = {
          provider: "gemini",
          workspace,
          model: modelId,
          api_key: apiKey,
        };
        await initAiSession(sessionId, config);
        setSessionAiConfig(sessionId, { status: "ready", provider: "gemini" });
      } else if (modelProvider === "groq") {
        // Groq model switch
        const apiKey = apiKeys.groq;
        if (!apiKey) {
          throw new Error("Groq API key not configured");
        }
        config = {
          provider: "groq",
          workspace,
          model: modelId,
          api_key: apiKey,
        };
        await initAiSession(sessionId, config);
        setSessionAiConfig(sessionId, { status: "ready", provider: "groq" });
      } else if (modelProvider === "xai") {
        // xAI model switch
        const apiKey = apiKeys.xai;
        if (!apiKey) {
          throw new Error("xAI API key not configured");
        }
        config = {
          provider: "xai",
          workspace,
          model: modelId,
          api_key: apiKey,
        };
        await initAiSession(sessionId, config);
        setSessionAiConfig(sessionId, { status: "ready", provider: "xai" });
      } else if (modelProvider === "zai_sdk") {
        // Z.AI SDK model switch
        const apiKey = apiKeys.zai_sdk;
        if (!apiKey) {
          throw new Error("Z.AI SDK API key not configured");
        }
        config = {
          provider: "zai_sdk",
          workspace,
          model: modelId,
          api_key: apiKey,
        };
        await initAiSession(sessionId, config);
        setSessionAiConfig(sessionId, { status: "ready", provider: "zai_sdk" });
      }

      notify.success(`Switched to ${modelName}`);

      // Save model selection to per-project settings
      try {
        // Map internal provider names to settings provider format
        const providerForSettings = modelProvider === "vertex" ? "vertex_ai" : modelProvider;
        await saveProjectModel(workspace, providerForSettings, modelId);
      } catch (saveError) {
        // Don't fail the switch if saving settings fails
        logger.warn("Failed to save project model settings:", saveError);
      }
    } catch (error) {
      logger.error("Failed to switch model:", error);
      setSessionAiConfig(sessionId, {
        status: "error",
        errorMessage: error instanceof Error ? error.message : "Failed to switch model",
      });
      notify.error(`Failed to switch to ${modelName}`);
    }
  };

  // Compute visibility flags for rendering
  const showVertexAi = providerVisibility.vertex_ai && providerEnabled.vertex_ai;
  const showVertexGemini = providerVisibility.vertex_gemini && providerEnabled.vertex_gemini;
  const showOpenRouter = providerVisibility.openrouter && providerEnabled.openrouter;
  const showOpenAi = providerVisibility.openai && providerEnabled.openai;
  const showAnthropic = providerVisibility.anthropic && providerEnabled.anthropic;
  const showOllama = providerVisibility.ollama && providerEnabled.ollama;
  const showGemini = providerVisibility.gemini && providerEnabled.gemini;
  const showGroq = providerVisibility.groq && providerEnabled.groq;
  const showXai = providerVisibility.xai && providerEnabled.xai;
  const showZaiSdk = providerVisibility.zai_sdk && providerEnabled.zai_sdk;
  const hasVisibleProviders =
    showVertexAi ||
    showVertexGemini ||
    showOpenRouter ||
    showOpenAi ||
    showAnthropic ||
    showOllama ||
    showGemini ||
    showGroq ||
    showXai ||
    showZaiSdk;

  return (
    // biome-ignore lint/a11y/noStaticElementInteractions: Used for hover interactions to show/hide labels
    <div
      ref={containerRef}
      role="presentation"
      className="flex items-center justify-between px-3 py-1 text-xs text-muted-foreground"
      onMouseEnter={handleMouseEnter}
      onMouseLeave={handleMouseLeave}
    >
      {/* Left side */}
      <div className="flex items-center gap-2">
        {/* Mode segmented control - icons only */}
        <div className="flex items-center rounded-lg bg-muted/50 p-0.5 border border-[var(--border-subtle)]/50">
          <button
            type="button"
            aria-label="Switch to Terminal mode"
            title="Terminal"
            onClick={() => setInputMode(sessionId, "terminal")}
            className={cn(
              "h-6 w-6 flex items-center justify-center rounded-md transition-all duration-200",
              inputMode === "terminal"
                ? "bg-accent/15 text-accent shadow-[0_0_8px_rgba(var(--accent-rgb),0.3)]"
                : "text-muted-foreground hover:text-foreground hover:bg-muted"
            )}
          >
            <Terminal className="w-3.5 h-3.5" />
          </button>
          <button
            type="button"
            aria-label="Switch to AI mode"
            title="AI"
            onClick={() => setInputMode(sessionId, "agent")}
            className={cn(
              "h-6 w-6 flex items-center justify-center rounded-md transition-all duration-200",
              inputMode === "agent"
                ? "bg-accent/15 text-accent shadow-[0_0_8px_rgba(var(--accent-rgb),0.3)]"
                : "text-muted-foreground hover:text-foreground hover:bg-muted"
            )}
          >
            <Bot className="w-3.5 h-3.5" />
          </button>
        </div>

        {/* Divider */}
        <div className="h-4 w-px bg-[var(--border-medium)]" />

        {/* Model selector badge */}
        {status === "disconnected" ? (
          <div className="h-6 px-2.5 gap-1.5 text-xs font-medium rounded-lg bg-muted/60 text-muted-foreground flex items-center border border-transparent">
            <Cpu className="w-3.5 h-3.5" />
            <span>AI Disconnected</span>
          </div>
        ) : status === "error" ? (
          <div className="h-6 px-2.5 gap-1.5 text-xs font-medium rounded-lg bg-destructive/10 text-destructive flex items-center border border-destructive/20">
            <Cpu className="w-3.5 h-3.5" />
            <span>AI Error</span>
          </div>
        ) : status === "initializing" ? (
          <div className="h-6 px-2.5 gap-1.5 text-xs font-medium rounded-lg bg-accent/10 text-accent flex items-center border border-accent/20">
            <Cpu className="w-3.5 h-3.5 animate-pulse" />
            <span>Initializing...</span>
          </div>
        ) : !hasVisibleProviders ? (
          <div className="h-6 px-2.5 gap-1.5 text-xs font-medium rounded-lg bg-muted/60 text-muted-foreground flex items-center border border-transparent">
            <Cpu className="w-3.5 h-3.5" />
            <span>Enable a provider in settings</span>
          </div>
        ) : (
          <DropdownMenu onOpenChange={(open) => open && refreshProviderSettings()}>
            <DropdownMenuTrigger asChild>
              <Button
                variant="ghost"
                size="sm"
                className={cn(
                  "h-6 text-xs font-medium rounded-lg bg-accent/10 text-accent hover:text-accent hover:bg-accent/20 border border-accent/20 hover:border-accent/30 transition-all duration-200",
                  showLabels ? "gap-1.5 px-2.5" : "gap-0 px-2"
                )}
              >
                <Cpu className="w-3.5 h-3.5" />
                <span
                  className={cn(
                    "transition-all duration-200 overflow-hidden",
                    showLabels ? "max-w-[150px] opacity-100" : "max-w-0 opacity-0"
                  )}
                >
                  {formatModelName(model, currentReasoningEffort)}
                </span>
              </Button>
            </DropdownMenuTrigger>
            <DropdownMenuContent
              align="start"
              className="bg-card border-[var(--border-medium)] min-w-[200px]"
            >
              {/* Vertex AI Models */}
              {showVertexAi && (
                <>
                  <div className="px-2 py-1 text-[10px] text-muted-foreground uppercase tracking-wide">
                    Vertex AI
                  </div>
                  {(getProviderGroup("vertex_ai")?.models ?? []).map((m) => (
                    <DropdownMenuItem
                      key={m.id}
                      onClick={() => handleModelSelect(m.id, "vertex")}
                      disabled={!aiConfig?.vertexConfig && !vertexAiCredentials}
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

              {/* Vertex Gemini Models with nested groups */}
              {showVertexGemini &&
                (() => {
                  // Helper to check if any nested model is selected
                  const isAnyNestedSelected = (entries: ModelEntry[]): boolean => {
                    return entries.some((e) => {
                      if (e.id) {
                        return provider === "vertex_gemini" && model === e.id;
                      }
                      if (e.subModels) {
                        return isAnyNestedSelected(e.subModels);
                      }
                      return false;
                    });
                  };

                  // Recursive renderer for model entries
                  const renderModelEntry = (
                    entry: ModelEntry,
                    keyPrefix: string
                  ): JSX.Element | null => {
                    // Entry with sub-options (nested menu)
                    if (entry.subModels && entry.subModels.length > 0) {
                      const isSubSelected = isAnyNestedSelected(entry.subModels);
                      return (
                        <DropdownMenuSub key={`${keyPrefix}-${entry.name}`}>
                          <DropdownMenuSubTrigger
                            className={cn(
                              "text-xs cursor-pointer",
                              isSubSelected
                                ? "text-accent bg-[var(--accent-dim)]"
                                : "text-foreground hover:text-accent"
                            )}
                          >
                            {entry.name}
                          </DropdownMenuSubTrigger>
                          <DropdownMenuSubContent className="bg-card border-[var(--border-medium)]">
                            {entry.subModels.map((sub) =>
                              renderModelEntry(sub, `${keyPrefix}-${entry.name}`)
                            )}
                          </DropdownMenuSubContent>
                        </DropdownMenuSub>
                      );
                    }

                    // Leaf model (selectable)
                    if (!entry.id) return null;
                    const entryId = entry.id;
                    const isSelected = provider === "vertex_gemini" && model === entryId;
                    return (
                      <DropdownMenuItem
                        key={`${keyPrefix}-${entryId}`}
                        onClick={() => handleModelSelect(entryId, "vertex_gemini")}
                        disabled={!vertexGeminiCredentials}
                        className={cn(
                          "text-xs cursor-pointer",
                          isSelected
                            ? "text-accent bg-[var(--accent-dim)]"
                            : "text-foreground hover:text-accent"
                        )}
                      >
                        {entry.name}
                      </DropdownMenuItem>
                    );
                  };

                  return (
                    <>
                      {showVertexAi && <DropdownMenuSeparator />}
                      <div className="px-2 py-1 text-[10px] text-muted-foreground uppercase tracking-wide">
                        Vertex AI Gemini
                      </div>
                      {(getProviderGroupNested("vertex_gemini")?.models ?? []).map((entry) =>
                        renderModelEntry(entry, "vertex_gemini")
                      )}
                    </>
                  );
                })()}

              {/* OpenRouter Models */}
              {showOpenRouter && (
                <>
                  {(showVertexAi || showVertexGemini) && <DropdownMenuSeparator />}
                  <div className="px-2 py-1 text-[10px] text-muted-foreground uppercase tracking-wide">
                    OpenRouter
                  </div>
                  {(getProviderGroup("openrouter")?.models ?? []).map((m) => (
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

              {/* OpenAI Models with nested reasoning effort */}
              {showOpenAi &&
                (() => {
                  // Helper to check if any nested model is selected
                  const isAnyNestedSelected = (entries: ModelEntry[]): boolean => {
                    return entries.some((e) => {
                      if (e.id) {
                        return (
                          provider === "openai" &&
                          model === e.id &&
                          e.reasoningEffort === currentReasoningEffort
                        );
                      }
                      if (e.subModels) {
                        return isAnyNestedSelected(e.subModels);
                      }
                      return false;
                    });
                  };

                  // Recursive renderer for model entries
                  const renderModelEntry = (
                    entry: ModelEntry,
                    keyPrefix: string
                  ): JSX.Element | null => {
                    // Entry with sub-options (nested menu)
                    if (entry.subModels && entry.subModels.length > 0) {
                      const isSubSelected = isAnyNestedSelected(entry.subModels);
                      return (
                        <DropdownMenuSub key={`${keyPrefix}-${entry.name}`}>
                          <DropdownMenuSubTrigger
                            className={cn(
                              "text-xs cursor-pointer",
                              isSubSelected
                                ? "text-accent bg-[var(--accent-dim)]"
                                : "text-foreground hover:text-accent"
                            )}
                          >
                            {entry.name}
                          </DropdownMenuSubTrigger>
                          <DropdownMenuSubContent className="bg-card border-[var(--border-medium)]">
                            {entry.subModels.map((sub) =>
                              renderModelEntry(sub, `${keyPrefix}-${entry.name}`)
                            )}
                          </DropdownMenuSubContent>
                        </DropdownMenuSub>
                      );
                    }

                    // Leaf model (selectable)
                    if (!entry.id) return null;
                    const entryId = entry.id;
                    const isSelected =
                      provider === "openai" &&
                      model === entryId &&
                      entry.reasoningEffort === currentReasoningEffort;
                    return (
                      <DropdownMenuItem
                        key={`${keyPrefix}-${entryId}-${entry.reasoningEffort ?? "default"}`}
                        onClick={() => handleModelSelect(entryId, "openai", entry.reasoningEffort)}
                        className={cn(
                          "text-xs cursor-pointer",
                          isSelected
                            ? "text-accent bg-[var(--accent-dim)]"
                            : "text-foreground hover:text-accent"
                        )}
                      >
                        {entry.name}
                      </DropdownMenuItem>
                    );
                  };

                  return (
                    <>
                      {(showVertexAi || showVertexGemini || showOpenRouter) && (
                        <DropdownMenuSeparator />
                      )}
                      <div className="px-2 py-1 text-[10px] text-muted-foreground uppercase tracking-wide">
                        OpenAI
                      </div>
                      {(getProviderGroupNested("openai")?.models ?? []).map((entry) =>
                        renderModelEntry(entry, "openai")
                      )}
                    </>
                  );
                })()}

              {/* Anthropic Models */}
              {showAnthropic && (
                <>
                  {(showVertexAi || showVertexGemini || showOpenRouter || showOpenAi) && (
                    <DropdownMenuSeparator />
                  )}
                  <div className="px-2 py-1 text-[10px] text-muted-foreground uppercase tracking-wide">
                    Anthropic
                  </div>
                  {(getProviderGroup("anthropic")?.models ?? []).map((m) => (
                    <DropdownMenuItem
                      key={m.id}
                      onClick={() => handleModelSelect(m.id, "anthropic")}
                      className={cn(
                        "text-xs cursor-pointer",
                        model === m.id && provider === "anthropic"
                          ? "text-accent bg-[var(--accent-dim)]"
                          : "text-foreground hover:text-accent"
                      )}
                    >
                      {m.name}
                    </DropdownMenuItem>
                  ))}
                </>
              )}

              {/* Ollama */}
              {showOllama && (
                <>
                  {(showVertexAi ||
                    showVertexGemini ||
                    showOpenRouter ||
                    showOpenAi ||
                    showAnthropic) && <DropdownMenuSeparator />}
                  <div className="px-2 py-1 text-[10px] text-muted-foreground uppercase tracking-wide">
                    Ollama (Local)
                  </div>
                  <div className="px-2 py-1.5 text-xs text-muted-foreground">
                    Configure in settings
                  </div>
                </>
              )}

              {/* Gemini Models */}
              {showGemini && (
                <>
                  {(showVertexAi ||
                    showVertexGemini ||
                    showOpenRouter ||
                    showOpenAi ||
                    showAnthropic ||
                    showOllama) && <DropdownMenuSeparator />}
                  <div className="px-2 py-1 text-[10px] text-muted-foreground uppercase tracking-wide">
                    Google Gemini
                  </div>
                  {(getProviderGroup("gemini")?.models ?? []).map((m) => (
                    <DropdownMenuItem
                      key={m.id}
                      onClick={() => handleModelSelect(m.id, "gemini")}
                      className={cn(
                        "text-xs cursor-pointer",
                        model === m.id && provider === "gemini"
                          ? "text-accent bg-[var(--accent-dim)]"
                          : "text-foreground hover:text-accent"
                      )}
                    >
                      {m.name}
                    </DropdownMenuItem>
                  ))}
                </>
              )}

              {/* Groq Models */}
              {showGroq && (
                <>
                  {(showVertexAi ||
                    showVertexGemini ||
                    showOpenRouter ||
                    showOpenAi ||
                    showAnthropic ||
                    showOllama ||
                    showGemini) && <DropdownMenuSeparator />}
                  <div className="px-2 py-1 text-[10px] text-muted-foreground uppercase tracking-wide">
                    Groq
                  </div>
                  {(getProviderGroup("groq")?.models ?? []).map((m) => (
                    <DropdownMenuItem
                      key={m.id}
                      onClick={() => handleModelSelect(m.id, "groq")}
                      className={cn(
                        "text-xs cursor-pointer",
                        model === m.id && provider === "groq"
                          ? "text-accent bg-[var(--accent-dim)]"
                          : "text-foreground hover:text-accent"
                      )}
                    >
                      {m.name}
                    </DropdownMenuItem>
                  ))}
                </>
              )}

              {/* xAI Models */}
              {showXai && (
                <>
                  {(showVertexAi ||
                    showVertexGemini ||
                    showOpenRouter ||
                    showOpenAi ||
                    showAnthropic ||
                    showOllama ||
                    showGemini ||
                    showGroq) && <DropdownMenuSeparator />}
                  <div className="px-2 py-1 text-[10px] text-muted-foreground uppercase tracking-wide">
                    xAI (Grok)
                  </div>
                  {(getProviderGroup("xai")?.models ?? []).map((m) => (
                    <DropdownMenuItem
                      key={m.id}
                      onClick={() => handleModelSelect(m.id, "xai")}
                      className={cn(
                        "text-xs cursor-pointer",
                        model === m.id && provider === "xai"
                          ? "text-accent bg-[var(--accent-dim)]"
                          : "text-foreground hover:text-accent"
                      )}
                    >
                      {m.name}
                    </DropdownMenuItem>
                  ))}
                </>
              )}

              {/* Z.AI SDK Models */}
              {showZaiSdk && (
                <>
                  {(showVertexAi ||
                    showVertexGemini ||
                    showOpenRouter ||
                    showOpenAi ||
                    showAnthropic ||
                    showOllama ||
                    showGemini ||
                    showGroq ||
                    showXai) && <DropdownMenuSeparator />}
                  <div className="px-2 py-1 text-[10px] text-muted-foreground uppercase tracking-wide">
                    Z.AI SDK
                  </div>
                  {(getProviderGroup("zai_sdk")?.models ?? []).map((m) => (
                    <DropdownMenuItem
                      key={m.id}
                      onClick={() => handleModelSelect(m.id, "zai_sdk")}
                      className={cn(
                        "text-xs cursor-pointer",
                        model === m.id && provider === "zai_sdk"
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
        )}

        {/* Agent Mode Selector */}
        {status === "ready" && <AgentModeSelector sessionId={sessionId} showLabel={showLabels} />}

        {/* Context utilization indicator */}
        {contextMetrics.maxTokens > 0 ? (
          <Popover>
            <PopoverTrigger asChild>
              <button
                type="button"
                title={`Context: ${Math.round(contextMetrics.utilization * 100)}% used`}
                className={cn(
                  "h-6 px-2 gap-1.5 text-xs font-medium rounded-lg flex items-center cursor-pointer transition-all duration-200",
                  contextMetrics.utilization < 0.7 &&
                    "bg-[#9ece6a]/10 text-[#9ece6a] hover:bg-[#9ece6a]/20 border border-[#9ece6a]/20 hover:border-[#9ece6a]/30",
                  contextMetrics.utilization >= 0.7 &&
                    contextMetrics.utilization < 0.85 &&
                    "bg-[#e0af68]/10 text-[#e0af68] hover:bg-[#e0af68]/20 border border-[#e0af68]/20 hover:border-[#e0af68]/30",
                  contextMetrics.utilization >= 0.85 &&
                    "bg-[#f7768e]/10 text-[#f7768e] hover:bg-[#f7768e]/20 border border-[#f7768e]/20 hover:border-[#f7768e]/30"
                )}
              >
                <Gauge className="w-3.5 h-3.5" />
                <span>{Math.round(contextMetrics.utilization * 100)}%</span>
              </button>
            </PopoverTrigger>
            <PopoverContent
              align="end"
              className="w-auto min-w-[200px] p-3 bg-card/95 backdrop-blur-sm border-[var(--border-medium)] shadow-lg"
            >
              <div className="text-xs font-medium text-muted-foreground mb-2">
                Context Window Usage
              </div>
              <div className="font-mono text-xs space-y-1">
                <div className="flex justify-between gap-4">
                  <span className="text-muted-foreground">Used</span>
                  <span className="text-foreground">
                    {formatTokenCountDetailed(contextMetrics.usedTokens)}
                  </span>
                </div>
                <div className="flex justify-between gap-4">
                  <span className="text-muted-foreground">Max</span>
                  <span className="text-foreground">
                    {formatTokenCountDetailed(contextMetrics.maxTokens)}
                  </span>
                </div>
                <div className="border-t border-[var(--border-subtle)] my-1.5" />
                <div className="flex justify-between gap-4">
                  <span className="text-muted-foreground">Utilization</span>
                  <span
                    className={cn(
                      "font-medium",
                      contextMetrics.utilization < 0.7 && "text-[#9ece6a]",
                      contextMetrics.utilization >= 0.7 &&
                        contextMetrics.utilization < 0.85 &&
                        "text-[#e0af68]",
                      contextMetrics.utilization >= 0.85 && "text-[#f7768e]"
                    )}
                  >
                    {Math.round(contextMetrics.utilization * 100)}%
                  </span>
                </div>
              </div>
            </PopoverContent>
          </Popover>
        ) : (
          <button
            type="button"
            title="Context: not available"
            className="h-6 px-2 gap-1.5 text-xs font-medium rounded-lg flex items-center text-muted-foreground/70 border border-[var(--border-subtle)]/60 bg-card/30"
          >
            <Gauge className="w-3.5 h-3.5" />
            <span>0%</span>
          </button>
        )}

        {/* Langfuse tracing indicator with stats */}
        {langfuseActive && (
          <Popover>
            <PopoverTrigger asChild>
              <button
                type="button"
                title="Langfuse tracing enabled"
                className="h-6 px-2 gap-1.5 text-xs font-medium rounded-lg flex items-center bg-[#7c3aed]/10 text-[#7c3aed] hover:bg-[#7c3aed]/20 border border-[#7c3aed]/20 hover:border-[#7c3aed]/30 transition-all duration-200 cursor-pointer"
                onClick={refreshProviderSettings}
              >
                <SiOpentelemetry className="w-3.5 h-3.5" />
                {telemetryStats && telemetryStats.spans_ended > 0 && (
                  <span className="tabular-nums">{telemetryStats.spans_ended}</span>
                )}
              </button>
            </PopoverTrigger>
            <PopoverContent
              align="end"
              className="w-auto min-w-[200px] p-3 bg-card/95 backdrop-blur-sm border-[var(--border-medium)] shadow-lg"
            >
              <div className="text-xs font-medium text-muted-foreground mb-2">Langfuse Tracing</div>
              {telemetryStats ? (
                <div className="font-mono text-xs space-y-1">
                  <div className="flex justify-between gap-4">
                    <span className="text-muted-foreground">Spans Started</span>
                    <span className="text-foreground tabular-nums">
                      {telemetryStats.spans_started.toLocaleString()}
                    </span>
                  </div>
                  <div className="flex justify-between gap-4">
                    <span className="text-muted-foreground">Spans Queued</span>
                    <span className="text-foreground tabular-nums">
                      {telemetryStats.spans_ended.toLocaleString()}
                    </span>
                  </div>
                  <div className="border-t border-[var(--border-subtle)] my-1.5" />
                  <div className="flex justify-between gap-4">
                    <span className="text-muted-foreground">Uptime</span>
                    <span className="text-foreground">
                      {formatUptime(telemetryStats.started_at)}
                    </span>
                  </div>
                </div>
              ) : (
                <div className="text-xs text-muted-foreground">Stats not available</div>
              )}
            </PopoverContent>
          </Popover>
        )}

        {import.meta.env.DEV && !isMockBrowserMode() && (
          <Popover open={debugOpen} onOpenChange={setDebugOpen}>
            <PopoverTrigger asChild>
              <button
                type="button"
                title="Debug (This Tab)"
                className="h-6 px-2 gap-1.5 text-xs font-medium rounded-lg flex items-center bg-[var(--ansi-yellow)]/10 text-[var(--ansi-yellow)] hover:bg-[var(--ansi-yellow)]/20 border border-[var(--ansi-yellow)]/20 hover:border-[var(--ansi-yellow)]/30 transition-all duration-200 cursor-pointer"
              >
                <Bug className="w-3.5 h-3.5" />
                <span>Debug</span>
              </button>
            </PopoverTrigger>
            <PopoverContent
              align="end"
              className="w-[340px] p-3 bg-card/95 backdrop-blur-sm border-[var(--border-medium)] shadow-lg"
            >
              <div className="text-xs font-medium text-muted-foreground mb-1">Debug (This Tab)</div>
              <div className="text-[11px] text-muted-foreground mb-3">
                LLM API Requests (main + sub-agents)
              </div>
              {apiRequestStatsError ? (
                <div className="text-xs text-destructive">{apiRequestStatsError}</div>
              ) : apiRequestStats ? (
                (() => {
                  const providerEntries = Object.entries(apiRequestStats.providers).sort(
                    ([, a], [, b]) => b.requests - a.requests
                  );
                  if (providerEntries.length === 0) {
                    return <div className="text-xs text-muted-foreground">No requests yet.</div>;
                  }
                  return (
                    <div className="space-y-2">
                      <div className="grid grid-cols-[1fr_auto_auto_auto] gap-x-3 text-[10px] uppercase tracking-wide text-muted-foreground">
                        <span>Provider</span>
                        <span className="text-right">Req</span>
                        <span className="text-right">Sent</span>
                        <span className="text-right">Recv</span>
                      </div>
                      <div className="border-t border-[var(--border-subtle)]" />
                      <div className="space-y-1">
                        {providerEntries.map(([name, stats]) => (
                          <div
                            key={name}
                            className="grid grid-cols-[1fr_auto_auto_auto] gap-x-3 text-xs font-mono"
                          >
                            <span className="truncate" title={name}>
                              {name}
                            </span>
                            <span className="text-right tabular-nums">{stats.requests}</span>
                            <span
                              className="text-right tabular-nums"
                              title={
                                stats.last_sent_at
                                  ? new Date(stats.last_sent_at).toLocaleString()
                                  : "—"
                              }
                            >
                              {stats.last_sent_at ? formatRelativeTime(stats.last_sent_at) : "—"}
                            </span>
                            <span
                              className="text-right tabular-nums"
                              title={
                                stats.last_received_at
                                  ? new Date(stats.last_received_at).toLocaleString()
                                  : "—"
                              }
                            >
                              {stats.last_received_at
                                ? formatRelativeTime(stats.last_received_at)
                                : "—"}
                            </span>
                          </div>
                        ))}
                      </div>
                    </div>
                  );
                })()
              ) : (
                <div className="text-xs text-muted-foreground">No requests yet.</div>
              )}
            </PopoverContent>
          </Popover>
        )}
      </div>

      {/* Right side */}
      <div className="flex items-center gap-2">
        {/* Status messages */}
        {isMockBrowserMode() ? (
          <span className="text-[var(--ansi-yellow)] text-[11px] truncate max-w-[200px]">
            Browser only mode
          </span>
        ) : (
          status === "error" &&
          errorMessage && (
            <span className="text-destructive text-[11px] truncate max-w-[200px]">
              ({errorMessage})
            </span>
          )
        )}
      </div>
    </div>
  );
}
