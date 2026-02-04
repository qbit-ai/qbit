import { ChevronDown, Eye, EyeOff, Star } from "lucide-react";
import { useEffect, useState } from "react";
import { Collapsible, CollapsibleContent, CollapsibleTrigger } from "@/components/ui/collapsible";
import { Input } from "@/components/ui/input";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Switch } from "@/components/ui/switch";
import { getProviders, type ProviderInfo } from "@/lib/model-registry";
import type { AiSettings, WebSearchContextSize } from "@/lib/settings";
import { cn } from "@/lib/utils";
import { ModelSelector } from "./ModelSelector";
import { logger } from "@/lib/logger";

interface ProviderSettingsProps {
  settings: AiSettings;
  onChange: (settings: AiSettings) => void;
}

type ProviderSettingsKey = keyof Pick<
  AiSettings,
  | "vertex_ai"
  | "vertex_gemini"
  | "openrouter"
  | "anthropic"
  | "openai"
  | "ollama"
  | "gemini"
  | "groq"
  | "xai"
  | "zai_sdk"
>;

interface ProviderConfig {
  id: ProviderSettingsKey;
  name: string;
  icon: string;
  description: string;
  getConfigured: (settings: AiSettings) => boolean;
}

/**
 * Check if a provider is configured based on its credentials.
 * This logic must remain in the frontend since it depends on the settings object.
 */
function isProviderConfigured(id: ProviderSettingsKey, settings: AiSettings): boolean {
  switch (id) {
    case "anthropic":
      return !!settings.anthropic.api_key;
    case "gemini":
      return !!settings.gemini.api_key;
    case "groq":
      return !!settings.groq.api_key;
    case "ollama":
      return !!settings.ollama.base_url;
    case "openai":
      return !!settings.openai.api_key;
    case "openrouter":
      return !!settings.openrouter.api_key;
    case "vertex_ai":
      return !!(settings.vertex_ai.credentials_path || settings.vertex_ai.project_id);
    case "vertex_gemini":
      return !!(settings.vertex_gemini.credentials_path || settings.vertex_gemini.project_id);
    case "xai":
      return !!settings.xai.api_key;
    case "zai_sdk":
      return !!settings.zai_sdk?.api_key;
    default:
      return false;
  }
}

/**
 * Map backend AiProvider to frontend settings key.
 */
function providerToSettingsKey(provider: string): ProviderSettingsKey | null {
  const mapping: Record<string, ProviderSettingsKey> = {
    vertex_ai: "vertex_ai",
    vertex_gemini: "vertex_gemini",
    anthropic: "anthropic",
    openai: "openai",
    openrouter: "openrouter",
    ollama: "ollama",
    gemini: "gemini",
    groq: "groq",
    xai: "xai",
    zai_sdk: "zai_sdk",
  };
  return mapping[provider] ?? null;
}

/**
 * Convert backend ProviderInfo to ProviderConfig with configuration checker.
 */
function toProviderConfig(info: ProviderInfo): ProviderConfig | null {
  const settingsKey = providerToSettingsKey(info.provider);
  if (!settingsKey) return null;

  return {
    id: settingsKey,
    name: info.name,
    icon: info.icon,
    description: info.description,
    getConfigured: (settings: AiSettings) => isProviderConfigured(settingsKey, settings),
  };
}

// Fallback static providers in case backend fetch fails
const FALLBACK_PROVIDERS: ProviderConfig[] = [
  {
    id: "anthropic",
    name: "Anthropic",
    icon: "🔶",
    description: "Direct Claude API access",
    getConfigured: (s) => !!s.anthropic.api_key,
  },
  {
    id: "gemini",
    name: "Gemini",
    icon: "💎",
    description: "Google AI models via API",
    getConfigured: (s) => !!s.gemini.api_key,
  },
  {
    id: "groq",
    name: "Groq",
    icon: "⚡",
    description: "Ultra-fast LLM inference",
    getConfigured: (s) => !!s.groq.api_key,
  },
  {
    id: "ollama",
    name: "Ollama",
    icon: "🦙",
    description: "Run models locally on your machine",
    getConfigured: (s) => !!s.ollama.base_url,
  },
  {
    id: "openai",
    name: "OpenAI",
    icon: "⚪",
    description: "GPT models via OpenAI API",
    getConfigured: (s) => !!s.openai.api_key,
  },
  {
    id: "openrouter",
    name: "OpenRouter",
    icon: "🔀",
    description: "Access multiple models via one API",
    getConfigured: (s) => !!s.openrouter.api_key,
  },
  {
    id: "vertex_ai",
    name: "Vertex AI",
    icon: "🔷",
    description: "Claude models via Google Cloud",
    getConfigured: (s) => !!(s.vertex_ai.credentials_path || s.vertex_ai.project_id),
  },
  {
    id: "vertex_gemini",
    name: "Vertex AI Gemini",
    icon: "💎",
    description: "Gemini models via Google Cloud",
    getConfigured: (s) => !!(s.vertex_gemini.credentials_path || s.vertex_gemini.project_id),
  },
  {
    id: "xai",
    name: "xAI",
    icon: "𝕏",
    description: "Grok models from xAI",
    getConfigured: (s) => !!s.xai.api_key,
  },
  {
    id: "zai_sdk",
    name: "Z.AI SDK",
    icon: "🤖",
    description: "Z.AI native SDK (GLM models)",
    getConfigured: (s) => !!s.zai_sdk?.api_key,
  },
];

function PasswordInput({
  id,
  value,
  onChange,
  placeholder,
}: {
  id: string;
  value: string;
  onChange: (value: string) => void;
  placeholder: string;
}) {
  const [showPassword, setShowPassword] = useState(false);

  return (
    <div className="relative">
      <Input
        id={id}
        type={showPassword ? "text" : "password"}
        value={value}
        onChange={(e) => onChange(e.target.value)}
        placeholder={placeholder}
        className="pr-10 font-mono text-sm"
      />
      <button
        type="button"
        onClick={() => setShowPassword(!showPassword)}
        className="absolute right-2 top-1/2 -translate-y-1/2 p-1 text-muted-foreground hover:text-foreground transition-colors"
      >
        {showPassword ? <EyeOff className="w-4 h-4" /> : <Eye className="w-4 h-4" />}
      </button>
    </div>
  );
}

export function ProviderSettings({ settings, onChange }: ProviderSettingsProps) {
  const [openProvider, setOpenProvider] = useState<string | null>(null);
  const [providers, setProviders] = useState<ProviderConfig[]>(FALLBACK_PROVIDERS);

  // Fetch providers from backend on mount
  useEffect(() => {
    getProviders()
      .then((backendProviders) => {
        const configs = backendProviders
          .map(toProviderConfig)
          .filter((p): p is ProviderConfig => p !== null);
        if (configs.length > 0) {
          setProviders(configs);
        }
      })
      .catch((err) => {
        logger.warn("Failed to fetch providers from backend, using fallback:", err);
      });
  }, []);

  const updateProvider = <K extends keyof AiSettings>(
    provider: K,
    field: string,
    value: string | boolean | null
  ) => {
    const providerSettings = settings[provider];
    if (typeof providerSettings === "object" && providerSettings !== null) {
      onChange({
        ...settings,
        [provider]: {
          ...providerSettings,
          [field]: typeof value === "boolean" ? value : value || null,
        },
      });
    }
  };

  const handleToggle = (providerId: string) => {
    setOpenProvider(openProvider === providerId ? null : providerId);
  };

  const renderProviderFields = (provider: ProviderConfig) => {
    switch (provider.id) {
      case "vertex_ai":
        return (
          <div className="space-y-4">
            <div className="space-y-2">
              <label htmlFor="vertex-credentials" className="text-sm text-foreground">
                Credentials Path
              </label>
              <Input
                id="vertex-credentials"
                value={settings.vertex_ai.credentials_path || ""}
                onChange={(e) => updateProvider("vertex_ai", "credentials_path", e.target.value)}
                placeholder="/path/to/service-account.json"
                className="font-mono text-sm"
              />
              <p className="text-xs text-muted-foreground">
                Path to your Google Cloud service account JSON file
              </p>
            </div>

            <div className="space-y-2">
              <label htmlFor="vertex-project" className="text-sm text-foreground">
                Project ID
              </label>
              <Input
                id="vertex-project"
                value={settings.vertex_ai.project_id || ""}
                onChange={(e) => updateProvider("vertex_ai", "project_id", e.target.value)}
                placeholder="your-gcp-project-id"
              />
            </div>

            <div className="space-y-2">
              <label htmlFor="vertex-location" className="text-sm text-foreground">
                Location
              </label>
              <Input
                id="vertex-location"
                value={settings.vertex_ai.location || ""}
                onChange={(e) => updateProvider("vertex_ai", "location", e.target.value)}
                placeholder="us-east5"
              />
              <p className="text-xs text-muted-foreground">
                Google Cloud region (e.g., us-east5, europe-west1)
              </p>
            </div>
          </div>
        );

      case "vertex_gemini":
        return (
          <div className="space-y-4">
            <div className="space-y-2">
              <label htmlFor="vertex-gemini-credentials" className="text-sm text-foreground">
                Credentials Path
              </label>
              <Input
                id="vertex-gemini-credentials"
                value={settings.vertex_gemini.credentials_path || ""}
                onChange={(e) =>
                  updateProvider("vertex_gemini", "credentials_path", e.target.value)
                }
                placeholder="/path/to/service-account.json"
                className="font-mono text-sm"
              />
              <p className="text-xs text-muted-foreground">
                Path to your Google Cloud service account JSON file
              </p>
            </div>

            <div className="space-y-2">
              <label htmlFor="vertex-gemini-project" className="text-sm text-foreground">
                Project ID
              </label>
              <Input
                id="vertex-gemini-project"
                value={settings.vertex_gemini.project_id || ""}
                onChange={(e) => updateProvider("vertex_gemini", "project_id", e.target.value)}
                placeholder="your-gcp-project-id"
              />
            </div>

            <div className="space-y-2">
              <label htmlFor="vertex-gemini-location" className="text-sm text-foreground">
                Location
              </label>
              <Input
                id="vertex-gemini-location"
                value={settings.vertex_gemini.location || ""}
                onChange={(e) => updateProvider("vertex_gemini", "location", e.target.value)}
                placeholder="us-central1"
              />
              <p className="text-xs text-muted-foreground">
                Google Cloud region (e.g., us-central1, europe-west1)
              </p>
            </div>
          </div>
        );

      case "anthropic":
        return (
          <div className="space-y-2">
            <label htmlFor="anthropic-key" className="text-sm text-foreground">
              API Key
            </label>
            <PasswordInput
              id="anthropic-key"
              value={settings.anthropic.api_key || ""}
              onChange={(value) => updateProvider("anthropic", "api_key", value)}
              placeholder="sk-ant-api03-..."
            />
            <p className="text-xs text-muted-foreground">
              Get your API key from console.anthropic.com
            </p>
          </div>
        );

      case "openai":
        return (
          <div className="space-y-4">
            <div className="space-y-2">
              <label htmlFor="openai-key" className="text-sm text-foreground">
                API Key
              </label>
              <PasswordInput
                id="openai-key"
                value={settings.openai.api_key || ""}
                onChange={(value) => updateProvider("openai", "api_key", value)}
                placeholder="sk-..."
              />
            </div>

            <div className="space-y-2">
              <label htmlFor="openai-base" className="text-sm text-foreground">
                Base URL <span className="text-muted-foreground font-normal">(optional)</span>
              </label>
              <Input
                id="openai-base"
                value={settings.openai.base_url || ""}
                onChange={(e) => updateProvider("openai", "base_url", e.target.value)}
                placeholder="https://api.openai.com/v1"
              />
              <p className="text-xs text-muted-foreground">
                Custom endpoint for OpenAI-compatible APIs
              </p>
            </div>

            {/* Web Search */}
            <div className="flex items-center justify-between py-2 border-t border-[var(--border-subtle)]">
              <div>
                <div className="text-sm font-medium text-foreground">Web Search</div>
                <div className="text-xs text-muted-foreground">
                  Enable OpenAI&apos;s native web search tool
                </div>
              </div>
              <Switch
                checked={settings.openai.enable_web_search}
                onCheckedChange={(checked) =>
                  updateProvider("openai", "enable_web_search", checked)
                }
              />
            </div>

            {settings.openai.enable_web_search && (
              <div className="space-y-2">
                <label htmlFor="openai-search-context" className="text-sm text-foreground">
                  Search Context Size
                </label>
                <Select
                  value={settings.openai.web_search_context_size}
                  onValueChange={(value: WebSearchContextSize) =>
                    updateProvider("openai", "web_search_context_size", value)
                  }
                >
                  <SelectTrigger id="openai-search-context">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="low">Low (faster, cheaper)</SelectItem>
                    <SelectItem value="medium">Medium (balanced)</SelectItem>
                    <SelectItem value="high">High (more thorough)</SelectItem>
                  </SelectContent>
                </Select>
                <p className="text-xs text-muted-foreground">
                  Controls how much web content is retrieved for context
                </p>
              </div>
            )}
          </div>
        );

      case "openrouter":
        return (
          <div className="space-y-2">
            <label htmlFor="openrouter-key" className="text-sm text-foreground">
              API Key
            </label>
            <PasswordInput
              id="openrouter-key"
              value={settings.openrouter.api_key || ""}
              onChange={(value) => updateProvider("openrouter", "api_key", value)}
              placeholder="sk-or-v1-..."
            />
            <p className="text-xs text-muted-foreground">Get your API key from openrouter.ai</p>
          </div>
        );

      case "ollama":
        return (
          <div className="space-y-2">
            <label htmlFor="ollama-url" className="text-sm text-foreground">
              Base URL
            </label>
            <Input
              id="ollama-url"
              value={settings.ollama.base_url}
              onChange={(e) => updateProvider("ollama", "base_url", e.target.value)}
              placeholder="http://localhost:11434"
            />
            <p className="text-xs text-muted-foreground">
              Ollama server endpoint (default: http://localhost:11434)
            </p>
          </div>
        );

      case "gemini":
        return (
          <div className="space-y-2">
            <label htmlFor="gemini-key" className="text-sm text-foreground">
              API Key
            </label>
            <PasswordInput
              id="gemini-key"
              value={settings.gemini.api_key || ""}
              onChange={(value) => updateProvider("gemini", "api_key", value)}
              placeholder="AIza..."
            />
            <p className="text-xs text-muted-foreground">
              Get your API key from aistudio.google.com
            </p>
          </div>
        );

      case "groq":
        return (
          <div className="space-y-2">
            <label htmlFor="groq-key" className="text-sm text-foreground">
              API Key
            </label>
            <PasswordInput
              id="groq-key"
              value={settings.groq.api_key || ""}
              onChange={(value) => updateProvider("groq", "api_key", value)}
              placeholder="gsk_..."
            />
            <p className="text-xs text-muted-foreground">Get your API key from console.groq.com</p>
          </div>
        );

      case "xai":
        return (
          <div className="space-y-2">
            <label htmlFor="xai-key" className="text-sm text-foreground">
              API Key
            </label>
            <PasswordInput
              id="xai-key"
              value={settings.xai.api_key || ""}
              onChange={(value) => updateProvider("xai", "api_key", value)}
              placeholder="xai-..."
            />
            <p className="text-xs text-muted-foreground">Get your API key from x.ai</p>
          </div>
        );

      case "zai_sdk":
        return (
          <div className="space-y-4">
            <div className="space-y-2">
              <label htmlFor="z-ai-sdk-key" className="text-sm text-foreground">
                API Key
              </label>
              <PasswordInput
                id="z-ai-sdk-key"
                value={settings.zai_sdk?.api_key || ""}
                onChange={(value) => updateProvider("zai_sdk", "api_key", value)}
                placeholder="your-zai-api-key"
              />
              <p className="text-xs text-muted-foreground">Get your API key from z.ai</p>
            </div>

            <div className="space-y-2">
              <label htmlFor="z-ai-sdk-base" className="text-sm text-foreground">
                Base URL <span className="text-muted-foreground font-normal">(optional)</span>
              </label>
              <Input
                id="z-ai-sdk-base"
                value={settings.zai_sdk?.base_url || ""}
                onChange={(e) => updateProvider("zai_sdk", "base_url", e.target.value)}
                placeholder="https://open.bigmodel.cn/api/paas/v4"
              />
              <p className="text-xs text-muted-foreground">Custom endpoint for Z.AI SDK API</p>
            </div>
          </div>
        );

      default:
        return null;
    }
  };

  const getShowInSelector = (providerId: ProviderConfig["id"]): boolean => {
    const providerSettings = settings[providerId];
    if (typeof providerSettings === "object" && "show_in_selector" in providerSettings) {
      return providerSettings.show_in_selector;
    }
    return true;
  };

  return (
    <div className="space-y-4">
      {/* Default Model Selector */}
      <div className="p-4 rounded-lg bg-muted border border-[var(--border-medium)]">
        <div className="text-sm font-medium text-foreground mb-2">Default Model</div>
        <ModelSelector
          provider={settings.default_provider}
          model={settings.default_model}
          reasoningEffort={settings.default_reasoning_effort}
          settings={settings}
          onChange={(provider, model, reasoningEffort) =>
            onChange({
              ...settings,
              default_provider: provider,
              default_model: model,
              default_reasoning_effort: reasoningEffort,
            })
          }
        />
        <p className="text-xs text-muted-foreground mt-2">
          The provider and model used when starting new conversations
        </p>
      </div>

      {/* Provider Configurations */}
      <div className="space-y-1">
        {providers.map((provider) => {
          const isOpen = openProvider === provider.id;
          const isConfigured = provider.getConfigured(settings);
          const showInSelector = getShowInSelector(provider.id);
          const isDefault = settings.default_provider === provider.id;

          return (
            <Collapsible
              key={provider.id}
              open={isOpen}
              onOpenChange={() => handleToggle(provider.id)}
            >
              <div
                className={cn(
                  "border border-[var(--border-medium)] rounded-lg overflow-hidden transition-colors",
                  isOpen && "border-accent/50"
                )}
              >
                <CollapsibleTrigger asChild>
                  <button
                    type="button"
                    className={cn(
                      "w-full flex items-center gap-3 px-4 py-3 text-left transition-colors",
                      "hover:bg-[var(--bg-hover)]",
                      isOpen && "bg-[var(--bg-hover)]"
                    )}
                  >
                    <span className="text-xl w-8 h-8 flex items-center justify-center bg-muted rounded-lg border border-[var(--border-subtle)]">
                      {provider.icon}
                    </span>

                    <div className="flex-1 min-w-0">
                      <div className="flex items-center gap-2">
                        <span className="text-sm font-medium text-foreground">{provider.name}</span>
                        {isDefault && (
                          <span className="inline-flex items-center gap-1 px-1.5 py-0.5 text-[10px] font-medium bg-accent/20 text-accent rounded">
                            <Star className="w-3 h-3 fill-current" />
                            Default
                          </span>
                        )}
                      </div>
                      <div className="flex items-center gap-2 mt-0.5">
                        <span
                          className={cn(
                            "inline-flex items-center gap-1 text-xs",
                            isConfigured ? "text-[var(--success)]" : "text-muted-foreground"
                          )}
                        >
                          <span
                            className={cn(
                              "w-1.5 h-1.5 rounded-full",
                              isConfigured ? "bg-[var(--success)]" : "bg-muted-foreground/50"
                            )}
                          />
                          {isConfigured ? "Configured" : "Not configured"}
                        </span>
                      </div>
                    </div>

                    <ChevronDown
                      className={cn(
                        "w-4 h-4 text-muted-foreground transition-transform duration-200",
                        isOpen && "rotate-180"
                      )}
                    />
                  </button>
                </CollapsibleTrigger>

                <CollapsibleContent>
                  <div className="px-4 pb-4 pt-2 border-t border-[var(--border-subtle)] bg-[var(--bg-subtle)]">
                    {/* Show in selector toggle */}
                    <div className="flex items-center justify-between py-3 mb-4 border-b border-[var(--border-subtle)]">
                      <div>
                        <div className="text-sm font-medium text-foreground">
                          Show in model selector
                        </div>
                        <div className="text-xs text-muted-foreground">
                          Make this provider available for selection
                        </div>
                      </div>
                      <Switch
                        checked={showInSelector}
                        onCheckedChange={(checked) =>
                          updateProvider(provider.id, "show_in_selector", checked)
                        }
                      />
                    </div>

                    {/* Provider-specific fields */}
                    {renderProviderFields(provider)}
                  </div>
                </CollapsibleContent>
              </div>
            </Collapsible>
          );
        })}
      </div>
    </div>
  );
}
