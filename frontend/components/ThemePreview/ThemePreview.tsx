import { useState } from "react";
import { cn } from "@/lib/utils";
import type { QbitTheme } from "@/lib/theme/types";

interface ThemePreviewProps {
  theme: QbitTheme;
  className?: string;
  showSyntax?: boolean;
  showAnsi?: boolean;
}

/**
 * A comprehensive theme preview component that displays
 * all theme colors in a visual preview format.
 */
export function ThemePreview({
  theme,
  className,
  showSyntax = true,
  showAnsi = true,
}: ThemePreviewProps) {
  const [activeTab, setActiveTab] = useState<"ui" | "syntax" | "ansi">("ui");

  const { ui, ansi, syntax } = theme.colors;

  return (
    <div
      className={cn("rounded-lg border overflow-hidden", className)}
      style={{
        backgroundColor: ui.background,
        borderColor: ui.border,
        color: ui.foreground,
      }}
    >
      {/* Header */}
      <div
        className="px-4 py-3 border-b flex items-center justify-between"
        style={{ borderColor: ui.border }}
      >
        <div className="flex items-center gap-2">
          <div
            className="w-3 h-3 rounded-full"
            style={{ backgroundColor: ui.destructive }}
          />
          <div
            className="w-3 h-3 rounded-full"
            style={{ backgroundColor: ansi.yellow }}
          />
          <div
            className="w-3 h-3 rounded-full"
            style={{ backgroundColor: ansi.green }}
          />
        </div>
        <span className="text-sm font-medium">{theme.name}</span>
        <span
          className="text-xs px-2 py-0.5 rounded"
          style={{
            backgroundColor: ui.muted,
            color: ui.mutedForeground,
          }}
        >
          v{theme.version}
        </span>
      </div>

      {/* Tab Navigation */}
      <div
        className="flex border-b"
        style={{ borderColor: ui.border, backgroundColor: ui.card }}
      >
        {["ui", ...(showSyntax ? ["syntax"] : []), ...(showAnsi ? ["ansi"] : [])].map(
          (tab) => (
            <button
              key={tab}
              onClick={() => setActiveTab(tab as "ui" | "syntax" | "ansi")}
              className="px-4 py-2 text-sm font-medium transition-colors"
              style={{
                backgroundColor: activeTab === tab ? ui.background : "transparent",
                color: activeTab === tab ? ui.foreground : ui.mutedForeground,
                borderBottom:
                  activeTab === tab ? `2px solid ${ui.primary}` : "2px solid transparent",
              }}
            >
              {tab.charAt(0).toUpperCase() + tab.slice(1)}
            </button>
          )
        )}
      </div>

      {/* Content */}
      <div className="p-4" style={{ backgroundColor: ui.background }}>
        {activeTab === "ui" && <UIColorsPreview ui={ui} />}
        {activeTab === "syntax" && syntax && <SyntaxPreview syntax={syntax} ui={ui} />}
        {activeTab === "ansi" && <AnsiPreview ansi={ansi} ui={ui} />}
      </div>
    </div>
  );
}

function UIColorsPreview({
  ui,
}: {
  ui: QbitTheme["colors"]["ui"];
}) {
  const colorGroups = [
    {
      label: "Base",
      colors: [
        { name: "Background", value: ui.background, fg: ui.foreground },
        { name: "Foreground", value: ui.foreground, fg: ui.background },
        { name: "Card", value: ui.card, fg: ui.cardForeground },
        { name: "Popover", value: ui.popover, fg: ui.popoverForeground },
      ],
    },
    {
      label: "Actions",
      colors: [
        { name: "Primary", value: ui.primary, fg: ui.primaryForeground },
        { name: "Secondary", value: ui.secondary, fg: ui.secondaryForeground },
        { name: "Accent", value: ui.accent, fg: ui.accentForeground },
        { name: "Destructive", value: ui.destructive, fg: ui.background },
      ],
    },
    {
      label: "Utility",
      colors: [
        { name: "Muted", value: ui.muted, fg: ui.mutedForeground },
        { name: "Border", value: ui.border, fg: ui.foreground },
        { name: "Input", value: ui.input, fg: ui.foreground },
        { name: "Ring", value: ui.ring, fg: ui.background },
      ],
    },
    {
      label: "Sidebar",
      colors: [
        { name: "Sidebar", value: ui.sidebar, fg: ui.sidebarForeground },
        { name: "Primary", value: ui.sidebarPrimary, fg: ui.sidebarPrimaryForeground },
        { name: "Accent", value: ui.sidebarAccent, fg: ui.sidebarAccentForeground },
        { name: "Border", value: ui.sidebarBorder, fg: ui.sidebarForeground },
      ],
    },
  ];

  return (
    <div className="space-y-4">
      {colorGroups.map((group) => (
        <div key={group.label}>
          <h4
            className="text-xs font-semibold uppercase tracking-wider mb-2"
            style={{ color: ui.mutedForeground }}
          >
            {group.label}
          </h4>
          <div className="grid grid-cols-4 gap-2">
            {group.colors.map((color) => (
              <div
                key={color.name}
                className="rounded-md p-2 text-center"
                style={{
                  backgroundColor: color.value,
                  color: color.fg,
                }}
              >
                <div className="text-xs font-medium truncate">{color.name}</div>
                <div className="text-[10px] opacity-70 font-mono">{color.value}</div>
              </div>
            ))}
          </div>
        </div>
      ))}

      {/* Chart Colors */}
      {ui.chart && (
        <div>
          <h4
            className="text-xs font-semibold uppercase tracking-wider mb-2"
            style={{ color: ui.mutedForeground }}
          >
            Chart
          </h4>
          <div className="flex gap-1 h-8 rounded-md overflow-hidden">
            {[ui.chart.c1, ui.chart.c2, ui.chart.c3, ui.chart.c4, ui.chart.c5].map(
              (color, i) => (
                <div
                  key={i}
                  className="flex-1"
                  style={{ backgroundColor: color }}
                  title={`Chart ${i + 1}: ${color}`}
                />
              )
            )}
          </div>
        </div>
      )}

      {/* Sample Components */}
      <div>
        <h4
          className="text-xs font-semibold uppercase tracking-wider mb-2"
          style={{ color: ui.mutedForeground }}
        >
          Components
        </h4>
        <div className="flex flex-wrap gap-2">
          <button
            className="px-3 py-1.5 rounded-md text-sm font-medium"
            style={{
              backgroundColor: ui.primary,
              color: ui.primaryForeground,
            }}
          >
            Primary
          </button>
          <button
            className="px-3 py-1.5 rounded-md text-sm font-medium"
            style={{
              backgroundColor: ui.secondary,
              color: ui.secondaryForeground,
            }}
          >
            Secondary
          </button>
          <button
            className="px-3 py-1.5 rounded-md text-sm font-medium"
            style={{
              backgroundColor: ui.destructive,
              color: ui.background,
            }}
          >
            Destructive
          </button>
          <input
            type="text"
            placeholder="Input field"
            className="px-3 py-1.5 rounded-md text-sm border"
            style={{
              backgroundColor: ui.background,
              borderColor: ui.input,
              color: ui.foreground,
            }}
          />
        </div>
      </div>
    </div>
  );
}

function SyntaxPreview({
  syntax,
  ui,
}: {
  syntax: NonNullable<QbitTheme["colors"]["syntax"]>;
  ui: QbitTheme["colors"]["ui"];
}) {
  // Sample code with syntax highlighting
  const codeLines = [
    { tokens: [
      { text: "function", color: syntax.keyword },
      { text: " ", color: ui.foreground },
      { text: "greet", color: syntax.function },
      { text: "(", color: syntax.punctuation },
      { text: "name", color: syntax.variable },
      { text: ":", color: syntax.punctuation },
      { text: " string", color: syntax.type },
      { text: ")", color: syntax.punctuation },
      { text: " {", color: syntax.punctuation },
    ]},
    { tokens: [
      { text: "  ", color: ui.foreground },
      { text: "// Greeting message", color: syntax.comment },
    ]},
    { tokens: [
      { text: "  ", color: ui.foreground },
      { text: "const", color: syntax.keyword },
      { text: " ", color: ui.foreground },
      { text: "message", color: syntax.variable },
      { text: " = ", color: syntax.operator },
      { text: "`Hello, ", color: syntax.string },
      { text: "${", color: syntax.punctuation },
      { text: "name", color: syntax.variable },
      { text: "}", color: syntax.punctuation },
      { text: "!`", color: syntax.string },
      { text: ";", color: syntax.punctuation },
    ]},
    { tokens: [
      { text: "  ", color: ui.foreground },
      { text: "return", color: syntax.keyword },
      { text: " ", color: ui.foreground },
      { text: "message", color: syntax.variable },
      { text: ";", color: syntax.punctuation },
    ]},
    { tokens: [
      { text: "}", color: syntax.punctuation },
    ]},
  ];

  return (
    <div className="space-y-4">
      {/* Code Preview */}
      <div
        className="rounded-md p-4 font-mono text-sm"
        style={{ backgroundColor: ui.card }}
      >
        {codeLines.map((line, i) => (
          <div key={i}>
            {line.tokens.map((token, j) => (
              <span key={j} style={{ color: token.color }}>
                {token.text}
              </span>
            ))}
          </div>
        ))}
      </div>

      {/* Token Legend */}
      <div className="grid grid-cols-3 gap-2">
        {Object.entries(syntax).map(([name, color]) => (
          <div key={name} className="flex items-center gap-2">
            <div
              className="w-3 h-3 rounded-sm"
              style={{ backgroundColor: color }}
            />
            <span className="text-xs" style={{ color: ui.mutedForeground }}>
              {name}
            </span>
          </div>
        ))}
      </div>
    </div>
  );
}

function AnsiPreview({
  ansi,
  ui,
}: {
  ansi: QbitTheme["colors"]["ansi"];
  ui: QbitTheme["colors"]["ui"];
}) {
  const normalColors = [
    { name: "Black", value: ansi.black },
    { name: "Red", value: ansi.red },
    { name: "Green", value: ansi.green },
    { name: "Yellow", value: ansi.yellow },
    { name: "Blue", value: ansi.blue },
    { name: "Magenta", value: ansi.magenta },
    { name: "Cyan", value: ansi.cyan },
    { name: "White", value: ansi.white },
  ];

  const brightColors = [
    { name: "Bright Black", value: ansi.brightBlack },
    { name: "Bright Red", value: ansi.brightRed },
    { name: "Bright Green", value: ansi.brightGreen },
    { name: "Bright Yellow", value: ansi.brightYellow },
    { name: "Bright Blue", value: ansi.brightBlue },
    { name: "Bright Magenta", value: ansi.brightMagenta },
    { name: "Bright Cyan", value: ansi.brightCyan },
    { name: "Bright White", value: ansi.brightWhite },
  ];

  return (
    <div className="space-y-4">
      {/* Terminal Preview */}
      <div
        className="rounded-md p-4 font-mono text-sm"
        style={{
          backgroundColor: ansi.defaultBg,
          color: ansi.defaultFg,
        }}
      >
        <div>
          <span style={{ color: ansi.green }}>user@qbit</span>
          <span style={{ color: ansi.white }}>:</span>
          <span style={{ color: ansi.blue }}>~/project</span>
          <span style={{ color: ansi.white }}>$ </span>
          <span style={{ color: ansi.yellow }}>npm run build</span>
        </div>
        <div style={{ color: ansi.cyan }}>Building project...</div>
        <div style={{ color: ansi.green }}>✓ Build completed successfully</div>
        <div style={{ color: ansi.red }}>✗ 2 warnings found</div>
      </div>

      {/* Normal Colors */}
      <div>
        <h4
          className="text-xs font-semibold uppercase tracking-wider mb-2"
          style={{ color: ui.mutedForeground }}
        >
          Normal
        </h4>
        <div className="grid grid-cols-8 gap-1">
          {normalColors.map((color) => (
            <div
              key={color.name}
              className="aspect-square rounded-md flex items-center justify-center"
              style={{ backgroundColor: color.value }}
              title={`${color.name}: ${color.value}`}
            />
          ))}
        </div>
      </div>

      {/* Bright Colors */}
      <div>
        <h4
          className="text-xs font-semibold uppercase tracking-wider mb-2"
          style={{ color: ui.mutedForeground }}
        >
          Bright
        </h4>
        <div className="grid grid-cols-8 gap-1">
          {brightColors.map((color) => (
            <div
              key={color.name}
              className="aspect-square rounded-md flex items-center justify-center"
              style={{ backgroundColor: color.value }}
              title={`${color.name}: ${color.value}`}
            />
          ))}
        </div>
      </div>
    </div>
  );
}

export default ThemePreview;
