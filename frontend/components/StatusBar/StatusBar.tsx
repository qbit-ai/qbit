import { Bot, Coins, Cpu, ListTodo, Monitor, Terminal } from "lucide-react";
import { useCallback, useEffect, useState } from "react";
import { AgentModeSelector } from "@/components/AgentModeSelector";
import { NotificationWidget } from "@/components/NotificationWidget";
import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { Popover, PopoverContent, PopoverTrigger } from "@/components/ui/popover";
import {
  getOpenAiApiKey,
  getOpenRouterApiKey,
  initAiSession,
  type ProviderConfig,
  type ReasoningEffort,
} from "@/lib/ai";
import { formatModelName, getProviderGroup } from "@/lib/models";
import { notify } from "@/lib/notify";
import { getSettings } from "@/lib/settings";
import { cn } from "@/lib/utils";
import { isMockBrowserMode } from "@/mocks";
import { useInputMode, useRenderMode, useSessionAiConfig, useStore } from "../../store";

interface StatusBarProps {
  sessionId: string | null;
  onOpenTaskPlanner?: () => void;
}

// Stable default for token usage to avoid infinite re-render loops
const EMPTY_TOKEN_USAGE = { input: 0, output: 0 } as const;

/**
 * Format token count to a compact human-readable string.
 * Examples: 1200 -> "1.2k", 15300 -> "15.3k", 1500000 -> "1.5M"
 */
function formatTokenCount(tokens: number): string {
  if (tokens >= 1_000_000) {
    return `${(tokens / 1_000_000).toFixed(1)}M`;
  }
  if (tokens >= 1_000) {
    return `${(tokens / 1_000).toFixed(1)}k`;
  }
  return tokens.toString();
}

/**
 * Format token count with commas for detailed display.
 * Examples: 1200 -> "1,200", 15300 -> "15,300"
 */
function formatTokenCountDetailed(tokens: number): string {
  return tokens.toLocaleString();
}

export function StatusBar({ sessionId, onOpenTaskPlanner }: StatusBarProps) {
  const aiConfig = useSessionAiConfig(sessionId ?? "");
  const model = aiConfig?.model ?? "";
  const status = aiConfig?.status ?? "disconnected";
  const errorMessage = aiConfig?.errorMessage;
  const provider = aiConfig?.provider ?? "";
  const currentReasoningEffort = aiConfig?.reasoningEffort;
  const inputMode = useInputMode(sessionId ?? "");
  const renderMode = useRenderMode(sessionId ?? "");
  const setInputMode = useStore((state) => state.setInputMode);
  const setSessionAiConfig = useStore((state) => state.setSessionAiConfig);
  const plan = useStore((state) => (sessionId ? state.sessions[sessionId]?.plan : undefined));
  const sessionWorkingDirectory = useStore((state) =>
    sessionId ? state.sessions[sessionId]?.workingDirectory : undefined
  );
  const sessionTokenUsage = useStore((state) =>
    sessionId ? (state.sessionTokenUsage[sessionId] ?? EMPTY_TOKEN_USAGE) : EMPTY_TOKEN_USAGE
  );

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
      });
    } catch (e) {
      console.warn("Failed to get provider settings:", e);
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
        });
      } catch (e) {
        console.warn("Failed to handle settings update:", e);
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
      | "xai",
    reasoningEffort?: ReasoningEffort
  ) => {
    if (!sessionId) return;

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
      }

      notify.success(`Switched to ${modelName}`);
    } catch (error) {
      console.error("Failed to switch model:", error);
      setSessionAiConfig(sessionId, {
        status: "error",
        errorMessage: error instanceof Error ? error.message : "Failed to switch model",
      });
      notify.error(`Failed to switch to ${modelName}`);
    }
  };

  return (
    <div className="py-1.5 bg-card/80 backdrop-blur-sm border-t border-[var(--border-subtle)] flex items-center justify-between px-3 text-xs text-muted-foreground relative z-10">
      {/* Left side */}
      <div className="flex items-center gap-2">
        {/* Mode segmented control - icons only */}
        <div className="flex items-center rounded-lg bg-muted/50 p-0.5 border border-[var(--border-subtle)]/50">
          <button
            type="button"
            aria-label="Switch to Terminal mode"
            title="Terminal"
            onClick={() => sessionId && setInputMode(sessionId, "terminal")}
            disabled={!sessionId}
            className={cn(
              "h-5 w-5 flex items-center justify-center rounded-md transition-all duration-200",
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
            onClick={() => sessionId && setInputMode(sessionId, "agent")}
            disabled={!sessionId}
            className={cn(
              "h-5 w-5 flex items-center justify-center rounded-md transition-all duration-200",
              inputMode === "agent"
                ? "bg-accent/15 text-accent shadow-[0_0_8px_rgba(var(--accent-rgb),0.3)]"
                : "text-muted-foreground hover:text-foreground hover:bg-muted"
            )}
          >
            <Bot className="w-3.5 h-3.5" />
          </button>
        </div>

        {/* Subtle divider */}
        <div className="h-4 w-px bg-[var(--border-subtle)]/50" />

        {/* Model selector badge or Terminal Mode indicator */}
        {inputMode === "terminal" ? (
          <div className="h-6 px-2.5 gap-1.5 text-xs font-medium rounded-lg bg-muted/60 text-muted-foreground flex items-center border border-transparent">
            <Terminal className="w-3.5 h-3.5 text-accent" />
            <span>Terminal</span>
          </div>
        ) : status === "disconnected" ? (
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
        ) : (
          // Check if any providers have visible models
          (() => {
            const showVertexAi = providerVisibility.vertex_ai && vertexAiEnabled;
            const showOpenRouter = providerVisibility.openrouter && openRouterEnabled;
            const showOpenAi = providerVisibility.openai && openAiEnabled;
            const showAnthropic = providerVisibility.anthropic && anthropicEnabled;
            const showOllama = providerVisibility.ollama && ollamaEnabled;
            const showGemini = providerVisibility.gemini && geminiEnabled;
            const showGroq = providerVisibility.groq && groqEnabled;
            const showXai = providerVisibility.xai && xaiEnabled;
            const hasVisibleProviders =
              showVertexAi ||
              showOpenRouter ||
              showOpenAi ||
              showAnthropic ||
              showOllama ||
              showGemini ||
              showGroq ||
              showXai;

            if (!hasVisibleProviders) {
              // All providers are hidden - show message
              return (
                <div className="h-6 px-2.5 gap-1.5 text-xs font-medium rounded-lg bg-muted/60 text-muted-foreground flex items-center border border-transparent">
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
                    className="h-6 px-2.5 gap-1.5 text-xs font-medium rounded-lg bg-accent/10 text-accent hover:bg-accent/20 border border-accent/20 hover:border-accent/30 transition-all duration-200"
                  >
                    <Cpu className="w-3.5 h-3.5" />
                    <span>{formatModelName(model, currentReasoningEffort)}</span>
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

                  {/* OpenRouter Models - show only if visibility is enabled AND API key configured */}
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
                              ? "text-accent bg-[var(--accent-dim)]"
                              : "text-foreground hover:text-accent"
                          )}
                        >
                          {m.name}
                        </DropdownMenuItem>
                      ))}
                    </>
                  )}

                  {/* OpenAI Models - show only if visibility is enabled AND API key configured */}
                  {showOpenAi && (
                    <>
                      {(showVertexAi || showOpenRouter) && <DropdownMenuSeparator />}
                      <div className="px-2 py-1 text-[10px] text-muted-foreground uppercase tracking-wide">
                        OpenAI
                      </div>
                      {(getProviderGroup("openai")?.models ?? []).map((m) => (
                        <DropdownMenuItem
                          key={`${m.id}-${m.reasoningEffort ?? "default"}`}
                          onClick={() => handleModelSelect(m.id, "openai", m.reasoningEffort)}
                          className={cn(
                            "text-xs cursor-pointer",
                            model === m.id &&
                              provider === "openai" &&
                              m.reasoningEffort === currentReasoningEffort
                              ? "text-accent bg-[var(--accent-dim)]"
                              : "text-foreground hover:text-accent"
                          )}
                        >
                          {m.name}
                        </DropdownMenuItem>
                      ))}
                    </>
                  )}

                  {/* Anthropic Models - show only if visibility is enabled AND API key configured */}
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
                              ? "text-accent bg-[var(--accent-dim)]"
                              : "text-foreground hover:text-accent"
                          )}
                        >
                          {m.name}
                        </DropdownMenuItem>
                      ))}
                    </>
                  )}

                  {/* Ollama - show only if visibility is enabled */}
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

                  {/* Gemini Models - show only if visibility is enabled AND API key configured */}
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
                              ? "text-accent bg-[var(--accent-dim)]"
                              : "text-foreground hover:text-accent"
                          )}
                        >
                          {m.name}
                        </DropdownMenuItem>
                      ))}
                    </>
                  )}

                  {/* Groq Models - show only if visibility is enabled AND API key configured */}
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
                              ? "text-accent bg-[var(--accent-dim)]"
                              : "text-foreground hover:text-accent"
                          )}
                        >
                          {m.name}
                        </DropdownMenuItem>
                      ))}
                    </>
                  )}

                  {/* xAI Models - show only if visibility is enabled AND API key configured */}
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

        {/* Agent Mode Selector - show when AI is ready in agent mode */}
        {inputMode === "agent" && status === "ready" && sessionId && (
          <AgentModeSelector sessionId={sessionId} />
        )}

        {/* Token usage indicator - click for breakdown */}
        {(sessionTokenUsage.input > 0 || sessionTokenUsage.output > 0) && (
          <Popover>
            <PopoverTrigger asChild>
              <button
                type="button"
                className="h-6 px-2 gap-1.5 text-xs font-medium rounded-lg bg-[#bb9af7]/10 text-[#bb9af7] hover:bg-[#bb9af7]/20 border border-[#bb9af7]/20 hover:border-[#bb9af7]/30 flex items-center cursor-pointer transition-all duration-200"
              >
                <Coins className="w-3.5 h-3.5" />
                <span>{formatTokenCount(sessionTokenUsage.input + sessionTokenUsage.output)} Tokens</span>
              </button>
            </PopoverTrigger>
            <PopoverContent
              align="start"
              className="w-auto min-w-[180px] p-3 bg-card/95 backdrop-blur-sm border-[var(--border-medium)] shadow-lg"
            >
              <div className="text-xs font-medium text-muted-foreground mb-2">Token Usage</div>
              <div className="font-mono text-xs space-y-1">
                <div className="flex justify-between gap-4">
                  <span className="text-muted-foreground">Input</span>
                  <span className="text-foreground">{formatTokenCountDetailed(sessionTokenUsage.input)}</span>
                </div>
                <div className="flex justify-between gap-4">
                  <span className="text-muted-foreground">Output</span>
                  <span className="text-foreground">{formatTokenCountDetailed(sessionTokenUsage.output)}</span>
                </div>
                <div className="border-t border-[var(--border-subtle)] my-1.5" />
                <div className="flex justify-between gap-4">
                  <span className="text-muted-foreground">Total</span>
                  <span className="text-[#bb9af7] font-medium">
                    {formatTokenCountDetailed(sessionTokenUsage.input + sessionTokenUsage.output)}
                  </span>
                </div>
              </div>
            </PopoverContent>
          </Popover>
        )}
      </div>

      {/* Right side - Status messages and notifications */}
      <div className="flex items-center gap-2">
        {isMockBrowserMode() ? (
          <span className="text-[var(--ansi-yellow)] text-xs truncate max-w-[200px]">
            Browser only mode
          </span>
        ) : (
          status === "error" &&
          errorMessage && (
            <span className="text-destructive text-xs truncate max-w-[200px]">({errorMessage})</span>
          )
        )}
        {/* Task Plan indicator */}
        {plan && plan.steps.length > 0 && onOpenTaskPlanner && (
          <Button
            variant="ghost"
            size="sm"
            onClick={onOpenTaskPlanner}
            className="h-6 px-2 gap-1.5 text-xs font-medium rounded-lg bg-[#7aa2f7]/10 text-[#7aa2f7] hover:bg-[#7aa2f7]/20 border border-[#7aa2f7]/20 hover:border-[#7aa2f7]/30 transition-all duration-200"
          >
            <ListTodo className="w-3.5 h-3.5" />
            <span>
              {plan.summary.completed}/{plan.summary.total}
            </span>
          </Button>
        )}
        {/* Full Terminal mode indicator */}
        {renderMode === "fullterm" && (
          <div className="h-6 px-2 gap-1.5 text-xs font-medium rounded-lg bg-[#9ece6a]/10 text-[#9ece6a] flex items-center border border-[#9ece6a]/20">
            <Monitor className="w-3.5 h-3.5" />
            <span>Full Term</span>
          </div>
        )}
        {/* Subtle divider before notifications */}
        <div className="h-4 w-px bg-[var(--border-subtle)]/50" />
        <NotificationWidget />
      </div>
    </div>
  );
}
