import { listen as tauriListen, type UnlistenFn } from "@tauri-apps/api/event";
import { useEffect } from "react";
import { isAiSessionInitialized, updateAiWorkspace } from "../lib/ai";
import { notify } from "../lib/notify";
import { getSettings } from "../lib/settings";
import { ptyGetForegroundProcess } from "../lib/tauri";
import { useStore } from "../store";

// In browser mode, use the mock listen function if available
declare global {
  interface Window {
    __MOCK_LISTEN__?: typeof tauriListen;
    __MOCK_BROWSER_MODE__?: boolean;
  }
}

// Use mock listen in browser mode, otherwise use real Tauri listen
const listen: typeof tauriListen = (...args) => {
  if (window.__MOCK_BROWSER_MODE__ && window.__MOCK_LISTEN__) {
    return window.__MOCK_LISTEN__(...args);
  }
  return tauriListen(...args);
};

interface TerminalOutputEvent {
  session_id: string;
  data: string;
}

interface CommandBlockEvent {
  session_id: string;
  command: string | null;
  exit_code: number | null;
  event_type: "prompt_start" | "prompt_end" | "command_start" | "command_end";
}

interface DirectoryChangedEvent {
  session_id: string;
  path: string;
}

interface SessionEndedEvent {
  sessionId: string;
}

interface AlternateScreenEvent {
  session_id: string;
  enabled: boolean;
}

// Commands that are typically fast and shouldn't trigger tab name updates
// This is a minimal fallback - the main filtering is duration-based
const FAST_COMMANDS = new Set([
  "ls",
  "pwd",
  "cd",
  "echo",
  "cat",
  "which",
  "whoami",
  "date",
  "clear",
  "exit",
  "history",
  "env",
  "printenv",
]);

// Built-in fallback list for interactive apps that need fullterm mode but don't use
// the alternate screen buffer (they want output to persist in terminal history).
// Most TUI apps are auto-detected via ANSI escape sequences - this is only for edge cases.
// Users can add additional commands via settings.terminal.fullterm_commands
const BUILTIN_FULLTERM_COMMANDS = [
  // AI coding agents - these use raw mode but not alternate screen
  "claude",
  "cc",
  "codex",
  "cdx",
  "aider",
  "cursor",
  "gemini",
];

function isFastCommand(command: string | null): boolean {
  if (!command) return true;
  const firstWord = command.trim().split(/\s+/)[0];
  return FAST_COMMANDS.has(firstWord);
}

/**
 * Extract the process name from a command string.
 * Returns just the base command (first word) without arguments.
 * Handles edge cases like sudo, env vars, and path prefixes.
 */
function extractProcessName(command: string | null): string | null {
  if (!command) return null;

  const trimmed = command.trim();
  if (!trimmed) return null;

  // Remove environment variable assignments at the start (e.g., "ENV=val command")
  const withoutEnv = trimmed.replace(/^[A-Z_][A-Z0-9_]*=\S+\s+/g, "");

  // Handle sudo/doas prefix
  const withoutSudo = withoutEnv.replace(/^(sudo|doas)\s+/, "");

  // Get the first word (the actual command)
  const firstWord = withoutSudo.split(/\s+/)[0];

  // Strip path if present (e.g., "/usr/bin/npm" -> "npm")
  const baseName = firstWord.split("/").pop() || firstWord;

  return baseName;
}

export function useTauriEvents() {
  // Get store actions directly - these are stable references from zustand
  const store = useStore;

  // biome-ignore lint/correctness/useExhaustiveDependencies: store.getState is stable zustand API
  useEffect(() => {
    const unlisteners: Promise<UnlistenFn>[] = [];
    // Track pending process detection timers per session
    const processDetectionTimers = new Map<string, NodeJS.Timeout>();

    // Merge built-in fullterm commands with user-configured ones from settings
    // Start with built-in defaults, then add user commands when settings load
    let fulltermCommands = new Set(BUILTIN_FULLTERM_COMMANDS);

    // Load settings and merge user's fullterm_commands with built-in defaults
    getSettings()
      .then((settings) => {
        const userCommands = settings.terminal.fullterm_commands ?? [];
        fulltermCommands = new Set([...BUILTIN_FULLTERM_COMMANDS, ...userCommands]);
      })
      .catch((err) => {
        console.debug("Failed to load settings for fullterm commands:", err);
      });

    // Command block events
    unlisteners.push(
      listen<CommandBlockEvent>("command_block", (event) => {
        const { session_id, command, exit_code, event_type } = event.payload;
        const state = store.getState();

        switch (event_type) {
          case "prompt_start":
            state.handlePromptStart(session_id);
            break;
          case "prompt_end":
            state.handlePromptEnd(session_id);
            break;
          case "command_start": {
            state.handleCommandStart(session_id, command);

            // Primary fullterm mode switching is handled via alternate_screen events
            // from the PTY parser detecting ANSI sequences. However, some apps
            // (like AI coding agents) don't use alternate screen buffer, so we
            // have a small fallback list for those edge cases.
            const processName = extractProcessName(command);
            if (processName && fulltermCommands.has(processName)) {
              state.setRenderMode(session_id, "fullterm");
            }

            // Skip process detection for known-fast commands
            if (isFastCommand(command)) {
              break;
            }

            // Clear any existing timer for this session
            const existingTimer = processDetectionTimers.get(session_id);
            if (existingTimer) {
              clearTimeout(existingTimer);
            }

            // Wait 300ms to verify the process is still running
            // This filters out fast commands while allowing long-running ones
            const timer = setTimeout(async () => {
              try {
                // Check if something is still running (OS verification)
                const osProcess = await ptyGetForegroundProcess(session_id);

                // If shell returned to foreground, the command finished quickly
                if (!osProcess || ["zsh", "bash", "sh", "fish"].includes(osProcess)) {
                  return; // Don't update tab name
                }

                // Command is still running - use the command name we extracted
                // This gives us "pnpm" instead of "node", "just" instead of child process
                if (processName) {
                  state.setProcessName(session_id, processName);
                }
              } catch (err) {
                // Silently ignore - process detection is best-effort
                console.debug("Failed to verify foreground process:", err);
              } finally {
                processDetectionTimers.delete(session_id);
              }
            }, 300);

            processDetectionTimers.set(session_id, timer);
            break;
          }
          case "command_end": {
            if (exit_code !== null) {
              state.handleCommandEnd(session_id, exit_code);
            }
            // Cancel any pending process detection for this session
            const timer = processDetectionTimers.get(session_id);
            if (timer) {
              clearTimeout(timer);
              processDetectionTimers.delete(session_id);
            }
            // Clear process name when command ends
            state.setProcessName(session_id, null);
            // Fallback: switch back to timeline mode if we were in fullterm mode
            // Primary switching is handled by alternate_screen events, but this
            // catches edge cases where an app crashes without sending the disable sequence
            const session = state.sessions[session_id];
            if (session?.renderMode === "fullterm") {
              state.setRenderMode(session_id, "timeline");
            }
            break;
          }
        }
      })
    );

    // Terminal output - capture for command blocks
    unlisteners.push(
      listen<TerminalOutputEvent>("terminal_output", (event) => {
        store.getState().appendOutput(event.payload.session_id, event.payload.data);
      })
    );

    // Directory changed
    unlisteners.push(
      listen<DirectoryChangedEvent>("directory_changed", async (event) => {
        const { session_id, path } = event.payload;
        store.getState().updateWorkingDirectory(session_id, path);

        // Also update the AI agent's workspace if initialized for this session
        // Pass session_id to update the session-specific AI bridge
        try {
          const initialized = await isAiSessionInitialized(session_id);
          if (initialized) {
            await updateAiWorkspace(path, session_id);
            notify.info("Workspace synced", { message: path });
          }
        } catch (error) {
          console.error("Error updating AI workspace:", error);
        }
      })
    );

    // Session ended
    unlisteners.push(
      listen<SessionEndedEvent>("session_ended", (event) => {
        store.getState().removeSession(event.payload.sessionId);
      })
    );

    // Alternate screen buffer state changes (TUI app detection)
    // This is the primary mechanism for detecting when to switch to fullterm mode
    unlisteners.push(
      listen<AlternateScreenEvent>("alternate_screen", (event) => {
        const { session_id, enabled } = event.payload;
        const state = store.getState();
        state.setRenderMode(session_id, enabled ? "fullterm" : "timeline");
      })
    );

    // Cleanup
    return () => {
      // Clear all pending timers
      for (const timer of processDetectionTimers.values()) {
        clearTimeout(timer);
      }
      processDetectionTimers.clear();

      // Unlisten from events
      for (const p of unlisteners) {
        p.then((unlisten) => unlisten());
      }
    };
  }, []);
}
