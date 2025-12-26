import { VirtualTerminal } from "./VirtualTerminal";

/**
 * Manages VirtualTerminal instances for pending commands.
 *
 * VirtualTerminal instances are stateful and can't be stored in Zustand,
 * so we manage them separately in this module-level Map.
 */
class VirtualTerminalManager {
  private terminals = new Map<string, VirtualTerminal>();

  /**
   * Create a new VirtualTerminal for a session.
   * Disposes any existing terminal for that session.
   */
  create(sessionId: string): VirtualTerminal {
    this.dispose(sessionId);
    const vt = new VirtualTerminal(120, 50);
    this.terminals.set(sessionId, vt);
    return vt;
  }

  /**
   * Get the VirtualTerminal for a session, or undefined if none exists.
   */
  get(sessionId: string): VirtualTerminal | undefined {
    return this.terminals.get(sessionId);
  }

  /**
   * Write data to the session's VirtualTerminal.
   * No-op if no terminal exists for the session.
   */
  write(sessionId: string, data: string): void {
    const vt = this.terminals.get(sessionId);
    if (vt) {
      vt.write(data);
    }
  }

  /**
   * Get processed output from the session's VirtualTerminal.
   * Returns empty string if no terminal exists.
   */
  async getProcessedOutput(sessionId: string): Promise<string> {
    const vt = this.terminals.get(sessionId);
    if (!vt) {
      return "";
    }
    return vt.getContentAsync();
  }

  /**
   * Dispose the VirtualTerminal for a session.
   */
  dispose(sessionId: string): void {
    const vt = this.terminals.get(sessionId);
    if (vt) {
      vt.dispose();
      this.terminals.delete(sessionId);
    }
  }

  /**
   * Dispose all VirtualTerminal instances.
   */
  disposeAll(): void {
    for (const vt of this.terminals.values()) {
      vt.dispose();
    }
    this.terminals.clear();
  }
}

/**
 * Singleton instance for managing VirtualTerminals across the app.
 */
export const virtualTerminalManager = new VirtualTerminalManager();
