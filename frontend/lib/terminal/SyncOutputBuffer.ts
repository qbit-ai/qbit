import type { Terminal as XTerm } from "@xterm/xterm";
import { logger } from "@/lib/logger";

/**
 * Buffer for DEC 2026 synchronized output.
 *
 * When synchronized output mode is enabled, terminal output is buffered
 * instead of being written immediately. When the mode is disabled, all
 * buffered content is flushed atomically, preventing screen flickering.
 *
 * This follows Ghostty's approach: batch renders during sync mode and
 * include a safety timeout to flush if an app forgets to disable the mode.
 */
export class SyncOutputBuffer {
  private buffer: string[] = [];
  private syncEnabled = false;
  private terminal: XTerm | null = null;
  private timeoutId: ReturnType<typeof setTimeout> | null = null;

  // Safety timeout: flush buffer if app forgets to disable sync mode (like Ghostty)
  private static readonly SYNC_TIMEOUT_MS = 1000;

  /**
   * Attach to an xterm.js terminal instance.
   */
  attach(terminal: XTerm): void {
    this.terminal = terminal;
  }

  /**
   * Detach from the terminal and cleanup.
   */
  detach(): void {
    this.terminal = null;
    this.clearTimeout();
    this.buffer = [];
    this.syncEnabled = false;
  }

  /**
   * Write data to the terminal, respecting synchronized output mode.
   * When sync mode is enabled, data is buffered.
   * When sync mode is disabled, data is written immediately.
   */
  write(data: string): void {
    if (!this.terminal) return;

    if (this.syncEnabled) {
      // Buffer the data
      this.buffer.push(data);
    } else {
      // Write immediately
      this.terminal.write(data);
    }
  }

  /**
   * Enable or disable synchronized output mode.
   * When disabled after being enabled, flushes all buffered content atomically.
   */
  setSyncEnabled(enabled: boolean): void {
    if (enabled === this.syncEnabled) return;

    this.syncEnabled = enabled;

    if (enabled) {
      // Start safety timeout
      this.startTimeout();
    } else {
      // Flush buffer and clear timeout
      this.clearTimeout();
      this.flush();
    }
  }

  /**
   * Flush all buffered content to the terminal atomically.
   */
  private flush(): void {
    if (!this.terminal || this.buffer.length === 0) return;

    // Join and write all buffered content atomically
    const content = this.buffer.join("");
    this.buffer = [];
    this.terminal.write(content);
  }

  /**
   * Start the safety timeout.
   * If sync mode is still enabled after the timeout, force a flush.
   */
  private startTimeout(): void {
    this.clearTimeout();
    this.timeoutId = setTimeout(() => {
      if (this.syncEnabled) {
        logger.warn("[SyncOutputBuffer] Timeout - forcing flush after 1s");
        this.syncEnabled = false;
        this.flush();
      }
    }, SyncOutputBuffer.SYNC_TIMEOUT_MS);
  }

  /**
   * Clear the safety timeout.
   */
  private clearTimeout(): void {
    if (this.timeoutId !== null) {
      clearTimeout(this.timeoutId);
      this.timeoutId = null;
    }
  }

  /**
   * Check if synchronized output mode is currently enabled.
   */
  get isSyncEnabled(): boolean {
    return this.syncEnabled;
  }
}
