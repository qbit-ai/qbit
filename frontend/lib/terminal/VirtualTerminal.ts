import { SerializeAddon } from "@xterm/addon-serialize";
import { Terminal } from "@xterm/headless";

/**
 * Virtual terminal for processing ANSI escape sequences.
 *
 * Uses @xterm/headless to properly handle terminal control sequences like:
 * - Carriage return (\r) for single-line spinners
 * - Cursor up/down (CSI A/B) for multi-line progress bars
 * - Erase sequences (CSI K, CSI 2K) for line clearing
 *
 * Outputs ANSI-encoded text that preserves colors for ansi-to-react rendering.
 */
export class VirtualTerminal {
  private terminal: Terminal;
  private serializeAddon: SerializeAddon;
  private cachedContent: string = "";
  private contentDirty = true;
  private pendingWrites: Promise<void>[] = [];

  /**
   * Create a new virtual terminal.
   * @param cols Terminal width in columns (default: 120)
   * @param rows Terminal height in rows (default: 50)
   */
  constructor(cols = 120, rows = 50) {
    this.terminal = new Terminal({
      cols,
      rows,
      scrollback: 1000,
      allowProposedApi: true,
    });
    this.serializeAddon = new SerializeAddon();
    this.terminal.loadAddon(this.serializeAddon);
  }

  /**
   * Write data to the virtual terminal (fire-and-forget).
   * The terminal will process all ANSI escape sequences.
   * Use writeAsync() if you need to wait for the write to complete.
   */
  write(data: string): void {
    const promise = new Promise<void>((resolve) => {
      this.terminal.write(data, resolve);
    });
    this.pendingWrites.push(promise);
    this.contentDirty = true;
  }

  /**
   * Write data and wait for it to be processed.
   * Useful for testing or when you need to read content immediately after.
   */
  async writeAsync(data: string): Promise<void> {
    return new Promise((resolve) => {
      this.terminal.write(data, () => {
        this.contentDirty = true;
        resolve();
      });
    });
  }

  /**
   * Wait for all pending writes to complete.
   */
  async flush(): Promise<void> {
    await Promise.all(this.pendingWrites);
    this.pendingWrites = [];
  }

  /**
   * Get the current visible content as ANSI-encoded text.
   * Colors and attributes are preserved for rendering with ansi-to-react.
   *
   * Note: If there are pending writes, this may not include them.
   * Use getContentAsync() to ensure all writes are processed first.
   */
  getContent(): string {
    if (!this.contentDirty) {
      return this.cachedContent;
    }

    // Use the serialize addon to get ANSI-encoded content
    // This includes all visible lines with color/attribute codes
    const serialized = this.serializeAddon.serialize({
      excludeModes: true,
      excludeAltBuffer: true,
    });

    // Post-process the serialized content to clean up for display:
    // 1. Remove cursor positioning sequences (we don't need them for static display)
    // 2. Trim trailing empty lines
    this.cachedContent = this.cleanSerializedContent(serialized);
    this.contentDirty = false;

    return this.cachedContent;
  }

  /**
   * Wait for pending writes, then get content.
   * Recommended for use after write() to ensure content is up-to-date.
   */
  async getContentAsync(): Promise<string> {
    await this.flush();
    return this.getContent();
  }

  /**
   * Clear the terminal content.
   */
  clear(): void {
    // terminal.clear() only clears scrollback, not the current screen
    // Use reset() to fully clear everything
    this.terminal.reset();
    this.cachedContent = "";
    this.contentDirty = true;
  }

  /**
   * Resize the terminal.
   */
  resize(cols: number, rows: number): void {
    this.terminal.resize(cols, rows);
    this.contentDirty = true;
  }

  /**
   * Dispose the terminal and release resources.
   */
  dispose(): void {
    this.terminal.dispose();
  }

  /**
   * Clean up serialized content for display.
   * Removes cursor positioning and trims empty lines.
   */
  private cleanSerializedContent(content: string): string {
    let result = content;

    // Remove cursor save/restore and positioning at the end
    // The serialize addon adds cursor positioning for restore purposes
    // Pattern: \x1b[row;colH at the end
    result = result.replace(/\x1b\[\d+;\d+H$/, "");

    // Remove cursor visibility toggles
    result = result.replace(/\x1b\[\?25[hl]/g, "");

    // Remove cursor positioning sequences (cursor forward, back, up, down)
    // CSI Ps C = cursor forward, CSI Ps G = cursor horizontal absolute
    result = result.replace(/\x1b\[\d*[ABCDG]/g, "");

    // Remove inverse video sequences (used by serialize addon for cursor position)
    // \x1b[7m = inverse on, \x1b[27m = inverse off
    result = result.replace(/\x1b\[2?7m/g, "");

    // Convert CRLF to LF (serialize addon uses CRLF)
    result = result.replace(/\r\n/g, "\n");

    // Remove standalone carriage returns at start of lines
    result = result.replace(/\n\r/g, "\n");

    // Trim trailing empty lines (lines that are just whitespace or reset codes)
    const lines = result.split("\n");
    while (
      lines.length > 0 &&
      lines[lines.length - 1].replace(/\x1b\[[0-9;]*m/g, "").trim() === ""
    ) {
      lines.pop();
    }
    result = lines.join("\n");

    // Remove trailing reset if it's the only thing on the last line
    result = result.replace(/\n\x1b\[0m$/, "");

    return result.trim();
  }

  /**
   * Get the current cursor position.
   */
  getCursorPosition(): { x: number; y: number } {
    const buffer = this.terminal.buffer.active;
    return {
      x: buffer.cursorX,
      y: buffer.cursorY,
    };
  }

  /**
   * Get the number of lines with content.
   */
  getLineCount(): number {
    const buffer = this.terminal.buffer.active;
    let count = 0;
    for (let i = 0; i < buffer.length; i++) {
      const line = buffer.getLine(i);
      if (line && line.translateToString().trim().length > 0) {
        count = i + 1;
      }
    }
    return count;
  }
}
