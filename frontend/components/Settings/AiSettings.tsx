import { useState } from "react";
import { Input } from "@/components/ui/input";
import type { ApiKeysSettings, SidecarSettings, SynthesisBackendType } from "@/lib/settings";

interface AiSettingsProps {
  apiKeys: ApiKeysSettings;
  sidecarSettings: SidecarSettings;
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
  apiKeys,
  sidecarSettings,
  onApiKeysChange,
  onSidecarChange,
}: AiSettingsProps) {
  const [synthesisStatus, setSynthesisStatus] = useState<string>("");
  const [isChangingBackend, setIsChangingBackend] = useState(false);

  const handleSynthesisBackendChange = (value: string) => {
    setIsChangingBackend(true);
    setSynthesisStatus("");

    onSidecarChange({ ...sidecarSettings, synthesis_backend: value as SynthesisBackendType });

    const backendNames: Record<string, string> = {
      local: "Local LLM",
      vertex_anthropic: "Vertex AI (Anthropic)",
      openai: "OpenAI",
      grok: "Grok",
      template: "Template-based",
    };
    setSynthesisStatus(`Set to ${backendNames[value] || value}`);
    setIsChangingBackend(false);
  };

  return (
    <div className="space-y-6">
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
          {synthesisStatus && <p className="text-xs text-[var(--success)]">{synthesisStatus}</p>}
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
                    value: "claude-sonnet-4-6-20260217",
                    label: "Claude Sonnet 4.6",
                  },
                  {
                    value: "claude-opus-4-5-20251101",
                    label: "Claude Opus 4.5 (Most Capable)",
                  },
                  {
                    value: "claude-sonnet-4-5@20250929",
                    label: "Claude Sonnet 4.5",
                  },
                  {
                    value: "claude-haiku-4-5-20251001",
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
                  By default, synthesis uses your Vertex AI configuration from the Providers
                  section.
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
