import { logger } from "@/lib/logger";
/**
 * TerminalInstanceManager - Singleton manager for xterm.js instances.
 *
 * This manager persists xterm.js Terminal instances across React component remounts.
 * When pane structure changes (splits, closes), React may remount the Terminal component.
 * Without this manager, remounting would dispose and recreate the xterm.js instance,
 * losing all terminal state (scrollback, cursor position, app display).
 *
 * Usage:
 * - Terminal component calls getOrCreate() to get an instance
 * - Terminal component calls attachToContainer() to attach/reattach to DOM
 * - On unmount, Terminal does NOT dispose - just detaches
 * - dispose() is called only when a session is fully closed
 */

import type { FitAddon } from "@xterm/addon-fit";
import type { Terminal as XTerm } from "@xterm/xterm";

interface TerminalInstance {
  terminal: XTerm;
  fitAddon: FitAddon;
  currentContainer: HTMLElement | null;
}

class TerminalInstanceManagerClass {
  private instances = new Map<string, TerminalInstance>();

  /**
   * Get an existing terminal instance for a session.
   * Returns undefined if no instance exists.
   */
  get(sessionId: string): TerminalInstance | undefined {
    return this.instances.get(sessionId);
  }

  /**
   * Check if a terminal instance exists for a session.
   */
  has(sessionId: string): boolean {
    return this.instances.has(sessionId);
  }

  /**
   * Register a new terminal instance.
   * Called by Terminal component after creating the xterm instance.
   */
  register(sessionId: string, terminal: XTerm, fitAddon: FitAddon): void {
    if (this.instances.has(sessionId)) {
      logger.warn(
        `[TerminalInstanceManager] Instance already exists for session ${sessionId}, replacing`
      );
      // Dispose the old one first
      const old = this.instances.get(sessionId);
      old?.terminal.dispose();
    }
    this.instances.set(sessionId, {
      terminal,
      fitAddon,
      currentContainer: null,
    });
  }

  /**
   * Attach terminal to a container element.
   * If already attached elsewhere, moves the terminal's DOM to the new container.
   * This is the key operation that allows terminals to survive remounts.
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
      // The terminal.element is the root element created by xterm.js
      container.appendChild(terminal.element);
      logger.debug(`[TerminalInstanceManager] Moved terminal ${sessionId} to new container`);
    } else {
      // First time opening - this shouldn't happen if register() was called after open()
      logger.warn(`[TerminalInstanceManager] Terminal ${sessionId} has no element, opening fresh`);
      terminal.open(container);
    }

    // Update the tracked container
    instance.currentContainer = container;

    // Fit to new container size
    fitAddon.fit();

    return true;
  }

  /**
   * Detach terminal from its container.
   * Called when Terminal component unmounts.
   * Does NOT dispose the terminal - it remains in the manager for reuse.
   */
  detach(sessionId: string): void {
    const instance = this.instances.get(sessionId);
    if (instance) {
      instance.currentContainer = null;
      // Note: We don't remove the DOM here - it will be removed when the container unmounts
      // The terminal element stays in the manager, ready to be reattached
    }
  }

  /**
   * Dispose terminal instance completely.
   * Call when a session is fully closed (tab closed, pane removed with no other references).
   */
  dispose(sessionId: string): void {
    const instance = this.instances.get(sessionId);
    if (instance) {
      logger.info(`[TerminalInstanceManager] Disposing terminal for session ${sessionId}`);
      instance.terminal.dispose();
      this.instances.delete(sessionId);
    }
  }

  /**
   * Get the current container for a terminal (if attached).
   */
  getContainer(sessionId: string): HTMLElement | null {
    return this.instances.get(sessionId)?.currentContainer ?? null;
  }

  /**
   * Check if a terminal is currently attached to a container.
   */
  isAttached(sessionId: string): boolean {
    return this.instances.get(sessionId)?.currentContainer !== null;
  }

  /**
   * Get all session IDs with active instances.
   */
  getSessionIds(): string[] {
    return Array.from(this.instances.keys());
  }

  /**
   * Dispose all instances. Used for cleanup on app unmount.
   */
  disposeAll(): void {
    for (const [sessionId, instance] of this.instances) {
      logger.info(`[TerminalInstanceManager] Disposing terminal for session ${sessionId}`);
      instance.terminal.dispose();
    }
    this.instances.clear();
  }
}

// Export singleton instance
export const TerminalInstanceManager = new TerminalInstanceManagerClass();
