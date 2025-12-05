import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import "./index.css";

/**
 * Check if we're running inside Tauri or in a browser.
 * When running in Tauri, window.__TAURI_INTERNALS__ is defined.
 */
function isTauri(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

/**
 * Initialize the application.
 * In browser mode (not Tauri), load mocks before rendering.
 */
async function initApp(): Promise<void> {
  if (!isTauri()) {
    console.log("[App] Running in browser mode - loading Tauri IPC mocks");
    const { setupMocks } = await import("./mocks");
    setupMocks();
  }

  ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
    <React.StrictMode>
      <App />
    </React.StrictMode>
  );
}

initApp();
