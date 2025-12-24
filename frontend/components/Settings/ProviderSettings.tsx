import { ChevronDown, Eye, EyeOff, Star } from "lucide-react";
import { useState } from "react";
import { Collapsible, CollapsibleContent, CollapsibleTrigger } from "@/components/ui/collapsible";
import { Input } from "@/components/ui/input";
import { Switch } from "@/components/ui/switch";
import type { AiSettings } from "@/lib/settings";
import { cn } from "@/lib/utils";
import { ModelSelector } from "./ModelSelector";

interface ProviderSettingsProps {
  settings: AiSettings;
  onChange: (settings: AiSettings) => void;
}

interface ProviderConfig {
  id: keyof Pick<
    AiSettings,
    | "vertex_ai"
    | "openrouter"
    | "anthropic"
    | "openai"
    | "ollama"
    | "gemini"
    | "groq"
    | "xai"
    | "zai"
  >;
  name: string;
  icon: string;
  description: string;
  getConfigured: (settings: AiSettings) => boolean;
}

const PROVIDERS: ProviderConfig[] = [
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
    id: "xai",
    name: "xAI",
    icon: "𝕏",
    description: "Grok models from xAI",
    getConfigured: (s) => !!s.xai.api_key,
  },
  {
    id: "zai",
    name: "Z.AI",
    icon: "🤖",
    description: "GLM models via Z.AI Coding Plan API",
    getConfigured: (s) => !!s.zai.api_key,
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

      case "zai":
        return (
          <div className="space-y-2">
            <label htmlFor="zai-key" className="text-sm text-foreground">
              API Key
            </label>
            <PasswordInput
              id="zai-key"
              value={settings.zai.api_key || ""}
              onChange={(value) => updateProvider("zai", "api_key", value)}
              placeholder="zai-..."
            />
            <p className="text-xs text-muted-foreground">Get your API key from z.ai</p>
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
          settings={settings}
          onChange={(provider, model) =>
            onChange({ ...settings, default_provider: provider, default_model: model })
          }
        />
        <p className="text-xs text-muted-foreground mt-2">
          The provider and model used when starting new conversations
        </p>
      </div>

      {/* Provider Configurations */}
      <div className="space-y-1">
        {PROVIDERS.map((provider) => {
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
