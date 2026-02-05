/**
 * SettingsTabContent - Settings panel rendered as tab content.
 * Extracted from SettingsDialog to enable settings as a tab instead of modal.
 */

import {
  Bell,
  Bot,
  Cog,
  FileCode,
  FolderCode,
  Loader2,
  Puzzle,
  Server,
  Shield,
  Terminal,
} from "lucide-react";
import { useCallback, useEffect, useState } from "react";
import { ScrollArea } from "@/components/ui/scroll-area";
import { listIndexedCodebases } from "@/lib/indexer";
import { logger } from "@/lib/logger";
import { notify } from "@/lib/notify";
import {
  type CodebaseConfig,
  getSettings,
  type QbitSettings,
  updateSettings,
} from "@/lib/settings";
import { cn } from "@/lib/utils";
import { AdvancedSettings } from "./AdvancedSettings";
import { AgentSettings } from "./AgentSettings";
import { AiSettings } from "./AiSettings";
import { CodebasesSettings } from "./CodebasesSettings";
import { EditorSettings } from "./EditorSettings";
import { NotificationsSettings } from "./NotificationsSettings";
import { ProviderSettings } from "./ProviderSettings";
import { McpSettings } from "./McpSettings";
import { TerminalSettings } from "./TerminalSettings";

type SettingsSection =
  | "providers"
  | "ai"
  | "terminal"
  | "editor"
  | "agent"
  | "mcp"
  | "codebases"
  | "notifications"
  | "advanced";

interface NavItem {
  id: SettingsSection;
  label: string;
  icon: React.ReactNode;
  description: string;
}

const NAV_ITEMS: NavItem[] = [
  {
    id: "providers",
    label: "Providers",
    icon: <Server className="w-4 h-4" />,
    description: "Configure AI provider credentials",
  },
  {
    id: "ai",
    label: "AI & Models",
    icon: <Bot className="w-4 h-4" />,
    description: "Default provider and synthesis",
  },
  {
    id: "terminal",
    label: "Terminal",
    icon: <Terminal className="w-4 h-4" />,
    description: "Shell and display settings",
  },
  {
    id: "editor",
    label: "Editor",
    icon: <FileCode className="w-4 h-4" />,
    description: "File editor preferences",
  },
  {
    id: "agent",
    label: "Agent",
    icon: <Cog className="w-4 h-4" />,
    description: "Session and approval settings",
  },
  {
    id: "mcp",
    label: "MCP Servers",
    icon: <Puzzle className="w-4 h-4" />,
    description: "External tools via Model Context Protocol",
  },
  {
    id: "codebases",
    label: "Codebases",
    icon: <FolderCode className="w-4 h-4" />,
    description: "Manage indexed repositories",
  },
  {
    id: "notifications",
    label: "Notifications",
    icon: <Bell className="w-4 h-4" />,
    description: "System notification settings",
  },
  {
    id: "advanced",
    label: "Advanced",
    icon: <Shield className="w-4 h-4" />,
    description: "Privacy and debug options",
  },
];

export function SettingsTabContent() {
  const [settings, setSettings] = useState<QbitSettings | null>(null);
  const [activeSection, setActiveSection] = useState<SettingsSection>("providers");
  const [isLoading, setIsLoading] = useState(true);

  // Load settings on mount
  useEffect(() => {
    setIsLoading(true);
    getSettings()
      .then(setSettings)
      .catch((err) => {
        logger.error("Failed to load settings:", err);
        notify.error("Failed to load settings");
      })
      .finally(() => setIsLoading(false));
  }, []);

  // Auto-save settings when they change
  const saveSettings = useCallback(async (settingsToSave: QbitSettings) => {
    try {
      // Reload codebases from backend before saving to preserve any changes made
      // via CodebasesSettings (which saves directly to backend, not to parent state)
      const currentCodebases = await listIndexedCodebases();
      const updatedCodebases: CodebaseConfig[] = currentCodebases.map((cb) => ({
        path: cb.path,
        memory_file: cb.memory_file,
      }));

      const finalSettings = {
        ...settingsToSave,
        codebases: updatedCodebases,
      };

      await updateSettings(finalSettings);
      // Notify other components (e.g., StatusBar) that settings have been updated
      window.dispatchEvent(new CustomEvent("settings-updated", { detail: finalSettings }));
    } catch (err) {
      logger.error("Failed to save settings:", err);
      notify.error("Failed to save settings");
    }
  }, []);

  // Handler to update a specific section of settings and auto-save
  const updateSection = useCallback(
    <K extends keyof QbitSettings>(section: K, value: QbitSettings[K]) => {
      setSettings((prev) => {
        if (!prev) return null;
        const updated = { ...prev, [section]: value };
        // Auto-save after state update
        saveSettings(updated);
        return updated;
      });
    },
    [saveSettings]
  );

  const renderContent = () => {
    if (!settings) return null;

    switch (activeSection) {
      case "providers":
        return (
          <ProviderSettings settings={settings.ai} onChange={(ai) => updateSection("ai", ai)} />
        );
      case "ai":
        return (
          <AiSettings
            apiKeys={settings.api_keys}
            sidecarSettings={settings.sidecar}
            onApiKeysChange={(keys) => updateSection("api_keys", keys)}
            onSidecarChange={(sidecar) => updateSection("sidecar", sidecar)}
          />
        );
      case "terminal":
        return (
          <TerminalSettings
            settings={settings.terminal}
            onChange={(terminal) => updateSection("terminal", terminal)}
          />
        );
      case "editor":
        return <EditorSettings />;
      case "agent":
        return (
          <AgentSettings
            settings={settings.agent}
            toolsSettings={settings.tools}
            subAgentModels={settings.ai.sub_agent_models || {}}
            onChange={(agent) => updateSection("agent", agent)}
            onToolsChange={(tools) => updateSection("tools", tools)}
            onSubAgentModelsChange={(models) =>
              updateSection("ai", { ...settings.ai, sub_agent_models: models })
            }
          />
        );
      case "mcp":
        return <McpSettings />;
      case "codebases":
        return <CodebasesSettings />;
      case "notifications":
        return (
          <NotificationsSettings
            settings={settings.notifications}
            onChange={(notifications) => updateSection("notifications", notifications)}
          />
        );
      case "advanced":
        return (
          <AdvancedSettings
            settings={settings.advanced}
            privacy={settings.privacy}
            onChange={(advanced) => updateSection("advanced", advanced)}
            onPrivacyChange={(privacy) => updateSection("privacy", privacy)}
          />
        );
      default:
        return null;
    }
  };

  // Memoize section navigation handler to prevent unnecessary re-renders in mapped buttons
  const handleSectionChange = useCallback((sectionId: SettingsSection) => {
    setActiveSection(sectionId);
  }, []);

  if (isLoading) {
    return (
      <div className="h-full w-full flex items-center justify-center">
        <Loader2 className="w-6 h-6 text-muted-foreground animate-spin" />
      </div>
    );
  }

  if (!settings) {
    return (
      <div className="h-full w-full flex items-center justify-center">
        <span className="text-destructive">Failed to load settings</span>
      </div>
    );
  }

  return (
    <div className="h-full w-full flex flex-col overflow-hidden bg-background">
      {/* Header */}
      <div className="flex items-center justify-between px-6 py-4 border-b border-[var(--border-medium)] flex-shrink-0">
        <h2 className="text-lg font-semibold text-foreground">Settings</h2>
      </div>

      <div className="flex-1 flex min-h-0 overflow-hidden">
        {/* Sidebar Navigation */}
        <nav className="w-64 border-r border-[var(--border-medium)] flex flex-col flex-shrink-0">
          <div className="flex-1 py-2">
            {NAV_ITEMS.map((item) => (
              <button
                key={item.id}
                type="button"
                onClick={() => handleSectionChange(item.id)}
                className={cn(
                  "w-full flex items-start gap-3 px-4 py-3 text-left transition-colors",
                  activeSection === item.id
                    ? "bg-[var(--accent-dim)] text-foreground border-l-2 border-accent"
                    : "text-muted-foreground hover:bg-[var(--bg-hover)] hover:text-foreground border-l-2 border-transparent"
                )}
              >
                <span className={cn("mt-0.5", activeSection === item.id ? "text-accent" : "")}>
                  {item.icon}
                </span>
                <div className="flex-1 min-w-0">
                  <div className="text-sm font-medium">{item.label}</div>
                  <div className="text-xs text-muted-foreground mt-0.5">{item.description}</div>
                </div>
              </button>
            ))}
          </div>
        </nav>

        {/* Main Content */}
        <div className="flex-1 flex flex-col min-w-0 min-h-0 overflow-hidden">
          <ScrollArea className="h-full">
            <div className="p-6 max-w-3xl">{renderContent()}</div>
          </ScrollArea>
        </div>
      </div>
    </div>
  );
}
