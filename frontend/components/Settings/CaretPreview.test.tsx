import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import type { CaretSettings } from "@/lib/settings";
import { CaretPreview } from "./CaretPreview";

const blockSettings: CaretSettings = {
  style: "block",
  width: 1.0,
  color: null,
  blink_speed: 530,
  opacity: 1.0,
};

const defaultSettings: CaretSettings = {
  style: "default",
  width: 1.0,
  color: null,
  blink_speed: 530,
  opacity: 1.0,
};

describe("CaretPreview", () => {
  describe("rendering", () => {
    it("should render the preview label", () => {
      render(<CaretPreview settings={blockSettings} />);
      expect(screen.getByText("Preview")).toBeInTheDocument();
    });

    it("should render preview text content", () => {
      render(<CaretPreview settings={blockSettings} />);
      // The text is split around the caret but should be present
      expect(screen.getByText(/qbit> hello/)).toBeInTheDocument();
    });

    it("should render in a monospace font container", () => {
      const { container } = render(<CaretPreview settings={blockSettings} />);
      const previewBox = container.querySelector(".font-mono");
      expect(previewBox).toBeInTheDocument();
    });
  });

  describe("block style", () => {
    it("should render a block caret element with configured width", () => {
      const { container } = render(<CaretPreview settings={{ ...blockSettings, width: 2.0 }} />);
      const caretEl = container.querySelector(".inline-block") as HTMLElement;
      expect(caretEl).toBeInTheDocument();
      expect(caretEl.style.width).toBe("2ch");
    });

    it("should apply custom color to block caret", () => {
      const { container } = render(
        <CaretPreview settings={{ ...blockSettings, color: "#ff0000" }} />
      );
      const caretEl = container.querySelector(".inline-block") as HTMLElement;
      expect(caretEl.style.backgroundColor).toBe("rgb(255, 0, 0)");
    });

    it("should use var(--foreground) when color is null", () => {
      const { container } = render(<CaretPreview settings={{ ...blockSettings, color: null }} />);
      const caretEl = container.querySelector(".inline-block") as HTMLElement;
      expect(caretEl.style.backgroundColor).toBe("var(--foreground)");
    });

    it("should apply opacity to block caret", () => {
      const { container } = render(<CaretPreview settings={{ ...blockSettings, opacity: 0.5 }} />);
      const caretEl = container.querySelector(".inline-block") as HTMLElement;
      expect(caretEl.style.opacity).toBe("0.5");
    });

    it("should set blink animation with configured speed", () => {
      const { container } = render(
        <CaretPreview settings={{ ...blockSettings, blink_speed: 800 }} />
      );
      const caretEl = container.querySelector(".inline-block") as HTMLElement;
      expect(caretEl.style.animation).toContain("800ms");
    });

    it("should set animation to none when blink_speed is 0", () => {
      const { container } = render(
        <CaretPreview settings={{ ...blockSettings, blink_speed: 0 }} />
      );
      const caretEl = container.querySelector(".inline-block") as HTMLElement;
      expect(caretEl.style.animation).toBe("none");
    });

    it("should update preview width when settings change", () => {
      const { container, rerender } = render(
        <CaretPreview settings={{ ...blockSettings, width: 1.0 }} />
      );
      let caretEl = container.querySelector(".inline-block") as HTMLElement;
      expect(caretEl.style.width).toBe("1ch");

      rerender(<CaretPreview settings={{ ...blockSettings, width: 2.5 }} />);
      caretEl = container.querySelector(".inline-block") as HTMLElement;
      expect(caretEl.style.width).toBe("2.5ch");
    });
  });

  describe("default style", () => {
    it("should render a thin default caret indicator", () => {
      const { container } = render(<CaretPreview settings={defaultSettings} />);
      const caretEl = container.querySelector(".inline-block") as HTMLElement;
      expect(caretEl).toBeInTheDocument();
      expect(caretEl.style.width).toBe("2px");
    });

    it("should use var(--foreground) for default caret color", () => {
      const { container } = render(<CaretPreview settings={defaultSettings} />);
      const caretEl = container.querySelector(".inline-block") as HTMLElement;
      expect(caretEl.style.backgroundColor).toBe("var(--foreground)");
    });
  });
});
