/**
 * Strip OSC (Operating System Command) sequences from terminal output.
 * These are control sequences like directory changes and shell integration markers,
 * not display formatting. ANSI color codes are preserved for rendering.
 */
export function stripOscSequences(str: string): string {
  // OSC sequences start with ESC ] and end with BEL (\x07) or ST (\x1b\)
  // Common OSC codes:
  // - OSC 0/1/2: Window/icon title
  // - OSC 7: Current directory
  // - OSC 133: Shell integration (prompt markers)

  let result = str;

  // Remove OSC sequences with ESC prefix: \x1b] ... (\x07 | \x1b\)
  result = result.replace(/\x1b\][\s\S]*?(?:\x07|\x1b\\)/g, "");

  // Remove OSC sequences with bare ] that might appear (defensive)
  // Match ]number; ... until we hit ESC[ (start of CSI) or end
  result = result.replace(/\](?:133|7|0|1|2|9);[^\x1b\x07]*(?:\x07|\x1b\\)?/g, "");

  // Simulate carriage return behavior: \r moves cursor to beginning of line,
  // so subsequent text overwrites previous content. We process line by line,
  // handling \r within each line to keep only the final visible content.
  result = result
    .split("\n")
    .map((line) => {
      // If line contains \r (not at end), split and keep only last segment
      // This simulates terminal overwrite behavior for progress bars
      if (line.includes("\r")) {
        const segments = line.split("\r");
        // Filter out empty segments and take the last non-empty one
        const nonEmpty = segments.filter((s) => s.length > 0);
        return nonEmpty.length > 0 ? nonEmpty[nonEmpty.length - 1] : "";
      }
      return line;
    })
    .join("\n");

  // Strip trailing prompt artifacts (%, $, >, etc.)
  // This handles cases where the shell prompt gets captured
  // The % is zsh's PROMPT_SP marker shown when output doesn't end with newline

  // Remove trailing prompt on its own line (with possible ANSI codes)
  result = result.replace(/\n\s*(?:\x1b\[[0-9;]*m)*[%$>→›❯➜]\s*(?:\x1b\[[0-9;]*m)*\s*$/g, "");

  // Remove standalone % at the very end (zsh PROMPT_SP)
  result = result.replace(/(?:\x1b\[[0-9;]*m)*[%]\s*(?:\x1b\[[0-9;]*m)*\s*$/g, "");

  // Clean up trailing whitespace
  result = result.replace(/\n\s*$/g, "\n");

  return result.trim();
}
