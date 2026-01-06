/**
 * Centralized logging utility for the frontend.
 *
 * Logs are written to ~/.qbit/frontend.log via Tauri IPC,
 * and also output to the browser console for development.
 *
 * API is compatible with console.* for easy migration:
 *   logger.info("message", data)  // like console.log
 *   logger.error("failed:", error) // like console.error
 */

import { invoke } from "@tauri-apps/api/core";

type LogLevel = "debug" | "info" | "warn" | "error";

/**
 * Check if we're running inside Tauri.
 * In Tauri v2, window.__TAURI_INTERNALS__ is defined.
 */
function isTauri(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

// Guard to prevent duplicate initialization logs (React StrictMode can cause double execution)
let loggerInitialized = false;

// Log initialization message when module loads
// Use delay to ensure Tauri IPC is fully ready
setTimeout(() => {
  if (!loggerInitialized && isTauri()) {
    loggerInitialized = true;
    invoke("write_frontend_log", {
      level: "debug",
      message: "[logger] Frontend logger initialized",
      context: null,
    }).catch(() => {
      // Silently ignore init failures - logging will still work via console
    });
  }
}, 500);

/**
 * Format arguments into a single string for file logging
 */
function formatArgs(args: unknown[]): string {
  return args
    .map((arg) => {
      if (typeof arg === "string") return arg;
      if (arg instanceof Error) return `${arg.message}\n${arg.stack ?? ""}`;
      try {
        return JSON.stringify(arg, null, 2);
      } catch {
        return String(arg);
      }
    })
    .join(" ");
}

/**
 * Write a log entry to the backend log file
 */
async function writeLog(level: LogLevel, args: unknown[]): Promise<void> {
  // Always log to console
  switch (level) {
    case "debug":
      console.debug(...args);
      break;
    case "info":
      console.info(...args);
      break;
    case "warn":
      console.warn(...args);
      break;
    case "error":
      console.error(...args);
      break;
  }

  // Skip IPC in browser mode (not running in Tauri)
  if (!isTauri()) {
    return;
  }

  // Format message for file logging
  const message = formatArgs(args);

  // Write to backend log file (fire and forget, don't block on logging)
  try {
    await invoke("write_frontend_log", { level, message, context: null });
  } catch (err) {
    // Don't throw on logging failures - just log to console as fallback
    console.error("[logger] Failed to write to backend log:", err);
  }
}

/**
 * Logger interface with level-specific methods.
 * API is compatible with console.* for easy migration.
 */
export const logger = {
  /**
   * Log a debug message (verbose, for development)
   */
  debug(...args: unknown[]): void {
    writeLog("debug", args);
  },

  /**
   * Log an info message (general information)
   */
  info(...args: unknown[]): void {
    writeLog("info", args);
  },

  /**
   * Log a warning message (potential issues)
   */
  warn(...args: unknown[]): void {
    writeLog("warn", args);
  },

  /**
   * Log an error message (errors and failures)
   */
  error(...args: unknown[]): void {
    writeLog("error", args);
  },

  /**
   * Alias for info() - matches console.log behavior
   */
  log(...args: unknown[]): void {
    writeLog("info", args);
  },
};

export default logger;
