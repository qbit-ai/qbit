import { sendNotification as doSendNotification } from "@tauri-apps/plugin-notification";
import { AlertCircle, Bell, CheckCircle, Terminal } from "lucide-react";
import { useState } from "react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Switch } from "@/components/ui/switch";
import type { NotificationsSettings as NotificationsSettingsType } from "@/lib/settings";
import { isPermissionGranted, requestPermission } from "@/lib/systemNotifications";

interface NotificationsSettingsProps {
  settings: NotificationsSettingsType;
  onChange: (settings: NotificationsSettingsType) => void;
}

export function NotificationsSettings({ settings, onChange }: NotificationsSettingsProps) {
  const [permissionStatus, setPermissionStatus] = useState<"unknown" | "granted" | "denied">(
    "unknown"
  );
  const [testStatus, setTestStatus] = useState<{
    agent?: "success" | "error";
    command?: "success" | "error";
  }>({});

  // Check permission status when component mounts or when enabled
  const checkPermission = async () => {
    const granted = await isPermissionGranted();
    setPermissionStatus(granted ? "granted" : "denied");
    return granted;
  };

  const handleToggle = async (checked: boolean) => {
    if (checked) {
      // Request permission when enabling
      const granted = await requestPermission();
      setPermissionStatus(granted ? "granted" : "denied");
      if (granted) {
        onChange({ ...settings, native_enabled: true });
      }
      // Don't enable if permission denied
    } else {
      onChange({ ...settings, native_enabled: false });
    }
  };

  const getSoundForTest = (): string | undefined => {
    // Use configured sound if provided and non-empty
    if (settings.sound && settings.sound.trim() !== "") {
      return settings.sound;
    }
    // Default to "Blow" on macOS, undefined on other platforms
    const isMacOS = navigator.platform.toLowerCase().includes("mac");
    return isMacOS ? "Blow" : undefined;
  };

  const handleSoundChange = (value: string) => {
    // Store empty string as null
    const soundValue = value.trim() === "" ? null : value;
    onChange({
      ...settings,
      sound: soundValue,
    });
  };

  const sendTestNotification = async (type: "agent" | "command") => {
    // Check permission first
    const granted = await checkPermission();
    if (!granted) {
      setTestStatus((prev) => ({ ...prev, [type]: "error" }));
      return;
    }

    const sound = getSoundForTest();

    try {
      if (type === "agent") {
        doSendNotification({
          title: "Agent Completed",
          body: "This is a test notification for agent completion.",
          sound,
        });
      } else {
        doSendNotification({
          title: "Command Completed",
          body: "✓ echo 'Hello, World!'",
          sound,
        });
      }
      setTestStatus((prev) => ({ ...prev, [type]: "success" }));

      // Clear status after 3 seconds
      setTimeout(() => {
        setTestStatus((prev) => ({ ...prev, [type]: undefined }));
      }, 3000);
    } catch {
      setTestStatus((prev) => ({ ...prev, [type]: "error" }));
    }
  };

  return (
    <div className="space-y-6">
      {/* Enable Notifications */}
      <div className="flex items-center justify-between">
        <div className="space-y-1">
          <label
            htmlFor="notifications-native"
            className="text-sm font-medium text-foreground cursor-pointer"
          >
            Native System Notifications
          </label>
          <p className="text-xs text-muted-foreground">
            Show OS notifications for agent and command completion when the app is in the background
          </p>
        </div>
        <Switch
          id="notifications-native"
          checked={settings.native_enabled}
          onCheckedChange={handleToggle}
        />
      </div>

      {/* Notification Sound */}
      {settings.native_enabled && (
        <div className="space-y-2">
          <label htmlFor="notifications-sound" className="text-sm font-medium text-foreground">
            Notification Sound
          </label>
          <Input
            id="notifications-sound"
            type="text"
            placeholder="Default (Blow on macOS)"
            value={settings.sound ?? ""}
            onChange={(e) => handleSoundChange(e.target.value)}
            className="max-w-md"
          />
          <p className="text-xs text-muted-foreground">
            macOS system sound names like <span className="font-mono">Ping</span> or{" "}
            <span className="font-mono">Blow</span>; leave blank to use default
          </p>
        </div>
      )}

      {/* Permission Status */}
      {settings.native_enabled && (
        <div className="flex items-center gap-2 text-sm">
          {permissionStatus === "granted" ? (
            <>
              <CheckCircle className="w-4 h-4 text-green-500" />
              <span className="text-muted-foreground">Notification permission granted</span>
            </>
          ) : permissionStatus === "denied" ? (
            <>
              <AlertCircle className="w-4 h-4 text-yellow-500" />
              <span className="text-muted-foreground">
                Permission denied. Enable in system settings.
              </span>
            </>
          ) : null}
        </div>
      )}

      {/* Test Notifications Section */}
      <div className="space-y-4 p-4 rounded-lg bg-muted border border-[var(--border-medium)]">
        <h4 className="text-sm font-medium text-accent">Test Notifications</h4>
        <p className="text-xs text-muted-foreground">
          Send test notifications to verify your system notification settings are working correctly.
        </p>

        <div className="space-y-3">
          {/* Agent Completion Test */}
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-2">
              <Bell className="w-4 h-4 text-muted-foreground" />
              <span className="text-sm text-foreground">Agent Completion</span>
              {testStatus.agent === "success" && <CheckCircle className="w-4 h-4 text-green-500" />}
              {testStatus.agent === "error" && <AlertCircle className="w-4 h-4 text-red-500" />}
            </div>
            <Button
              variant="outline"
              size="sm"
              onClick={() => sendTestNotification("agent")}
              disabled={!settings.native_enabled}
            >
              Send Test
            </Button>
          </div>

          {/* Command Completion Test */}
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-2">
              <Terminal className="w-4 h-4 text-muted-foreground" />
              <span className="text-sm text-foreground">Command Completion</span>
              {testStatus.command === "success" && (
                <CheckCircle className="w-4 h-4 text-green-500" />
              )}
              {testStatus.command === "error" && <AlertCircle className="w-4 h-4 text-red-500" />}
            </div>
            <Button
              variant="outline"
              size="sm"
              onClick={() => sendTestNotification("command")}
              disabled={!settings.native_enabled}
            >
              Send Test
            </Button>
          </div>
        </div>

        {!settings.native_enabled && (
          <p className="text-xs text-muted-foreground italic">
            Enable notifications above to test.
          </p>
        )}
      </div>
    </div>
  );
}
