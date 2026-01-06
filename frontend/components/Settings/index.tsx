import { Bot, Cog, FolderCode, Loader2, Server, Shield, Terminal, X } from "lucide-react";
import { useCallback, useEffect, useState } from "react";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogTitle } from "@/components/ui/dialog";
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
import { ProviderSettings } from "./ProviderSettings";
import { TerminalSettings } from "./TerminalSettings";

interface SettingsDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

type SettingsSection = "providers" | "ai" | "terminal" | "agent" | "codebases" | "advanced";

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
    id: "agent",
    label: "Agent",
    icon: <Cog className="w-4 h-4" />,
    description: "Session and approval settings",
  },
  {
    id: "codebases",
    label: "Codebases",
    icon: <FolderCode className="w-4 h-4" />,
    description: "Manage indexed repositories",
  },
  {
    id: "advanced",
    label: "Advanced",
    icon: <Shield className="w-4 h-4" />,
    description: "Privacy and debug options",
  },
];

export function SettingsDialog({ open, onOpenChange }: SettingsDialogProps) {
  const [settings, setSettings] = useState<QbitSettings | null>(null);
  const [activeSection, setActiveSection] = useState<SettingsSection>("providers");
  const [isLoading, setIsLoading] = useState(false);
  const [isSaving, setIsSaving] = useState(false);

  // Load settings when dialog opens
  useEffect(() => {
    if (open) {
      setIsLoading(true);
      getSettings()
        .then(setSettings)
        .catch((err) => {
          logger.error("Failed to load settings:", err);
          notify.error("Failed to load settings");
        })
        .finally(() => setIsLoading(false));
    }
  }, [open]);

  const handleSave = useCallback(async () => {
    if (!settings) return;

    setIsSaving(true);
    try {
      // Reload codebases from backend before saving to preserve any changes made
      // via CodebasesSettings (which saves directly to backend, not to parent state)
      const currentCodebases = await listIndexedCodebases();
      const updatedCodebases: CodebaseConfig[] = currentCodebases.map((cb) => ({
        path: cb.path,
        memory_file: cb.memory_file,
      }));

      const settingsToSave = {
        ...settings,
        codebases: updatedCodebases,
      };

      await updateSettings(settingsToSave);
      // Notify other components (e.g., StatusBar) that settings have been updated
      window.dispatchEvent(new CustomEvent("settings-updated", { detail: settingsToSave }));
      notify.success("Settings saved");
      onOpenChange(false);
    } catch (err) {
      logger.error("Failed to save settings:", err);
      notify.error("Failed to save settings");
    } finally {
      setIsSaving(false);
    }
  }, [settings, onOpenChange]);

  const handleCancel = useCallback(() => {
    onOpenChange(false);
  }, [onOpenChange]);

  // Handler to update a specific section of settings
  const updateSection = useCallback(
    <K extends keyof QbitSettings>(section: K, value: QbitSettings[K]) => {
      setSettings((prev) => (prev ? { ...prev, [section]: value } : null));
    },
    []
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
      case "agent":
        return (
          <AgentSettings
            settings={settings.agent}
            onChange={(agent) => updateSection("agent", agent)}
          />
        );
      case "codebases":
        return <CodebasesSettings />;
      case "advanced":
        return (
          <AdvancedSettings
            settings={settings.advanced}
            privacy={settings.privacy}
            onChange={(advanced) => updateSection("advanced", advanced)}
            onPrivacyChange={(privacy) => updateSection("privacy", privacy)}
          />
        );
    }
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent
        showCloseButton={false}
        className="!max-w-none !inset-0 !translate-x-0 !translate-y-0 !w-screen !h-screen p-0 bg-background border-0 rounded-none text-foreground flex flex-col overflow-hidden"
      >
        {/* Visually hidden title for screen readers */}
        <DialogTitle className="sr-only">Settings</DialogTitle>

        {/* Header */}
        <div className="flex items-center justify-between px-6 py-4 border-b border-[var(--border-medium)] flex-shrink-0">
          <h2 className="text-lg font-semibold text-foreground">Settings</h2>
          <button
            type="button"
            onClick={handleCancel}
            className="p-1.5 rounded-md hover:bg-[var(--bg-hover)] text-muted-foreground hover:text-foreground transition-colors"
          >
            <X className="w-5 h-5" />
          </button>
        </div>

        {isLoading ? (
          <div className="flex-1 flex items-center justify-center">
            <Loader2 className="w-6 h-6 text-muted-foreground animate-spin" />
          </div>
        ) : settings ? (
          <div className="flex-1 flex min-h-0 overflow-hidden">
            {/* Sidebar Navigation */}
            <nav className="w-64 border-r border-[var(--border-medium)] flex flex-col flex-shrink-0">
              <div className="flex-1 py-2">
                {NAV_ITEMS.map((item) => (
                  <button
                    key={item.id}
                    type="button"
                    onClick={() => setActiveSection(item.id)}
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
        ) : (
          <div className="flex-1 flex items-center justify-center">
            <span className="text-destructive">Failed to load settings</span>
          </div>
        )}

        {/* Footer */}
        <div className="flex items-center justify-end gap-3 px-6 py-4 border-t border-[var(--border-medium)] flex-shrink-0">
          <Button variant="outline" onClick={handleCancel}>
            Cancel
          </Button>
          <Button onClick={handleSave} disabled={!settings || isSaving}>
            {isSaving ? "Saving..." : "Save Changes"}
          </Button>
        </div>
      </DialogContent>
    </Dialog>
  );
}
