import { listen as tauriListen, type UnlistenFn } from "@tauri-apps/api/event";
import { useEffect } from "react";
import { isAiSessionInitialized, updateAiWorkspace } from "@/lib/ai";
import { addCommandHistory } from "@/lib/history";
import { logger } from "@/lib/logger";
import { notify } from "@/lib/notify";
import { getSettings } from "@/lib/settings";
import { getGitBranch, gitStatus, ptyGetForegroundProcess } from "@/lib/tauri";
import { liveTerminalManager, virtualTerminalManager } from "@/lib/terminal";
import { useStore } from "@/store";

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

interface VirtualEnvChangedEvent {
  session_id: string;
  name: string | null;
}

interface SessionEndedEvent {
  sessionId: string;
}

interface AlternateScreenEvent {
  session_id: string;
  enabled: boolean;
}

const PROCESS_DETECTION_DELAY_MS = 300;
const SHELL_PROCESSES = new Set(["zsh", "bash", "sh", "fish"]);

// Interval for periodic git status refresh (in milliseconds)
const GIT_STATUS_POLL_INTERVAL_MS = 5000;

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

function shouldRefreshGitInfo(command: string | null): boolean {
  if (!command) return false;
  const trimmed = command.trim();
  if (!trimmed) return false;

  // Keep this narrow: only commands that are expected to change HEAD.
  // We intentionally don't refresh on every git command to avoid extra IPC.
  return (
    /(?:^|\s|&&|\|\||;|\()git\s+(?:checkout|switch)\b/.test(trimmed) ||
    /(?:^|\s|&&|\|\||;|\()gh\s+pr\s+checkout\b/.test(trimmed)
  );
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
    const processDetectionTimers = new Map<string, ReturnType<typeof setTimeout>>();
    // Track whether current command used alternate screen (TUI apps)
    // Used to skip output serialization for fullterm apps
    const usedAlternateScreen = new Map<string, boolean>();

    // Prevent out-of-order git refreshes per session
    const gitRefreshSeq = new Map<string, number>();
    // Track in-flight git refresh requests to avoid duplicate requests
    const gitRefreshInFlight = new Set<string>();

    // Some PTY integrations send `command_end` with `command: null`.
    // Track the last command seen on `command_start` so we can still
    // run post-command actions (like git refresh) reliably.
    const lastStartedCommand = new Map<string, string | null>();

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
        logger.debug("Failed to load settings for fullterm commands:", err);
      });

    function refreshGitInfo(sessionId: string, cwd: string) {
      // Skip if a request is already in flight for this session
      // This prevents duplicate requests from polling while a request is pending
      if (gitRefreshInFlight.has(sessionId)) {
        return;
      }

      const state = store.getState();
      const nextSeq = (gitRefreshSeq.get(sessionId) ?? 0) + 1;
      gitRefreshSeq.set(sessionId, nextSeq);

      const isLatest = () => (gitRefreshSeq.get(sessionId) ?? 0) === nextSeq;

      gitRefreshInFlight.add(sessionId);
      state.setGitStatusLoading(sessionId, true);
      void (async () => {
        try {
          const [branch, status] = await Promise.all([getGitBranch(cwd), gitStatus(cwd)]);

          // If a newer refresh started while we were awaiting, ignore this result.
          if (!isLatest()) return;

          state.updateGitBranch(sessionId, branch);
          state.setGitStatus(sessionId, status);
        } catch {
          if (!isLatest()) return;
          state.updateGitBranch(sessionId, null);
          state.setGitStatus(sessionId, null);
        } finally {
          gitRefreshInFlight.delete(sessionId);
          if (isLatest()) {
            state.setGitStatusLoading(sessionId, false);
          }
        }
      })();
    }

    function clearProcessDetectionTimer(sessionId: string) {
      const timer = processDetectionTimers.get(sessionId);
      if (timer) {
        clearTimeout(timer);
        processDetectionTimers.delete(sessionId);
      }
    }

    // Command block events
    unlisteners.push(
      listen<CommandBlockEvent>("command_block", (event) => {
        const { session_id, command, exit_code, event_type } = event.payload;
        const state = store.getState();

        switch (event_type) {
          case "prompt_start": {
            // Capture pending output BEFORE handlePromptStart clears it
            const pendingOutput = state.pendingCommand[session_id]?.output;
            const pendingCommand = state.pendingCommand[session_id]?.command;

            // Dispose VirtualTerminal for this command (it's no longer needed)
            virtualTerminalManager.dispose(session_id);
            // Scroll live terminal to bottom and dispose
            liveTerminalManager.scrollToBottom(session_id);
            liveTerminalManager.dispose(session_id);

            state.handlePromptStart(session_id);

            // Command lifecycle has completed; clear last-started command.
            lastStartedCommand.delete(session_id);
            // Switch back to timeline mode when shell is ready for next command
            // This handles both alternate screen apps and fallback list apps
            // (moved from command_end to prevent premature switching for apps like codex/cdx)
            const session = state.sessions[session_id];
            if (session?.renderMode) {
              logger.debug("[fullterm] prompt_start: renderMode =", session.renderMode);
            }
            if (session?.renderMode === "fullterm") {
              // Log the output that would otherwise be lost when switching from fullterm
              if (pendingOutput) {
                logger.debug("[fullterm] Captured output from fullterm command:", pendingCommand);
                logger.debug("[fullterm] Output:", pendingOutput);
              }
              logger.debug("[fullterm] Switching back to timeline mode");
              state.setRenderMode(session_id, "timeline");
            }
            break;
          }
          case "prompt_end":
            state.handlePromptEnd(session_id);
            break;
          case "command_start": {
            state.handleCommandStart(session_id, command);

            lastStartedCommand.set(session_id, command);

            // Reset alternate screen tracking for new command
            usedAlternateScreen.set(session_id, false);

            // Create a VirtualTerminal for processing ANSI sequences in this command's output
            // This enables proper rendering of spinners, progress bars, and other animations
            virtualTerminalManager.create(session_id);
            // Create live terminal for embedded xterm.js display
            liveTerminalManager.getOrCreate(session_id);

            // Primary fullterm mode switching is handled via alternate_screen events
            // from the PTY parser detecting ANSI sequences. However, some apps
            // (like AI coding agents) don't use alternate screen buffer, so we
            // have a small fallback list for those edge cases.
            const processName = extractProcessName(command);
            if (processName && fulltermCommands.has(processName)) {
              logger.debug("[fullterm] Switching to fullterm mode for", {
                session_id,
                processName,
              });
              state.setRenderMode(session_id, "fullterm");
            }

            // Skip process detection for known-fast commands
            if (isFastCommand(command)) {
              break;
            }

            clearProcessDetectionTimer(session_id);

            // Wait 300ms to verify the process is still running
            // This filters out fast commands while allowing long-running ones
            const timer = setTimeout(async () => {
              try {
                // Check if something is still running (OS verification)
                const osProcess = await ptyGetForegroundProcess(session_id);

                // If shell returned to foreground, the command finished quickly
                if (!osProcess || SHELL_PROCESSES.has(osProcess)) {
                  return; // Don't update tab name
                }

                // Command is still running - use the command name we extracted
                // This gives us "pnpm" instead of "node", "just" instead of child process
                if (processName) {
                  state.setProcessName(session_id, processName);
                }
              } catch (err) {
                // Silently ignore - process detection is best-effort
                logger.debug("Failed to verify foreground process:", err);
              } finally {
                processDetectionTimers.delete(session_id);
              }
            }, PROCESS_DETECTION_DELAY_MS);

            processDetectionTimers.set(session_id, timer);
            break;
          }
          case "command_end": {
            const commandText =
              command ??
              lastStartedCommand.get(session_id) ??
              state.pendingCommand[session_id]?.command ??
              null;

            if (exit_code !== null) {
              // Check if this command used alternate screen (TUI apps like top, htop, vim)
              // If so, skip output serialization - alternate screen content is discarded
              const wasFulltermApp = usedAlternateScreen.get(session_id) ?? false;
              usedAlternateScreen.delete(session_id);

              if (wasFulltermApp) {
                // TUI app - dispose terminal without serializing, no output to show
                liveTerminalManager.dispose(session_id);
                state.setPendingOutput(session_id, "");
                state.handleCommandEnd(session_id, exit_code);
              } else {
                // Normal command - serialize output for display
                // This is async because terminal.write() is async and we need to
                // wait for pending writes to complete before serializing
                void (async () => {
                  const serializedOutput =
                    await liveTerminalManager.serializeAndDispose(session_id);
                  if (serializedOutput) {
                    // Update the pending command output with the serialized terminal content
                    // This ensures we capture all scrollback that xterm accumulated
                    state.setPendingOutput(session_id, serializedOutput);
                  }
                  state.handleCommandEnd(session_id, exit_code);
                })();
              }

              if (commandText) {
                addCommandHistory(session_id, commandText, exit_code).catch((err) => {
                  logger.debug("Failed to save command history:", err);
                });
              }
            }

            // Refresh git branch/status after successful branch-changing commands.
            // Without this, the git badge in UnifiedInput can get stale after `git checkout`.
            const commandForRefresh =
              command ??
              lastStartedCommand.get(session_id) ??
              state.pendingCommand[session_id]?.command;

            if (exit_code === 0 && shouldRefreshGitInfo(commandForRefresh ?? null)) {
              const cwd = state.sessions[session_id]?.workingDirectory;
              if (cwd) {
                refreshGitInfo(session_id, cwd);
              }
            }

            // If exit_code is null, don't create a block - we don't have valid completion info
            // Cancel any pending process detection for this session
            clearProcessDetectionTimer(session_id);
            // Clear process name when command ends
            state.setProcessName(session_id, null);
            // Note: We don't switch back to timeline mode here anymore.
            // The prompt_start event handles this more reliably, preventing
            // premature switching for apps like codex/cdx that may trigger
            // command_end before they're actually done.
            break;
          }
        }
      })
    );

    // Terminal output - capture for command blocks
    unlisteners.push(
      listen<TerminalOutputEvent>("terminal_output", (event) => {
        const { session_id, data } = event.payload;
        const state = store.getState();
        state.appendOutput(session_id, data);
        // Also write to VirtualTerminal for proper ANSI sequence processing
        virtualTerminalManager.write(session_id, data);
        // Write to live terminal for embedded xterm.js display
        liveTerminalManager.write(session_id, data);
      })
    );

    // Directory changed
    unlisteners.push(
      listen<DirectoryChangedEvent>("directory_changed", async (event) => {
        const { session_id, path } = event.payload;
        const state = store.getState();

        state.updateWorkingDirectory(session_id, path);

        // Fetch git branch for the new directory
        try {
          const branch = await getGitBranch(path);
          state.updateGitBranch(session_id, branch);
        } catch (_error) {
          // Silently ignore errors (not a git repo, git not installed, etc.)
          state.updateGitBranch(session_id, null);
        }

        // Also update the AI agent's workspace if initialized for this session
        // Pass session_id to update the session-specific AI bridge
        try {
          const initialized = await isAiSessionInitialized(session_id);
          if (initialized) {
            await updateAiWorkspace(path, session_id);
            notify.info("Workspace synced", { message: path });
          }
        } catch (error) {
          logger.error("Error updating AI workspace:", error);
        }
      })
    );

    // Virtual environment changed
    unlisteners.push(
      listen<VirtualEnvChangedEvent>("virtual_env_changed", (event) => {
        const { session_id, name } = event.payload;
        store.getState().updateVirtualEnv(session_id, name);
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
        // Track that this command used alternate screen (for skipping output on completion)
        if (enabled) {
          usedAlternateScreen.set(session_id, true);
        }
      })
    );

    // Periodic git status refresh for all active sessions
    // This ensures the git badge in the status bar stays up-to-date
    const gitStatusPollInterval = setInterval(() => {
      const state = store.getState();
      const sessions = state.sessions;
      for (const sessionId of Object.keys(sessions)) {
        const session = sessions[sessionId];
        if (session?.workingDirectory) {
          refreshGitInfo(sessionId, session.workingDirectory);
        }
      }
    }, GIT_STATUS_POLL_INTERVAL_MS);

    // Cleanup
    return () => {
      // Clear all pending timers
      for (const timer of processDetectionTimers.values()) {
        clearTimeout(timer);
      }
      processDetectionTimers.clear();

      // Clear git status polling interval
      clearInterval(gitStatusPollInterval);

      // Unlisten from events - properly await cleanup promises
      Promise.all(
        unlisteners.map((p) =>
          p.then((unlisten) => {
            unlisten();
          })
        )
      ).catch((err) => {
        logger.warn("Failed to unlisten from some events:", err);
      });
    };
  }, []);
}
