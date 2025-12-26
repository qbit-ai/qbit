import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { VirtualTerminal } from "./VirtualTerminal";

describe("VirtualTerminal", () => {
  let vt: VirtualTerminal;

  beforeEach(() => {
    vt = new VirtualTerminal(80, 24);
  });

  afterEach(() => {
    vt.dispose();
  });

  describe("Basic Output", () => {
    it("handles plain text", async () => {
      await vt.writeAsync("Hello World");
      expect(vt.getContent()).toBe("Hello World");
    });

    it("handles newlines", async () => {
      await vt.writeAsync("Line 1\nLine 2\nLine 3");
      expect(vt.getContent()).toBe("Line 1\nLine 2\nLine 3");
    });

    it("handles empty writes", async () => {
      await vt.writeAsync("");
      expect(vt.getContent()).toBe("");
    });

    it("handles multiple writes", async () => {
      await vt.writeAsync("Hello ");
      await vt.writeAsync("World");
      expect(vt.getContent()).toBe("Hello World");
    });

    it("handles fire-and-forget writes with flush", async () => {
      vt.write("Hello ");
      vt.write("World");
      await vt.flush();
      expect(vt.getContent()).toBe("Hello World");
    });

    it("getContentAsync waits for pending writes", async () => {
      vt.write("Async content");
      const content = await vt.getContentAsync();
      expect(content).toBe("Async content");
    });
  });

  describe("Carriage Return (Single-line spinners)", () => {
    it("overwrites line with carriage return", async () => {
      await vt.writeAsync("Loading...\rDone!     ");
      const content = vt.getContent();
      expect(content).toContain("Done!");
      expect(content).not.toContain("Loading");
    });

    it("handles spinner animation frames", async () => {
      // Simulate a spinner: | / - \
      await vt.writeAsync("| Processing\r");
      await vt.writeAsync("/ Processing\r");
      await vt.writeAsync("- Processing\r");
      await vt.writeAsync("\\ Processing");
      const content = vt.getContent();
      expect(content).toContain("\\ Processing");
      expect(content).not.toContain("| Processing");
    });

    it("handles progress percentage updates", async () => {
      await vt.writeAsync("Progress: 0%\r");
      await vt.writeAsync("Progress: 50%\r");
      await vt.writeAsync("Progress: 100%");
      const content = vt.getContent();
      expect(content).toContain("100%");
      // Check that old percentages are overwritten (not on separate lines)
      expect(content).not.toContain("Progress: 0%");
      expect(content).not.toContain("Progress: 50%");
    });

    it("handles carriage return followed by newline", async () => {
      await vt.writeAsync("Line 1\r\n");
      await vt.writeAsync("Line 2");
      const content = vt.getContent();
      expect(content).toContain("Line 1");
      expect(content).toContain("Line 2");
    });
  });

  describe("Cursor Up (Multi-line progress bars)", () => {
    it("handles cursor up and overwrite", async () => {
      // Write two lines, cursor up 1, overwrite
      await vt.writeAsync("Line 1: original\n");
      await vt.writeAsync("Line 2: original");
      await vt.writeAsync("\x1b[1A"); // Cursor up 1
      await vt.writeAsync("\r"); // Carriage return
      await vt.writeAsync("Line 1: UPDATED");

      const content = vt.getContent();
      expect(content).toContain("Line 1: UPDATED");
      expect(content).toContain("Line 2: original");
    });

    it("handles npm/pnpm-style multi-line progress", async () => {
      // Simulate npm progress that updates multiple lines
      await vt.writeAsync("pkg1: downloading\n");
      await vt.writeAsync("pkg2: downloading\n");
      await vt.writeAsync("pkg3: downloading");

      // Now update first line: cursor up 2, overwrite
      await vt.writeAsync("\x1b[2A"); // Cursor up 2
      await vt.writeAsync("\r"); // Carriage return
      await vt.writeAsync("pkg1: done       ");
      await vt.writeAsync("\n"); // Move to next line
      await vt.writeAsync("pkg2: done       ");
      await vt.writeAsync("\n"); // Move to next line
      await vt.writeAsync("pkg3: done       ");

      const content = vt.getContent();
      expect(content).toContain("pkg1: done");
      expect(content).toContain("pkg2: done");
      expect(content).toContain("pkg3: done");
      expect(content).not.toContain("downloading");
    });

    it("handles cursor up with column positioning", async () => {
      await vt.writeAsync("Line 1\nLine 2");
      await vt.writeAsync("\x1b[1A"); // Cursor up 1
      await vt.writeAsync("\x1b[1G"); // Column 1 (1-indexed in xterm)
      await vt.writeAsync("NEW  ");

      const content = vt.getContent();
      expect(content).toContain("NEW");
      expect(content).toContain("Line 2");
    });
  });

  describe("Erase Sequences", () => {
    it("handles erase to end of line (CSI 0K)", async () => {
      await vt.writeAsync("Hello World");
      await vt.writeAsync("\r"); // Carriage return
      await vt.writeAsync("\x1b[K"); // Erase to end of line (CSI 0K)
      await vt.writeAsync("Bye");

      const content = vt.getContent();
      expect(content).toBe("Bye");
    });

    it("handles erase entire line (CSI 2K)", async () => {
      await vt.writeAsync("Some long text here");
      await vt.writeAsync("\x1b[2K"); // Erase entire line
      await vt.writeAsync("\rNew");

      const content = vt.getContent();
      expect(content).toBe("New");
    });

    it("handles erase in combination with cursor movement", async () => {
      await vt.writeAsync("Line 1\nLine 2\nLine 3");
      await vt.writeAsync("\x1b[2A"); // Cursor up 2
      await vt.writeAsync("\x1b[2K"); // Erase entire line
      await vt.writeAsync("\rReplaced");
      await vt.writeAsync("\n"); // Move down
      await vt.writeAsync("\x1b[2K\rAlso replaced");

      const content = vt.getContent();
      expect(content).toContain("Replaced");
      expect(content).toContain("Also replaced");
      expect(content).toContain("Line 3");
    });
  });

  describe("ANSI Colors", () => {
    it("preserves basic color codes", async () => {
      await vt.writeAsync("\x1b[32mGreen\x1b[0m Normal");
      const content = vt.getContent();
      // The serialize addon should preserve SGR codes
      expect(content).toContain("Green");
      expect(content).toContain("Normal");
    });

    it("preserves bold attribute", async () => {
      await vt.writeAsync("\x1b[1mBold\x1b[0m Normal");
      const content = vt.getContent();
      expect(content).toContain("Bold");
      expect(content).toContain("Normal");
    });

    it("preserves 256-color codes", async () => {
      await vt.writeAsync("\x1b[38;5;196mRed256\x1b[0m");
      const content = vt.getContent();
      expect(content).toContain("Red256");
    });

    it("preserves RGB true color codes", async () => {
      await vt.writeAsync("\x1b[38;2;255;128;0mOrange\x1b[0m");
      const content = vt.getContent();
      expect(content).toContain("Orange");
    });

    it("preserves colors after overwrite", async () => {
      await vt.writeAsync("\x1b[31mRed\x1b[0m\r");
      await vt.writeAsync("\x1b[32mGreen\x1b[0m");
      const content = vt.getContent();
      expect(content).toContain("Green");
      // Should contain green color code, not red
    });
  });

  describe("Real-world Patterns", () => {
    it("handles cargo build progress", async () => {
      // Cargo updates the same line for compiling
      await vt.writeAsync("   Compiling foo v1.0.0\r");
      await vt.writeAsync("   Compiling bar v2.0.0\r");
      await vt.writeAsync("    Finished dev [unoptimized + debuginfo] target(s)");

      const content = vt.getContent();
      expect(content).toContain("Finished");
      // The previous "Compiling" lines should be overwritten
    });

    it("handles vitest progress", async () => {
      // Vitest shows progress with cursor movement
      await vt.writeAsync(" PASS  test1.ts\n");
      await vt.writeAsync(" PASS  test2.ts\n");
      await vt.writeAsync("\n Test Files  2 passed");

      const content = vt.getContent();
      expect(content).toContain("PASS  test1.ts");
      expect(content).toContain("PASS  test2.ts");
      expect(content).toContain("2 passed");
    });

    it("handles progress bar with percentage", async () => {
      // Common pattern: [=====>    ] 50%
      await vt.writeAsync("[          ] 0%\r");
      await vt.writeAsync("[==        ] 20%\r");
      await vt.writeAsync("[====      ] 40%\r");
      await vt.writeAsync("[======    ] 60%\r");
      await vt.writeAsync("[========  ] 80%\r");
      await vt.writeAsync("[==========] 100%");

      const content = vt.getContent();
      expect(content).toContain("[==========] 100%");
      // Old progress states should be overwritten
      expect(content).not.toContain("] 0%");
      expect(content).not.toContain("] 20%");
    });

    it("handles spinner that clears on completion", async () => {
      // Spinner frames
      await vt.writeAsync("Loading |\r");
      await vt.writeAsync("Loading /\r");
      await vt.writeAsync("Loading -\r");
      // Clear and show done
      await vt.writeAsync("\x1b[2K\rDone!");

      const content = vt.getContent();
      expect(content).toBe("Done!");
    });
  });

  describe("Caching", () => {
    it("caches content between reads", async () => {
      await vt.writeAsync("Test content");
      const content1 = vt.getContent();
      const content2 = vt.getContent();
      expect(content1).toBe(content2);
    });

    it("invalidates cache on new write", async () => {
      await vt.writeAsync("First");
      const content1 = vt.getContent();
      await vt.writeAsync(" Second");
      const content2 = vt.getContent();
      expect(content1).toBe("First");
      expect(content2).toBe("First Second");
    });
  });

  describe("Terminal Operations", () => {
    it("clear() resets content", async () => {
      await vt.writeAsync("Some content");
      expect(vt.getContent()).toBe("Some content");
      vt.clear();
      expect(vt.getContent()).toBe("");
    });

    it("resize() updates dimensions", async () => {
      vt = new VirtualTerminal(40, 10);
      await vt.writeAsync("Test");
      vt.resize(80, 24);
      // Content should still be there
      expect(vt.getContent()).toContain("Test");
    });

    it("getCursorPosition() returns current position", async () => {
      await vt.writeAsync("Hello\nWorld");
      const pos = vt.getCursorPosition();
      expect(pos.y).toBeGreaterThanOrEqual(1);
      expect(pos.x).toBeGreaterThanOrEqual(0);
    });

    it("getLineCount() counts non-empty lines", async () => {
      await vt.writeAsync("Line 1\nLine 2\nLine 3");
      expect(vt.getLineCount()).toBe(3);
    });
  });
});
