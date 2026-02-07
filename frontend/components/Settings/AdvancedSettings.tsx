import { getVersion } from "@tauri-apps/api/app";
import { useEffect, useState } from "react";
import { Input } from "@/components/ui/input";
import { Switch } from "@/components/ui/switch";
import type {
  AdvancedSettings as AdvancedSettingsType,
  PrivacySettings,
  ProxySettings,
} from "@/lib/settings";

interface AdvancedSettingsProps {
  settings: AdvancedSettingsType;
  privacy: PrivacySettings;
  proxy: ProxySettings;
  onChange: (settings: AdvancedSettingsType) => void;
  onPrivacyChange: (privacy: PrivacySettings) => void;
  onProxyChange: (proxy: ProxySettings) => void;
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
  proxy,
  onChange,
  onPrivacyChange,
  onProxyChange,
}: AdvancedSettingsProps) {
  const [version, setVersion] = useState<string>("...");

  useEffect(() => {
    if (import.meta.env.DEV) {
      setVersion("dev");
    } else {
      getVersion()
        .then(setVersion)
        .catch(() => setVersion("unknown"));
    }
  }, []);

  const logLevelOptions = [
    { value: "error", label: "Error" },
    { value: "warn", label: "Warn" },
    { value: "info", label: "Info" },
    { value: "debug", label: "Debug" },
    { value: "trace", label: "Trace" },
  ];

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

      {/* Proxy Section */}
      <div className="space-y-4 p-4 rounded-lg bg-muted border border-[var(--border-medium)]">
        <h4 className="text-sm font-medium text-accent">Proxy</h4>
        <p className="text-xs text-muted-foreground">
          Configure an HTTP/HTTPS proxy for all outbound API requests
        </p>

        <div className="space-y-2">
          <label htmlFor="proxy-url" className="text-sm text-foreground">
            Proxy URL
          </label>
          <Input
            id="proxy-url"
            type="text"
            placeholder="http://proxy:8080 or socks5://proxy:1080"
            value={proxy.url ?? ""}
            onChange={(e) =>
              onProxyChange({ ...proxy, url: e.target.value.trim() || null })
            }
          />
        </div>

        <div className="space-y-2">
          <label htmlFor="proxy-username" className="text-sm text-foreground">
            Username
          </label>
          <Input
            id="proxy-username"
            type="text"
            placeholder="Optional"
            value={proxy.username ?? ""}
            onChange={(e) =>
              onProxyChange({ ...proxy, username: e.target.value.trim() || null })
            }
          />
        </div>

        <div className="space-y-2">
          <label htmlFor="proxy-password" className="text-sm text-foreground">
            Password
          </label>
          <Input
            id="proxy-password"
            type="password"
            placeholder="Optional"
            value={proxy.password ?? ""}
            onChange={(e) =>
              onProxyChange({ ...proxy, password: e.target.value || null })
            }
          />
        </div>

        <div className="space-y-2">
          <label htmlFor="proxy-no-proxy" className="text-sm text-foreground">
            No Proxy
          </label>
          <Input
            id="proxy-no-proxy"
            type="text"
            placeholder="localhost,127.0.0.1,.internal.corp"
            value={proxy.no_proxy ?? ""}
            onChange={(e) =>
              onProxyChange({ ...proxy, no_proxy: e.target.value.trim() || null })
            }
          />
          <p className="text-xs text-muted-foreground">
            Comma-separated hosts that should bypass the proxy
          </p>
        </div>

        <div className="space-y-2">
          <label htmlFor="proxy-ca-cert-path" className="text-sm text-foreground">
            CA Certificate
          </label>
          <Input
            id="proxy-ca-cert-path"
            type="text"
            placeholder="/path/to/corporate-ca.pem"
            value={proxy.ca_cert_path ?? ""}
            onChange={(e) =>
              onProxyChange({ ...proxy, ca_cert_path: e.target.value.trim() || null })
            }
          />
          <p className="text-xs text-muted-foreground">
            Path to a PEM-encoded CA certificate file for custom TLS validation
          </p>
        </div>

        <div className="flex items-center justify-between">
          <div className="space-y-1">
            <label htmlFor="proxy-accept-invalid-certs" className="text-sm text-foreground">
              Accept Invalid Certificates
            </label>
            <p className="text-xs text-muted-foreground">
              Skip TLS verification (self-signed certs). <span className="text-red-500">WARNING: Only use in trusted dev environments</span>
            </p>
          </div>
          <Switch
            id="proxy-accept-invalid-certs"
            checked={proxy.accept_invalid_certs}
            onCheckedChange={(checked) =>
              onProxyChange({ ...proxy, accept_invalid_certs: checked })
            }
          />
        </div>
      </div>

      {/* Version */}
      <div className="pt-4 border-t border-[var(--border-medium)]">
        <div className="flex items-center justify-between">
          <span className="text-sm text-muted-foreground">Version</span>
          <span className="text-sm font-mono text-muted-foreground">{version}</span>
        </div>
      </div>
    </div>
  );
}
