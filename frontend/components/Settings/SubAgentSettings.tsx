import { Trash2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import type { AiProvider, SubAgentModelConfig } from "@/lib/settings";

interface SubAgentSettingsProps {
  subAgentModels: Record<string, SubAgentModelConfig>;
  onChange: (models: Record<string, SubAgentModelConfig>) => void;
}

// Known sub-agents (these match what's registered in the backend)
const KNOWN_SUB_AGENTS = [
  { id: "coder", name: "Coder", description: "Specialized for code generation tasks" },
  { id: "researcher", name: "Researcher", description: "Specialized for research tasks" },
  { id: "analyzer", name: "Analyzer", description: "Specialized for code analysis" },
];

const PROVIDER_OPTIONS: { value: AiProvider; label: string }[] = [
  { value: "vertex_ai", label: "Vertex AI (Claude)" },
  { value: "anthropic", label: "Anthropic" },
  { value: "openai", label: "OpenAI" },
  { value: "openrouter", label: "OpenRouter" },
  { value: "gemini", label: "Gemini" },
  { value: "groq", label: "Groq" },
  { value: "ollama", label: "Ollama" },
  { value: "xai", label: "xAI (Grok)" },
  { value: "zai", label: "Z.AI (GLM)" },
  { value: "zai_anthropic", label: "Z.AI (Anthropic)" },
];

// Common models by provider (not exhaustive, users can type custom models)
const MODEL_SUGGESTIONS: Record<AiProvider, string[]> = {
  vertex_ai: [
    "claude-opus-4-5@20251101",
    "claude-sonnet-4-5@20250929",
    "claude-haiku-4-5@20251001",
  ],
  anthropic: [
    "claude-opus-4-5-20251101",
    "claude-sonnet-4-5-20250929",
    "claude-haiku-4-5-20251001",
  ],
  openai: ["gpt-4o", "gpt-4o-mini", "o3", "o3-mini", "gpt-5"],
  openrouter: [
    "anthropic/claude-opus-4.5",
    "anthropic/claude-sonnet-4.5",
    "openai/gpt-4o",
    "google/gemini-2.5-pro",
  ],
  gemini: ["gemini-2.5-pro", "gemini-2.5-flash", "gemini-3-pro-preview"],
  groq: ["llama-3.3-70b-versatile", "llama-3.1-8b-instant"],
  ollama: ["llama3.2", "codellama", "mistral"],
  xai: ["grok-4-1-fast-reasoning", "grok-4-1-fast-non-reasoning"],
  zai: ["GLM-4.7", "GLM-4.5-air"],
  zai_anthropic: ["GLM-4.7", "GLM-4.6", "GLM-4.5-Air"],
};

// Simple Select component
function SimpleSelect({
  id,
  value,
  onValueChange,
  options,
  placeholder,
}: {
  id?: string;
  value: string;
  onValueChange: (value: string) => void;
  options: { value: string; label: string }[];
  placeholder?: string;
}) {
  return (
    <select
      id={id}
      value={value}
      onChange={(e) => onValueChange(e.target.value)}
      className="w-full h-9 rounded-md border border-[var(--color-border-medium)] bg-muted px-3 py-1 text-sm text-foreground focus:outline-none focus:ring-1 focus:ring-accent cursor-pointer appearance-none"
      style={{
        backgroundImage:
          "url(\"data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='12' height='12' viewBox='0 0 24 24' fill='none' stroke='%239aa0a6' stroke-width='2'%3E%3Cpath d='m6 9 6 6 6-6'/%3E%3C/svg%3E\")",
        backgroundRepeat: "no-repeat",
        backgroundPosition: "right 12px center",
      }}
    >
      {placeholder && (
        <option value="" className="bg-card">
          {placeholder}
        </option>
      )}
      {options.map((opt) => (
        <option key={opt.value} value={opt.value} className="bg-card">
          {opt.label}
        </option>
      ))}
    </select>
  );
}

export function SubAgentSettings({ subAgentModels, onChange }: SubAgentSettingsProps) {
  const updateSubAgent = (agentId: string, config: SubAgentModelConfig | null) => {
    if (config === null) {
      // Remove the config
      const { [agentId]: _, ...rest } = subAgentModels;
      onChange(rest);
    } else {
      onChange({ ...subAgentModels, [agentId]: config });
    }
  };

  const getConfig = (agentId: string): SubAgentModelConfig => {
    return subAgentModels[agentId] || {};
  };

  const hasOverride = (agentId: string): boolean => {
    const config = subAgentModels[agentId];
    return Boolean(config?.provider && config?.model);
  };

  return (
    <div className="space-y-6">
      <div className="space-y-2">
        <h4 className="text-sm font-medium text-accent">Sub-Agent Model Overrides</h4>
        <p className="text-xs text-muted-foreground">
          Configure different models for specific sub-agents. By default, sub-agents inherit the
          main agent&apos;s model. Changes apply to new sessions.
        </p>
      </div>

      <div className="space-y-4">
        {KNOWN_SUB_AGENTS.map((agent) => {
          const config = getConfig(agent.id);
          const isConfigured = hasOverride(agent.id);

          return (
            <div
              key={agent.id}
              className="p-4 rounded-lg bg-muted border border-[var(--color-border-medium)] space-y-3"
            >
              <div className="flex items-center justify-between">
                <div>
                  <h5 className="text-sm font-medium text-foreground">{agent.name}</h5>
                  <p className="text-xs text-muted-foreground">{agent.description}</p>
                </div>
                {isConfigured && (
                  <Button
                    variant="ghost"
                    size="sm"
                    onClick={() => updateSubAgent(agent.id, null)}
                    className="text-muted-foreground hover:text-destructive"
                    title="Clear override (use main model)"
                  >
                    <Trash2 className="w-4 h-4" />
                  </Button>
                )}
              </div>

              <div className="grid grid-cols-2 gap-3">
                <div className="space-y-1">
                  <label htmlFor={`${agent.id}-provider`} className="text-xs text-muted-foreground">
                    Provider
                  </label>
                  <SimpleSelect
                    id={`${agent.id}-provider`}
                    value={config.provider || ""}
                    onValueChange={(value) =>
                      updateSubAgent(agent.id, {
                        ...config,
                        provider: value as AiProvider,
                        // Clear model when provider changes
                        model: value !== config.provider ? undefined : config.model,
                      })
                    }
                    options={PROVIDER_OPTIONS}
                    placeholder="Use main agent"
                  />
                </div>

                <div className="space-y-1">
                  <label htmlFor={`${agent.id}-model`} className="text-xs text-muted-foreground">
                    Model
                  </label>
                  {config.provider ? (
                    <div className="relative">
                      <Input
                        id={`${agent.id}-model`}
                        type="text"
                        value={config.model || ""}
                        onChange={(e) =>
                          updateSubAgent(agent.id, { ...config, model: e.target.value })
                        }
                        placeholder="Enter model name"
                        list={`${agent.id}-models`}
                        className="bg-background border-border text-foreground h-9"
                      />
                      <datalist id={`${agent.id}-models`}>
                        {(MODEL_SUGGESTIONS[config.provider] || []).map((model) => (
                          <option key={model} value={model} />
                        ))}
                      </datalist>
                    </div>
                  ) : (
                    <Input
                      disabled
                      placeholder="Select provider first"
                      className="bg-muted border-border text-muted-foreground h-9"
                    />
                  )}
                </div>
              </div>

              {isConfigured && (
                <p className="text-xs text-[var(--color-success)]">
                  Using {config.provider} / {config.model}
                </p>
              )}
              {!isConfigured && (
                <p className="text-xs text-muted-foreground">
                  Using main agent&apos;s model (default)
                </p>
              )}
            </div>
          );
        })}
      </div>

      <div className="text-xs text-muted-foreground border-t border-[var(--color-border-medium)] pt-4">
        <p>
          <strong>Tip:</strong> Use a faster/cheaper model for sub-agents that do simpler tasks. For
          example, use GPT-4o-mini for the coder agent while keeping Claude Opus for the main agent.
        </p>
      </div>
    </div>
  );
}
