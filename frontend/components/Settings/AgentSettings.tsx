import { Input } from "@/components/ui/input";
import { Switch } from "@/components/ui/switch";
import type {
  AgentSettings as AgentSettingsType,
  SubAgentModelConfig,
  ToolsSettings,
} from "@/lib/settings";
import { SubAgentSettings } from "./SubAgentSettings";

interface AgentSettingsProps {
  settings: AgentSettingsType;
  toolsSettings: ToolsSettings;
  subAgentModels: Record<string, SubAgentModelConfig>;
  onChange: (settings: AgentSettingsType) => void;
  onToolsChange: (tools: ToolsSettings) => void;
  onSubAgentModelsChange: (models: Record<string, SubAgentModelConfig>) => void;
}

export function AgentSettings({
  settings,
  toolsSettings,
  subAgentModels,
  onChange,
  onToolsChange,
  onSubAgentModelsChange,
}: AgentSettingsProps) {
  const updateField = <K extends keyof AgentSettingsType>(key: K, value: AgentSettingsType[K]) => {
    onChange({ ...settings, [key]: value });
  };

  const updateToolsField = <K extends keyof ToolsSettings>(key: K, value: ToolsSettings[K]) => {
    onToolsChange({ ...toolsSettings, [key]: value });
  };

  return (
    <div className="space-y-8">
      {/* Session Persistence */}
      <div className="flex items-center justify-between">
        <div className="space-y-1">
          <label
            htmlFor="agent-session-persistence"
            className="text-sm font-medium text-foreground"
          >
            Session Persistence
          </label>
          <p className="text-xs text-muted-foreground">Auto-save conversations to disk</p>
        </div>
        <Switch
          id="agent-session-persistence"
          checked={settings.session_persistence}
          onCheckedChange={(checked) => updateField("session_persistence", checked)}
        />
      </div>

      {/* Session Retention */}
      <div className="space-y-2">
        <label htmlFor="agent-session-retention" className="text-sm font-medium text-foreground">
          Session Retention (days)
        </label>
        <Input
          id="agent-session-retention"
          type="number"
          min={0}
          max={365}
          value={settings.session_retention_days}
          onChange={(e) => updateField("session_retention_days", parseInt(e.target.value, 10) || 0)}
          className="w-24"
        />
        <p className="text-xs text-muted-foreground">
          How long to keep saved sessions (0 = forever)
        </p>
      </div>

      {/* Pattern Learning */}
      <div className="flex items-center justify-between">
        <div className="space-y-1">
          <label htmlFor="agent-pattern-learning" className="text-sm font-medium text-foreground">
            Pattern Learning
          </label>
          <p className="text-xs text-muted-foreground">Learn from approvals for auto-approval</p>
        </div>
        <Switch
          id="agent-pattern-learning"
          checked={settings.pattern_learning}
          onCheckedChange={(checked) => updateField("pattern_learning", checked)}
        />
      </div>

      {/* Min Approvals */}
      <div className="space-y-2">
        <label htmlFor="agent-min-approvals" className="text-sm font-medium text-foreground">
          Minimum Approvals
        </label>
        <Input
          id="agent-min-approvals"
          type="number"
          min={1}
          max={10}
          value={settings.min_approvals_for_auto}
          onChange={(e) => updateField("min_approvals_for_auto", parseInt(e.target.value, 10) || 3)}
          className="w-24"
        />
        <p className="text-xs text-muted-foreground">
          Minimum approvals before a tool can be auto-approved
        </p>
      </div>

      {/* Approval Threshold */}
      <div className="space-y-2">
        <label htmlFor="agent-approval-threshold" className="text-sm font-medium text-foreground">
          Approval Threshold: {(settings.approval_threshold * 100).toFixed(0)}%
        </label>
        <input
          id="agent-approval-threshold"
          type="range"
          min={0}
          max={100}
          value={settings.approval_threshold * 100}
          onChange={(e) => updateField("approval_threshold", parseInt(e.target.value, 10) / 100)}
          className="w-full h-2 bg-muted rounded-lg appearance-none cursor-pointer accent-accent"
        />
        <p className="text-xs text-muted-foreground">Required approval rate for auto-approval</p>
      </div>

      {/* Divider */}
      <div className="border-t border-[var(--color-border-medium)]" />

      {/* Tools Section */}
      <div className="space-y-4">
        <h3 className="text-sm font-medium text-foreground">Tools</h3>

        {/* Tavily Web Search */}
        <div className="flex items-center justify-between">
          <div className="space-y-1">
            <label htmlFor="tools-web-search" className="text-sm font-medium text-foreground">
              Web Search (Tavily)
            </label>
            <p className="text-xs text-muted-foreground">
              Enable web search, extract, crawl, and map tools. Requires TAVILY_API_KEY.
            </p>
          </div>
          <Switch
            id="tools-web-search"
            checked={toolsSettings.web_search}
            onCheckedChange={(checked) => updateToolsField("web_search", checked)}
          />
        </div>
      </div>

      {/* Divider */}
      <div className="border-t border-[var(--color-border-medium)]" />

      {/* Sub-Agent Model Overrides */}
      <SubAgentSettings subAgentModels={subAgentModels} onChange={onSubAgentModelsChange} />
    </div>
  );
}
