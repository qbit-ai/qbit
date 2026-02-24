import { useCallback, useState } from "react";
import { HexColorPicker } from "react-colorful";
import { Input } from "@/components/ui/input";
import { Popover, PopoverContent, PopoverTrigger } from "@/components/ui/popover";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Slider } from "@/components/ui/slider";
import {
  type CaretSettings,
  DEFAULT_CARET_SETTINGS,
  type TerminalSettings as TerminalSettingsType,
} from "@/lib/settings";
import { CaretPreview } from "./CaretPreview";
import { ThemePicker } from "./ThemePicker";

interface TerminalSettingsProps {
  settings: TerminalSettingsType;
  onChange: (settings: TerminalSettingsType) => void;
}

export function TerminalSettings({ settings, onChange }: TerminalSettingsProps) {
  const updateField = useCallback(
    <K extends keyof TerminalSettingsType>(key: K, value: TerminalSettingsType[K]) => {
      onChange({ ...settings, [key]: value });
    },
    [settings, onChange]
  );

  // Ensure caret settings exist (backward compat with old configs)
  const caret: CaretSettings = settings.caret ?? DEFAULT_CARET_SETTINGS;

  const updateCaret = useCallback(
    <K extends keyof CaretSettings>(key: K, value: CaretSettings[K]) => {
      updateField("caret", { ...caret, [key]: value });
    },
    [caret, updateField]
  );

  const [showColorPicker, setShowColorPicker] = useState(false);

  return (
    <div className="space-y-6">
      {/* Theme */}
      <div className="space-y-2">
        <h3 className="text-sm font-medium text-foreground mb-4">Theme</h3>
        <ThemePicker />
      </div>

      {/* Divider */}
      <div className="border-t border-[var(--border-subtle)]" />

      {/* Shell */}
      <div className="space-y-2">
        <label htmlFor="terminal-shell" className="text-sm font-medium text-foreground">
          Shell
        </label>
        <Input
          id="terminal-shell"
          value={settings.shell || ""}
          onChange={(e) => updateField("shell", e.target.value || null)}
          placeholder="Auto-detect from environment"
        />
        <p className="text-xs text-muted-foreground">
          Override the default shell. Leave empty to auto-detect.
        </p>
      </div>

      {/* Font Family */}
      <div className="space-y-2">
        <label htmlFor="terminal-font-family" className="text-sm font-medium text-foreground">
          Font Family
        </label>
        <Input
          id="terminal-font-family"
          value={settings.font_family}
          onChange={(e) => updateField("font_family", e.target.value)}
          placeholder="JetBrains Mono"
        />
        <p className="text-xs text-muted-foreground">Monospace font for the terminal</p>
      </div>

      {/* Font Size */}
      <div className="space-y-2">
        <label htmlFor="terminal-font-size" className="text-sm font-medium text-foreground">
          Font Size
        </label>
        <Input
          id="terminal-font-size"
          type="number"
          min={8}
          max={32}
          value={settings.font_size}
          onChange={(e) => updateField("font_size", parseInt(e.target.value, 10) || 14)}
          className="w-24"
        />
        <p className="text-xs text-muted-foreground">Font size in pixels (8-32)</p>
      </div>

      {/* Scrollback */}
      <div className="space-y-2">
        <label htmlFor="terminal-scrollback" className="text-sm font-medium text-foreground">
          Scrollback Lines
        </label>
        <Input
          id="terminal-scrollback"
          type="number"
          min={1000}
          max={100000}
          step={1000}
          value={settings.scrollback}
          onChange={(e) => updateField("scrollback", parseInt(e.target.value, 10) || 10000)}
          className="w-32"
        />
        <p className="text-xs text-muted-foreground">
          Number of lines to keep in scrollback buffer
        </p>
      </div>

      {/* Divider */}
      <div className="border-t border-[var(--border-subtle)]" />

      {/* Caret Customization */}
      <div className="space-y-4">
        <h3 className="text-sm font-medium text-foreground">Input Caret</h3>

        {/* Preview */}
        <CaretPreview settings={caret} />

        {/* Style selector */}
        <div className="space-y-2">
          <span className="text-sm font-medium text-foreground">Style</span>
          <Select
            value={caret.style}
            onValueChange={(value: "block" | "default") => updateCaret("style", value)}
          >
            <SelectTrigger className="w-40">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="default">Default</SelectItem>
              <SelectItem value="block">Block</SelectItem>
            </SelectContent>
          </Select>
          <p className="text-xs text-muted-foreground">
            Default uses the native browser text caret. Block renders a customizable overlay.
          </p>
        </div>

        {/* Block-specific settings */}
        {caret.style === "block" && (
          <div className="space-y-4 pl-2 border-l-2 border-[var(--border-subtle)]">
            {/* Width */}
            <div className="space-y-2">
              <div className="flex items-center justify-between">
                <span className="text-sm font-medium text-foreground">Width</span>
                <span className="text-xs text-muted-foreground tabular-nums">
                  {caret.width.toFixed(1)}ch
                </span>
              </div>
              <Slider
                value={[caret.width]}
                onValueChange={([v]) => updateCaret("width", Math.round(v * 10) / 10)}
                min={0.1}
                max={3.0}
                step={0.1}
                className="w-full"
              />
              <p className="text-xs text-muted-foreground">
                Caret width in character units (0.1–3.0)
              </p>
            </div>

            {/* Color */}
            <div className="space-y-2">
              <div className="flex items-center justify-between">
                <span className="text-sm font-medium text-foreground">Color</span>
                {caret.color && (
                  <button
                    type="button"
                    className="text-xs text-accent hover:underline"
                    onClick={() => updateCaret("color", null)}
                  >
                    Reset to theme default
                  </button>
                )}
              </div>
              <div className="flex items-center gap-3">
                <Popover open={showColorPicker} onOpenChange={setShowColorPicker}>
                  <PopoverTrigger asChild>
                    <button
                      type="button"
                      className="h-8 w-8 rounded-md border border-[var(--border-subtle)] shrink-0"
                      style={{ backgroundColor: caret.color ?? "var(--foreground)" }}
                      aria-label="Pick caret color"
                    />
                  </PopoverTrigger>
                  <PopoverContent align="start" className="w-auto p-3">
                    <HexColorPicker
                      color={caret.color ?? "#ffffff"}
                      onChange={(color) => updateCaret("color", color)}
                    />
                  </PopoverContent>
                </Popover>
                <Input
                  value={caret.color ?? ""}
                  onChange={(e) => {
                    const val = e.target.value;
                    if (val === "") {
                      updateCaret("color", null);
                    } else {
                      updateCaret("color", val);
                    }
                  }}
                  placeholder="Theme default"
                  className="w-32 font-mono text-xs"
                />
              </div>
              <p className="text-xs text-muted-foreground">
                Hex color for the caret. Leave empty to use the theme foreground color.
              </p>
            </div>

            {/* Blink Speed */}
            <div className="space-y-2">
              <div className="flex items-center justify-between">
                <span className="text-sm font-medium text-foreground">Blink Speed</span>
                <span className="text-xs text-muted-foreground tabular-nums">
                  {caret.blink_speed === 0 ? "No blink" : `${caret.blink_speed}ms`}
                </span>
              </div>
              <Slider
                value={[caret.blink_speed]}
                onValueChange={([v]) => updateCaret("blink_speed", Math.round(v / 10) * 10)}
                min={0}
                max={2000}
                step={10}
                className="w-full"
              />
              <p className="text-xs text-muted-foreground">
                Blink cycle duration in milliseconds. Set to 0 to disable blinking.
              </p>
            </div>

            {/* Opacity */}
            <div className="space-y-2">
              <div className="flex items-center justify-between">
                <span className="text-sm font-medium text-foreground">Opacity</span>
                <span className="text-xs text-muted-foreground tabular-nums">
                  {Math.round(caret.opacity * 100)}%
                </span>
              </div>
              <Slider
                value={[caret.opacity]}
                onValueChange={([v]) => updateCaret("opacity", Math.round(v * 100) / 100)}
                min={0}
                max={1.0}
                step={0.01}
                className="w-full"
              />
              <p className="text-xs text-muted-foreground">Caret opacity (0%–100%)</p>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
