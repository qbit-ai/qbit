import { Switch } from "@/components/ui/switch";
import { notify } from "@/lib/notify";
import type {
  AdvancedSettings as AdvancedSettingsType,
  NotificationsSettings,
  PrivacySettings,
} from "@/lib/settings";
import { requestNativeNotificationPermission } from "@/lib/systemNotifications";

interface AdvancedSettingsProps {
  settings: AdvancedSettingsType;
  privacy: PrivacySettings;
  notifications: NotificationsSettings;
  onChange: (settings: AdvancedSettingsType) => void;
  onPrivacyChange: (privacy: PrivacySettings) => void;
  onNotificationsChange: (notifications: NotificationsSettings) => void;
}

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

export function AdvancedSettings({
  settings,
  privacy,
  notifications,
  onChange,
  onPrivacyChange,
  onNotificationsChange,
}: AdvancedSettingsProps) {
  const logLevelOptions = [
    { value: "error", label: "Error" },
    { value: "warn", label: "Warn" },
    { value: "info", label: "Info" },
    { value: "debug", label: "Debug" },
    { value: "trace", label: "Trace" },
  ];

  const handleNotificationToggle = async (checked: boolean) => {
    if (checked) {
      const granted = await requestNativeNotificationPermission();
      if (granted) {
        onNotificationsChange({ ...notifications, native_enabled: true });
      } else {
        notify.error("Notification permission denied");
        onNotificationsChange({ ...notifications, native_enabled: false });
      }
    } else {
      onNotificationsChange({ ...notifications, native_enabled: false });
    }
  };

  return (
    <div className="space-y-6">
      {/* Log Level */}
      <div className="space-y-2">
        <label htmlFor="advanced-log-level" className="text-sm font-medium text-foreground">
          Log Level
        </label>
        <SimpleSelect
          id="advanced-log-level"
          value={settings.log_level}
          onValueChange={(value) =>
            onChange({ ...settings, log_level: value as AdvancedSettingsType["log_level"] })
          }
          options={logLevelOptions}
        />
        <p className="text-xs text-muted-foreground">Verbosity of debug logging</p>
      </div>

      {/* Notifications */}
      <div className="flex items-center justify-between">
        <div className="space-y-1">
          <label htmlFor="notifications-native" className="text-sm font-medium text-foreground">
            Native System Notifications
          </label>
          <p className="text-xs text-muted-foreground">
            Show OS notifications when tasks complete in background
          </p>
        </div>
        <Switch
          id="notifications-native"
          checked={notifications.native_enabled}
          onCheckedChange={handleNotificationToggle}
        />
      </div>

      {/* Experimental Features */}
      <div className="flex items-center justify-between">
        <div className="space-y-1">
          <label htmlFor="advanced-experimental" className="text-sm font-medium text-foreground">
            Experimental Features
          </label>
          <p className="text-xs text-muted-foreground">Enable experimental functionality</p>
        </div>
        <Switch
          id="advanced-experimental"
          checked={settings.enable_experimental}
          onCheckedChange={(checked) => onChange({ ...settings, enable_experimental: checked })}
        />
      </div>

      {/* LLM API Logs */}
      <div className="flex items-center justify-between">
        <div className="space-y-1">
          <label htmlFor="advanced-llm-api-logs" className="text-sm font-medium text-foreground">
            LLM API Logs
          </label>
          <p className="text-xs text-muted-foreground">
            Log raw API request/response to ./logs/api/
          </p>
        </div>
        <Switch
          id="advanced-llm-api-logs"
          checked={settings.enable_llm_api_logs}
          onCheckedChange={(checked) => onChange({ ...settings, enable_llm_api_logs: checked })}
        />
      </div>

      {/* Extract Raw SSE */}
      <div className="flex items-center justify-between">
        <div className="space-y-1">
          <label htmlFor="advanced-extract-raw-sse" className="text-sm font-medium text-foreground">
            Extract Raw SSE Property
          </label>
          <p className="text-xs text-muted-foreground">
            Parse SSE chunks as JSON objects instead of escaped strings
          </p>
        </div>
        <Switch
          id="advanced-extract-raw-sse"
          checked={settings.extract_raw_sse}
          onCheckedChange={(checked) => onChange({ ...settings, extract_raw_sse: checked })}
        />
      </div>

      {/* Privacy Section */}
      <div className="space-y-4 p-4 rounded-lg bg-muted border border-[var(--border-medium)]">
        <h4 className="text-sm font-medium text-accent">Privacy</h4>

        {/* Usage Statistics */}
        <div className="flex items-center justify-between">
          <div className="space-y-1">
            <label htmlFor="privacy-usage-stats" className="text-sm text-foreground">
              Usage Statistics
            </label>
            <p className="text-xs text-muted-foreground">Send anonymous usage data</p>
          </div>
          <Switch
            id="privacy-usage-stats"
            checked={privacy.usage_statistics}
            onCheckedChange={(checked) =>
              onPrivacyChange({ ...privacy, usage_statistics: checked })
            }
          />
        </div>

        {/* Log Prompts */}
        <div className="flex items-center justify-between">
          <div className="space-y-1">
            <label htmlFor="privacy-log-prompts" className="text-sm text-foreground">
              Log Prompts
            </label>
            <p className="text-xs text-muted-foreground">Save prompts locally for debugging</p>
          </div>
          <Switch
            id="privacy-log-prompts"
            checked={privacy.log_prompts}
            onCheckedChange={(checked) => onPrivacyChange({ ...privacy, log_prompts: checked })}
          />
        </div>
      </div>
    </div>
  );
}
