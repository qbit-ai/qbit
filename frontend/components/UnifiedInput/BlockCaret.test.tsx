import { render } from "@testing-library/react";
import { createRef } from "react";
import { describe, expect, it } from "vitest";
import type { CaretSettings } from "@/lib/settings";
import { BlockCaret, getCaretCoordinates, getRowCol } from "./BlockCaret";

const defaultSettings: CaretSettings = {
  style: "block",
  width: 1.0,
  color: null,
  blink_speed: 530,
  opacity: 1.0,
};

describe("BlockCaret", () => {
  describe("getRowCol", () => {
    it("should return row 0, col 0 for empty text at position 0", () => {
      expect(getRowCol("", 0)).toEqual({ row: 0, col: 0 });
    });

    it("should return correct col on single line", () => {
      expect(getRowCol("hello", 3)).toEqual({ row: 0, col: 3 });
    });

    it("should return col at end of single line", () => {
      expect(getRowCol("hello", 5)).toEqual({ row: 0, col: 5 });
    });

    it("should return row 1 after a newline", () => {
      expect(getRowCol("hello\nworld", 6)).toEqual({ row: 1, col: 0 });
    });

    it("should return correct row and col on second line", () => {
      expect(getRowCol("hello\nworld", 9)).toEqual({ row: 1, col: 3 });
    });

    it("should handle multiple newlines", () => {
      expect(getRowCol("a\nb\nc\nd", 6)).toEqual({ row: 3, col: 0 });
    });

    it("should handle cursor at end of multi-line text", () => {
      const text = "line1\nline2\nline3";
      expect(getRowCol(text, text.length)).toEqual({ row: 2, col: 5 });
    });

    it("should handle consecutive newlines (empty lines)", () => {
      expect(getRowCol("a\n\n\nb", 3)).toEqual({ row: 2, col: 0 });
    });

    it("should handle position 0 on non-empty text", () => {
      expect(getRowCol("hello\nworld", 0)).toEqual({ row: 0, col: 0 });
    });

    it("should handle cursor right before newline", () => {
      expect(getRowCol("hello\nworld", 5)).toEqual({ row: 0, col: 5 });
    });
  });

  describe("getCaretCoordinates", () => {
    it("should return top, left, and height properties", () => {
      const textarea = document.createElement("textarea");
      textarea.value = "hello world";
      document.body.appendChild(textarea);

      const coords = getCaretCoordinates(textarea, 5);
      expect(coords).toHaveProperty("top");
      expect(coords).toHaveProperty("left");
      expect(coords).toHaveProperty("height");
      expect(typeof coords.top).toBe("number");
      expect(typeof coords.left).toBe("number");
      expect(typeof coords.height).toBe("number");

      document.body.removeChild(textarea);
    });

    it("should return numeric coordinates at position 0", () => {
      const textarea = document.createElement("textarea");
      textarea.value = "test";
      document.body.appendChild(textarea);

      const coords = getCaretCoordinates(textarea, 0);
      expect(Number.isFinite(coords.top)).toBe(true);
      expect(Number.isFinite(coords.left)).toBe(true);
      expect(Number.isFinite(coords.height)).toBe(true);

      document.body.removeChild(textarea);
    });

    it("should clean up the mirror div after measurement", () => {
      const textarea = document.createElement("textarea");
      textarea.value = "hello";
      document.body.appendChild(textarea);

      getCaretCoordinates(textarea, 3);

      // The mirror div should be removed from the DOM
      const mirror = document.getElementById("textarea-caret-mirror");
      expect(mirror).toBeNull();

      document.body.removeChild(textarea);
    });

    it("should handle empty text at position 0", () => {
      const textarea = document.createElement("textarea");
      textarea.value = "";
      document.body.appendChild(textarea);

      const coords = getCaretCoordinates(textarea, 0);
      expect(Number.isFinite(coords.top)).toBe(true);
      expect(Number.isFinite(coords.left)).toBe(true);

      document.body.removeChild(textarea);
    });

    it("should handle position at end of text", () => {
      const textarea = document.createElement("textarea");
      textarea.value = "hello world";
      document.body.appendChild(textarea);

      const coords = getCaretCoordinates(textarea, 11);
      expect(Number.isFinite(coords.top)).toBe(true);
      expect(Number.isFinite(coords.left)).toBe(true);

      document.body.removeChild(textarea);
    });
  });

  describe("rendering", () => {
    it("should render nothing when visible is false", () => {
      const ref = createRef<HTMLTextAreaElement>();
      const { container } = render(
        <BlockCaret textareaRef={ref} text="hello" settings={defaultSettings} visible={false} />
      );
      expect(container.innerHTML).toBe("");
    });

    it("should render a div when visible with a textarea ref", () => {
      // Create a real textarea for the ref
      const textarea = document.createElement("textarea");
      textarea.value = "hello";
      document.body.appendChild(textarea);

      const ref = { current: textarea };

      const { container } = render(
        <BlockCaret textareaRef={ref} text="hello" settings={defaultSettings} visible={true} />
      );

      const caret = container.querySelector("[aria-hidden='true']");
      expect(caret).toBeInTheDocument();
      expect(caret).toHaveClass("absolute", "pointer-events-none");

      document.body.removeChild(textarea);
    });

    it("should apply custom color from settings", () => {
      const textarea = document.createElement("textarea");
      textarea.value = "hello";
      document.body.appendChild(textarea);
      const ref = { current: textarea };

      const { container } = render(
        <BlockCaret
          textareaRef={ref}
          text="hello"
          settings={{ ...defaultSettings, color: "#ff0000" }}
          visible={true}
        />
      );

      const caret = container.querySelector("[aria-hidden='true']") as HTMLElement;
      expect(caret.style.backgroundColor).toBe("rgb(255, 0, 0)");

      document.body.removeChild(textarea);
    });

    it("should use var(--foreground) when color is null", () => {
      const textarea = document.createElement("textarea");
      textarea.value = "hello";
      document.body.appendChild(textarea);
      const ref = { current: textarea };

      const { container } = render(
        <BlockCaret
          textareaRef={ref}
          text="hello"
          settings={{ ...defaultSettings, color: null }}
          visible={true}
        />
      );

      const caret = container.querySelector("[aria-hidden='true']") as HTMLElement;
      expect(caret.style.backgroundColor).toBe("var(--foreground)");

      document.body.removeChild(textarea);
    });

    it("should apply width from settings in ch units", () => {
      const textarea = document.createElement("textarea");
      textarea.value = "hello";
      document.body.appendChild(textarea);
      const ref = { current: textarea };

      const { container } = render(
        <BlockCaret
          textareaRef={ref}
          text="hello"
          settings={{ ...defaultSettings, width: 2.5 }}
          visible={true}
        />
      );

      const caret = container.querySelector("[aria-hidden='true']") as HTMLElement;
      expect(caret.style.width).toBe("2.5ch");

      document.body.removeChild(textarea);
    });

    it("should set animation none when blink_speed is 0", () => {
      const textarea = document.createElement("textarea");
      textarea.value = "hello";
      document.body.appendChild(textarea);
      const ref = { current: textarea };

      const { container } = render(
        <BlockCaret
          textareaRef={ref}
          text="hello"
          settings={{ ...defaultSettings, blink_speed: 0 }}
          visible={true}
        />
      );

      const caret = container.querySelector("[aria-hidden='true']") as HTMLElement;
      expect(caret.style.animation).toBe("none");

      document.body.removeChild(textarea);
    });

    it("should set blink animation with custom speed", () => {
      const textarea = document.createElement("textarea");
      textarea.value = "hello";
      document.body.appendChild(textarea);
      const ref = { current: textarea };

      const { container } = render(
        <BlockCaret
          textareaRef={ref}
          text="hello"
          settings={{ ...defaultSettings, blink_speed: 800 }}
          visible={true}
        />
      );

      const caret = container.querySelector("[aria-hidden='true']") as HTMLElement;
      expect(caret.style.animation).toContain("800ms");

      document.body.removeChild(textarea);
    });

    it("should apply opacity from settings", () => {
      const textarea = document.createElement("textarea");
      textarea.value = "hello";
      document.body.appendChild(textarea);
      const ref = { current: textarea };

      const { container } = render(
        <BlockCaret
          textareaRef={ref}
          text="hello"
          settings={{ ...defaultSettings, opacity: 0.5 }}
          visible={true}
        />
      );

      const caret = container.querySelector("[aria-hidden='true']") as HTMLElement;
      expect(caret.style.opacity).toBe("0.5");

      document.body.removeChild(textarea);
    });

    it("should use pixel units for left position (mirror-div technique)", () => {
      const textarea = document.createElement("textarea");
      textarea.value = "hello";
      document.body.appendChild(textarea);
      const ref = { current: textarea };

      const { container } = render(
        <BlockCaret textareaRef={ref} text="hello" settings={defaultSettings} visible={true} />
      );

      const caret = container.querySelector("[aria-hidden='true']") as HTMLElement;
      // Left should be in px (from mirror-div measurement), not ch units
      expect(caret.style.left).toMatch(/px$/);

      document.body.removeChild(textarea);
    });
  });
});
