import path from "node:path";
import tailwindcss from "@tailwindcss/vite";
import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";

// @ts-expect-error process is a nodejs global
// Use 127.0.0.1 explicitly to avoid IPv6 localhost issues in Node 18+
const host = process.env.TAURI_DEV_HOST || "127.0.0.1";

// https://vite.dev/config/
export default defineConfig(async () => ({
  plugins: [
    react({
      babel: {
        plugins: [["babel-plugin-react-compiler", { target: "19" }]],
      },
    }),
    tailwindcss(),
  ],

  resolve: {
    alias: {
      "@": path.resolve(__dirname, "./frontend"),
    },
  },

  // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
  //
  // 1. prevent Vite from obscuring rust errors
  clearScreen: false,
  // 2. tauri expects a fixed port, fail if that port is not available
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      // 3. tell Vite to ignore watching `backend`
      ignored: ["**/backend/**"],
    },
  },

  // Build optimizations: manual chunk splitting for better caching
  // Each chunk loads independently, so unchanged vendor code stays cached
  build: {
    rollupOptions: {
      output: {
        manualChunks: {
          // React core - rarely changes, caches well
          "react-vendor": ["react", "react-dom", "react/jsx-runtime"],
          // State management
          state: ["zustand", "immer"],
          // Terminal - xterm.js and addons
          xterm: [
            "@xterm/xterm",
            "@xterm/addon-fit",
            "@xterm/addon-webgl",
            "@xterm/addon-web-links",
            "@xterm/addon-serialize",
          ],
          // Markdown rendering - large (~170KB for syntax highlighter)
          markdown: ["react-markdown", "react-syntax-highlighter", "remark-gfm"],
          // Radix UI primitives - used across many components
          radix: [
            "@radix-ui/react-dialog",
            "@radix-ui/react-dropdown-menu",
            "@radix-ui/react-scroll-area",
            "@radix-ui/react-tabs",
            "@radix-ui/react-tooltip",
            "@radix-ui/react-popover",
            "@radix-ui/react-select",
            "@radix-ui/react-switch",
            "@radix-ui/react-checkbox",
            "@radix-ui/react-slot",
          ],
          // CodeMirror - loaded on demand by FileEditorSidebar
          // Note: Individual language packages are dynamically imported
          codemirror: [
            "@codemirror/state",
            "@codemirror/view",
            "@codemirror/commands",
            "@codemirror/language",
            "@codemirror/search",
          ],
        },
      },
    },
  },
}));
