import { useState } from "react";
import { Input } from "@/components/ui/input";
import { Switch } from "@/components/ui/switch";
import {
  GEMINI_MODELS,
  GROQ_MODELS,
  getOpenRouterApiKey,
  initAiAgent,
  initVertexAiAgent,
  OPENAI_MODELS,
  VERTEX_AI_MODELS,
  XAI_MODELS,
} from "@/lib/ai";
import { notify } from "@/lib/notify";
import type {
  AiSettings as AiSettingsType,
  ApiKeysSettings,
  SidecarSettings,
  SynthesisBackendType,
} from "@/lib/settings";
import { useAiConfig, useStore } from "../../store";

// Vertex AI models (matching StatusBar)
const VERTEX_AI_MODELS_LIST = [
  { id: VERTEX_AI_MODELS.CLAUDE_OPUS_4_5, name: "Claude Opus 4.5" },
  { id: VERTEX_AI_MODELS.CLAUDE_SONNET_4_5, name: "Claude Sonnet 4.5" },
  { id: VERTEX_AI_MODELS.CLAUDE_HAIKU_4_5, name: "Claude Haiku 4.5" },
];

// OpenRouter models (fixed list matching StatusBar)
const OPENROUTER_MODELS = [
  { id: "mistralai/devstral-2512", name: "Devstral 2512" },
  { id: "deepseek/deepseek-v3.2", name: "Deepseek v3.2" },
  { id: "z-ai/glm-4.6", name: "GLM 4.6" },
  { id: "x-ai/grok-code-fast-1", name: "Grok Code Fast 1" },
  { id: "openai/gpt-oss-20b", name: "GPT OSS 20b" },
  { id: "openai/gpt-oss-120b", name: "GPT OSS 120b" },
  { id: "openai/gpt-5.2", name: "GPT 5.2" },
];

// OpenAI models (matching StatusBar)
const OPENAI_MODELS_LIST = [{ id: OPENAI_MODELS.GPT_5_2, name: "GPT 5.2" }];

// Anthropic direct API models
const ANTHROPIC_MODELS = [
  { id: "claude-opus-4-5-20251101", name: "Claude Opus 4.5" },
  { id: "claude-sonnet-4-5-20250514", name: "Claude Sonnet 4.5" },
  { id: "claude-haiku-4-5-20250514", name: "Claude Haiku 4.5" },
];

// Gemini models
const GEMINI_MODELS_LIST = [
  { id: GEMINI_MODELS.GEMINI_3_PRO, name: "Gemini 3 Pro" },
  { id: GEMINI_MODELS.GEMINI_2_5_FLASH, name: "Gemini 2.5 Flash" },
  { id: GEMINI_MODELS.GEMINI_2_5_PRO, name: "Gemini 2.5 Pro" },
  { id: GEMINI_MODELS.GEMINI_2_5_FLASH_LITE, name: "Gemini 2.5 Flash Lite" },
  { id: GEMINI_MODELS.GEMINI_2_0_FLASH, name: "Gemini 2.0 Flash" },
];

// Groq models
const GROQ_MODELS_LIST = [
  { id: GROQ_MODELS.LLAMA_4_SCOUT, name: "Llama 4 Scout 17B" },
  { id: GROQ_MODELS.LLAMA_4_MAVERICK, name: "Llama 4 Maverick 17B" },
  { id: GROQ_MODELS.LLAMA_3_3_70B, name: "Llama 3.3 70B" },
  { id: GROQ_MODELS.LLAMA_3_1_8B, name: "Llama 3.1 8B Instant" },
  { id: GROQ_MODELS.QWEN_QWQ_32B, name: "Qwen QWQ 32B" },
];

// xAI models
const XAI_MODELS_LIST = [
  { id: XAI_MODELS.GROK_4_1_FAST_REASONING, name: "Grok 4.1 Fast (Reasoning)" },
  { id: XAI_MODELS.GROK_4_1_FAST_NON_REASONING, name: "Grok 4.1 Fast" },
  { id: XAI_MODELS.GROK_CODE_FAST_1, name: "Grok Code Fast" },
  { id: XAI_MODELS.GROK_4_FAST_REASONING, name: "Grok 4 Fast (Reasoning)" },
  { id: XAI_MODELS.GROK_4_FAST_NON_REASONING, name: "Grok 4 Fast" },
];

interface AiSettingsProps {
  settings: AiSettingsType;
  apiKeys: ApiKeysSettings;
  sidecarSettings: SidecarSettings;
  onChange: (settings: AiSettingsType) => void;
  onApiKeysChange: (keys: ApiKeysSettings) => void;
  onSidecarChange: (settings: SidecarSettings) => void;
}

// Simple Select component using native select for now
function SimpleSelect({
  id,
  value,
  onValueChange,
  options,
}: {
  id?: string;
  value: string;
  onValueChange: (value: string) => void;
  options: { value: string; label: string }[];
}) {
  return (
    <select
      id={id}
      value={value}
      onChange={(e) => onValueChange(e.target.value)}
      className="w-full h-9 rounded-md border border-[var(--border-medium)] bg-muted px-3 py-1 text-sm text-foreground focus:outline-none focus:ring-1 focus:ring-accent cursor-pointer appearance-none"
      style={{
        backgroundImage:
          "url(\"data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='12' height='12' viewBox='0 0 24 24' fill='none' stroke='%239aa0a6' stroke-width='2'%3E%3Cpath d='m6 9 6 6 6-6'/%3E%3C/svg%3E\")",
        backgroundRepeat: "no-repeat",
        backgroundPosition: "right 12px center",
      }}
    >
      {options.map((opt) => (
        <option key={opt.value} value={opt.value} className="bg-card">
          {opt.label}
        </option>
      ))}
    </select>
  );
}

export function AiSettings({
  settings,
  apiKeys,
  sidecarSettings,
  onChange,
  onApiKeysChange,
  onSidecarChange,
}: AiSettingsProps) {
  const [synthesisStatus, setSynthesisStatus] = useState<string>("");
  // Note: synthesis backend selection is now just a preference stored in settings
  // The actual backend implementation was removed in the sidecar simplification
  const [isChangingBackend, setIsChangingBackend] = useState(false);

  // Access current AI config for auto-switching when provider is disabled
  const aiConfig = useAiConfig();
  const setAiConfig = useStore((state) => state.setAiConfig);

  /**
   * Switch to an alternative provider when the current one is disabled.
   * @param targetProvider - The provider to switch to
   * @param newSettings - The updated settings with the disabled provider
   */
  const switchToAlternativeProvider = async (
    targetProvider: "vertex" | "openrouter",
    newSettings: AiSettingsType
  ) => {
    try {
      if (targetProvider === "openrouter") {
        // Check if OpenRouter is available
        if (!newSettings.openrouter.show_in_selector) {
          notify.warning("No active providers available");
          return;
        }
        const apiKey = newSettings.openrouter.api_key ?? (await getOpenRouterApiKey());
        if (!apiKey) {
          notify.warning("OpenRouter API key not configured");
          return;
        }

        const firstModel = OPENROUTER_MODELS[0];
        setAiConfig({ status: "initializing", model: firstModel.id });
        const workspace = aiConfig.vertexConfig?.workspace ?? ".";
        await initAiAgent({
          workspace,
          provider: "openrouter",
          model: firstModel.id,
          apiKey,
        });
        setAiConfig({ status: "ready", provider: "openrouter" });
        notify.success(`Switched to ${firstModel.name}`);
      } else {
        // Switch to Vertex AI
        if (!newSettings.vertex_ai.show_in_selector) {
          notify.warning("No active providers available");
          return;
        }
        if (!aiConfig.vertexConfig) {
          notify.warning("Vertex AI not configured");
          return;
        }

        const firstModel = VERTEX_AI_MODELS.CLAUDE_OPUS_4_5;
        setAiConfig({ status: "initializing", model: firstModel });
        await initVertexAiAgent({
          workspace: aiConfig.vertexConfig.workspace,
          credentialsPath: aiConfig.vertexConfig.credentialsPath,
          projectId: aiConfig.vertexConfig.projectId,
          location: aiConfig.vertexConfig.location,
          model: firstModel,
        });
        setAiConfig({ status: "ready", provider: "anthropic_vertex" });
        notify.success("Switched to Claude Opus 4.5");
      }
    } catch (error) {
      console.error("Failed to switch provider:", error);
      setAiConfig({
        status: "error",
        errorMessage: error instanceof Error ? error.message : "Failed to switch provider",
      });
      notify.error("Failed to switch to alternative provider");
    }
  };

  const updateField = <K extends keyof AiSettingsType>(key: K, value: AiSettingsType[K]) => {
    onChange({ ...settings, [key]: value });
  };

  const updateSidecar = <K extends keyof SidecarSettings>(key: K, value: SidecarSettings[K]) => {
    onSidecarChange({ ...sidecarSettings, [key]: value });
  };

  const handleSynthesisBackendChange = (value: string) => {
    setIsChangingBackend(true);
    setSynthesisStatus("");

    // Just update the preference - actual synthesis implementation was simplified
    updateSidecar("synthesis_backend", value as SynthesisBackendType);

    const backendNames: Record<string, string> = {
      local: "Local LLM",
      vertex_anthropic: "Vertex AI (Anthropic)",
      openai: "OpenAI",
      grok: "Grok",
      template: "Template-based",
    };
    setSynthesisStatus(`✓ Set to ${backendNames[value] || value}`);
    setIsChangingBackend(false);
  };

  const updateVertexAi = async (field: string, value: string | boolean | null) => {
    const newVertexSettings = {
      ...settings.vertex_ai,
      [field]: typeof value === "boolean" ? value : value || null,
    };
    const newSettings = {
      ...settings,
      vertex_ai: newVertexSettings,
    };
    onChange(newSettings);

    // If we just disabled show_in_selector and this is the current provider, switch
    if (
      field === "show_in_selector" &&
      value === false &&
      aiConfig.provider === "anthropic_vertex"
    ) {
      await switchToAlternativeProvider("openrouter", newSettings);
    }
  };

  const updateOpenRouter = async (field: string, value: string | boolean | null) => {
    const newOpenRouterSettings = {
      ...settings.openrouter,
      [field]: typeof value === "boolean" ? value : value || null,
    };
    const newSettings = {
      ...settings,
      openrouter: newOpenRouterSettings,
    };
    onChange(newSettings);

    // If we just disabled show_in_selector and this is the current provider, switch
    if (field === "show_in_selector" && value === false && aiConfig.provider === "openrouter") {
      await switchToAlternativeProvider("vertex", newSettings);
    }
  };

  const updateOpenAi = async (field: string, value: string | boolean | null) => {
    const newOpenAiSettings = {
      ...settings.openai,
      [field]: typeof value === "boolean" ? value : value || null,
    };
    const newSettings = {
      ...settings,
      openai: newOpenAiSettings,
    };
    onChange(newSettings);

    // If we just disabled show_in_selector and this is the current provider, switch
    if (field === "show_in_selector" && value === false && aiConfig.provider === "openai") {
      await switchToAlternativeProvider("vertex", newSettings);
    }
  };

  const updateAnthropic = async (field: string, value: string | boolean | null) => {
    const newAnthropicSettings = {
      ...settings.anthropic,
      [field]: typeof value === "boolean" ? value : value || null,
    };
    const newSettings = {
      ...settings,
      anthropic: newAnthropicSettings,
    };
    onChange(newSettings);

    // If we just disabled show_in_selector and this is the current provider, switch
    if (field === "show_in_selector" && value === false && aiConfig.provider === "anthropic") {
      await switchToAlternativeProvider("vertex", newSettings);
    }
  };

  const updateOllama = async (field: string, value: string | boolean | null) => {
    const newOllamaSettings = {
      ...settings.ollama,
      [field]: typeof value === "boolean" ? value : value || null,
    };
    const newSettings = {
      ...settings,
      ollama: newOllamaSettings,
    };
    onChange(newSettings);

    // If we just disabled show_in_selector and this is the current provider, switch
    if (field === "show_in_selector" && value === false && aiConfig.provider === "ollama") {
      await switchToAlternativeProvider("vertex", newSettings);
    }
  };

  const updateGemini = async (field: string, value: string | boolean | null) => {
    const newGeminiSettings = {
      ...settings.gemini,
      [field]: typeof value === "boolean" ? value : value || null,
    };
    const newSettings = {
      ...settings,
      gemini: newGeminiSettings,
    };
    onChange(newSettings);

    // If we just disabled show_in_selector and this is the current provider, switch
    if (field === "show_in_selector" && value === false && aiConfig.provider === "gemini") {
      await switchToAlternativeProvider("vertex", newSettings);
    }
  };

  const updateGroq = async (field: string, value: string | boolean | null) => {
    const newGroqSettings = {
      ...settings.groq,
      [field]: typeof value === "boolean" ? value : value || null,
    };
    const newSettings = {
      ...settings,
      groq: newGroqSettings,
    };
    onChange(newSettings);

    // If we just disabled show_in_selector and this is the current provider, switch
    if (field === "show_in_selector" && value === false && aiConfig.provider === "groq") {
      await switchToAlternativeProvider("vertex", newSettings);
    }
  };

  const updateXai = async (field: string, value: string | boolean | null) => {
    const newXaiSettings = {
      ...settings.xai,
      [field]: typeof value === "boolean" ? value : value || null,
    };
    const newSettings = {
      ...settings,
      xai: newXaiSettings,
    };
    onChange(newSettings);

    // If we just disabled show_in_selector and this is the current provider, switch
    if (field === "show_in_selector" && value === false && aiConfig.provider === "xai") {
      await switchToAlternativeProvider("vertex", newSettings);
    }
  };

  const providerOptions = [
    { value: "vertex_ai", label: "Vertex AI (Anthropic)" },
    { value: "openrouter", label: "OpenRouter" },
    { value: "anthropic", label: "Anthropic" },
    { value: "openai", label: "OpenAI" },
    { value: "ollama", label: "Ollama (Local)" },
    { value: "gemini", label: "Google Gemini" },
    { value: "groq", label: "Groq" },
    { value: "xai", label: "xAI (Grok)" },
  ];

  return (
    <div className="space-y-6">
      {/* Default Provider */}
      <div className="space-y-2">
        <label htmlFor="ai-default-provider" className="text-sm font-medium text-foreground">
          Default Provider
        </label>
        <SimpleSelect
          id="ai-default-provider"
          value={settings.default_provider}
          onValueChange={(value) =>
            updateField("default_provider", value as AiSettingsType["default_provider"])
          }
          options={providerOptions}
        />
        <p className="text-xs text-muted-foreground">The AI provider to use for conversations</p>
      </div>

      {/* Default Model */}
      <div className="space-y-2">
        <label htmlFor="ai-default-model" className="text-sm font-medium text-foreground">
          Default Model
        </label>
        {settings.default_provider === "vertex_ai" ? (
          <SimpleSelect
            id="ai-default-model"
            value={settings.default_model}
            onValueChange={(value) => updateField("default_model", value)}
            options={VERTEX_AI_MODELS_LIST.map((m) => ({ value: m.id, label: m.name }))}
          />
        ) : settings.default_provider === "openrouter" ? (
          <SimpleSelect
            id="ai-default-model"
            value={settings.default_model}
            onValueChange={(value) => updateField("default_model", value)}
            options={OPENROUTER_MODELS.map((m) => ({ value: m.id, label: m.name }))}
          />
        ) : settings.default_provider === "anthropic" ? (
          <SimpleSelect
            id="ai-default-model"
            value={settings.default_model}
            onValueChange={(value) => updateField("default_model", value)}
            options={ANTHROPIC_MODELS.map((m) => ({ value: m.id, label: m.name }))}
          />
        ) : settings.default_provider === "openai" ? (
          <SimpleSelect
            id="ai-default-model"
            value={settings.default_model}
            onValueChange={(value) => updateField("default_model", value)}
            options={OPENAI_MODELS_LIST.map((m) => ({ value: m.id, label: m.name }))}
          />
        ) : settings.default_provider === "gemini" ? (
          <SimpleSelect
            id="ai-default-model"
            value={settings.default_model}
            onValueChange={(value) => updateField("default_model", value)}
            options={GEMINI_MODELS_LIST.map((m) => ({ value: m.id, label: m.name }))}
          />
        ) : settings.default_provider === "groq" ? (
          <SimpleSelect
            id="ai-default-model"
            value={settings.default_model}
            onValueChange={(value) => updateField("default_model", value)}
            options={GROQ_MODELS_LIST.map((m) => ({ value: m.id, label: m.name }))}
          />
        ) : settings.default_provider === "xai" ? (
          <SimpleSelect
            id="ai-default-model"
            value={settings.default_model}
            onValueChange={(value) => updateField("default_model", value)}
            options={XAI_MODELS_LIST.map((m) => ({ value: m.id, label: m.name }))}
          />
        ) : (
          <Input
            id="ai-default-model"
            value={settings.default_model}
            onChange={(e) => updateField("default_model", e.target.value)}
            placeholder={settings.default_provider === "ollama" ? "llama3.2:latest" : "model-name"}
            className="bg-card border-border text-foreground"
          />
        )}
        <p className="text-xs text-muted-foreground">
          {settings.default_provider === "ollama"
            ? "Enter the name of your local Ollama model"
            : "Select from available models for the selected provider"}
        </p>
      </div>

      {/* Vertex AI Settings */}
      {settings.default_provider === "vertex_ai" && (
        <div className="space-y-4 p-4 rounded-lg bg-muted border border-[var(--border-medium)]">
          <div className="flex items-center justify-between">
            <h4 className="text-sm font-medium text-accent">Vertex AI Configuration</h4>
            <div className="flex items-center gap-2">
              <label htmlFor="vertex-show-in-selector" className="text-xs text-muted-foreground">
                Show in model selector
              </label>
              <Switch
                id="vertex-show-in-selector"
                checked={settings.vertex_ai.show_in_selector}
                onCheckedChange={(checked) => updateVertexAi("show_in_selector", checked)}
              />
            </div>
          </div>

          <div className="space-y-2">
            <label htmlFor="vertex-credentials-path" className="text-sm text-foreground">
              Credentials Path
            </label>
            <Input
              id="vertex-credentials-path"
              value={settings.vertex_ai.credentials_path || ""}
              onChange={(e) => updateVertexAi("credentials_path", e.target.value)}
              placeholder="/path/to/service-account.json"
              className="bg-background border-border text-foreground"
            />
          </div>

          <div className="space-y-2">
            <label htmlFor="vertex-project-id" className="text-sm text-foreground">
              Project ID
            </label>
            <Input
              id="vertex-project-id"
              value={settings.vertex_ai.project_id || ""}
              onChange={(e) => updateVertexAi("project_id", e.target.value)}
              placeholder="your-project-id"
              className="bg-background border-border text-foreground"
            />
          </div>

          <div className="space-y-2">
            <label htmlFor="vertex-location" className="text-sm text-foreground">
              Location
            </label>
            <Input
              id="vertex-location"
              value={settings.vertex_ai.location || ""}
              onChange={(e) => updateVertexAi("location", e.target.value)}
              placeholder="us-east5"
              className="bg-background border-border text-foreground"
            />
          </div>
        </div>
      )}

      {/* OpenRouter Settings */}
      {settings.default_provider === "openrouter" && (
        <div className="space-y-4 p-4 rounded-lg bg-muted border border-[var(--border-medium)]">
          <div className="flex items-center justify-between">
            <h4 className="text-sm font-medium text-accent">OpenRouter Configuration</h4>
            <div className="flex items-center gap-2">
              <label
                htmlFor="openrouter-show-in-selector"
                className="text-xs text-muted-foreground"
              >
                Show in model selector
              </label>
              <Switch
                id="openrouter-show-in-selector"
                checked={settings.openrouter.show_in_selector}
                onCheckedChange={(checked) => updateOpenRouter("show_in_selector", checked)}
              />
            </div>
          </div>

          <div className="space-y-2">
            <label htmlFor="openrouter-api-key" className="text-sm text-foreground">
              API Key
            </label>
            <Input
              id="openrouter-api-key"
              type="password"
              value={settings.openrouter.api_key || ""}
              onChange={(e) => updateOpenRouter("api_key", e.target.value)}
              placeholder="sk-or-..."
              className="bg-background border-border text-foreground"
            />
            <p className="text-xs text-muted-foreground">
              Use $OPENROUTER_API_KEY to reference an environment variable
            </p>
          </div>
        </div>
      )}

      {/* OpenAI Settings */}
      {settings.default_provider === "openai" && (
        <div className="space-y-4 p-4 rounded-lg bg-muted border border-[var(--border-medium)]">
          <div className="flex items-center justify-between">
            <h4 className="text-sm font-medium text-accent">OpenAI Configuration</h4>
            <div className="flex items-center gap-2">
              <label htmlFor="openai-show-in-selector" className="text-xs text-muted-foreground">
                Show in model selector
              </label>
              <Switch
                id="openai-show-in-selector"
                checked={settings.openai.show_in_selector}
                onCheckedChange={(checked) => updateOpenAi("show_in_selector", checked)}
              />
            </div>
          </div>

          <div className="space-y-2">
            <label htmlFor="openai-api-key" className="text-sm text-foreground">
              API Key
            </label>
            <Input
              id="openai-api-key"
              type="password"
              value={settings.openai.api_key || ""}
              onChange={(e) => updateOpenAi("api_key", e.target.value)}
              placeholder="sk-..."
              className="bg-background border-border text-foreground"
            />
            <p className="text-xs text-muted-foreground">
              Use $OPENAI_API_KEY to reference an environment variable
            </p>
          </div>

          <div className="space-y-2">
            <label htmlFor="openai-base-url" className="text-sm text-foreground">
              Base URL (optional)
            </label>
            <Input
              id="openai-base-url"
              value={settings.openai.base_url || ""}
              onChange={(e) => updateOpenAi("base_url", e.target.value)}
              placeholder="https://api.openai.com/v1"
              className="bg-background border-border text-foreground"
            />
            <p className="text-xs text-muted-foreground">
              Custom endpoint for OpenAI-compatible APIs (Azure, local servers, etc.)
            </p>
          </div>
        </div>
      )}

      {/* Anthropic Settings */}
      {settings.default_provider === "anthropic" && (
        <div className="space-y-4 p-4 rounded-lg bg-muted border border-[var(--border-medium)]">
          <div className="flex items-center justify-between">
            <h4 className="text-sm font-medium text-accent">Anthropic Configuration</h4>
            <div className="flex items-center gap-2">
              <label htmlFor="anthropic-show-in-selector" className="text-xs text-muted-foreground">
                Show in model selector
              </label>
              <Switch
                id="anthropic-show-in-selector"
                checked={settings.anthropic.show_in_selector}
                onCheckedChange={(checked) => updateAnthropic("show_in_selector", checked)}
              />
            </div>
          </div>

          <div className="space-y-2">
            <label htmlFor="anthropic-api-key" className="text-sm text-foreground">
              API Key
            </label>
            <Input
              id="anthropic-api-key"
              type="password"
              value={settings.anthropic.api_key || ""}
              onChange={(e) => updateAnthropic("api_key", e.target.value)}
              placeholder="sk-ant-..."
              className="bg-background border-border text-foreground"
            />
            <p className="text-xs text-muted-foreground">
              Use $ANTHROPIC_API_KEY to reference an environment variable
            </p>
          </div>
        </div>
      )}

      {/* Ollama Settings */}
      {settings.default_provider === "ollama" && (
        <div className="space-y-4 p-4 rounded-lg bg-muted border border-[var(--border-medium)]">
          <div className="flex items-center justify-between">
            <h4 className="text-sm font-medium text-accent">Ollama Configuration</h4>
            <div className="flex items-center gap-2">
              <label htmlFor="ollama-show-in-selector" className="text-xs text-muted-foreground">
                Show in model selector
              </label>
              <Switch
                id="ollama-show-in-selector"
                checked={settings.ollama.show_in_selector}
                onCheckedChange={(checked) => updateOllama("show_in_selector", checked)}
              />
            </div>
          </div>

          <div className="space-y-2">
            <label htmlFor="ollama-base-url" className="text-sm text-foreground">
              Base URL
            </label>
            <Input
              id="ollama-base-url"
              value={settings.ollama.base_url}
              onChange={(e) => updateOllama("base_url", e.target.value)}
              placeholder="http://localhost:11434"
              className="bg-background border-border text-foreground"
            />
            <p className="text-xs text-muted-foreground">URL of your local Ollama server</p>
          </div>
        </div>
      )}

      {/* Gemini Settings */}
      {settings.default_provider === "gemini" && (
        <div className="space-y-4 p-4 rounded-lg bg-muted border border-[var(--border-medium)]">
          <div className="flex items-center justify-between">
            <h4 className="text-sm font-medium text-accent">Google Gemini Configuration</h4>
            <div className="flex items-center gap-2">
              <label htmlFor="gemini-show-in-selector" className="text-xs text-muted-foreground">
                Show in model selector
              </label>
              <Switch
                id="gemini-show-in-selector"
                checked={settings.gemini.show_in_selector}
                onCheckedChange={(checked) => updateGemini("show_in_selector", checked)}
              />
            </div>
          </div>

          <div className="space-y-2">
            <label htmlFor="gemini-api-key" className="text-sm text-foreground">
              API Key
            </label>
            <Input
              id="gemini-api-key"
              type="password"
              value={settings.gemini.api_key || ""}
              onChange={(e) => updateGemini("api_key", e.target.value)}
              placeholder="AIza..."
              className="bg-background border-border text-foreground"
            />
            <p className="text-xs text-muted-foreground">
              Use $GEMINI_API_KEY to reference an environment variable
            </p>
          </div>
        </div>
      )}

      {/* Groq Settings */}
      {settings.default_provider === "groq" && (
        <div className="space-y-4 p-4 rounded-lg bg-muted border border-[var(--border-medium)]">
          <div className="flex items-center justify-between">
            <h4 className="text-sm font-medium text-accent">Groq Configuration</h4>
            <div className="flex items-center gap-2">
              <label htmlFor="groq-show-in-selector" className="text-xs text-muted-foreground">
                Show in model selector
              </label>
              <Switch
                id="groq-show-in-selector"
                checked={settings.groq.show_in_selector}
                onCheckedChange={(checked) => updateGroq("show_in_selector", checked)}
              />
            </div>
          </div>

          <div className="space-y-2">
            <label htmlFor="groq-api-key" className="text-sm text-foreground">
              API Key
            </label>
            <Input
              id="groq-api-key"
              type="password"
              value={settings.groq.api_key || ""}
              onChange={(e) => updateGroq("api_key", e.target.value)}
              placeholder="gsk_..."
              className="bg-background border-border text-foreground"
            />
            <p className="text-xs text-muted-foreground">
              Use $GROQ_API_KEY to reference an environment variable
            </p>
          </div>
        </div>
      )}

      {/* xAI Settings */}
      {settings.default_provider === "xai" && (
        <div className="space-y-4 p-4 rounded-lg bg-muted border border-[var(--border-medium)]">
          <div className="flex items-center justify-between">
            <h4 className="text-sm font-medium text-accent">xAI (Grok) Configuration</h4>
            <div className="flex items-center gap-2">
              <label htmlFor="xai-show-in-selector" className="text-xs text-muted-foreground">
                Show in model selector
              </label>
              <Switch
                id="xai-show-in-selector"
                checked={settings.xai.show_in_selector}
                onCheckedChange={(checked) => updateXai("show_in_selector", checked)}
              />
            </div>
          </div>

          <div className="space-y-2">
            <label htmlFor="xai-api-key" className="text-sm text-foreground">
              API Key
            </label>
            <Input
              id="xai-api-key"
              type="password"
              value={settings.xai.api_key || ""}
              onChange={(e) => updateXai("api_key", e.target.value)}
              placeholder="xai-..."
              className="bg-background border-border text-foreground"
            />
            <p className="text-xs text-muted-foreground">
              Use $XAI_API_KEY to reference an environment variable
            </p>
          </div>
        </div>
      )}

      {/* API Keys */}
      <div className="space-y-4 p-4 rounded-lg bg-muted border border-[var(--border-medium)]">
        <h4 className="text-sm font-medium text-accent">API Keys</h4>

        <div className="space-y-2">
          <label htmlFor="api-key-tavily" className="text-sm text-foreground">
            Tavily (Web Search)
          </label>
          <Input
            id="api-key-tavily"
            type="password"
            value={apiKeys.tavily || ""}
            onChange={(e) => onApiKeysChange({ ...apiKeys, tavily: e.target.value || null })}
            placeholder="tvly-..."
            className="bg-background border-border text-foreground"
          />
          <p className="text-xs text-muted-foreground">
            Use $TAVILY_API_KEY to reference an environment variable
          </p>
        </div>
      </div>

      {/* Synthesis Backend (Sidecar) */}
      <div className="space-y-4 p-4 rounded-lg bg-muted border border-[var(--border-medium)]">
        <h4 className="text-sm font-medium text-accent">Commit Synthesis Backend</h4>
        <p className="text-xs text-muted-foreground">
          Choose the AI backend for generating commit messages and session summaries
        </p>

        <div className="space-y-2">
          <label htmlFor="synthesis-backend" className="text-sm text-foreground">
            Backend
          </label>
          <SimpleSelect
            id="synthesis-backend"
            value={sidecarSettings.synthesis_backend}
            onValueChange={handleSynthesisBackendChange}
            options={[
              { value: "local", label: "Local (Qwen via mistral.rs)" },
              { value: "vertex_anthropic", label: "Vertex AI (Claude)" },
              { value: "openai", label: "OpenAI" },
              { value: "grok", label: "xAI Grok" },
              { value: "template", label: "Template Only (No LLM)" },
            ]}
          />
          {isChangingBackend && <p className="text-xs text-accent">Switching backend...</p>}
          {synthesisStatus && (
            <p
              className={`text-xs ${synthesisStatus.startsWith("✓") ? "text-[var(--success)]" : "text-destructive"}`}
            >
              {synthesisStatus}
            </p>
          )}
        </div>

        {sidecarSettings.synthesis_backend === "local" && (
          <div className="text-xs text-muted-foreground space-y-1">
            <p>• Uses Qwen 2.5 0.5B model for on-device inference</p>
            <p>• Slower but works offline</p>
            <p>• Model downloads automatically on first use (~350MB)</p>
          </div>
        )}

        {sidecarSettings.synthesis_backend === "vertex_anthropic" && (
          <div className="space-y-3">
            <div className="text-xs text-muted-foreground space-y-1">
              <p>• Uses Claude via your Vertex AI configuration</p>
              <p>• Fast and high quality</p>
              <p>• Requires active Vertex AI credentials</p>
            </div>

            <div className="space-y-2">
              <label htmlFor="synthesis-vertex-model" className="text-sm text-foreground">
                Model
              </label>
              <SimpleSelect
                id="synthesis-vertex-model"
                value={sidecarSettings.synthesis_vertex.model}
                onValueChange={(value) =>
                  onSidecarChange({
                    ...sidecarSettings,
                    synthesis_vertex: {
                      ...sidecarSettings.synthesis_vertex,
                      model: value,
                    },
                  })
                }
                options={[
                  {
                    value: "claude-opus-4-5-20251101",
                    label: "Claude Opus 4.5 (Most Capable)",
                  },
                  {
                    value: "claude-sonnet-4-5-20250514",
                    label: "Claude Sonnet 4.5",
                  },
                  {
                    value: "claude-haiku-4-5-20250514",
                    label: "Claude Haiku 4.5 (Fastest)",
                  },
                ]}
              />
            </div>

            {/* Optional: Override credentials for synthesis */}
            <details className="text-xs">
              <summary className="text-muted-foreground cursor-pointer hover:text-foreground">
                Override Vertex AI credentials (optional)
              </summary>
              <div className="mt-2 space-y-2 pl-2 border-l border-border">
                <p className="text-muted-foreground">
                  By default, synthesis uses your main Vertex AI configuration above.
                </p>
                <Input
                  placeholder="Project ID (leave empty to use main config)"
                  value={sidecarSettings.synthesis_vertex.project_id || ""}
                  onChange={(e) =>
                    onSidecarChange({
                      ...sidecarSettings,
                      synthesis_vertex: {
                        ...sidecarSettings.synthesis_vertex,
                        project_id: e.target.value || null,
                      },
                    })
                  }
                  className="bg-background border-border text-foreground h-8"
                />
                <Input
                  placeholder="Location (leave empty to use main config)"
                  value={sidecarSettings.synthesis_vertex.location || ""}
                  onChange={(e) =>
                    onSidecarChange({
                      ...sidecarSettings,
                      synthesis_vertex: {
                        ...sidecarSettings.synthesis_vertex,
                        location: e.target.value || null,
                      },
                    })
                  }
                  className="bg-background border-border text-foreground h-8"
                />
              </div>
            </details>
          </div>
        )}

        {sidecarSettings.synthesis_backend === "openai" && (
          <div className="space-y-3">
            <div className="text-xs text-muted-foreground space-y-1">
              <p>• Uses OpenAI API</p>
              <p>• Fast and reliable</p>
            </div>

            <div className="space-y-2">
              <label htmlFor="synthesis-openai-model" className="text-sm text-foreground">
                Model
              </label>
              <SimpleSelect
                id="synthesis-openai-model"
                value={sidecarSettings.synthesis_openai.model}
                onValueChange={(value) =>
                  onSidecarChange({
                    ...sidecarSettings,
                    synthesis_openai: {
                      ...sidecarSettings.synthesis_openai,
                      model: value,
                    },
                  })
                }
                options={[
                  { value: "gpt-4o-mini", label: "GPT-4o Mini (Fastest)" },
                  { value: "gpt-4o", label: "GPT-4o" },
                  { value: "gpt-4-turbo", label: "GPT-4 Turbo" },
                ]}
              />
            </div>

            <div className="space-y-2">
              <label htmlFor="synthesis-openai-key" className="text-sm text-foreground">
                API Key
              </label>
              <Input
                id="synthesis-openai-key"
                type="password"
                placeholder="sk-..."
                value={sidecarSettings.synthesis_openai.api_key || ""}
                onChange={(e) =>
                  onSidecarChange({
                    ...sidecarSettings,
                    synthesis_openai: {
                      ...sidecarSettings.synthesis_openai,
                      api_key: e.target.value || null,
                    },
                  })
                }
                className="bg-background border-border text-foreground"
              />
            </div>
          </div>
        )}

        {sidecarSettings.synthesis_backend === "grok" && (
          <div className="space-y-3">
            <div className="text-xs text-muted-foreground space-y-1">
              <p>• Uses xAI Grok API</p>
            </div>

            <div className="space-y-2">
              <label htmlFor="synthesis-grok-model" className="text-sm text-foreground">
                Model
              </label>
              <SimpleSelect
                id="synthesis-grok-model"
                value={sidecarSettings.synthesis_grok.model}
                onValueChange={(value) =>
                  onSidecarChange({
                    ...sidecarSettings,
                    synthesis_grok: {
                      ...sidecarSettings.synthesis_grok,
                      model: value,
                    },
                  })
                }
                options={[
                  { value: "grok-2", label: "Grok 2" },
                  { value: "grok-2-mini", label: "Grok 2 Mini (Faster)" },
                ]}
              />
            </div>

            <div className="space-y-2">
              <label htmlFor="synthesis-grok-key" className="text-sm text-foreground">
                API Key
              </label>
              <Input
                id="synthesis-grok-key"
                type="password"
                placeholder="xai-..."
                value={sidecarSettings.synthesis_grok.api_key || ""}
                onChange={(e) =>
                  onSidecarChange({
                    ...sidecarSettings,
                    synthesis_grok: {
                      ...sidecarSettings.synthesis_grok,
                      api_key: e.target.value || null,
                    },
                  })
                }
                className="bg-background border-border text-foreground"
              />
            </div>
          </div>
        )}

        {sidecarSettings.synthesis_backend === "template" && (
          <div className="text-xs text-muted-foreground space-y-1">
            <p>• Uses simple templates without LLM enhancement</p>
            <p>• Fastest option, works offline</p>
            <p>• Basic commit messages based on file changes</p>
          </div>
        )}
      </div>
    </div>
  );
}
