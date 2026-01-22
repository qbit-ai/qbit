/**
 * InputStatusRow - Status elements for per-pane display.
 * Contains mode toggle, model selector, token usage, context metrics, and task plan button.
 * This component was extracted from StatusBar to support multi-pane layouts.
 */

import { Bot, Cpu, Gauge, ListTodo, Terminal } from "lucide-react";
import { type JSX, useCallback, useEffect, useRef, useState } from "react";
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
import {
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
import { getSettings } from "@/lib/settings";
import { cn } from "@/lib/utils";
import { isMockBrowserMode } from "@/mocks";
import { useContextMetrics, useInputMode, useSessionAiConfig, useStore } from "@/store";

interface InputStatusRowProps {
  sessionId: string;
  onOpenTaskPlanner?: () => void;
}

/**
 * Format token count with commas for detailed display.
 */
function formatTokenCountDetailed(tokens: number): string {
  return tokens.toLocaleString();
}

// How long to show labels before hiding them (in ms)
const LABEL_HIDE_DELAY = 3000;

export function InputStatusRow({ sessionId, onOpenTaskPlanner }: InputStatusRowProps) {
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
  const plan = useStore((state) => state.sessions[sessionId]?.plan);
  const sessionWorkingDirectory = useStore((state) => state.sessions[sessionId]?.workingDirectory);
  const contextMetrics = useContextMetrics(sessionId);

  // Track OpenRouter availability
  const [openRouterEnabled, setOpenRouterEnabled] = useState(false);
  const [openRouterApiKey, setOpenRouterApiKey] = useState<string | null>(null);

  // Track OpenAI availability
  const [openAiEnabled, setOpenAiEnabled] = useState(false);
  const [openAiApiKey, setOpenAiApiKey] = useState<string | null>(null);

  // Track Anthropic availability
  const [anthropicEnabled, setAnthropicEnabled] = useState(false);
  const [anthropicApiKey, setAnthropicApiKey] = useState<string | null>(null);

  // Track Ollama availability
  const [ollamaEnabled, setOllamaEnabled] = useState(false);

  // Track Gemini availability
  const [geminiEnabled, setGeminiEnabled] = useState(false);
  const [geminiApiKey, setGeminiApiKey] = useState<string | null>(null);

  // Track Groq availability
  const [groqEnabled, setGroqEnabled] = useState(false);
  const [groqApiKey, setGroqApiKey] = useState<string | null>(null);

  // Track xAI availability
  const [xaiEnabled, setXaiEnabled] = useState(false);
  const [xaiApiKey, setXaiApiKey] = useState<string | null>(null);

  // Track Z.AI availability
  const [zaiEnabled, setZaiEnabled] = useState(false);
  const [zaiApiKey, setZaiApiKey] = useState<string | null>(null);
  const [zaiUseCodingEndpoint, setZaiUseCodingEndpoint] = useState(true);

  // Track Z.AI (Anthropic) availability
  const [zaiAnthropicEnabled, setZaiAnthropicEnabled] = useState(false);
  const [zaiAnthropicApiKey, setZaiAnthropicApiKey] = useState<string | null>(null);

  // Track Vertex AI availability
  const [vertexAiEnabled, setVertexAiEnabled] = useState(false);
  const [vertexAiCredentials, setVertexAiCredentials] = useState<{
    credentials_path: string | null;
    project_id: string | null;
    location: string | null;
  } | null>(null);

  // Track provider visibility settings
  const [providerVisibility, setProviderVisibility] = useState({
    vertex_ai: true,
    openrouter: true,
    openai: true,
    anthropic: true,
    ollama: true,
    gemini: true,
    groq: true,
    xai: true,
    zai: true,
    zai_anthropic: true,
  });

  // Check for provider API keys and visibility on mount and when dropdown opens
  const refreshProviderSettings = useCallback(async () => {
    try {
      const settings = await getSettings();
      setOpenRouterApiKey(settings.ai.openrouter.api_key);
      setOpenRouterEnabled(!!settings.ai.openrouter.api_key);
      setOpenAiApiKey(settings.ai.openai.api_key);
      setOpenAiEnabled(!!settings.ai.openai.api_key);
      setAnthropicApiKey(settings.ai.anthropic.api_key);
      setAnthropicEnabled(!!settings.ai.anthropic.api_key);
      setOllamaEnabled(true); // Ollama doesn't require an API key
      setGeminiApiKey(settings.ai.gemini.api_key);
      setGeminiEnabled(!!settings.ai.gemini.api_key);
      setGroqApiKey(settings.ai.groq.api_key);
      setGroqEnabled(!!settings.ai.groq.api_key);
      setXaiApiKey(settings.ai.xai.api_key);
      setXaiEnabled(!!settings.ai.xai.api_key);
      setZaiApiKey(settings.ai.zai.api_key);
      setZaiEnabled(!!settings.ai.zai.api_key);
      setZaiUseCodingEndpoint(settings.ai.zai.use_coding_endpoint);
      setZaiAnthropicApiKey(settings.ai.zai_anthropic.api_key);
      setZaiAnthropicEnabled(!!settings.ai.zai_anthropic.api_key);
      // Vertex AI - check for credentials_path OR project_id
      const hasVertexCredentials = !!(
        settings.ai.vertex_ai.credentials_path || settings.ai.vertex_ai.project_id
      );
      setVertexAiEnabled(hasVertexCredentials);
      setVertexAiCredentials({
        credentials_path: settings.ai.vertex_ai.credentials_path,
        project_id: settings.ai.vertex_ai.project_id,
        location: settings.ai.vertex_ai.location,
      });
      setProviderVisibility({
        vertex_ai: settings.ai.vertex_ai.show_in_selector,
        openrouter: settings.ai.openrouter.show_in_selector,
        openai: settings.ai.openai.show_in_selector,
        anthropic: settings.ai.anthropic.show_in_selector,
        ollama: settings.ai.ollama.show_in_selector,
        gemini: settings.ai.gemini.show_in_selector,
        groq: settings.ai.groq.show_in_selector,
        xai: settings.ai.xai.show_in_selector,
        zai: settings.ai.zai.show_in_selector,
        zai_anthropic: settings.ai.zai_anthropic.show_in_selector,
      });
    } catch (e) {
      logger.warn("Failed to get provider settings:", e);
      // Fallback to legacy method for API keys
      try {
        const [orKey, oaiKey] = await Promise.all([getOpenRouterApiKey(), getOpenAiApiKey()]);
        setOpenRouterApiKey(orKey);
        setOpenRouterEnabled(!!orKey);
        setOpenAiApiKey(oaiKey);
        setOpenAiEnabled(!!oaiKey);
      } catch {
        setOpenRouterEnabled(false);
        setOpenAiEnabled(false);
      }
    }
  }, []);

  useEffect(() => {
    refreshProviderSettings();
  }, [refreshProviderSettings]);

  // Listen for settings-updated events to refresh provider visibility
  useEffect(() => {
    const handleSettingsUpdated = async () => {
      try {
        const settings = await getSettings();
        setOpenRouterApiKey(settings.ai.openrouter.api_key);
        setOpenRouterEnabled(!!settings.ai.openrouter.api_key);
        setOpenAiApiKey(settings.ai.openai.api_key);
        setOpenAiEnabled(!!settings.ai.openai.api_key);
        setAnthropicApiKey(settings.ai.anthropic.api_key);
        setAnthropicEnabled(!!settings.ai.anthropic.api_key);
        setOllamaEnabled(true); // Ollama doesn't require an API key
        setGeminiApiKey(settings.ai.gemini.api_key);
        setGeminiEnabled(!!settings.ai.gemini.api_key);
        setGroqApiKey(settings.ai.groq.api_key);
        setGroqEnabled(!!settings.ai.groq.api_key);
        setXaiApiKey(settings.ai.xai.api_key);
        setXaiEnabled(!!settings.ai.xai.api_key);
        setZaiApiKey(settings.ai.zai.api_key);
        setZaiEnabled(!!settings.ai.zai.api_key);
        setZaiUseCodingEndpoint(settings.ai.zai.use_coding_endpoint);
        setZaiAnthropicApiKey(settings.ai.zai_anthropic.api_key);
        setZaiAnthropicEnabled(!!settings.ai.zai_anthropic.api_key);
        // Vertex AI - check for credentials_path OR project_id
        const hasVertexCredentials = !!(
          settings.ai.vertex_ai.credentials_path || settings.ai.vertex_ai.project_id
        );
        setVertexAiEnabled(hasVertexCredentials);
        setVertexAiCredentials({
          credentials_path: settings.ai.vertex_ai.credentials_path,
          project_id: settings.ai.vertex_ai.project_id,
          location: settings.ai.vertex_ai.location,
        });
        setProviderVisibility({
          vertex_ai: settings.ai.vertex_ai.show_in_selector,
          openrouter: settings.ai.openrouter.show_in_selector,
          openai: settings.ai.openai.show_in_selector,
          anthropic: settings.ai.anthropic.show_in_selector,
          ollama: settings.ai.ollama.show_in_selector,
          gemini: settings.ai.gemini.show_in_selector,
          groq: settings.ai.groq.show_in_selector,
          xai: settings.ai.xai.show_in_selector,
          zai: settings.ai.zai.show_in_selector,
          zai_anthropic: settings.ai.zai_anthropic.show_in_selector,
        });
      } catch (e) {
        logger.warn("Failed to handle settings update:", e);
      }
    };

    window.addEventListener("settings-updated", handleSettingsUpdated);
    return () => {
      window.removeEventListener("settings-updated", handleSettingsUpdated);
    };
  }, []);

  const handleModelSelect = async (
    modelId: string,
    modelProvider:
      | "vertex"
      | "openrouter"
      | "openai"
      | "anthropic"
      | "ollama"
      | "gemini"
      | "groq"
      | "xai"
      | "zai"
      | "zai_anthropic",
    reasoningEffort?: ReasoningEffort
  ) => {
    // Don't switch if already on this model (and same reasoning effort for OpenAI)
    const providerMap = {
      vertex: "anthropic_vertex",
      openrouter: "openrouter",
      openai: "openai",
      anthropic: "anthropic",
      ollama: "ollama",
      gemini: "gemini",
      groq: "groq",
      xai: "xai",
      zai: "zai",
      zai_anthropic: "zai_anthropic",
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
      } else if (modelProvider === "openrouter") {
        // OpenRouter model switch
        const apiKey = openRouterApiKey ?? (await getOpenRouterApiKey());
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
        const apiKey = openAiApiKey ?? (await getOpenAiApiKey());
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
        const apiKey = anthropicApiKey;
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
        const apiKey = geminiApiKey;
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
        const apiKey = groqApiKey;
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
        const apiKey = xaiApiKey;
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
      } else if (modelProvider === "zai") {
        // Z.AI model switch
        const apiKey = zaiApiKey;
        if (!apiKey) {
          throw new Error("Z.AI API key not configured");
        }
        config = {
          provider: "zai",
          workspace,
          model: modelId,
          api_key: apiKey,
          use_coding_endpoint: zaiUseCodingEndpoint,
        };
        await initAiSession(sessionId, config);
        setSessionAiConfig(sessionId, { status: "ready", provider: "zai" });
      } else if (modelProvider === "zai_anthropic") {
        // Z.AI (Anthropic) model switch
        const apiKey = zaiAnthropicApiKey;
        if (!apiKey) {
          throw new Error("Z.AI (Anthropic) API key not configured");
        }
        config = {
          provider: "zai_anthropic",
          workspace,
          model: modelId,
          api_key: apiKey,
        };
        await initAiSession(sessionId, config);
        setSessionAiConfig(sessionId, { status: "ready", provider: "zai_anthropic" });
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
  const showVertexAi = providerVisibility.vertex_ai && vertexAiEnabled;
  const showOpenRouter = providerVisibility.openrouter && openRouterEnabled;
  const showOpenAi = providerVisibility.openai && openAiEnabled;
  const showAnthropic = providerVisibility.anthropic && anthropicEnabled;
  const showOllama = providerVisibility.ollama && ollamaEnabled;
  const showGemini = providerVisibility.gemini && geminiEnabled;
  const showGroq = providerVisibility.groq && groqEnabled;
  const showXai = providerVisibility.xai && xaiEnabled;
  const showZai = providerVisibility.zai && zaiEnabled;
  const showZaiAnthropic = providerVisibility.zai_anthropic && zaiAnthropicEnabled;
  const hasVisibleProviders =
    showVertexAi ||
    showOpenRouter ||
    showOpenAi ||
    showAnthropic ||
    showOllama ||
    showGemini ||
    showGroq ||
    showXai ||
    showZai ||
    showZaiAnthropic;

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
        <div className="flex items-center rounded-lg bg-muted/50 p-0.5 border border-[var(--color-border-subtle)]/50">
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
        <div className="h-4 w-px bg-[var(--color-border-medium)]" />

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
              className="bg-card border-[var(--color-border-medium)] min-w-[200px]"
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
                          ? "text-accent bg-[var(--color-accent-dim)]"
                          : "text-foreground hover:text-accent"
                      )}
                    >
                      {m.name}
                    </DropdownMenuItem>
                  ))}
                </>
              )}

              {/* OpenRouter Models */}
              {showOpenRouter && (
                <>
                  {showVertexAi && <DropdownMenuSeparator />}
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
                          ? "text-accent bg-[var(--color-accent-dim)]"
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
                                ? "text-accent bg-[var(--color-accent-dim)]"
                                : "text-foreground hover:text-accent"
                            )}
                          >
                            {entry.name}
                          </DropdownMenuSubTrigger>
                          <DropdownMenuSubContent className="bg-card border-[var(--color-border-medium)]">
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
                            ? "text-accent bg-[var(--color-accent-dim)]"
                            : "text-foreground hover:text-accent"
                        )}
                      >
                        {entry.name}
                      </DropdownMenuItem>
                    );
                  };

                  return (
                    <>
                      {(showVertexAi || showOpenRouter) && <DropdownMenuSeparator />}
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
                  {(showVertexAi || showOpenRouter || showOpenAi) && <DropdownMenuSeparator />}
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
                          ? "text-accent bg-[var(--color-accent-dim)]"
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
                  {(showVertexAi || showOpenRouter || showOpenAi || showAnthropic) && (
                    <DropdownMenuSeparator />
                  )}
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
                          ? "text-accent bg-[var(--color-accent-dim)]"
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
                          ? "text-accent bg-[var(--color-accent-dim)]"
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
                          ? "text-accent bg-[var(--color-accent-dim)]"
                          : "text-foreground hover:text-accent"
                      )}
                    >
                      {m.name}
                    </DropdownMenuItem>
                  ))}
                </>
              )}

              {/* Z.AI Models */}
              {showZai && (
                <>
                  {(showVertexAi ||
                    showOpenRouter ||
                    showOpenAi ||
                    showAnthropic ||
                    showOllama ||
                    showGemini ||
                    showGroq ||
                    showXai) && <DropdownMenuSeparator />}
                  <div className="px-2 py-1 text-[10px] text-muted-foreground uppercase tracking-wide">
                    Z.AI (GLM)
                  </div>
                  {(getProviderGroup("zai")?.models ?? []).map((m) => (
                    <DropdownMenuItem
                      key={m.id}
                      onClick={() => handleModelSelect(m.id, "zai")}
                      className={cn(
                        "text-xs cursor-pointer",
                        model === m.id && provider === "zai"
                          ? "text-accent bg-[var(--color-accent-dim)]"
                          : "text-foreground hover:text-accent"
                      )}
                    >
                      {m.name}
                    </DropdownMenuItem>
                  ))}
                </>
              )}

              {/* Z.AI (Anthropic) Models */}
              {showZaiAnthropic && (
                <>
                  {(showVertexAi ||
                    showOpenRouter ||
                    showOpenAi ||
                    showAnthropic ||
                    showOllama ||
                    showGemini ||
                    showGroq ||
                    showXai ||
                    showZai) && <DropdownMenuSeparator />}
                  <div className="px-2 py-1 text-[10px] text-muted-foreground uppercase tracking-wide">
                    Z.AI (Anthropic)
                  </div>
                  {(getProviderGroup("zai_anthropic")?.models ?? []).map((m) => (
                    <DropdownMenuItem
                      key={m.id}
                      onClick={() => handleModelSelect(m.id, "zai_anthropic")}
                      className={cn(
                        "text-xs cursor-pointer",
                        model === m.id && provider === "zai_anthropic"
                          ? "text-accent bg-[var(--color-accent-dim)]"
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
      </div>

      {/* Right side */}
      <div className="flex items-center gap-2">
        {/* Status messages */}
        {isMockBrowserMode() ? (
          <span className="text-[var(--color-ansi-yellow)] text-[11px] truncate max-w-[200px]">
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
              className="w-auto min-w-[200px] p-3 bg-card/95 backdrop-blur-sm border-[var(--color-border-medium)] shadow-lg"
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
                <div className="border-t border-[var(--color-border-subtle)] my-1.5" />
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
            className="h-6 px-2 gap-1.5 text-xs font-medium rounded-lg flex items-center text-muted-foreground/70 border border-[var(--color-border-subtle)]/60 bg-card/30"
          >
            <Gauge className="w-3.5 h-3.5" />
            <span>0%</span>
          </button>
        )}

        {/* Task Plan indicator */}
        {plan && plan.steps.length > 0 && onOpenTaskPlanner && (
          <Button
            variant="ghost"
            size="sm"
            onClick={onOpenTaskPlanner}
            className="h-5 px-1.5 gap-1 text-[11px] font-medium rounded-md bg-[#7aa2f7]/10 text-[#7aa2f7] hover:bg-[#7aa2f7]/20 border border-[#7aa2f7]/20 hover:border-[#7aa2f7]/30 transition-all duration-200"
          >
            <ListTodo className="w-3 h-3" />
            <span>
              {plan.summary.completed}/{plan.summary.total}
            </span>
          </Button>
        )}
      </div>
    </div>
  );
}
