# Frontend Bundle Optimization Review

## Summary

This review identifies bundle size and loading performance issues in the Qbit frontend. The codebase already implements some good practices (lazy loading for Settings, GitPanel, etc.) but has several opportunities for improvement in dependency management, code splitting, and tree shaking.

**Key Findings:**
- Heavy dependencies loaded at startup (react-syntax-highlighter, CodeMirror languages)
- Icon libraries imported individually (good for lucide-react, suboptimal for react-icons)
- No Vite bundle splitting configuration
- console.log statements scattered across production code
- Large mocks.ts file included in production builds

---

## Issue 1: react-syntax-highlighter Not Lazily Loaded

**Priority: HIGH**

`react-syntax-highlighter` with Prism is a heavy dependency (~170KB gzipped with all languages). It's imported synchronously in components that could be lazy loaded.

### Files Affected:
- `/frontend/components/Markdown/Markdown.tsx:10-11`
- `/frontend/components/ToolCallDisplay/ToolDetailsModal.tsx:15`

```typescript
// Markdown.tsx lines 10-11
import { Prism as SyntaxHighlighter } from "react-syntax-highlighter";
import { oneDark } from "react-syntax-highlighter/dist/esm/styles/prism";
```

### Recommendation:
1. The `Markdown` component is used during streaming AI responses, so it needs the highlighter. However, consider using a lighter alternative like `shiki` or `highlight.js` for code highlighting.

2. For `ToolDetailsModal`, since it's only shown on user interaction, consider dynamic import:

```typescript
const SyntaxHighlighter = lazy(() =>
  import("react-syntax-highlighter").then(mod => ({
    default: mod.Prism
  }))
);
```

---

## Issue 2: CodeMirror Language Packages Loaded Eagerly

**Priority: HIGH**

All 13+ CodeMirror language packages are imported at the top level of `FileEditorSidebarPanel.tsx`, even though this component is lazy loaded. This adds significant weight.

### File Affected:
- `/frontend/components/FileEditorSidebar/FileEditorSidebarPanel.tsx:1-15`

```typescript
import { cpp } from "@codemirror/lang-cpp";
import { css } from "@codemirror/lang-css";
import { go } from "@codemirror/lang-go";
import { html } from "@codemirror/lang-html";
import { java } from "@codemirror/lang-java";
import { javascript } from "@codemirror/lang-javascript";
import { json } from "@codemirror/lang-json";
import { markdown } from "@codemirror/lang-markdown";
import { python } from "@codemirror/lang-python";
import { rust } from "@codemirror/lang-rust";
import { sql } from "@codemirror/lang-sql";
import { xml } from "@codemirror/lang-xml";
import { yaml } from "@codemirror/lang-yaml";
```

### Recommendation:
Dynamically import language extensions based on the file being edited:

```typescript
async function getLanguageExtension(language?: string): Promise<Extension | null> {
  switch (language) {
    case "typescript":
    case "javascript":
      const { javascript } = await import("@codemirror/lang-javascript");
      return javascript({ jsx: true, typescript: language === "typescript" });
    case "python":
      const { python } = await import("@codemirror/lang-python");
      return python();
    // ... other languages
    default:
      return null;
  }
}
```

---

## Issue 3: react-icons Imported from Subpackages (Good) but Could Be Better

**Priority: MEDIUM**

The codebase correctly imports from subpackages (`react-icons/si`, `react-icons/fa`) rather than the main barrel export. However, there are 25+ icons imported from `react-icons/si` in one file.

### File Affected:
- `/frontend/lib/file-icons.tsx:21-47`

```typescript
import { FaJava } from "react-icons/fa";
import {
  SiC,
  SiCplusplus,
  SiCss3,
  // ... 20+ more icons
} from "react-icons/si";
```

### Recommendation:
This is fine for tree shaking with Vite, but if bundle size is a concern, consider:
1. Using lucide-react equivalents where available (already used elsewhere)
2. Creating a custom icon set using SVGs for file type icons

---

## Issue 4: Missing Vite Build Configuration for Chunk Splitting

**Priority: HIGH**

The `vite.config.ts` has minimal configuration and no manual chunk splitting for vendor dependencies.

### File Affected:
- `/vite.config.ts` (entire file)

### Current Config:
```typescript
export default defineConfig(async () => ({
  plugins: [react(), tailwindcss()],
  resolve: { alias: { "@": path.resolve(__dirname, "./frontend") } },
  clearScreen: false,
  server: { /* ... */ },
}));
```

### Recommendation:
Add build configuration with manual chunks:

```typescript
export default defineConfig(async () => ({
  plugins: [react(), tailwindcss()],
  resolve: { alias: { "@": path.resolve(__dirname, "./frontend") } },
  build: {
    rollupOptions: {
      output: {
        manualChunks: {
          // Core React
          'react-vendor': ['react', 'react-dom'],
          // State management
          'state': ['zustand', 'immer'],
          // Terminal (heavy, but needed early)
          'xterm': ['@xterm/xterm', '@xterm/addon-fit', '@xterm/addon-webgl', '@xterm/addon-web-links'],
          // CodeMirror core (lazy load languages separately)
          'codemirror': ['@uiw/react-codemirror', '@codemirror/state', '@codemirror/view'],
          // Markdown rendering
          'markdown': ['react-markdown', 'remark-gfm', 'react-syntax-highlighter'],
          // UI primitives
          'radix': [
            '@radix-ui/react-dialog',
            '@radix-ui/react-dropdown-menu',
            '@radix-ui/react-popover',
            '@radix-ui/react-scroll-area',
            '@radix-ui/react-tabs',
            '@radix-ui/react-tooltip',
          ],
        },
      },
    },
  },
  // ... rest of config
}));
```

---

## Issue 5: console.log Statements in Production Code

**Priority: MEDIUM**

Multiple `console.log` statements exist in production code, particularly in `mocks.ts` and some components.

### Files Affected:
- `/frontend/mocks.ts:71,87,97,109,908,912,931,1022,1794,1801,1853`
- `/frontend/main.tsx:23`
- `/frontend/components/HomeView/HomeView.tsx:439`
- `/frontend/components/UnifiedInput/UnifiedInput.tsx:626,633,703,718,720`

### Recommendation:
1. Use the existing `logger` abstraction consistently:
```typescript
// Instead of:
console.log("[DEBUG] handleSubmit called", {...});
// Use:
logger.debug("[DEBUG] handleSubmit called", {...});
```

2. Add Vite define to strip logs in production:
```typescript
// vite.config.ts
define: {
  'import.meta.env.DEV': JSON.stringify(process.env.NODE_ENV !== 'production'),
},
```

3. The logger already checks for DEV mode, but ensure all console statements use it.

---

## Issue 6: mocks.ts Potentially Included in Production

**Priority: HIGH**

The `mocks.ts` file is ~1800 lines and should only be used in browser development mode, but its import structure could cause it to be included in the bundle.

### File Affected:
- `/frontend/mocks.ts` (entire file - 1800+ lines)
- `/frontend/main.tsx:24`
- `/frontend/App.tsx:71`

### Current Pattern:
```typescript
// main.tsx
if (!isTauri()) {
  const { setupMocks } = await import("./mocks");
  setupMocks();
}

// App.tsx
import { isMockBrowserMode } from "./mocks";
```

### Issue:
The `isMockBrowserMode` function is imported directly from `mocks.ts` in `App.tsx`, which may cause bundlers to include the entire file.

### Recommendation:
Extract the browser mode check to a separate tiny module:

```typescript
// frontend/lib/isMockBrowser.ts
export function isMockBrowserMode(): boolean {
  return typeof window !== "undefined" && window.__MOCK_BROWSER_MODE__ === true;
}

// Then update imports:
import { isMockBrowserMode } from "@/lib/isMockBrowser";
```

---

## Issue 7: Settings Dialog Loads All Tab Components Eagerly

**Priority: MEDIUM**

While `SettingsDialog` itself is lazy loaded, it immediately imports all settings tab components at the top level.

### File Affected:
- `/frontend/components/Settings/index.tsx:26-33`

```typescript
import { AdvancedSettings } from "./AdvancedSettings";
import { AgentSettings } from "./AgentSettings";
import { AiSettings } from "./AiSettings";
import { CodebasesSettings } from "./CodebasesSettings";
import { EditorSettings } from "./EditorSettings";
import { NotificationsSettings } from "./NotificationsSettings";
import { ProviderSettings } from "./ProviderSettings";
import { TerminalSettings } from "./TerminalSettings";
```

### Recommendation:
Lazy load each settings tab:

```typescript
const AdvancedSettings = lazy(() => import("./AdvancedSettings").then(m => ({ default: m.AdvancedSettings })));
const AgentSettings = lazy(() => import("./AgentSettings").then(m => ({ default: m.AgentSettings })));
// ... etc
```

---

## Issue 8: ComponentTestbed Always Lazy Loaded (Good) But Contains Heavy Imports

**Priority: LOW**

`ComponentTestbed` is correctly lazy loaded but imports many UI components. This is fine since it's a dev/test page.

### File Affected:
- `/frontend/pages/ComponentTestbed.tsx:17-71`

### Assessment:
This is acceptable since:
1. It's already lazy loaded in `App.tsx`
2. It's only accessed via command palette navigation
3. It's primarily for development/testing

No action needed.

---

## Issue 9: date-fns Full Import

**Priority: LOW**

Only `formatDistanceToNow` is used from date-fns, which is fine for tree shaking, but verify bundle includes only this function.

### File Affected:
- `/frontend/components/SessionBrowser/SessionBrowser.tsx:1`

```typescript
import { formatDistanceToNow } from "date-fns";
```

### Assessment:
Modern bundlers tree-shake date-fns well. No action needed, but monitor bundle analyzer output.

---

## Issue 10: Barrel Exports Are Minimal (Good)

**Priority: INFO**

The codebase uses focused barrel exports that re-export only what's needed:

```typescript
// Example: frontend/components/AgentChat/index.ts
export { AgentMessage } from "./AgentMessage";
export { ToolApprovalDialog } from "./ToolApprovalDialog";
```

This pattern is tree-shaking friendly. No issues found.

---

## Issue 11: ansi-to-react May Have Lighter Alternatives

**Priority: LOW**

`ansi-to-react` is used for terminal output rendering. Consider if a lighter alternative exists or if custom ANSI parsing could work.

### Files Affected:
- `/frontend/components/ToolCallDisplay/ToolDetailsModal.tsx:1`

### Current Usage:
```typescript
import Ansi from "ansi-to-react";
// ...
<Ansi useClasses>{resultString}</Ansi>
```

### Assessment:
The package is reasonably sized. Keep as-is unless bundle analysis shows it's a significant contributor.

---

## Recommendations Summary

| Priority | Issue | Estimated Impact | Effort |
|----------|-------|------------------|--------|
| HIGH | Add Vite manual chunks config | 15-25% faster initial load | Low |
| HIGH | Dynamic import CodeMirror languages | 50-100KB reduction | Medium |
| HIGH | Extract isMockBrowserMode | Exclude 50KB+ mocks | Low |
| HIGH | Lazy load SyntaxHighlighter | 170KB reduction (modal) | Medium |
| MEDIUM | Use logger consistently | Better debug control | Low |
| MEDIUM | Lazy load Settings tabs | 10-20KB deferred | Medium |
| LOW | Monitor date-fns tree-shaking | Verify in bundle | None |

---

## Next Steps

1. **Analyze current bundle:** Run `npx vite-bundle-visualizer` to see actual sizes
2. **Implement quick wins:** Items 3, 5, and 6 are low effort, high impact
3. **Profile initial load:** Use Chrome DevTools to measure actual load times
4. **Consider alternatives:** Evaluate lighter alternatives to react-syntax-highlighter

---

## Tools for Further Analysis

```bash
# Install bundle analyzer
pnpm add -D rollup-plugin-visualizer

# Add to vite.config.ts
import { visualizer } from 'rollup-plugin-visualizer';

plugins: [
  // ... existing plugins
  visualizer({ open: true, gzipSize: true }),
]

# Build and analyze
pnpm build
```
