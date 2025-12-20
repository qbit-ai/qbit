import path from "node:path";
import react from "@vitejs/plugin-react";
import { defineConfig } from "vitest/config";

export default defineConfig({
  plugins: [react()],
  test: {
    globals: true,
    environment: "jsdom",
    setupFiles: ["./frontend/test/setup.ts"],
    include: ["frontend/**/*.{test,spec}.{js,ts,jsx,tsx}"],
    coverage: {
      provider: "v8",
      reporter: ["text", "json", "html"],
      include: ["frontend/**/*.{ts,tsx}"],
      exclude: ["frontend/test/**", "frontend/**/*.d.ts"],
    },
  },
  resolve: {
    alias: {
      "@": path.resolve(__dirname, "./frontend"),
      // Mock Tauri APIs
      "@tauri-apps/api/event": path.resolve(__dirname, "./frontend/test/mocks/tauri-event.ts"),
    },
  },
});
