import { memo, useMemo } from "react";
import type { CaretSettings } from "@/lib/settings";

interface CaretPreviewProps {
  settings: CaretSettings;
}

const PREVIEW_TEXT = "qbit> hello world";
const CARET_POS = 12; // After "qbit> hello " — mid-text position

/**
 * Live preview of the block caret rendered in a mock input area.
 * Mirrors the font/styling of UnifiedInput so the preview is accurate.
 */
export const CaretPreview = memo(function CaretPreview({ settings }: CaretPreviewProps) {
  const isBlock = settings.style === "block";
  const beforeCaret = PREVIEW_TEXT.slice(0, CARET_POS);
  const afterCaret = PREVIEW_TEXT.slice(CARET_POS);

  const blinkAnimation = useMemo(() => {
    if (!isBlock || settings.blink_speed <= 0) return "none";
    return `blink ${settings.blink_speed}ms step-end infinite`;
  }, [isBlock, settings.blink_speed]);

  return (
    <div className="space-y-2">
      <span className="text-sm font-medium text-foreground">Preview</span>
      <div className="relative rounded-md border border-[var(--border-subtle)] bg-background px-3 py-3 font-mono text-[13px] leading-relaxed overflow-hidden">
        {isBlock ? (
          <span className="whitespace-pre">
            {beforeCaret}
            <span
              className="relative inline-block"
              style={{
                width: `${settings.width}ch`,
                height: "1lh",
                backgroundColor: settings.color ?? "var(--foreground)",
                opacity: settings.opacity,
                animation: blinkAnimation,
                verticalAlign: "text-bottom",
              }}
            />
            {afterCaret}
          </span>
        ) : (
          <span className="whitespace-pre">
            {beforeCaret}
            <span
              className="relative inline-block"
              style={{
                width: "2px",
                height: "1lh",
                backgroundColor: "var(--foreground)",
                verticalAlign: "text-bottom",
                animation: "blink 1s step-end infinite",
              }}
            />
            {afterCaret}
          </span>
        )}
      </div>
    </div>
  );
});
