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
import {
  getOpenAiApiKey,
  getOpenRouterApiKey,
  initAiAgent,
  initAiAgentUnified,
  initOpenAiAgent,
  initVertexAiAgent,
  type ReasoningEffort,
} from "@/lib/ai";
import { formatModelName, getProviderGroup } from "@/lib/models";
import { notify } from "@/lib/notify";
import { getSettings } from "@/lib/settings";
import { cn } from "@/lib/utils";
import { isMockBrowserMode } from "@/mocks";
import { useAiConfig, useInputMode, useStore } from "../../store";

interface StatusBarProps {
  sessionId: string | null;
}

export function StatusBar({ sessionId }: StatusBarProps) {
  const aiConfig = useAiConfig();
  const {
    model,
    status,
    errorMessage,
    provider,
    reasoningEffort: currentReasoningEffort,
  } = aiConfig;
  const inputMode = useInputMode(sessionId ?? "");
  const setInputMode = useStore((state) => state.setInputMode);
  const setAiConfig = useStore((state) => state.setAiConfig);

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

  // Listen for settings-updated events to refresh provider visibility and auto-switch if needed
  useEffect(() => {
    const handleSettingsUpdated = async () => {
      try {
        const settings = await getSettings();
        const newOpenRouterApiKey = settings.ai.openrouter.api_key;
        const newOpenRouterEnabled = !!newOpenRouterApiKey;
        const newOpenAiApiKey = settings.ai.openai.api_key;
        const newOpenAiEnabled = !!newOpenAiApiKey;
        const newAnthropicApiKey = settings.ai.anthropic.api_key;
        const newAnthropicEnabled = !!newAnthropicApiKey;
        const newOllamaEnabled = true; // Ollama doesn't require an API key
        const newGeminiApiKey = settings.ai.gemini.api_key;
        const newGeminiEnabled = !!newGeminiApiKey;
        const newGroqApiKey = settings.ai.groq.api_key;
        const newGroqEnabled = !!newGroqApiKey;
        const newXaiApiKey = settings.ai.xai.api_key;
        const newXaiEnabled = !!newXaiApiKey;
        const newVisibility = {
          vertex_ai: settings.ai.vertex_ai.show_in_selector,
          openrouter: settings.ai.openrouter.show_in_selector,
          openai: settings.ai.openai.show_in_selector,
          anthropic: settings.ai.anthropic.show_in_selector,
          ollama: settings.ai.ollama.show_in_selector,
          gemini: settings.ai.gemini.show_in_selector,
          groq: settings.ai.groq.show_in_selector,
          xai: settings.ai.xai.show_in_selector,
        };

        // Update state
        setOpenRouterApiKey(newOpenRouterApiKey);
        setOpenRouterEnabled(newOpenRouterEnabled);
        setOpenAiApiKey(newOpenAiApiKey);
        setOpenAiEnabled(newOpenAiEnabled);
        setAnthropicApiKey(newAnthropicApiKey);
        setAnthropicEnabled(newAnthropicEnabled);
        setOllamaEnabled(newOllamaEnabled);
        setGeminiApiKey(newGeminiApiKey);
        setGeminiEnabled(newGeminiEnabled);
        setGroqApiKey(newGroqApiKey);
        setGroqEnabled(newGroqEnabled);
        setXaiApiKey(newXaiApiKey);
        setXaiEnabled(newXaiEnabled);
        setProviderVisibility(newVisibility);

        // Check if current provider is now disabled and needs auto-switch
        const isCurrentVertexAi = provider === "anthropic_vertex";
        const isCurrentOpenRouter = provider === "openrouter";
        const isCurrentOpenAi = provider === "openai";
        const isCurrentAnthropic = provider === "anthropic";
        const isCurrentOllama = provider === "ollama";
        const isCurrentGemini = provider === "gemini";
        const isCurrentGroq = provider === "groq";
        const isCurrentXai = provider === "xai";
        const vertexDisabled = !newVisibility.vertex_ai;
        const openRouterDisabled = !newVisibility.openrouter || !newOpenRouterEnabled;
        const openAiDisabled = !newVisibility.openai || !newOpenAiEnabled;
        const anthropicDisabled = !newVisibility.anthropic || !newAnthropicEnabled;
        const ollamaDisabled = !newVisibility.ollama;
        const geminiDisabled = !newVisibility.gemini || !newGeminiEnabled;
        const groqDisabled = !newVisibility.groq || !newGroqEnabled;
        const xaiDisabled = !newVisibility.xai || !newXaiEnabled;

        // Find first available provider for auto-switch
        const findAlternativeProvider = () => {
          if (!vertexDisabled && aiConfig.vertexConfig) return "vertex";
          if (!openRouterDisabled && newOpenRouterApiKey) return "openrouter";
          if (!openAiDisabled && newOpenAiApiKey) return "openai";
          if (!anthropicDisabled && newAnthropicApiKey) return "anthropic";
          if (!ollamaDisabled) return "ollama";
          if (!geminiDisabled && newGeminiApiKey) return "gemini";
          if (!groqDisabled && newGroqApiKey) return "groq";
          if (!xaiDisabled && newXaiApiKey) return "xai";
          return null;
        };

        if (
          (isCurrentVertexAi && vertexDisabled) ||
          (isCurrentOpenRouter && openRouterDisabled) ||
          (isCurrentOpenAi && openAiDisabled) ||
          (isCurrentAnthropic && anthropicDisabled) ||
          (isCurrentOllama && ollamaDisabled) ||
          (isCurrentGemini && geminiDisabled) ||
          (isCurrentGroq && groqDisabled) ||
          (isCurrentXai && xaiDisabled)
        ) {
          const alternative = findAlternativeProvider();
          if (alternative === "vertex" && aiConfig.vertexConfig) {
            const vertexGroup = getProviderGroup("vertex_ai");
            const firstModel = vertexGroup?.models[0];
            if (firstModel) {
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
          } else if (alternative === "openrouter" && newOpenRouterApiKey) {
            const openrouterGroup = getProviderGroup("openrouter");
            const firstModel = openrouterGroup?.models[0];
            if (firstModel) {
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
          } else if (alternative === "openai" && newOpenAiApiKey) {
            const openaiGroup = getProviderGroup("openai");
            // Get the medium effort model (index 1) or first available
            const firstModel = openaiGroup?.models[1] ?? openaiGroup?.models[0];
            if (firstModel) {
              setAiConfig({
                status: "initializing",
                model: firstModel.id,
                reasoningEffort: firstModel.reasoningEffort,
              });
              const workspace = aiConfig.vertexConfig?.workspace ?? ".";
              await initOpenAiAgent({
                workspace,
                model: firstModel.id,
                apiKey: newOpenAiApiKey,
                reasoningEffort: firstModel.reasoningEffort,
              });
              setAiConfig({
                status: "ready",
                provider: "openai",
                reasoningEffort: firstModel.reasoningEffort,
              });
              notify.success(`Switched to ${firstModel.name}`);
            }
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
      } else if (modelProvider === "openrouter") {
        // OpenRouter model switch
        const apiKey = openRouterApiKey ?? (await getOpenRouterApiKey());
        if (!apiKey) {
          throw new Error("OpenRouter API key not configured");
        }
        const workspace = aiConfig.vertexConfig?.workspace ?? ".";
        await initAiAgent({
          workspace,
          provider: "openrouter",
          model: modelId,
          apiKey,
        });
        setAiConfig({ status: "ready", provider: "openrouter" });
      } else if (modelProvider === "openai") {
        // OpenAI model switch
        const apiKey = openAiApiKey ?? (await getOpenAiApiKey());
        if (!apiKey) {
          throw new Error("OpenAI API key not configured");
        }
        const workspace = aiConfig.vertexConfig?.workspace ?? ".";
        await initOpenAiAgent({
          workspace,
          model: modelId,
          apiKey,
          reasoningEffort,
        });
        setAiConfig({ status: "ready", provider: "openai", reasoningEffort });
      } else if (modelProvider === "anthropic") {
        // Anthropic direct API model switch
        const apiKey = anthropicApiKey;
        if (!apiKey) {
          throw new Error("Anthropic API key not configured");
        }
        const workspace = aiConfig.vertexConfig?.workspace ?? ".";
        await initAiAgent({
          workspace,
          provider: "anthropic",
          model: modelId,
          apiKey,
        });
        setAiConfig({ status: "ready", provider: "anthropic" });
      } else if (modelProvider === "ollama") {
        // Ollama local model switch
        const workspace = aiConfig.vertexConfig?.workspace ?? ".";
        await initAiAgent({
          workspace,
          provider: "ollama",
          model: modelId,
          apiKey: "", // Ollama doesn't require an API key
        });
        setAiConfig({ status: "ready", provider: "ollama" });
      } else if (modelProvider === "gemini") {
        // Gemini model switch
        const apiKey = geminiApiKey;
        if (!apiKey) {
          throw new Error("Gemini API key not configured");
        }
        const workspace = aiConfig.vertexConfig?.workspace ?? ".";
        await initAiAgentUnified({
          provider: "gemini",
          workspace,
          model: modelId,
          api_key: apiKey,
        });
        setAiConfig({ status: "ready", provider: "gemini" });
      } else if (modelProvider === "groq") {
        // Groq model switch
        const apiKey = groqApiKey;
        if (!apiKey) {
          throw new Error("Groq API key not configured");
        }
        const workspace = aiConfig.vertexConfig?.workspace ?? ".";
        await initAiAgentUnified({
          provider: "groq",
          workspace,
          model: modelId,
          api_key: apiKey,
        });
        setAiConfig({ status: "ready", provider: "groq" });
      } else if (modelProvider === "xai") {
        // xAI model switch
        const apiKey = xaiApiKey;
        if (!apiKey) {
          throw new Error("xAI API key not configured");
        }
        const workspace = aiConfig.vertexConfig?.workspace ?? ".";
        await initAiAgentUnified({
          provider: "xai",
          workspace,
          model: modelId,
          api_key: apiKey,
        });
        setAiConfig({ status: "ready", provider: "xai" });
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
    <div className="py-1.5 bg-card border-t border-[var(--border-subtle)] flex items-center justify-between px-3 text-xs text-muted-foreground relative z-10">
      {/* Left side */}
      <div className="flex items-center gap-3">
        {/* Mode segmented control - icons only */}
        <div className="flex items-center rounded-md bg-muted p-0.5 border border-[var(--border-subtle)]">
          <button
            type="button"
            aria-label="Switch to Terminal mode"
            title="Terminal"
            onClick={() => sessionId && setInputMode(sessionId, "terminal")}
            disabled={!sessionId}
            className={cn(
              "h-5 w-5 flex items-center justify-center rounded transition-all duration-150",
              inputMode === "terminal"
                ? "bg-[var(--bg-hover)] text-accent"
                : "text-muted-foreground hover:text-foreground"
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
              "h-5 w-5 flex items-center justify-center rounded transition-all duration-150",
              inputMode === "agent"
                ? "bg-[var(--bg-hover)] text-accent"
                : "text-muted-foreground hover:text-foreground"
            )}
          >
            <Bot className="w-3.5 h-3.5" />
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
                    <span>{formatModelName(model, currentReasoningEffort)}</span>
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
                      {(getProviderGroup("vertex_ai")?.models ?? []).map((m) => (
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
