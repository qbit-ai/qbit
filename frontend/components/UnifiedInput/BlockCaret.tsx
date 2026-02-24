import { memo, useCallback, useEffect, useRef, useState } from "react";
import type { CaretSettings } from "@/lib/settings";

interface BlockCaretProps {
  /** Ref to the textarea this caret overlays */
  textareaRef: React.RefObject<HTMLTextAreaElement | null>;
  /** Current text value (used to compute row/col) */
  text: string;
  /** Caret settings from user preferences */
  settings: CaretSettings;
  /** Whether the caret should be visible (textarea focused + block style) */
  visible: boolean;
}

/** Compute row and column from selectionStart in a text string */
export function getRowCol(text: string, pos: number): { row: number; col: number } {
  const before = text.slice(0, pos);
  const lines = before.split("\n");
  return {
    row: lines.length - 1,
    col: lines[lines.length - 1].length,
  };
}

/**
 * CSS properties copied from the textarea to the mirror div for pixel-accurate
 * caret measurement. Based on the technique from textarea-caret-position.
 */
const MIRROR_PROPERTIES = [
  "direction",
  "boxSizing",
  "width",
  "height",
  "overflowX",
  "overflowY",
  "borderTopWidth",
  "borderRightWidth",
  "borderBottomWidth",
  "borderLeftWidth",
  "borderStyle",
  "paddingTop",
  "paddingRight",
  "paddingBottom",
  "paddingLeft",
  "fontStyle",
  "fontVariant",
  "fontWeight",
  "fontStretch",
  "fontSize",
  "fontSizeAdjust",
  "lineHeight",
  "fontFamily",
  "textAlign",
  "textTransform",
  "textIndent",
  "textDecoration",
  "letterSpacing",
  "wordSpacing",
  "tabSize",
  "whiteSpace",
  "wordWrap",
  "wordBreak",
  "overflowWrap",
] as const;

/**
 * Measure pixel-accurate caret coordinates using the mirror-div technique.
 *
 * Creates a hidden div styled identically to the textarea, copies text up to
 * the cursor position, places a marker <span>, and reads its offsetLeft/offsetTop.
 * This naturally accounts for padding, font metrics, letter-spacing, tab-size,
 * word wrapping, and avoids the cumulative drift of CSS `ch` units.
 */
export function getCaretCoordinates(
  textarea: HTMLTextAreaElement,
  position: number
): { top: number; left: number; height: number } {
  const div = document.createElement("div");
  div.id = "textarea-caret-mirror";

  const style = div.style;
  const computed = getComputedStyle(textarea);

  // Render off-screen but measurable (visibility: hidden keeps layout)
  style.position = "absolute";
  style.visibility = "hidden";
  style.overflow = "hidden";
  style.whiteSpace = "pre-wrap";
  style.wordWrap = "break-word";

  // Copy all relevant styles from the textarea
  for (const prop of MIRROR_PROPERTIES) {
    style[prop as unknown as number] = computed[prop as keyof CSSStyleDeclaration] as string;
  }

  // Firefox adds a scrollbar width to the overflow calculation
  const isFirefox =
    typeof navigator !== "undefined" && navigator.userAgent.toLowerCase().includes("firefox");
  if (isFirefox) {
    if (textarea.scrollHeight > parseInt(computed.height, 10)) {
      style.overflowY = "scroll";
    }
  } else {
    style.overflow = "hidden";
  }

  document.body.appendChild(div);

  // Insert text before cursor as a text node (preserves whitespace exactly)
  const textContent = textarea.value.substring(0, position);
  div.textContent = textContent;

  // Marker span at caret position — its offset gives pixel-accurate coordinates
  const span = document.createElement("span");
  // Use zero-width space so the span has height but no width contribution
  span.textContent = "\u200b";
  div.appendChild(span);

  const coords = {
    top: span.offsetTop - textarea.scrollTop,
    left: span.offsetLeft - textarea.scrollLeft,
    height: span.offsetHeight,
  };

  document.body.removeChild(div);

  return coords;
}

/**
 * A custom block caret overlay rendered on top of a <textarea>.
 *
 * Uses the mirror-div measurement technique for pixel-perfect positioning,
 * avoiding cumulative drift from CSS `ch` units.
 */
export const BlockCaret = memo(function BlockCaret({
  textareaRef,
  text,
  settings,
  visible,
}: BlockCaretProps) {
  const [coords, setCoords] = useState<{
    top: number;
    left: number;
    height: number;
  }>({ top: 0, left: 0, height: 0 });
  const fontSizeRef = useRef(13); // default, measured at mount

  // Measure font size from the textarea's computed style
  useEffect(() => {
    const textarea = textareaRef.current;
    if (!textarea) return;
    const computed = getComputedStyle(textarea);
    const fs = Number.parseFloat(computed.fontSize);
    if (!Number.isNaN(fs) && fs > 0) {
      fontSizeRef.current = fs;
    }
  }, [textareaRef]);

  // Update caret pixel coordinates via mirror-div measurement
  const updatePosition = useCallback(() => {
    const textarea = textareaRef.current;
    if (!textarea) return;
    const pos = textarea.selectionStart ?? 0;
    const measured = getCaretCoordinates(textarea, pos);
    setCoords(measured);
  }, [textareaRef]);

  // Listen to events that move the caret
  useEffect(() => {
    const textarea = textareaRef.current;
    if (!textarea || !visible) return;

    // Initial position
    updatePosition();

    // input/keyup catch typing and arrow keys
    textarea.addEventListener("input", updatePosition);
    textarea.addEventListener("keyup", updatePosition);
    textarea.addEventListener("mouseup", updatePosition);
    textarea.addEventListener("scroll", updatePosition);

    // selectionchange catches programmatic selection changes
    const onSelectionChange = () => {
      if (document.activeElement === textarea) {
        updatePosition();
      }
    };
    document.addEventListener("selectionchange", onSelectionChange);

    return () => {
      textarea.removeEventListener("input", updatePosition);
      textarea.removeEventListener("keyup", updatePosition);
      textarea.removeEventListener("mouseup", updatePosition);
      textarea.removeEventListener("scroll", updatePosition);
      document.removeEventListener("selectionchange", onSelectionChange);
    };
  }, [textareaRef, visible, updatePosition]);

  // Also update when text changes (e.g., from history navigation or programmatic set).
  // We reference `text` in the condition so biome sees it used inside the effect body.
  useEffect(() => {
    if (visible && text !== undefined) {
      updatePosition();
    }
  }, [text, visible, updatePosition]);

  if (!visible) return null;

  const fontSize = fontSizeRef.current;
  // Scale font size by 1.2 to cover ascenders/descenders (full glyph bounding box)
  const caretHeight = Math.round(fontSize * 1.2);
  // Use measured line height from mirror div, center the caret vertically
  const verticalOffset = coords.height > 0 ? (coords.height - caretHeight) / 2 : 0;
  const top = coords.top + verticalOffset;

  const blinkAnimation =
    settings.blink_speed > 0 ? `blink ${settings.blink_speed}ms step-end infinite` : "none";

  return (
    <div
      className="absolute pointer-events-none"
      style={{
        top: `${top}px`,
        left: `${coords.left}px`,
        width: `${settings.width}ch`,
        height: `${caretHeight}px`,
        backgroundColor: settings.color ?? "var(--foreground)",
        opacity: settings.opacity,
        animation: blinkAnimation,
        // Small z-index to render over text but under popups
        zIndex: 1,
      }}
      aria-hidden="true"
    />
  );
});
