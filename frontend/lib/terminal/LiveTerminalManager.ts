import { FitAddon } from "@xterm/addon-fit";
import { SerializeAddon } from "@xterm/addon-serialize";
import { Terminal } from "@xterm/xterm";
import { ThemeManager } from "@/lib/theme";

/**
 * LiveTerminalManager - Singleton manager for a single xterm.js instance
 * used for streaming live command output.
 *
 * Unlike TerminalInstanceManager (which manages persistent terminals for interactive
 * shell sessions), this manager handles ephemeral terminals for displaying live
 * command output in the timeline. The terminal is read-only and gets serialized
 * and disposed when the command completes.
 *
 * Usage:
 * - getOrCreate(sessionId) to get or create a terminal for streaming
 * - attachToContainer(sessionId, container) to attach to DOM
 * - write(sessionId, data) to stream output
 * - serializeAndDispose(sessionId) when command completes to get final content
 */

interface LiveTerminalInstance {
  terminal: Terminal;
  fitAddon: FitAddon;
  serializeAddon: SerializeAddon;
  currentContainer: HTMLElement | null;
  themeUnsubscribe: (() => void) | null;
  pendingWrites: string[]; // Buffer for writes before terminal is opened
  isOpened: boolean; // Track if terminal has been opened
}

class LiveTerminalManagerClass {
  private instances = new Map<string, LiveTerminalInstance>();

  /**
   * Get or create a terminal instance for a session.
   * If one already exists, returns it.
   */
  getOrCreate(sessionId: string): Terminal {
    const existing = this.instances.get(sessionId);
    if (existing) {
      return existing.terminal;
    }

    // Create new terminal with read-only configuration
    const terminal = new Terminal({
      cursorBlink: false,
      cursorInactiveStyle: "none",
      disableStdin: true,
      fontSize: 12,
      fontFamily: "JetBrains Mono, Menlo, Monaco, Consolas, monospace",
      fontWeight: "normal",
      fontWeightBold: "bold",
      lineHeight: 1.4,
      scrollback: 500,
      convertEol: true,
      allowProposedApi: true,
    });

    // Create addons
    const fitAddon = new FitAddon();
    const serializeAddon = new SerializeAddon();

    terminal.loadAddon(fitAddon);
    terminal.loadAddon(serializeAddon);

    // Apply current theme (colors only - we override font settings below)
    ThemeManager.applyToTerminal(terminal);

    // Override font settings to match CommandBlock's ansi-output styling
    // Theme may set larger fontSize for interactive terminals, but for
    // command output display we need smaller text to match ansi-to-react
    terminal.options.fontSize = 12;
    terminal.options.lineHeight = 1.4;
    terminal.options.fontWeight = "normal";
    terminal.options.letterSpacing = 0;
    // Use transparent background so it blends with the timeline
    terminal.options.theme = {
      ...terminal.options.theme,
      background: "rgba(0,0,0,0)",
    };

    // Subscribe to theme changes
    const themeUnsubscribe = ThemeManager.onChange(() => {
      ThemeManager.applyToTerminal(terminal);
      // Re-apply our font overrides after theme changes
      terminal.options.fontSize = 12;
      terminal.options.lineHeight = 1.4;
      terminal.options.fontWeight = "normal";
      terminal.options.letterSpacing = 0;
      terminal.options.theme = {
        ...terminal.options.theme,
        background: "rgba(0,0,0,0)",
      };
    });

    const instance: LiveTerminalInstance = {
      terminal,
      fitAddon,
      serializeAddon,
      currentContainer: null,
      themeUnsubscribe,
      pendingWrites: [],
      isOpened: false,
    };

    this.instances.set(sessionId, instance);

    return terminal;
  }

  /**
   * Get an existing terminal instance for a session.
   * Returns undefined if no instance exists.
   */
  get(sessionId: string): Terminal | undefined {
    return this.instances.get(sessionId)?.terminal;
  }

  /**
   * Check if there's pending output waiting to be displayed.
   * This can be used to show the terminal even before command_start.
   */
  hasPendingOutput(sessionId: string): boolean {
    const instance = this.instances.get(sessionId);
    return instance ? instance.pendingWrites.length > 0 : false;
  }

  /**
   * Check if a terminal instance exists for a session.
   */
  has(sessionId: string): boolean {
    return this.instances.has(sessionId);
  }

  /**
   * Write data to the session's terminal.
   * If terminal doesn't exist, creates it first.
   * If terminal isn't opened yet, buffers the data for later.
   */
  write(sessionId: string, data: string): void {
    let instance = this.instances.get(sessionId);

    // Auto-create terminal if it doesn't exist (handles cases where terminal_output
    // arrives before command_start, or when shell doesn't have OSC 133 integration)
    if (!instance) {
      this.getOrCreate(sessionId);
      instance = this.instances.get(sessionId);
      if (!instance) {
        console.error(
          `[LiveTerminalManager] write() - Failed to create instance for session ${sessionId}`
        );
        return;
      }
    }

    if (instance.isOpened) {
      instance.terminal.write(data);
    } else {
      // Buffer writes until terminal is opened
      instance.pendingWrites.push(data);
    }
  }

  /**
   * Attach terminal to a container element.
   * If already attached elsewhere, moves the terminal's DOM to the new container.
   */
  attachToContainer(sessionId: string, container: HTMLElement): boolean {
    const instance = this.instances.get(sessionId);
    if (!instance) {
      return false;
    }

    const { terminal, fitAddon, currentContainer } = instance;

    if (currentContainer === container) {
      // Already attached to this container, just fit
      fitAddon.fit();
      return true;
    }

    if (terminal.element) {
      // Terminal was opened before - move its DOM to new container
      container.appendChild(terminal.element);
      // Fit to new container size
      fitAddon.fit();
    } else {
      // First time opening
      terminal.open(container);
      instance.isOpened = true;

      // Fit BEFORE flushing writes to ensure terminal has proper dimensions
      // This prevents data loss when pending writes exceed initial row count
      fitAddon.fit();

      // Flush any pending writes that happened before open
      if (instance.pendingWrites.length > 0) {
        for (const data of instance.pendingWrites) {
          terminal.write(data);
        }
        instance.pendingWrites = [];
      }
    }

    // Update the tracked container
    instance.currentContainer = container;

    return true;
  }

  /**
   * Scroll terminal to the bottom.
   */
  scrollToBottom(sessionId: string): void {
    const instance = this.instances.get(sessionId);
    // Only scroll if terminal is opened (renderer must be ready)
    if (instance?.isOpened) {
      try {
        instance.terminal.scrollToBottom();
      } catch {
        // Ignore renderer race condition errors
      }
    }
  }

  /**
   * Serialize terminal content and dispose the instance.
   * Returns the serialized ANSI content for static rendering.
   *
   * This is async because terminal.write() is async - we must wait for
   * all writes to complete before serializing to avoid data loss.
   */
  async serializeAndDispose(sessionId: string): Promise<string> {
    const instance = this.instances.get(sessionId);
    if (!instance) {
      return "";
    }

    // Write any buffered data (for fast commands where terminal was never opened)
    if (instance.pendingWrites.length > 0) {
      // Write all pending data and wait for completion
      // terminal.write() is async, so we use the callback form to know when done
      const writePromises = instance.pendingWrites.map(
        (data) =>
          new Promise<void>((resolve) => {
            instance.terminal.write(data, resolve);
          })
      );
      await Promise.all(writePromises);
      instance.pendingWrites = [];
    }

    // Wait for any queued writes to complete by writing empty string with callback
    // This ensures all prior terminal.write() calls have been processed
    await new Promise<void>((resolve) => {
      instance.terminal.write("", resolve);
    });

    const serialized = instance.serializeAddon.serialize({
      excludeModes: true,
      excludeAltBuffer: true,
    });

    this.dispose(sessionId);
    return serialized;
  }

  /**
   * Dispose terminal instance without serializing.
   */
  dispose(sessionId: string): void {
    const instance = this.instances.get(sessionId);
    if (instance) {
      instance.themeUnsubscribe?.();
      instance.terminal.dispose();
      this.instances.delete(sessionId);
    }
  }

  /**
   * Dispose all instances. Used for cleanup on app unmount.
   */
  disposeAll(): void {
    for (const sessionId of this.instances.keys()) {
      this.dispose(sessionId);
    }
  }
}

// Export singleton instance
export const liveTerminalManager = new LiveTerminalManagerClass();
