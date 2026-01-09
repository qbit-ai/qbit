import { HighlightStyle, syntaxHighlighting } from "@codemirror/language";
import { EditorView } from "@codemirror/view";
import { tags as t } from "@lezer/highlight";

/**
 * Custom CodeMirror theme matching the Qbit app design
 * Colors derived from frontend/lib/theme/builtin/qbit/theme.ts
 */

// Qbit palette
const palette = {
  // Backgrounds
  bgPrimary: "#0d0f12",
  bgSecondary: "#13161b",
  bgTertiary: "#1a1e24",

  // Text
  textPrimary: "#e8eaed",
  textSecondary: "#9aa0a6",
  textMuted: "#5f6368",

  // Accent
  accent: "#5eead4",

  // Borders
  borderSubtle: "rgba(255, 255, 255, 0.06)",

  // ANSI colors for syntax highlighting
  red: "#f7768e",
  green: "#9ece6a",
  yellow: "#e0af68",
  blue: "#7aa2f7",
  magenta: "#bb9af7",
  cyan: "#7dcfff",
  white: "#c0caf5",
};

// Editor theme (UI styling)
export const qbitEditorTheme = EditorView.theme(
  {
    "&": {
      backgroundColor: palette.bgPrimary,
      color: palette.textPrimary,
      fontSize: "12px",
      fontFamily: "'JetBrains Mono', ui-monospace, monospace",
    },
    ".cm-content": {
      caretColor: palette.accent,
      fontFamily: "'JetBrains Mono', ui-monospace, monospace",
    },
    ".cm-cursor, .cm-dropCursor": {
      borderLeftColor: palette.accent,
    },
    "&.cm-focused .cm-selectionBackground, .cm-selectionBackground, .cm-content ::selection": {
      backgroundColor: palette.bgTertiary,
    },
    ".cm-panels": {
      backgroundColor: palette.bgSecondary,
      color: palette.textPrimary,
    },
    ".cm-panels.cm-panels-top": {
      borderBottom: `1px solid ${palette.borderSubtle}`,
    },
    ".cm-panels.cm-panels-bottom": {
      borderTop: `1px solid ${palette.borderSubtle}`,
    },
    ".cm-searchMatch": {
      backgroundColor: "rgba(94, 234, 212, 0.2)",
      outline: `1px solid ${palette.accent}`,
    },
    ".cm-searchMatch.cm-searchMatch-selected": {
      backgroundColor: "rgba(94, 234, 212, 0.3)",
    },
    ".cm-activeLine": {
      backgroundColor: "rgba(255, 255, 255, 0.03)",
    },
    ".cm-selectionMatch": {
      backgroundColor: "rgba(94, 234, 212, 0.15)",
    },
    "&.cm-focused .cm-matchingBracket, &.cm-focused .cm-nonmatchingBracket": {
      backgroundColor: "rgba(94, 234, 212, 0.2)",
    },
    ".cm-gutters": {
      backgroundColor: palette.bgSecondary,
      color: palette.textMuted,
      border: "none",
      borderRight: `1px solid ${palette.borderSubtle}`,
    },
    ".cm-activeLineGutter": {
      backgroundColor: "rgba(255, 255, 255, 0.03)",
      color: palette.textSecondary,
    },
    ".cm-foldPlaceholder": {
      backgroundColor: palette.bgTertiary,
      color: palette.textSecondary,
      border: "none",
    },
    ".cm-tooltip": {
      backgroundColor: palette.bgSecondary,
      border: `1px solid ${palette.borderSubtle}`,
      color: palette.textPrimary,
    },
    ".cm-tooltip .cm-tooltip-arrow:before": {
      borderTopColor: "transparent",
      borderBottomColor: "transparent",
    },
    ".cm-tooltip .cm-tooltip-arrow:after": {
      borderTopColor: palette.bgSecondary,
      borderBottomColor: palette.bgSecondary,
    },
    ".cm-tooltip-autocomplete": {
      "& > ul > li[aria-selected]": {
        backgroundColor: palette.bgTertiary,
        color: palette.textPrimary,
      },
    },
    // Vim command line panel
    ".cm-vim-panel": {
      backgroundColor: palette.bgSecondary,
      color: palette.textPrimary,
      padding: "2px 8px",
      fontFamily: "'JetBrains Mono', ui-monospace, monospace",
      fontSize: "12px",
    },
    ".cm-vim-panel input": {
      backgroundColor: "transparent",
      color: palette.textPrimary,
      border: "none",
      outline: "none",
      fontFamily: "inherit",
      fontSize: "inherit",
    },
  },
  { dark: true }
);

// Syntax highlighting
export const qbitHighlightStyle = HighlightStyle.define([
  { tag: t.comment, color: palette.textMuted, fontStyle: "italic" },
  { tag: t.lineComment, color: palette.textMuted, fontStyle: "italic" },
  { tag: t.blockComment, color: palette.textMuted, fontStyle: "italic" },
  { tag: t.docComment, color: palette.textMuted, fontStyle: "italic" },

  { tag: t.keyword, color: palette.magenta },
  { tag: t.controlKeyword, color: palette.magenta },
  { tag: t.operatorKeyword, color: palette.magenta },
  { tag: t.definitionKeyword, color: palette.magenta },
  { tag: t.moduleKeyword, color: palette.magenta },

  { tag: t.operator, color: palette.cyan },
  { tag: t.derefOperator, color: palette.textPrimary },
  { tag: t.arithmeticOperator, color: palette.cyan },
  { tag: t.logicOperator, color: palette.cyan },
  { tag: t.bitwiseOperator, color: palette.cyan },
  { tag: t.compareOperator, color: palette.cyan },

  { tag: t.string, color: palette.green },
  { tag: t.special(t.string), color: palette.green },
  { tag: t.regexp, color: palette.yellow },

  { tag: t.number, color: palette.yellow },
  { tag: t.integer, color: palette.yellow },
  { tag: t.float, color: palette.yellow },

  { tag: t.bool, color: palette.yellow },
  { tag: t.null, color: palette.yellow },

  { tag: t.variableName, color: palette.textPrimary },
  { tag: t.definition(t.variableName), color: palette.blue },
  { tag: t.function(t.variableName), color: palette.blue },
  { tag: t.propertyName, color: palette.accent },
  { tag: t.definition(t.propertyName), color: palette.accent },

  { tag: t.typeName, color: palette.cyan },
  { tag: t.className, color: palette.cyan },
  { tag: t.namespace, color: palette.cyan },
  { tag: t.macroName, color: palette.cyan },
  { tag: t.labelName, color: palette.cyan },

  { tag: t.attributeName, color: palette.accent },
  { tag: t.attributeValue, color: palette.green },

  { tag: t.meta, color: palette.textSecondary },
  { tag: t.annotation, color: palette.yellow },

  { tag: t.tagName, color: palette.red },
  { tag: t.angleBracket, color: palette.textSecondary },

  { tag: t.punctuation, color: palette.textSecondary },
  { tag: t.separator, color: palette.textSecondary },
  { tag: t.bracket, color: palette.textSecondary },
  { tag: t.paren, color: palette.textSecondary },
  { tag: t.squareBracket, color: palette.textSecondary },
  { tag: t.brace, color: palette.textSecondary },

  { tag: t.heading, color: palette.blue, fontWeight: "bold" },
  { tag: t.heading1, color: palette.blue, fontWeight: "bold" },
  { tag: t.heading2, color: palette.blue, fontWeight: "bold" },
  { tag: t.heading3, color: palette.blue, fontWeight: "bold" },

  { tag: t.link, color: palette.cyan, textDecoration: "underline" },
  { tag: t.url, color: palette.cyan, textDecoration: "underline" },

  { tag: t.emphasis, fontStyle: "italic" },
  { tag: t.strong, fontWeight: "bold" },
  { tag: t.strikethrough, textDecoration: "line-through" },

  { tag: t.invalid, color: palette.red },
]);

// Combined theme extension
export const qbitTheme = [qbitEditorTheme, syntaxHighlighting(qbitHighlightStyle)];
